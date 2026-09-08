// join_csr.rs — B2 twin (i), THIRD Rust side: the SAME ALGORITHM bebop runs (counting-sort CSR
// over the dense key domain + Gustavson probe), written the way a competent Rust author would
// write it (docs/blueprints/B2-decisive-twins.md §3(i), risk row "the Rust side is strawmanned").
//
// Why this file exists at all. The blueprint names `HashMap` and sort-merge as the Rust twins,
// and both are honest *generic* joins — but neither is what a Rust author would write once told
// the join keys are dense integers in [0, n). Told that, they write exactly this: a counting
// sort into rp/ci and a probe over the buckets. Without this row, a bebop win on the gate could
// be entirely an ALGORITHM win (CSR vs hashing) dressed up as a language win, which is the one
// outcome the B2 row must not produce. With it, the `bebop / rust_csr` ratio isolates the thing
// the thesis actually claims: bebop's generated code on the identical algorithm and identical
// data. Treat rust_csr, not rust_hash, as the number that decides "≥ 0.7x best Rust".
//
// Index width. bebop has one array type — i64 — so its rp/ci/cur are 8 bytes per slot. Rust's
// natural choice at n = 1e6 is u32 (4 bytes), which halves the bytes the counting sort and the
// probe pull through the cache and is therefore what "best Rust" means here. To keep the two
// effects separable, this binary carries BOTH widths and selects at run time:
//     argv[3] = "32" (default, best Rust form) | "64" (bebop's i64 layout, same code shape)
// so `rust_csr:64` vs bebop measures codegen alone, and `rust_csr:32` vs `rust_csr:64` prices
// the i64-only array type as a language cost. Both print the same fold — the width is a layout
// choice, never a semantic one.
//
// Generator is bit-for-bit identical to join_hash.rs / join_merge.rs / join_twin.bp /
// bench/oracles/join_twin.py: one continuous LCG stream, R's n rows then S's n rows, 3 draws per
// row regardless of distribution, zipf = the top 1% of keys taking 30% of the rows. Timing, as
// in the sibling twins, covers build+probe only (generation is outside t0, exactly as
// join_twin.bp's mode 't' is t3-t1) and goes to stderr; stdout carries count/checksum/fold.
// REPS (env, default 1) reruns the whole build+probe in-process for the ~100 ms floor that
// bench/vs_rust/kernel_reps.txt requires of any timed row.
use std::hint::black_box;

const A: u64 = 6364136223846793005;
const C: u64 = 1442695040888963407;
const MOD61: i64 = 1i64 << 61;

fn lcg(x: u64) -> u64 { x.wrapping_mul(A).wrapping_add(C) }

fn gen(seed: u64, n: usize, zipf: bool) -> (Vec<i64>, Vec<i64>, Vec<i64>, Vec<i64>) {
    let heavy = std::cmp::max(1, n / 100) as u64;
    let light = n as u64 - heavy;
    let mut x = seed;
    let mut rk = vec![0i64; n];
    let mut ra = vec![0i64; n];
    let mut sk = vec![0i64; n];
    let mut sb = vec![0i64; n];
    for i in 0..n {
        x = lcg(x);
        let k0 = (x >> 20) % n as u64;
        let coin = (x >> 40) % 1000;
        x = lcg(x);
        let kz = if coin < 300 { (x >> 20) % heavy } else { heavy + (x >> 20) % light };
        let k = if zipf { kz } else { k0 };
        x = lcg(x);
        rk[i] = k as i64;
        ra[i] = ((x >> 20) % 65536) as i64;
    }
    for i in 0..n {
        x = lcg(x);
        let k0 = (x >> 20) % n as u64;
        let coin = (x >> 40) % 1000;
        x = lcg(x);
        let kz = if coin < 300 { (x >> 20) % heavy } else { heavy + (x >> 20) % light };
        let k = if zipf { kz } else { k0 };
        x = lcg(x);
        sk[i] = k as i64;
        sb[i] = ((x >> 20) % 65536) as i64;
    }
    (rk, ra, sk, sb)
}

/// Best-Rust form: 4-byte row ids and 4-byte row pointers. Allocation of rp/ci/cur is INSIDE the
/// timed region because it is inside bebop's too (join_twin.bp takes t1 before its three
/// `zeros()` calls), so neither side gets a free zeroed working set.
fn join_fold_csr32(rk: &[i64], ra: &[i64], sk: &[i64], sb: &[i64]) -> (i64, i64) {
    let n = sk.len();
    let mut rp = vec![0u32; n + 1];
    for &k in sk {
        rp[k as usize + 1] += 1;
    }
    for i in 0..n {
        rp[i + 1] += rp[i];
    }
    let mut cur: Vec<u32> = rp[..n].to_vec();
    let mut ci = vec![0u32; n];
    for (s, &k) in sk.iter().enumerate() {
        let p = cur[k as usize];
        ci[p as usize] = s as u32;
        cur[k as usize] = p + 1;
    }
    let mut cnt: i64 = 0;
    let mut chk: i64 = 0;
    for r in 0..rk.len() {
        let k = rk[r] as usize;
        let a = ra[r];
        for &s in &ci[rp[k] as usize..rp[k + 1] as usize] {
            let p = (a * sb[s as usize]) % MOD61;
            cnt = cnt.wrapping_add(1);
            chk = chk.wrapping_add(p);
        }
    }
    (cnt, chk)
}

/// bebop's layout: every working array is i64, because that is the only array element type the
/// language has. Same code shape as csr32 — the only difference is 8 bytes per slot instead of 4.
fn join_fold_csr64(rk: &[i64], ra: &[i64], sk: &[i64], sb: &[i64]) -> (i64, i64) {
    let n = sk.len();
    let mut rp = vec![0i64; n + 1];
    for &k in sk {
        rp[k as usize + 1] += 1;
    }
    for i in 0..n {
        rp[i + 1] += rp[i];
    }
    let mut cur: Vec<i64> = rp[..n].to_vec();
    let mut ci = vec![0i64; n];
    for (s, &k) in sk.iter().enumerate() {
        let p = cur[k as usize];
        ci[p as usize] = s as i64;
        cur[k as usize] = p + 1;
    }
    let mut cnt: i64 = 0;
    let mut chk: i64 = 0;
    for r in 0..rk.len() {
        let k = rk[r] as usize;
        let a = ra[r];
        for &s in &ci[rp[k] as usize..rp[k + 1] as usize] {
            let p = (a * sb[s as usize]) % MOD61;
            cnt = cnt.wrapping_add(1);
            chk = chk.wrapping_add(p);
        }
    }
    (cnt, chk)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(2000);
    let zipf = args.get(2).map(|s| s == "z").unwrap_or(false);
    let width: u32 = args.get(3).map(|s| s.parse().unwrap()).unwrap_or(32);
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let seed: u64 = if zipf { 8823 } else { 4711 };
    let (rk, ra, sk, sb) = gen(seed, n, zipf);
    let t0 = std::time::Instant::now();
    let mut cnt = 0i64;
    let mut chk = 0i64;
    for _ in 0..reps {
        let r = if width == 64 {
            join_fold_csr64(black_box(&rk), black_box(&ra), black_box(&sk), black_box(&sb))
        } else {
            join_fold_csr32(black_box(&rk), black_box(&ra), black_box(&sk), black_box(&sb))
        };
        cnt = r.0;
        chk = r.1;
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    let fold = cnt.wrapping_mul(1000000007).wrapping_add(chk);
    eprintln!("{:.3}", ms);
    println!("count {}\nchecksum {}\nfold {}", cnt, chk, fold);
}
