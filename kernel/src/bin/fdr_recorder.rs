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
use dowiz_kernel::fdr::Level;
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
        Some("recover") => {
            let dir = PathBuf::from(&args[2]);
            let rec = ring::recover(&dir);
            let mut postmortem_written = false;
            if !rec.clean {
                if ring::emit_post_mortem(&dir, &rec).is_ok() {
                    postmortem_written = true;
                }
            }
            let first = rec.first_seq().map(|s| s.to_string()).unwrap_or_else(|| "-1".into());
            let last = rec.last_seq().map(|s| s.to_string()).unwrap_or_else(|| "-1".into());
            println!(
                "{{\"recovered\":{},\"clean\":{},\"torn_tail\":{},\"crc_failures\":{},\"first_seq\":{},\"last_seq\":{},\"postmortem_written\":{}}}",
                rec.records.len(),
                rec.clean,
                rec.torn_tail,
                rec.crc_failures,
                first,
                last,
                postmortem_written
            );
            exit(0);
        }
        _ => {
            eprintln!("usage: fdr_recorder <write|write-clean|recover> <dir> [n] [ready_file]");
            exit(2);
        }
    }
}
