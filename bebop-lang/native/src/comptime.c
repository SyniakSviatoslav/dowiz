/* Bebop comptime — compile-time evaluation of pure (const) expressions.
 * Zig comptime parity: `comptime expr` is evaluated at elaboration and baked
 * into .rodata; runtime = pointer deref. Zero dependencies. */
#include "comptime.h"

#include <stdio.h>
#include <string.h>

/* Evaluate a closed, pure term at compile time. Returns 0 on success, fills
 * fills kind/i/b with the result. Returns -1 if impure, open, or non-constant. */
int bp_comptime_eval(const Term *t, int *out_kind, long *out_i, int *out_b,
                     char *err, size_t cap) {
    if (!t) { snprintf(err, cap, "null term"); return -1; }
    /* Impure constructs cannot be comptime */
    if (qtt_term_has_io(t)) {
        snprintf(err, cap, "comptime expression has IO effect");
        return -1;
    }
    /* Evaluate via the interpreter (closed term, no environment) */
    if (qtt_eval(t, out_kind, out_i, out_b, err, cap) != 0) {
        return -1;
    }
    return 0;
}

/* Verify a comptime expression type-checks and is constant, then run it.
 * Used by the `comptime` subcommand for self-testing. */
int comptime_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[256];
#undef A
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    static Term pool[16];
    int pi = 0;
    int k; long i; int b;

    /* comptime: 40 + 2 == 42 */
    Term *l40 = &pool[pi++]; memset(l40, 0, sizeof *l40); l40->kind = TERM_LIT; l40->ival = 40;
    Term *l2 = &pool[pi++]; memset(l2, 0, sizeof *l2); l2->kind = TERM_LIT; l2->ival = 2;
    Term *add = &pool[pi++]; memset(add, 0, sizeof *add);
    add->kind = TERM_BIN; add->op = BOP_ADD; add->a = l40; add->b = l2;
    A(bp_comptime_eval(add, &k, &i, &b, err, sizeof err) == 0 && i == 42,
      "comptime 40+2 == 42");

    /* comptime: string length of "hello" == 5 */
    Term *s = &pool[pi++]; memset(s, 0, sizeof *s); s->kind = TERM_STR; s->name = "hello";
    Term *sl = &pool[pi++]; memset(sl, 0, sizeof *sl); sl->kind = TERM_STR_LEN; sl->a = s;
    A(bp_comptime_eval(sl, &k, &i, &b, err, sizeof err) == 0 && i == 5,
      "comptime str_len(\"hello\") == 5");

    return all_ok ? 0 : -1;
}
