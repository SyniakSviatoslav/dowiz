// R2 Allocation Rate — Rust equivalent: alloc/dealloc 1KB blocks, count ops/sec.
// Uses raw FFI to getrusage for peak RSS (no external crates).
// Compile: rustc -O -o bench file.rs

use std::alloc::{alloc, dealloc, Layout};
use std::hint::black_box;
use std::time::Instant;

const ALLOC_SIZE: usize = 1024;
const N_ITERS: usize = 500_000;

#[repr(C)]
struct Timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
struct Rusage {
    ru_utime: Timeval,
    ru_stime: Timeval,
    ru_maxrss: i64,
    ru_ixrss: i64,
    ru_idrss: i64,
    ru_isrss: i64,
    ru_minflt: i64,
    ru_majflt: i64,
    ru_nswap: i64,
    ru_inblock: i64,
    ru_oublock: i64,
    ru_msgsnd: i64,
    ru_msgrcv: i64,
    ru_nsignals: i64,
    ru_nvcsw: i64,
    ru_nivcsw: i64,
}

const RUSAGE_SELF: i32 = 0;

extern "C" {
    fn getrusage(who: i32, usage: *mut Rusage) -> i32;
}

fn get_peak_rss_kb() -> i64 {
    let mut ru: Rusage = unsafe { std::mem::zeroed() };
    unsafe { getrusage(RUSAGE_SELF, &mut ru) };
    ru.ru_maxrss
}

fn main() {
    let layout = Layout::new::<[u8; ALLOC_SIZE]>();
    let n_runs = 5;
    let mut best_rate: f64 = 0.0;

    for _run in 0..n_runs {
        let t0 = Instant::now();
        let mut acc: u64 = 0;
        for _ in 0..N_ITERS {
            let ptr = unsafe { alloc(layout) };
            if ptr.is_null() {
                eprintln!("alloc fail");
                return;
            }
            unsafe {
                std::ptr::write_bytes(ptr, 0xAB, ALLOC_SIZE);
                // read the written bytes through a black_box so LLVM cannot
                // dead-store-eliminate the write (previous version measured
                // malloc+free only, an upper bound, not real alloc+write+free).
                let b0 = std::ptr::read_volatile(ptr as *const u8);
                let b1 = std::ptr::read_volatile(ptr.add(ALLOC_SIZE - 1) as *const u8);
                acc = acc.wrapping_add(black_box(b0 as u64) ^ black_box(b1 as u64));
                dealloc(ptr, layout);
            }
        }
        black_box(acc);
        let elapsed_s = t0.elapsed().as_secs_f64();
        let rate = N_ITERS as f64 / elapsed_s;
        if rate > best_rate {
            best_rate = rate;
        }
    }

    let rss = get_peak_rss_kb();
    println!("alloc_rate_per_sec: {:.0}", best_rate);
    println!("peak_rss_kb: {}", rss);
}