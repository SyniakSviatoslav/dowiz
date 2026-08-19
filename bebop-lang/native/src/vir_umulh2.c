/* Bebop VIR — vector umulh (2×64 multiply-high), synthesized from NEON UMULL.
 *
 * AArch64 NEON has NO native 2×64 multiply-high (the scalar `umulh x,x,x`
 * exists, but no vector form). We synthesize it via 32×32→64 UMULL
 * decomposition so the hot loop stays fully vectorized (scalars = fallback only):
 *
 *   a = a1·2³² + a0,  b = b1·2³² + b0
 *   hi64(a·b) = a1·b1 + ((a1·b0 + a0·b1 + (a0·b0 >> 32)) >> 32)
 *
 * All 11 steps are NEON 2-lane ops. Encodings verified against objdump.
 */
#include "vir.h"
#include <string.h>
#include <sys/mman.h>
#include <stdio.h>

#define NEON_UMULL   0x2EA2C000u  /* umull  vd.2d, vn.2s, vm.2s  (low×low→64) */
#define NEON_UMULL2  0x6EA2C000u  /* umull2 vd.2d, vn.4s, vm.4s  (high×high)   */
#define NEON_USHR    0x6F600400u  /* ushr   vd.2d, vn.2d, #32                 */
#define NEON_ADD2D   0x4EE08400u  /* add    vd.2d, vn.2d, vm.2d               */

#define MAX_INS 32
static unsigned int code[MAX_INS];
static size_t clen;

static void e(unsigned int ins) {
    if (clen < MAX_INS) code[clen] = ins;
    clen++;
}

/* v0.2d = hi64(v1.2d ⊗ v2.2d) — synthesized vector umulh.
 * Returns 0, or -1 (err filled). */
int vir_umulh2(Vir128 a, Vir128 b, Vir128 *out, char *err, size_t cap_err) {
    clen = 0;
    e(0x4C407C01u); /* ld1 {v1}, [x0] — a */
    e(0x4C407C22u); /* ld1 {v2}, [x1] — b */

    /* 1. t0 = a0*b0 (low×low) → v3 */
    e(NEON_UMULL  | (2u << 16) | (1u << 5) | 3u);
    /* 2. t2 = a1*b1 (high×high) → v4 */
    e(NEON_UMULL2 | (2u << 16) | (1u << 5) | 4u);
    /* 3. a_hi = a >> 32 → v5 */
    e(NEON_USHR   | (1u << 5) | 5u);
    /* 4. b_hi = b >> 32 → v6 */
    e(NEON_USHR   | (2u << 5) | 6u);
    /* 5. t1a = a_hi * b0 (umull reads low 32 of v5 = a_hi) → v7 */
    e(NEON_UMULL  | (2u << 16) | (5u << 5) | 7u);
    /* 6. t1b = a0 * b_hi → v8 */
    e(NEON_UMULL  | (6u << 16) | (1u << 5) | 8u);
    /* 7. t1 = t1a + t1b → v9 */
    e(NEON_ADD2D  | (8u << 16) | (7u << 5) | 9u);
    /* 8. t0_hi = t0 >> 32 → v10 */
    e(NEON_USHR   | (3u << 5) | 10u);
    /* 9. t1' = t1 + t0_hi → v11 */
    e(NEON_ADD2D  | (10u << 16) | (9u << 5) | 11u);
    /* 10. t1'' = t1' >> 32 → v12 */
    e(NEON_USHR   | (11u << 5) | 12u);
    /* 11. hi = t2 + t1'' → v0 */
    e(NEON_ADD2D  | (12u << 16) | (4u << 5) | 0u);

    e(0x4C007C00u); /* st1 {v0}, [x2] — out */
    e(0xD65F03C0u); /* ret */

    size_t sz = clen * sizeof(unsigned int);
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE,
                     MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (mem == MAP_FAILED) {
        snprintf(err, cap_err, "vir_umulh2: mmap failed");
        return -1;
    }
    memcpy(mem, code, sz);
    __builtin___clear_cache((char *)mem, (char *)mem + sz);
    if (mprotect(mem, sz, PROT_READ | PROT_EXEC) != 0) {
        munmap(mem, sz);
        snprintf(err, cap_err, "vir_umulh2: mprotect W^X failed");
        return -1;
    }
    void (*fn)(const uint64_t *, const uint64_t *, uint64_t *);
    memcpy(&fn, &mem, sizeof(fn));
    fn((const uint64_t *)&a, (const uint64_t *)&b, (uint64_t *)out);
    munmap(mem, sz);
    return 0;
}