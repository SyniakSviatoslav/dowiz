// Reference kernels, rustc --release. Identical algorithms to kernels/*.bp
// and c/kernels.c. Prints per-run wall times in ns for aggregation.
use std::env;
use std::time::Instant;

fn k1() -> i64 {
    let nseed: i64 = std::hint::black_box(1_000_000);
    let mut s: i64 = 0;
    let mut i: i64 = nseed;
    while i > 0 { s = black_box(s + i); i -= 1; }
    s
}

fn fib(n: i64) -> i64 {
    if n < 2 { n } else { fib(n - 1) + fib(n - 2) }
}

fn k2() -> i64 { fib(25) }

fn k3() -> i64 {
    let nseed: i64 = std::hint::black_box(300);
    let mut a: i64 = 0;
    let mut x: i64 = nseed;
    while x > 0 {
        let mut y: i64 = nseed;
        while y > 0 { a = black_box(a + x * 2 + y * 3); y -= 1; }
        x -= 1;
    }
    a
}

fn k4() -> i64 {
    let mut v: i64 = 1;
    let mut i: i64 = 2_000_000;
    while i > 0 { v = (v + i * 7) * 3 - 11; i -= 1; }
    v
}

fn black_box(v: i64) -> i64 {
    std::hint::black_box(v)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 { eprintln!("usage: kernels <kernel> <iters>"); std::process::exit(2); }
    let which: u32 = args[1].parse().unwrap();
    let mut iters: usize = args[2].parse().unwrap();
    if iters > 4096 { iters = 4096; }
    let f: fn() -> i64 = match which { 1 => k1, 2 => k2, 3 => k3, _ => k4 };
    let warm = black_box(f());
    let mut runs = [0u128; 4096];
    for r in runs.iter_mut().take(iters) {
        let t0 = Instant::now();
        let v = black_box(f());
        *r = t0.elapsed().as_nanos() + (v & 0) as u128;
    }
    println!("result={}", black_box(f()) + warm * 0);
    print!("ns");
    for r in runs.iter().take(iters) { print!(" {}", r); }
    println!();
}
