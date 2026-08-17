/* Bebop Hypervector — fixed-width 1024-bit VSA primitive (port of dowiz
 * hypervector.rs). bind (XOR), bundle (majority), similarity (Hamming),
 * permute (bit rotation), shift-invariant similarity (NTT cross-correlation).
 */
#ifndef BEBOP_HYPER_H
#define BEBOP_HYPER_H

#include <stddef.h>
#include <stdint.h>

#define BEBOP_HV_D 1024
#define BEBOP_HV_WORDS 16

/* dowiz best practice: #[repr(align(64))] — 1024 bits = 128 bytes = exactly 2
 * cache lines, aligned so popcount/similarity never straddles an L1 line. */
typedef struct {
    uint64_t words[BEBOP_HV_WORDS];
} __attribute__((aligned(64))) Hypervector;

Hypervector hv_zero(void);
Hypervector hv_code(uint64_t seed);
Hypervector hv_bind(const Hypervector *a, const Hypervector *b);
Hypervector hv_bundle(const Hypervector *items, size_t n);
uint32_t hv_hamming(const Hypervector *a, const Hypervector *b);
double hv_similarity(const Hypervector *a, const Hypervector *b);

/* NEON-accelerated bind (XOR) + hamming (popcount) — 128-bit per instruction. */
Hypervector hv_bind_neon(const Hypervector *a, const Hypervector *b);
uint32_t hv_hamming_neon(const Hypervector *a, const Hypervector *b);
Hypervector hv_bind_neon2(const Hypervector *a, const Hypervector *b);

/* Benchmark: binds/sec (scalar vs NEON). Returns chars written (snprintf-style). */
int hv_benchmark(char *out, size_t cap);
Hypervector hv_permute(const Hypervector *v, uint32_t shift);
uint32_t hv_popcount(const Hypervector *v);
int hv_to_hex(const Hypervector *v, char *out, size_t cap);
int hv_from_hex(const char *s, Hypervector *out);
double hv_shift_invariant_similarity(const Hypervector *a, const Hypervector *b);

/* NEON SIMD variants (AArch64 eor + cnt) — faster bind/hamming. */
Hypervector hv_bind_neon(const Hypervector *a, const Hypervector *b);
uint32_t hv_hamming_neon(const Hypervector *a, const Hypervector *b);
Hypervector hv_bind_neon2(const Hypervector *a, const Hypervector *b);

/* Benchmark: scalar vs NEON bind/hamming, reports Mops/s. */
int hv_benchmark(char *out, size_t cap);

/* djb2 string hash (seed for hv_code). */
uint64_t hv_hash(const char *s);

/* VSA text encoding: bundle of trigram codes (fuzzy/semantic identity). */
Hypervector hv_encode_text(const char *text);

int hyper_self_test(char *out, size_t cap);

#endif /* BEBOP_HYPER_H */
