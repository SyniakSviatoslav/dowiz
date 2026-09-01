/* Bebop PQ — Keccak-f[1600] + SHAKE128/256 + SHA3-256/512 (port of dowiz
 * crates/dowiz-core/src/pq/keccak.rs, FIPS 202). Zero external dependencies;
 * the permutation/sponge are pure C (no libc). */
#include "pq.h"

#include <stdio.h>
#include <string.h>

/* ── Keccak-f[1600] ─────────────────────────────────────────────────────────
 * State = 25 lanes of u64. Round constants (RC) and rotation offsets (RHO)
 * per FIPS 202 §3.2.1 / §3.2.2. */
static const uint64_t RC[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL, 0x800000000000808aULL,
    0x8000000080008000ULL, 0x000000000000808bULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL, 0x000000000000008aULL,
    0x0000000000000088ULL, 0x0000000080008009ULL, 0x000000008000000aULL,
    0x000000008000808bULL, 0x800000000000008bULL, 0x8000000000008089ULL,
    0x8000000000008003ULL, 0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800aULL, 0x800000008000000aULL, 0x8000000080008081ULL,
    0x8000000000008080ULL, 0x0000000080000001ULL, 0x8000000080008008ULL,
};

/* Rotation offsets r[x][y] (FIPS 202 §3.2.2), flat-indexed by x + 5*y. */
static const uint32_t RHO[25] = {
    0,  1,  62, 28, 27, 36, 44, 6,  55, 20, 3,  10, 43,
    25, 39, 41, 45, 15, 21, 8,  18, 2,  61, 56, 14,
};

static inline uint64_t rotl64(uint64_t x, uint32_t n) {
    return n == 0 ? x : (x << n) | (x >> (64 - n));
}

/* One Keccak-f[1600] permutation in place. */
void keccak_f(uint64_t state[25]) {
    for (int round = 0; round < 24; round++) {
        /* Theta */
        uint64_t c[5], d[5];
        for (int x = 0; x < 5; x++) {
            c[x] = state[x] ^ state[x + 5] ^ state[x + 10] ^ state[x + 15] ^
                   state[x + 20];
        }
        for (int x = 0; x < 5; x++) {
            d[x] = c[(x + 4) % 5] ^ rotl64(c[(x + 1) % 5], 1);
        }
        for (int x = 0; x < 5; x++) {
            for (int y = 0; y < 5; y++) {
                state[x + 5 * y] ^= d[x];
            }
        }
        /* Rho + Pi: rotated lane A[col=x][row=y] lands at
         * B[col=y][row=(2x+3y)%5], i.e. index y + 5*((2x+3y)%5). */
        uint64_t b[25];
        for (int x = 0; x < 5; x++) {
            for (int y = 0; y < 5; y++) {
                b[y + 5 * ((2 * x + 3 * y) % 5)] =
                    rotl64(state[x + 5 * y], RHO[x + 5 * y]);
            }
        }
        /* Chi */
        for (int y = 0; y < 5; y++) {
            uint64_t row[5];
            for (int x = 0; x < 5; x++) {
                row[x] = b[5 * y + x];
            }
            for (int x = 0; x < 5; x++) {
                state[x + 5 * y] = row[x] ^ ((~row[(x + 1) % 5]) & row[(x + 2) % 5]);
            }
        }
        /* Iota */
        state[0] ^= RC[round];
    }
}

/* Sponge over Keccak-f[1600]. `rate` is the block size in bytes (168 for
 * SHAKE128, 136 for SHAKE256/SHA3-256, 72 for SHA3-512). `pad` is the domain
 * suffix byte (0x1f for SHAKE, 0x06 for SHA-3). Squeezes exactly outlen bytes.
 * Streaming absorb + a small stack tail buffer, so no allocation is needed. */
static void sponge(size_t rate, uint8_t pad, const uint8_t *input, size_t inlen,
                   uint8_t *out, size_t outlen) {
    uint64_t state[25];
    for (int i = 0; i < 25; i++) {
        state[i] = 0;
    }
    size_t pos = 0;
    /* Absorb full rate-blocks. */
    while (pos + rate <= inlen) {
        for (size_t j = 0; j < rate; j++) {
            state[j / 8] ^= (uint64_t)input[pos + j] << ((j % 8) * 8);
        }
        keccak_f(state);
        pos += rate;
    }
    /* pad10*1 with the SHAKE/SHA-3 domain suffix (the suffix byte already
     * carries the leading `1` as its LSB); zero-fill then set the trailing
     * `1` (0x80). The tail spans one or two rate-blocks. */
    size_t rem = inlen - pos;
    size_t t = rem + 1;
    size_t z;
    if (t % rate == rate - 1) {
        z = 0;
    } else {
        z = rate - 1 - (t % rate);
    }
    uint8_t tail[2 * 168];
    size_t tl = 0;
    for (size_t i = 0; i < rem; i++) {
        tail[tl++] = input[pos + i];
    }
    tail[tl++] = pad;
    for (size_t i = 0; i < z; i++) {
        tail[tl++] = 0;
    }
    tail[tl++] = 0x80;
    for (size_t off = 0; off < tl; off += rate) {
        for (size_t j = 0; j < rate; j++) {
            state[j / 8] ^= (uint64_t)tail[off + j] << ((j % 8) * 8);
        }
        keccak_f(state);
    }
    /* Squeeze */
    size_t produced = 0;
    while (produced < outlen) {
        for (size_t lane = 0; lane < rate / 8; lane++) {
            uint64_t v = state[lane];
            for (size_t k = 0; k < 8; k++) {
                size_t idx = produced + lane * 8 + k;
                if (idx < outlen) {
                    out[idx] = (uint8_t)(v & 0xff);
                }
                v >>= 8;
            }
        }
        produced += rate;
        if (produced < outlen) {
            keccak_f(state);
        }
    }
}

void shake128(const uint8_t *input, size_t inlen, uint8_t *out, size_t outlen) {
    sponge(168, 0x1f, input, inlen, out, outlen);
}

void shake256(const uint8_t *input, size_t inlen, uint8_t *out, size_t outlen) {
    sponge(136, 0x1f, input, inlen, out, outlen);
}

void sha3_256(const uint8_t *input, size_t inlen, uint8_t out[32]) {
    sponge(136, 0x06, input, inlen, out, 32);
}

void sha3_512(const uint8_t *input, size_t inlen, uint8_t out[64]) {
    sponge(72, 0x06, input, inlen, out, 64);
}

/* Compare `got` against a hex string literal (FIPS 202 KAT anchors). */
static int eq_hex(const uint8_t *got, const char *hex) {
    size_t n = strlen(hex) / 2;
    for (size_t i = 0; i < n; i++) {
        unsigned char hi = (unsigned char)hex[2 * i];
        unsigned char lo = (unsigned char)hex[2 * i + 1];
        unsigned char b;
        hi = (hi >= '0' && hi <= '9')
                 ? (unsigned char)(hi - '0')
                 : (unsigned char)((hi >= 'a' ? hi - 'a' : hi - 'A') + 10);
        lo = (lo >= '0' && lo <= '9')
                 ? (unsigned char)(lo - '0')
                 : (unsigned char)((lo >= 'a' ? lo - 'a' : lo - 'A') + 10);
        b = (unsigned char)((hi << 4) | lo);
        if (got[i] != b) {
            return 0;
        }
    }
    return 1;
}

int pq_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) {                                                          \
            pos += (size_t)r_;                                                 \
        }                                                                      \
        if (!c_) {                                                             \
            all_ok = 0;                                                        \
        }                                                                      \
    } while (0)

    uint8_t buf[64];

    /* SHAKE256("") = 46b9...5762f (FIPS 202 KAT). */
    shake256((const uint8_t *)"", 0, buf, 32);
    A(eq_hex(buf, "46b9dd2b0ba88d13233b3feb743eeb24"
                  "3fcd52ea62b81b82b50c27646ed5762f"),
      "SHAKE256(\"\")");

    /* SHAKE128("") first 16 bytes = 7f9c...853e. */
    shake128((const uint8_t *)"", 0, buf, 16);
    A(eq_hex(buf, "7f9c2ba4e88f827d616045507605853e"), "SHAKE128(\"\")[0..16)");

    /* SHA3-256("") = a7ff...434a. */
    sha3_256((const uint8_t *)"", 0, buf);
    A(eq_hex(buf, "a7ffc6f8bf1ed76651c14756a061d662"
                  "f580ff4de43b49fa82d80a4b80f8434a"),
      "SHA3-256(\"\")");

    /* SHA3-512("") = a69f...cd26. */
    sha3_512((const uint8_t *)"", 0, buf);
    A(eq_hex(buf, "a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859"
                  "e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558"
                  "f500199d95b6d3e301758586281dcd26"),
      "SHA3-512(\"\")");

    /* SHAKE256("abc") = 4833...5739 (FIPS 202 KAT). */
    shake256((const uint8_t *)"abc", 3, buf, 32);
    A(eq_hex(buf, "483366601360a8771c6863080cc4114"
                  "d8db44530f8f1e1ee4f94ea37e78b5739"),
      "SHAKE256(\"abc\")");

    return all_ok ? 0 : -1;
}
