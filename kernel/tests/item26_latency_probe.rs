//! Item 26 — direct per-operation latency probe (MEASUREMENT scaffold).
//!
//! Criterion reports sample-level central estimates (each sample averages ~hundreds
//! of ops), which smooths away the per-op tail. The batching-research pass needs a
//! TRUE per-append P50/P99, so this probe times every individual operation with
//! `Instant::now()` and computes exact percentiles from the sorted sample.
//!
//! Measurement-only, changes NO production behaviour. `#[ignore]` so it never runs
//! in normal CI; run explicitly:
//!   cargo test -p dowiz-kernel --release --test item26_latency_probe -- --ignored --nocapture
//!
//! ext4 block device in this env — the fsync latencies are real, not tmpfs no-ops.

use std::io::Write;
use std::time::{Duration, Instant};

use dowiz_kernel::decision::import::import_unit;
use dowiz_kernel::decision::{Decision, DecisionRegistry, DecisionUnitMeta, DomainTag, UnitEpoch};
use dowiz_kernel::decision::import::Source;
use dowiz_kernel::event_log::{sha3_256, EventLog, MemEventStore, MeshEvent};
use dowiz_kernel::fdr::ring::FdrRing;
use dowiz_kernel::fdr::schema::{FdrEvent, Kind, StampPolicy};
use dowiz_kernel::fdr::Level;
use dowiz_kernel::hydra::FileEventStore;

fn scratch(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("dowiz-item26-probe-{tag}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&base);
    base
}

fn pct(sorted_ns: &[u64], p: f64) -> u64 {
    if sorted_ns.is_empty() {
        return 0;
    }
    let idx = ((p / 100.0) * (sorted_ns.len() as f64 - 1.0)).round() as usize;
    sorted_ns[idx.min(sorted_ns.len() - 1)]
}

fn report(label: &str, mut ns: Vec<u64>, total: Duration) {
    ns.sort_unstable();
    let n = ns.len();
    let sum: u128 = ns.iter().map(|&x| x as u128).sum();
    let mean = sum as f64 / n as f64;
    let ops_per_s = n as f64 / total.as_secs_f64();
    println!(
        "{label:<38} n={n:>7}  p50={:>10.3}us  p99={:>10.3}us  p99.9={:>10.3}us  max={:>10.3}us  mean={:>10.3}us  thru={:>12.1} ops/s",
        pct(&ns, 50.0) as f64 / 1000.0,
        pct(&ns, 99.0) as f64 / 1000.0,
        pct(&ns, 99.9) as f64 / 1000.0,
        *ns.last().unwrap() as f64 / 1000.0,
        mean / 1000.0,
        ops_per_s,
    );
}

// ── M1: event-log commit — one open+write+flush+fsync+close per event ──
#[test]
#[ignore]
fn m1_event_log_append_percentiles() {
    let dir = scratch("m1");
    let path = dir.join("eventlog.jsonl");
    let _ = std::fs::remove_file(&path);
    let mut log = EventLog::new(FileEventStore::open(&path).unwrap());
    const N: u64 = 3000;
    let mut lat = Vec::with_capacity(N as usize);
    let start = Instant::now();
    for seq in 1..=N {
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [0u8; 32],
            actor_seq: seq,
            payload: b"order-transition-payload-approx-32b".to_vec(),
        };
        let t = Instant::now();
        log.append(ev).unwrap();
        lat.push(t.elapsed().as_nanos() as u64);
    }
    let total = start.elapsed();
    report("M1 event_log append (fsync/event)", lat, total);
    let _ = std::fs::remove_dir_all(&dir);
}

// ── M1: raw fsync amortization — barrier latency + throughput at g=1/4/16/64 ──
#[test]
#[ignore]
fn m1_fsync_amortization_curve() {
    let dir = scratch("m1amort");
    let line = b"{\"prev\":0,\"seq\":0,\"payload\":\"order-transition-approx-32-bytes\"}\n";
    const RECORDS: u64 = 20_000;
    println!("--- M1 fsync amortization (bench-crate model of group-commit; {RECORDS} records) ---");
    for g in [1u64, 4, 16, 64] {
        let path = dir.join(format!("amort-{g}.log"));
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        let mut barrier = Vec::new();
        let start = Instant::now();
        for i in 0..RECORDS {
            f.write_all(line).unwrap();
            if (i + 1) % g == 0 {
                let t = Instant::now();
                f.sync_all().unwrap();
                barrier.push(t.elapsed().as_nanos() as u64);
            }
        }
        f.sync_all().unwrap();
        let total = start.elapsed();
        let per_record_us = total.as_secs_f64() * 1e6 / RECORDS as f64;
        let thru = RECORDS as f64 / total.as_secs_f64();
        barrier.sort_unstable();
        println!(
            "  g={g:<3} per_record={per_record_us:>8.3}us  thru={thru:>12.1} rec/s  fsync_barrier_p50={:>9.3}us p99={:>9.3}us  (fsyncs={})",
            pct(&barrier, 50.0) as f64 / 1000.0,
            pct(&barrier, 99.0) as f64 / 1000.0,
            barrier.len(),
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ── M2: FDR ring — normal (page cache) vs alarm (fsync) vs forced segment switch ──
#[test]
#[ignore]
fn m2_fdr_ring_percentiles() {
    let dir = scratch("m2");

    // normal Kind::Event, Cheap stamp — write(2) to page cache, no fsync
    {
        let mut ring = FdrRing::open(dir.join("normal"), 1 << 20).unwrap();
        const N: usize = 200_000;
        let mut lat = Vec::with_capacity(N);
        let start = Instant::now();
        for _ in 0..N {
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(seq, Level::Info, Kind::Event, "n".into(), StampPolicy::Cheap, vec![("k", "v".to_string())]);
            let t = Instant::now();
            ring.append(&ev).unwrap();
            lat.push(t.elapsed().as_nanos() as u64);
        }
        report("M2 fdr normal (page cache, no fsync)", lat, start.elapsed());
    }
    // alarm Kind::Alarm, Cheap stamp — write(2) + sync_data per record
    {
        let mut ring = FdrRing::open(dir.join("alarm"), 1 << 20).unwrap();
        const N: usize = 3000;
        let mut lat = Vec::with_capacity(N);
        let start = Instant::now();
        for _ in 0..N {
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(seq, Level::Error, Kind::Alarm, "a".into(), StampPolicy::Cheap, vec![("k", "v".to_string())]);
            let t = Instant::now();
            ring.append(&ev).unwrap();
            lat.push(t.elapsed().as_nanos() as u64);
        }
        report("M2 fdr alarm (sync_data/record)", lat, start.elapsed());
    }
    // forced segment switch — tiny cap so nearly every append switches (fsync + reopen/truncate)
    {
        let mut ring = FdrRing::open(dir.join("switch"), 64).unwrap();
        const N: usize = 2000;
        let mut lat = Vec::with_capacity(N);
        let start = Instant::now();
        for _ in 0..N {
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(seq, Level::Info, Kind::Event, "s".into(), StampPolicy::Cheap, vec![]);
            let t = Instant::now();
            ring.append(&ev).unwrap();
            lat.push(t.elapsed().as_nanos() as u64);
        }
        report("M2 fdr forced segment-switch", lat, start.elapsed());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ── M3: decision-unit import — per-unit CPU cost at replay-set N=8 (no IO) ──
#[test]
#[ignore]
fn m3_import_unit_percentiles() {
    let artifact = b"dispatch-v1-benchmark-artifact-bytes-for-item-26".as_slice();
    let ish = [7u8; 32];
    for n_cases in [1usize, 8, 64] {
        let cases: Vec<(u8, Decision<u8>)> = (0..n_cases).map(|i| (i as u8, Decision::Answer(i as u8))).collect();
        let mut meta = DecisionUnitMeta::new(DomainTag::Harness, UnitEpoch(1));
        meta.content_id = sha3_256(artifact);
        meta.instance_set_hash = ish;
        let reg = DecisionRegistry::new();
        let mut log = EventLog::new(MemEventStore::default());
        const N: usize = 200_000;
        let mut lat = Vec::with_capacity(N);
        let start = Instant::now();
        for _ in 0..N {
            let t = Instant::now();
            let res = import_unit(meta.clone(), artifact, |x| Decision::Answer(*x), &cases, ish, Source::Local, &reg, &mut log);
            lat.push(t.elapsed().as_nanos() as u64);
            assert!(res.is_ok());
        }
        report(&format!("M3 import_unit N={n_cases}"), lat, start.elapsed());
    }
}
