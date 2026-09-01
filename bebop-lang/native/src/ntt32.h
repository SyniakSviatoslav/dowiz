/* Bebop NTT32 — quantized integer NTT (uint32_t element type).
 *
 * MOD = 998244353 fits in uint32_t (MOD < 2^31). Products fit in uint64_t.
 * Barrett reduction uses 128-bit intermediate.
 *
 * Pure C11, zero deps.
 */
#ifndef BEBOP_NTT32_H
#define BEBOP_NTT32_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#define BEBOP_NTT32_MOD  998244353U
#define BEBOP_NTT32_ROOT 3U

void ntt32_transform(uint32_t *a, size_t n, bool invert);
void ntt32_convolve(const uint32_t *a, size_t alen,
                    const uint32_t *b, size_t blen,
                    uint32_t *out);
void ntt32_circular(const uint32_t *a, const uint32_t *b,
                    size_t n, uint32_t *out);
int64_t ntt32_centered(uint32_t v);
int ntt32_self_test(char *out, size_t cap);

#endif
