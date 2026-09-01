/* Bebop SMT — native DPLL SAT solver (5B). Zero dependencies. */
#include "smt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ─── DPLL state ─── */
/* We work on a copy of the CNF with an assignment. A clause is "false" when
 * every literal is currently false; "satisfied" when any literal is true. */

#define MAX_VARS 64
#define MAX_CLAUSES 512
#define MAX_LITS 4096

typedef struct {
    int nvars;
    int nclauses;
    int clauses[MAX_CLAUSES][8]; /* fixed-width clauses (bounded kernel) */
    int lens[MAX_CLAUSES];
    int assign[MAX_VARS + 1]; /* 0=unassigned, 1=true, -1=false */
} Dpll;

/* copy the flat CNF into the fixed-width representation. Returns 0 on success,
 * -1 on malformed/too-large input. */
static int load_cnf(Dpll *d, int nvars, const int *cnf) {
    memset(d, 0, sizeof *d);
    d->nvars = nvars;
    int ci = 0;
    int li = 0;
    for (;;) {
        int lit = *cnf;
        if (lit == 0) {
            if (li == 0) {
                break; /* double 0 = end of formula */
            }
            d->lens[ci] = li; /* commit this clause's length */
            ci++;
            li = 0;
            cnf++;
            continue;
        }
        int v = lit < 0 ? -lit : lit;
        if (v > nvars || v <= 0 || li >= 8 || ci >= MAX_CLAUSES) {
            return -1;
        }
        d->clauses[ci][li++] = lit;
        cnf++;
    }
    d->nclauses = ci;
    return 0;
}

/* literal value under the current assignment: 1 true, 0 false, -1 undecided. */
static int lit_val(const Dpll *d, int lit) {
    int v = lit < 0 ? -lit : lit;
    int a = d->assign[v];
    if (a == 0) {
        return -1;
    }
    int truth = (a == 1);
    return (lit > 0) ? truth : !truth;
}

/* clause status: 1 satisfied, 0 still undecided, -1 conflicting (all false). */
static int clause_status(const Dpll *d, int ci) {
    int any_undec = 0;
    for (int i = 0; i < d->lens[ci]; i++) {
        int v = lit_val(d, d->clauses[ci][i]);
        if (v == 1) {
            return 1;
        }
        if (v == -1) {
            any_undec = 1;
        }
    }
    return any_undec ? 0 : -1;
}

/* DPLL with unit propagation. Returns 1 SAT, 0 UNSAT. */
static int solve(Dpll *d) {
    /* propagate unit clauses to fixpoint */
    int changed;
    do {
        changed = 0;
        for (int ci = 0; ci < d->nclauses; ci++) {
            if (clause_status(d, ci) == -1) {
                return 0; /* conflict */
            }
            /* unit clause: exactly one undecided literal, ZERO true literals
             * (a satisfied clause must not force its remaining undecided lits) */
            int undec = 0, unit_lit = 0, has_true = 0;
            for (int i = 0; i < d->lens[ci]; i++) {
                int v = lit_val(d, d->clauses[ci][i]);
                if (v == 1) {
                    has_true = 1;
                    break;
                }
                if (v == -1) {
                    undec++;
                    unit_lit = d->clauses[ci][i];
                }
            }
            if (!has_true && undec == 1) {
                int var = unit_lit < 0 ? -unit_lit : unit_lit;
                d->assign[var] = (unit_lit > 0) ? 1 : -1;
                changed = 1;
            }
        }
    } while (changed);

    /* check for conflict after propagation */
    for (int ci = 0; ci < d->nclauses; ci++) {
        if (clause_status(d, ci) == -1) {
            return 0;
        }
    }
    /* all clauses satisfied? */
    int all_sat = 1;
    for (int ci = 0; ci < d->nclauses; ci++) {
        if (clause_status(d, ci) != 1) {
            all_sat = 0;
            break;
        }
    }
    if (all_sat) {
        return 1;
    }
    /* pick an unassigned variable (first-fit) and branch */
    int var = 0;
    for (int v = 1; v <= d->nvars; v++) {
        if (d->assign[v] == 0) {
            var = v;
            break;
        }
    }
    if (var == 0) {
        return 0; /* no unassigned var but not all satisfied → UNSAT */
    }
    /* branch true */
    {
        Dpll a = *d;
        a.assign[var] = 1;
        if (solve(&a)) {
            *d = a;
            return 1;
        }
    }
    /* branch false */
    {
        Dpll b = *d;
        b.assign[var] = -1;
        if (solve(&b)) {
            *d = b;
            return 1;
        }
    }
    return 0;
}

int smt_dpll(int nvars, const int *cnf, int *model) {
    Dpll d;
    if (load_cnf(&d, nvars, cnf) != 0) {
        return -1;
    }
    if (solve(&d) != 1) {
        return 0;
    }
    if (model) {
        for (int v = 1; v <= nvars; v++) {
            model[v] = d.assign[v] == 1 ? 1 : 0;
        }
    }
    return 1;
}

/* ─── Self-test ─── */

int smt_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define S(cond, name)                                                      \
    do {                                                                   \
        int c_ = (int)(cond);                                              \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",               \
                          c_ ? "ok" : "FAIL", name);                       \
        if (r_ > 0) pos += (size_t)r_;                                     \
        if (!c_) all_ok = 0;                                               \
    } while (0)

    /* SAT: (x1 ∨ x2) ∧ (x1 ∨ ¬x2) ∧ (¬x1 ∨ x2)  →  x1=x2=true */
    {
        int cnf[] = {1, 2, 0, 1, -2, 0, -1, 2, 0, 0};
        int m[8] = {0};
        int r = smt_dpll(2, cnf, m);
        S(r == 1 && m[1] == 1 && m[2] == 1, "DPLL SAT (x1∨x2)∧(x1∨¬x2)∧(¬x1∨x2) → x1=x2=T");
    }
    /* UNSAT: (x1) ∧ (¬x1) */
    {
        int cnf[] = {1, 0, -1, 0, 0};
        int m[8] = {0};
        S(smt_dpll(1, cnf, m) == 0, "DPLL UNSAT (x1)∧(¬x1)");
    }
    /* UNSAT: the pigeonhole-less 3-clause contradiction
     * (x1∨x2) ∧ (x1∨¬x2) ∧ (¬x1∨x2) ∧ (¬x1∨¬x2) */
    {
        int cnf[] = {1, 2, 0, 1, -2, 0, -1, 2, 0, -1, -2, 0, 0};
        S(smt_dpll(2, cnf, NULL) == 0, "DPLL UNSAT 4-clause contradiction");
    }
    /* SAT: single unit (x3) → x3 true, x1 free */
    {
        int cnf[] = {3, 0, 0};
        int m[8] = {0};
        int r = smt_dpll(3, cnf, m);
        S(r == 1 && m[3] == 1, "DPLL unit (x3) → x3=true");
    }
    /* UNSAT: chain x1, x1→¬x2, x2  (i.e. x1 ∧ (¬x1∨¬x2) ∧ x2) */
    {
        int cnf[] = {1, 0, -1, -2, 0, 2, 0, 0};
        S(smt_dpll(2, cnf, NULL) == 0, "DPLL UNSAT chain x1 ∧ (¬x1∨¬x2) ∧ x2");
    }
    return all_ok ? 0 : -1;
}
