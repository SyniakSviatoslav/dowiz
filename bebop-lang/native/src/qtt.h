/* Bebop QTT core — quantities (the {0,1,ω} semiring) + type representation.
 * Phase 2 first slice. This is the foundation of linear/affine typing:
 * quantity 0 = erased (proofs/types), 1 = linear (exactly-once), ω = unrestricted.
 */
#ifndef BEBOP_QTT_H
#define BEBOP_QTT_H

#include <stddef.h>

/* The QTT rig {0, 1, ω}. */
typedef enum {
    Q_ZERO = 0, /* 0: erased — types, proofs, compile-time only */
    Q_ONE = 1,  /* 1: linear — used exactly once (affine: at most once) */
    Q_MANY = 2, /* ω: unrestricted */
} Quantity;

/* Semiring ops.
 *   add  ⊕ (combine uses): 0+p=p, 1+1=ω, ω+p=ω
 *   mult ⊗ (nested use):   0·p=0, 1·p=p, ω·0=0, ω·1=ω, ω·ω=ω
 */
Quantity qtt_add(Quantity a, Quantity b);
Quantity qtt_mul(Quantity a, Quantity b);
const char *qtt_q_name(Quantity q);

/* Core types. */
typedef enum {
    TY_I64,
    TY_U8,
    TY_U32,
    TY_U64,
    TY_F64,
    TY_BOOL,
    TY_VOID,
    TY_FN,       /* non-dependent arrow: dom -> cod */
    TY_PI,       /* dependent product: (x :^q dom) -> cod, x bound in cod */
    TY_FIELD,    /* finite field F_p for NTT (p = .n) */
    TY_HYPERVEC, /* D-dim hypervector, D = .n (1024) */
    TY_VEC,      /* fixed-width SIMD vector: width = .n, elem = .elem */
    TY_STRUCT,   /* record: named fields (.fields / .nfields) */
    TY_ENUM,     /* sum: named ctors (.ctors / .nctors) */
    TY_TYPE,     /* universe: the type of types (Type₀) */
    TY_VAR,      /* type variable (name in .x) */
    TY_EQ,       /* propositional equality: a = b (Eq A a b) */
    TY_NAT,      /* natural numbers (Peano): Z | S n */
    TY_STR,      /* string literal (immutable byte sequence) */
    TY_PTR,      /* pointer to elem: t->elem = pointee type */
} TyKind;

typedef struct Ty Ty;
typedef struct {
    const char *name;
    Ty *ty;
} TyField;
typedef struct {
    const char *name;
    Ty *payload; /* NULL for unit constructors */
} Ctor;

struct Ty {
    TyKind kind;
    long n;        /* field prime / hypervector dim / vector width */
    Quantity q;    /* PI quantity */
    const char *x; /* PI binder name (borrowed, not owned) */
    Ty *dom;       /* FN / PI domain */
    Ty *cod;       /* FN / PI codomain */
    Ty *elem;      /* VEC element type */
    TyField *fields; /* TY_STRUCT: named fields */
    int nfields;
    Ctor *ctors;   /* TY_ENUM: constructors */
    int nctors;
    Ty *eq_a;      /* TY_EQ: the type A (both sides live in A) */
    struct Term *eq_l; /* TY_EQ: left side term */
    struct Term *eq_r; /* TY_EQ: right side term */
};

/* Pretty-print a type into `out` (NUL-terminated). Returns bytes written. */
int qtt_ty_print(const Ty *t, char *out, size_t cap);

/* Run the QTT self-test (semiring laws + type round-trip). Returns 0 on
 * success, -1 on failure; appends human-readable results to `out`. */
int qtt_self_test(char *out, size_t cap);

/* ─── Terms (core) ─── */

typedef enum {
    TERM_VAR,
    TERM_LIT,
    TERM_LAM,
    TERM_APP,
    TERM_ANN,
    TERM_BIN,
    TERM_IF,
    TERM_LET,
    TERM_STRUCT,
    TERM_FIELD,
    TERM_ENUM_CTOR,
    TERM_MATCH,
    TERM_TYPE,
    TERM_IO,
    TERM_REFL,    /* refl : a = a (propositional equality intro) */
    TERM_SUBST,   /* subst : a = b -> P a -> P b (equality elim / transport) */
    TERM_NAT_Z,   /* Z : Nat (zero constructor) */
    TERM_NAT_S,   /* S n : Nat (successor, .a = the predecessor) */
    TERM_NAT_REC, /* nat_rec : P -> (Nat -> P -> P) -> Nat -> P
                   *   .a = base, .b = step, .c = target */
    TERM_NAT_IND, /* nat_ind : (n. motive) -> motive Z
                   *         -> ((k. motive k -> motive (S k))) -> (n. motive n)
                   *   .ty = motive LAM, .a = base, .b = step, .c = target */
    TERM_CONG,    /* congr : (a = b) -> (f a = f b)
                   *   .a = f (a function), .b = the equality proof */
    TERM_EQ_TYPE, /* "a = b" as a TERM of type Type (the motive's body).
                   *   .a = left, .b = right */
    TERM_STR,     /* string literal — content is borrowed in t->name */
    TERM_STR_LEN, /* length of a string (t->a) : i64 */
    TERM_STR_CAT, /* concatenation of two strings (t->a, t->b) : str */
    TERM_WHILE,   /* while loop: a = cond, b = body; evaluates to void */
    TERM_ARRAY,   /* array literal: fields/nfields hold indexed elements */
    TERM_ARRAY_GET, /* array indexing: a = array, b = index (i64) */
    TERM_STR_CHAR,  /* string char access: a = str, b = index (i64) -> i64 */
    TERM_ARRAY_SET, /* array mutation: a = array, b = index, c = value -> void */
    TERM_SYSCALL,  /* raw syscall: t->ival = syscall number, t->a = first arg */
    TERM_CHR,      /* chr(i64) -> single-char string; t->a = the i64 */
    TERM_SPAWN,    /* spawn fn(arg): t->a = fn expr (must be closure/lambda) */
    TERM_AWAIT,   /* await expr: t->a = expr (must be spawn handle, i64) */
    TERM_ADDR_OF, /* &e: t->a = expr, returns pointer-to-type-of-e */
    TERM_DEREF_PTR, /* *e: t->a = pointer expr, returns pointee */
} TermKind;

typedef enum {
    BOP_ADD,
    BOP_SUB,
    BOP_MUL,
    BOP_EQ,
    BOP_LT,
    BOP_NE,
    BOP_LE,
    BOP_GE,
    BOP_GT,
    BOP_CAT, /* string concatenation (++) */
} BinOp;

typedef struct Term Term;
typedef struct {
    const char *name;
    Term *val;
} TermField;
typedef struct {
    const char *ctor; /* constructor name */
    const char *var;  /* bound variable (NULL for unit ctor) */
    Term *body;
} MatchArm;

struct Term {
    TermKind kind;
    const char *name; /* VAR name / LAM binder / FIELD name / ctor name */
    Quantity q;       /* LAM binder quantity */
    long ival;        /* LIT int value */
    int bval;         /* LIT bool value (0/1) */
    BinOp op;         /* BIN operator */
    Ty *ty;           /* LAM domain / ANN type / STRUCT type / ENUM type */
    Term *a, *b, *c;  /* APP/LAM/ANN/BIN/IF/LET/FIELD(base)/ENUM_CTOR(payload)/MATCH(scrut) */
    Term *d;          /* NAT_IND motive (4th subterm slot) */
    TermField *fields; /* TERM_STRUCT: field name → value */
    int nfields;
    MatchArm *arms;   /* TERM_MATCH: arms */
    int narms;
};

/* Typecheck a closed term (empty context). Returns 0 on success (type printed
 * into out_ty via qtt_ty_print), -1 on error (err filled). */
int qtt_check_closed(const Term *t, char *out_ty, size_t cap_ty, char *err,
                     size_t cap_err);
/* Typecheck a term in a context pre-bound with named types (e.g. earlier fns). */
int qtt_check_binds(const Term *t, const char **names, const Ty **tys,
                    int n, char *out_ty, size_t cap_ty, char *err, size_t cap_err);
/* Save the current type-pool position as the reuse floor (so pre-bound types
 * from a prior pass are not overwritten by later allocations). */
void qtt_ty_checkpoint(void);
/* Evaluate a closed term with `n` function names pre-bound to lambdas (closures). */
int qtt_eval_binds(const Term *t, const char **names, Term *const *lams, int n,
                   int *out_kind, long *out_i, int *out_b, char *err, size_t cap);

/* Run the typechecker self-test. Returns 0 on success, -1 on failure. */
int qtt_check_test(char *out, size_t cap);

/* Evaluate a closed term (call-by-value, environment-based). Returns 0 on
 * success; the result is in *out_i (when *out_kind==0) or *out_b (when
 * *out_kind==1). Returns -1 on error (err filled). */
int qtt_eval(const Term *t, int *out_kind, long *out_i, int *out_b, char *err,
             size_t cap);

/* Run the evaluator self-test. Returns 0 on success, -1 on failure. */
int qtt_eval_test(char *out, size_t cap);

/* A name → int binding, used to evaluate terms with free variables. */
typedef struct {
    const char *name;
    long i;
} QttBind;

/* Evaluate a term with initial integer bindings (for contract checking). */
int qtt_eval_bound(const Term *t, const QttBind *binds, int n, int *out_kind,
                   long *out_i, int *out_b, char *err, size_t cap);

/* Public type singletons (for building terms from the parser). */
Ty *qtt_i64(void);
Ty *qtt_bool(void);

/* Accessor for the Type universe singleton. */
Ty *qtt_type(void);
Ty *qtt_str(void);
Ty *qtt_vec(Ty *elem);

/* Run the struct (record) typecheck + eval self-test. */
int qtt_struct_test(char *out, size_t cap);

/* Run the enum (sum) + match typecheck + eval self-test. */
int qtt_enum_test(char *out, size_t cap);

/* Run the dependent-type (universe + type substitution) self-test. */
int qtt_dep_test(char *out, size_t cap);

/* Effect analysis: does the term contain an I/O side effect (TERM_IO)? */
int qtt_term_has_io(const Term *t);

/* Run the effect (pure/io) analysis self-test. */
int qtt_effect_test(char *out, size_t cap);

/* ═══ Proof kernel: definitional equality (conversion) ═══
 * Lean-4-like mathematical proof rests on a *small* trustworthy core: a
 * term normalizer (β-reduction) + a conversion check (α-equivalence up to
 * normalization). These three primitives are that core. */

/* Reset the kernel's term scratch pool (call before a fresh proof batch). */
void qtt_term_pool_reset(void);

/* Substitute `v` for free occurrences of `name` in `t` (capture-avoiding —
 * a binder shadowing `name` blocks the substitution). Returns a fresh term
 * from the kernel pool, or NULL on pool exhaustion. */
Term *qtt_subst(const Term *t, const char *name, const Term *v);

/* β-normalize a term to normal form (call-by-name weak-head reduction +
 * structural recursion). Returns a fresh term, or NULL on exhaustion. */
Term *qtt_norm(const Term *t);

/* Definitional equality: are `a` and `b` β-convertible? Normalizes both and
 * compares structurally (α-equivalence on binder names). */
int qtt_conv(const Term *a, const Term *b);

/* Run the proof-kernel (conversion) self-test. */
int qtt_conv_test(char *out, size_t cap);

/* Prove: check `proof : goal` in the empty context (the kernel's judgement
 * for a completed theorem). Returns 0 on success (goal type printed into
 * out_ty), -1 if `proof` does not inhabit `goal`. */
int qtt_prove(const Term *proof, const Ty *goal, char *out_ty, size_t cap_ty,
              char *err, size_t cap_err);

/* Prove a definitional equality `l = r` by refl: builds Eq(i64, l, r) and
 * checks `refl l` against it (the conversion check accepts when l ≡ r up to
 * β+δ). Returns 0 on success, -1 if l and r are not definitionally equal. */
int qtt_prove_refl(const Term *l, const Term *r, char *out, size_t cap,
                   char *err, size_t cap_err);

/* Run the propositional-equality (refl / conversion) proof self-test. */
int qtt_proof_test(char *out, size_t cap);

/* Run the Nat + recursor (definitional computation) proof self-test. */
int qtt_nat_test(char *out, size_t cap);

/* Run the string (literal / length / concat) self-test. */
int qtt_str_test(char *out, size_t cap);

/* Run the array (literal / indexing) self-test. */
int qtt_array_test(char *out, size_t cap);

/* Run the universe (cumulativity) self-test. */
int qtt_universe_test(char *out, size_t cap);

#endif /* BEBOP_QTT_H */
