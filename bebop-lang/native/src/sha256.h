/* Bebop sha256 — SHA-256 / SHA-224 / HMAC-SHA256 (port of dowiz sha256_hw.rs,
 * FIPS 180-4 + FIPS 198-1). Pure-scalar C, no SHA-NI, no external deps. */
#ifndef BEBOP_SHA256_H
#define BEBOP_SHA256_H

#include <stddef.h>
#include <stdint.h>

/* SHA-256 (FIPS 180-4): 32-byte digest. */
void sha256(const uint8_t *data, size_t len, uint8_t out[32]);

/* SHA-224 (FIPS 180-4): 28-byte digest (truncated SHA-256 variant). */
void sha224(const uint8_t *data, size_t len, uint8_t out[28]);

/* HMAC-SHA256 (RFC 2104 / FIPS 198-1): keyed hash, 32-byte digest. */
void hmac_sha256(const uint8_t *key, size_t key_len,
                 const uint8_t *data, size_t data_len,
                 uint8_t out[32]);

/* NIST FIPS 180-4 / RFC 4231 known-answer self-test.
 * Returns 0 on success, non-zero on failure. */
int sha256_self_test(char *out, size_t cap);

#endif /* BEBOP_SHA256_H */
