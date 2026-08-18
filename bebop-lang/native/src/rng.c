/* Bebop rng — implementation (port of dowiz rng.rs). */
#include "rng.h"

#include <stdio.h>

/* PCG64 LCG multiplier (6364136223846793005). */
#define RNG_PCG_MUL 6364136223846793005ULL
/* SplitMix64 golden-ratio increment. */
#define RNG_GOLDEN 0x9E3779B97F4A7C15ULL

uint64_t rng_splitmix64(uint64_t *state) {
    uint64_t z = (*state += RNG_GOLDEN);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

/* One PCG64 step: LCG state = state * MUL + inc. */
static uint64_t pcg_step(uint64_t state, uint64_t inc) {
    return state * RNG_PCG_MUL + inc;
}

/* Right-rotate by rot in [0, 63]. Branchless — compiles to a single ROR/EXTR. */
static uint64_t rotate_right64(uint64_t x, unsigned rot) {
    return (x >> rot) | (x << ((64 - rot) & 63));
}

Rng rng_new(uint64_t seed, uint64_t stream) {
    Rng r;
    uint64_t tmp = seed ^ RNG_GOLDEN;
    /* PCG64 increment must be odd; derive from stream deterministically. */
    r.pcg_inc = (stream << 1) | 1;
    r.sm_state = seed;
    /* PCG64 requires the state be advanced once before first output. */
    r.pcg_state = rng_splitmix64(&tmp);
    r.pcg_state += r.pcg_inc;
    r.pcg_state = pcg_step(r.pcg_state, r.pcg_inc);
    return r;
}

Rng rng_new_reference(void) {
    return rng_new(0x4D595DF4D0F33173ULL, 0xDA3E39CB94B95BDBULL);
}

uint64_t rng_next_u64(Rng *r) {
    /* Pull the next SplitMix64 value as the LCG seed for this step. */
    uint64_t pre = rng_splitmix64(&r->sm_state);
    r->pcg_state = r->pcg_state * RNG_PCG_MUL + r->pcg_inc;
    {
        /* PCG64 output function: xorshift + rotate (RXS-M-XS permutation). */
        uint64_t x = r->pcg_state;
        unsigned rot = (unsigned)((x >> 59) & 31);
        uint64_t xorshifted = (x ^ (x >> 18)) >> 27;
        uint64_t out = rotate_right64(xorshifted, rot);
        /* Mix SplitMix64 entropy in so the stream is not a bare LCG. */
        return out ^ pre;
    }
}

double rng_next_f64(Rng *r) {
    /* Take top 53 bits. */
    return (double)(rng_next_u64(r) >> 11) / (double)(1ULL << 53);
}

size_t rng_next_index(Rng *r, size_t n) {
    uint64_t range;
    if (n == 0) {
        return 0;
    }
    /* Rejection sampling over a multiple of n <= 2^64. */
    range = (UINT64_MAX / (uint64_t)n) * (uint64_t)n;
    for (;;) {
        uint64_t v = rng_next_u64(r);
        if (v < range) {
            return (size_t)(v % (uint64_t)n);
        }
    }
}

size_t rng_sample_categorical(Rng *r, const double *w, size_t n) {
    size_t i;
    double total = 0.0;
    double acc = 0.0;
    double rnd;
    if (n == 0) {
        return 0; /* fail-closed: empty weight vector */
    }
    for (i = 0; i < n; i++) {
        total += w[i];
    }
    if (!(total > 0.0)) {
        return 0; /* fail-closed: all weights non-positive */
    }
    rnd = rng_next_f64(r) * total;
    for (i = 0; i < n; i++) {
        acc += w[i];
        if (rnd < acc) {
            return i;
        }
    }
    return n - 1; /* numerical tail fallback */
}

int rng_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name) do { \
    int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n", (cond) ? "ok" : "FAIL", name); \
    if (r_ > 0) pos += (size_t)r_; \
    if (!(cond)) all_ok = 0; \
} while (0)

    /* Canonical SplitMix64 reference stream (seed = 0): published test vectors. */
    {
        uint64_t s = 0;
        const uint64_t expected[5] = {
            0xE220A8397B1DCDAFULL,
            0x6E789E6AA1B965F4ULL,
            0x06C45D188009454FULL,
            0xF88BB8A8724C81ECULL,
            0x1B39896A51A8749BULL,
        };
        int i, ok = 1;
        for (i = 0; i < 5; i++) {
            if (rng_splitmix64(&s) != expected[i]) {
                ok = 0;
            }
        }
        A(ok, "splitmix64 reference stream");
    }

    /* Determinism: same seed + stream => bit-identical 8-draw sequence. */
    {
        Rng a = rng_new(0x123456789ABCDEF0ULL, 1);
        Rng b = rng_new(0x123456789ABCDEF0ULL, 1);
        int i, ok = 1;
        for (i = 0; i < 8; i++) {
            if (rng_next_u64(&a) != rng_next_u64(&b)) {
                ok = 0;
            }
        }
        A(ok, "same seed+stream reproducible");
    }

    /* Different seed => different stream (no constant output). */
    {
        Rng a = rng_new(0x123456789ABCDEF0ULL, 1);
        Rng c = rng_new(0x123456789ABCDEF0ULL ^ 0xFFFF, 1);
        A(rng_next_u64(&a) != rng_next_u64(&c), "different seed differs");
    }

    /* next_index stays in [0, n); next_index(0) == 0 (fail-closed). */
    {
        Rng r = rng_new(0xBEEF, 3);
        int i, ok = 1;
        for (i = 0; i < 10000; i++) {
            if (rng_next_index(&r, 4) >= 4) {
                ok = 0;
            }
        }
        if (rng_next_index(&r, 0) != 0) {
            ok = 0;
        }
        A(ok, "next_index in [0,n) and (0)==0");
    }

    /* next_f64 stays in [0, 1). */
    {
        Rng r = rng_new(0x1234, 5);
        int i, ok = 1;
        for (i = 0; i < 10000; i++) {
            double f = rng_next_f64(&r);
            if (!(f >= 0.0 && f < 1.0)) {
                ok = 0;
            }
        }
        A(ok, "next_f64 in [0,1)");
    }

    /* Categorical sampling is deterministic/reproducible across instances. */
    {
        Rng a = rng_new(0xC0FFEE, 7);
        Rng b = rng_new(0xC0FFEE, 7);
        const double w[4] = {1.0, 1.0, 1.0, 1.0};
        int i, ok = 1;
        for (i = 0; i < 16; i++) {
            if (rng_sample_categorical(&a, w, 4) !=
                rng_sample_categorical(&b, w, 4)) {
                ok = 0;
            }
        }
        A(ok, "categorical reproducible");
    }

#undef A
    return all_ok ? 0 : -1;
}
