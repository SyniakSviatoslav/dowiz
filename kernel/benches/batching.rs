//! Item 26 (space-grade exec roadmap §C) — batching MEASUREMENT scaffold.
//!
//! Measurement-only. This bench crate exercises three existing kernel paths to
//! establish the baselines the batching-research pass requires. It adds NO
//! production code and changes NO kernel behaviour — every symbol it touches is
//! the public API exactly as it ships.
//!
//!   M1 — event-log commit (`EventLog<FileEventStore>::append`): per-append
//!        wall-clock + appends/sec (one open+write+flush+fsync+close per event),
//!        plus a bench-crate-only raw-fsync amortization curve (sync every
//!        1/4/16/64 records) that models a hypothetical group-commit WITHOUT
//!        touching the kernel.
//!   M2 — FDR ring (`FdrRing::append`): normal page-cache path (Kind::Event) vs
//!        Alarm path (per-record sync_data), segment-switch cost, and a segment
//!        cap sweep 256 KiB / 1 MiB / 4 MiB.
//!   M3 — decision-unit import (`import_unit`): per-unit CPU cost swept over the
//!        replay-set size N ∈ {1,8,64}. Hypothesis under test: per-unit cost is
//!        CPU-trivial and batching would buy nothing.
//!
//! Run: `cargo bench -p dowiz-kernel --bench batching`
//! P99 is reported by criterion's own high-percentile estimate; group runtimes
//! are bounded (small sample size) because the fsync paths do real block IO.

use std::time::Duration;

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};

use dowiz_kernel::decision::{
    Decision, DecisionRegistry, DecisionUnitMeta, DomainTag, UnitEpoch,
};
use dowiz_kernel::event_log::{sha3_256, EventLog, MemEventStore, MeshEvent};
use dowiz_kernel::fdr::ring::FdrRing;
use dowiz_kernel::fdr::schema::{FdrEvent, Kind, StampPolicy};
use dowiz_kernel::fdr::Level;
use dowiz_kernel::hydra::FileEventStore;

/// A unique scratch dir per bench process (block-backed ext4 in this env — the
/// fsync numbers are real, not tmpfs no-ops).
fn scratch(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "dowiz-item26-{}-{}",
        tag,
        std::process::id()
    ));
    let _ = std::fs::create_dir_all(&base);
    base
}

// ─────────────────────────── M1: event-log commit ───────────────────────────

fn m1_event_log_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("m1_event_log");
    // fsync-bound: keep the sample count modest so the group finishes in seconds.
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_millis(500));
    group.throughput(Throughput::Elements(1));

    let dir = scratch("m1");
    let path = dir.join("eventlog.jsonl");
    let _ = std::fs::remove_file(&path);
    let store = FileEventStore::open(&path).expect("open FileEventStore");
    let mut log = EventLog::new(store);
    let mut seq: u64 = 0;

    // Each append = open(append) + write_all + flush + sync_all + close, and the
    // in-memory index advances only after the fsync succeeds. actor_seq is bumped
    // per iteration so every event has a distinct content-id (a real commit, never
    // an idempotent Duplicate no-op).
    group.bench_function("append_fsync_per_event", |b| {
        b.iter(|| {
            seq += 1;
            let ev = MeshEvent {
                prev: [0u8; 32],
                actor_pubkey: [0u8; 32],
                actor_seq: seq,
                payload: b"order-transition-payload-approx-32b".to_vec(),
            };
            black_box(log.append(black_box(ev)).expect("append"));
        })
    });
    group.finish();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Bench-crate-only model of group-commit: append `RECORDS` fixed-size lines to
/// one file, calling fsync every `group_sz` records. Throughput is records/sec;
/// dividing the per-batch wall-clock by `group_sz` gives the amortized per-record
/// cost at that grouping level. NO kernel code is involved — this measures the
/// raw fsync-amortization ceiling a future group-commit could approach.
fn m1_fsync_amortization(c: &mut Criterion) {
    use std::io::Write;

    const RECORDS: u64 = 256;
    let line = b"{\"prev\":0,\"seq\":0,\"payload\":\"order-transition-approx-32-bytes\"}\n";

    let mut group = c.benchmark_group("m1_fsync_amortization");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_millis(500));
    group.throughput(Throughput::Elements(RECORDS));

    let dir = scratch("m1amort");

    for group_sz in [1u64, 4, 16, 64] {
        group.bench_with_input(
            BenchmarkId::from_parameter(group_sz),
            &group_sz,
            |b, &g| {
                let path = dir.join(format!("amort-{g}.log"));
                b.iter(|| {
                    let mut f = std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(&path)
                        .expect("open");
                    for i in 0..RECORDS {
                        f.write_all(line).expect("write");
                        if (i + 1) % g == 0 {
                            f.sync_all().expect("fsync");
                        }
                    }
                    // final barrier so every record is durable regardless of g
                    f.sync_all().expect("fsync-tail");
                    black_box(&f);
                })
            },
        );
    }
    group.finish();
    let _ = std::fs::remove_dir_all(&dir);
}

// ─────────────────────────────── M2: FDR ring ───────────────────────────────

fn m2_fdr_ring(c: &mut Criterion) {
    let mut group = c.benchmark_group("m2_fdr_ring");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_millis(500));
    group.throughput(Throughput::Elements(1));

    let dir = scratch("m2");

    // Normal path: Kind::Event, Cheap stamp → write(2) to page cache, NO fsync.
    group.bench_function("append_normal_event", |b| {
        let mut ring = FdrRing::open(dir.join("normal"), 1 << 20).expect("open ring");
        b.iter(|| {
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(
                seq,
                Level::Info,
                Kind::Event,
                "bench_event".into(),
                StampPolicy::Cheap,
                vec![("k", "v".to_string())],
            );
            black_box(ring.append(black_box(&ev)).expect("append"));
        })
    });

    // Alarm path: Kind::Alarm, Cheap stamp → write(2) + sync_data per record.
    // Stamp policy held Cheap so this isolates the fsync cost (not the /proc read).
    group.bench_function("append_alarm_fsync", |b| {
        let mut ring = FdrRing::open(dir.join("alarm"), 1 << 20).expect("open ring");
        b.iter(|| {
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(
                seq,
                Level::Error,
                Kind::Alarm,
                "bench_alarm".into(),
                StampPolicy::Cheap,
                vec![("k", "v".to_string())],
            );
            black_box(ring.append(black_box(&ev)).expect("append"));
        })
    });

    // Segment-switch cost in isolation: a tiny cap forces switch() (fsync + reopen
    // + truncate the other segment) on nearly every append.
    group.bench_function("append_forces_segment_switch", |b| {
        // cap small enough that a single record overflows it every time.
        let mut ring = FdrRing::open(dir.join("switch"), 64).expect("open ring");
        b.iter(|| {
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(
                seq,
                Level::Info,
                Kind::Event,
                "bench_switch".into(),
                StampPolicy::Cheap,
                vec![],
            );
            black_box(ring.append(black_box(&ev)).expect("append"));
        })
    });
    group.finish();

    // Segment-cap sweep: throughput of a burst of normal (non-fsync) records under
    // 256 KiB / 1 MiB / 4 MiB caps — learns whether the cap is throughput-relevant
    // at all (it only changes how often switch()'s fsync fires).
    const BURST: u64 = 4096;
    let mut sweep = c.benchmark_group("m2_seg_cap_sweep");
    sweep.sample_size(20);
    sweep.measurement_time(Duration::from_secs(5));
    sweep.warm_up_time(Duration::from_millis(500));
    sweep.throughput(Throughput::Elements(BURST));
    for cap in [256u64 * 1024, 1024 * 1024, 4 * 1024 * 1024] {
        sweep.bench_with_input(
            BenchmarkId::from_parameter(cap),
            &cap,
            |b, &cap| {
                b.iter(|| {
                    let mut ring =
                        FdrRing::open(dir.join(format!("sweep-{cap}")), cap).expect("open");
                    for i in 0..BURST {
                        let seq = ring.next_seq();
                        let _ = i;
                        let ev = FdrEvent::stamp(
                            seq,
                            Level::Info,
                            Kind::Event,
                            "sweep".into(),
                            StampPolicy::Cheap,
                            vec![],
                        );
                        black_box(ring.append(&ev).expect("append"));
                    }
                    black_box(&ring);
                })
            },
        );
    }
    sweep.finish();
    let _ = std::fs::remove_dir_all(&dir);
}

// ──────────────────────────── M3: decision import ───────────────────────────

fn m3_import_unit(c: &mut Criterion) {
    let mut group = c.benchmark_group("m3_import_unit");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(4));

    let artifact = b"dispatch-v1-benchmark-artifact-bytes-for-item-26".as_slice();
    let ish = [7u8; 32];

    for n_cases in [1usize, 8, 64] {
        // Identity replay set: proc(x)=x, expected==input (no u8 overflow at N=64).
        let cases: Vec<(u8, Decision<u8>)> =
            (0..n_cases).map(|i| (i as u8, Decision::Answer(i as u8))).collect();

        let mut meta = DecisionUnitMeta::new(DomainTag::Harness, UnitEpoch(1));
        meta.content_id = sha3_256(artifact);
        meta.instance_set_hash = ish;

        let reg = DecisionRegistry::new();
        // Empty registry ⇒ route_live is None ⇒ every import reaches admit+append.
        let mut log = EventLog::new(MemEventStore::default());
        // Prime once so the lineage row exists; subsequent appends are idempotent
        // Duplicate no-ops (contains-lookup cost only) — realistic re-import shape.

        group.bench_with_input(
            BenchmarkId::from_parameter(n_cases),
            &n_cases,
            |b, _| {
                b.iter(|| {
                    let res = dowiz_kernel::decision::import::import_unit(
                        meta.clone(),
                        artifact,
                        |x| Decision::Answer(*x),
                        &cases,
                        ish,
                        &reg,
                        &mut log,
                    );
                    black_box(res.is_ok())
                })
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    m1_event_log_append,
    m1_fsync_amortization,
    m2_fdr_ring,
    m3_import_unit
);
criterion_main!(benches);
