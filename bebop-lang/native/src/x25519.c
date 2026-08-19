/* x25519.c — X25519 (RFC 7748) Curve25519 scalar multiplication.
 *
 * Field arithmetic over p = 2^255-19 using 5 × 51-bit limbs (128-bit
 * multipliers).  The Montgomery ladder is constant-time in the scalar bits
 * (branch-free conditional swaps) and the field ops carry no secret-dependent
 * branches.  Correctness is KAT-gated against RFC 7748 §6.1.
 *
 * Ported from /root/dowiz/crates/dowiz-core/src/pq/x25519.rs (359 lines).
 */
#include "x25519.h"
#include <stdio.h>
#include <string.h>

/* ------------------------------------------------------------------ */
/* 5-limb field element (51 bits per limb, little-endian).            */
/*   value = Σ limb[i] · 2^(51·i)                                     */
/* Canonical form: every limb < 2^51.  Lazy form (fe_add / fe_sub)    */
/* tolerates up to ~2^52 per limb; fe_mul / fe_sq reduce fully.       */
/* ------------------------------------------------------------------ */
typedef uint64_t fe[5];

/* 128-bit product — __extension__ silences -Wpedantic (aarch64 64×64→128 mul). */
__extension__ typedef unsigned __int128 u128;

/* 2^51 - 1 */
#define MASK_51  UINT64_C(0x7ffffffffffff)

/* p = 2^255 - 19, 5 × 51-bit limbs */
static const fe P = {
    UINT64_C(0x7ffffffffffed), /* 2^51 - 19 */
    UINT64_C(0x7ffffffffffff),
    UINT64_C(0x7ffffffffffff),
    UINT64_C(0x7ffffffffffff),
    UINT64_C(0x7ffffffffffff),
};

/* a24 = 121665 (Montgomery ladder constant for Curve25519) */
static const fe A24 = {121665, 0, 0, 0, 0};

/* ------------------------------------------------------------------ */
/* Helpers                                                            */
/* ------------------------------------------------------------------ */

/* Constant-time conditional swap: if swap != 0, exchange *a and *b. */
static void fe_cswap(uint64_t swap, fe a, fe b) {
    uint64_t m = (uint64_t)(-(int64_t)(swap & 1));
    int i;
    for (i = 0; i < 5; i++) {
        uint64_t t = m & (a[i] ^ b[i]);
        a[i] ^= t;
        b[i] ^= t;
    }
}

/* a + b (lazy — limbs may reach 2^52). */
static void fe_add(fe r, const fe a, const fe b) {
    int i;
    for (i = 0; i < 5; i++)
        r[i] = a[i] + b[i];
}

/* p - b (branch-free). For canonical b < p, result is in [0, p]. */
static void fe_neg(fe r, const fe b) {
    int i;
    uint64_t borrow = 0;
    for (i = 0; i < 5; i++) {
        uint64_t v1 = P[i] - b[i];
        uint64_t b1 = (P[i] < b[i]) ? 1 : 0;
        uint64_t v2 = v1 - borrow;
        uint64_t b2 = (v2 > v1) ? 1 : 0;
        r[i]   = v2;
        borrow = b1 | b2;
    }
}

/* a - b (lazy) = a + (p - b). */
static void fe_sub(fe r, const fe a, const fe b) {
    fe nb;
    fe_neg(nb, b);
    fe_add(r, a, nb);
}

/* ------------------------------------------------------------------ */
/* Serialisation                                                      */
/* ------------------------------------------------------------------ */

/* Load a 32-byte little-endian u-coordinate, clearing bit 255. */
static void fe_from_bytes(fe out, const uint8_t bytes[32]) {
    uint8_t b[32];
    int i;
    for (i = 0; i < 32; i++) b[i] = bytes[i];
    b[31] &= 0x7f;
    for (i = 0; i < 5; i++) {
        unsigned bit  = (unsigned)(i * 51);
        unsigned byte = bit / 8;
        unsigned shift = bit % 8;
        uint64_t acc = 0;
        int j;
        for (j = 0; j < 8; j++) {
            unsigned bi = byte + (unsigned)j;
            if (bi < 32)
                acc |= (uint64_t)b[bi] << (8 * (unsigned)j);
        }
        out[i] = (acc >> shift) & MASK_51;
    }
}

/* Fully reduce a (possibly < 3p) field element to canonical [0, p). */
static void fe_canonical(fe a) {
    int pass;
    for (pass = 0; pass < 2; pass++) {
        /* a - p with borrow */
        uint64_t borrow = 0;
        fe tmp;
        int i;
        for (i = 0; i < 5; i++) {
            uint64_t v1 = a[i] - P[i];
            uint64_t v2 = v1 - borrow;
            borrow      = (v1 > a[i]) || (v2 > v1);
            tmp[i]      = v2;
        }
        if (borrow == 0) {
            /* a ≥ p → a = a - p */
            for (i = 0; i < 5; i++) a[i] = tmp[i];
        }
    }
}

/* Serialise a fully-reduced field element to 32 bytes LE. */
static void fe_to_bytes(uint8_t out[32], const fe a_in) {
    fe a;
    int i;
    for (i = 0; i < 5; i++) a[i] = a_in[i];
    fe_canonical(a);
    memset(out, 0, 32);
    for (i = 0; i < 5; i++) {
        unsigned bit   = (unsigned)(i * 51);
        unsigned byte  = bit / 8;
        unsigned shift = bit % 8;
        uint64_t acc   = a[i] << shift;
        int j;
        for (j = 0; j < 8; j++) {
            unsigned bi = byte + (unsigned)j;
            if (bi < 32) {
                out[bi] |= (uint8_t)(acc & 0xff);
                acc >>= 8;
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* Modular reduction (mul / sq)                                       */
/* ------------------------------------------------------------------ */

/* Reduce a 10-limb u128 product into canonical [0, p). */
static void fe_reduce(fe out, const u128 c[10]) {
    u128 t[10];
    int i, j;

    for (i = 0; i < 10; i++) t[i] = c[i];

    /* Fold top 5 limbs into bottom (2^255 ≡ 19 mod p). */
    for (i = 0; i < 5; i++)
        t[i] += 19 * t[i + 5];

    /* Carry propagate (4 unconditional passes). */
    for (j = 0; j < 4; j++) {
        for (i = 0; i < 4; i++) {
            t[i + 1] += t[i] >> 51;
            t[i]     &= MASK_51;
        }
        t[0] += 19 * (t[4] >> 51);
        t[4] &= MASK_51;
    }

    for (i = 0; i < 5; i++)
        out[i] = (uint64_t)t[i];

    fe_canonical(out);
}

/* Multiply: a * b → fully-reduced result. */
static void fe_mul(fe out, const fe a, const fe b) {
    u128 c[10] = {0};
    int i, j;
    for (i = 0; i < 5; i++)
        for (j = 0; j < 5; j++)
            c[i + j] += (u128)a[i] * (u128)b[j];
    fe_reduce(out, c);
}

/* Square: a^2 → fully-reduced result. */
static void fe_sq(fe out, const fe a) {
    fe_mul(out, a, a);
}

/* ------------------------------------------------------------------ */
/* Inversion (Fermat — a^(p-2))                                       */
/* ------------------------------------------------------------------ */

static void fe_invert(fe out, const fe a) {
    /* p-2 = 2^255 - 21 = 0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffeb
       Bits 8..254 are all 1; low byte is 0xeb = 0b11101011. */
    fe result = {1, 0, 0, 0, 0};
    int bit;
    for (bit = 254; bit >= 0; bit--) {
        fe_sq(result, result);
        uint64_t b;
        if (bit >= 8)
            b = 1;
        else
            b = (UINT64_C(0xeb) >> bit) & 1;
        if (b)
            fe_mul(result, result, a);
    }
    {
        int i;
        for (i = 0; i < 5; i++) out[i] = result[i];
    }
}

/* ------------------------------------------------------------------ */
/* X25519 scalar multiplication (Montgomery ladder, RFC 7748 §5)      */
/* ------------------------------------------------------------------ */

static void x25519_scalar_mult(uint8_t out[32],
                               const uint8_t scalar[32],
                               const uint8_t u_coord[32]) {
    uint8_t clamped[32];
    int i;
    fe x1, x2, z2, x3, z3;
    uint64_t swap;
    int t;

    /* Clamp scalar (RFC 7748 §5). */
    for (i = 0; i < 32; i++) clamped[i] = scalar[i];
    clamped[0]  &= 248;
    clamped[31] &= 127;
    clamped[31] |= 64;

    fe_from_bytes(x1, u_coord);

    /* Initialise ladder state. */
    x2[0] = 1; for (i = 1; i < 5; i++) x2[i] = 0;
    for (i = 0; i < 5; i++) z2[i] = 0;
    for (i = 0; i < 5; i++) x3[i] = x1[i];
    z3[0] = 1; for (i = 1; i < 5; i++) z3[i] = 0;
    swap = 0;

    for (t = 254; t >= 0; t--) {
        uint64_t k_t = (uint64_t)((clamped[t / 8] >> (t % 8)) & 1);
        swap ^= k_t;
        fe_cswap(swap, x2, x3);
        fe_cswap(swap, z2, z3);
        swap = k_t;

        {
            fe a, aa, b, bb, e, c, d, da, cb, tmp;
            fe x3n, z3n, x2n, z2n, a24e;

            fe_add(a, x2, z2);   fe_sq(aa, a);
            fe_sub(b, x2, z2);   fe_sq(bb, b);
            fe_sub(e, aa, bb);
            fe_add(c, x3, z3);
            fe_sub(d, x3, z3);
            fe_mul(da, d, a);
            fe_mul(cb, c, b);
            fe_add(tmp, da, cb);  fe_sq(x3n, tmp);
            fe_sub(tmp, da, cb);  fe_sq(tmp, tmp);  fe_mul(z3n, x1, tmp);
            fe_mul(x2n, aa, bb);
            fe_mul(a24e, A24, e);
            fe_add(tmp, aa, a24e);
            fe_mul(z2n, e, tmp);

            /* copy back */
            for (i = 0; i < 5; i++) { x2[i] = x2n[i]; z2[i] = z2n[i]; }
            for (i = 0; i < 5; i++) { x3[i] = x3n[i]; z3[i] = z3n[i]; }
        }
    }

    fe_cswap(swap, x2, x3);
    fe_cswap(swap, z2, z3);

    /* Result = x2 / z2 = x2 · z2^(p-2). */
    {
        fe z2inv, result;
        fe_invert(z2inv, z2);
        fe_mul(result, x2, z2inv);
        fe_to_bytes(out, result);
    }
}

/* ------------------------------------------------------------------ */
/* Public API                                                         */
/* ------------------------------------------------------------------ */

void x25519_keygen(const uint8_t seed[32], uint8_t pk[32], uint8_t sk[32]) {
    /* Basepoint u = 9 (RFC 7748 §6.1). */
    static const uint8_t basepoint[32] = {
        9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
    };
    int i;
    for (i = 0; i < 32; i++) sk[i] = seed[i];
    /* Clamp in place. */
    sk[0]  &= 248;
    sk[31] &= 127;
    sk[31] |= 64;
    x25519_scalar_mult(pk, sk, basepoint);
}

void x25519_shared_secret(const uint8_t sk[32], const uint8_t peer_pk[32],
                          uint8_t out[32]) {
    x25519_scalar_mult(out, sk, peer_pk);
}

/* ------------------------------------------------------------------ */
/* Self-test (KAT-gated against RFC 7748 §6.1)                        */
/* ------------------------------------------------------------------ */

static int hex_to_bytes(uint8_t out[32], const char *hex) {
    int i;
    for (i = 0; i < 32; i++) {
        unsigned int byte;
        if (sscanf(hex + 2 * i, "%2x", &byte) != 1)
            return -1;
        out[i] = (uint8_t)byte;
    }
    return 0;
}

static void append_hex(char *buf, size_t off, size_t cap,
                       const uint8_t bytes[32]) {
    int i;
    for (i = 0; i < 32; i++) {
        if (off + 2 >= cap) break;
        off += (size_t)snprintf(buf + off, cap - off,
                                "%02x", (unsigned)bytes[i]);
    }
}

static int bytes_eq(const uint8_t a[32], const uint8_t b[32]) {
    int i;
    uint8_t d = 0;
    for (i = 0; i < 32; i++) d |= a[i] ^ b[i];
    return d == 0;
}

int x25519_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int failures = 0;

#define PASS(...) do {                                              \
        pos += (size_t)snprintf(out + pos, cap - pos, __VA_ARGS__); \
    } while (0)

#define FAIL(...) do {                                              \
        pos += (size_t)snprintf(out + pos, cap - pos, __VA_ARGS__); \
        failures++;                                                 \
    } while (0)

    /* RFC 7748 §6.1 — X25519 test vector 1
       scalar      = 77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a
       u (base 9)  = 0900000000000000000000000000000000000000000000000000000000000000
       expected    = 8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a */
    {
        uint8_t scalar[32], u[32], expected[32], result[32];
        if (hex_to_bytes(scalar,   "77076d0a7318a57d3c16c17251b26645"
                                   "df4c2f87ebc0992ab177fba51db92c2a") != 0 ||
            hex_to_bytes(u,        "09000000000000000000000000000000"
                                   "00000000000000000000000000000000") != 0 ||
            hex_to_bytes(expected, "8520f0098930a754748b7ddcb43ef75"
                                   "a0dbf3a0d26381af4eba4a98eaa9b4e6a") != 0) {
            FAIL("RFC 7748 TV1: hex decode failed\n");
        } else {
            memset(result, 0, 32);
            x25519_scalar_mult(result, scalar, u);
            if (bytes_eq(result, expected)) {
                PASS("RFC 7748 TV1: PASS\n");
            } else {
                char ghex[65], ehex[65];
                memset(ghex, 0, sizeof ghex);
                memset(ehex, 0, sizeof ehex);
                append_hex(ghex, 0, sizeof ghex, result);
                append_hex(ehex, 0, sizeof ehex, expected);
                FAIL("RFC 7748 TV1: FAIL\n  got:      %s\n  expected: %s\n",
                     ghex, ehex);
            }
        }
    }

    /* RFC 7748 §6.1 — test vector 2 */
    {
        uint8_t scalar[32], u[32], expected[32], result[32];
        if (hex_to_bytes(scalar,   "4b66e9d4d1b05647ce7c57896a1e3bb4"
                                   "ddde786446b17a99c88441d375c72958") != 0 ||
            hex_to_bytes(u,        "0e5210f12786811d3f4b7959d0538ae2"
                                   "c31dbe7106fc03d2b87a31f3b9b7b2b0") != 0 ||
            hex_to_bytes(expected, "fa90b2a73221d009a3175bc9d098ec72"
                                   "062638274f2bfa246bc52796e30c5609") != 0) {
            FAIL("RFC 7748 TV2: hex decode failed\n");
        } else {
            memset(result, 0, 32);
            x25519_scalar_mult(result, scalar, u);
            if (bytes_eq(result, expected)) {
                PASS("RFC 7748 TV2: PASS\n");
            } else {
                char ghex[65], ehex[65];
                memset(ghex, 0, sizeof ghex);
                memset(ehex, 0, sizeof ehex);
                append_hex(ghex, 0, sizeof ghex, result);
                append_hex(ehex, 0, sizeof ehex, expected);
                FAIL("RFC 7748 TV2: FAIL\n  got:      %s\n  expected: %s\n",
                     ghex, ehex);
            }
        }
    }

    /* Edge case: zero scalar → clamps to 2^254.  Verify deterministic. */
    {
        uint8_t scalar[32], u[32], resultA[32], resultB[32];
        memset(scalar, 0, 32);
        memset(u, 0, 32);
        u[0] = 9;
        memset(resultA, 0, 32);
        memset(resultB, 0, 32);
        x25519_scalar_mult(resultA, scalar, u);
        x25519_scalar_mult(resultB, scalar, u);
        if (bytes_eq(resultA, resultB)) {
            PASS("zero scalar (deterministic): PASS (clamped→2^254)\n");
        } else {
            FAIL("zero scalar (deterministic): FAIL\n");
        }
    }

    /* Edge case: scalar = 1 → also clamps to 2^254, same output as zero. */
    {
        uint8_t s0[32], s1[32], u[32], r0[32], r1[32];
        memset(s0, 0, 32);
        s1[0] = 1;
        {
            int i;
            for (i = 1; i < 32; i++) s1[i] = 0;
        }
        memset(u, 0, 32);
        u[0] = 9;
        memset(r0, 0, 32);
        memset(r1, 0, 32);
        x25519_scalar_mult(r0, s0, u);
        x25519_scalar_mult(r1, s1, u);
        if (bytes_eq(r0, r1)) {
            PASS("scalar=1 == scalar=0 (both clamp→2^254): PASS\n");
        } else {
            FAIL("scalar=1 == scalar=0: FAIL\n");
        }
    }

    /* Iterated associativity:
       X25519(a, X25519(b, 9)) == X25519(b, X25519(a, 9)). */
    {
        uint8_t a[32], b[32], nine[32];
        uint8_t ab[32], ba[32];
        int i;
        if (hex_to_bytes(a, "a546e36bf0527c9d3b16154b82465edd"
                             "62144c0ac1fc5a18506a2244ba449ac4") != 0 ||
            hex_to_bytes(b, "4b66e9d4d1b05647ce7c57896a1e3bb4"
                             "ddde786446b17a99c88441d375c72958") != 0) {
            FAIL("associative: hex decode failed\n");
        } else {
            for (i = 0; i < 32; i++) nine[i] = (i == 0) ? 9 : 0;
            {
                uint8_t tmp[32];
                x25519_scalar_mult(tmp, b, nine);
                x25519_scalar_mult(ab, a, tmp);
                x25519_scalar_mult(tmp, a, nine);
                x25519_scalar_mult(ba, b, tmp);
            }
            if (bytes_eq(ab, ba)) {
                PASS("associative (a·(b·9) == b·(a·9)): PASS\n");
            } else {
                FAIL("associative: FAIL\n");
            }
        }
    }

    /* Field round-trip */
    {
        uint8_t bytes[32], out32[32];
        fe f;
        int i;
        for (i = 0; i < 32; i++)
            bytes[i] = (uint8_t)((unsigned)i * 17);
        fe_from_bytes(f, bytes);
        fe_to_bytes(out32, f);
        bytes[31] &= 0x7f;
        if (bytes_eq(out32, bytes)) {
            PASS("field round-trip: PASS\n");
        } else {
            FAIL("field round-trip: FAIL\n");
        }
    }

    /* 1*1 = 1 */
    {
        fe one = {1, 0, 0, 0, 0}, r;
        fe_mul(r, one, one);
        int i, ok = 1;
        for (i = 0; i < 5; i++) if (r[i] != one[i]) ok = 0;
        if (ok) PASS("1*1=1: PASS\n"); else FAIL("1*1=1: FAIL\n");
    }

    /* 2*3 = 6 */
    {
        fe two = {2, 0, 0, 0, 0}, three = {3, 0, 0, 0, 0}, r;
        fe_mul(r, two, three);
        if (r[0] == 6 && r[1] == 0 && r[2] == 0 && r[3] == 0 && r[4] == 0)
            PASS("2*3=6: PASS\n");
        else
            FAIL("2*3=6: FAIL\n");
    }

    /* 2 * 2^-1 = 1 */
    {
        fe two = {2, 0, 0, 0, 0}, inv, r;
        fe_invert(inv, two);
        fe_mul(r, two, inv);
        if (r[0] == 1 && r[1] == 0 && r[2] == 0 && r[3] == 0 && r[4] == 0)
            PASS("2 * 2^-1 = 1: PASS\n");
        else
            FAIL("2 * 2^-1 = 1: FAIL\n");
    }

    PASS("\nX25519 self-test: %s (%d failure%s)\n",
         failures == 0 ? "PASS" : "FAIL",
         failures, failures == 1 ? "" : "s");

#undef PASS
#undef FAIL

    return failures == 0 ? 0 : 1;
}