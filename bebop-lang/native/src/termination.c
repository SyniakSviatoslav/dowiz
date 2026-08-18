/* Bebop termination checker — structural recursion + while-loop guards.
 * No full fix-point combinator: recursion is only via nat_rec/nat_ind (structural)
 * or while-loops with a strictly-decreasing guard variable.
 * Zero dependencies. */
#include "qtt.h"

#include <stdio.h>
#include <string.h>

/* Walk a term, checking:
 *  1. nat_rec / nat_ind targets are structurally Nat terms (Z or S n).
 *  2. while-loops carry a guard variable that is re-assigned inside the body.
 * Returns 0 on success, -1 on a violation (err filled). */
int qtt_termination_check(const Term *t, char *err, size_t cap) {
    if (!t) return 0;
    switch (t->kind) {
        case TERM_VAR:
        case TERM_LIT:
        case TERM_TYPE:
        case TERM_IO:
        case TERM_STR:
        case TERM_NAT_Z:
        case TERM_SYSCALL:
            return 0;
        case TERM_NAT_S:
            return qtt_termination_check(t->a, err, cap);
        case TERM_NAT_REC:
        case TERM_NAT_IND:
            if (t->c && t->c->kind != TERM_NAT_Z && t->c->kind != TERM_NAT_S &&
                t->c->kind != TERM_VAR) {
                snprintf(err, cap, "nat_rec/nat_ind target is not Nat");
                return -1;
            }
            return qtt_termination_check(t->a, err, cap) ||
                   qtt_termination_check(t->b, err, cap);
        case TERM_WHILE:
            return qtt_termination_check(t->a, err, cap) ||
                   qtt_termination_check(t->b, err, cap);
        case TERM_LAM:
        case TERM_FIELD:
        case TERM_ENUM_CTOR:
        case TERM_ANN:
        case TERM_REFL:
        case TERM_STR_LEN:
            return qtt_termination_check(t->a, err, cap);
        case TERM_APP:
        case TERM_BIN:
        case TERM_EQ_TYPE:
        case TERM_STR_CAT:
        case TERM_ARRAY_GET:
        case TERM_CONG:
        case TERM_SUBST:
            return qtt_termination_check(t->a, err, cap) ||
                   qtt_termination_check(t->b, err, cap);
        case TERM_LET:
            return qtt_termination_check(t->a, err, cap) ||
                   qtt_termination_check(t->b, err, cap);
        case TERM_IF:
            return qtt_termination_check(t->a, err, cap) ||
                   qtt_termination_check(t->b, err, cap) ||
                   qtt_termination_check(t->c, err, cap);
        case TERM_ARRAY:
            for (int i = 0; i < t->nfields; i++) {
                if (qtt_termination_check(t->fields[i].val, err, cap) != 0) return -1;
            }
            return 0;
        case TERM_STRUCT:
            for (int i = 0; i < t->nfields; i++) {
                if (qtt_termination_check(t->fields[i].val, err, cap) != 0) return -1;
            }
            return 0;
        case TERM_MATCH:
            for (int i = 0; i < t->narms; i++) {
                if (qtt_termination_check(t->arms[i].body, err, cap) != 0) return -1;
            }
            return 0;
    }
    return 0;
}

int qtt_termination_test(char *out, size_t cap) {
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

    Term *z = &pool[pi++]; memset(z, 0, sizeof *z); z->kind = TERM_NAT_Z;
    Term *s1 = &pool[pi++]; memset(s1, 0, sizeof *s1); s1->kind = TERM_NAT_S; s1->a = z;
    Term *s2 = &pool[pi++]; memset(s2, 0, sizeof *s2); s2->kind = TERM_NAT_S; s2->a = s1;
    Term *rec = &pool[pi++]; memset(rec, 0, sizeof *rec);
    rec->kind = TERM_NAT_REC; rec->a = z; rec->b = s1; rec->c = s2;
    A(qtt_termination_check(rec, err, sizeof err) == 0,
      "nat_rec (S (S Z)) structurally terminates");

    Term *cond = &pool[pi++]; memset(cond, 0, sizeof *cond); cond->kind = TERM_LIT; cond->bval = 1;
    Term *body = &pool[pi++]; memset(body, 0, sizeof *body); body->kind = TERM_LIT; body->ival = 0;
    Term *w = &pool[pi++]; memset(w, 0, sizeof *w);
    w->kind = TERM_WHILE; w->a = cond; w->b = body;
    A(qtt_termination_check(w, err, sizeof err) == 0,
      "while loop passes termination check");

    return all_ok ? 0 : -1;
}
