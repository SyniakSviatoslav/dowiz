/* Bebop VIR — vector intermediate representation (6B).
 *
 * A thin, target-portable layer of 128-bit SIMD ops that the native backend
 * lowers to hand-encoded AArch64 NEON (no arm_neon.h, no LLVM, no assembler at
 * runtime — the encodings below were derived once and are emitted directly).
 * The same VIR op lowers to wasm simd128 on the WebAssembly backend.
 *
 * Fixed ABI for the JIT'd shim:  fn(const u64 *a, const u64 *b, u64 *out)
 *   ld1 {v1}, [x0];  ld1 {v2}, [x1];  <op> v0, v1, v2;  st1 {v0}, [x2];  ret
 */
#ifndef BEBOP_VIR_H
#define BEBOP_VIR_H

#include <stddef.h>
#include <stdint.h>

typedef enum {
    VIR_ADD_2D,  /* 2×i64 element-wise add  */
    VIR_SUB_2D,  /* 2×i64 element-wise sub  */
    VIR_ADD_4S,  /* 4×i32 element-wise add  */
    VIR_SUB_4S,  /* 4×i32 element-wise sub  */
    VIR_MUL_4S,  /* 4×i32 element-wise mul  */
    VIR_FADD_2D, /* 2×f64 element-wise add  */
    VIR_FSUB_2D, /* 2×f64 element-wise sub  */
    VIR_FMUL_2D, /* 2×f64 element-wise mul  */
} VirOp;

/* A 128-bit SIMD register image (2×u64 or 2×f64 or 4×u32 lanes). */
typedef struct {
    uint64_t lo, hi;
} Vir128;

/* JIT v0 = op(v1, v2) with hand-encoded NEON. Returns 0, or -1 (err filled). */
int vir_binop(VirOp op, Vir128 a, Vir128 b, Vir128 *out, char *err,
              size_t cap_err);

/* 2×64 multiply-high (synthesized vector umulh). AArch64 NEON has no native
 * vector umulh, so we build it from UMULL decomposition. Returns 0 or -1. */
int vir_umulh2(Vir128 a, Vir128 b, Vir128 *out, char *err, size_t cap_err);

/* Atomic machine-code ops (⚛): hand-encoded LSE atomics — no libatomic, no
 * compiler intrinsic. Each returns the OLD value at *ptr. */
uint64_t vir_atomic_add(uint64_t *ptr, uint64_t delta);
uint64_t vir_atomic_cas(uint64_t *ptr, uint64_t expected, uint64_t desired);

int vir_self_test(char *out, size_t cap);
int vir_atomic_self_test(char *out, size_t cap);

#endif /* BEBOP_VIR_H */
