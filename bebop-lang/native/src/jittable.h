/* Bebop hot-swappable JIT table (13B / #24).
 *
 * A function-table slot whose pointer is _Atomic: a running node can swap its
 * logic in-place (CAS/exchange) without ever stopping — the slot always holds
 * exactly one valid, fully-published function, so callers observe either the
 * old or the new fn, never a torn/partial pointer. This is the substrate for
 * live self-upgrade (PGO-driven recompilation, hot-patching, agent evolution).
 */
#ifndef BEBOP_JITTABLE_H
#define BEBOP_JITTABLE_H

#include <stddef.h>

typedef long (*JitFn)(void);

typedef struct {
    _Atomic(JitFn) fn;
} JitSlot;

void jitslot_init(JitSlot *s, JitFn fn);
/* Load (acquire) + call. Returns the fn's result. */
long jitslot_call(JitSlot *s);
/* Atomically swap in `newfn`; returns the previously-installed fn. */
JitFn jitslot_swap(JitSlot *s, JitFn newfn);

int jittable_self_test(char *out, size_t cap);

#endif /* BEBOP_JITTABLE_H */
