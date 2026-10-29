//! THE REGRESSION GATE (T-ES-50). Four rows, all CPU-only over a warm page cache, each timed as
//! a RATIO to a same-process control doing the same logical work with plain std code, the two
//! interleaved inside every repetition so a slow moment of the machine lands on both:
//!
//!   get     bebop `get_many` (100k random gets, one read txn)  /  binary search on a sorted Vec
//!   prefix  bebop `prefix_many` (1000 prefixes x 100 keys)       /  partition_point on that Vec
//!   verify  bebop's verify-on-open over the image bytes (crc)    /  FNV-1a 64 over the same bytes
//!   build   bebop `image(entries)` (compacted_bytes_fit)         /  the entries concatenated
//!
//! The ratio, not the time, is compared to the committed baseline: it cancels the machine's clock
//! and most of its noise, but NOT its cache and branch-predictor shape, so a baseline is per
//! architecture and the gate refuses (exit 2) on one it has no baseline for. Write rows (B2/B3)
//! are report-only per §4.6: fsync time here is the box's, not the engine's.
//! Every repetition asserts bebop's checksum equals the control's: a row that reads nothing,
//! or reads something else, panics instead of getting faster.
use std::fs;
use std::hint::black_box;
use std::path::Path;

use crate::bebop::{self, Bebop};
use crate::data::*;
use crate::engine::Engine;

pub const N: u64 = 10_000;
pub const VS: usize = 256;
pub const ROWS: [&str; 4] = ["get", "prefix", "verify", "build"];

fn fnv(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &x| (h ^ x as u64).wrapping_mul(0x100_0000_01b3))
}

/// Median and p95 of bebop/control per row, `reps` interleaved repetitions each.
pub fn measure(dir: &Path, reps: usize) -> Vec<(&'static str, St)> {
    let e = entries(N, VS);
    let _ = fs::remove_dir_all(dir);
    let db = Bebop::load(dir, &e);
    let img = bebop::image(&e);
    let ks = lookups(N, 100_000, 7);
    let ps = prefixes(N, 1_000, 11);
    let mut out = vec![];
    for row in ROWS {
        let mut v = vec![];
        // Rep 0 is a warm-up and is not kept: the first `get` rep of the first run read 4.15
        // where the next four runs read 2.42-2.58 (2026-10-29, gate-record 15 5) -- page faults
        // on the fresh map and a cold cache, not bebop.
        for rep in 0..=reps {
            let (tc, ac, tb, ab);
            match row {
                "get" => {
                    let t0 = t();
                    ac = ks.iter().map(|k| acc_of(&e[e.binary_search_by(|(x, _)| x.cmp(k)).unwrap()].1)).fold(0, u64::wrapping_add);
                    tc = us(t0);
                    let t1 = t();
                    ab = db.get_many(&ks);
                    tb = us(t1);
                }
                "prefix" => {
                    let t0 = t();
                    let mut a = 0u64;
                    for p in &ps {
                        let lo = e.partition_point(|(x, _)| x.as_slice() < p.as_slice());
                        for (k, v) in e[lo..].iter().take_while(|(x, _)| x.starts_with(p)) {
                            a = a.wrapping_add(v.len() as u64 + k[14] as u64);
                        }
                    }
                    ac = a;
                    tc = us(t0);
                    let t1 = t();
                    ab = db.prefix_many(&ps);
                    tb = us(t1);
                }
                "verify" => {
                    let t0 = t();
                    ac = (0..4).map(|_| { black_box(fnv(black_box(&img))); 1u64 }).sum();
                    tc = us(t0);
                    let t1 = t();
                    ab = (0..4).map(|_| bebop::verify_bytes(&img) as u64).sum();
                    tb = us(t1);
                }
                _ => {
                    let t0 = t();
                    let mut cat = Vec::with_capacity(img.len());
                    for (k, v) in &e {
                        cat.extend_from_slice(&(k.len() as u32).to_le_bytes());
                        cat.extend_from_slice(k);
                        cat.extend_from_slice(v);
                    }
                    ac = (black_box(&cat).len() > 0) as u64;
                    tc = us(t0);
                    let t1 = t();
                    ab = (black_box(bebop::image(&e)).len() > 0) as u64;
                    tb = us(t1);
                }
            }
            assert_eq!(ab, ac, "gate row {row}: bebop read something other than the control");
            assert!(tc > 50.0, "gate row {row}: control took {tc:.1} us, too short to time");
            if rep > 0 {
                v.push(tb / tc);
            }
        }
        out.push((row, stats(v)));
    }
    drop(db);
    let _ = fs::remove_dir_all(dir);
    out
}

/// `key=value` lines; `#` comments.
pub fn read_baseline(p: &Path) -> Option<Vec<(String, f64)>> {
    let s = fs::read_to_string(p).ok()?;
    Some(s.lines().filter(|l| !l.starts_with('#') && l.contains('=')).filter_map(|l| {
        let (k, v) = l.split_once('=')?;
        Some((k.trim().to_string(), v.trim().parse().ok()?))
    }).collect())
}

/// 0 green, 1 a row regressed past the tolerance, 2 no usable baseline.
pub fn check(base: &Path, dir: &Path, reps: usize) -> i32 {
    let Some(b) = read_baseline(base) else {
        println!("es-bench gate: REFUSED -- no baseline at {} (record one with `es-bench gate-record`)", base.display());
        return 2;
    };
    let get = |k: &str| b.iter().find(|(x, _)| x == k).map(|(_, v)| *v);
    if get("n") != Some(N as f64) || get("vs") != Some(VS as f64) {
        println!("es-bench gate: REFUSED -- baseline is for another data set (n/vs), this binary measures n={N} vs={VS}");
        return 2;
    }
    let Some(tol) = get("tol_pct") else {
        println!("es-bench gate: REFUSED -- baseline names no tol_pct");
        return 2;
    };
    let mut rc = 0;
    for (row, s) in measure(dir, reps) {
        let Some(bv) = get(row) else {
            println!("es-bench gate: REFUSED -- baseline has no row {row}");
            return 2;
        };
        let lim = bv * (1.0 + tol / 100.0);
        let verdict = if s.med > lim { rc = 1; "RED" } else { "ok" };
        println!("GATE|{row}|ratio med={:.3} p95={:.3} min={:.3} reps={}|baseline={bv:.3}|limit={lim:.3} (+{tol}%)|{verdict}", s.med, s.p95, s.min, s.n);
        if s.med < bv * (1.0 - tol / 100.0) {
            println!("es-bench gate: {row} is {:.0}% FASTER than its baseline -- lower it in this commit (gate-record)", 100.0 * (1.0 - s.med / bv));
        }
    }
    println!("es-bench gate: {}", if rc == 0 { "GREEN" } else { "RED -- bebop regressed past the tolerance on a row above" });
    rc
}

/// Print a baseline file from `runs` independent measurements: per row the WORST median seen.
pub fn record(dir: &Path, reps: usize, runs: usize, tol: f64) {
    let all: Vec<_> = (0..runs).map(|_| measure(dir, reps)).collect();
    println!("# es-bench gate baseline: ratio bebop/control, median of {reps} interleaved reps, worst of {runs} runs.");
    println!("# arch={} recorded by `es-bench gate-record` -- see docs/measurements/ES-BENCH-2026-10-29.md", std::env::consts::ARCH);
    for (i, row) in ROWS.iter().enumerate() {
        let meds: Vec<String> = all.iter().map(|r| format!("{:.3}", r[i].1.med)).collect();
        println!("# {row}: run medians {}", meds.join(" "));
    }
    println!("n={N}\nvs={VS}\ntol_pct={tol}");
    for (i, row) in ROWS.iter().enumerate() {
        let worst = all.iter().map(|r| r[i].1.med).fold(0.0f64, f64::max);
        println!("{row}={worst:.3}");
    }
}
