//! R-GRAPH §1.3/§1.4: the `Store::from_bytes` probe that settles the 80x disagreement.
//!
//!   R  (2026-09-28 §4):  31.1 ms for a 2,859,472 B evlog image   (~10.9 us/KB)
//!   BN (2026-10-01 §2.8): 70 us for 538,000 B                   (~0.13 us/KB)
//!
//! ONE process, both sizes, black_box, R=11, and each candidate explanation as its own row:
//!   copy        -- an image whose capacity equals its length (nothing to pad): the copy alone
//!   pad-to-cap  -- a TRIMMED image of a store created larger: `from_bytes` re-creates the
//!                  zero tail with `resize`, i.e. it writes (and page-faults) the whole capacity
//!   memcpy-*    -- the floor: one memcpy into a fresh Vec (page faults included) and into a
//!                  reused buffer (no faults)
//!
//! `#[ignore]` because it is a measurement, not a check; run it with
//!   cargo test --release --test from_bytes_probe -- --ignored --nocapture
//! and once without `--release` for the debug-build row. The assertions only pin that every
//! row loaded the store the probe meant to load (a probe that measures nothing must fail).

use bebop_store::evlog::{EvLog, Record};
use bebop_store::Store;
use std::hint::black_box;
use std::time::Instant;

const R: usize = 11;

/// Median and min of R timed runs of `f`, in nanoseconds.
fn time<F: FnMut()>(mut f: F) -> (u128, u128) {
    f(); // one untimed warm-up so the first row is not paying for code paging
    let mut v: Vec<u128> = (0..R)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_nanos()
        })
        .collect();
    v.sort_unstable();
    (v[R / 2], v[0])
}

fn row(label: &str, bytes: usize, (med, min): (u128, u128)) {
    let kb = bytes as f64 / 1024.0;
    println!(
        "PROBE {label:<34} bytes={bytes:>9} median={:>10} ns  min={:>10} ns  median_ns_per_KB={:>9.1}",
        med,
        min,
        med as f64 / kb
    );
}

/// An evlog image of `events` records with `payload`-byte bodies -- the shape R §4 used
/// (5,400 events x 330 B = one venue's 30-day hot log). Returns (store, trimmed image).
fn evlog_store(capacity: usize, events: usize, payload: usize) -> (Store, Vec<u8>) {
    let mut st = Store::create_bytes(capacity).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    for i in 0..events {
        let mut id = [0u8; 32];
        id[..8].copy_from_slice(&(i as u64 + 1).to_le_bytes());
        let rec = Record {
            id,
            prev: [0u8; 32],
            actor_pubkey: [0xAA; 32],
            actor_seq: i as u64,
            payload: vec![(i % 251) as u8; payload],
        };
        EvLog::append_tip_bytes(&mut st, &rec).unwrap();
    }
    let trimmed = st.to_bytes_trimmed();
    (st, trimmed)
}

/// A store whose whole capacity is exactly `bytes`, so `from_bytes` has nothing to pad.
fn exact_image(bytes: usize) -> Vec<u8> {
    Store::create_bytes(bytes).unwrap().to_bytes()
}

fn probe_size(name: &str, image: &[u8], expect_cells: usize) {
    let bytes = image.len();
    // The probe must load what it says it loads: same cell count every time.
    assert_eq!(Store::from_bytes(image).cells.len(), expect_cells, "{name}: wrong store loaded");
    row(&format!("{name} from_bytes"), bytes, time(|| {
        let st = Store::from_bytes(black_box(image));
        black_box(st.cells.len());
    }));
}

#[test]
#[ignore]
fn from_bytes_probe() {
    println!("PROBE build={}", if cfg!(debug_assertions) { "debug" } else { "release" });

    // BN's size: the live catalogue image, 538,000 B.
    let cat = exact_image(538_000);
    probe_size("538KB copy (cap==len)", &cat, 538_000 / 8);

    // R's size: a real evlog, 5,400 x 330 B, in a store sized to fit it exactly.
    let (big, _) = evlog_store(4 << 20, 5_400, 330);
    let used = big.pick().unwrap().arena_used as usize;
    let log_exact = exact_image_from(&big, used);
    println!("PROBE evlog arena_used_cells={used} image_bytes={}", log_exact.len());
    probe_size("2.9MB evlog copy (cap==len)", &log_exact, used);

    // The same evlog as a TRIMMED image of a store created at 4 MB and at 32 MB: the reader
    // re-creates the zero tail up to capacity, which is a write of every padded page.
    for cap_mb in [4usize, 32] {
        let (_, trimmed) = evlog_store(cap_mb << 20, 5_400, 330);
        probe_size(&format!("2.9MB evlog trimmed, pad to {cap_mb}MB"), &trimmed, (cap_mb << 20) / 8);
    }
    let (_, trimmed_cat) = evlog_store(32 << 20, 1_000, 330);
    probe_size("~0.5MB evlog trimmed, pad to 32MB", &trimmed_cat, (32 << 20) / 8);

    // The floor: one memcpy, into a fresh allocation (page faults) and into a reused one.
    for (name, img) in [("538KB", &cat), ("2.9MB", &log_exact)] {
        row(&format!("{name} memcpy fresh Vec"), img.len(), time(|| {
            let v = black_box(img.as_slice()).to_vec();
            black_box(v.len());
        }));
        let mut buf = vec![0u8; img.len()];
        row(&format!("{name} memcpy reused buf"), img.len(), time(|| {
            buf.copy_from_slice(black_box(img.as_slice()));
            black_box(buf[img.len() - 1]);
        }));
    }
}

/// The first `cells` cells of `st` as an image whose capacity cell is rewritten to say
/// exactly that, so nothing is padded on load.
fn exact_image_from(st: &Store, cells: usize) -> Vec<u8> {
    let mut fresh = Store::create_bytes(cells * 8).unwrap();
    let n = fresh.cells.len();
    // Copy the arena (records) but keep the fresh store's superblocks, then point its
    // arena cursor at the copied end: from_bytes reads only superblocks + length.
    fresh.cells[1024..n.min(cells)].copy_from_slice(&st.cells[1024..n.min(cells)]);
    fresh.to_bytes()
}
