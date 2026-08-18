/* Bebop green threads — stackful coroutines in fixed arenas (17B / #33 / #37).
 *
 * Cooperative, deterministic, lock-free: each coroutine owns a fixed stack
 * from a static arena and switches context by saving/restoring the callee-saved
 * register file (x19..x28, fp, lr, sp) — hand-written AArch64 asm, no setjmp,
 * no OS scheduler, no allocation at runtime. */
#ifndef BEBOP_GT_H
#define BEBOP_GT_H

#include <stddef.h>
#include <stdint.h>

typedef void (*GtFn)(void *arg);

/* The saved register file. Layout MUST match gt_switch's stp/ldp offsets. */
typedef struct {
    uint64_t x19, x20, x21, x22, x23, x24, x25, x26, x27, x28;
    uint64_t fp, lr;
    uint64_t sp;
} GtContext;

typedef struct {
    GtContext ctx;
    unsigned char *stack; /* base of the owned stack (for diagnostics) */
    int done;
    void *result;         /* async return value (set by fn via gt_return) */
} GtCoroutine;

/* Switch context: save *from, restore *to. Does not return to the caller's
 * original frame — it returns on the *to* context's saved link register. */
void gt_switch(GtContext *from, GtContext *to);

/* Initialize a coroutine to run fn(arg) on a fresh stack. Returns 0 on
 * success, -1 if the arena is exhausted. */
int gt_spawn(GtCoroutine *co, GtFn fn, void *arg);

/* Yield control back to the scheduler. */
void gt_yield(void);

/* Run all spawned coroutines round-robin until every one finishes. */
void gt_sched_run(void);

/* Reset the scheduler (fresh spawns after a run). */
void gt_sched_reset(void);

int gt_self_test(char *out, size_t cap);

/* ─── async/await ──────────────────────────────────────────────────────── */

/* Set the current coroutine's return value (call from within coroutine fn). */
void gt_return(void *val);

/* Yield until coroutine at index completes, return its result. */
void *gt_await(int co_idx);

/* Non-blocking: has coroutine finished? */
int gt_async_done(int co_idx);

#endif /* BEBOP_GT_H */
