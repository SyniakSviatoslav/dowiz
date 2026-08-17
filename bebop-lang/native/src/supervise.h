/* Bebop supervision tree (20B / #34).
 *
 * A supervisor checkpoints each child's arena (CoW snapshot) before it runs.
 * A child that fails is rolled back to its checkpoint in O(1) — its arena
 * mutations are undone, so the fault is isolated and the child can be retried
 * from a clean state without corrupting the parent or siblings. This is the
 * "watchdog tree with instant CoW rollback" from the architecture catalog.
 */
#ifndef BEBOP_SUPERVISE_H
#define BEBOP_SUPERVISE_H

#include <stddef.h>

#include "arena.h"

typedef int (*ChildFn)(BumpArena *);

typedef struct {
    const char *name;
    BumpArena *arena;
    ArenaSnapshot checkpoint; /* CoW snapshot taken before the child runs */
    int failed;
} SupervisedChild;

typedef struct {
    SupervisedChild children[16];
    int n;
} Supervisor;

void supervisor_init(Supervisor *s);
/* Checkpoint the child's arena, run it; on failure (non-zero return) roll the
 * arena back to the checkpoint and mark the child failed. Returns 0 on success,
 * -1 on failure (rolled back) or -2 if the tree is full. */
int supervisor_run(Supervisor *s, const char *name, BumpArena *arena, ChildFn f);
/* Roll back a specific child to its checkpoint (manual isolation). */
int supervisor_rollback(Supervisor *s, int idx);

int supervise_self_test(char *out, size_t cap);

#endif /* BEBOP_SUPERVISE_H */
