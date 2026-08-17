/* Bebop verification — bounded contract checking with counterexamples.
 * "Self-Healing SMT" primitive: requires P(x) ⟹ ensures Q(x, body(x)) is
 * checked over a bounded domain; a violating input is reported as a concrete
 * counterexample (the mathematical counterexample the agent self-corrects on). */
#ifndef BEBOP_VERIFY_H
#define BEBOP_VERIFY_H

#include <stddef.h>

/* Check: forall x in [lo,hi], if requires(x) then ensures(x, body(x)).
 * Returns 0 on success (out = "verified (N cases)"), -1 if a counterexample
 * is found (out = the counterexample) or on parse/eval error. */
int verify_bounded(const char *body, const char *requires, const char *ensures,
                   long lo, long hi, char *out, size_t cap);

/* Generate the SMT-LIB verification condition for a contract (for Z3/CVC5).
 * Returns 0 on success; the SMT-LIB is written to `out`. */
int verify_smtlib(const char *body, const char *requires, const char *ensures,
                  char *out, size_t cap);

int verify_self_test(char *out, size_t cap);

#endif /* BEBOP_VERIFY_H */
