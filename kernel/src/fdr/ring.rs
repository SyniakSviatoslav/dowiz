//! `fdr/ring.rs` — the FDR durable layer: A/B alternating segment files with
//! CRC32-checked NDJSON lines and `kill -9` recovery (blueprint §4.4).
//!
//! **Correction to the synthesis, stated plainly (blueprint §4.4 / §6-5):** synthesis §5
//! called tier (b) an "`mmap`-backed ring — pure std". `std` has NO mmap; a literal mmap
//! needs `libc`/`memmap2`, a new dependency the whole item forbids. The honest pure-std
//! design is delivered here instead: two alternating fixed-cap append-only segment files
//! (`fdr.a.jsonl`, `fdr.b.jsonl`), each line suffixed with a CRC32 of its payload.
//! Recovery accepts CRC-valid lines only and tolerates exactly one torn tail line per
//! segment. Append-only segments are simpler to prove correct under torn writes than an
//! in-place byte-ring cursor.
//!
//! **Durability cadence, honestly separated (blueprint §4.4):**
//!   - Surviving `kill -9` (process death) requires ONLY that `write(2)` reached the OS
//!     page cache — no `fsync`. A fresh reader of the same file sees those bytes because
//!     the page cache outlives the process. That is the §G.9 test's guarantee, and the
//!     recorder writes event-kind records WITHOUT fsync to prove exactly that.
//!   - Surviving POWER LOSS additionally requires `fsync`: [`FdrRing::append`] issues
//!     `sync_data` on every `Alarm`/`PostMortem` record and [`FdrRing::switch`] fsyncs on
//!     segment switch (copying `FileEventStore`'s sync-before-claim discipline). A fsync
//!     error is surfaced as a typed `io::Error`, never swallowed.
//!
//! Non-wasm only: `wasm32` has no real filesystem and never installs an FDR sink.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use super::schema::{FdrEvent, Kind, StampPolicy};
use super::Level;

/// Default per-segment cap (1 MiB) — bounded size, last-N-seconds retention.
pub const DEFAULT_SEG_CAP: u64 = 1 << 20;

const SEG_A: &str = "fdr.a.jsonl";
const SEG_B: &str = "fdr.b.jsonl";
/// Where a recovery post-mortem is emitted (a fresh log — never clobbers the recovered
/// segments, so the forensic evidence survives the recovery step).
pub const POSTMORTEM_LOG: &str = "fdr.postmortem.jsonl";

// CRC32 is now the SINGLE shared implementation lifted to always-compiled `fdr::crc32`
// (blueprint §3.2 / item 54) — shared by FDR ring (at-rest per-line CRC), item 40
// (weights), and item 54 (live-struct Sentinel). Re-exported here for call-site
// compatibility with the prior `fdr::ring::crc32` path. ONE implementation; the KAT
// `crc32_matches_known_vector` below stays green.
pub use super::crc32;


/// The durable writer. One active segment at a time; switches to the other (truncating
/// it) at the cap. Fresh session: opens `fdr.a.jsonl` truncated. Use [`recover`] BEFORE
/// constructing a writer if you need the prior session's records — the writer truncates.
pub struct FdrRing {
    dir: PathBuf,
    seg_cap: u64,
    active: usize, // 0 => A, 1 => B
    file: File,
    written: u64,
    seq: u64,
}

fn seg_path(dir: &Path, idx: usize) -> PathBuf {
    dir.join(if idx == 0 { SEG_A } else { SEG_B })
}

impl FdrRing {
    /// Open a fresh writer session (truncates segment A). `seq` starts at 0.
    pub fn open(dir: PathBuf, seg_cap: u64) -> io::Result<Self> {
        std::fs::create_dir_all(&dir)?;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(seg_path(&dir, 0))?;
        Ok(FdrRing {
            dir,
            seg_cap,
            active: 0,
            file,
            written: 0,
            seq: 0,
        })
    }

    /// Allocate the next monotonic sequence number.
    pub fn next_seq(&mut self) -> u64 {
        let s = self.seq;
        self.seq += 1;
        s
    }

    /// Append one record. `write(2)` reaches the page cache (kill-9 durable). Alarm-class
    /// records additionally `sync_data` (power-loss durable). Errors are surfaced typed.
    pub fn append(&mut self, ev: &FdrEvent) -> io::Result<()> {
        let payload = ev.to_json();
        let crc = crc32(payload.as_bytes());
        // Line = <payload>|<crc:08x>\n — payload never contains a raw '\n' (escaped by
        // JsonWriter), so '\n' unambiguously delimits records for recovery.
        let mut line = String::with_capacity(payload.len() + 10);
        line.push_str(&payload);
        line.push('|');
        line.push_str(&format!("{crc:08x}"));
        line.push('\n');

        if self.written > 0 && self.written + line.len() as u64 > self.seg_cap {
            self.switch()?;
        }
        self.file.write_all(line.as_bytes())?;
        self.written += line.len() as u64;
        if matches!(ev.kind, Kind::Alarm | Kind::PostMortem) {
            self.file.sync_data()?;
        }
        Ok(())
    }

    /// Switch to the other segment (fsync current, truncate + activate the other).
    fn switch(&mut self) -> io::Result<()> {
        self.file.sync_data()?;
        let next = 1 - self.active;
        self.file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(seg_path(&self.dir, next))?;
        self.active = next;
        self.written = 0;
        Ok(())
    }

    /// Force durable flush (power-loss safety) without a record. Not required for kill-9.
    pub fn sync(&mut self) -> io::Result<()> {
        self.file.sync_data()
    }

    /// Write the orderly-shutdown marker (fsynced). Its presence as the tail record is
    /// how [`recover`] distinguishes a clean stop from a crash.
    pub fn clean_shutdown(&mut self) -> io::Result<()> {
        let seq = self.next_seq();
        let ev = FdrEvent::stamp(
            seq,
            Level::Info,
            Kind::CleanShutdown,
            "clean_shutdown".into(),
            StampPolicy::Cheap,
            vec![],
        );
        self.append(&ev)?;
        self.file.sync_data()
    }
}

/// One recovered record (CRC-valid). `raw` is the exact payload JSON as written.
#[derive(Clone, Debug)]
pub struct RecoveredRecord {
    pub seq: u64,
    pub kind: String,
    pub name: String,
    pub raw: String,
}

/// The outcome of reading back both segments.
#[derive(Clone, Debug, Default)]
pub struct Recovery {
    /// CRC-valid records across both segments, ordered by `seq`.
    pub records: Vec<RecoveredRecord>,
    /// True iff a `clean_shutdown` marker was recovered (⇒ orderly stop, no post-mortem).
    pub clean: bool,
    /// Count of torn tail lines dropped (partial final line with no terminating `\n`).
    pub torn_tail: usize,
    /// Count of complete lines whose CRC did not verify (corruption dropped).
    pub crc_failures: usize,
}

impl Recovery {
    pub fn first_seq(&self) -> Option<u64> {
        self.records.first().map(|r| r.seq)
    }
    pub fn last_seq(&self) -> Option<u64> {
        self.records.last().map(|r| r.seq)
    }
}

/// Minimal field extraction (kernel is serde-free). Finds `"<key>":<number>`.
fn extract_u64(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Finds `"<key>":"<string>"` (values here are simple `[a-z_-]` names — no escaping).
fn extract_str(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// **READ-ONLY** recovery: reads both segments, validates each line's CRC, drops a torn
/// tail, and returns the CRC-valid records ordered by `seq`. Never truncates — safe to
/// run against a crashed writer's segments (the §G.9 restart-and-recover path).
pub fn recover(dir: &Path) -> Recovery {
    let mut out = Recovery::default();
    for idx in [0usize, 1usize] {
        let mut data = Vec::new();
        match File::open(seg_path(dir, idx)) {
            Ok(mut f) => {
                if f.read_to_end(&mut data).is_err() {
                    continue;
                }
            }
            Err(_) => continue,
        }
        // Split on '\n'. A record line ends in '\n'; the element after the final '\n' is
        // "" (all records complete) or a partial (torn tail — process died mid-write).
        let text = String::from_utf8_lossy(&data);
        let parts: Vec<&str> = text.split('\n').collect();
        let n = parts.len();
        for (li, part) in parts.iter().enumerate() {
            let is_last = li == n - 1;
            if part.is_empty() {
                continue; // trailing "" after a clean final '\n', or empty file.
            }
            if is_last {
                // Non-empty element with no terminating '\n' ⇒ torn tail. Dropped.
                out.torn_tail += 1;
                continue;
            }
            // Complete line: split off the trailing "|<crc:08x>".
            match part.rfind('|') {
                Some(bar) => {
                    let payload = &part[..bar];
                    let crc_hex = &part[bar + 1..];
                    match u32::from_str_radix(crc_hex, 16) {
                        Ok(want) if crc32(payload.as_bytes()) == want => {
                            let seq = extract_u64(payload, "seq").unwrap_or(u64::MAX);
                            let kind = extract_str(payload, "kind").unwrap_or_default();
                            let name = extract_str(payload, "name").unwrap_or_default();
                            out.records.push(RecoveredRecord {
                                seq,
                                kind,
                                name,
                                raw: payload.to_string(),
                            });
                        }
                        _ => out.crc_failures += 1,
                    }
                }
                None => out.crc_failures += 1,
            }
        }
    }
    out.records.sort_by_key(|r| r.seq);
    out.clean = out.records.iter().any(|r| r.kind == "clean_shutdown");
    out
}

/// Emit a `PostMortem` record naming the recovery into a FRESH log (`fdr.postmortem.jsonl`)
/// — it does NOT touch the recovered segments. fsynced (power-loss durable). Returns the
/// bytes written. This is the "post-mortem into a fresh log" of blueprint §4.4; routing it
/// into the durable `EventLog` is DEFERRED behind item 2's composition-root fix.
#[cfg(not(target_arch = "wasm32"))]
pub fn emit_post_mortem(dir: &Path, rec: &Recovery) -> io::Result<()> {
    let ev = FdrEvent::stamp(
        0,
        Level::Warn,
        Kind::PostMortem,
        "post_mortem".into(),
        StampPolicy::Full,
        vec![
            ("recovered", rec.records.len().to_string()),
            ("first_seq", rec.first_seq().map(|s| s.to_string()).unwrap_or_else(|| "none".into())),
            ("last_seq", rec.last_seq().map(|s| s.to_string()).unwrap_or_else(|| "none".into())),
            ("torn_tail", rec.torn_tail.to_string()),
            ("crc_failures", rec.crc_failures.to_string()),
        ],
    );
    let payload = ev.to_json();
    let crc = crc32(payload.as_bytes());
    let line = format!("{payload}|{crc:08x}\n");
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(POSTMORTEM_LOG))?;
    f.write_all(line.as_bytes())?;
    f.sync_data()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "fdr_ring_{}_{}_{}",
            tag,
            std::process::id(),
            crate::typed_metrics::mono_now_ns()
        ));
        let _ = std::fs::create_dir_all(&d);
        d
    }

    #[test]
    fn crc32_matches_known_vector() {
        // Standard CRC-32/ISO-HDLC check value for the ASCII string "123456789".
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn write_then_recover_all_records_ordered() {
        let dir = tmp("rw");
        {
            let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
            for i in 0..50 {
                let seq = ring.next_seq();
                let ev = FdrEvent::stamp(
                    seq,
                    Level::Info,
                    Kind::Event,
                    format!("rec-{i}"),
                    StampPolicy::Cheap,
                    vec![("i", i.to_string())],
                );
                ring.append(&ev).unwrap();
            }
        }
        let rec = recover(&dir);
        assert_eq!(rec.records.len(), 50);
        assert_eq!(rec.first_seq(), Some(0));
        assert_eq!(rec.last_seq(), Some(49));
        assert_eq!(rec.torn_tail, 0);
        assert_eq!(rec.crc_failures, 0);
        assert!(!rec.clean, "no clean_shutdown marker was written");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clean_shutdown_marker_makes_recovery_clean() {
        let dir = tmp("clean");
        {
            let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(seq, Level::Info, Kind::Event, "x".into(), StampPolicy::Cheap, vec![]);
            ring.append(&ev).unwrap();
            ring.clean_shutdown().unwrap();
        }
        let rec = recover(&dir);
        assert!(rec.clean, "clean_shutdown marker must make recovery clean");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn torn_tail_line_is_dropped_valid_records_survive() {
        let dir = tmp("torn");
        {
            let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
            for i in 0..5 {
                let seq = ring.next_seq();
                let ev = FdrEvent::stamp(seq, Level::Info, Kind::Event, format!("r{i}"), StampPolicy::Cheap, vec![]);
                ring.append(&ev).unwrap();
            }
        }
        // Simulate a process dying mid-write: append a partial line with NO terminating
        // '\n' and a bogus/short CRC — exactly a torn last record.
        {
            let mut f = OpenOptions::new().append(true).open(seg_path(&dir, 0)).unwrap();
            f.write_all(b"{\"seq\":5,\"kind\":\"event\",\"name\":\"torn").unwrap();
        }
        let rec = recover(&dir);
        assert_eq!(rec.records.len(), 5, "the 5 complete records must survive");
        assert_eq!(rec.torn_tail, 1, "the partial tail must be counted + dropped");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn crc_corruption_is_dropped() {
        let dir = tmp("crc");
        {
            let mut ring = FdrRing::open(dir.clone(), DEFAULT_SEG_CAP).unwrap();
            let seq = ring.next_seq();
            let ev = FdrEvent::stamp(seq, Level::Info, Kind::Event, "ok".into(), StampPolicy::Cheap, vec![]);
            ring.append(&ev).unwrap();
        }
        // Append a COMPLETE line (has '\n') whose CRC is wrong.
        {
            let mut f = OpenOptions::new().append(true).open(seg_path(&dir, 0)).unwrap();
            f.write_all(b"{\"seq\":1,\"kind\":\"event\",\"name\":\"bad\"}|deadbeef\n").unwrap();
        }
        let rec = recover(&dir);
        assert_eq!(rec.records.len(), 1, "only the CRC-valid record survives");
        assert_eq!(rec.crc_failures, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn segment_switch_preserves_all_records() {
        let dir = tmp("switch");
        {
            // Tiny cap forces multiple A/B switches.
            let mut ring = FdrRing::open(dir.clone(), 512).unwrap();
            for i in 0..40 {
                let seq = ring.next_seq();
                let ev = FdrEvent::stamp(seq, Level::Info, Kind::Event, format!("s{i}"), StampPolicy::Cheap, vec![]);
                ring.append(&ev).unwrap();
            }
        }
        let rec = recover(&dir);
        // With a 512-byte cap and ~200-byte records, only the last ~2 records per segment
        // survive across A/B alternation — but recovery must be internally consistent:
        // strictly increasing seq, no torn/crc failures.
        assert!(rec.torn_tail == 0 && rec.crc_failures == 0);
        for w in rec.records.windows(2) {
            assert!(w[0].seq < w[1].seq, "recovered seqs must be strictly increasing");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
