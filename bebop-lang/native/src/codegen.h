/* Bebop codegen — emit a WebAssembly module from a core Term (own backend,
 * no LLVM). A `main` function evaluates the term and returns i64 (or i32 for
 * bool). Targets the browser + any WASM runtime (point 10). */
#ifndef BEBOP_CODEGEN_H
#define BEBOP_CODEGEN_H

#include <stddef.h>

#include "qtt.h"

/* Compile a closed term to a complete WASM module (exported `main`). Returns
 * the byte length, or -1 on error (err filled). */
int codegen_wasm(const Term *t, unsigned char *out, size_t cap, char *err,
                 size_t cap_err);

/* Compile a lambda (fn) to a WASM module: a `main` function with the lambda's
 * parameter as a WASM param and the body as its result. */
int codegen_wasm_fn(const Term *lam, unsigned char *out, size_t cap, char *err,
                    size_t cap_err);

int codegen_self_test(char *out, size_t cap);

#endif /* BEBOP_CODEGEN_H */
