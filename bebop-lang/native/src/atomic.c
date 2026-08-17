/* Bebop atomic — implementation (C11 atomics + lock-free stack). */
#include "atomic.h"

#include <pthread.h>
#include <stdatomic.h>
#include <stdio.h>

uint64_t bp_atomic_fetch_add(AtomicU64 *a, uint64_t v) {
    return atomic_fetch_add_explicit(a, v, memory_order_relaxed);
}

uint64_t bp_atomic_load(const AtomicU64 *a) {
    return atomic_load_explicit(a, memory_order_relaxed);
}

void bp_atomic_store(AtomicU64 *a, uint64_t v) {
    atomic_store_explicit(a, v, memory_order_relaxed);
}

void spinlock_init(Spinlock *l) {
    atomic_store_explicit(&l->locked, 0, memory_order_relaxed);
}

void spinlock_lock(Spinlock *l) {
    while (atomic_exchange_explicit(&l->locked, 1, memory_order_acquire)) {
        /* spin */
    }
}

void spinlock_unlock(Spinlock *l) {
    atomic_store_explicit(&l->locked, 0, memory_order_release);
}

void lfstack_init(LockFreeStack *s) {
    atomic_store_explicit(&s->head, NULL, memory_order_relaxed);
}

void lfstack_push(LockFreeStack *s, StackNode *n) {
    StackNode *old = atomic_load_explicit(&s->head, memory_order_relaxed);
    do {
        n->next = old;
    } while (!atomic_compare_exchange_weak_explicit(
        &s->head, &old, n, memory_order_release, memory_order_relaxed));
}

StackNode *lfstack_pop(LockFreeStack *s) {
    StackNode *old = atomic_load_explicit(&s->head, memory_order_relaxed);
    StackNode *next;
    do {
        if (!old) {
            return NULL;
        }
        next = old->next;
    } while (!atomic_compare_exchange_weak_explicit(
        &s->head, &old, next, memory_order_acquire, memory_order_relaxed));
    return old;
}

/* concurrent increment test: N threads each do M atomic increments. */
#define NTHREADS 4
#define NINCR 10000

static AtomicU64 g_counter;

static void *incr_thread(void *arg) {
    (void)arg;
    for (int i = 0; i < NINCR; i++) {
        bp_atomic_fetch_add(&g_counter, 1);
    }
    return NULL;
}

int atomic_self_test(char *out, size_t cap) {
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

    AtomicU64 a = 0;
    A(bp_atomic_fetch_add(&a, 5) == 0 && bp_atomic_load(&a) == 5, "fetch_add");

    Spinlock l;
    spinlock_init(&l);
    spinlock_lock(&l);
    spinlock_unlock(&l);
    A(1, "spinlock lock/unlock");

    LockFreeStack s;
    lfstack_init(&s);
    StackNode n1 = {NULL, 1}, n2 = {NULL, 2}, n3 = {NULL, 3};
    lfstack_push(&s, &n1);
    lfstack_push(&s, &n2);
    lfstack_push(&s, &n3);
    StackNode *p = lfstack_pop(&s);
    A(p == &n3, "lfstack pop LIFO");
    p = lfstack_pop(&s);
    A(p == &n2, "lfstack pop 2nd");

    g_counter = 0;
    pthread_t th[NTHREADS];
    for (int i = 0; i < NTHREADS; i++) {
        pthread_create(&th[i], NULL, incr_thread, NULL);
    }
    for (int i = 0; i < NTHREADS; i++) {
        pthread_join(th[i], NULL);
    }
    A(bp_atomic_load(&g_counter) == NTHREADS * NINCR, "concurrent increment == N*M");

    return all_ok ? 0 : -1;
}
