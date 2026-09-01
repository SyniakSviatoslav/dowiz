/* Bebop verifier — automatic safety prover (SPARK/Ada style). */
#ifndef BEBOP_VERIFIER_H
#define BEBOP_VERIFIER_H

#include <stddef.h>

/* Verify ALL safety properties on a Bebop module source.
 * Returns: 0 = all safe, 1 = violations found, -1 = error.
 * Writes proof report to `out`. */
int verifier_prove(const char *source, char *out, size_t cap);

int verifier_self_test(char *out, size_t cap);

#endif