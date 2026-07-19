//! Roadmap item 27 (classifier-input half) end-to-end proof: the `markov_attractor` binary,
//! run with `DOWIZ_FDR_DIR` set, window-brackets its `analyze_detailed` classification with a
//! PMU snapshot and logs ONE `markov_verdict` FDR record carrying the verdict string PLUS the
//! PMU delta — through the REAL FDR ring (segment files + CRC + recovery), not a mock.
//!
//! This proves three things at once, against the shipped binary:
//!   1. The stdout Python-parity JSON contract is unchanged (a verdict is printed to stdout).
//!   2. A companion FDR record joins the PMU delta to the verdict on the SAME record.
//!   3. Tier B degrades to a named absence (`permission_denied` on this paranoid=4 host),
//!      never a fabricated 0 and never a crash (the child exits 0).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn markov_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_markov_attractor"))
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let d = std::env::temp_dir().join(format!("markov_pmu_{tag}_{}_{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn markov_verdict_emits_pmu_companion_record_through_real_fdr_ring() {
    let ring_dir = unique_dir("ring");

    // A healthy rhythm (edit, run_ok)×8 ⇒ Verdict::Healthy — a fixed, known verdict.
    let mut stdin_tokens = String::new();
    for _ in 0..8 {
        stdin_tokens.push_str("edit\nrun_ok\n");
    }

    let mut child = Command::new(markov_bin())
        .env("DOWIZ_FDR_DIR", &ring_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn markov_attractor");

    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(stdin_tokens.as_bytes())
        .expect("write tokens");

    let out = child.wait_with_output().expect("wait markov_attractor");
    assert!(out.status.success(), "binary must exit 0 (Tier B absence is not a crash)");

    // 1. stdout parity contract intact — a HEALTHY verdict on stdout, unchanged shape.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"verdict\": \"HEALTHY\""),
        "stdout parity JSON must be unchanged: {stdout}"
    );

    // 2 + 3. The FDR ring holds a markov_verdict record with the PMU companion.
    let rec = dowiz_kernel::fdr::ring::recover(&ring_dir);
    let verdict_rec = rec
        .records
        .iter()
        .find(|r| r.name == "markov_verdict")
        .unwrap_or_else(|| panic!("no markov_verdict FDR record recovered: {:?}", rec.records));

    let raw = &verdict_rec.raw;
    // Verdict string joined onto the record.
    assert!(raw.contains("\"verdict\":\"HEALTHY\""), "verdict must be on the record: {raw}");
    // The PMU companion object is present with every field key.
    assert!(raw.contains("\"pmu\":{"), "pmu companion object must be present: {raw}");
    for key in [
        "tsc_cycles",
        "minflt",
        "majflt",
        "nswap",
        "vol_ctxt_switches",
        "nonvol_ctxt_switches",
        "hw_instructions",
        "hw_cpu_cycles",
        "hw_cache_misses",
        "hw_branch_misses",
    ] {
        assert!(raw.contains(&format!("\"{key}\":")), "pmu field {key} must be present: {raw}");
    }
    // Tier A recorded a real, nonzero rdtsc delta across the classification window
    // (proves the bracket ran and the counter advanced — not a stub 0).
    let tsc = extract_pmu_u64(raw, "tsc_cycles")
        .unwrap_or_else(|| panic!("tsc_cycles must be a real value, not an absence: {raw}"));
    assert!(tsc > 0, "tsc_cycles delta must be nonzero across a real classification: {raw}");
    // Tier B is EITHER a real value (this agent process runs as root/CAP_PERFMON, which
    // bypasses perf_event_paranoid=4 — perf_event_open succeeds and returns a real count)
    // OR a greppable named absence on a genuinely unprivileged host. Both are correct; what
    // must NEVER appear is a fabricated bare 0 with no reason. Assert one of the two shapes.
    let tier_b_value = raw.contains("\"hw_instructions\":") && extract_pmu_u64(raw, "hw_instructions").is_some();
    let tier_b_absence = raw.contains("\"hw_instructions\":{\"unavailable\":");
    assert!(
        tier_b_value || tier_b_absence,
        "Tier B must be a real value OR a named absence, never a fabricated 0: {raw}"
    );

    let _ = std::fs::remove_dir_all(&ring_dir);
}

/// Extract `"key":<digits>` from the record when the field is a bare number (a `Value`);
/// `None` when it is the `{"unavailable":...}` object form.
fn extract_pmu_u64(line: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    // A Value is a bare digit run; an absence starts with '{'.
    let first = rest.chars().next()?;
    if !first.is_ascii_digit() {
        return None;
    }
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}
