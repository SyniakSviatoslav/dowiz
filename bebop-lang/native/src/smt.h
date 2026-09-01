/* Bebop SMT — native DPLL decision procedure (5B).
 *
 * The pipeline's verification step solves verification conditions with a
 * hand-written DPLL SAT solver — no Z3, no CVC5, no external tool. This is the
 * "execute the checks" half of contracts→SMT; verify.c still emits the
 * SMT-LIB VC for interop/audit, but the decision itself is native.
 *
 * CNF representation: a flat array of literals, each clause terminated by 0,
 * the whole formula terminated by an extra 0. Literal +v = variable v true,
 * -v = variable v false, v ∈ [1, nvars]. */
#ifndef BEBOP_SMT_H
#define BEBOP_SMT_H

#include <stddef.h>

/* DPLL: decide whether the CNF is satisfiable. Returns 1 (SAT), 0 (UNSAT),
 * -1 (malformed). On SAT, `model` (if non-NULL) is filled with the assignment:
 * model[v] = 1 (true) or 0 (false), indexed 1..nvars. */
int smt_dpll(int nvars, const int *cnf, int *model);

/* Run the DPLL self-test (SAT / UNSAT / unit-propagation / backtracking). */
int smt_self_test(char *out, size_t cap);

#endif /* BEBOP_SMT_H */
