// join_merge.rs — B2 twin (i): 2-way join via sort-merge (sort both sides by k, merge runs of
// equal keys) (docs/blueprints/B2-decisive-twins.md §3(i)). Same generator as join_hash.rs
// (bit-identical to the bebop/python twins). argv: n, dist('u'|'z'). REPS (env, default 1),
// see join_hash.rs's header for why default 1 is enough for a folds-only build now.
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

fn join_fold_merge(rk: &[i64], ra: &[i64], sk: &[i64], sb: &[i64]) -> (i64, i64) {
    let mut ri: Vec<u32> = (0..rk.len() as u32).collect();
    ri.sort_by_key(|&i| rk[i as usize]);
    let mut si: Vec<u32> = (0..sk.len() as u32).collect();
    si.sort_by_key(|&i| sk[i as usize]);
    let mut cnt: i64 = 0;
    let mut chk: i64 = 0;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < ri.len() && j < si.len() {
        let kr = rk[ri[i] as usize];
        let ks = sk[si[j] as usize];
        if kr < ks {
            i += 1;
        } else if kr > ks {
            j += 1;
        } else {
            let mut i2 = i;
            while i2 < ri.len() && rk[ri[i2] as usize] == kr { i2 += 1; }
            let mut j2 = j;
            while j2 < si.len() && sk[si[j2] as usize] == ks { j2 += 1; }
            for a_idx in i..i2 {
                let a = ra[ri[a_idx] as usize];
                for b_idx in j..j2 {
                    let b = sb[si[b_idx] as usize];
                    let p = (a * b) % MOD61;
                    cnt = cnt.wrapping_add(1);
                    chk = chk.wrapping_add(p);
                }
            }
            i = i2;
            j = j2;
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
        let r = join_fold_merge(black_box(&rk), black_box(&ra), black_box(&sk), black_box(&sb));
        cnt = r.0;
        chk = r.1;
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    let fold = cnt.wrapping_mul(1000000007).wrapping_add(chk);
    eprintln!("{:.3}", ms);
    println!("count {}\nchecksum {}\nfold {}", cnt, chk, fold);
}
