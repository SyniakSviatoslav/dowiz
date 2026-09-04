// Rust twin of run_sub (the SAME cell substrate algorithm: tables, LSB-first
// drain, branch-free 6-way op select, candidate/readiness scan). Gives the
// model's floor on this ISA independent of the bebop code generator.
use std::hint::black_box;
const N: i64 = 300000;
const TAB: [i64; 80] = [0,0,0,48,0,0,0,0,80,0,0,0,0,32800,0,0,0,0,2112,0,0,0,1,640,3,2,0,2,384,5,3,1,3,768,10,1,4,5,9216,48,4,5,6,17408,96,5,4,6,2048,80,2,7,8,4096,384,0,9,3,4096,520,3,10,11,8192,3072,1,12,7,16384,4224,0,13,8,32768,8448,2,14,2,0,16388];
#[inline(always)]
fn cell_eval(op: i64, x: i64, y: i64) -> i64 {
    ((op == 0) as i64).wrapping_mul(x.wrapping_add(y)) + ((op == 1) as i64).wrapping_mul(x.wrapping_sub(y)) + ((op == 2) as i64).wrapping_mul(x.wrapping_mul(y)) + ((op == 3) as i64) * (x ^ y) + ((op == 4) as i64) * (x & y) + ((op == 5) as i64) * (x | y)
}
fn sweep(val: &mut [i64; 16], st: &mut [i64; 4]) -> i64 {
    let mut act = st[0];
    while act != 0 {
        let lsb = act & act.wrapping_neg();
        let i = lsb.trailing_zeros() as usize;
        val[i] = cell_eval(TAB[i*5], val[TAB[i*5+1] as usize], val[TAB[i*5+2] as usize]);
        st[1] |= lsb; st[2] |= TAB[i*5+3];
        act -= lsb;
    }
    let mut cand = st[2]; let mut nxt = 0i64;
    while cand != 0 {
        let lsb = cand & cand.wrapping_neg();
        let d = lsb.trailing_zeros() as usize;
        nxt += (((TAB[d*5+4] & !st[1]) == 0) as i64) * lsb;
        cand -= lsb;
    }
    st[2] = 0; st[0] = nxt; nxt
}
fn main() {
    let n = black_box(N);
    let mut val = [0i64; 16]; val[2] = 3; val[3] = 7;
    let mut st = [0i64; 4];
    let t0 = std::time::Instant::now();
    let mut acc = 0i64; let mut sw = 0i64; let mut k = n;
    while k > 0 {
        val[0] = black_box(k); val[1] = k.wrapping_mul(3).wrapping_add(1);
        st[0] = 112; st[1] = 15; st[2] = 0;
        while st[0] != 0 { sweep(&mut val, &mut st); sw += 1; }
        acc = acc.wrapping_add(val[15]);
        k -= 1;
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!("{} {:.3} {}", black_box(acc), ms, sw);
}
