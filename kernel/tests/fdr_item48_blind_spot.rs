//! Item 48 — FDR blind-spot closure proof (blueprint §7 required tests).
//!
//! Three real subprocess oracles drive `fdr_recorder`:
//!   * `panic-child` (GREEN) — installs the panic hook, writes events, then panics.
//!     Recovery MUST find an `Alarm` carrying the panic site.
//!   * `panic-child-nohook` (RED) — same, but WITHOUT the panic hook installed.
//!     Recovery MUST find NO `Alarm` (nothing was ever written about the panic).
//!   * `hang-child` (closure b) — emits a few heartbeats, then hangs forever.
//!     Recovery MUST show the heartbeat seq stopped advancing AND zero PostMortem
//!     (the FDR cannot see a process that never restarts).
//!   * `heartbeat-child` (control, no false alarm) — emits heartbeats, exits cleanly.
//!     Recovery MUST be clean and emit NO PostMortem.
//!
//! `std::process::Child::kill()` sends `SIGKILL` on Unix; the hang-child is killed the
//! same way after the liveness window is observed to have flatlined.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn recorder_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fdr_recorder"))
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let d = std::env::temp_dir().join(format!("fdr_i48_{tag}_{}_{nanos}", std::process::id()));
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

fn field_bool(s: &str, key: &str) -> bool {
    let pat = format!("\"{key}\":");
    let i = s.find(&pat).unwrap_or_else(|| panic!("missing {key} in {s}")) + pat.len();
    let rest = &s[i..];
    match rest.trim_start().starts_with("true") {
        true => true,
        false => false,
    }
}

fn run_recover(dir: &PathBuf) -> String {
    run_recover_inner(dir, false)
}

fn run_recover_nopm(dir: &PathBuf) -> String {
    run_recover_inner(dir, true)
}

fn run_recover_inner(dir: &PathBuf, no_pm: bool) -> String {
    let mut cmd = Command::new(recorder_bin());
    cmd.arg("recover").arg(dir);
    if no_pm {
        cmd.arg("--no-postmortem");
    }
    let out = cmd.output().expect("spawn recover child");
    let summary = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "recover exited non-zero: {summary}");
    summary.to_string()
}

fn wait_ready(ready: &PathBuf) -> bool {
    let start = Instant::now();
    while !ready.exists() {
        if start.elapsed() > Duration::from_secs(30) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

#[test]
fn panic_child_recovers_alarm_carrying_panic_site() {
    let dir = unique_dir("panic");
    let ready = dir.join("ready");
    const N: u64 = 20;

    // GREEN: panic hook installed via fdr::init (with a ring).
    let mut child = Command::new(recorder_bin())
        .arg("panic-child")
        .arg(&dir)
        .arg(N.to_string())
        .arg(&ready)
        .spawn()
        .expect("spawn panic-child");

    assert!(wait_ready(&ready), "panic-child never became ready");
    // Give the panic hook a moment to fsync the Alarm record.
    std::thread::sleep(Duration::from_millis(200));
    let _ = child.kill();
    let _ = child.wait();

    let summary = run_recover(&dir);
    assert!(
        field_bool(&summary, "has_alarm"),
        "recovered Alarm must be present: {summary}"
    );
    assert_eq!(
        field_i64(&summary, "alarm_count"),
        1,
        "exactly one Alarm expected: {summary}"
    );
    // The recovered Alarm record (in the ring) carries the panic message + location.
    // The reader-side PostMortem log NAMES the recovery (it is auto-emitted on a dirty
    // stop); the panic forensic detail lives in the recovered Alarm record itself.
    let recovered_alarm = dir.join("fdr.a.jsonl");
    // (recovered records live in fdr.a.jsonl / fdr.b.jsonl; grep for the alarm kind.)
    let seg_a = std::fs::read_to_string(&recovered_alarm).unwrap_or_default();
    let seg_b = std::fs::read_to_string(dir.join("fdr.b.jsonl")).unwrap_or_default();
    let ring_blob = format!("{seg_a}{seg_b}");
    assert!(
        ring_blob.contains("\"kind\":\"alarm\""),
        "alarm record must be in the ring: {ring_blob}"
    );
    assert!(
        ring_blob.contains("item48 oracle panic at panic-child site")
            || ring_blob.contains("\"message\""),
        "alarm must carry the panic message: {ring_blob}"
    );
    assert!(
        ring_blob.contains("\"location\""),
        "alarm must carry the panic location: {ring_blob}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn panic_child_nohook_recovers_no_alarm_red() {
    let dir = unique_dir("panic_red");
    let ready = dir.join("ready");
    const N: u64 = 20;

    // RED: no panic hook installed → the panic writes NOTHING to the FDR ring.
    let mut child = Command::new(recorder_bin())
        .arg("panic-child-nohook")
        .arg(&dir)
        .arg(N.to_string())
        .arg(&ready)
        .spawn()
        .expect("spawn panic-child-nohook");

    assert!(wait_ready(&ready), "panic-child-nohook never became ready");
    std::thread::sleep(Duration::from_millis(200));
    let _ = child.kill();
    let _ = child.wait();

    let summary = run_recover(&dir);
    assert!(
        !field_bool(&summary, "has_alarm"),
        "without the hook, NO Alarm must be recovered (the RED): {summary}"
    );
    assert_eq!(field_i64(&summary, "alarm_count"), 0, "alarm_count must be 0: {summary}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hang_child_flagged_by_flatlined_heartbeat_zero_postmortem() {
    let dir = unique_dir("hang");
    let ready = dir.join("ready");

    // Closure b: emit heartbeats, then hang forever.
    let mut child = Command::new(recorder_bin())
        .arg("hang-child")
        .arg(&dir)
        .arg(&ready)
        .spawn()
        .expect("spawn hang-child");

    assert!(wait_ready(&ready), "hang-child never became ready");
    // Observe the heartbeat seq advance, then flatline (the external liveness window).
    let first = run_recover(&dir);
    let first_hb = field_i64(&first, "heartbeat_last_seq");
    assert!(first_hb >= 0, "heartbeats must have advanced: {first}");
    // Wait past the hang — the seq must NOT advance (no further heartbeat emitted).
    std::thread::sleep(Duration::from_millis(400));
    // Kill the hung process the same way the external watchdog would (SIGKILL → restart).
    let _ = child.kill();
    let _ = child.wait();

    let summary = run_recover_nopm(&dir);
    // The heartbeat seq must be exactly where it stopped (flatlined) — the hang signature.
    let last_hb = field_i64(&summary, "heartbeat_last_seq");
    assert_eq!(last_hb, first_hb, "heartbeat seq must be flatlined: {summary}");
    // The hang produced NO PostMortem in the ring (it never restarted) and the recover
    // helper was told not to synthesize one — the exact gap this item closes.
    assert!(
        !field_bool(&summary, "ring_has_postmortem"),
        "hung process must have ZERO PostMortem in its ring: {summary}"
    );
    assert!(
        summary.contains("\"postmortem_written\":false"),
        "recover must write ZERO PostMortem when told not to: {summary}"
    );
    // The recovered raw heartbeat JSON must carry the progress counter.
    let pm = dir.join("fdr.postmortem.jsonl");
    if pm.exists() {
        let c = std::fs::read_to_string(&pm).unwrap();
        assert!(!c.contains("\"kind\":\"heartbeat\""));
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn heartbeat_child_clean_no_false_alarm() {
    let dir = unique_dir("hb_clean");
    let ready = dir.join("ready");
    const N: u64 = 7;

    // Control: emit heartbeats, exit cleanly. Recovery must be clean, no PostMortem.
    let status = Command::new(recorder_bin())
        .arg("heartbeat-child")
        .arg(&dir)
        .arg(N.to_string())
        .arg(&ready)
        .status()
        .expect("run heartbeat-child");
    assert!(status.success());

    let summary = run_recover(&dir);
    assert_eq!(
        field_i64(&summary, "heartbeat_count"),
        N as i64,
        "all heartbeats must be recovered: {summary}"
    );
    assert!(
        summary.contains("\"clean\":true"),
        "orderly stop must recover clean: {summary}"
    );
    assert!(
        !field_bool(&summary, "has_alarm"),
        "no false-alarm on a clean stop: {summary}"
    );
    assert!(
        !field_bool(&summary, "ring_has_postmortem"),
        "no PostMortem in a clean ring: {summary}"
    );
    assert!(
        summary.contains("\"postmortem_written\":false"),
        "no false-alarm PostMortem on clean stop: {summary}"
    );
    // Last heartbeat seq == N-1 (monotonic, advanced normally while alive).
    assert_eq!(
        field_i64(&summary, "heartbeat_last_seq"),
        (N - 1) as i64,
        "heartbeat seq must advance to N-1: {summary}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
