//! `fdr_recorder` — FDR durable-ring test/ops tool (blueprint §4.4 kill-9 proof harness).
//!
//! Dual mode, argv-selected (links `dowiz_kernel` directly, pure std, no interpreter):
//!   * `fdr_recorder write <dir> <n> <ready_file>` — open a fresh ring in <dir>, write <n>
//!     event records (NO fsync — proving kill-9 survives on page-cache writes alone), touch
//!     <ready_file> as a barrier, then block forever waiting to be `SIGKILL`ed.
//!   * `fdr_recorder write-clean <dir> <n>` — same, then write the clean_shutdown marker and
//!     exit 0 (the orderly-stop control case).
//!   * `fdr_recorder recover <dir>` — READ-ONLY recovery of both segments; on a dirty stop
//!     emit a PostMortem into `fdr.postmortem.jsonl`; print a one-line JSON summary to
//!     stdout for the integration test to parse.
//!
//! The `kill9_recovers…` integration test drives this via `CARGO_BIN_EXE_fdr_recorder`.

// The FDR ring + `FdrEvent::stamp` are gated off `wasm32` (they take `Instant`/`SystemTime`
// stamps + do file I/O — never reachable on wasm). `cargo build --target wasm32` still
// compiles every `[[bin]]`, so this tool needs a trivial wasm stub `main` (same shape as the
// other kernel bins). The real recorder is native-only.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
use dowiz_kernel::fdr::ring::{self, FdrRing};
#[cfg(not(target_arch = "wasm32"))]
use dowiz_kernel::fdr::schema::{FdrEvent, Kind, StampPolicy};
#[cfg(not(target_arch = "wasm32"))]
use dowiz_kernel::fdr::{self, FdrConfig, Level};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::process::exit;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("write") => {
            let dir = PathBuf::from(&args[2]);
            let n: u64 = args[3].parse().expect("n must be a number");
            let ready = PathBuf::from(&args[4]);
            let mut ring = FdrRing::open(dir, ring::DEFAULT_SEG_CAP).expect("open ring");
            for i in 0..n {
                let seq = ring.next_seq();
                let ev = FdrEvent::stamp(
                    seq,
                    Level::Info,
                    Kind::Event,
                    format!("rec-{i}"),
                    StampPolicy::Full,
                    vec![("i", i.to_string())],
                );
                ring.append(&ev).expect("append");
            }
            // NO fsync here on purpose: kill-9 durability must rely only on write(2)
            // reaching the OS page cache. Touch the ready barrier, then wait to be killed.
            std::fs::write(&ready, b"ready").expect("touch ready");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        Some("write-clean") => {
            let dir = PathBuf::from(&args[2]);
            let n: u64 = args[3].parse().expect("n must be a number");
            let mut ring = FdrRing::open(dir, ring::DEFAULT_SEG_CAP).expect("open ring");
            for i in 0..n {
                let seq = ring.next_seq();
                let ev = FdrEvent::stamp(
                    seq,
                    Level::Info,
                    Kind::Event,
                    format!("rec-{i}"),
                    StampPolicy::Full,
                    vec![("i", i.to_string())],
                );
                ring.append(&ev).expect("append");
            }
            ring.clean_shutdown().expect("clean shutdown");
            exit(0);
        }
        // Item 48 RED control: identical to `panic-child` BUT deliberately does NOT install
        // the panic hook (no fdr::init). Demonstrates the blind-spot as-it-was: the panic
        // writes NOTHING to the FDR ring, so recovery finds events but NO Alarm — the exact
        // RED this item turns green.
        Some("panic-child-nohook") => {
            let dir = PathBuf::from(&args[2]);
            let n: u64 = args[3].parse().expect("n must be a number");
            let ready = PathBuf::from(&args[4]);
            // NOTE: no fdr::init → no panic hook AND no sink (the pre-item-48 behaviour).
            // We write events through a raw FdrRing exactly like the old 'write' child did,
            // so the ONLY difference vs `panic-child` is the absence of the panic hook.
            let mut ring = FdrRing::open(dir, ring::DEFAULT_SEG_CAP).expect("open ring");
            for i in 0..n {
                let seq = ring.next_seq();
                let ev = FdrEvent::stamp(
                    seq,
                    Level::Info,
                    Kind::Event,
                    format!("rec-{i}"),
                    StampPolicy::Full,
                    vec![("i", i.to_string())],
                );
                ring.append(&ev).expect("append");
            }
            std::fs::write(&ready, b"ready").expect("touch ready");
            panic!("item48 RED oracle panic (no hook installed)");
        }
        // Item 48 (closure a) oracle child: install the FDR panic hook (via fdr::init with a
        // ring), write N events THROUGH THE SINK, touch the ready barrier, then PANIC. The
        // panic hook must emit one fsynced `Alarm` carrying the panic site — recovered on the
        // reader's side. (We do NOT open a second raw FdrRing here — events go through the
        // same sink the panic hook appends to, so there is a single writer to the segment
        // files and the Alarm is not corrupted by a dual-writer race.)
        Some("panic-child") => {
            let dir = PathBuf::from(&args[2]);
            let n: u64 = args[3].parse().expect("n must be a number");
            let ready = PathBuf::from(&args[4]);
            let _ = fdr::init(FdrConfig {
                stderr: true,
                ring_dir: Some(dir.clone()),
                seg_cap: ring::DEFAULT_SEG_CAP,
                level: Level::Info,
            });
            // Emit events through the FDR sink (the same path the panic hook uses).
            for i in 0..n {
                fdr::event!(Level::Info, i = i.to_string(), "rec-{i}");
            }
            std::fs::write(&ready, b"ready").expect("touch ready");
            // RED→GREEN: this panic must be recorded by the hook BEFORE the process dies.
            panic!("item48 oracle panic at panic-child site");
        }
        // Item 48 (closure b) oracle child: emit a few heartbeats with increasing seq, then
        // HANG forever (no further heartbeat). The external liveness check asserts the last
        // heartbeat seq stopped advancing WHILE producing NO PostMortem — the exact gap
        // closed. The panic hook is installed (fdr::init) but the hang class is what we
        // exercise here; the kernel never self-kills.
        Some("hang-child") => {
            let dir = PathBuf::from(&args[2]);
            let ready = PathBuf::from(&args[3]);
            let _ = fdr::init(FdrConfig {
                stderr: true,
                ring_dir: Some(dir.clone()),
                seg_cap: ring::DEFAULT_SEG_CAP,
                level: Level::Info,
            });
            for i in 0..5u64 {
                fdr::emit_heartbeat(i, &[("tick", i.to_string())]);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            std::fs::write(&ready, b"ready").expect("touch ready");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        // Item 48 control: a child with a ring + the panic hook installed, that does NOT
        // panic (orderly, alive). Its last heartbeat advances normally; recovery is clean
        // and emits NO PostMortem — the "no false alarm" acceptance criterion.
        Some("heartbeat-child") => {
            let dir = PathBuf::from(&args[2]);
            let n: u64 = args[3].parse().expect("n must be a number");
            let ready = PathBuf::from(&args[4]);
            let _ = fdr::init(FdrConfig {
                stderr: true,
                ring_dir: Some(dir.clone()),
                seg_cap: ring::DEFAULT_SEG_CAP,
                level: Level::Info,
            });
            for i in 0..n {
                fdr::emit_heartbeat(i, &[("tick", i.to_string())]);
            }
            fdr::clean_shutdown(); // Item 48 §5.3: final heartbeat-ish marker = orderly stop.
            std::fs::write(&ready, b"ready").expect("touch ready");
            exit(0);
        }
        // Item 48 recover (with optional `--no-postmortem` so the integration test can read
        // the RING's own `post_mortem` presence, distinct from a reader-side postmortem).
        Some("recover") => {
            let dir = PathBuf::from(&args[2]);
            let no_pm = args.get(3).map(String::as_str) == Some("--no-postmortem");
            let rec = ring::recover(&dir);
            // Did the CHILD's ring itself contain a PostMortem? (A hang-child never restarts,
            // so its ring must NOT have one.) This is the blind-spot-closure signal.
            let ring_has_postmortem = rec.records.iter().any(|r| r.kind == "post_mortem");
            let mut postmortem_written = false;
            if !no_pm && !rec.clean {
                if ring::emit_post_mortem(&dir, &rec).is_ok() {
                    postmortem_written = true;
                }
            }
            // Item 48: report the Alarm / Heartbeat forensic summary so the integration test
            // can assert the panic site was recovered and the heartbeat flatlined (the two
            // blind-spots this item closes).
            let mut has_alarm = false;
            let mut alarm_count = 0usize;
            let mut heartbeat_count = 0usize;
            let mut heartbeat_last_seq: i64 = -1;
            for r in &rec.records {
                if r.kind == "alarm" {
                    has_alarm = true;
                    alarm_count += 1;
                } else if r.kind == "heartbeat" {
                    heartbeat_count += 1;
                    if (r.seq as i64) > heartbeat_last_seq {
                        heartbeat_last_seq = r.seq as i64;
                    }
                }
            }
            let first = rec
                .first_seq()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-1".into());
            let last = rec
                .last_seq()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-1".into());
            println!(
                "{{\"recovered\":{},\"clean\":{},\"torn_tail\":{},\"crc_failures\":{},\"first_seq\":{},\"last_seq\":{},\"postmortem_written\":{},\"ring_has_postmortem\":{},\"has_alarm\":{},\"alarm_count\":{},\"heartbeat_count\":{},\"heartbeat_last_seq\":{}}}",
                rec.records.len(),
                rec.clean,
                rec.torn_tail,
                rec.crc_failures,
                first,
                last,
                postmortem_written,
                ring_has_postmortem,
                has_alarm,
                alarm_count,
                heartbeat_count,
                heartbeat_last_seq
            );
            exit(0);
        }
        _ => {
            eprintln!("usage: fdr_recorder <write|write-clean|recover> <dir> [n] [ready_file]");
            exit(2);
        }
    }
}
