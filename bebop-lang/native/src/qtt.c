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
        case TY_U8:
            return snprintf(out, cap, "u8");
        case TY_U32:
            return snprintf(out, cap, "u32");
        case TY_U64:
            return snprintf(out, cap, "u64");
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
        case TY_STRUCT: {
            int n = snprintf(out, cap, "struct{");
            for (int i = 0; i < t->nfields; i++) {
                char ft[64];
                qtt_ty_print(t->fields[i].ty, ft, sizeof ft);
                if ((size_t)n < cap) {
                    n += snprintf(out + n, cap - (size_t)n, "%s%s: %s",
                                  i ? ", " : "", t->fields[i].name, ft);
                }
            }
            if ((size_t)n < cap) {
                n += snprintf(out + n, cap - (size_t)n, "}");
            }
            return n;
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
    Ty i64 = {TY_I64, 0, Q_ZERO, 0, 0, 0, 0, 0, 0};
    Ty f = {TY_FIELD, 0xFFFFFFFF00000001ULL, Q_ZERO, 0, 0, 0, 0, 0, 0};
    Ty hv = {TY_HYPERVEC, 1024, Q_ZERO, 0, 0, 0, 0, 0, 0};
    Ty fn = {TY_FN, 0, Q_ZERO, 0, &i64, &i64, 0, 0, 0};
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

static Ty I64_TY = {TY_I64, 0, Q_ZERO, 0, 0, 0, 0, 0, 0};
static Ty BOOL_TY = {TY_BOOL, 0, Q_ZERO, 0, 0, 0, 0, 0, 0};

Ty *qtt_i64(void) {
    return &I64_TY;
}
Ty *qtt_bool(void) {
    return &BOOL_TY;
}

/* ─── Struct (record) self-test ─── */
int qtt_struct_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) {                                                          \
            pos += (size_t)r_;                                                 \
        }                                                                      \
        if (!c_) {                                                             \
            all_ok = 0;                                                        \
        }                                                                      \
    } while (0)

    static Term pool[64];
    static int pi = 0;
    pi = 0;
    Term *lx = &pool[pi++];
    memset(lx, 0, sizeof *lx);
    lx->kind = TERM_LIT;
    lx->ival = 3;
    Term *ly = &pool[pi++];
    memset(ly, 0, sizeof *ly);
    ly->kind = TERM_LIT;
    ly->ival = 4;
    Term *ltrue = &pool[pi++];
    memset(ltrue, 0, sizeof *ltrue);
    ltrue->kind = TERM_LIT;
    ltrue->bval = 1;

    static TyField pt_fields[2] = {{"x", &I64_TY}, {"y", &I64_TY}};
    static Ty pt_ty = {TY_STRUCT, 0, Q_ZERO, 0, 0, 0, 0, pt_fields, 2};

    static TermField lit_fields[2];
    lit_fields[0].name = "x";
    lit_fields[0].val = lx;
    lit_fields[1].name = "y";
    lit_fields[1].val = ly;
    Term *lit = &pool[pi++];
    memset(lit, 0, sizeof *lit);
    lit->kind = TERM_STRUCT;
    lit->ty = &pt_ty;
    lit->fields = lit_fields;
    lit->nfields = 2;

    Term *fx = &pool[pi++];
    memset(fx, 0, sizeof *fx);
    fx->kind = TERM_FIELD;
    fx->name = "x";
    fx->a = lit;

    char ty[128], err[256];
    A(qtt_check_closed(lit, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "struct{x: i64, y: i64}") == 0,
      "struct literal typechecks");
    A(qtt_check_closed(fx, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "i64") == 0,
      "field access typechecks");

    int k;
    long i;
    int b;
    A(qtt_eval(fx, &k, &i, &b, err, sizeof err) == 0 && i == 3, "eval field x == 3");

    static TermField bad_fields[1];
    bad_fields[0].name = "x";
    bad_fields[0].val = lx;
    Term *badlit = &pool[pi++];
    memset(badlit, 0, sizeof *badlit);
    badlit->kind = TERM_STRUCT;
    badlit->ty = &pt_ty;
    badlit->fields = bad_fields;
    badlit->nfields = 1;
    A(qtt_check_closed(badlit, ty, sizeof ty, err, sizeof err) != 0,
      "missing field rejected");

    static TermField wt_fields[2];
    wt_fields[0].name = "x";
    wt_fields[0].val = ltrue;
    wt_fields[1].name = "y";
    wt_fields[1].val = ly;
    Term *wtlit = &pool[pi++];
    memset(wtlit, 0, sizeof *wtlit);
    wtlit->kind = TERM_STRUCT;
    wtlit->ty = &pt_ty;
    wtlit->fields = wt_fields;
    wtlit->nfields = 2;
    A(qtt_check_closed(wtlit, ty, sizeof ty, err, sizeof err) != 0,
      "wrong field type rejected");

    Term *ns = &pool[pi++];
    memset(ns, 0, sizeof *ns);
    ns->kind = TERM_FIELD;
    ns->name = "x";
    ns->a = lx;
    A(qtt_check_closed(ns, ty, sizeof ty, err, sizeof err) != 0,
      "field on non-struct rejected");

    return all_ok ? 0 : -1;
}

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
        case TY_U8:
        case TY_U32:
        case TY_U64:
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
        case TY_STRUCT: {
            if (a->nfields != b->nfields) {
                return 0;
            }
            for (int i = 0; i < a->nfields; i++) {
                if (strcmp(a->fields[i].name, b->fields[i].name) != 0) {
                    return 0;
                }
                if (!ty_eq(a->fields[i].ty, b->fields[i].ty)) {
                    return 0;
                }
            }
            return 1;
        }
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
            *out = (t->op == BOP_EQ || t->op == BOP_LT || t->op == BOP_NE ||
                    t->op == BOP_LE || t->op == BOP_GE || t->op == BOP_GT)
                       ? &BOOL_TY
                       : &I64_TY;
            return 0;
        }
        case TERM_ANN: {
            if (check(c, t->a, t->ty, err, cap) != 0) {
                return -1;
            }
            *out = t->ty;
            return 0;
        }
        case TERM_IF: {
            Ty *ct = NULL;
            if (infer(c, t->a, &ct, err, cap) != 0) {
                return -1;
            }
            if (ct->kind != TY_BOOL) {
                snprintf(err, cap, "if condition must be bool");
                return -1;
            }
            Ty *tt = NULL, *et = NULL;
            if (infer(c, t->b, &tt, err, cap) != 0) {
                return -1;
            }
            if (infer(c, t->c, &et, err, cap) != 0) {
                return -1;
            }
            if (!ty_eq(tt, et)) {
                snprintf(err, cap, "if branches have different types");
                return -1;
            }
            *out = tt;
            return 0;
        }
        case TERM_LET: {
            Ty *vt = NULL;
            if (infer(c, t->a, &vt, err, cap) != 0) {
                return -1;
            }
            c->b[c->len].name = t->name;
            c->b[c->len].q = Q_MANY;
            c->b[c->len].ty = vt;
            c->b[c->len].used = 0;
            c->len++;
            int r = infer(c, t->b, out, err, cap);
            c->len--;
            return r;
        }
        case TERM_STRUCT: {
            if (!t->ty || t->ty->kind != TY_STRUCT) {
                snprintf(err, cap, "struct literal needs a struct type");
                return -1;
            }
            if (t->nfields != t->ty->nfields) {
                snprintf(err, cap, "struct literal field count mismatch");
                return -1;
            }
            for (int i = 0; i < t->ty->nfields; i++) {
                Term *val = NULL;
                for (int j = 0; j < t->nfields; j++) {
                    if (strcmp(t->fields[j].name, t->ty->fields[i].name) == 0) {
                        val = t->fields[j].val;
                        break;
                    }
                }
                if (!val) {
                    snprintf(err, cap, "missing field '%s'", t->ty->fields[i].name);
                    return -1;
                }
                if (check(c, val, t->ty->fields[i].ty, err, cap) != 0) {
                    return -1;
                }
            }
            *out = t->ty;
            return 0;
        }
        case TERM_FIELD: {
            Ty *bt = NULL;
            if (infer(c, t->a, &bt, err, cap) != 0) {
                return -1;
            }
            if (bt->kind != TY_STRUCT) {
                snprintf(err, cap, "field access on non-struct");
                return -1;
            }
            for (int i = 0; i < bt->nfields; i++) {
                if (strcmp(bt->fields[i].name, t->name) == 0) {
                    *out = bt->fields[i].ty;
                    return 0;
                }
            }
            snprintf(err, cap, "no field '%s'", t->name);
            return -1;
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

/* ═══ Evaluator (call-by-value, environment + closures) ═══ */

typedef struct Value Value;
typedef struct Env Env;
struct Value {
    int kind; /* 0=int, 1=bool, 2=closure, 3=struct, -1=error */
    long i;
    int b;
    const Term *lam; /* closure */
    Env *env;        /* closure env */
    const Ty *sty;          /* struct: the struct type */
    struct FieldValue *fv;  /* struct: field values */
    int nfv;
};
typedef struct FieldValue {
    const char *name;
    Value val;
} FieldValue;
struct Env {
    const char *name;
    Value val;
    Env *next;
};

static Value eval(const Term *t, Env *env) {
    Value v;
    memset(&v, 0, sizeof v);
    switch (t->kind) {
        case TERM_LIT:
            v.kind = t->bval ? 1 : 0;
            v.i = t->ival;
            v.b = t->bval;
            return v;
        case TERM_VAR:
            for (Env *e = env; e; e = e->next) {
                if (strcmp(e->name, t->name) == 0) {
                    return e->val;
                }
            }
            v.kind = -1;
            return v;
        case TERM_LAM:
            v.kind = 2;
            v.lam = t;
            v.env = env;
            return v;
        case TERM_APP: {
            Value f = eval(t->a, env);
            Value arg = eval(t->b, env);
            if (f.kind != 2) {
                v.kind = -1;
                return v;
            }
            Env e = {f.lam->name, arg, f.env};
            return eval(f.lam->a, &e);
        }
        case TERM_BIN: {
            Value l = eval(t->a, env);
            Value r = eval(t->b, env);
            v.kind = 0;
            switch (t->op) {
                case BOP_ADD: v.i = l.i + r.i; break;
                case BOP_SUB: v.i = l.i - r.i; break;
                case BOP_MUL: v.i = l.i * r.i; break;
                case BOP_EQ:  v.kind = 1; v.b = (l.i == r.i); break;
                case BOP_NE:  v.kind = 1; v.b = (l.i != r.i); break;
                case BOP_LT:  v.kind = 1; v.b = (l.i < r.i); break;
                case BOP_LE:  v.kind = 1; v.b = (l.i <= r.i); break;
                case BOP_GT:  v.kind = 1; v.b = (l.i > r.i); break;
                case BOP_GE:  v.kind = 1; v.b = (l.i >= r.i); break;
            }
            return v;
        }
        case TERM_IF: {
            Value c = eval(t->a, env);
            return eval(c.b ? t->b : t->c, env);
        }
        case TERM_LET: {
            Value x = eval(t->a, env);
            Env e = {t->name, x, env};
            return eval(t->b, &e);
        }
        case TERM_STRUCT: {
            static FieldValue fvs[64];
            v.kind = 3;
            v.sty = t->ty;
            v.fv = fvs;
            v.nfv = t->nfields;
            for (int i = 0; i < t->nfields; i++) {
                fvs[i].name = t->fields[i].name;
                fvs[i].val = eval(t->fields[i].val, env);
            }
            return v;
        }
        case TERM_FIELD: {
            Value base = eval(t->a, env);
            if (base.kind != 3) {
                v.kind = -1;
                return v;
            }
            for (int i = 0; i < base.nfv; i++) {
                if (strcmp(base.fv[i].name, t->name) == 0) {
                    return base.fv[i].val;
                }
            }
            v.kind = -1;
            return v;
        }
        default:
            v.kind = -1;
            return v;
    }
}

int qtt_eval(const Term *t, int *out_kind, long *out_i, int *out_b, char *err, size_t cap) {
    Value v = eval(t, NULL);
    if (v.kind < 0) {
        snprintf(err, cap, "evaluation error");
        return -1;
    }
    *out_kind = v.kind;
    *out_i = v.i;
    *out_b = v.b;
    return 0;
}

int qtt_eval_bound(const Term *t, const QttBind *binds, int n, int *out_kind,
                   long *out_i, int *out_b, char *err, size_t cap) {
    Env stack[64];
    for (int i = 0; i < n && i < 64; i++) {
        stack[i].name = binds[n - 1 - i].name;
        stack[i].val.kind = 0;
        stack[i].val.i = binds[n - 1 - i].i;
        stack[i].val.b = 0;
        stack[i].next = (i > 0) ? &stack[i - 1] : NULL;
    }
    Env *env = (n > 0) ? &stack[n - 1] : NULL;
    Value v = eval(t, env);
    if (v.kind < 0) {
        snprintf(err, cap, "evaluation error (unbound variable?)");
        return -1;
    }
    *out_kind = v.kind;
    *out_i = v.i;
    *out_b = v.b;
    return 0;
}

int qtt_eval_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[128];
    long i;
    int b, k;
#define E(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    static Term pool[64];
    static int pi = 0;
#define NEW() (&pool[pi++])

    /* 1. (λx:^ω i64. x + x) 21  →  42 */
    pi = 0;
    Term *x1 = NEW(); x1->kind = TERM_VAR; x1->name = "x";
    Term *x2 = NEW(); x2->kind = TERM_VAR; x2->name = "x";
    Term *add = NEW(); add->kind = TERM_BIN; add->op = BOP_ADD; add->a = x1; add->b = x2;
    Term *lam = NEW(); lam->kind = TERM_LAM; lam->name = "x"; lam->q = Q_MANY; lam->ty = &I64_TY; lam->a = add;
    Term *n21 = NEW(); n21->kind = TERM_LIT; n21->ival = 21;
    Term *app = NEW(); app->kind = TERM_APP; app->a = lam; app->b = n21;
    E(qtt_eval(app, &k, &i, &b, err, sizeof err) == 0 && i == 42,
      "eval ((λx.x+x) 21) == 42");

    /* 2. if (1 == 1) 10 else 20  →  10 */
    pi = 0;
    Term *e1 = NEW(); e1->kind = TERM_LIT; e1->ival = 1;
    Term *e2 = NEW(); e2->kind = TERM_LIT; e2->ival = 1;
    Term *eq = NEW(); eq->kind = TERM_BIN; eq->op = BOP_EQ; eq->a = e1; eq->b = e2;
    Term *t10 = NEW(); t10->kind = TERM_LIT; t10->ival = 10;
    Term *t20 = NEW(); t20->kind = TERM_LIT; t20->ival = 20;
    Term *iff = NEW(); iff->kind = TERM_IF; iff->a = eq; iff->b = t10; iff->c = t20;
    E(qtt_eval(iff, &k, &i, &b, err, sizeof err) == 0 && i == 10,
      "eval (if (1==1) 10 else 20) == 10");

    /* 3. let y = 2 in y * y  →  4 */
    pi = 0;
    Term *n2 = NEW(); n2->kind = TERM_LIT; n2->ival = 2;
    Term *y1 = NEW(); y1->kind = TERM_VAR; y1->name = "y";
    Term *y2 = NEW(); y2->kind = TERM_VAR; y2->name = "y";
    Term *mul = NEW(); mul->kind = TERM_BIN; mul->op = BOP_MUL; mul->a = y1; mul->b = y2;
    Term *let = NEW(); let->kind = TERM_LET; let->name = "y"; let->a = n2; let->b = mul;
    E(qtt_eval(let, &k, &i, &b, err, sizeof err) == 0 && i == 4,
      "eval (let y=2 in y*y) == 4");

    /* 4. typecheck + eval agreement: (λx:^1 i64. x + 1) 41  →  42 */
    pi = 0;
    Term *p1 = NEW(); p1->kind = TERM_VAR; p1->name = "x";
    Term *p2 = NEW(); p2->kind = TERM_LIT; p2->ival = 1;
    Term *padd = NEW(); padd->kind = TERM_BIN; padd->op = BOP_ADD; padd->a = p1; padd->b = p2;
    Term *plam = NEW(); plam->kind = TERM_LAM; plam->name = "x"; plam->q = Q_ONE; plam->ty = &I64_TY; plam->a = padd;
    Term *p41 = NEW(); p41->kind = TERM_LIT; p41->ival = 41;
    Term *papp = NEW(); papp->kind = TERM_APP; papp->a = plam; papp->b = p41;
    char ty[64];
    E(qtt_check_closed(papp, ty, sizeof ty, err, sizeof err) == 0 &&
      qtt_eval(papp, &k, &i, &b, err, sizeof err) == 0 && i == 42,
      "typecheck + eval ((λx:^1 i64. x+1) 41) == 42");

    return all_ok ? 0 : -1;
}
