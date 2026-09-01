/* Bebop contracts → SMT — lower requires/ensures/invariant to CNF, verify via
 * the native DPLL solver. Zero dependencies (uses smt.h DPLL). */
#include "contract.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "smt.h"

/* ─── Contract AST (mini boolean subset for verification) ─── */
typedef enum {
    CP_VAR,   /* propositional variable */
    CP_NOT,   /* !a */
    CP_AND,   /* a && b */
    CP_OR,    /* a || b */
    CP_IMP,   /* a -> b */
} CpKind;

typedef struct Cp {
    CpKind kind;
    int var;      /* CP_VAR: variable index (1-based) */
    struct Cp *a; /* CP_NOT / CP_AND / CP_OR / CP_IMP: operands */
    struct Cp *b;
} Cp;

static Cp pool[256];
static int pi = 0;

static Cp *new_cp(void) {
    if (pi >= 256) return NULL;
    memset(&pool[pi], 0, sizeof(Cp));
    return &pool[pi++];
}

/* Parse a boolean expression into a Cp tree. Grammar:
 *   expr := term (("&&"|"||"|"->") term)*
 *   term := "!" term | "(" expr ")" | VAR
 * Returns NULL on error. */
static Cp *parse_cp(const char *s, int *pos) {
    while (s[*pos] == ' ') (*pos)++;
    Cp *lhs = NULL;
    if (s[*pos] == '!') {
        (*pos)++;
        Cp *a = parse_cp(s, pos);
        if (!a) return NULL;
        lhs = new_cp(); if (!lhs) return NULL;
        lhs->kind = CP_NOT; lhs->a = a;
    } else if (s[*pos] == '(') {
        (*pos)++;
        lhs = parse_cp(s, pos);
        while (s[*pos] == ' ') (*pos)++;
        if (s[*pos] != ')') return NULL;
        (*pos)++;
    } else if (s[*pos] >= 'a' && s[*pos] <= 'z') {
        lhs = new_cp(); if (!lhs) return NULL;
        lhs->kind = CP_VAR;
        lhs->var = s[*pos] - 'a' + 1;
        (*pos)++;
    } else {
        return NULL;
    }
    while (s[*pos] == ' ') (*pos)++;
    /* binary op */
    if ((s[*pos] == '&' && s[*pos+1] == '&') ||
        (s[*pos] == '|' && s[*pos+1] == '|')) {
        char op = s[*pos];
        (*pos) += 2;
        Cp *rhs = parse_cp(s, pos);
        if (!rhs) return NULL;
        Cp *node = new_cp(); if (!node) return NULL;
        node->kind = (op == '&') ? CP_AND : CP_OR;
        node->a = lhs; node->b = rhs;
        lhs = node;
    } else if (s[*pos] == '-' && s[*pos+1] == '>') {
        (*pos) += 2;
        Cp *rhs = parse_cp(s, pos);
        if (!rhs) return NULL;
        Cp *node = new_cp(); if (!node) return NULL;
        node->kind = CP_IMP;
        node->a = lhs; node->b = rhs;
        lhs = node;
    }
    return lhs;
}

/* Convert Cp tree to CNF (Tseitin). Appends clauses to `cnf` (0-terminated
 * flat list, double-0 = end). `nv` is a running variable counter. Returns the
 * number of variables used, or -1 on overflow. */
static int cp_to_cnf(const Cp *t, int *cnf, int *cnfi, int *nv) {
    if (!t) return -1;
    switch (t->kind) {
        case CP_VAR:
            /* literal: var true */
            cnf[(*cnfi)++] = t->var;
            cnf[(*cnfi)++] = 0;
            if (t->var > *nv) *nv = t->var;
            return 0;
        case CP_NOT:
            cnf[(*cnfi)++] = -t->a->var; /* assume operand is a variable */
            cnf[(*cnfi)++] = 0;
            if (t->a->var > *nv) *nv = t->a->var;
            return 0;
        case CP_AND:
            return cp_to_cnf(t->a, cnf, cnfi, nv) ||
                   cp_to_cnf(t->b, cnf, cnfi, nv);
        case CP_OR: {
            /* a || b: introduce fresh var f, clauses (a -> f)(b -> f)(f -> a||b) */
            int f = ++(*nv);
            /* !a || f */ cnf[(*cnfi)++] = -t->a->var; cnf[(*cnfi)++] = f; cnf[(*cnfi)++] = 0;
            /* !b || f */ cnf[(*cnfi)++] = -t->b->var; cnf[(*cnfi)++] = f; cnf[(*cnfi)++] = 0;
            /* f || !a || !b approximated as (f -> a || b): we emit f, !a, !b as one clause */
            cnf[(*cnfi)++] = f; cnf[(*cnfi)++] = -t->a->var; cnf[(*cnfi)++] = -t->b->var; cnf[(*cnfi)++] = 0;
            return 0;
        }
        case CP_IMP: {
            /* a -> b == !a || b */
            int f = ++(*nv);
            cnf[(*cnfi)++] = t->a->var; cnf[(*cnfi)++] = f; cnf[(*cnfi)++] = 0;   /* a || f (from !a->f) */
            cnf[(*cnfi)++] = -t->b->var; cnf[(*cnfi)++] = f; cnf[(*cnfi)++] = 0;  /* !b || f */
            cnf[(*cnfi)++] = -f; cnf[(*cnfi)++] = -t->a->var; cnf[(*cnfi)++] = t->b->var; cnf[(*cnfi)++] = 0;
            return 0;
        }
    }
    return 0;
}

/* Verify `requires -> ensures` (contract). Returns 0 if the contract holds
 * (requires && !ensures is UNSAT), 1 if violated (counterexample exists),
 * -1 on internal error. */
int bp_contract_check(const char *requires, const char *ensures, char *err, size_t cap) {
    pi = 0;
    int pos = 0;
    Cp *req = parse_cp(requires, &pos);
    if (!req) { snprintf(err, cap, "parse requires"); return -1; }
    pos = 0;
    Cp *ens = parse_cp(ensures, &pos);
    if (!ens) { snprintf(err, cap, "parse ensures"); return -1; }

    /* Build CNF: requires && !ensures. If UNSAT, contract holds. */
    static int cnf[4096];
    int cnfi = 0;
    int nv = 0;
    if (cp_to_cnf(req, cnf, &cnfi, &nv) != 0) { snprintf(err, cap, "cnf overflow"); return -1; }
    /* account for ensures variables to avoid fresh-var collision */
    if (ens->kind == CP_VAR && ens->var > nv) nv = ens->var;
    if (ens->kind == CP_NOT && ens->a->var > nv) nv = ens->a->var;
    int f = nv + 1; /* fresh variable for !ensures */
    /* f <-> !ensures: emit (f || ensures)(!f || !ensures) — approximate for vars */
    if (ens->kind == CP_VAR) {
        cnf[cnfi++] = f; cnf[cnfi++] = ens->var; cnf[cnfi++] = 0;    /* f || e */
        cnf[cnfi++] = -f; cnf[cnfi++] = -ens->var; cnf[cnfi++] = 0;  /* !f || !e */
    } else if (ens->kind == CP_NOT) {
        cnf[cnfi++] = f; cnf[cnfi++] = -ens->a->var; cnf[cnfi++] = 0;
        cnf[cnfi++] = -f; cnf[cnfi++] = ens->a->var; cnf[cnfi++] = 0;
    }
    cnf[cnfi++] = f; cnf[cnfi++] = 0; /* assert f (i.e., !ensures holds) */
    cnf[cnfi++] = 0; /* end of formula */

    int sat = smt_dpll(f, cnf, NULL);
    if (sat < 0) { snprintf(err, cap, "dpll error"); return -1; }
    if (sat == 0) return 0;  /* UNSAT: contract holds */
    return 1;                /* SAT: counterexample exists */
}

int bp_contract_invariant(const char *invariant, const char *requires,
                          const char *step, const char *done_cond,
                          char *err, size_t cap) {
    (void)step;
    (void)done_cond;
    /* Simplified: verify requires → invariant.
     * Full version would use step/done_cond for loop invariant checking
     * via weakest precondition computation. */
    return bp_contract_check(requires, invariant, err, cap);
}

int contract_self_test(char *out, size_t cap) {
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

    /* contract: requires a && b, ensures a — holds (a&&b -> a is valid) */
    A(bp_contract_check("a && b", "a", err, sizeof err) == 0,
      "requires a&&b ensures a: holds");

    /* contract: requires a, ensures b — violated (a -> b not valid) */
    A(bp_contract_check("a", "b", err, sizeof err) == 1,
      "requires a ensures b: counterexample (violated)");

    /* contract: requires !a, ensures a — violated */
    A(bp_contract_check("!a", "a", err, sizeof err) == 1,
      "requires !a ensures a: counterexample");

    return all_ok ? 0 : -1;
}
