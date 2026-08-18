/* Bebop termination checker — structural recursion verification. */
#ifndef BEBOP_TERMINATION_H
#define BEBOP_TERMINATION_H

#include <stddef.h>

#include "qtt.h"

/* Verify a term's recursion is structural. Returns 0 on success, -1 on a
 * violation (err filled). */
int qtt_termination_check(const Term *t, char *err, size_t cap);

/* Run the termination self-test. Returns 0 on success. */
int qtt_termination_test(char *out, size_t cap);

#endif /* BEBOP_TERMINATION_H */
