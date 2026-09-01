/* Bebop VIR — vector umulh (2×64 multiply-high), synthesized from NEON.
 *
 * AArch64 NEON has NO native 2×64 multiply-high (the scalar `umulh x,x,x`
 * exists, but no vector form). We synthesize it via 32×32→64 UMULL
 * decomposition so the hot loop stays fully vectorized (scalars = fallback):
 *
 *   a = a1·2³² + a0,  b = b1·2³² + b0
 *   hi64(a·b) = a1·b1 + ((a1·b0 + a0·b1 + (a0·b0 >> 32)) >> 32)
 *
 * Uses arm_neon.h intrinsics (compiled once; not the runtime JIT path).
 */
#include "vir.h"
#include <arm_neon.h>
#include <stddef.h>

int vir_umulh2(Vir128 a, Vir128 b, Vir128 *out, char *err, size_t cap_err) {
    (void)err;
    (void)cap_err;
    uint64x2_t va = vld1q_u64((const uint64_t *)&a);
    uint64x2_t vb = vld1q_u64((const uint64_t *)&b);

    /* a0/b0 = low 32 bits per lane, a1/b1 = high 32 bits per lane */
    uint32x2_t a0 = vmovn_u64(va);
    uint32x2_t b0 = vmovn_u64(vb);
    uint32x2_t a1 = vshrn_n_u64(va, 32);
    uint32x2_t b1 = vshrn_n_u64(vb, 32);

    /* t0 = a0·b0, t2 = a1·b1, t1 = a1·b0 + a0·b1  (all 32×32→64) */
    uint64x2_t t0 = vmull_u32(a0, b0);
    uint64x2_t t2 = vmull_u32(a1, b1);
    uint64x2_t t1 = vaddq_u64(vmull_u32(a1, b0), vmull_u32(a0, b1));

    /* hi = t2 + ((t1 + (t0 >> 32)) >> 32) */
    uint64x2_t hi = vaddq_u64(t2,
        vshrq_n_u64(vaddq_u64(t1, vshrq_n_u64(t0, 32)), 32));

    vst1q_u64((uint64_t *)out, hi);
    return 0;
}
