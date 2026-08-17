/* Bebop Hypervector — implementation (port of dowiz hypervector.rs). */
#define _POSIX_C_SOURCE 199309L
#include "hyper.h"

#include <limits.h>
#include <stdio.h>
#include <string.h>

#include "ntt.h"

static uint64_t splitmix64(uint64_t *state) {
    uint64_t z = (*state += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

Hypervector hv_zero(void) {
    Hypervector v;
    memset(&v, 0, sizeof v);
    return v;
}

Hypervector hv_code(uint64_t seed) {
    Hypervector v;
    uint64_t s = seed;
    for (int i = 0; i < BEBOP_HV_WORDS; i++) {
        v.words[i] = splitmix64(&s);
    }
    return v;
}

uint64_t hv_hash(const char *s) {
    uint64_t h = 5381;
    while (*s) {
        h = ((h << 5) + h) + (unsigned char)*s++;
    }
    return h;
}

Hypervector hv_encode_text(const char *text) {
    size_t len = strlen(text);
    Hypervector trigrams[256];
    size_t n = 0;
    for (size_t i = 0; i + 3 <= len && n < 256; i++) {
        char tri[4];
        memcpy(tri, text + i, 3);
        tri[3] = '\0';
        trigrams[n++] = hv_code(hv_hash(tri));
    }
    if (n == 0) {
        return hv_code(hv_hash(text));
    }
    return hv_bundle(trigrams, n);
}

Hypervector hv_bind(const Hypervector *a, const Hypervector *b) {
    Hypervector v;
    for (int i = 0; i < BEBOP_HV_WORDS; i++) {
        v.words[i] = a->words[i] ^ b->words[i];
    }
    return v;
}

Hypervector hv_bundle(const Hypervector *items, size_t n) {
    int32_t counts[BEBOP_HV_D];
    memset(counts, 0, sizeof counts);
    for (size_t k = 0; k < n; k++) {
        for (int w = 0; w < BEBOP_HV_WORDS; w++) {
            for (int b = 0; b < 64; b++) {
                if ((items[k].words[w] >> b) & 1) {
                    counts[w * 64 + b]++;
                }
            }
        }
    }
    Hypervector v = hv_zero();
    if (n > 0) {
        for (int i = 0; i < BEBOP_HV_D; i++) {
            if (counts[i] * 2 > (int32_t)n) {
                v.words[i / 64] |= 1ULL << (i % 64);
            }
        }
    }
    return v;
}

uint32_t hv_hamming(const Hypervector *a, const Hypervector *b) {
    uint32_t diff = 0;
    for (int i = 0; i < BEBOP_HV_WORDS; i++) {
        diff += (uint32_t)__builtin_popcountll(a->words[i] ^ b->words[i]);
    }
    return diff;
}

double hv_similarity(const Hypervector *a, const Hypervector *b) {
    return (double)(BEBOP_HV_D - hv_hamming(a, b)) / (double)BEBOP_HV_D;
}

Hypervector hv_permute(const Hypervector *v, uint32_t shift) {
    shift %= BEBOP_HV_D;
    if (shift == 0) {
        return *v;
    }
    uint32_t word_shift = shift / 64;
    uint32_t bit_shift = shift % 64;
    Hypervector out = hv_zero();
    for (int i = 0; i < BEBOP_HV_WORDS; i++) {
        int src = (i + BEBOP_HV_WORDS - (int)word_shift) % BEBOP_HV_WORDS;
        uint64_t hi = v->words[src] << bit_shift;
        uint64_t lo = 0;
        if (bit_shift != 0) {
            int prev = (src + BEBOP_HV_WORDS - 1) % BEBOP_HV_WORDS;
            lo = v->words[prev] >> (64 - bit_shift);
        }
        out.words[i] = hi | lo;
    }
    return out;
}

uint32_t hv_popcount(const Hypervector *v) {
    uint32_t n = 0;
    for (int i = 0; i < BEBOP_HV_WORDS; i++) {
        n += (uint32_t)__builtin_popcountll(v->words[i]);
    }
    return n;
}

int hv_to_hex(const Hypervector *v, char *out, size_t cap) {
    if (cap < BEBOP_HV_WORDS * 16 + 1) {
        return -1;
    }
    static const char HEX[] = "0123456789abcdef";
    size_t pos = 0;
    for (int i = 0; i < BEBOP_HV_WORDS; i++) {
        for (int shift = 60; shift >= 0; shift -= 4) {
            out[pos++] = HEX[(v->words[i] >> shift) & 0xf];
        }
    }
    out[pos] = '\0';
    return 0;
}

int hv_from_hex(const char *s, Hypervector *out) {
    if (strlen(s) != BEBOP_HV_WORDS * 16) {
        return -1;
    }
    for (int i = 0; i < BEBOP_HV_WORDS; i++) {
        uint64_t v = 0;
        for (int j = 0; j < 16; j++) {
            char c = s[i * 16 + j];
            uint64_t d;
            if (c >= '0' && c <= '9') d = (uint64_t)(c - '0');
            else if (c >= 'a' && c <= 'f') d = (uint64_t)(c - 'a' + 10);
            else if (c >= 'A' && c <= 'F') d = (uint64_t)(c - 'A' + 10);
            else return -1;
            v = (v << 4) | d;
        }
        out->words[i] = v;
    }
    return 0;
}

double hv_shift_invariant_similarity(const Hypervector *a, const Hypervector *b) {
    static uint64_t av[BEBOP_HV_D], bv[BEBOP_HV_D], a_rev[BEBOP_HV_D], corr[BEBOP_HV_D];
    for (int i = 0; i < BEBOP_HV_D; i++) {
        av[i] = ((a->words[i / 64] >> (i % 64)) & 1) ? 1 : (BEBOP_NTT_MOD - 1);
    }
    for (int i = 0; i < BEBOP_HV_D; i++) {
        bv[i] = ((b->words[i / 64] >> (i % 64)) & 1) ? 1 : (BEBOP_NTT_MOD - 1);
    }
    a_rev[0] = av[0];
    for (int i = 1; i < BEBOP_HV_D; i++) {
        a_rev[i] = av[BEBOP_HV_D - i];
    }
    ntt_circular(a_rev, bv, BEBOP_HV_D, corr);
    int64_t best = INT64_MIN;
    for (int i = 0; i < BEBOP_HV_D; i++) {
        int64_t c = ntt_centered(corr[i]);
        if (c > best) {
            best = c;
        }
    }
    double sim = ((double)best + (double)BEBOP_HV_D) / (2.0 * (double)BEBOP_HV_D);
    if (sim < 0.0) sim = 0.0;
    if (sim > 1.0) sim = 1.0;
    return sim;
}

int hyper_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define H(cond, name)                                                    \
    do {                                                                 \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",              \
                         (cond) ? "ok" : "FAIL", name);                  \
        if (r > 0) pos += (size_t)r;                                     \
        if (!(cond)) all_ok = 0;                                         \
    } while (0)

    Hypervector a = hv_code(1), b = hv_code(2), c = hv_code(3);
    Hypervector bound = hv_bind(&a, &b);
    Hypervector unbound = hv_bind(&bound, &b);
    H(memcmp(&unbound, &a, sizeof a) == 0, "bind is self-inverse");
    Hypervector ba = hv_bind(&b, &a);
    H(memcmp(&ba, &bound, sizeof bound) == 0, "bind commutative");

    double sim = hv_similarity(&a, &b);
    H(sim > 0.45 && sim < 0.55, "distinct codes ~0.5");
    Hypervector c42 = hv_code(42);
    H(hv_similarity(&c42, &c42) == 1.0, "same seed identical");

    Hypervector items[3] = {a, b, c};
    Hypervector bundled = hv_bundle(items, 3);
    H(hv_similarity(&bundled, &a) > 0.55 && hv_similarity(&bundled, &b) > 0.55 &&
      hv_similarity(&bundled, &c) > 0.55, "bundle similar to constituents");

    Hypervector v99 = hv_code(99);
    Hypervector p0 = hv_permute(&v99, 0);
    H(memcmp(&p0, &v99, sizeof v99) == 0, "permute(0) == identity");
    Hypervector half = hv_permute(&v99, BEBOP_HV_D / 2);
    Hypervector hh = hv_permute(&half, BEBOP_HV_D / 2);
    H(memcmp(&hh, &v99, sizeof v99) == 0,
      "permute(D/2) twice == identity");

    char hex[BEBOP_HV_WORDS * 16 + 1];
    hv_to_hex(&v99, hex, sizeof hex);
    Hypervector back;
    H(hv_from_hex(hex, &back) == 0 && memcmp(&back, &v99, sizeof v99) == 0,
      "hex round-trip");
    H(hv_from_hex("zz", &back) != 0, "hex rejects non-hex");

    int rot_ok = 1;
    uint32_t shifts[5] = {1, 37, 512, 1000, 1023};
    for (int i = 0; i < 5; i++) {
        Hypervector r = hv_permute(&v99, shifts[i]);
        double s = hv_shift_invariant_similarity(&v99, &r);
        if (!(s > 0.999999999)) rot_ok = 0;
    }
    H(rot_ok, "shift-invariant finds rotations (~1.0)");

    Hypervector h1 = hv_code(1), h2 = hv_code(2);
    double sun = hv_shift_invariant_similarity(&h1, &h2);
    H(sun > 0.45 && sun < 0.55, "shift-invariant unrelated ~0.5");

    Hypervector nb = hv_bind_neon(&a, &b);
    H(memcmp(&nb, &bound, sizeof nb) == 0, "NEON bind == scalar bind");
    H(hv_hamming_neon(&a, &b) == hv_hamming(&a, &b), "NEON hamming == scalar hamming");

    return all_ok ? 0 : -1;
}

/* ─── NEON SIMD (AArch64) ─── */
#ifdef __aarch64__
#include <arm_neon.h>

Hypervector hv_bind_neon(const Hypervector *a, const Hypervector *b) {
    Hypervector v;
    for (int i = 0; i < BEBOP_HV_WORDS; i += 2) {
        uint64x2_t va = vld1q_u64(&a->words[i]);
        uint64x2_t vb = vld1q_u64(&b->words[i]);
        vst1q_u64(&v.words[i], veorq_u64(va, vb));
    }
    return v;
}

uint32_t hv_hamming_neon(const Hypervector *a, const Hypervector *b) {
    uint32_t total = 0;
    for (int i = 0; i < BEBOP_HV_WORDS; i += 2) {
        uint64x2_t vx = veorq_u64(vld1q_u64(&a->words[i]),
                                  vld1q_u64(&b->words[i]));
        total += (uint32_t)vaddvq_u8(vcntq_u8(vreinterpretq_u8_u64(vx)));
    }
    return total;
}
#else
Hypervector hv_bind_neon(const Hypervector *a, const Hypervector *b) {
    return hv_bind(a, b);
}
uint32_t hv_hamming_neon(const Hypervector *a, const Hypervector *b) {
    return hv_hamming(a, b);
}
#endif

#include <time.h>

int hv_benchmark(char *out, size_t cap) {
    Hypervector a = hv_code(1), b = hv_code(2);
    const int N = 5000000;
    volatile uint64_t sink = 0;
    struct timespec t0, t1;
    double sb, nb, sh, nh;

    Hypervector v = hv_zero();
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int i = 0; i < N; i++) {
        v = hv_bind(&a, &b);
        sink ^= v.words[0];
        __asm__ __volatile__("" ::: "memory");
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    sb = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9;

    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int i = 0; i < N; i++) {
        v = hv_bind_neon(&a, &b);
        sink ^= v.words[0];
        __asm__ __volatile__("" ::: "memory");
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    nb = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9;

    double n2;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int i = 0; i < N; i++) {
        v = hv_bind_neon2(&a, &b);
        sink ^= v.words[0];
        __asm__ __volatile__("" ::: "memory");
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    n2 = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9;

    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int i = 0; i < N; i++) {
        sink ^= hv_hamming(&a, &b);
        __asm__ __volatile__("" ::: "memory");
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    sh = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9;

    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int i = 0; i < N; i++) {
        sink ^= hv_hamming_neon(&a, &b);
        __asm__ __volatile__("" ::: "memory");
    }
    clock_gettime(CLOCK_MONOTONIC, &t1);
    nh = (t1.tv_sec - t0.tv_sec) + (t1.tv_nsec - t0.tv_nsec) / 1e9;

    (void)sink;
    return snprintf(out, cap,
        "hypervector bind:   scalar %7.1f | NEON %7.1f | NEON2 %7.1f Mops/s\n"
        "hypervector hamming: scalar %7.1f Mops/s | NEON %7.1f Mops/s\n",
        N / sb / 1e6, N / nb / 1e6, N / n2 / 1e6, N / sh / 1e6, N / nh / 1e6);
}

/* 2x-unrolled NEON bind: 4 u64 (256-bit) per iteration via two 128-bit vectors. */
Hypervector hv_bind_neon2(const Hypervector *a, const Hypervector *b) {
    Hypervector v;
    for (int i = 0; i < BEBOP_HV_WORDS; i += 4) {
        uint64x2_t va0 = vld1q_u64(&a->words[i]);
        uint64x2_t va1 = vld1q_u64(&a->words[i + 2]);
        uint64x2_t vb0 = vld1q_u64(&b->words[i]);
        uint64x2_t vb1 = vld1q_u64(&b->words[i + 2]);
        vst1q_u64(&v.words[i], veorq_u64(va0, vb0));
        vst1q_u64(&v.words[i + 2], veorq_u64(va1, vb1));
    }
    return v;
}
