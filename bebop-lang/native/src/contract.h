/* Bebop contracts → SMT — requires/ensures/invariant verification. */
#ifndef BEBOP_CONTRACT_H
#define BEBOP_CONTRACT_H

#include <stddef.h>

/* Verify `requires -> ensures`. Returns 0 (holds), 1 (counterexample), -1 (error). */
int bp_contract_check(const char *requires, const char *ensures, char *err, size_t cap);

/* Run the contract self-test. Returns 0 on success. */
int contract_self_test(char *out, size_t cap);

#endif /* BEBOP_CONTRACT_H */
