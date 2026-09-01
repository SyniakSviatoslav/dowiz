/* Bebop rng — deterministic SplitMix64→PCG64 PRNG (port of dowiz rng.rs).
 * SplitMix64 mixes the state; a PCG64 RXS-M-XS output permutation (xorshift +
 * rotate) decorrelates the stream. Integer stream is bit-identical across
 * runs, platforms, and builds (a hard requirement for reproducible Monte-Carlo).
 */
#ifndef BEBOP_RNG_H
#define BEBOP_RNG_H

#include <stddef.h>
#include <stdint.h>

/* SplitMix64 mixing function (single step). Pure; advances and returns *state. */
uint64_t rng_splitmix64(uint64_t *state);

/* A deterministic 64-bit generator: SplitMix64 state mixed through a PCG64
 * output permutation. One struct, two composable transforms, zero dependencies. */
typedef struct {
    uint64_t sm_state;  /* SplitMix64 internal state. */
    uint64_t pcg_state; /* PCG64 LCG state. */
    uint64_t pcg_inc;   /* PCG64 stream selector (odd increment). */
} Rng;

/* New generator. `seed` mixes into both states; `stream` selects an independent
 * PCG64 subsequence (distinct streams never collide — useful for parallel MC). */
Rng rng_new(uint64_t seed, uint64_t stream);

/* Canonical reference seed (matches the official PCG-C demo). */
Rng rng_new_reference(void);

/* Next raw u64 from the PCG64 output permutation of the SplitMix64 stream. */
uint64_t rng_next_u64(Rng *r);

/* Uniform double in [0, 1) (53-bit mantissa). */
double rng_next_f64(Rng *r);

/* Uniform integer in [0, n) via rejection sampling (no modulo bias). */
size_t rng_next_index(Rng *r, size_t n);

/* Categorical sample from unnormalized weights w (non-negative, sum > 0).
 * Deterministic and fail-closed: empty or non-positive total returns 0. */
size_t rng_sample_categorical(Rng *r, const double *w, size_t n);

/* Self-test: appends "[ok] NAME"/"[FAIL] NAME" lines to out; 0 if all pass. */
int rng_self_test(char *out, size_t cap);

#endif /* BEBOP_RNG_H */
