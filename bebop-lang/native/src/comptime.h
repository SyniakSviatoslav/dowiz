/* Bebop comptime — compile-time evaluation. */
#ifndef BEBOP_COMPTIME_H
#define BEBOP_COMPTIME_H

#include <stddef.h>

#include "qtt.h"

/* Evaluate a closed, pure term at compile time. Returns 0 on success. */
int bp_comptime_eval(const Term *t, int *out_kind, long *out_i, int *out_b,
                     char *err, size_t cap);

/* Run the comptime self-test. Returns 0 on success. */
int comptime_self_test(char *out, size_t cap);

#endif /* BEBOP_COMPTIME_H */
