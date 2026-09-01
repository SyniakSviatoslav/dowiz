/* aes_gcm.c — AES-128 + GCM (Galois/Counter Mode) authenticated encryption.
 *
 * Pure-scalar C, no external dependencies.  KAT-gated against NIST SP 800-38D
 * (the all-zero-key / all-zero-IV empty-plaintext vector from the GCM spec).
 *
 * Structure:
 *   - AES-128 block cipher (FIPS 197): key expansion + encrypt block.
 *   - GHASH: GF(2^128) multiply + polynomial reduction.
 *   - GCM: J0 derivation, CTR (inc32) keystream, tag = E_K(J0) ^ GHASH(...).
 *
 * innovate: GHASH multiplication is the textbook bit-by-bit right-shift form
 * (branchy on secret data).  Correct and KAT-gated, but not constant-time.
 * Upgrade trigger: swap `gf128_mul` for a bit-sliced / CLMUL / table-based
 * constant-time reduction when this primitive is used on live secrets.
 */
#include "aes_gcm.h"

#include <stdio.h>
#include <string.h>

/* ------------------------------------------------------------------ */
/* AES S-box (FIPS 197 §5.1.1)                                        */
/* ------------------------------------------------------------------ */
static const uint8_t SBOX[256] = {
    0x63,0x7c,0x77,0x7b,0xf2,0x6b,0x6f,0xc5,0x30,0x01,0x67,0x2b,0xfe,0xd7,0xab,0x76,
    0xca,0x82,0xc9,0x7d,0xfa,0x59,0x47,0xf0,0xad,0xd4,0xa2,0xaf,0x9c,0xa4,0x72,0xc0,
    0xb7,0xfd,0x93,0x26,0x36,0x3f,0xf7,0xcc,0x34,0xa5,0xe5,0xf1,0x71,0xd8,0x31,0x15,
    0x04,0xc7,0x23,0xc3,0x18,0x96,0x05,0x9a,0x07,0x12,0x80,0xe2,0xeb,0x27,0xb2,0x75,
    0x09,0x83,0x2c,0x1a,0x1b,0x6e,0x5a,0xa0,0x52,0x3b,0xd6,0xb3,0x29,0xe3,0x2f,0x84,
    0x53,0xd1,0x00,0xed,0x20,0xfc,0xb1,0x5b,0x6a,0xcb,0xbe,0x39,0x4a,0x4c,0x58,0xcf,
    0xd0,0xef,0xaa,0xfb,0x43,0x4d,0x33,0x85,0x45,0xf9,0x02,0x7f,0x50,0x3c,0x9f,0xa8,
    0x51,0xa3,0x40,0x8f,0x92,0x9d,0x38,0xf5,0xbc,0xb6,0xda,0x21,0x10,0xff,0xf3,0xd2,
    0xcd,0x0c,0x13,0xec,0x5f,0x97,0x44,0x17,0xc4,0xa7,0x7e,0x3d,0x64,0x5d,0x19,0x73,
    0x60,0x81,0x4f,0xdc,0x22,0x2a,0x90,0x88,0x46,0xee,0xb8,0x14,0xde,0x5e,0x0b,0xdb,
    0xe0,0x32,0x3a,0x0a,0x49,0x06,0x24,0x5c,0xc2,0xd3,0xac,0x62,0x91,0x95,0xe4,0x79,
    0xe7,0xc8,0x37,0x6d,0x8d,0xd5,0x4e,0xa9,0x6c,0x56,0xf4,0xea,0x65,0x7a,0xae,0x08,
    0xba,0x78,0x25,0x2e,0x1c,0xa6,0xb4,0xc6,0xe8,0xdd,0x74,0x1f,0x4b,0xbd,0x8b,0x8a,
    0x70,0x3e,0xb5,0x66,0x48,0x03,0xf6,0x0e,0x61,0x35,0x57,0xb9,0x86,0xc1,0x1d,0x9e,
    0xe1,0xf8,0x98,0x11,0x69,0xd9,0x8e,0x94,0x9b,0x1e,0x87,0xe9,0xce,0x55,0x28,0xdf,
    0x8c,0xa1,0x89,0x0d,0xbf,0xe6,0x42,0x68,0x41,0x99,0x2d,0x0f,0xb0,0x54,0xbb,0x16,
};

/* ------------------------------------------------------------------ */
/* Key schedule (FIPS 197 §5.2)                                       */
/* ------------------------------------------------------------------ */

static uint32_t load_be32(const uint8_t p[4]) {
    return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
           ((uint32_t)p[2] << 8)  |  (uint32_t)p[3];
}

static void store_be32(uint8_t p[4], uint32_t v) {
    p[0] = (uint8_t)(v >> 24);
    p[1] = (uint8_t)(v >> 16);
    p[2] = (uint8_t)(v >> 8);
    p[3] = (uint8_t)v;
}

/* SubWord: apply S-box to each byte of a word. */
static uint32_t sub_word(uint32_t w) {
    return ((uint32_t)SBOX[(w >> 24) & 0xff] << 24) |
           ((uint32_t)SBOX[(w >> 16) & 0xff] << 16) |
           ((uint32_t)SBOX[(w >> 8)  & 0xff] << 8)  |
           ((uint32_t)SBOX[w         & 0xff]);
}

/* RotWord: left-rotate a word by one byte. */
static uint32_t rot_word(uint32_t w) {
    return (w << 8) | (w >> 24);
}

/* Round constants Rcon[i] = x^(i-1) in GF(2^8), i = 1..10. */
static const uint8_t RCON[11] = {
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
};

static void aes128_key_expand(uint32_t rk[44], const uint8_t key[AES128_KEY_LEN]) {
    int i;
    for (i = 0; i < 4; i++)
        rk[i] = load_be32(key + 4 * i);

    for (i = 4; i < 44; i++) {
        uint32_t temp = rk[i - 1];
        if ((i % 4) == 0) {
            temp = sub_word(rot_word(temp)) ^ ((uint32_t)RCON[i / 4] << 24);
        }
        rk[i] = rk[i - 4] ^ temp;
    }
}

/* ------------------------------------------------------------------ */
/* AES-128 encrypt one 16-byte block (FIPS 197 §5.1).                 */
/* ------------------------------------------------------------------ */

static void add_round_key(uint8_t s[16], const uint32_t rk[4]) {
    int i;
    for (i = 0; i < 4; i++) {
        uint32_t w = load_be32(s + 4 * i) ^ rk[i];
        store_be32(s + 4 * i, w);
    }
}

static void sub_bytes(uint8_t s[16]) {
    int i;
    for (i = 0; i < 16; i++)
        s[i] = SBOX[s[i]];
}

static void shift_rows(uint8_t s[16]) {
    uint8_t t[16];
    /* Row 0 stays; rows 1,2,3 rotate left by 1,2,3 bytes. */
    t[0]  = s[0];  t[4]  = s[4];  t[8]  = s[8];  t[12] = s[12];
    t[1]  = s[5];  t[5]  = s[9];  t[9]  = s[13]; t[13] = s[1];
    t[2]  = s[10]; t[6]  = s[14]; t[10] = s[2];  t[14] = s[6];
    t[3]  = s[15]; t[7]  = s[3];  t[11] = s[7];  t[15] = s[11];
    memcpy(s, t, 16);
}

static uint8_t xtime(uint8_t a) {
    return (uint8_t)((a << 1) ^ (((a >> 7) & 1) * 0x1b));
}

static void mix_columns(uint8_t s[16]) {
    int c;
    for (c = 0; c < 4; c++) {
        uint8_t *col = s + 4 * c;
        uint8_t a0 = col[0], a1 = col[1], a2 = col[2], a3 = col[3];
        uint8_t t = a0 ^ a1 ^ a2 ^ a3;
        col[0] = a0 ^ xtime(a0 ^ a1) ^ t;
        col[1] = a1 ^ xtime(a1 ^ a2) ^ t;
        col[2] = a2 ^ xtime(a2 ^ a3) ^ t;
        col[3] = a3 ^ xtime(a3 ^ a0) ^ t;
    }
}

static void aes128_encrypt_block(const uint32_t rk[44],
                                 const uint8_t in[16], uint8_t out[16]) {
    uint8_t s[16];
    int round;
    memcpy(s, in, 16);

    add_round_key(s, rk + 0);
    for (round = 1; round <= 9; round++) {
        sub_bytes(s);
        shift_rows(s);
        mix_columns(s);
        add_round_key(s, rk + 4 * round);
    }
    sub_bytes(s);
    shift_rows(s);
    add_round_key(s, rk + 40);

    memcpy(out, s, 16);
}

/* ------------------------------------------------------------------ */
/* GHASH — GF(2^128) arithmetic                                        */
/* ------------------------------------------------------------------ */

/* Right-shift a 128-bit big-endian value by one bit (in place). */
static void gf128_shr(uint8_t v[16]) {
    int i;
    for (i = 15; i > 0; i--)
        v[i] = (uint8_t)((v[i] >> 1) | (v[i - 1] << 7));
    v[0] >>= 1;
}

/* r = a * b in GF(2^128) with reduction polynomial x^128+x^7+x^2+x+1.
 * Textbook bit-serial multiply (see innovate: note re constant-time). */
static void gf128_mul(uint8_t r[16], const uint8_t a[16], const uint8_t b[16]) {
    uint8_t z[16] = {0};
    uint8_t v[16];
    int i, bit;
    memcpy(v, b, 16);

    for (i = 0; i < 16; i++) {
        uint8_t x = a[i];
        for (bit = 0; bit < 8; bit++) {
            if (x & (uint8_t)(0x80 >> bit)) {
                int k;
                for (k = 0; k < 16; k++) z[k] ^= v[k];
            }
            if (v[15] & 1) {
                gf128_shr(v);
                v[0] ^= 0xe1; /* R = 0xE100...00, high byte 0xE1 */
            } else {
                gf128_shr(v);
            }
        }
    }
    memcpy(r, z, 16);
}

/* GHASH: Y = (Y ^ X_i) · H over each 16-byte block X_i. */
static void ghash_update(uint8_t y[16], const uint8_t h[16],
                         const uint8_t *data, size_t len) {
    size_t off;
    for (off = 0; off + 16 <= len; off += 16) {
        int k;
        for (k = 0; k < 16; k++) y[k] ^= data[off + k];
        gf128_mul(y, y, h);
    }
}

/* ------------------------------------------------------------------ */
/* GCM helper: 32-bit big-endian increment (SP 800-38D §6.2 inc32)     */
/* ------------------------------------------------------------------ */

static void inc32(uint8_t blk[16]) {
    uint32_t c = load_be32(blk + 12) + 1;
    store_be32(blk + 12, c);
}

/* ------------------------------------------------------------------ */
/* GCM context setup                                                   */
/* ------------------------------------------------------------------ */

int aes_gcm_init(aes_gcm_ctx *ctx,
                 const uint8_t key[AES128_KEY_LEN],
                 const uint8_t *iv, size_t iv_len) {
    uint8_t zero[16] = {0};

    if (!ctx || !key || (!iv && iv_len > 0))
        return -1;

    aes128_key_expand(ctx->rk, key);

    /* H = AES_K(0^128). */
    aes128_encrypt_block(ctx->rk, zero, ctx->H);

    /* J0 (SP 800-38D §7.1). */
    if (iv_len == 12) {
        memcpy(ctx->J0, iv, 12);
        ctx->J0[12] = 0; ctx->J0[13] = 0; ctx->J0[14] = 0; ctx->J0[15] = 1;
    } else {
        /* J0 = GHASH( IV || 0^(s) || [len(IV)]_64 ), s = pad to block. */
        uint8_t y[16] = {0};
        size_t off = 0;
        memset(ctx->J0, 0, 16);
        ghash_update(y, ctx->H, iv, iv_len);
        /* Pad the remainder of the final partial block with zeros. */
        off = iv_len % 16;
        if (off != 0) {
            uint8_t tail[16] = {0};
            memcpy(tail, iv + (iv_len - off), off);
            ghash_update(y, ctx->H, tail, 16);
        } else if (iv_len == 0) {
            /* no-op: empty IV has no blocks, but len block is still GHASHed */
        }
        {
            uint8_t lenblk[16] = {0};
            store_be32(lenblk + 12, (uint32_t)(iv_len * 8));
            int k;
            for (k = 0; k < 16; k++) y[k] ^= lenblk[k];
            gf128_mul(ctx->J0, y, ctx->H);
        }
    }

    return 0;
}

/* ------------------------------------------------------------------ */
/* GCM core: produce the GHASH tag S and the keystream.                */
/* ------------------------------------------------------------------ */

static void gcm_tag(aes_gcm_ctx *ctx,
                    const uint8_t *aad, size_t aad_len,
                    const uint8_t *c, size_t c_len,
                    uint8_t tag[GCM_TAG_LEN]) {
    uint8_t y[16] = {0};
    uint8_t pad[16] = {0};

    /* GHASH(A), then GHASH(C), each zero-padded to a block boundary. */
    ghash_update(y, ctx->H, aad, aad_len);
    if (aad_len % 16) {
        size_t off = aad_len % 16;
        ghash_update(y, ctx->H, aad + (aad_len - off), off);
        ghash_update(y, ctx->H, pad, 16 - off);
    }

    ghash_update(y, ctx->H, c, c_len);
    if (c_len % 16) {
        size_t off = c_len % 16;
        ghash_update(y, ctx->H, c + (c_len - off), off);
        ghash_update(y, ctx->H, pad, 16 - off);
    }

    /* len(A) || len(C) as 64-bit big-endian bit lengths. */
    {
        uint8_t lenblk[16] = {0};
        uint64_t a_bits = (uint64_t)aad_len * 8;
        uint64_t c_bits = (uint64_t)c_len * 8;
        int k;
        store_be32(lenblk + 0,  (uint32_t)(a_bits >> 32));
        store_be32(lenblk + 4,  (uint32_t)(a_bits & 0xffffffffu));
        store_be32(lenblk + 8,  (uint32_t)(c_bits >> 32));
        store_be32(lenblk + 12, (uint32_t)(c_bits & 0xffffffffu));
        for (k = 0; k < 16; k++) y[k] ^= lenblk[k];
        gf128_mul(y, y, ctx->H);
    }

    /* T = E_K(J0) ^ S. */
    aes128_encrypt_block(ctx->rk, ctx->J0, tag);
    {
        int k;
        for (k = 0; k < 16; k++) tag[k] ^= y[k];
    }
}

static void gcm_ctr_xor(aes_gcm_ctx *ctx, const uint8_t *in, uint8_t *out,
                        size_t len) {
    uint8_t ctr[16];
    uint8_t ks[16];
    size_t off = 0;
    memcpy(ctr, ctx->J0, 16);
    inc32(ctr); /* first block uses J0+1 */

    while (off < len) {
        aes128_encrypt_block(ctx->rk, ctr, ks);
        size_t n = len - off;
        if (n > 16) n = 16;
        {
            size_t k;
            for (k = 0; k < n; k++)
                out[off + k] = in[off + k] ^ ks[k];
        }
        inc32(ctr);
        off += n;
    }
}

/* ------------------------------------------------------------------ */
/* Public API                                                          */
/* ------------------------------------------------------------------ */

void aes_gcm_encrypt(aes_gcm_ctx *ctx,
                     const uint8_t *aad, size_t aad_len,
                     const uint8_t *pt, size_t pt_len,
                     uint8_t *ct,
                     uint8_t tag[GCM_TAG_LEN]) {
    gcm_ctr_xor(ctx, pt, ct, pt_len);
    gcm_tag(ctx, aad, aad_len, ct, pt_len, tag);
}

int aes_gcm_decrypt(aes_gcm_ctx *ctx,
                    const uint8_t *aad, size_t aad_len,
                    const uint8_t *ct, size_t ct_len,
                    uint8_t *pt,
                    const uint8_t tag[GCM_TAG_LEN]) {
    uint8_t expect[GCM_TAG_LEN];
    uint8_t diff = 0;
    int k;

    gcm_tag(ctx, aad, aad_len, ct, ct_len, expect);
    for (k = 0; k < 16; k++)
        diff |= expect[k] ^ tag[k];

    if (diff != 0)
        return -1; /* tag mismatch: do not release plaintext */

    gcm_ctr_xor(ctx, ct, pt, ct_len);
    return 0;
}

/* ------------------------------------------------------------------ */
/* Self-test (NIST SP 800-38D known-answer)                            */
/* ------------------------------------------------------------------ */

static int hex_byte(char hi, char lo) {
    int v = 0;
    if (hi >= '0' && hi <= '9') v = (hi - '0') << 4;
    else if (hi >= 'a' && hi <= 'f') v = (hi - 'a' + 10) << 4;
    else if (hi >= 'A' && hi <= 'F') v = (hi - 'A' + 10) << 4;
    else return -1;

    if (lo >= '0' && lo <= '9') v |= (lo - '0');
    else if (lo >= 'a' && lo <= 'f') v |= (lo - 'a' + 10);
    else if (lo >= 'A' && lo <= 'F') v |= (lo - 'A' + 10);
    else return -1;

    return v;
}

static int hex_decode(uint8_t *out, size_t out_cap, const char *hex,
                      size_t *out_len) {
    size_t n = strlen(hex);
    size_t i;
    if (n % 2) return -1;
    if (n / 2 > out_cap) return -1;
    for (i = 0; i < n / 2; i++) {
        int b = hex_byte(hex[2 * i], hex[2 * i + 1]);
        if (b < 0) return -1;
        out[i] = (uint8_t)b;
    }
    *out_len = n / 2;
    return 0;
}

static void to_hex(char *buf, size_t cap, const uint8_t *data, size_t len) {
    static const char digits[] = "0123456789abcdef";
    size_t i;
    for (i = 0; i < len && 2 * i + 2 < cap; i++) {
        buf[2 * i]     = digits[data[i] >> 4];
        buf[2 * i + 1] = digits[data[i] & 0xf];
    }
    buf[2 * i] = '\0';
}

static int ct_eq(const uint8_t *a, const uint8_t *b, size_t n) {
    uint8_t d = 0;
    size_t i;
    for (i = 0; i < n; i++) d |= a[i] ^ b[i];
    return d == 0;
}

int aes_gcm_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int failures = 0;

#define PASS(...) do {                                              \
        pos += (size_t)snprintf(out + pos, cap - pos, __VA_ARGS__); \
    } while (0)
#define FAIL(...) do {                                              \
        pos += (size_t)snprintf(out + pos, cap - pos, __VA_ARGS__); \
        failures++;                                                 \
    } while (0)

    /* --- NIST SP 800-38D / GCM spec test case 1 ---
       key = 16 × 00, IV = 12 × 00, empty plaintext, empty AAD,
       tag = 58e2fccefa7e3061367f1d57a4e7455a. */
    {
        uint8_t key[16] = {0};
        uint8_t iv[12]  = {0};
        uint8_t expected[16];
        uint8_t tag[16];
        size_t elen;
        aes_gcm_ctx ctx;

        if (hex_decode(expected, sizeof expected,
                       "58e2fccefa7e3061367f1d57a4e7455a", &elen) != 0 ||
            elen != 16) {
            FAIL("GCM KAT: expected-tag hex decode failed\n");
        } else if (aes_gcm_init(&ctx, key, iv, sizeof iv) != 0) {
            FAIL("GCM KAT: init failed\n");
        } else {
            aes_gcm_encrypt(&ctx, NULL, 0, NULL, 0, NULL, tag);
            if (ct_eq(tag, expected, 16)) {
                PASS("GCM KAT (empty pt, zero key/iv): PASS\n");
            } else {
                char g[33], e[33];
                to_hex(g, sizeof g, tag, 16);
                to_hex(e, sizeof e, expected, 16);
                FAIL("GCM KAT (empty pt, zero key/iv): FAIL\n"
                     "  got:      %s\n  expected: %s\n", g, e);
            }
        }
    }

    /* --- Round-trip: encrypt then decrypt a non-empty payload --- */
    {
        static const uint8_t key[16] = {
            0xfe,0xff,0xe9,0x92,0x86,0x65,0x73,0x1c,
            0x6d,0x6a,0x8f,0x94,0x67,0x30,0x83,0x08,
        };
        static const uint8_t iv[12] = {
            0xca,0xfe,0xba,0xbe,0xfa,0xce,0xdb,0xad,
            0xde,0xca,0xf8,0x88,
        };
        static const char *msg =
            "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72"
            "1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255";
        static const char *aad_hex =
            "feedfacedeadbeeffeedfacedeadbeefabaddad2";
        uint8_t pt[64], ct[64], rt[64], aad[20];
        size_t ptlen, aadlen;
        uint8_t tag[16];
        aes_gcm_ctx ctx;
        int ok;

        if (hex_decode(pt, sizeof pt, msg, &ptlen) != 0 ||
            hex_decode(aad, sizeof aad, aad_hex, &aadlen) != 0) {
            FAIL("round-trip: input hex decode failed\n");
        } else if (aes_gcm_init(&ctx, key, iv, sizeof iv) != 0) {
            FAIL("round-trip: init failed\n");
        } else {
            memset(ct, 0, sizeof ct);
            aes_gcm_encrypt(&ctx, aad, aadlen, pt, ptlen, ct, tag);

            memset(rt, 0, sizeof rt);
            ok = aes_gcm_decrypt(&ctx, aad, aadlen, ct, ptlen, rt, tag);

            if (ok == 0 && memcmp(rt, pt, ptlen) == 0) {
                PASS("round-trip encrypt/decrypt (%zu B + AAD): PASS\n", ptlen);
            } else {
                FAIL("round-trip encrypt/decrypt: FAIL (ok=%d)\n", ok);
            }

            /* Tamper probe: flip one ciphertext byte → tag must reject. */
            ct[0] ^= 0x01;
            memset(rt, 0, sizeof rt);
            if (aes_gcm_decrypt(&ctx, aad, aadlen, ct, ptlen, rt, tag) != 0) {
                PASS("tamper detection (bad tag rejected): PASS\n");
            } else {
                FAIL("tamper detection: FAIL (forged tag accepted)\n");
            }
        }
    }

    /* --- 96-bit IV path already covered; exercise the GHASH-J0 path --- */
    {
        static const uint8_t key[16] = {
            0x00,0x01,0x02,0x03,0x04,0x05,0x06,0x07,
            0x08,0x09,0x0a,0x0b,0x0c,0x0d,0x0e,0x0f,
        };
        static const uint8_t iv[8] = {1,2,3,4,5,6,7,8}; /* non-96-bit */
        static const uint8_t pt[8]  = {0xde,0xad,0xbe,0xef,0x0b,0xad,0xf0,0x0d};
        uint8_t ct[8], rt[8], tag[16];
        aes_gcm_ctx ctx;
        if (aes_gcm_init(&ctx, key, iv, sizeof iv) != 0) {
            FAIL("non-96-bit IV: init failed\n");
        } else {
            aes_gcm_encrypt(&ctx, NULL, 0, pt, sizeof pt, ct, tag);
            if (aes_gcm_decrypt(&ctx, NULL, 0, ct, sizeof ct, rt, tag) == 0 &&
                memcmp(rt, pt, sizeof pt) == 0) {
                PASS("non-96-bit IV (GHASH-J0) round-trip: PASS\n");
            } else {
                FAIL("non-96-bit IV (GHASH-J0) round-trip: FAIL\n");
            }
        }
    }

#undef PASS
#undef FAIL

    return failures == 0 ? 0 : 1;
}
