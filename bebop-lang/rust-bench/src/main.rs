//! Bebop↔Rust comparison benchmark — the SAME primitives as `bench_all.c`,
//! measured against the dowiz-core Rust reference with the same methodology:
//! inner-loop batching, min/median of N reps, `black_box` to block DCE.
//!
//! This is the language comparison: Bebop native C bootstrap vs the Rust
//! reference implementation of the same algorithms.

use std::hint::black_box;
use std::time::Instant;

const REPS: usize = 64;

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 0 {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    }
}

fn report(name: &str, times_ns: &[f64], inner: usize) {
    let mut v = times_ns.to_vec();
    let mn = v.iter().cloned().fold(f64::INFINITY, f64::min);
    let md = median(&mut v);
    let ns_op_min = mn / inner as f64;
    let ns_op_med = md / inner as f64;
    println!(
        "{:<26} {:9.2} ns/op   med {:9.2} ns/op   {:9.2} Mops/s",
        name,
        ns_op_min,
        ns_op_med,
        1000.0 / ns_op_med
    );
}

fn main() {
    println!("=== Rust dowiz-core benchmark ({} reps, inner-batched, black_box) ===", REPS);

    bench_ntt();
    bench_hyper();
    bench_fft();
    bench_modular();
    bench_money();
    bench_sort();
    bench_checksum();
    bench_trig();
    bench_rng();
    bench_stats();
    bench_gcra();
}

fn bench_ntt() {
    use dowiz_core::ntt::convolve;
    let n = 1024;
    let a: Vec<u64> = (0..n).map(|i| (i as u64 * 2654435761 % 1000000)).collect();
    let b: Vec<u64> = (0..n).map(|i| (i as u64 * 7 % 1000000)).collect();
    let inner = 16usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut out = a.clone();
        for _ in 0..inner {
            out = convolve(&a, &b);
        }
        black_box(&out);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("ntt_convolve n=1024", &t, inner);
}

fn bench_hyper() {
    use dowiz_core::hypervector::Hypervector;
    let a = Hypervector::code(1);
    let b = Hypervector::code(2);
    let inner = 4096usize;
    let mut t = vec![0.0f64; REPS];

    for r in 0..REPS {
        let t0 = Instant::now();
        let mut sink = 0u64;
        for _ in 0..inner {
            let (aa, bb) = (black_box(&a), black_box(&b));
            let w = aa.bind(bb);
            sink ^= w.as_words().iter().fold(0u64, |acc, &x| acc ^ x);
        }
        black_box(&sink);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("hv_bind", &t, inner);

    for r in 0..REPS {
        let t0 = Instant::now();
        let mut h = 0u32;
        for _ in 0..inner {
            let (aa, bb) = (black_box(&a), black_box(&b));
            h = h.wrapping_add(aa.hamming(bb));
        }
        black_box(&h);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("hv_hamming", &t, inner);

    let items = [Hypervector::code(10), Hypervector::code(11), Hypervector::code(12), Hypervector::code(13)];
    let inner_b = 512usize;
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut c = a;
        for _ in 0..inner_b {
            c = Hypervector::bundle([&items[0], &items[1], &items[2], &items[3]]);
        }
        black_box(&c);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("hv_bundle(4)", &t, inner_b);

    for r in 0..REPS {
        let t0 = Instant::now();
        let mut sink = 0u64;
        for _ in 0..inner {
            let w = black_box(&a).permute(37);
            sink ^= w.as_words().iter().fold(0u64, |acc, &x| acc ^ x);
        }
        black_box(&sink);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("hv_permute", &t, inner);

    let inner_s = 16usize;
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut s = 0.0f64;
        for _ in 0..inner_s {
            s += a.shift_invariant_similarity(&b);
        }
        black_box(&s);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("hv_shift_invariant_sim", &t, inner_s);
}

fn bench_fft() {
    use dowiz_core::fft::fft;
    use dowiz_core::complex::Complex;
    let n = 1024;
    let x: Vec<Complex> = (0..n).map(|i| Complex::new((i % 17) as f64 * 0.1, 0.0)).collect();
    let inner = 32usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut y = None;
        for _ in 0..inner {
            y = fft(&x);
        }
        black_box(&y);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("fft n=1024", &t, inner);
}

fn bench_modular() {
    use dowiz_core::complex::Complex;
    use dowiz_core::modular::{Mobius, reduce};
    let m = Mobius::s().compose(Mobius::t());
    let z = Complex::new(0.3, 1.5);
    let inner = 4096usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut w = z;
        for _ in 0..inner {
            w = m.apply(w);
        }
        black_box(&w);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("mobius_apply", &t, inner);

    let inner_r = 256usize;
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut w = z;
        for _ in 0..inner_r {
            w = reduce(black_box(z), 20); /* reset from z: full reduction each iter */
        }
        black_box(&w);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("mobius_reduce(20)", &t, inner_r);
}

fn bench_money() {
    use dowiz_core::money::{Currency, Money};
    let a = Money::new(5000, Currency::Eur);
    let b = Money::new(1234, Currency::Eur);
    let inner = 4096usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut o = a;
        for _ in 0..inner {
            let (aa, bb) = (black_box(&a), black_box(&b));
            o = aa.checked_add(*bb).unwrap_or(a);
        }
        black_box(&o);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("money_checked_add", &t, inner);
}

fn bench_sort() {
    use dowiz_core::sort::sort_by_f64_desc;
    let n = 10000;
    let inner = 4usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        for k in 0..inner {
            let mut x: Vec<f64> = (0..n)
                .map(|i| ((i as u64 * 2654435761 + k as u64) & 0xfffff) as f64)
                .collect();
            sort_by_f64_desc(&mut x, |v| *v);
            black_box(&x[0]);
        }
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("sort_f64_desc n=10000", &t, inner);
}

fn bench_checksum() {
    use dowiz_core::checksum::checksum_fold;
    let buf: Vec<u8> = (0..4096).map(|i| (i as u8).wrapping_mul(31)).collect();
    let inner = 256usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut h = 0u64;
        for _ in 0..inner {
            h = h.wrapping_add(checksum_fold(&buf));
        }
        black_box(&h);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("checksum_fold 4KB", &t, inner);
}

fn bench_trig() {
    // Match the C side: sin + cos + atan2 (3 transcendental calls), using the
    // same hand-rolled dowiz math layer (bit-exact across native/wasm).
    use dowiz_core::math;
    let inner = 2048usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut acc = 0.0f64;
        for k in 0..inner {
            let th = black_box(0.7 + (k as f64) * 1e-9);
            acc += math::sin(th) + math::cos(th) + math::atan2(3.0, 4.0);
        }
        black_box(&acc);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("trig sin+cos+atan2", &t, inner);
}

fn bench_rng() {
    use dowiz_core::rng::Rng;
    let mut r = Rng::new(42, 1);
    let inner = 65536usize;
    let mut t = vec![0.0f64; REPS];
    for r_ in 0..REPS {
        let t0 = Instant::now();
        let mut acc = 0u64;
        for _ in 0..inner {
            acc ^= r.next_u64();
        }
        black_box(&acc);
        let t1 = Instant::now();
        t[r_] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("rng_next_u64", &t, inner);
}

fn bench_stats() {
    // Same Kahan-compensated mean + two-pass variance as the Bebop C port
    // (the dowiz-core `stats` module holds welch/chi2/cohens_d — different
    // functions — so the descriptive-stats algorithm is inlined here for a
    // same-algorithm language comparison).
    fn kahan_mean(x: &[f64]) -> f64 {
        if x.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0f64;
        let mut c = 0.0f64;
        for &v in x {
            let y = v - c;
            let t = sum + y;
            c = (t - sum) - y;
            sum = t;
        }
        sum / x.len() as f64
    }
    fn kahan_var(x: &[f64]) -> f64 {
        if x.len() < 2 {
            return 0.0;
        }
        let m = kahan_mean(x);
        let mut ss = 0.0f64;
        let mut c = 0.0f64;
        for &v in x {
            let d = v - m;
            let y = d * d - c;
            let t = ss + y;
            c = (t - ss) - y;
            ss = t;
        }
        ss / (x.len() - 1) as f64
    }
    let n = 1024;
    let x: Vec<f64> = (0..n).map(|i| (i % 100) as f64).collect();
    let inner = 64usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut acc = 0.0f64;
        for _ in 0..inner {
            acc += kahan_mean(&x) + kahan_var(&x);
        }
        black_box(&acc);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("stats mean+variance n=1024", &t, inner);
}

fn bench_gcra() {
    use dowiz_core::token_bucket::gcra_decide;
    let inner = 65536usize;
    let mut t = vec![0.0f64; REPS];
    for r in 0..REPS {
        let t0 = Instant::now();
        let mut out = 0u64;
        for k in 0..inner {
            let now = black_box((k as u64) * 1000);
            out = out.wrapping_add(gcra_decide(now, 0, 1000, 5000).map(|v| v).unwrap_or(0));
        }
        black_box(&out);
        let t1 = Instant::now();
        t[r] = t1.duration_since(t0).as_nanos() as f64;
    }
    report("gcra_decide", &t, inner);
}
