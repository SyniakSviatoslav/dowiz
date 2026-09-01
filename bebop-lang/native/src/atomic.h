/* Bebop atomic — lock-free primitives (C11 atomics + spinlock + Treiber stack).
 * Gap 6 (concurrency) slice 1: the foundation of lock-free arenas and Send/Sync
 * thread-safety. */
#ifndef BEBOP_ATOMIC_H
#define BEBOP_ATOMIC_H

#include <stddef.h>
#include <stdint.h>

typedef _Atomic uint64_t AtomicU64;

uint64_t bp_atomic_fetch_add(AtomicU64 *a, uint64_t v);
uint64_t bp_atomic_load(const AtomicU64 *a);
void bp_atomic_store(AtomicU64 *a, uint64_t v);

typedef struct {
    _Atomic int locked;
} Spinlock;
void spinlock_init(Spinlock *l);
void spinlock_lock(Spinlock *l);
void spinlock_unlock(Spinlock *l);

/* Treiber lock-free stack (intrusive). */
typedef struct StackNode {
    struct StackNode *next;
    uint64_t value;
} StackNode;
typedef struct {
    _Atomic(StackNode *) head;
} LockFreeStack;
void lfstack_init(LockFreeStack *s);
void lfstack_push(LockFreeStack *s, StackNode *n);
StackNode *lfstack_pop(LockFreeStack *s);

int atomic_self_test(char *out, size_t cap);

#endif /* BEBOP_ATOMIC_H */
