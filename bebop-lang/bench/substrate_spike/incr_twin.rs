// T107 Rust twin of incr.bp: same grid DAG, same LCG, same op, same fold, same
// sweep engine (dirty bitset, LSB drain, mark real dependents). Usage:
//   incr_twin <k> <s|f>   -> prints "us=<per rep> fold=<fold>"
use std::time::Instant;
const N: usize = 65536; const W: usize = 4096; const REP: usize = 64;
#[inline(always)] fn op(x: i64, y: i64) -> i64 { (x.wrapping_mul(31).wrapping_add(y.wrapping_mul(17))) ^ (x >> 3) }
fn full(s1: &[u32], s2: &[u32], val: &mut [i64]) { for i in W..N { val[i] = op(val[s1[i] as usize], val[s2[i] as usize]); } }
fn mark(d: &mut [u64], s1: &[u32], s2: &[u32], j: usize, i: usize) {
    for t in j - 1..=j + 1 { if s1[t] as usize == i || s2[t] as usize == i { d[t >> 6] |= 1u64 << (t & 63); } }
}
fn sweep(d: &mut [u64], s1: &[u32], s2: &[u32], val: &mut [i64], fired: &mut u64) {
    for w in 0..N / 64 {
        while d[w] != 0 {
            let x = d[w]; let low = x & x.wrapping_neg(); d[w] = x - low;
            let i = w * 64 + low.trailing_zeros() as usize; *fired += 1;
            if i >= W { val[i] = op(val[s1[i] as usize], val[s2[i] as usize]); }
            let c = i & (W - 1); let j = i + W;
            let j0 = if c == 0 { j + 1 } else if c == W - 1 { j - 1 } else { j };
            if i < N - W { mark(d, s1, s2, j0, i); }
        }
    }
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let k: usize = a[1].parse().unwrap(); let is_sweep = a[2].starts_with('s');
    let mut s1 = vec![0u32; N]; let mut s2 = vec![0u32; N]; let mut val = vec![0i64; N];
    for c in 0..W { val[c] = (c as i64).wrapping_mul(2654435761); }
    let mut g: i64 = 88172645463325252;
    for i in W..N {
        g = g.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let l = i >> 12; let c = (i & (W - 1)) as i64;
        let ai = c + (((g as u64) >> 33) % 3) as i64 - 1; let bi = c + (((g as u64) >> 40) % 3) as i64 - 1; // bebop `>>` is LSR
        let ai = ai.clamp(0, 4095); let bi = bi.clamp(0, 4095);
        s1[i] = ((l - 1) * W) as u32 + ai as u32; s2[i] = ((l - 1) * W) as u32 + bi as u32;
    }
    full(&s1, &s2, &mut val);
    let mut d = vec![0u64; N / 64]; let mut fired = 0u64;
    let t0 = Instant::now();
    let mut g: i64 = 1;
    for rep in 0..REP {
        for _ in 0..k {
            g = g.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let c = ((g as u64 >> 20) & 4095) as usize;
            val[c] = val[c].wrapping_add(12345 + rep as i64);
            if is_sweep { d[c >> 6] |= 1u64 << (c & 63); }
        }
        if is_sweep { sweep(&mut d, &s1, &s2, &mut val, &mut fired); } else { full(&s1, &s2, &mut val); }
    }
    let us = t0.elapsed().as_micros() as u64 / REP as u64;
    let mut f: i64 = 0; for i in 0..N { f = f.wrapping_add(val[i] ^ i as i64); }
    println!("us={} fold={} fired={}", us, f, fired);
}
