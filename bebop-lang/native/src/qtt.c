/* Bebop QTT core — implementation. Zero dependencies. */
#include "qtt.h"

#include <stdio.h>
#include <string.h>

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

/* ═══ Terms + bidirectional typechecker (linear/affine) ═══ */

static Ty I64_TY = {TY_I64, 0, Q_ZERO, 0, 0, 0, 0};
static Ty BOOL_TY = {TY_BOOL, 0, Q_ZERO, 0, 0, 0, 0};

/* Bump-allocated type pool (types live for the self-test lifetime). */
static Ty ty_pool[256];
static int ty_len = 0;

static Ty *ty_alloc(TyKind kind) {
    if (ty_len >= (int)(sizeof ty_pool / sizeof ty_pool[0])) {
        return NULL;
    }
    Ty *t = &ty_pool[ty_len++];
    memset(t, 0, sizeof *t);
    t->kind = kind;
    return t;
}

typedef struct {
    const char *name;
    Quantity q;
    Ty *ty;
    int used;
} Binding;

typedef struct {
    Binding b[64];
    int len;
} Ctx;

static Binding *ctx_lookup(Ctx *c, const char *name) {
    for (int i = c->len - 1; i >= 0; i--) {
        if (strcmp(c->b[i].name, name) == 0) {
            return &c->b[i];
        }
    }
    return NULL;
}

static int ty_eq(const Ty *a, const Ty *b) {
    if (a->kind != b->kind) {
        return 0;
    }
    switch (a->kind) {
        case TY_I64:
        case TY_F64:
        case TY_BOOL:
        case TY_VOID:
            return 1;
        case TY_FIELD:
        case TY_HYPERVEC:
            return a->n == b->n;
        case TY_FN:
        case TY_PI:
            return ty_eq(a->dom, b->dom) && ty_eq(a->cod, b->cod);
        case TY_VEC:
            return a->n == b->n && ty_eq(a->elem, b->elem);
    }
    return 0;
}

static int infer(Ctx *c, const Term *t, Ty **out, char *err, size_t cap);
static int check(Ctx *c, const Term *t, const Ty *want, char *err, size_t cap);

static int infer(Ctx *c, const Term *t, Ty **out, char *err, size_t cap) {
    switch (t->kind) {
        case TERM_VAR: {
            Binding *b = ctx_lookup(c, t->name);
            if (!b) {
                snprintf(err, cap, "unbound variable '%s'", t->name);
                return -1;
            }
            if (b->q == Q_ONE) {
                if (b->used) {
                    snprintf(err, cap, "linear variable '%s' used more than once", t->name);
                    return -1;
                }
                b->used = 1;
            }
            *out = b->ty;
            return 0;
        }
        case TERM_LIT: {
            *out = t->bval ? &BOOL_TY : &I64_TY;
            return 0;
        }
        case TERM_LAM: {
            if (!t->ty) {
                snprintf(err, cap, "lambda requires a domain annotation");
                return -1;
            }
            Ty *pi = ty_alloc(TY_PI);
            if (!pi) {
                snprintf(err, cap, "type pool exhausted");
                return -1;
            }
            pi->q = t->q;
            pi->x = t->name;
            pi->dom = t->ty;
            c->b[c->len].name = t->name;
            c->b[c->len].q = t->q;
            c->b[c->len].ty = t->ty;
            c->b[c->len].used = 0;
            c->len++;
            Ty *cod = NULL;
            int r = infer(c, t->a, &cod, err, cap);
            c->len--;
            if (r != 0) {
                return r;
            }
            pi->cod = cod;
            *out = pi;
            return 0;
        }
        case TERM_APP: {
            Ty *ft = NULL;
            if (infer(c, t->a, &ft, err, cap) != 0) {
                return -1;
            }
            if (ft->kind != TY_FN && ft->kind != TY_PI) {
                snprintf(err, cap, "applied a non-function type");
                return -1;
            }
            if (check(c, t->b, ft->dom, err, cap) != 0) {
                return -1;
            }
            *out = ft->cod; /* non-dependent substitution for now */
            return 0;
        }
        case TERM_BIN: {
            Ty *l = NULL, *r = NULL;
            if (infer(c, t->a, &l, err, cap) != 0) {
                return -1;
            }
            if (infer(c, t->b, &r, err, cap) != 0) {
                return -1;
            }
            if (l->kind != TY_I64 || r->kind != TY_I64) {
                snprintf(err, cap, "binary op requires i64 operands");
                return -1;
            }
            *out = (t->op == BOP_EQ || t->op == BOP_LT) ? &BOOL_TY : &I64_TY;
            return 0;
        }
        case TERM_ANN: {
            if (check(c, t->a, t->ty, err, cap) != 0) {
                return -1;
            }
            *out = t->ty;
            return 0;
        }
    }
    snprintf(err, cap, "unknown term");
    return -1;
}

static int check(Ctx *c, const Term *t, const Ty *want, char *err, size_t cap) {
    if (t->kind == TERM_LAM) {
        if (want->kind != TY_FN && want->kind != TY_PI) {
            snprintf(err, cap, "expected a function type");
            return -1;
        }
        const Ty *dom = t->ty ? t->ty : want->dom;
        c->b[c->len].name = t->name;
        c->b[c->len].q = t->q;
        c->b[c->len].ty = (Ty *)dom;
        c->b[c->len].used = 0;
        c->len++;
        int r = check(c, t->a, want->cod, err, cap);
        c->len--;
        return r;
    }
    Ty *got = NULL;
    if (infer(c, t, &got, err, cap) != 0) {
        return -1;
    }
    if (!ty_eq(got, want)) {
        snprintf(err, cap, "type mismatch");
        return -1;
    }
    return 0;
}

int qtt_check_closed(const Term *t, char *out_ty, size_t cap_ty, char *err, size_t cap_err) {
    Ctx c;
    memset(&c, 0, sizeof c);
    ty_len = 0;
    Ty *got = NULL;
    if (infer(&c, t, &got, err, cap_err) != 0) {
        return -1;
    }
    qtt_ty_print(got, out_ty, cap_ty);
    return 0;
}

int qtt_check_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[128], ty[128];
#define T(cond, name)                                               \
    do {                                                            \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",         \
                         (cond) ? "ok" : "FAIL", name);             \
        if (r > 0) pos += (size_t)r;                                \
        if (!(cond)) all_ok = 0;                                    \
    } while (0)

    static Term pool[64];
    static int pi = 0;
#define NEW() (&pool[pi++])

    /* 1. (λx:^1 i64. x) 1  :  i64 */
    pi = 0;
    Term *xv = NEW(); xv->kind = TERM_VAR; xv->name = "x";
    Term *lam = NEW(); lam->kind = TERM_LAM; lam->name = "x"; lam->q = Q_ONE; lam->ty = &I64_TY; lam->a = xv;
    Term *one = NEW(); one->kind = TERM_LIT; one->ival = 1;
    Term *app = NEW(); app->kind = TERM_APP; app->a = lam; app->b = one;
    T(qtt_check_closed(app, ty, sizeof ty, err, sizeof err) == 0 && strcmp(ty, "i64") == 0,
      "check ((λx:^1 i64. x) 1) : i64");

    /* 2. λx:^1 i64. (x + x)  →  linear violation */
    pi = 0;
    Term *x1 = NEW(); x1->kind = TERM_VAR; x1->name = "x";
    Term *x2 = NEW(); x2->kind = TERM_VAR; x2->name = "x";
    Term *add = NEW(); add->kind = TERM_BIN; add->op = BOP_ADD; add->a = x1; add->b = x2;
    Term *lam2 = NEW(); lam2->kind = TERM_LAM; lam2->name = "x"; lam2->q = Q_ONE; lam2->ty = &I64_TY; lam2->a = add;
    T(qtt_check_closed(lam2, ty, sizeof ty, err, sizeof err) != 0,
      "linear var used twice → error");

    /* 3. (1 + 2) : i64 */
    pi = 0;
    Term *l1 = NEW(); l1->kind = TERM_LIT; l1->ival = 1;
    Term *l2 = NEW(); l2->kind = TERM_LIT; l2->ival = 2;
    Term *add2 = NEW(); add2->kind = TERM_BIN; add2->op = BOP_ADD; add2->a = l1; add2->b = l2;
    T(qtt_check_closed(add2, ty, sizeof ty, err, sizeof err) == 0 && strcmp(ty, "i64") == 0,
      "check (1 + 2) : i64");

    /* 4. (1 == 2) : bool */
    pi = 0;
    Term *e1 = NEW(); e1->kind = TERM_LIT; e1->ival = 1;
    Term *e2 = NEW(); e2->kind = TERM_LIT; e2->ival = 2;
    Term *eq = NEW(); eq->kind = TERM_BIN; eq->op = BOP_EQ; eq->a = e1; eq->b = e2;
    T(qtt_check_closed(eq, ty, sizeof ty, err, sizeof err) == 0 && strcmp(ty, "bool") == 0,
      "check (1 == 2) : bool");

    /* 5. (1 + true) → error */
    pi = 0;
    Term *n1 = NEW(); n1->kind = TERM_LIT; n1->ival = 1;
    Term *b1 = NEW(); b1->kind = TERM_LIT; b1->bval = 1;
    Term *bad = NEW(); bad->kind = TERM_BIN; bad->op = BOP_ADD; bad->a = n1; bad->b = b1;
    T(qtt_check_closed(bad, ty, sizeof ty, err, sizeof err) != 0,
      "(1 + true) → error");

    /* 6. (λx:^ω i64. (x + x)) : (i64 -> i64)  (unrestricted OK) */
    pi = 0;
    Term *w1 = NEW(); w1->kind = TERM_VAR; w1->name = "x";
    Term *w2 = NEW(); w2->kind = TERM_VAR; w2->name = "x";
    Term *wadd = NEW(); wadd->kind = TERM_BIN; wadd->op = BOP_ADD; wadd->a = w1; wadd->b = w2;
    Term *wlam = NEW(); wlam->kind = TERM_LAM; wlam->name = "x"; wlam->q = Q_MANY; wlam->ty = &I64_TY; wlam->a = wadd;
    T(qtt_check_closed(wlam, ty, sizeof ty, err, sizeof err) == 0,
      "ω binder used twice is OK");

    return all_ok ? 0 : -1;
}
