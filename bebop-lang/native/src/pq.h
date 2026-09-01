/* Bebop PQ — post-quantum crypto primitives (port of dowiz pq/keccak.rs, FIPS 202).
 * Zero-dependency Keccak-f[1600] + SHAKE128/256 + SHA3-256/512. */
#ifndef BEBOP_PQ_H
#define BEBOP_PQ_H

#include <stddef.h>
#include <stdint.h>

/* Keccak-f[1600] permutation (24 rounds), in place on 25 u64 lanes. */
void keccak_f(uint64_t state[25]);

/* SHAKE128 (rate 168, pad 0x1f): squeeze outlen bytes. */
void shake128(const uint8_t *input, size_t inlen, uint8_t *out, size_t outlen);

/* SHAKE256 (rate 136, pad 0x1f): squeeze outlen bytes. */
void shake256(const uint8_t *input, size_t inlen, uint8_t *out, size_t outlen);

/* SHA3-256 (rate 136, pad 0x06): 32-byte digest. */
void sha3_256(const uint8_t *input, size_t inlen, uint8_t out[32]);

/* SHA3-512 (rate 72, pad 0x06): 64-byte digest. */
void sha3_512(const uint8_t *input, size_t inlen, uint8_t out[64]);

/* FIPS 202 known-answer self-test. Returns 0 on success, non-zero on failure. */
int pq_self_test(char *out, size_t cap);

#endif /* BEBOP_PQ_H */
