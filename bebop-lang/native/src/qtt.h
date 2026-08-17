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
} TyKind;

typedef struct Ty Ty;
typedef struct {
    const char *name;
    Ty *ty;
} TyField;

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
} BinOp;

typedef struct Term Term;
typedef struct {
    const char *name;
    Term *val;
} TermField;

struct Term {
    TermKind kind;
    const char *name; /* VAR name / LAM binder / FIELD name */
    Quantity q;       /* LAM binder quantity */
    long ival;        /* LIT int value */
    int bval;         /* LIT bool value (0/1) */
    BinOp op;         /* BIN operator */
    Ty *ty;           /* LAM domain / ANN type / STRUCT type */
    Term *a, *b, *c;  /* APP/LAM/ANN/BIN/IF/LET/FIELD(base) */
    TermField *fields; /* TERM_STRUCT: field name → value */
    int nfields;
};

/* Typecheck a closed term (empty context). Returns 0 on success (type printed
 * into out_ty via qtt_ty_print), -1 on error (err filled). */
int qtt_check_closed(const Term *t, char *out_ty, size_t cap_ty, char *err,
                     size_t cap_err);

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

/* Run the struct (record) typecheck + eval self-test. */
int qtt_struct_test(char *out, size_t cap);

#endif /* BEBOP_QTT_H */
