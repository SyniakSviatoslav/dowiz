// R2 Peak RSS — Rust equivalent: NTT n=4096 + hypervector workload.
// Measures max RSS via raw FFI to getrusage (no external crates).
// Compile: rustc -O -o bench file.rs

use std::alloc::{alloc, dealloc, Layout};
use std::hint::black_box;
use std::time::Instant;

const NTT_MOD: u64 = 998244353;
const NTT_ROOT: u64 = 3;

fn mod_pow(mut base: u64, mut exp: u64, m: u64) -> u64 {
    let mut result = 1;
    base %= m;
    while exp > 0 {
        if exp & 1 != 0 {
            result = (result * base) % m;
        }
        base = (base * base) % m;
        exp >>= 1;
    }
    result
}

fn ntt_transform(a: &mut [u64], n: usize, invert: bool) {
    let mut len = 2;
    while len <= n {
        let mut wlen = mod_pow(NTT_ROOT, (NTT_MOD - 1) / len as u64, NTT_MOD);
        if invert {
            wlen = mod_pow(wlen, NTT_MOD - 2, NTT_MOD);
        }
        let mut i = 0;
        while i < n {
            let mut w = 1u64;
            for j in 0..len / 2 {
                let u = a[i + j];
                let v = (a[i + j + len / 2] * w) % NTT_MOD;
                a[i + j] = (u + v) % NTT_MOD;
                a[i + j + len / 2] = (u + NTT_MOD - v) % NTT_MOD;
                w = (w * wlen) % NTT_MOD;
            }
            i += len;
        }
        len <<= 1;
    }
    if invert {
        let inv_n = mod_pow(n as u64, NTT_MOD - 2, NTT_MOD);
        for ai in a.iter_mut() {
            *ai = (*ai * inv_n) % NTT_MOD;
        }
    }
}

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

#[repr(align(64))]
#[derive(Copy, Clone)]
struct Hypervector {
    words: [u64; 16],
}

fn main() {
    let n: usize = 4096;
    let nhv: usize = 1024;
    let n_runs = 5;
    let mut best_rss: i64 = i64::MAX;
    let mut best_us: u128 = u128::MAX;

    for _run in 0..n_runs {
        let layout_u64 = Layout::array::<u64>(n).unwrap();
        let layout_c = Layout::array::<u64>(2 * n - 1).unwrap();
        let layout_hv = Layout::array::<Hypervector>(nhv).unwrap();

        let a_ptr = unsafe { alloc(layout_u64) };
        let b_ptr = unsafe { alloc(layout_u64) };
        let c_ptr = unsafe { alloc(layout_c) };
        let hv_ptr = unsafe { alloc(layout_hv) };

        if a_ptr.is_null() || b_ptr.is_null() || c_ptr.is_null() || hv_ptr.is_null() {
            eprintln!("alloc fail");
            return;
        }

        let a: &mut [u64] = unsafe { std::slice::from_raw_parts_mut(a_ptr as *mut u64, n) };
        let b: &mut [u64] = unsafe { std::slice::from_raw_parts_mut(b_ptr as *mut u64, n) };
        let c: &mut [u64] = unsafe { std::slice::from_raw_parts_mut(c_ptr as *mut u64, 2 * n - 1) };
        let hvs: &mut [Hypervector] =
            unsafe { std::slice::from_raw_parts_mut(hv_ptr as *mut Hypervector, nhv) };

        for i in 0..n {
            a[i] = (i as u64 * 2654435761) % 1000000;
            b[i] = (i as u64 * 7) % 1000000;
        }

        let t0 = Instant::now();
        for _rep in 0..10 {
            ntt_transform(a, n, false);
            ntt_transform(b, n, false);
            for i in 0..n {
                c[i] = (a[i] * b[i]) % NTT_MOD;
            }
            ntt_transform(c, n, true);

            for k in 0..nhv {
                let mut seed = (k + 1) as u64 * 0x9E3779B97F4A7C15u64;
                for w in 0..16 {
                    seed ^= seed >> 30;
                    seed = seed.wrapping_mul(0xBF58476D1CE4E5B9);
                    seed ^= seed >> 27;
                    seed = seed.wrapping_mul(0x94D049BB133111EB);
                    seed ^= seed >> 31;
                    hvs[k].words[w] = seed;
                }
            }
            let mut sink: u32 = 0;
            let mut k = 0;
            while k < nhv - 1 {
                let mut bound = Hypervector { words: [0; 16] };
                for w in 0..16 {
                    bound.words[w] = hvs[k].words[w] ^ hvs[k + 1].words[w];
                }
                let mut dist: u32 = 0;
                for w in 0..16 {
                    dist += bound.words[w].count_ones();
                }
                sink += dist;
                k += 2;
            }
            black_box(sink);
        }
        let elapsed = t0.elapsed().as_micros();
        let rss = get_peak_rss_kb();

        if rss < best_rss {
            best_rss = rss;
        }
        if elapsed < best_us {
            best_us = elapsed;
        }

        unsafe {
            dealloc(a_ptr, layout_u64);
            dealloc(b_ptr, layout_u64);
            dealloc(c_ptr, layout_c);
            dealloc(hv_ptr, layout_hv);
        }
    }

    println!("peak_rss_kb: {}", best_rss);
    println!("best_of_5_us: {}", best_us);
}