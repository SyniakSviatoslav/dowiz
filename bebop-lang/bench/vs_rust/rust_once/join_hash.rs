// join_hash.rs — B2 twin (i): 2-way join via std::collections::HashMap<i64, Vec<u32>>
// (docs/blueprints/B2-decisive-twins.md §3(i)). Generator is bit-for-bit identical to
// bench/vs_rust/std_tests/join_twin.bp's gen() / bench/oracles/join_twin.py's gen(): one
// continuous LCG stream, R's n rows then S's n rows, 3 draws/row regardless of distribution
// (key, coin, payload), zipf = 1% of keys (n/100) take 30% of the rows. argv: n, dist('u'|'z').
// REPS (env, default 1) reruns the whole join in-process (honest.sh convention: black_box on
// carried state) for a later finer-grained timing row; default 1 keeps this a folds-only
// build now — no timing is claimed from this file yet (register-model bebop.bin not landed).
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

fn join_fold(rk: &[i64], ra: &[i64], sk: &[i64], sb: &[i64]) -> (i64, i64) {
    let mut buckets: HashMap<i64, Vec<u32>> = HashMap::with_capacity(sk.len());
    for (s, &k) in sk.iter().enumerate() {
        buckets.entry(k).or_default().push(s as u32);
    }
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
    let reps: usize = std::env::var("REPS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let seed: u64 = if zipf { 8823 } else { 4711 };
    let (rk, ra, sk, sb) = gen(seed, n, zipf);
    let t0 = std::time::Instant::now();
    let mut cnt = 0i64;
    let mut chk = 0i64;
    for _ in 0..reps {
        let r = join_fold(black_box(&rk), black_box(&ra), black_box(&sk), black_box(&sb));
        cnt = r.0;
        chk = r.1;
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    let fold = cnt.wrapping_mul(1000000007).wrapping_add(chk);
    eprintln!("{:.3}", ms);
    println!("count {}\nchecksum {}\nfold {}", cnt, chk, fold);
}
