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
    TY_F64,
    TY_BOOL,
    TY_VOID,
    TY_FN,       /* non-dependent arrow: dom -> cod */
    TY_PI,       /* dependent product: (x :^q dom) -> cod, x bound in cod */
    TY_FIELD,    /* finite field F_p for NTT (p = .n) */
    TY_HYPERVEC, /* D-dim hypervector, D = .n (1024) */
    TY_VEC,      /* fixed-width SIMD vector: width = .n, elem = .elem */
} TyKind;

typedef struct Ty Ty;
struct Ty {
    TyKind kind;
    long n;        /* field prime / hypervector dim / vector width */
    Quantity q;    /* PI quantity */
    const char *x; /* PI binder name (borrowed, not owned) */
    Ty *dom;       /* FN / PI domain */
    Ty *cod;       /* FN / PI codomain */
    Ty *elem;      /* VEC element type */
};

/* Pretty-print a type into `out` (NUL-terminated). Returns bytes written. */
int qtt_ty_print(const Ty *t, char *out, size_t cap);

/* Run the QTT self-test (semiring laws + type round-trip). Returns 0 on
 * success, -1 on failure; appends human-readable results to `out`. */
int qtt_self_test(char *out, size_t cap);

#endif /* BEBOP_QTT_H */
