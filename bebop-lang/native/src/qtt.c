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
        case TY_NAT:
            return snprintf(out, cap, "Nat");
        case TY_TYPE:
            return snprintf(out, cap, "Type");
        case TY_VAR:
            return snprintf(out, cap, "%s", t->x ? t->x : "_");
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
        case TY_ENUM: {
            int n = snprintf(out, cap, "enum{");
            for (int i = 0; i < t->nctors; i++) {
                if ((size_t)n < cap) {
                    if (t->ctors[i].payload) {
                        char pt[64];
                        qtt_ty_print(t->ctors[i].payload, pt, sizeof pt);
                        n += snprintf(out + n, cap - (size_t)n, "%s%s(%s)",
                                      i ? ", " : "", t->ctors[i].name, pt);
                    } else {
                        n += snprintf(out + n, cap - (size_t)n, "%s%s",
                                      i ? ", " : "", t->ctors[i].name);
                    }
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
        case TY_EQ: {
            char a[128];
            qtt_ty_print(t->eq_a, a, sizeof a);
            return snprintf(out, cap, "= %s", a);
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
    Ty i64 = {.kind = TY_I64};
    Ty f = {.kind = TY_FIELD, .n = 0xFFFFFFFF00000001ULL};
    Ty hv = {.kind = TY_HYPERVEC, .n = 1024};
    Ty fn = {.kind = TY_FN, .dom = &i64, .cod = &i64};
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

static Ty I64_TY = {.kind = TY_I64};
static Ty BOOL_TY = {.kind = TY_BOOL};
static Ty TYPE_TY = {.kind = TY_TYPE};
static Ty VOID_TY = {.kind = TY_VOID};
static Ty NAT_TY = {.kind = TY_NAT};

Ty *qtt_i64(void) {
    return &I64_TY;
}
Ty *qtt_bool(void) {
    return &BOOL_TY;
}

Ty *qtt_type(void) {
    return &TYPE_TY;
}

/* ─── Struct (record) self-test ─── */
int qtt_struct_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#undef A
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
    static Ty pt_ty = {.kind = TY_STRUCT, .fields = pt_fields, .nfields = 2};

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

/* ─── Enum (sum) + match self-test ─── */
int qtt_enum_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#undef A
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

    static Ctor opt_ctors[2] = {{"None", NULL}, {"Some", &I64_TY}};
    static Ty opt_ty = {.kind = TY_ENUM, .ctors = opt_ctors, .nctors = 2};

    Term *none = &pool[pi++];
    memset(none, 0, sizeof *none);
    none->kind = TERM_ENUM_CTOR;
    none->name = "None";
    none->ty = &opt_ty;

    Term *n42 = &pool[pi++];
    memset(n42, 0, sizeof *n42);
    n42->kind = TERM_LIT;
    n42->ival = 42;
    Term *some42 = &pool[pi++];
    memset(some42, 0, sizeof *some42);
    some42->kind = TERM_ENUM_CTOR;
    some42->name = "Some";
    some42->ty = &opt_ty;
    some42->a = n42;

    Term *z = &pool[pi++];
    memset(z, 0, sizeof *z);
    z->kind = TERM_LIT;
    z->ival = 0;
    Term *xvar = &pool[pi++];
    memset(xvar, 0, sizeof *xvar);
    xvar->kind = TERM_VAR;
    xvar->name = "x";
    static MatchArm arms[2];
    arms[0].ctor = "None";
    arms[0].var = NULL;
    arms[0].body = z;
    arms[1].ctor = "Some";
    arms[1].var = "x";
    arms[1].body = xvar;
    Term *m = &pool[pi++];
    memset(m, 0, sizeof *m);
    m->kind = TERM_MATCH;
    m->a = some42;
    m->arms = arms;
    m->narms = 2;

    char ty[128], err[256];
    A(qtt_check_closed(some42, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "enum{None, Some(i64)}") == 0,
      "ctor typechecks");
    A(qtt_check_closed(m, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "i64") == 0,
      "match typechecks");

    int k;
    long i;
    int b;
    A(qtt_eval(m, &k, &i, &b, err, sizeof err) == 0 && i == 42,
      "eval match Some(42) == 42");

    Term *mn = &pool[pi++];
    memset(mn, 0, sizeof *mn);
    mn->kind = TERM_MATCH;
    mn->a = none;
    mn->arms = arms;
    mn->narms = 2;
    A(qtt_eval(mn, &k, &i, &b, err, sizeof err) == 0 && i == 0,
      "eval match None == 0");

    Term *m2 = &pool[pi++];
    memset(m2, 0, sizeof *m2);
    m2->kind = TERM_MATCH;
    m2->a = some42;
    m2->arms = arms;
    m2->narms = 1;
    A(qtt_check_closed(m2, ty, sizeof ty, err, sizeof err) != 0,
      "missing arm rejected");

    Term *badct = &pool[pi++];
    memset(badct, 0, sizeof *badct);
    badct->kind = TERM_ENUM_CTOR;
    badct->name = "None";
    badct->ty = &opt_ty;
    badct->a = n42;
    A(qtt_check_closed(badct, ty, sizeof ty, err, sizeof err) != 0,
      "unit ctor payload rejected");

    return all_ok ? 0 : -1;
}

/* ─── Dependent types (universe + type substitution) self-test ─── */
int qtt_dep_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#undef A
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

    static Ty tyvar_A = {.kind = TY_VAR, .x = "A"};

    Term *xvar = &pool[pi++];
    memset(xvar, 0, sizeof *xvar);
    xvar->kind = TERM_VAR;
    xvar->name = "x";
    Term *inner = &pool[pi++];
    memset(inner, 0, sizeof *inner);
    inner->kind = TERM_LAM;
    inner->name = "x";
    inner->q = Q_ONE;
    inner->ty = &tyvar_A;
    inner->a = xvar;
    Term *id = &pool[pi++];
    memset(id, 0, sizeof *id);
    id->kind = TERM_LAM;
    id->name = "A";
    id->q = Q_ZERO;
    id->ty = &TYPE_TY;
    id->a = inner;

    Term *ti64 = &pool[pi++];
    memset(ti64, 0, sizeof *ti64);
    ti64->kind = TERM_TYPE;
    ti64->ty = &I64_TY;
    Term *app = &pool[pi++];
    memset(app, 0, sizeof *app);
    app->kind = TERM_APP;
    app->a = id;
    app->b = ti64;

    char ty[128], err[256];
    A(qtt_check_closed(id, ty, sizeof ty, err, sizeof err) == 0,
      "polymorphic id typechecks");
    A(qtt_check_closed(app, ty, sizeof ty, err, sizeof err) == 0 &&
          strstr(ty, "i64") != NULL,
      "id(i64) : i64 -> i64 (dependent substitution)");

    Term *n5 = &pool[pi++];
    memset(n5, 0, sizeof *n5);
    n5->kind = TERM_LIT;
    n5->ival = 5;
    Term *bad = &pool[pi++];
    memset(bad, 0, sizeof *bad);
    bad->kind = TERM_APP;
    bad->a = id;
    bad->b = n5;
    A(qtt_check_closed(bad, ty, sizeof ty, err, sizeof err) != 0,
      "id(5) rejected (not a type)");

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

/* Capture-avoiding-free substitution of a type variable (var → repl). */
static Ty *ty_subst(Ty *t, const char *var, Ty *repl) {
    if (!t) {
        return NULL;
    }
    switch (t->kind) {
        case TY_VAR:
            if (t->x && strcmp(t->x, var) == 0) {
                return repl;
            }
            return t;
        case TY_FN:
        case TY_PI: {
            Ty *d = ty_subst(t->dom, var, repl);
            Ty *c = ty_subst(t->cod, var, repl);
            if (d == t->dom && c == t->cod) {
                return t;
            }
            Ty *r = ty_alloc(t->kind);
            r->n = t->n;
            r->q = t->q;
            r->x = t->x;
            r->dom = d;
            r->cod = c;
            return r;
        }
        case TY_VEC: {
            Ty *e = ty_subst(t->elem, var, repl);
            if (e == t->elem) {
                return t;
            }
            Ty *r = ty_alloc(TY_VEC);
            r->n = t->n;
            r->elem = e;
            return r;
        }
        default:
            return t;
    }
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
        case TY_TYPE:
            return 1;
        case TY_VAR:
            return (a->x && b->x) ? strcmp(a->x, b->x) == 0 : a->x == b->x;
        case TY_I64:
        case TY_U8:
        case TY_U32:
        case TY_U64:
        case TY_F64:
        case TY_BOOL:
        case TY_VOID:
        case TY_NAT:
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
        case TY_ENUM: {
            if (a->nctors != b->nctors) {
                return 0;
            }
            for (int i = 0; i < a->nctors; i++) {
                if (strcmp(a->ctors[i].name, b->ctors[i].name) != 0) {
                    return 0;
                }
                if ((a->ctors[i].payload == NULL) != (b->ctors[i].payload == NULL)) {
                    return 0;
                }
                if (a->ctors[i].payload &&
                    !ty_eq(a->ctors[i].payload, b->ctors[i].payload)) {
                    return 0;
                }
            }
            return 1;
        }
        case TY_EQ:
            /* Propositional equality: A l₁ = A l₂ are definitionally equal
             * iff the ambient types match and both sides are β-convertible. */
            return ty_eq(a->eq_a, b->eq_a) &&
                   qtt_conv(a->eq_l, b->eq_l) &&
                   qtt_conv(a->eq_r, b->eq_r);
    }
    return 0;
}

static int infer(Ctx *c, const Term *t, Ty **out, char *err, size_t cap);
static int check(Ctx *c, const Term *t, const Ty *want, char *err, size_t cap);

/* Transport: rewrite `from` → `to` in the term sides of a type. Only TY_EQ
 * embeds terms, so this is the complete term-rewrite over types. Returns the
 * (possibly unchanged) type, or NULL on pool exhaustion. */
static Ty *rewrite_eq_ty(Ty *h, Term *from, Term *to) {
    if (!h || h->kind != TY_EQ) {
        return h; /* non-equality motive: transport is the identity */
    }
    Term *l = qtt_conv(h->eq_l, from) ? to : h->eq_l;
    Term *r = qtt_conv(h->eq_r, from) ? to : h->eq_r;
    if (l == h->eq_l && r == h->eq_r) {
        return h;
    }
    Ty *out = ty_alloc(TY_EQ);
    if (!out) {
        return NULL;
    }
    out->eq_a = h->eq_a;
    out->eq_l = l;
    out->eq_r = r;
    return out;
}

/* ═══ Persistent term pool (for substitution results stored in TY_EQ types) ═══
 * qtt_conv resets the kernel scratch pool (kpool) every call, so terms that
 * must outlive a conversion — e.g. the `f n` / `g n` sides of an induction
 * motive — are substituted into THIS pool instead, reset only per proof batch. */
static Term ppool[512];
static int ppi = 0;

static Term *pnew(void) {
    if (ppi >= (int)(sizeof ppool / sizeof ppool[0])) {
        return NULL;
    }
    Term *t = &ppool[ppi++];
    memset(t, 0, sizeof *t);
    return t;
}

/* Capture-avoiding substitution into the persistent pool (same semantics as
 * qtt_subst, different allocator). */
static Term *subst_p(const Term *t, const char *name, const Term *v) {
    if (!t) {
        return NULL;
    }
    Term *o = pnew();
    if (!o) {
        return NULL;
    }
    *o = *t;
    switch (t->kind) {
        case TERM_VAR:
            if (t->name && name && strcmp(t->name, name) == 0) {
                *o = *v;
            }
            return o;
        case TERM_LIT:
        case TERM_TYPE:
        case TERM_IO:
        case TERM_NAT_Z:
            return o;
        case TERM_LAM:
            if (t->name && name && strcmp(t->name, name) != 0) {
                o->a = subst_p(t->a, name, v);
            }
            return o;
        case TERM_APP:
        case TERM_BIN:
            o->a = subst_p(t->a, name, v);
            o->b = subst_p(t->b, name, v);
            return o;
        case TERM_NAT_S:
        case TERM_FIELD:
        case TERM_ENUM_CTOR:
            o->a = subst_p(t->a, name, v);
            return o;
        case TERM_NAT_REC:
        case TERM_SUBST:
            o->a = subst_p(t->a, name, v);
            o->b = subst_p(t->b, name, v);
            o->c = subst_p(t->c, name, v);
            return o;
        case TERM_NAT_IND:
            o->a = subst_p(t->a, name, v);
            o->b = subst_p(t->b, name, v);
            o->c = subst_p(t->c, name, v);
            o->d = subst_p(t->d, name, v);
            return o;
        case TERM_CONG:
            o->a = subst_p(t->a, name, v);
            o->b = subst_p(t->b, name, v);
            return o;
        case TERM_EQ_TYPE:
            o->a = subst_p(t->a, name, v);
            o->b = subst_p(t->b, name, v);
            return o;
        case TERM_REFL:
            o->a = subst_p(t->a, name, v);
            return o;
        case TERM_IF:
            o->a = subst_p(t->a, name, v);
            o->b = subst_p(t->b, name, v);
            o->c = subst_p(t->c, name, v);
            return o;
        case TERM_LET:
            o->a = subst_p(t->a, name, v);
            if (t->name && name && strcmp(t->name, name) != 0) {
                o->b = subst_p(t->b, name, v);
            }
            return o;
        case TERM_ANN:
            o->a = subst_p(t->a, name, v);
            return o;
        case TERM_MATCH:
        case TERM_STRUCT:
            return o; /* not needed for Nat motives; shallow copy */
    }
    return o;
}

/* Apply an induction motive LAM (λn. TERM_EQ_TYPE(f n, g n)) to a Nat argument,
 * producing the TY_EQ proposition `f arg = g arg`. NULL on malformed motive or
 * pool exhaustion. */
static Ty *motive_apply(const Term *motive, const Term *arg) {
    if (!motive || motive->kind != TERM_LAM || !motive->a ||
        motive->a->kind != TERM_EQ_TYPE) {
        return NULL;
    }
    Term *l = subst_p(motive->a->a, motive->name, arg);
    Term *r = subst_p(motive->a->b, motive->name, arg);
    Ty *eq = ty_alloc(TY_EQ);
    if (!eq) {
        return NULL;
    }
    eq->eq_a = &NAT_TY;
    eq->eq_l = l;
    eq->eq_r = r;
    return eq;
}

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
            *out = ft->cod;
            if (t->b->kind == TERM_TYPE && ft->kind == TY_PI && ft->x) {
                *out = ty_subst(ft->cod, ft->x, t->b->ty);
            }
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
        case TERM_REFL: {
            /* refl : a = a — the sole constructor of propositional equality.
             * infers the ambient type of `a`, then forms `a = a`. */
            Ty *a_ty = NULL;
            if (infer(c, t->a, &a_ty, err, cap) != 0) {
                return -1;
            }
            Ty *eq = ty_alloc(TY_EQ);
            if (!eq) {
                snprintf(err, cap, "type pool exhausted");
                return -1;
            }
            eq->eq_a = a_ty;
            eq->eq_l = t->a;
            eq->eq_r = t->a;
            *out = eq;
            return 0;
        }
        case TERM_SUBST: {
            /* subst : l = r -> H[l] -> H[r]  (the equality eliminator).
             * t->a : proof of (l = r); t->b : inhabitant of H (which mentions
             * l); the result is H with l rewritten to r. */
            Ty *eq_ty = NULL;
            if (infer(c, t->a, &eq_ty, err, cap) != 0) {
                return -1;
            }
            if (eq_ty->kind != TY_EQ) {
                snprintf(err, cap, "subst needs an equality proof");
                return -1;
            }
            Ty *h_ty = NULL;
            if (infer(c, t->b, &h_ty, err, cap) != 0) {
                return -1;
            }
            *out = rewrite_eq_ty(h_ty, eq_ty->eq_l, eq_ty->eq_r);
            if (!*out) {
                snprintf(err, cap, "type pool exhausted");
                return -1;
            }
            return 0;
        }
        case TERM_NAT_Z:
            *out = &NAT_TY;
            return 0;
        case TERM_NAT_S:
            if (check(c, t->a, &NAT_TY, err, cap) != 0) {
                return -1;
            }
            *out = &NAT_TY;
            return 0;
        case TERM_NAT_REC: {
            /* nat_rec base step target : P, where
             *   base : P,  step : Nat -> P -> P,  target : Nat */
            Ty *p = NULL;
            if (infer(c, t->a, &p, err, cap) != 0) {
                return -1;
            }
            Ty *st = NULL;
            if (infer(c, t->b, &st, err, cap) != 0) {
                return -1;
            }
            if ((st->kind != TY_FN && st->kind != TY_PI) ||
                !ty_eq(st->dom, &NAT_TY)) {
                snprintf(err, cap, "nat_rec step must be Nat -> P -> P");
                return -1;
            }
            if ((st->cod->kind != TY_FN && st->cod->kind != TY_PI) ||
                !ty_eq(st->cod->dom, p) || !ty_eq(st->cod->cod, p)) {
                snprintf(err, cap, "nat_rec step type mismatch");
                return -1;
            }
            if (check(c, t->c, &NAT_TY, err, cap) != 0) {
                return -1;
            }
            *out = p;
            return 0;
        }
        case TERM_EQ_TYPE:
            /* "a = b" is a proposition: a term of the universe Type. Its
             * interpretation as an actual proposition type is done by
             * motive_apply (nat_ind). */
            *out = &TYPE_TY;
            return 0;
        case TERM_CONG: {
            /* congr f p : f a = f b, from p : a = b and f : A -> B. */
            Ty *ft = NULL;
            if (infer(c, t->a, &ft, err, cap) != 0) {
                return -1;
            }
            if (ft->kind != TY_FN && ft->kind != TY_PI) {
                snprintf(err, cap, "congr needs a function");
                return -1;
            }
            Ty *pt = NULL;
            if (infer(c, t->b, &pt, err, cap) != 0) {
                return -1;
            }
            if (pt->kind != TY_EQ || !ty_eq(pt->eq_a, ft->dom)) {
                snprintf(err, cap, "congr equality domain mismatch");
                return -1;
            }
            Term *fa = pnew();
            Term *fb = pnew();
            Ty *eq = ty_alloc(TY_EQ);
            if (!fa || !fb || !eq) {
                snprintf(err, cap, "pool exhausted");
                return -1;
            }
            fa->kind = TERM_APP;
            fa->a = t->a;
            fa->b = pt->eq_l;
            fb->kind = TERM_APP;
            fb->a = t->a;
            fb->b = pt->eq_r;
            eq->eq_a = ft->cod;
            eq->eq_l = fa;
            eq->eq_r = fb;
            *out = eq;
            return 0;
        }
        case TERM_NAT_IND: {
            /* nat_ind motive base step target : motive target, where
             *   motive : λn. (f n = g n)  (a LAM body = TERM_EQ_TYPE)
             *   base   : motive Z
             *   step   : (k : Nat) -> motive k -> motive (S k)
             *   target : Nat */
            const Term *motive = t->d;
            if (!motive || motive->kind != TERM_LAM || !motive->a ||
                motive->a->kind != TERM_EQ_TYPE) {
                snprintf(err, cap, "nat_ind motive must be λn. (f n = g n)");
                return -1;
            }
            Term *zterm = pnew();
            if (!zterm) {
                snprintf(err, cap, "pool exhausted");
                return -1;
            }
            zterm->kind = TERM_NAT_Z;
            Ty *mot_z = motive_apply(motive, zterm);
            if (!mot_z || check(c, t->a, mot_z, err, cap) != 0) {
                if (mot_z) {
                    return -1;
                }
                snprintf(err, cap, "nat_ind base type error");
                return -1;
            }
            if (t->b->kind != TERM_LAM || !ty_eq(t->b->ty, &NAT_TY)) {
                snprintf(err, cap, "nat_ind step must be λk:Nat. ...");
                return -1;
            }
            Term *kvar = pnew();
            Term *sk = pnew();
            if (!kvar || !sk) {
                snprintf(err, cap, "pool exhausted");
                return -1;
            }
            kvar->kind = TERM_VAR;
            kvar->name = t->b->name;
            sk->kind = TERM_NAT_S;
            sk->a = kvar;
            Ty *mot_k = motive_apply(motive, kvar);
            Ty *mot_sk = motive_apply(motive, sk);
            Ty *cod = ty_alloc(TY_FN);
            if (!mot_k || !mot_sk || !cod) {
                snprintf(err, cap, "pool exhausted");
                return -1;
            }
            cod->dom = mot_k;
            cod->cod = mot_sk;
            c->b[c->len].name = t->b->name;
            c->b[c->len].q = t->b->q;
            c->b[c->len].ty = &NAT_TY;
            c->b[c->len].used = 0;
            c->len++;
            int r = check(c, t->b->a, cod, err, cap);
            c->len--;
            if (r != 0) {
                return r;
            }
            if (check(c, t->c, &NAT_TY, err, cap) != 0) {
                return -1;
            }
            Ty *res = motive_apply(motive, t->c);
            if (!res) {
                snprintf(err, cap, "pool exhausted");
                return -1;
            }
            *out = res;
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
        case TERM_ENUM_CTOR: {
            if (!t->ty || t->ty->kind != TY_ENUM) {
                snprintf(err, cap, "enum ctor needs an enum type");
                return -1;
            }
            for (int i = 0; i < t->ty->nctors; i++) {
                Ctor *ct = &t->ty->ctors[i];
                if (strcmp(ct->name, t->name) == 0) {
                    if (ct->payload) {
                        if (check(c, t->a, ct->payload, err, cap) != 0) {
                            return -1;
                        }
                    } else if (t->a) {
                        snprintf(err, cap, "unit ctor takes no payload");
                        return -1;
                    }
                    *out = t->ty;
                    return 0;
                }
            }
            snprintf(err, cap, "no ctor '%s'", t->name);
            return -1;
        }
        case TERM_MATCH: {
            Ty *st = NULL;
            if (infer(c, t->a, &st, err, cap) != 0) {
                return -1;
            }
            if (st->kind != TY_ENUM) {
                snprintf(err, cap, "match scrutinee must be an enum");
                return -1;
            }
            Ty *result = NULL;
            for (int i = 0; i < st->nctors; i++) {
                MatchArm *arm = NULL;
                for (int j = 0; j < t->narms; j++) {
                    if (strcmp(t->arms[j].ctor, st->ctors[i].name) == 0) {
                        arm = &t->arms[j];
                        break;
                    }
                }
                if (!arm) {
                    snprintf(err, cap, "match missing arm for '%s'", st->ctors[i].name);
                    return -1;
                }
                Ty *bt = NULL;
                if (st->ctors[i].payload) {
                    if (!arm->var) {
                        snprintf(err, cap, "arm needs a bound var");
                        return -1;
                    }
                    c->b[c->len].name = arm->var;
                    c->b[c->len].q = Q_MANY;
                    c->b[c->len].ty = st->ctors[i].payload;
                    c->b[c->len].used = 0;
                    c->len++;
                    int r = infer(c, arm->body, &bt, err, cap);
                    c->len--;
                    if (r != 0) {
                        return r;
                    }
                } else {
                    if (infer(c, arm->body, &bt, err, cap) != 0) {
                        return -1;
                    }
                }
                if (!result) {
                    result = bt;
                } else if (!ty_eq(result, bt)) {
                    snprintf(err, cap, "match arms differ in type");
                    return -1;
                }
            }
            *out = result;
            return 0;
        }
        case TERM_TYPE: {
            *out = &TYPE_TY;
            return 0;
        }
        case TERM_IO: {
            *out = &VOID_TY;
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

/* ═══ Evaluator (call-by-value, environment + closures) ═══ */

typedef struct Value Value;
typedef struct Env Env;
struct Value {
    int kind; /* 0=int, 1=bool, 2=closure, 3=struct, 4=enum, -1=error */
    long i;
    int b;
    const Term *lam; /* closure */
    Env *env;        /* closure env */
    const Ty *sty;          /* struct/enum: the type */
    struct FieldValue *fv;  /* struct: field values */
    int nfv;
    const char *ctor; /* enum: constructor name */
    Value *payload;   /* enum: payload value (pointer) */
    int has_payload;  /* enum: 1 if payload present */
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
        case TERM_ENUM_CTOR: {
            static Value payload_storage;
            v.kind = 4;
            v.sty = t->ty;
            v.ctor = t->name;
            v.has_payload = (t->a != NULL);
            if (t->a) {
                payload_storage = eval(t->a, env);
                v.payload = &payload_storage;
            }
            return v;
        }
        case TERM_MATCH: {
            Value scrut = eval(t->a, env);
            if (scrut.kind != 4) {
                v.kind = -1;
                return v;
            }
            for (int j = 0; j < t->narms; j++) {
                if (strcmp(t->arms[j].ctor, scrut.ctor) == 0) {
                    if (scrut.has_payload) {
                        Env e = {t->arms[j].var, *scrut.payload, env};
                        return eval(t->arms[j].body, &e);
                    }
                    return eval(t->arms[j].body, env);
                }
            }
            v.kind = -1;
            return v;
        }
        case TERM_IO:
            v.kind = 0; /* unit */
            v.i = 0;
            return v;
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

int qtt_term_has_io(const Term *t) {
    if (!t) {
        return 0;
    }
    if (t->kind == TERM_IO) {
        return 1;
    }
    if (qtt_term_has_io(t->a) || qtt_term_has_io(t->b) ||
        qtt_term_has_io(t->c)) {
        return 1;
    }
    for (int i = 0; i < t->nfields; i++) {
        if (qtt_term_has_io(t->fields[i].val)) {
            return 1;
        }
    }
    for (int i = 0; i < t->narms; i++) {
        if (qtt_term_has_io(t->arms[i].body)) {
            return 1;
        }
    }
    return 0;
}

int qtt_effect_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
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
    static int pi = 0;
    pi = 0;
    Term *l1 = &pool[pi++]; memset(l1, 0, sizeof *l1); l1->kind = TERM_LIT; l1->ival = 1;
    Term *l2 = &pool[pi++]; memset(l2, 0, sizeof *l2); l2->kind = TERM_LIT; l2->ival = 2;
    Term *add = &pool[pi++]; memset(add, 0, sizeof *add);
    add->kind = TERM_BIN; add->op = BOP_ADD; add->a = l1; add->b = l2;
    A(!qtt_term_has_io(add), "pure term (1+2) has no IO");

    Term *io = &pool[pi++]; memset(io, 0, sizeof *io); io->kind = TERM_IO;
    A(qtt_term_has_io(io), "TERM_IO has IO");

    Term *nested = &pool[pi++]; memset(nested, 0, sizeof *nested);
    nested->kind = TERM_BIN; nested->op = BOP_ADD; nested->a = l1; nested->b = io;
    A(qtt_term_has_io(nested), "nested IO detected");

    return all_ok ? 0 : -1;
}

/* ═══ Proof kernel: definitional equality (conversion) ═══
 *
 * The Lean-4-style kernel is SMALL on purpose: a bounded term pool, one
 * capture-avoiding substitution, one β-normalizer, one conversion check.
 * Every proof term the checker accepts is checked against exactly this core.
 */
#define KERNEL_POOL 4096
static Term kpool[KERNEL_POOL];
static int kpi = 0;

void qtt_term_pool_reset(void) { kpi = 0; }

static Term *knew(void) {
    if (kpi >= KERNEL_POOL) return NULL;
    Term *t = &kpool[kpi++];
    memset(t, 0, sizeof *t);
    return t;
}

/* Capture-avoiding substitution: [v/name]t. A binder shadowing `name` blocks
 * the substitution (no capture). Returns a fresh kernel term or NULL. */
Term *qtt_subst(const Term *t, const char *name, const Term *v) {
    if (!t) return NULL;
    Term *o = knew();
    if (!o) return NULL;
    *o = *t; /* shallow copy, children overwritten below */
    switch (t->kind) {
        case TERM_VAR:
            if (t->name && name && strcmp(t->name, name) == 0) *o = *v;
            return o;
        case TERM_LIT:
        case TERM_TYPE:
        case TERM_IO:
            return o;
        case TERM_LAM:
            if (t->name && name && strcmp(t->name, name) != 0) {
                o->a = qtt_subst(t->a, name, v);
            }
            return o;
        case TERM_APP:
        case TERM_BIN:
            o->a = qtt_subst(t->a, name, v);
            o->b = qtt_subst(t->b, name, v);
            return o;
        case TERM_ANN:
            o->a = qtt_subst(t->a, name, v);
            return o;
        case TERM_IF:
            o->a = qtt_subst(t->a, name, v);
            o->b = qtt_subst(t->b, name, v);
            o->c = qtt_subst(t->c, name, v);
            return o;
        case TERM_LET:
            o->a = qtt_subst(t->a, name, v);
            if (t->name && name && strcmp(t->name, name) != 0) {
                o->b = qtt_subst(t->b, name, v);
            }
            return o;
        case TERM_FIELD:
            o->a = qtt_subst(t->a, name, v);
            return o;
        case TERM_ENUM_CTOR:
            o->a = qtt_subst(t->a, name, v);
            return o;
        case TERM_MATCH:
            o->a = qtt_subst(t->a, name, v);
            for (int i = 0; i < t->narms; i++) {
                if (t->arms[i].var && name && strcmp(t->arms[i].var, name) != 0) {
                    o->arms[i].body = qtt_subst(t->arms[i].body, name, v);
                }
            }
            return o;
        case TERM_STRUCT:
            for (int i = 0; i < t->nfields; i++) {
                o->fields[i].val = qtt_subst(t->fields[i].val, name, v);
            }
            return o;
        case TERM_REFL:
            o->a = qtt_subst(t->a, name, v);
            return o;
        case TERM_SUBST:
            o->a = qtt_subst(t->a, name, v);
            o->b = qtt_subst(t->b, name, v);
            o->c = qtt_subst(t->c, name, v);
            return o;
        case TERM_NAT_Z:
            return o;
        case TERM_NAT_S:
            o->a = qtt_subst(t->a, name, v);
            return o;
        case TERM_NAT_REC:
            o->a = qtt_subst(t->a, name, v);
            o->b = qtt_subst(t->b, name, v);
            o->c = qtt_subst(t->c, name, v);
            return o;
        case TERM_NAT_IND:
            o->a = qtt_subst(t->a, name, v);
            o->b = qtt_subst(t->b, name, v);
            o->c = qtt_subst(t->c, name, v);
            o->d = qtt_subst(t->d, name, v);
            return o;
        case TERM_CONG:
            o->a = qtt_subst(t->a, name, v);
            o->b = qtt_subst(t->b, name, v);
            return o;
        case TERM_EQ_TYPE:
            o->a = qtt_subst(t->a, name, v);
            o->b = qtt_subst(t->b, name, v);
            return o;
    }
    return o;
}

/* β-normalizer: weak-head reduce applications, recurse into subterms. */
static Term *norm_rec(const Term *t) {
    if (!t) return NULL;
    Term *o = knew();
    if (!o) return NULL;
    *o = *t;
    switch (t->kind) {
        case TERM_LIT:
        case TERM_VAR:
        case TERM_TYPE:
        case TERM_IO:
            return o;
        case TERM_REFL:
            o->a = norm_rec(t->a);
            return o;
        case TERM_SUBST:
            o->a = norm_rec(t->a);
            o->b = norm_rec(t->b);
            o->c = norm_rec(t->c);
            return o;
        case TERM_NAT_Z:
            return o; /* already normal */
        case TERM_NAT_S:
            o->a = norm_rec(t->a);
            return o;
        case TERM_NAT_IND:
            o->a = norm_rec(t->a);
            o->b = norm_rec(t->b);
            o->c = norm_rec(t->c);
            o->d = norm_rec(t->d);
            return o;
        case TERM_CONG:
            o->a = norm_rec(t->a);
            o->b = norm_rec(t->b);
            return o;
        case TERM_EQ_TYPE:
            o->a = norm_rec(t->a);
            o->b = norm_rec(t->b);
            return o;
        case TERM_NAT_REC: {
            /* definitional reduction of the recursor:
             *   nat_rec b s Z      → b
             *   nat_rec b s (S k)  → (s k) (nat_rec b s k)  */
            Term *tg = norm_rec(t->c);
            if (!tg) {
                return NULL;
            }
            if (tg->kind == TERM_NAT_Z) {
                return norm_rec(t->a);
            }
            if (tg->kind == TERM_NAT_S) {
                Term *rec = knew();
                if (!rec) return NULL;
                rec->kind = TERM_NAT_REC;
                rec->a = t->a;
                rec->b = t->b;
                rec->c = tg->a;
                Term *app1 = knew();
                if (!app1) return NULL;
                app1->kind = TERM_APP;
                app1->a = t->b; /* step */
                app1->b = tg->a; /* k */
                Term *app2 = knew();
                if (!app2) return NULL;
                app2->kind = TERM_APP;
                app2->a = app1;
                app2->b = rec;
                return norm_rec(app2);
            }
            o->a = norm_rec(t->a);
            o->b = norm_rec(t->b);
            o->c = tg;
            return o;
        }
        case TERM_APP: {
            Term *f = norm_rec(t->a);
            if (!f) return NULL;
            if (f->kind == TERM_LAM) {
                /* β: (λx.body) arg → body[arg/x], re-normalize */
                Term *s = qtt_subst(f->a, f->name, t->b);
                return norm_rec(s);
            }
            o->a = f;
            o->b = norm_rec(t->b);
            return o;
        }
        case TERM_LAM:
            o->a = norm_rec(t->a);
            return o;
        case TERM_BIN: {
            Term *la = norm_rec(t->a);
            Term *lb = norm_rec(t->b);
            /* δ-reduction: constant-fold closed arithmetic (the kernel's
             * definitional reduction of primitive operators, as Lean does for
             * closed numerals). Only when both sides are integer literals. */
            if (la && lb && la->kind == TERM_LIT && lb->kind == TERM_LIT &&
                !la->bval && !lb->bval) {
                long l = la->ival, r = lb->ival;
                /* Reset `o` to a scalar literal: constant-folding collapses a
                 * BIN node into a LIT node (δ-reduction produces a value). */
                o->kind = TERM_LIT;
                o->op = 0;
                o->a = o->b = o->c = NULL;
                o->ival = 0;
                o->bval = 0;
                switch (t->op) {
                    case BOP_ADD: o->ival = l + r; return o;
                    case BOP_SUB: o->ival = l - r; return o;
                    case BOP_MUL: o->ival = l * r; return o;
                    case BOP_EQ:  o->bval = (l == r); return o;
                    case BOP_NE:  o->bval = (l != r); return o;
                    case BOP_LT:  o->bval = (l < r);  return o;
                    case BOP_LE:  o->bval = (l <= r); return o;
                    case BOP_GT:  o->bval = (l > r);  return o;
                    case BOP_GE:  o->bval = (l >= r); return o;
                }
            }
            o->a = la;
            o->b = lb;
            return o;
        }
        case TERM_ANN:
            o->a = norm_rec(t->a);
            return o;
        case TERM_IF:
            o->a = norm_rec(t->a);
            o->b = norm_rec(t->b);
            o->c = norm_rec(t->c);
            return o;
        case TERM_LET:
            o->a = norm_rec(t->a);
            o->b = norm_rec(t->b);
            return o;
        case TERM_FIELD:
            o->a = norm_rec(t->a);
            return o;
        case TERM_ENUM_CTOR:
            o->a = norm_rec(t->a);
            return o;
        case TERM_MATCH:
            o->a = norm_rec(t->a);
            for (int i = 0; i < t->narms; i++) {
                o->arms[i].body = norm_rec(t->arms[i].body);
            }
            return o;
        case TERM_STRUCT:
            for (int i = 0; i < t->nfields; i++) {
                o->fields[i].val = norm_rec(t->fields[i].val);
            }
            return o;
    }
    return o;
}

Term *qtt_norm(const Term *t) { return norm_rec(t); }

/* Structural equality on normalized terms (α by identical binder names). */
static int conv_rec(const Term *a, const Term *b) {
    if (!a || !b) return a == b;
    if (a->kind != b->kind) return 0;
    switch (a->kind) {
        case TERM_LIT:
            return a->ival == b->ival && a->bval == b->bval;
        case TERM_VAR:
            return a->name && b->name && strcmp(a->name, b->name) == 0;
        case TERM_TYPE:
        case TERM_IO:
            return 1;
        case TERM_REFL:
            return conv_rec(a->a, b->a);
        case TERM_SUBST:
            return conv_rec(a->a, b->a) && conv_rec(a->b, b->b) &&
                   conv_rec(a->c, b->c);
        case TERM_NAT_Z:
            return 1;
        case TERM_NAT_S:
            return conv_rec(a->a, b->a);
        case TERM_NAT_REC:
            return conv_rec(a->a, b->a) && conv_rec(a->b, b->b) &&
                   conv_rec(a->c, b->c);
        case TERM_NAT_IND:
            return conv_rec(a->a, b->a) && conv_rec(a->b, b->b) &&
                   conv_rec(a->c, b->c) && conv_rec(a->d, b->d);
        case TERM_CONG:
            return conv_rec(a->a, b->a) && conv_rec(a->b, b->b);
        case TERM_EQ_TYPE:
            return conv_rec(a->a, b->a) && conv_rec(a->b, b->b);
        case TERM_LAM:
            return a->name && b->name && strcmp(a->name, b->name) == 0 &&
                   conv_rec(a->a, b->a);
        case TERM_APP:
            return conv_rec(a->a, b->a) && conv_rec(a->b, b->b);
        case TERM_BIN:
            return a->op == b->op && conv_rec(a->a, b->a) && conv_rec(a->b, b->b);
        case TERM_ANN:
            return conv_rec(a->a, b->a);
        case TERM_IF:
            return conv_rec(a->a, b->a) && conv_rec(a->b, b->b) &&
                   conv_rec(a->c, b->c);
        case TERM_LET:
            return a->name && b->name && strcmp(a->name, b->name) == 0 &&
                   conv_rec(a->a, b->a) && conv_rec(a->b, b->b);
        case TERM_FIELD:
            return a->name && b->name && strcmp(a->name, b->name) == 0 &&
                   conv_rec(a->a, b->a);
        case TERM_ENUM_CTOR:
            return a->name && b->name && strcmp(a->name, b->name) == 0 &&
                   conv_rec(a->a, b->a);
        case TERM_MATCH:
            if (a->narms != b->narms || !conv_rec(a->a, b->a)) return 0;
            for (int i = 0; i < a->narms; i++) {
                if (!a->arms[i].ctor || !b->arms[i].ctor ||
                    strcmp(a->arms[i].ctor, b->arms[i].ctor) != 0) return 0;
                if (!conv_rec(a->arms[i].body, b->arms[i].body)) return 0;
            }
            return 1;
        case TERM_STRUCT:
            if (a->nfields != b->nfields) return 0;
            for (int i = 0; i < a->nfields; i++) {
                if (!conv_rec(a->fields[i].val, b->fields[i].val)) return 0;
            }
            return 1;
    }
    return 0;
}

int qtt_conv(const Term *a, const Term *b) {
    qtt_term_pool_reset();
    Term *na = qtt_norm(a);
    Term *nb = qtt_norm(b);
    return conv_rec(na, nb);
}

int qtt_conv_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#undef A
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    static Term pool[64];
    static int pi = 0;

    /* (λx. x) 5 ≡ 5 */
    pi = 0;
    Term *xv = &pool[pi++]; memset(xv, 0, sizeof *xv); xv->kind = TERM_VAR; xv->name = "x";
    Term *id = &pool[pi++]; memset(id, 0, sizeof *id);
    id->kind = TERM_LAM; id->name = "x"; id->q = Q_MANY; id->ty = &I64_TY; id->a = xv;
    Term *five = &pool[pi++]; memset(five, 0, sizeof *five); five->kind = TERM_LIT; five->ival = 5;
    Term *app = &pool[pi++]; memset(app, 0, sizeof *app);
    app->kind = TERM_APP; app->a = id; app->b = five;
    A(qtt_conv(app, five), "conv ((λx.x) 5) ≡ 5");

    /* (λx. x + 1) 4 ≡ 5 */
    pi = 0;
    Term *x2 = &pool[pi++]; memset(x2, 0, sizeof *x2); x2->kind = TERM_VAR; x2->name = "x";
    Term *one = &pool[pi++]; memset(one, 0, sizeof *one); one->kind = TERM_LIT; one->ival = 1;
    Term *add = &pool[pi++]; memset(add, 0, sizeof *add);
    add->kind = TERM_BIN; add->op = BOP_ADD; add->a = x2; add->b = one;
    Term *lam = &pool[pi++]; memset(lam, 0, sizeof *lam);
    lam->kind = TERM_LAM; lam->name = "x"; lam->q = Q_MANY; lam->ty = &I64_TY; lam->a = add;
    Term *four = &pool[pi++]; memset(four, 0, sizeof *four); four->kind = TERM_LIT; four->ival = 4;
    Term *app2 = &pool[pi++]; memset(app2, 0, sizeof *app2);
    app2->kind = TERM_APP; app2->a = lam; app2->b = four;
    Term *five2 = &pool[pi++]; memset(five2, 0, sizeof *five2); five2->kind = TERM_LIT; five2->ival = 5;
    A(qtt_conv(app2, five2), "conv ((λx.x+1) 4) ≡ 5");

    /* nested β: (λx.x) ((λy.y) 7) ≡ 7 */
    pi = 0;
    Term *y = &pool[pi++]; memset(y, 0, sizeof *y); y->kind = TERM_VAR; y->name = "y";
    Term *idy = &pool[pi++]; memset(idy, 0, sizeof *idy);
    idy->kind = TERM_LAM; idy->name = "y"; idy->q = Q_MANY; idy->ty = &I64_TY; idy->a = y;
    Term *seven = &pool[pi++]; memset(seven, 0, sizeof *seven); seven->kind = TERM_LIT; seven->ival = 7;
    Term *inner = &pool[pi++]; memset(inner, 0, sizeof *inner);
    inner->kind = TERM_APP; inner->a = idy; inner->b = seven;
    Term *xid = &pool[pi++]; memset(xid, 0, sizeof *xid); xid->kind = TERM_VAR; xid->name = "x";
    Term *idx = &pool[pi++]; memset(idx, 0, sizeof *idx);
    idx->kind = TERM_LAM; idx->name = "x"; idx->q = Q_MANY; idx->ty = &I64_TY; idx->a = xid;
    Term *outer = &pool[pi++]; memset(outer, 0, sizeof *outer);
    outer->kind = TERM_APP; outer->a = idx; outer->b = inner;
    A(qtt_conv(outer, seven), "conv ((λx.x)((λy.y)7)) ≡ 7");

    /* non-equal: 5 ≢ 6 (fresh literals, pool already reset) */
    pi = 0;
    Term *fv = &pool[pi++]; memset(fv, 0, sizeof *fv); fv->kind = TERM_LIT; fv->ival = 5;
    Term *six = &pool[pi++]; memset(six, 0, sizeof *six); six->kind = TERM_LIT; six->ival = 6;
    A(!qtt_conv(fv, six), "5 ≢ 6");

    return all_ok ? 0 : -1;
}

/* ═══ Proof kernel: propositional equality (refl) ═══ */

int qtt_prove(const Term *proof, const Ty *goal, char *out_ty, size_t cap_ty,
              char *err, size_t cap_err) {
    Ctx c;
    memset(&c, 0, sizeof c);
    /* NOTE: does NOT reset ty_len — the caller owns the goal type's storage
     * (ty_alloc'd below); resetting here would let infer() clobber it. */
    if (check(&c, proof, goal, err, cap_err) != 0) {
        return -1;
    }
    qtt_ty_print(goal, out_ty, cap_ty);
    return 0;
}

int qtt_proof_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[256], ty[128];
#undef A
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    static Term pool[64];
    static int pi = 0;
    ty_len = 0; /* fresh type pool for this proof batch */

    /* 1. refl 5 : 5 = 5 */
    pi = 0;
    Term *five = &pool[pi++]; memset(five, 0, sizeof *five);
    five->kind = TERM_LIT; five->ival = 5;
    Term *refl5 = &pool[pi++]; memset(refl5, 0, sizeof *refl5);
    refl5->kind = TERM_REFL; refl5->a = five;
    Ty *goal55 = ty_alloc(TY_EQ);
    goal55->eq_a = &I64_TY; goal55->eq_l = five; goal55->eq_r = five;
    A(qtt_prove(refl5, goal55, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "= i64") == 0,
      "prove refl 5 : 5 = 5");

    /* 2. refl ((λx.x+1) 4) : ((λx.x+1) 4) = 5  (via β+δ conversion) */
    pi = 0;
    Term *x = &pool[pi++]; memset(x, 0, sizeof *x); x->kind = TERM_VAR; x->name = "x";
    Term *one = &pool[pi++]; memset(one, 0, sizeof *one); one->kind = TERM_LIT; one->ival = 1;
    Term *add = &pool[pi++]; memset(add, 0, sizeof *add);
    add->kind = TERM_BIN; add->op = BOP_ADD; add->a = x; add->b = one;
    Term *lam = &pool[pi++]; memset(lam, 0, sizeof *lam);
    lam->kind = TERM_LAM; lam->name = "x"; lam->q = Q_MANY; lam->ty = &I64_TY; lam->a = add;
    Term *four = &pool[pi++]; memset(four, 0, sizeof *four); four->kind = TERM_LIT; four->ival = 4;
    Term *app = &pool[pi++]; memset(app, 0, sizeof *app);
    app->kind = TERM_APP; app->a = lam; app->b = four;
    Term *five2 = &pool[pi++]; memset(five2, 0, sizeof *five2); five2->kind = TERM_LIT; five2->ival = 5;
    Term *reflApp = &pool[pi++]; memset(reflApp, 0, sizeof *reflApp);
    reflApp->kind = TERM_REFL; reflApp->a = app;
    Ty *goalApp = ty_alloc(TY_EQ);
    goalApp->eq_a = &I64_TY; goalApp->eq_l = app; goalApp->eq_r = five2;
    A(qtt_prove(reflApp, goalApp, ty, sizeof ty, err, sizeof err) == 0,
      "prove refl ((λx.x+1) 4) : ((λx.x+1) 4) = 5");

    /* 3. refl 5 : 5 = 6  must FAIL (no conversion can bridge 5 and 6) */
    pi = 0;
    Term *five3 = &pool[pi++]; memset(five3, 0, sizeof *five3); five3->kind = TERM_LIT; five3->ival = 5;
    Term *six3 = &pool[pi++]; memset(six3, 0, sizeof *six3); six3->kind = TERM_LIT; six3->ival = 6;
    Term *reflBad = &pool[pi++]; memset(reflBad, 0, sizeof *reflBad);
    reflBad->kind = TERM_REFL; reflBad->a = five3;
    Ty *goalBad = ty_alloc(TY_EQ);
    goalBad->eq_a = &I64_TY; goalBad->eq_l = five3; goalBad->eq_r = six3;
    A(qtt_prove(reflBad, goalBad, ty, sizeof ty, err, sizeof err) != 0,
      "reject refl 5 : 5 = 6");

    /* 4. subst (transport) with hypothesis H : (1+1) = 2.
     *   subst H H : 2 = 2  (rewrites (1+1) → 2 in H's type) */
    pi = 0;
    Term *oneL = &pool[pi++]; memset(oneL, 0, sizeof *oneL); oneL->kind = TERM_LIT; oneL->ival = 1;
    Term *oneR = &pool[pi++]; memset(oneR, 0, sizeof *oneR); oneR->kind = TERM_LIT; oneR->ival = 1;
    Term *addH = &pool[pi++]; memset(addH, 0, sizeof *addH);
    addH->kind = TERM_BIN; addH->op = BOP_ADD; addH->a = oneL; addH->b = oneR;
    Term *twoH = &pool[pi++]; memset(twoH, 0, sizeof *twoH); twoH->kind = TERM_LIT; twoH->ival = 2;
    Term *hvar = &pool[pi++]; memset(hvar, 0, sizeof *hvar); hvar->kind = TERM_VAR; hvar->name = "H";
    Ty *hypTy = ty_alloc(TY_EQ);
    hypTy->eq_a = &I64_TY; hypTy->eq_l = addH; hypTy->eq_r = twoH;
    Term *substTerm = &pool[pi++]; memset(substTerm, 0, sizeof *substTerm);
    substTerm->kind = TERM_SUBST; substTerm->a = hvar; substTerm->b = hvar;
    Term *twoL = &pool[pi++]; memset(twoL, 0, sizeof *twoL); twoL->kind = TERM_LIT; twoL->ival = 2;
    Term *twoR = &pool[pi++]; memset(twoR, 0, sizeof *twoR); twoR->kind = TERM_LIT; twoR->ival = 2;
    Ty *goal22 = ty_alloc(TY_EQ);
    goal22->eq_a = &I64_TY; goal22->eq_l = twoL; goal22->eq_r = twoR;
    Ctx hc;
    memset(&hc, 0, sizeof hc);
    hc.b[0].name = "H"; hc.b[0].q = Q_MANY; hc.b[0].ty = hypTy; hc.b[0].used = 0;
    hc.len = 1;
    A(check(&hc, substTerm, goal22, err, sizeof err) == 0,
      "prove subst H H : 2 = 2  (H : (1+1) = 2)");

    return all_ok ? 0 : -1;
}

/* ═══ Proof kernel: Nat + recursor (definitional computation) ═══ */

int qtt_nat_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[256], ty[128];
#undef A
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    static Term pool[64];
    static int pi = 0;

    /* 1. Z : Nat, and S(S Z) : Nat */
    pi = 0;
    ty_len = 0;
    Term *z = &pool[pi++]; memset(z, 0, sizeof *z); z->kind = TERM_NAT_Z;
    A(qtt_check_closed(z, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "Nat") == 0,
      "check Z : Nat");

    pi = 0;
    ty_len = 0;
    Term *z2 = &pool[pi++]; memset(z2, 0, sizeof *z2); z2->kind = TERM_NAT_Z;
    Term *s1 = &pool[pi++]; memset(s1, 0, sizeof *s1); s1->kind = TERM_NAT_S; s1->a = z2;
    Term *s2 = &pool[pi++]; memset(s2, 0, sizeof *s2); s2->kind = TERM_NAT_S; s2->a = s1;
    A(qtt_check_closed(s2, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "Nat") == 0,
      "check S(S Z) : Nat");

    /* 2. double = nat_rec Z (λk. λacc. S(S acc)) : Nat -> Nat.
     *    double 2 = nat_rec Z step (S(S Z))  reduces definitionally to S^4 Z,
     *    so `refl` proves  double 2 = 4  (Lean's `rfl` on a closed computation). */
    pi = 0;
    ty_len = 0;
    Term *base = &pool[pi++]; memset(base, 0, sizeof *base); base->kind = TERM_NAT_Z;
    Term *acc = &pool[pi++]; memset(acc, 0, sizeof *acc); acc->kind = TERM_VAR; acc->name = "acc";
    Term *sa = &pool[pi++]; memset(sa, 0, sizeof *sa); sa->kind = TERM_NAT_S; sa->a = acc;
    Term *ssa = &pool[pi++]; memset(ssa, 0, sizeof *ssa); ssa->kind = TERM_NAT_S; ssa->a = sa;
    Term *lam_acc = &pool[pi++]; memset(lam_acc, 0, sizeof *lam_acc);
    lam_acc->kind = TERM_LAM; lam_acc->name = "acc"; lam_acc->q = Q_MANY;
    lam_acc->ty = &NAT_TY; lam_acc->a = ssa;
    Term *step = &pool[pi++]; memset(step, 0, sizeof *step);
    step->kind = TERM_LAM; step->name = "k"; step->q = Q_MANY; step->ty = &NAT_TY; step->a = lam_acc;
    Term *z3 = &pool[pi++]; memset(z3, 0, sizeof *z3); z3->kind = TERM_NAT_Z;
    Term *t1 = &pool[pi++]; memset(t1, 0, sizeof *t1); t1->kind = TERM_NAT_S; t1->a = z3;
    Term *t2 = &pool[pi++]; memset(t2, 0, sizeof *t2); t2->kind = TERM_NAT_S; t2->a = t1;
    Term *double2 = &pool[pi++]; memset(double2, 0, sizeof *double2);
    double2->kind = TERM_NAT_REC; double2->a = base; double2->b = step; double2->c = t2;
    A(qtt_check_closed(double2, ty, sizeof ty, err, sizeof err) == 0 &&
          strcmp(ty, "Nat") == 0,
      "check nat_rec (double) 2 : Nat");

    /* S^4 Z — the numeral 4 (built in the SAME pool, so double2 stays alive) */
    Term *z4 = &pool[pi++]; memset(z4, 0, sizeof *z4); z4->kind = TERM_NAT_Z;
    Term *n1 = &pool[pi++]; memset(n1, 0, sizeof *n1); n1->kind = TERM_NAT_S; n1->a = z4;
    Term *n2 = &pool[pi++]; memset(n2, 0, sizeof *n2); n2->kind = TERM_NAT_S; n2->a = n1;
    Term *n3 = &pool[pi++]; memset(n3, 0, sizeof *n3); n3->kind = TERM_NAT_S; n3->a = n2;
    Term *four = &pool[pi++]; memset(four, 0, sizeof *four); four->kind = TERM_NAT_S; four->a = n3;
    Term *reflD = &pool[pi++]; memset(reflD, 0, sizeof *reflD);
    reflD->kind = TERM_REFL; reflD->a = double2;
    Ty *goalD = ty_alloc(TY_EQ);
    goalD->eq_a = &NAT_TY; goalD->eq_l = double2; goalD->eq_r = four;
    A(qtt_prove(reflD, goalD, ty, sizeof ty, err, sizeof err) == 0,
      "prove refl (double 2) : double 2 = S(S(S(S Z)))");

    /* 3. INDUCTION: prove add 2 Z = 2, where add m n = nat_rec n (λk acc. S acc) m.
     *    motive = λn. (nat_rec Z step n = n);  base = refl Z;
     *    step = λk. λih. congr (λx. S x) ih;   target = S(S Z). */
    ppi = 0;
    pi = 0;
    ty_len = 0;
    Term *accv = &pool[pi++]; memset(accv, 0, sizeof *accv); accv->kind = TERM_VAR; accv->name = "acc";
    Term *sacc = &pool[pi++]; memset(sacc, 0, sizeof *sacc); sacc->kind = TERM_NAT_S; sacc->a = accv;
    Term *lamacc = &pool[pi++]; memset(lamacc, 0, sizeof *lamacc);
    lamacc->kind = TERM_LAM; lamacc->name = "acc"; lamacc->q = Q_MANY; lamacc->ty = &NAT_TY; lamacc->a = sacc;
    Term *add_step = &pool[pi++]; memset(add_step, 0, sizeof *add_step);
    add_step->kind = TERM_LAM; add_step->name = "k"; add_step->q = Q_MANY; add_step->ty = &NAT_TY; add_step->a = lamacc;
    /* motive = λn. (nat_rec Z add_step n = n) */
    Term *fz = &pool[pi++]; memset(fz, 0, sizeof *fz); fz->kind = TERM_NAT_Z;
    Term *nvar = &pool[pi++]; memset(nvar, 0, sizeof *nvar); nvar->kind = TERM_VAR; nvar->name = "n";
    Term *f_n = &pool[pi++]; memset(f_n, 0, sizeof *f_n);
    f_n->kind = TERM_NAT_REC; f_n->a = fz; f_n->b = add_step; f_n->c = nvar;
    Term *g_n = &pool[pi++]; memset(g_n, 0, sizeof *g_n); g_n->kind = TERM_VAR; g_n->name = "n";
    Term *eqty = &pool[pi++]; memset(eqty, 0, sizeof *eqty);
    eqty->kind = TERM_EQ_TYPE; eqty->a = f_n; eqty->b = g_n;
    Term *motive = &pool[pi++]; memset(motive, 0, sizeof *motive);
    motive->kind = TERM_LAM; motive->name = "n"; motive->q = Q_MANY; motive->ty = &NAT_TY; motive->a = eqty;
    /* base = refl Z */
    Term *zbase = &pool[pi++]; memset(zbase, 0, sizeof *zbase); zbase->kind = TERM_NAT_Z;
    Term *ind_base = &pool[pi++]; memset(ind_base, 0, sizeof *ind_base); ind_base->kind = TERM_REFL; ind_base->a = zbase;
    /* succ = λx. S x */
    Term *xv = &pool[pi++]; memset(xv, 0, sizeof *xv); xv->kind = TERM_VAR; xv->name = "x";
    Term *sx = &pool[pi++]; memset(sx, 0, sizeof *sx); sx->kind = TERM_NAT_S; sx->a = xv;
    Term *succ = &pool[pi++]; memset(succ, 0, sizeof *succ);
    succ->kind = TERM_LAM; succ->name = "x"; succ->q = Q_MANY; succ->ty = &NAT_TY; succ->a = sx;
    /* step = λk. λih. congr succ ih */
    Term *ihv = &pool[pi++]; memset(ihv, 0, sizeof *ihv); ihv->kind = TERM_VAR; ihv->name = "ih";
    Term *cong = &pool[pi++]; memset(cong, 0, sizeof *cong); cong->kind = TERM_CONG; cong->a = succ; cong->b = ihv;
    Term *lamih = &pool[pi++]; memset(lamih, 0, sizeof *lamih);
    lamih->kind = TERM_LAM; lamih->name = "ih"; lamih->q = Q_MANY; lamih->ty = NULL; lamih->a = cong;
    Term *ind_step = &pool[pi++]; memset(ind_step, 0, sizeof *ind_step);
    ind_step->kind = TERM_LAM; ind_step->name = "k"; ind_step->q = Q_MANY; ind_step->ty = &NAT_TY; ind_step->a = lamih;
    /* target = S(S Z) = 2 */
    Term *z2t = &pool[pi++]; memset(z2t, 0, sizeof *z2t); z2t->kind = TERM_NAT_Z;
    Term *s1t = &pool[pi++]; memset(s1t, 0, sizeof *s1t); s1t->kind = TERM_NAT_S; s1t->a = z2t;
    Term *s2t = &pool[pi++]; memset(s2t, 0, sizeof *s2t); s2t->kind = TERM_NAT_S; s2t->a = s1t;
    /* proof = nat_ind motive base step target */
    Term *proof = &pool[pi++]; memset(proof, 0, sizeof *proof);
    proof->kind = TERM_NAT_IND; proof->d = motive; proof->a = ind_base; proof->b = ind_step; proof->c = s2t;
    /* goal = add 2 Z = 2  =  nat_rec Z add_step (S(S Z)) = S(S Z) */
    Term *gfz = &pool[pi++]; memset(gfz, 0, sizeof *gfz); gfz->kind = TERM_NAT_Z;
    Term *gadd = &pool[pi++]; memset(gadd, 0, sizeof *gadd);
    gadd->kind = TERM_NAT_REC; gadd->a = gfz; gadd->b = add_step; gadd->c = s2t;
    Ty *goalInd = ty_alloc(TY_EQ);
    goalInd->eq_a = &NAT_TY; goalInd->eq_l = gadd; goalInd->eq_r = s2t;
    A(qtt_prove(proof, goalInd, ty, sizeof ty, err, sizeof err) == 0,
      "prove nat_ind (add 2 Z = 2) by induction");

    return all_ok ? 0 : -1;
}
