/* Bebop NTT — Number-Theoretic Transform + exact integer convolution.
 * Native primitive (port of dowiz ntt.rs). MOD = 998244353 = 2^23·119 + 1.
 * All products fit in u64 (operands < ~1e9 → product < ~1e18 < 1.8e19).
 */
#ifndef BEBOP_NTT_H
#define BEBOP_NTT_H

#include <stddef.h>
#include <stdint.h>

#define BEBOP_NTT_MOD 998244353ULL
#define BEBOP_NTT_ROOT 3ULL

uint64_t ntt_mod_pow(uint64_t base, uint64_t exp, uint64_t m);
uint64_t ntt_mod_inv(uint64_t a, uint64_t m);

/* In-place iterative NTT / inverse-NTT. n must be a power of two. */
void ntt_transform(uint64_t *a, size_t n, int invert);

/* Linear convolution (exact mod MOD). out must hold alen+blen-1 elements. */
void ntt_convolve(const uint64_t *a, size_t alen, const uint64_t *b, size_t blen, uint64_t *out);

/* Circular convolution of two equal-length (power-of-two) sequences. */
void ntt_circular(const uint64_t *a, const uint64_t *b, size_t n, uint64_t *out);

/* Map a value back to the signed range (−MOD/2, MOD/2]. */
int64_t ntt_centered(uint64_t v);

/* Self-test (round-trip, convolution parity, circular shift, centered corr). */
int ntt_self_test(char *out, size_t cap);

#endif /* BEBOP_NTT_H */
