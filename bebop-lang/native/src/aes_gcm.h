/* aes_gcm.h — AES-128 + GCM (Galois/Counter Mode) authenticated encryption.
 *
 * NIST SP 800-38D.  Pure-scalar C, no OpenSSL, no hardware AES-NI, no libc
 * beyond stdint/stddef.  Only the forward AES cipher is needed (GCM CTR and
 * the GHASH subkey H both use the encrypt direction), so no inverse S-box is
 * provided.
 *
 * AES:    FIPS 197 (S-box, ShiftRows, MixColumns, AddRoundKey).
 * GHASH:  GF(2^128) multiply with reduction polynomial
 *         x^128 + x^7 + x^2 + x + 1 (R = 0xE1 << 120).
 * GCM:    NIST SP 800-38D (J0, CTR inc32, tag = E_K(J0) ^ GHASH(A||C||len)).
 */
#ifndef BEBOP_AES_GCM_H
#define BEBOP_AES_GCM_H

#include <stddef.h>
#include <stdint.h>

#define AES_BLOCK_LEN 16
#define AES128_KEY_LEN 16
#define GCM_TAG_LEN 16
#define GCM_IV_LEN 12 /* 96-bit nonce, the SP 800-38D recommended length */

/* Opaque per-message GCM context (key schedule + H + J0 precomputed). */
typedef struct aes_gcm_ctx {
    uint32_t rk[44];   /* AES-128 round keys (11 × 128-bit words) */
    uint8_t  H[16];    /* GHASH subkey = AES_K(0^128)             */
    uint8_t  J0[16];   /* initial counter block                   */
} aes_gcm_ctx;

/* Initialise a GCM context from a 16-byte key and an arbitrary-length IV.
 * Returns 0 on success, -1 on bad parameters (NULL pointers). */
int aes_gcm_init(aes_gcm_ctx *ctx,
                 const uint8_t key[AES128_KEY_LEN],
                 const uint8_t *iv, size_t iv_len);

/* AEAD encrypt: in-place or distinct plaintext/ciphertext buffers are both
 * supported (may alias).  Produces a 16-byte authentication tag. */
void aes_gcm_encrypt(aes_gcm_ctx *ctx,
                     const uint8_t *aad, size_t aad_len,
                     const uint8_t *pt, size_t pt_len,
                     uint8_t *ct,
                     uint8_t tag[GCM_TAG_LEN]);

/* AEAD decrypt: writes plaintext and returns 0 only if the supplied tag
 * matches the recomputed tag (constant-time comparison).  On mismatch the
 * output buffer is left unmodified and -1 is returned. */
int aes_gcm_decrypt(aes_gcm_ctx *ctx,
                    const uint8_t *aad, size_t aad_len,
                    const uint8_t *ct, size_t ct_len,
                    uint8_t *pt,
                    const uint8_t tag[GCM_TAG_LEN]);

/* NIST SP 800-38D known-answer self-test (empty-plaintext vector + a
 * non-empty round-trip and a tamper-detection probe).
 * Returns 0 on success, non-zero on failure. */
int aes_gcm_self_test(char *out, size_t cap);

#endif /* BEBOP_AES_GCM_H */
