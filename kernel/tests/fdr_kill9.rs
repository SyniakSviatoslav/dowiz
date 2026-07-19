//! §G.9 kill-9 durability proof (blueprint §4.4). Spawns a REAL child (`fdr_recorder`),
//! waits on a barrier file (not a sleep), SIGKILLs it mid-flight, then runs a fresh reader
//! against the same segment files and asserts full recovery + graceful torn-tail handling +
//! a PostMortem record emitted naming the recovery.
//!
//! `std::process::Child::kill()` sends `SIGKILL` on Unix — a genuine, unblockable process
//! death (no `libc`, no fake abort).

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn recorder_bin() -> PathBuf {
    // cargo sets CARGO_BIN_EXE_<name> for integration tests of a package with that bin.
    PathBuf::from(env!("CARGO_BIN_EXE_fdr_recorder"))
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let d = std::env::temp_dir().join(format!("fdr_kill9_{tag}_{}_{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn field_i64(s: &str, key: &str) -> i64 {
    let pat = format!("\"{key}\":");
    let i = s.find(&pat).unwrap_or_else(|| panic!("missing {key} in {s}")) + pat.len();
    let rest = &s[i..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '-'))
        .unwrap_or(rest.len());
    rest[..end].parse().unwrap_or_else(|_| panic!("bad {key} in {s}"))
}

#[test]
fn kill9_recovers_all_events_and_emits_postmortem() {
    let dir = unique_dir("kill");
    let ready = dir.join("ready");
    const N: u64 = 300;

    // 1. Spawn the writer child; it writes N events (no fsync) then touches `ready`.
    let mut child = Command::new(recorder_bin())
        .arg("write")
        .arg(&dir)
        .arg(N.to_string())
        .arg(&ready)
        .spawn()
        .expect("spawn writer child");

    // 2. Barrier: wait for the child to signal it finished writing (bounded, no fixed sleep).
    let start = Instant::now();
    while !ready.exists() {
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            panic!("child never became ready within 30s");
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    // 3. SIGKILL the child — genuine process death, no chance to fsync or clean-shutdown.
    child.kill().expect("SIGKILL child");
    let _ = child.wait();

    // 4. Fresh reader process recovers against the same segment files.
    let out = Command::new(recorder_bin())
        .arg("recover")
        .arg(&dir)
        .output()
        .expect("spawn recover child");
    let summary = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "recover exited non-zero: {summary}");

    let recovered = field_i64(&summary, "recovered");
    let torn = field_i64(&summary, "torn_tail");
    let crc_failures = field_i64(&summary, "crc_failures");
    let last_seq = field_i64(&summary, "last_seq");

    // Everything written before the kill is recovered. At most the final in-flight record
    // may be torn (page-cache write interrupted) — allow N-1, require CRC integrity.
    assert!(
        recovered >= (N as i64) - 1,
        "expected >= {} recovered, got {recovered} (summary: {summary})",
        N - 1
    );
    assert!(torn <= 1, "at most one torn tail line expected, got {torn}");
    assert_eq!(crc_failures, 0, "no CRC-valid record may be corrupted: {summary}");
    assert!(
        summary.contains("\"clean\":false"),
        "kill-9 must be recovered as a DIRTY stop: {summary}"
    );
    assert!(
        summary.contains("\"postmortem_written\":true"),
        "a PostMortem record must be emitted naming the recovery: {summary}"
    );
    assert!(recovered - 1 <= last_seq, "last_seq must cover the recovered range: {summary}");

    // 5. The PostMortem log exists and names the recovery.
    let pm = dir.join("fdr.postmortem.jsonl");
    assert!(pm.exists(), "fdr.postmortem.jsonl must exist");
    let pm_contents = std::fs::read_to_string(&pm).unwrap();
    assert!(pm_contents.contains("\"kind\":\"post_mortem\""), "post-mortem record: {pm_contents}");
    assert!(pm_contents.contains("\"recovered\":"), "post-mortem names the count");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clean_shutdown_is_recovered_as_clean_no_postmortem() {
    let dir = unique_dir("clean");
    const N: u64 = 50;

    // Orderly stop: writer writes N events + the clean_shutdown marker, exits 0.
    let status = Command::new(recorder_bin())
        .arg("write-clean")
        .arg(&dir)
        .arg(N.to_string())
        .status()
        .expect("run write-clean");
    assert!(status.success());

    let out = Command::new(recorder_bin())
        .arg("recover")
        .arg(&dir)
        .output()
        .expect("run recover");
    let summary = String::from_utf8_lossy(&out.stdout);
    assert!(summary.contains("\"clean\":true"), "orderly stop must recover clean: {summary}");
    assert!(
        summary.contains("\"postmortem_written\":false"),
        "no post-mortem on a clean stop: {summary}"
    );
    // No post-mortem file should be written on the clean path.
    assert!(!dir.join("fdr.postmortem.jsonl").exists());

    let _ = std::fs::remove_dir_all(&dir);
}
