/* Bebop supervision tree — implementation. */
#include "supervise.h"

#include <stdio.h>
#include <string.h>

void supervisor_init(Supervisor *s) {
    s->n = 0;
}

int supervisor_run(Supervisor *s, const char *name, BumpArena *arena, ChildFn f) {
    if (s->n >= (int)(sizeof s->children / sizeof s->children[0])) {
        return -2;
    }
    SupervisedChild *c = &s->children[s->n++];
    c->name = name;
    c->arena = arena;
    c->checkpoint = arena_snapshot_take(arena); /* CoW checkpoint */
    c->failed = 0;
    int r = f(arena);
    if (r != 0) {
        arena_snapshot_restore(c->checkpoint); /* O(1) rollback */
        c->failed = 1;
        return -1;
    }
    return 0;
}

int supervisor_rollback(Supervisor *s, int idx) {
    if (idx < 0 || idx >= s->n) {
        return -1;
    }
    arena_snapshot_restore(s->children[idx].checkpoint);
    return 0;
}

/* a child that succeeds: allocates 64 bytes and returns 0 */
static int child_ok(BumpArena *a) {
    return arena_alloc(a, 64) == NULL ? -1 : 0;
}

/* a child that fails: allocates 64 bytes then reports an error */
static int child_fail(BumpArena *a) {
    arena_alloc(a, 64);
    return -1; /* boom */
}

int supervise_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    unsigned char buf[4096];
    BumpArena a;
    arena_init(&a, buf, sizeof buf);

    Supervisor s;
    supervisor_init(&s);

    /* a succeeding child keeps its allocations */
    A(supervisor_run(&s, "ok", &a, child_ok) == 0, "child_ok succeeds");
    A(!s.children[0].failed, "child_ok not marked failed");
    A(arena_used(&a) == 64, "child_ok allocations persist");

    /* a failing child is rolled back to its checkpoint */
    size_t before = arena_used(&a);
    A(supervisor_run(&s, "fail", &a, child_fail) == -1, "child_fail reports failure");
    A(s.children[1].failed, "child_fail marked failed");
    A(arena_used(&a) == before, "child_fail arena rolled back to checkpoint (isolation)");

    /* manual rollback of a healthy child frees its work */
    A(supervisor_rollback(&s, 0) == 0, "manual rollback ok");
    A(arena_used(&a) == 0, "manual rollback frees child_ok work");

    return all_ok ? 0 : -1;
}
