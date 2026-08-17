/* Bebop green threads — stackful coroutines, hand-written AArch64 context
 * switch. Zero allocation at runtime, fixed arena, deterministic round-robin. */
#include "gt.h"

#include <stdio.h>
#include <string.h>

/* ─── fixed stack arena ─── */
#define GT_STACK 4096
#define GT_MAX 8

static unsigned char gt_arena[GT_MAX][GT_STACK];
static int gt_used[GT_MAX];

static GtCoroutine gt_cos[GT_MAX];
static int gt_nco = 0;
static GtContext gt_sched;
static GtCoroutine *gt_current = NULL;

/* ─── context switch (hand-written asm in gt_switch.S) ─── */
extern void gt_switch(GtContext *from, GtContext *to);
extern void gt_trampoline(void);

void gt_finish(void) {
    gt_current->done = 1;
    gt_switch(&gt_current->ctx, &gt_sched); /* never returns */
}

int gt_spawn(GtCoroutine *co, GtFn fn, void *arg) {
    int slot = -1;
    for (int i = 0; i < GT_MAX; i++) {
        if (!gt_used[i]) {
            slot = i;
            break;
        }
    }
    if (slot < 0) {
        return -1;
    }
    gt_used[slot] = 1;
    unsigned char *base = gt_arena[slot];
    unsigned char *top = base + GT_STACK;
    top = (unsigned char *)((uintptr_t)top & ~(uintptr_t)15); /* 16-align */

    memset(&co->ctx, 0, sizeof co->ctx);
    co->ctx.x19 = (uint64_t)(uintptr_t)fn;
    co->ctx.x20 = (uint64_t)(uintptr_t)arg;
    co->ctx.lr = (uint64_t)(uintptr_t)gt_trampoline;
    co->ctx.sp = (uint64_t)(uintptr_t)top;
    co->stack = base;
    co->done = 0;
    return 0;
}

void gt_yield(void) {
    gt_switch(&gt_current->ctx, &gt_sched);
}

void gt_sched_reset(void) {
    gt_nco = 0;
    memset(gt_used, 0, sizeof gt_used);
    gt_current = NULL;
}

/* spawn into the scheduler's fixed table (convenience for the test harness) */
static void gt_sched_spawn(GtFn fn, void *arg) {
    if (gt_nco < GT_MAX) {
        GtCoroutine *co = &gt_cos[gt_nco];
        if (gt_spawn(co, fn, arg) == 0) {
            gt_nco++;
        }
    }
}

void gt_sched_run(void) {
    for (;;) {
        int any = 0;
        for (int i = 0; i < gt_nco; i++) {
            if (!gt_cos[i].done) {
                any = 1;
                gt_current = &gt_cos[i];
                gt_switch(&gt_sched, &gt_cos[i].ctx);
            }
        }
        if (!any) {
            break;
        }
    }
    gt_sched_reset();
}

/* ─── self-test ─── */

static int gt_order[16];
static int gt_total = 0;

static void gt_worker(void *arg) {
    int id = (int)(intptr_t)arg;
    for (int i = 0; i < 3; i++) {
        gt_order[gt_total++] = id;
        gt_yield();
    }
}

int gt_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define G(cond, name)                                                      \
    do {                                                                   \
        int c_ = (int)(cond);                                              \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",               \
                          c_ ? "ok" : "FAIL", name);                       \
        if (r_ > 0) pos += (size_t)r_;                                     \
        if (!c_) all_ok = 0;                                               \
    } while (0)

    gt_total = 0;
    gt_sched_reset();
    gt_sched_spawn(gt_worker, (void *)(intptr_t)0);
    gt_sched_spawn(gt_worker, (void *)(intptr_t)1);
    gt_sched_spawn(gt_worker, (void *)(intptr_t)2);
    gt_sched_run();

    G(gt_total == 9, "green threads: 3 coroutines × 3 yields = 9 steps");
    int expect[9] = {0, 1, 2, 0, 1, 2, 0, 1, 2};
    int order_ok = 1;
    for (int i = 0; i < 9; i++) {
        if (gt_order[i] != expect[i]) {
            order_ok = 0;
            break;
        }
    }
    G(order_ok, "green threads: round-robin interleaving 0,1,2,0,1,2,0,1,2");

    return all_ok ? 0 : -1;
}
