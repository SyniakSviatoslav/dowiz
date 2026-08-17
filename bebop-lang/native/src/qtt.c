/* Bebop QTT core — implementation. Zero dependencies. */
#include "qtt.h"

#include <stdio.h>

Quantity qtt_add(Quantity a, Quantity b) {
    if (a == Q_MANY || b == Q_MANY) {
        return Q_MANY;
    }
    if (a == Q_ONE && b == Q_ONE) {
        return Q_MANY; /* 1 + 1 = ω */
    }
    if (a == Q_ONE || b == Q_ONE) {
        return Q_ONE;
    }
    return Q_ZERO;
}

Quantity qtt_mul(Quantity a, Quantity b) {
    if (a == Q_ZERO || b == Q_ZERO) {
        return Q_ZERO;
    }
    if (a == Q_MANY || b == Q_MANY) {
        return Q_MANY;
    }
    return Q_ONE; /* 1 · 1 = 1 */
}

const char *qtt_q_name(Quantity q) {
    switch (q) {
        case Q_ZERO:
            return "0";
        case Q_ONE:
            return "1";
        case Q_MANY:
            return "ω";
    }
    return "?";
}

int qtt_ty_print(const Ty *t, char *out, size_t cap) {
    switch (t->kind) {
        case TY_I64:
            return snprintf(out, cap, "i64");
        case TY_F64:
            return snprintf(out, cap, "f64");
        case TY_BOOL:
            return snprintf(out, cap, "bool");
        case TY_VOID:
            return snprintf(out, cap, "void");
        case TY_FIELD:
            return snprintf(out, cap, "F_%ld", t->n);
        case TY_HYPERVEC:
            return snprintf(out, cap, "Hypervector<%ld>", t->n);
        case TY_VEC: {
            char e[64];
            qtt_ty_print(t->elem, e, sizeof e);
            return snprintf(out, cap, "Vector<%ld,%s>", t->n, e);
        }
        case TY_FN: {
            char d[128], c[128];
            qtt_ty_print(t->dom, d, sizeof d);
            qtt_ty_print(t->cod, c, sizeof c);
            return snprintf(out, cap, "(%s -> %s)", d, c);
        }
        case TY_PI: {
            char d[128], c[128];
            qtt_ty_print(t->dom, d, sizeof d);
            qtt_ty_print(t->cod, c, sizeof c);
            return snprintf(out, cap, "(%s :^%s %s -> %s)", t->x ? t->x : "_",
                            qtt_q_name(t->q), d, c);
        }
    }
    return snprintf(out, cap, "?");
}

int qtt_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define CHECK(cond, name)                                            \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) {                                                 \
            pos += (size_t)r;                                        \
        }                                                            \
        if (!(cond)) {                                               \
            all_ok = 0;                                              \
        }                                                            \
    } while (0)

    /* Semiring laws (the QTT rig). */
    for (int p = 0; p <= 2; p++) {
        CHECK(qtt_add(Q_ZERO, (Quantity)p) == (Quantity)p, "add(0,p) == p");
        CHECK(qtt_mul(Q_ONE, (Quantity)p) == (Quantity)p, "mul(1,p) == p");
        CHECK(qtt_mul(Q_ZERO, (Quantity)p) == Q_ZERO, "mul(0,p) == 0");
    }
    CHECK(qtt_add(Q_ONE, Q_ONE) == Q_MANY, "add(1,1) == ω");
    CHECK(qtt_add(Q_MANY, Q_ONE) == Q_MANY, "add(ω,1) == ω");
    CHECK(qtt_mul(Q_MANY, Q_ONE) == Q_MANY, "mul(ω,1) == ω");
    CHECK(qtt_mul(Q_MANY, Q_ZERO) == Q_ZERO, "mul(ω,0) == 0");
    CHECK(qtt_mul(Q_MANY, Q_MANY) == Q_MANY, "mul(ω,ω) == ω");

    /* Type pretty-printing round-trip sanity. */
    Ty i64 = {TY_I64, 0, Q_ZERO, 0, 0, 0, 0};
    Ty f = {TY_FIELD, 0xFFFFFFFF00000001ULL, Q_ZERO, 0, 0, 0, 0};
    Ty hv = {TY_HYPERVEC, 1024, Q_ZERO, 0, 0, 0, 0};
    Ty fn = {TY_FN, 0, Q_ZERO, 0, &i64, &i64, 0};
    char b[128];
    qtt_ty_print(&i64, b, sizeof b);
    CHECK(b[0] == 'i' && b[1] == '6' && b[2] == '4', "print i64");
    qtt_ty_print(&hv, b, sizeof b);
    CHECK(b[0] == 'H', "print Hypervector<1024>");
    qtt_ty_print(&fn, b, sizeof b);
    CHECK(b[0] == '(', "print (i64 -> i64)");
    qtt_ty_print(&f, b, sizeof b);
    CHECK(b[0] == 'F', "print F_p");

    return all_ok ? 0 : -1;
}
