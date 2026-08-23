/* Bebop native backend — aarch64 machine code (own JIT, no LLVM).
 * Emits a stack-machine function (i64 arithmetic) directly to AArch64
 * instructions, mmaps it executable, and calls it. */
#ifndef BEBOP_NATIVE_H
#define BEBOP_NATIVE_H

#include <stddef.h>

#include "qtt.h"

/* Compile + run a closed i64 term as native AArch64 machine code. Returns the
 * result. On error, fills `err` and returns 0. */
long native_eval(const Term *t, char *err, size_t cap);

/* Compile-only variant: emits the same words native_eval would run, into out[]
 * (capacity cap words). Returns word count, or -1 on error. */
int native_compile_words(const Term *t, unsigned int *out, size_t cap,
                         char *err, size_t cerr);

int native_self_test(char *out, size_t cap);

#endif /* BEBOP_NATIVE_H */
