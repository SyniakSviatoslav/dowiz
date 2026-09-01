/* Bebop hot-swappable JIT table — implementation. */
#include "jittable.h"

#include <stdatomic.h>
#include <stdio.h>
#include <string.h>

void jitslot_init(JitSlot *s, JitFn fn) {
    atomic_store_explicit(&s->fn, fn, memory_order_release);
}

long jitslot_call(JitSlot *s) {
    JitFn f = atomic_load_explicit(&s->fn, memory_order_acquire);
    return f();
}

JitFn jitslot_swap(JitSlot *s, JitFn newfn) {
    return atomic_exchange_explicit(&s->fn, newfn, memory_order_acq_rel);
}

static long fn_a(void) { return 1; }
static long fn_b(void) { return 2; }
static long fn_c(void) { return 3; }

int jittable_self_test(char *out, size_t cap) {
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

    JitSlot s;
    jitslot_init(&s, fn_a);
    A(jitslot_call(&s) == 1, "slot calls the installed fn (fn_a -> 1)");

    JitFn old = jitslot_swap(&s, fn_b);
    A(old == fn_a, "swap returns the previously-installed fn");
    A(jitslot_call(&s) == 2, "slot calls the new fn after swap (fn_b -> 2)");

    jitslot_swap(&s, fn_c);
    A(jitslot_call(&s) == 3, "second swap (fn_c -> 3)");

    /* atomicity: rapid concurrent swaps must never observe a torn fn —
     * every call returns one of {1,2,3}, never garbage. */
    int valid = 1;
    for (int i = 0; i < 1000; i++) {
        jitslot_swap(&s, (i & 1) ? fn_a : fn_b);
        long r = jitslot_call(&s);
        if (r != 1 && r != 2) {
            valid = 0;
            break;
        }
    }
    A(valid, "no torn reads across 1000 rapid swaps");

    return all_ok ? 0 : -1;
}
