/* Bebop comprehensive benchmark — every primitive, honest methodology.
 *
 * Methodology (matches the dowiz "honest benchmark" standard):
 *   - CLOCK_MONOTONIC, warmup, min-of-N + median
 *   - compiler barrier (asm volatile "" ::: "memory") between reps
 *   - every result sunk into a `volatile` accumulator (no DCE)
 *   - INNER-LOOP BATCHING: each timed region runs `inner` iterations so the
 *     measured span is ~µs-scale and the ~200ns clock_gettime floor is
 *     amortized to <1%. Per-op cost = (t1-t0)/inner.
 *
 * Headline number = ns/op (median) and Mops/s.
 */
#define _POSIX_C_SOURCE 200809L

#include "bench_all.h"

#include "ntt.h"
#include "ntt32.h"
#include "hyper.h"
#include "fft.h"
#include "modular.h"
#include "money.h"
#include "sort.h"
#include "checksum.h"
#include "trig.h"
#include "rng.h"
#include "stats.h"
#include "markov.h"
#include "pid.h"
#include "token_bucket.h"
#include "atomic.h"
#include "arena.h"
#include "vsa.h"
#include "mem.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define BENCH_REPS 64
#define WARMUP 8

static double now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

static inline void bench_barrier(void) {
    __asm__ __volatile__("" ::: "memory");
}

/* volatile sink: prevents DCE of the measured body. */
static volatile uint64_t bench_sink;

static double median(double *v, int n) {
    for (int i = 1; i < n; i++) {
        double key = v[i];
        int j = i - 1;
        while (j >= 0 && v[j] > key) {
            v[j + 1] = v[j];
            j--;
        }
        v[j + 1] = key;
    }
    if (n % 2 == 0) {
        return (v[n / 2 - 1] + v[n / 2]) / 2.0;
    }
    return v[n / 2];
}

/* Report: ns/op (min+median) + Mops/s. `total` is the per-rep span; divide by
 * `inner` to get per-op. */
static void report(const char *name, double *times_ns, long inner) {
    double mn = times_ns[0];
    for (int i = 1; i < BENCH_REPS; i++) {
        if (times_ns[i] < mn) {
            mn = times_ns[i];
        }
    }
    double md = median(times_ns, BENCH_REPS);
    double ns_op_min = mn / (double)inner;
    double ns_op_med = md / (double)inner;
    printf("%-26s %9.2f ns/op   med %9.2f ns/op   %9.2f Mops/s\n",
           name, ns_op_min, ns_op_med, 1000.0 / ns_op_med);
}

/* ─── NTT ──────────────────────────────────────────────────────────────── */
static void bench_ntt(void) {
    int n = 1024;
    uint64_t *a = calloc((size_t)n, sizeof *a);
    uint64_t *b = calloc((size_t)n, sizeof *b);
    uint64_t *out = calloc((size_t)(2 * n - 1), sizeof *out);
    for (int i = 0; i < n; i++) {
        a[i] = (uint64_t)(i * 2654435761u % 1000000);
        b[i] = (uint64_t)((i * 7) % 1000000);
    }
    const long inner = 16;
    double t[BENCH_REPS];
    for (int w = 0; w < WARMUP; w++) {
        ntt_convolve(a, (size_t)n, b, (size_t)n, out);
    }
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        for (long k = 0; k < inner; k++) {
            ntt_convolve(a, (size_t)n, b, (size_t)n, out);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += out[(size_t)n / 2];
        t[r] = t1 - t0;
    }
    report("ntt_convolve n=1024", t, inner);
    free(a);
    free(b);
    free(out);
}

/* ─── NTT32 (uint32 quantized) ─────────────────────────────────────────── */
static void bench_ntt32(void) {
    int n = 1024;
    uint32_t *a = calloc((size_t)n, sizeof *a);
    uint32_t *b = calloc((size_t)n, sizeof *b);
    uint32_t *out = calloc((size_t)(2 * n - 1), sizeof *out);
    for (int i = 0; i < n; i++) {
        a[i] = (uint32_t)(i * 2654435761u % 1000000);
        b[i] = (uint32_t)((i * 7) % 1000000);
    }
    const long inner = 16;
    double t[BENCH_REPS];
    for (int w = 0; w < WARMUP; w++)
        ntt32_convolve(a, (size_t)n, b, (size_t)n, out);
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        for (long k = 0; k < inner; k++)
            ntt32_convolve(a, (size_t)n, b, (size_t)n, out);
        bench_barrier();
        double t1 = now_ns();
        bench_sink += out[(size_t)n / 2];
        t[r] = t1 - t0;
    }
    report("ntt32_convolve n=1024", t, inner);
    free(a); free(b); free(out);
}

/* ─── Hypervector ──────────────────────────────────────────────────────── */
static void bench_hyper(void) {
    Hypervector a = hv_code(1), b = hv_code(2);
    double t[BENCH_REPS];
    const long inner = 4096;

    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Hypervector c = a;
        for (long k = 0; k < inner; k++) {
            c = hv_bind(&a, &b);
            bench_sink ^= c.words[0];
        }
        bench_barrier();
        double t1 = now_ns();
        t[r] = t1 - t0;
    }
    report("hv_bind scalar", t, inner);

    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Hypervector c = a;
        for (long k = 0; k < inner; k++) {
            c = hv_bind_neon2(&a, &b);
            bench_sink ^= c.words[0];
        }
        bench_barrier();
        double t1 = now_ns();
        t[r] = t1 - t0;
    }
    report("hv_bind NEON2", t, inner);

    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        uint32_t h = 0;
        for (long k = 0; k < inner; k++) {
            h += hv_hamming(&a, &b);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += h;
        t[r] = t1 - t0;
    }
    report("hv_hamming scalar", t, inner);

    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        uint32_t h = 0;
        for (long k = 0; k < inner; k++) {
            h += hv_hamming_neon(&a, &b);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += h;
        t[r] = t1 - t0;
    }
    report("hv_hamming NEON", t, inner);

    Hypervector items[4];
    for (int i = 0; i < 4; i++) {
        items[i] = hv_code((uint64_t)(10 + i));
    }
    const long inner_b = 512;
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Hypervector c = a;
        for (long k = 0; k < inner_b; k++) {
            c = hv_bundle(items, 4);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += c.words[0];
        t[r] = t1 - t0;
    }
    report("hv_bundle(4)", t, inner_b);

    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Hypervector c = a;
        for (long k = 0; k < inner; k++) {
            c = hv_permute(&a, 37);
            bench_sink ^= c.words[0];
        }
        bench_barrier();
        double t1 = now_ns();
        t[r] = t1 - t0;
    }
    report("hv_permute", t, inner);

    const long inner_s = 16;
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        double s = 0;
        for (long k = 0; k < inner_s; k++) {
            s += hv_shift_invariant_similarity(&a, &b);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(s * 1e6);
        t[r] = t1 - t0;
    }
    report("hv_shift_invariant_sim", t, inner_s);

    const char *txt = "quantum hypervector superposition oracle";
    const long inner_t = 64;
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Hypervector c = a;
        for (long k = 0; k < inner_t; k++) {
            c = hv_encode_text(txt);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += c.words[0];
        t[r] = t1 - t0;
    }
    report("hv_encode_text", t, inner_t);
}

/* ─── FFT ──────────────────────────────────────────────────────────────── */
static void bench_fft(void) {
    int n = 1024;
    Complex *x = calloc((size_t)n, sizeof *x);
    const long inner = 32;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        for (int i = 0; i < n; i++) {
            x[i] = c_new((double)(i % 17) * 0.1, 0.0);
        }
        double t0 = now_ns();
        for (long k = 0; k < inner; k++) {
            fft_inplace(x, (size_t)n, 0);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(x[n / 2].re * 1e6);
        t[r] = t1 - t0;
    }
    report("fft n=1024", t, inner);
    free(x);
}

/* ─── Modular (Möbius) ─────────────────────────────────────────────────── */
static void bench_modular(void) {
    Mobius m = mobius_compose(mobius_s(), mobius_t());
    Complex z = c_new(0.3, 1.5);
    const long inner = 4096;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Complex w = z;
        for (long k = 0; k < inner; k++) {
            w = mobius_apply(m, w);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(w.re * 1e9) + (uint64_t)(w.im * 1e9);
        t[r] = t1 - t0;
    }
    report("mobius_apply", t, inner);

    const long inner_r = 256;
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Complex w = z;
        for (long k = 0; k < inner_r; k++) {
            w = mobius_reduce(z, 20); /* reset from z: full reduction each iter */
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(w.re * 1e9);
        t[r] = t1 - t0;
    }
    report("mobius_reduce(20)", t, inner_r);
}

/* ─── Money ────────────────────────────────────────────────────────────── */
static void bench_money(void) {
    Money a = money_new(5000, CUR_EUR);
    Money b = money_new(1234, CUR_EUR);
    char err[64];
    const long inner = 4096;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Money o = a;
        for (long k = 0; k < inner; k++) {
            int rc = money_checked_add(a, b, &o, err, sizeof err);
            bench_sink += (uint64_t)rc;
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)o.minor;
        t[r] = t1 - t0;
    }
    report("money_checked_add", t, inner);
}

/* ─── Sort ─────────────────────────────────────────────────────────────── */
static void bench_sort(void) {
    int n = 10000;
    double *x = malloc((size_t)n * sizeof *x);
    const long inner = 4;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        for (long k = 0; k < inner; k++) {
            for (int i = 0; i < n; i++) {
                x[i] = (double)((i * 2654435761u + (uint32_t)k) & 0xfffff);
            }
            sort_f64_desc(x, (size_t)n);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)x[0];
        t[r] = t1 - t0;
    }
    report("sort_f64_desc n=10000", t, inner);
    free(x);
}

/* ─── Checksum ─────────────────────────────────────────────────────────── */
static void bench_checksum(void) {
    uint8_t buf[4096];
    for (int i = 0; i < 4096; i++) {
        buf[i] = (uint8_t)(i * 31);
    }
    const long inner = 256;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        uint64_t h = 0;
        for (long k = 0; k < inner; k++) {
            h += checksum_fold(buf, sizeof buf);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += h;
        t[r] = t1 - t0;
    }
    report("checksum_fold 4KB", t, inner);
}

/* ─── Trig ─────────────────────────────────────────────────────────────── */
static void bench_trig(void) {
    const long inner = 2048;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        double acc = 0;
        for (long k = 0; k < inner; k++) {
            acc += trig_sin(0.7) + trig_cos(0.7) + trig_atan2(3.0, 4.0);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(acc * 1e12);
        t[r] = t1 - t0;
    }
    report("trig sin+cos+atan2", t, inner);
}

/* ─── RNG ──────────────────────────────────────────────────────────────── */
static void bench_rng(void) {
    Rng r = rng_new(42, 1);
    const long inner = 65536;
    double t[BENCH_REPS];
    for (int r_ = 0; r_ < BENCH_REPS; r_++) {
        double t0 = now_ns();
        uint64_t acc = 0;
        for (long k = 0; k < inner; k++) {
            acc ^= rng_next_u64(&r);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += acc;
        t[r_] = t1 - t0;
    }
    report("rng_next_u64", t, inner);
}

/* ─── Stats ────────────────────────────────────────────────────────────── */
static void bench_stats(void) {
    int n = 1024;
    double *x = malloc((size_t)n * sizeof *x);
    for (int i = 0; i < n; i++) {
        x[i] = (double)(i % 100);
    }
    const long inner = 64;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        double acc = 0;
        for (long k = 0; k < inner; k++) {
            acc += stats_mean(x, (size_t)n) + stats_variance(x, (size_t)n);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(acc * 1e6);
        t[r] = t1 - t0;
    }
    report("stats mean+variance n=1024", t, inner);
    free(x);

    const long inner_p = 65536;
    for (int r = 0; r < BENCH_REPS; r++) {
        RunningStats rs;
        stats_running_init(&rs);
        double t0 = now_ns();
        for (long k = 0; k < inner_p; k++) {
            stats_running_push(&rs, (double)(k & 255));
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(rs.mean * 1e6);
        t[r] = t1 - t0;
    }
    report("stats_running_push", t, inner_p);
}

/* ─── Markov ───────────────────────────────────────────────────────────── */
static void bench_markov(void) {
    MarkovMatrix m;
    markov_matrix_init(&m, 8);
    for (int i = 0; i < 8; i++) {
        for (int j = 0; j < 8; j++) {
            m.m[i][j] = (i == j) ? 2.0 : 1.0;
        }
    }
    markov_row_normalize(&m);
    const long inner = 64;
    double out[8];
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        double acc = 0;
        for (long k = 0; k < inner; k++) {
            markov_stationary(&m, 0.85, 200, out);
            acc += out[0];
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(acc * 1e9);
        t[r] = t1 - t0;
    }
    report("markov_stationary n=8", t, inner);
}

/* ─── PID ──────────────────────────────────────────────────────────────── */
static void bench_pid(void) {
    BebopPid p = pid_new(1.0, 0.1, 0.01, -100.0, 100.0);
    const long inner = 4096;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        double acc = 0;
        for (long k = 0; k < inner; k++) {
            acc += pid_update(&p, 50.0, 40.0);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(acc * 1e6);
        t[r] = t1 - t0;
    }
    report("pid_update", t, inner);
}

/* ─── Token bucket (GCRA) ──────────────────────────────────────────────── */
static void bench_gcra(void) {
    const long inner = 65536;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        uint64_t out = 0;
        for (long k = 0; k < inner; k++) {
            gcra_decide((uint64_t)k * 1000, 0, 1000, 5000, &out);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += out;
        t[r] = t1 - t0;
    }
    report("gcra_decide", t, inner);
}

/* ─── Atomic ───────────────────────────────────────────────────────────── */
static void bench_atomic(void) {
    AtomicU64 a = 0;
    const long inner = 4096;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        uint64_t v = 0;
        for (long k = 0; k < inner; k++) {
            v += bp_atomic_fetch_add(&a, 1);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += v;
        t[r] = t1 - t0;
    }
    report("atomic fetch_add", t, inner);

    Spinlock l;
    spinlock_init(&l);
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        for (long k = 0; k < inner; k++) {
            spinlock_lock(&l);
            spinlock_unlock(&l);
        }
        bench_barrier();
        double t1 = now_ns();
        t[r] = t1 - t0;
    }
    report("spinlock lock+unlock", t, inner);
}

/* ─── Arena ────────────────────────────────────────────────────────────── */
static void bench_arena(void) {
    unsigned char buf[1 << 16];
    BumpArena ar;
    const long inner = 65536;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        arena_init(&ar, buf, sizeof buf);
        double t0 = now_ns();
        void *p = NULL;
        for (long k = 0; k < inner; k++) {
            p = arena_alloc(&ar, 64);
            if ((k & 1023) == 1023) {
                arena_reset(&ar);
            }
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += (uint64_t)(uintptr_t)p;
        t[r] = t1 - t0;
    }
    report("arena_alloc(64)", t, inner);
}

/* ─── VSA ──────────────────────────────────────────────────────────────── */
static void bench_vsa(void) {
    const long inner = 256;
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        Hypervector v = hv_zero();
        for (long k = 0; k < inner; k++) {
            v = vsa_encode("glyphic");
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += v.words[0];
        t[r] = t1 - t0;
    }
    report("vsa_encode", t, inner);
}

/* ─── Memory (living-memory search) ────────────────────────────────────── */
static void bench_mem(void) {
    Memory m;
    mem_init(&m);
    const char *names[32] = {
        "auth", "ntt", "hyper", "money", "quantum", "oracle", "mesh", "crypto",
        "ledger", "fdr", "pid", "markov", "spectral", "arena", "event", "vsa",
        "token", "sort", "stats", "rng", "trig", "checksum", "hex", "modular",
        "fft", "complex", "atomic", "glyph", "morse", "codegen", "native", "mem"};
    for (int i = 0; i < 32; i++) {
        mem_add(&m, names[i], "semantic");
    }
    const long inner = 256;
    size_t ids[8];
    double t[BENCH_REPS];
    for (int r = 0; r < BENCH_REPS; r++) {
        double t0 = now_ns();
        size_t k = 0;
        for (long kk = 0; kk < inner; kk++) {
            k += mem_search_semantic(&m, "quantum", 8, ids, 8);
        }
        bench_barrier();
        double t1 = now_ns();
        bench_sink += k;
        t[r] = t1 - t0;
    }
    report("mem_search_semantic", t, inner);
}

int bench_all_run(void) {
    printf("=== Bebop native benchmark (%d reps, inner-batched, compiler barriers) ===\n",
           BENCH_REPS);
    bench_ntt();
    bench_ntt32();
    bench_hyper();
    bench_fft();
    bench_modular();
    bench_money();
    bench_sort();
    bench_checksum();
    bench_trig();
    bench_rng();
    bench_stats();
    bench_markov();
    bench_pid();
    bench_gcra();
    bench_atomic();
    bench_arena();
    bench_vsa();
    bench_mem();
    printf("(sink=%llu — prevents dead-code elimination)\n",
           (unsigned long long)bench_sink);
    return 0;
}

/* ─── Batched parallel benchmark ───────────────────────────────────────...[truncated]
