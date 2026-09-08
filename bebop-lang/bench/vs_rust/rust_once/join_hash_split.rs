// join_hash_split.rs -- bench/vs_rust/rust_once/join_hash.rs VERBATIM except that join_fold is
// split into its BUILD half (filling the HashMap) and its PROBE half (the R-side lookup+fold),
// and each is timed separately. D2's gate is stated per PROBE PHASE, and the committed twin
// times build+probe together. Nothing else is changed: same generator, same LCG stream, same
// HashMap<i64, Vec<u32>>, same MOD61 fold, same black_box discipline.
// argv: n dist('u'|'z'); prints "build_ms probe_ms" on stderr and the folds on stdout.
use std::collections::HashMap;
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

fn build(sk: &[i64]) -> HashMap<i64, Vec<u32>> {
    let mut buckets: HashMap<i64, Vec<u32>> = HashMap::with_capacity(sk.len());
    for (s, &k) in sk.iter().enumerate() {
        buckets.entry(k).or_default().push(s as u32);
    }
    buckets
}

fn probe(buckets: &HashMap<i64, Vec<u32>>, rk: &[i64], ra: &[i64], sb: &[i64]) -> (i64, i64) {
    let mut cnt: i64 = 0;
    let mut chk: i64 = 0;
    for r in 0..rk.len() {
        let k = rk[r];
        let a = ra[r];
        if let Some(v) = buckets.get(&k) {
            for &s in v {
                let p = (a * sb[s as usize]) % MOD61;
                cnt = cnt.wrapping_add(1);
                chk = chk.wrapping_add(p);
            }
        }
    }
    (cnt, chk)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(2000);
    let zipf = args.get(2).map(|s| s == "z").unwrap_or(false);
    let seed: u64 = if zipf { 8823 } else { 4711 };
    let (rk, ra, sk, sb) = gen(seed, n, zipf);
    let tb = std::time::Instant::now();
    let buckets = build(black_box(&sk));
    let build_ms = tb.elapsed().as_secs_f64() * 1000.0;
    let tp = std::time::Instant::now();
    let (cnt, chk) = probe(black_box(&buckets), black_box(&rk), black_box(&ra), black_box(&sb));
    let probe_ms = tp.elapsed().as_secs_f64() * 1000.0;
    let fold = cnt.wrapping_mul(1000000007).wrapping_add(chk);
    eprintln!("{:.3} {:.3}", build_ms, probe_ms);
    println!("count {}\nchecksum {}\nfold {}", cnt, chk, fold);
}
