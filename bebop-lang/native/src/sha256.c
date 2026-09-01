/* Bebop sha256 — SHA-256 / SHA-224 / HMAC-SHA256 implementation.
 *
 * Port of dowiz's sha256_hw.rs (the pure-scalar fallback path). Standard
 * FIPS 180-4 construction: 64-entry K table, message schedule, compression
 * function, Merkle-Damgård padding (0x80 … 64-bit big-endian bit length).
 *
 * Deliberately libc-free (only <stdint.h>/<stddef.h>): the core only uses
 * integer arithmetic so it also links in a bare-metal AArch64 context. The
 * only host-isms (snprintf) live inside sha256_self_test for the CLI driver.
 */
#include "sha256.h"

#include <stdint.h>
#include <stddef.h>

/* snprintf is used only inside sha256_self_test (the host-side CLI driver);
 * the hash core itself stays libc-free. */
#include <stdio.h>

typedef struct {
    uint32_t state[8];
    uint64_t bitlen;
    uint8_t buffer[64];
    size_t buflen;
} ShaCtx;

static uint32_t rotr32(uint32_t x, int n) {
    return (x >> n) | (x << (32 - n));
}

/* FIPS 180-4 §4.2.2 — round constants (first 32 bits of fractional parts of
 * the cube roots of the first 64 primes). */
static const uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
};

/* Compress one 64-byte block into state[8]. */
static void sha_transform(uint32_t state[8], const uint8_t block[64]) {
    uint32_t w[64];
    int i;

    for (i = 0; i < 16; i++) {
        w[i] = ((uint32_t)block[i * 4] << 24) |
               ((uint32_t)block[i * 4 + 1] << 16) |
               ((uint32_t)block[i * 4 + 2] << 8) |
               ((uint32_t)block[i * 4 + 3]);
    }
    for (i = 16; i < 64; i++) {
        uint32_t s0 = rotr32(w[i - 15], 7) ^ rotr32(w[i - 15], 18) ^
                      (w[i - 15] >> 3);
        uint32_t s1 = rotr32(w[i - 2], 17) ^ rotr32(w[i - 2], 19) ^
                      (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1; /* u32 wrap is well-defined */
    }

    uint32_t a = state[0], b = state[1], c = state[2], d = state[3];
    uint32_t e = state[4], f = state[5], g = state[6], h = state[7];

    for (i = 0; i < 64; i++) {
        uint32_t S1 = rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25);
        uint32_t ch = (e & f) ^ (~e & g);
        uint32_t temp1 = h + S1 + ch + K[i] + w[i];
        uint32_t S0 = rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22);
        uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
        uint32_t temp2 = S0 + maj;

        h = g;
        g = f;
        f = e;
        e = d + temp1;
        d = c;
        c = b;
        b = a;
        a = temp1 + temp2;
    }

    state[0] += a;
    state[1] += b;
    state[2] += c;
    state[3] += d;
    state[4] += e;
    state[5] += f;
    state[6] += g;
    state[7] += h;
}

static void sha256_init(ShaCtx *ctx) {
    ctx->state[0] = 0x6a09e667;
    ctx->state[1] = 0xbb67ae85;
    ctx->state[2] = 0x3c6ef372;
    ctx->state[3] = 0xa54ff53a;
    ctx->state[4] = 0x510e527f;
    ctx->state[5] = 0x9b05688c;
    ctx->state[6] = 0x1f83d9ab;
    ctx->state[7] = 0x5be0cd19;
    ctx->bitlen = 0;
    ctx->buflen = 0;
}

static void sha224_init(ShaCtx *ctx) {
    ctx->state[0] = 0xc1059ed8;
    ctx->state[1] = 0x367cd507;
    ctx->state[2] = 0x3070dd17;
    ctx->state[3] = 0xf70e5939;
    ctx->state[4] = 0xffc00b31;
    ctx->state[5] = 0x68581511;
    ctx->state[6] = 0x64f98fa7;
    ctx->state[7] = 0xbefa4fa4;
    ctx->bitlen = 0;
    ctx->buflen = 0;
}

static void sha_update(ShaCtx *ctx, const uint8_t *data, size_t len) {
    ctx->bitlen += (uint64_t)len * 8;
    size_t idx = ctx->buflen;
    for (size_t i = 0; i < len; i++) {
        ctx->buffer[idx++] = data[i];
        if (idx == 64) {
            sha_transform(ctx->state, ctx->buffer);
            idx = 0;
        }
    }
    ctx->buflen = idx;
}

static void sha_final(ShaCtx *ctx, uint8_t out[32]) {
    uint64_t bitlen = ctx->bitlen; /* capture before padding mutates the buffer */
    size_t idx = ctx->buflen;

    ctx->buffer[idx++] = 0x80;
    if (idx == 64) {
        sha_transform(ctx->state, ctx->buffer);
        idx = 0;
    }
    while (idx != 56) {
        ctx->buffer[idx++] = 0x00;
        if (idx == 64) {
            sha_transform(ctx->state, ctx->buffer);
            idx = 0;
        }
    }
    for (int i = 0; i < 8; i++) {
        ctx->buffer[idx++] = (uint8_t)(bitlen >> (56 - 8 * i));
    }
    sha_transform(ctx->state, ctx->buffer); /* idx is now exactly 64 */

    for (int i = 0; i < 8; i++) {
        out[i * 4] = (uint8_t)(ctx->state[i] >> 24);
        out[i * 4 + 1] = (uint8_t)(ctx->state[i] >> 16);
        out[i * 4 + 2] = (uint8_t)(ctx->state[i] >> 8);
        out[i * 4 + 3] = (uint8_t)(ctx->state[i]);
    }
}

void sha256(const uint8_t *data, size_t len, uint8_t out[32]) {
    ShaCtx ctx;
    sha256_init(&ctx);
    sha_update(&ctx, data, len);
    sha_final(&ctx, out);
}

void sha224(const uint8_t *data, size_t len, uint8_t out[28]) {
    ShaCtx ctx;
    uint8_t digest[32];
    sha224_init(&ctx);
    sha_update(&ctx, data, len);
    sha_final(&ctx, digest);
    for (int i = 0; i < 28; i++) {
        out[i] = digest[i];
    }
}

void hmac_sha256(const uint8_t *key, size_t key_len,
                 const uint8_t *data, size_t data_len,
                 uint8_t out[32]) {
    uint8_t k[64] = {0};
    uint8_t ipad[64];
    uint8_t opad[64];
    uint8_t inner[32];
    ShaCtx ctx;

    if (key_len > 64) {
        sha256(key, key_len, k); /* hashed key lands in k[0..31], rest zero */
    } else {
        for (size_t i = 0; i < key_len; i++) {
            k[i] = key[i];
        }
    }

    for (int i = 0; i < 64; i++) {
        ipad[i] = k[i] ^ 0x36;
        opad[i] = k[i] ^ 0x5c;
    }

    sha256_init(&ctx);
    sha_update(&ctx, ipad, 64);
    sha_update(&ctx, data, data_len);
    sha_final(&ctx, inner);

    sha256_init(&ctx);
    sha_update(&ctx, opad, 64);
    sha_update(&ctx, inner, 32);
    sha_final(&ctx, out);
}

/* ──────────────────────────────────────────────────────────────────────────
 * Self-test (libc host-side driver)
 * ────────────────────────────────────────────────────────────────────────── */

/* Compare n-byte digest against a lowercase hex string (no libc strtoul). */
static int hex_eq(const uint8_t *digest, size_t n, const char *hex) {
    for (size_t i = 0; i < n; i++) {
        unsigned v = 0;
        for (int j = 0; j < 2; j++) {
            char c = hex[2 * i + j];
            unsigned d;
            if (c >= '0' && c <= '9') {
                d = (unsigned)(c - '0');
            } else if (c >= 'a' && c <= 'f') {
                d = (unsigned)(c - 'a') + 10;
            } else if (c >= 'A' && c <= 'F') {
                d = (unsigned)(c - 'A') + 10;
            } else {
                return 0;
            }
            v = (v << 4) | d;
        }
        if ((uint8_t)v != digest[i]) {
            return 0;
        }
    }
    return 1;
}

/* Byte-wise comparison of two digests (no libc memcmp). */
static int digest_eq(const uint8_t *a, const uint8_t *b, size_t n) {
    for (size_t i = 0; i < n; i++) {
        if (a[i] != b[i]) {
            return 0;
        }
    }
    return 1;
}

int sha256_self_test(char *out, size_t cap) {
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

    {
        uint8_t d[32];
        sha256((const uint8_t *)"abc", 3, d);
        A(hex_eq(d, 32, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
          "SHA-256(\"abc\") NIST FIPS 180-4");
    }
    {
        uint8_t d[32];
        sha256((const uint8_t *)"", 0, d);
        A(hex_eq(d, 32, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
          "SHA-256(\"\") NIST FIPS 180-4");
    }
    {
        uint8_t d[32];
        const char *msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        sha256((const uint8_t *)msg, 56, d);
        A(hex_eq(d, 32, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"),
          "SHA-256(448-bit) NIST FIPS 180-4");
    }
    {
        uint8_t d[28];
        sha224((const uint8_t *)"abc", 3, d);
        A(hex_eq(d, 28, "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7"),
          "SHA-224(\"abc\") NIST FIPS 180-4");
    }
    {
        uint8_t d[32];
        const char *key = "key";
        const char *msg = "The quick brown fox jumps over the lazy dog";
        hmac_sha256((const uint8_t *)key, 3, (const uint8_t *)msg, 43, d);
        A(hex_eq(d, 32, "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"),
          "HMAC-SHA256(key, \"The quick brown fox …\") RFC 4231");
    }
    {
        /* Long key (> 64 bytes) exercises the key-hash path. */
        uint8_t d[32];
        uint8_t longkey[80];
        for (int i = 0; i < 80; i++) {
            longkey[i] = (uint8_t)(0xaa);
        }
        uint8_t d2[32];
        hmac_sha256(longkey, 80, (const uint8_t *)"msg", 3, d);
        hmac_sha256(longkey, 80, (const uint8_t *)"msg", 3, d2);
        A(digest_eq(d, d2, 32), "HMAC long-key deterministic");
    }
    {
        uint8_t a[32], b[32];
        sha256((const uint8_t *)"fixed test vector", 17, a);
        sha256((const uint8_t *)"fixed test vector", 17, b);
        A(digest_eq(a, b, 32), "SHA-256 idempotent");
    }
    {
        uint8_t d[32];
        const char *m =
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()";
        sha256((const uint8_t *)m, 72, d);
        A(hex_eq(d, 32, "948731362ccbece05cb17ce5c46166d61048d21290e812427a4d77d7eed1bd61"),
          "SHA-256 multi-block (72 bytes)");
    }

    return all_ok ? 0 : -1;
}
