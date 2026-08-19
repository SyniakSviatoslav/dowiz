/* Bebop tls — native cryptography module (no OpenSSL, no libc deps beyond
 * stdint/stddef). Self-contained primitives for a minimal TLS-style stack:
 *
 *   SHA-256 / SHA-512            FIPS 180-4
 *   AES-128/192/256              FIPS 197 (S-box + round-constant tables)
 *   ChaCha20                     RFC 8439 §2.4
 *   Poly1305                     RFC 8439 §2.5
 *   ChaCha20-Poly1305 AEAD       RFC 8439 §2.8
 *   X25519                       RFC 7748 (Curve25519, constant-time ladder)
 *   memcmp_ct                    constant-time comparison
 *
 * All symbols are prefixed `tls_` and all internal helpers are static, so this
 * file coexists with the standalone sha256.c / x25519.c modules without
 * symbol clashes.
 */
#ifndef BEBOP_TLS_H
#define BEBOP_TLS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- SHA-256 / SHA-512 (one-shot) ------------------------------------ */
void tls_sha256(const uint8_t *msg, size_t len, uint8_t out[32]);
void tls_sha512(const uint8_t *msg, size_t len, uint8_t out[64]);

/* ---- AES -------------------------------------------------------------- */
/* Maximum expanded-key size is AES-256: 4 * (14 + 1) = 60 words. */
#define TLS_AES_MAX_RK_WORDS 60

/* Expand a key into round keys. `key_bits` is 128, 192 or 256.
 * Returns the number of rounds (10/12/14) or -1 on a bad key size. */
int tls_aes_set_key(uint32_t rk[TLS_AES_MAX_RK_WORDS],
                    const uint8_t *key, unsigned key_bits);

void tls_aes_encrypt_block(const uint32_t rk[TLS_AES_MAX_RK_WORDS], int rounds,
                           const uint8_t in[16], uint8_t out[16]);
void tls_aes_decrypt_block(const uint32_t rk[TLS_AES_MAX_RK_WORDS], int rounds,
                           const uint8_t in[16], uint8_t out[16]);

/* ---- ChaCha20 ---------------------------------------------------------- */
/* Encrypt/decrypt (stream cipher, XOR with keystream). `nonce` is 12 bytes,
 * `counter` the 32-bit initial block counter. in/out may alias. */
void tls_chacha20(const uint8_t key[32], const uint8_t nonce[12],
                  uint32_t counter, const uint8_t *in, uint8_t *out, size_t len);

/* ---- Poly1305 ---------------------------------------------------------- */
/* One-shot MAC. `key` is a 32-byte one-time key (r || s). */
void tls_poly1305(const uint8_t key[32], const uint8_t *msg, size_t len,
                  uint8_t tag[16]);

/* ---- ChaCha20-Poly1305 AEAD (RFC 8439) --------------------------------- */
/* Encrypt `pt` -> `ct` in place or separately; write 16-byte `tag`. */
void tls_aead_chacha20_poly1305_encrypt(
    const uint8_t key[32], const uint8_t nonce[12],
    const uint8_t *aad, size_t aad_len,
    const uint8_t *pt, uint8_t *ct, size_t len, uint8_t tag[16]);

/* Decrypt `ct` -> `pt` and verify `tag`. Returns 0 on success, -1 if the tag
 * does not match (auth failure). */
int tls_aead_chacha20_poly1305_decrypt(
    const uint8_t key[32], const uint8_t nonce[12],
    const uint8_t *aad, size_t aad_len,
    const uint8_t *ct, uint8_t *pt, size_t len, const uint8_t tag[16]);

/* ---- Constant-time comparison ------------------------------------------ */
/* Returns 0 if a and b are equal, non-zero otherwise. Runs in time
 * independent of the data (and of where the first difference occurs). */
int tls_memcmp_ct(const uint8_t *a, const uint8_t *b, size_t n);

/* ---- X25519 (RFC 7748) -------------------------------------------------- */
/* Compute out = X25519(scalar, point). `scalar` is clamped internally;
 * `point` is the 32-byte little-endian u-coordinate (high bit masked).
 * Returns 0 on success, -1 if the result is the all-zero value (low-order
 * point, rejected per RFC 7748). */
int tls_x25519(uint8_t out[32], const uint8_t scalar[32],
               const uint8_t point[32]);

/* ---- Self-test ---------------------------------------------------------- */
/* Runs known-answer tests for every primitive. Returns 0 on PASS. */
int tls_self_test(char *out, size_t cap);

#ifdef __cplusplus
}
#endif

#endif /* BEBOP_TLS_H */
