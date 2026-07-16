//! ci-truth — native Rust CI truth-floor (no bash).
//!
//! Two subcommands, ported 1:1 from the retired bash scripts. `git` and `cargo`
//! are genuinely external binaries (shelled via std::process::Command); all the
//! *logic* — parsing, idempotency, red-line detection, JSON emission, worktree
//! lifecycle — is native Rust, not bash.
//!
//!   ci-truth claim-latency [<sha> | <base> <head> | <base>..<head>]
//!       V5-B claim-latency ledger appender (was scripts/claim-latency-append.sh).
//!       Appends ONE JSONL entry per NEW commit to docs/ledger/claim-latency.jsonl:
//!         {commit_sha, authored_ts, ci_observed_green_ts, delta_s, diff_loc}
//!       Idempotent: a commit already in the ledger is skipped.
//!       EXIT 0 on success (incl. "nothing to do"); non-zero on usage/git error.
//!
//!   ci-truth v5c-reexec [<base>] [<head>]        (defaults: origin/main HEAD)
//!       V5-C independent re-execution verifier (was scripts/v5c-reexec.sh).
//!       Red-line-gated: when the diff range touches money.rs / order_machine.rs /
//!       event_log.rs / auth·otp·jwt, it re-runs kernel + engine `cargo test` in a
//!       CLEAN independent git worktree and emits RED|GREEN; otherwise SKIP.
//!       EXIT 0 GREEN|SKIP, 1 RED, 2 usage/git-resolution error.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// ===========================================================================
// git helpers (git is an external binary the kernel never reimplements).
// ===========================================================================

/// Run `git <args>`; return trimmed stdout on success, `None` on non-zero exit
/// or spawn failure. Mirrors a bash `$(git …)` that aborts under `set -e`.
fn git_ok(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ===========================================================================
// Subcommand 1 — claim-latency  (port of scripts/claim-latency-append.sh)
// ===========================================================================

/// Sum added+deleted across a `git show --numstat --format=` body. Binary files
/// render their columns as `-` (counted 0); blank lines contribute 0. Pure port
/// of the awk `{a=$1;d=$2; if(a=="-")a=0; if(d=="-")d=0; s+=a+d}`.
fn numstat_sum(output: &str) -> i64 {
    let mut sum: i64 = 0;
    for line in output.lines() {
        let mut cols = line.split_whitespace();
        let a = cols.next();
        let d = cols.next();
        if let (Some(a), Some(d)) = (a, d) {
            let av = if a == "-" { 0 } else { a.parse::<i64>().unwrap_or(0) };
            let dv = if d == "-" { 0 } else { d.parse::<i64>().unwrap_or(0) };
            sum += av + dv;
        }
    }
    sum
}

/// diff_loc for one commit: `git show --numstat --format= <sha>` summed.
fn diff_loc(sha: &str) -> i64 {
    let out = git_ok(&["show", "--numstat", "--format=", sha]).unwrap_or_default();
    numstat_sum(&out)
}

/// Resolve the oldest-first commit list from the positional args, exactly as the
/// bash `case "$#"` did. Err(code) mirrors the script's usage/git exits.
fn resolve_commits(pos: &[String]) -> Result<Vec<String>, i32> {
    match pos.len() {
        0 => {
            let head = git_ok(&["rev-parse", "HEAD"]).ok_or(1)?;
            Ok(vec![head])
        }
        1 => {
            if pos[0].contains("..") {
                let out = git_ok(&["rev-list", "--reverse", &pos[0]]).ok_or(1)?;
                Ok(out.lines().filter(|l| !l.is_empty()).map(String::from).collect())
            } else {
                let spec = format!("{}^{{commit}}", pos[0]);
                let c = git_ok(&["rev-parse", &spec]).ok_or(1)?;
                Ok(vec![c])
            }
        }
        2 => {
            let range = format!("{}..{}", pos[0], pos[1]);
            let out = git_ok(&["rev-list", "--reverse", &range]).ok_or(1)?;
            Ok(out.lines().filter(|l| !l.is_empty()).map(String::from).collect())
        }
        _ => Err(2),
    }
}

fn claim_latency(pos: &[String]) -> i32 {
    let repo_root = match git_ok(&["rev-parse", "--show-toplevel"]) {
        Some(r) => r,
        None => {
            eprintln!("claim-latency: not inside a git repo");
            return 1;
        }
    };
    let ledger = PathBuf::from(&repo_root).join("docs/ledger/claim-latency.jsonl");
    if let Some(dir) = ledger.parent() {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("claim-latency: cannot create {}: {e}", dir.display());
            return 1;
        }
    }

    let commits = match resolve_commits(pos) {
        Ok(c) => c,
        Err(2) => {
            eprintln!(
                "claim-latency-append: usage: ci-truth claim-latency [<sha> | <base> <head> | <base>..<head>]"
            );
            return 2;
        }
        Err(code) => {
            eprintln!("claim-latency: git resolution failed");
            return code;
        }
    };

    if commits.is_empty() {
        eprintln!("claim-latency-append: no commits in range — nothing to append.");
        return 0;
    }

    // Observation time captured ONCE per invocation (the Rust-native `date +%s`):
    // the moment the caller/CI is confirming this run's tree is GREEN.
    let observed_ts = now_unix();

    // Read the current ledger once for the substring idempotency check.
    let existing = fs::read_to_string(&ledger).unwrap_or_default();

    let mut appended = 0i64;
    let mut skipped = 0i64;
    let mut pending = String::new();
    for sha in &commits {
        let full_sha = match git_ok(&["rev-parse", sha]) {
            Some(s) => s,
            None => {
                eprintln!("claim-latency: cannot resolve commit '{sha}'");
                return 1;
            }
        };

        // Idempotency: never append a commit already recorded (substring match,
        // same as the bash `grep -q "\"commit_sha\":\"<sha>\""`).
        let needle = format!("\"commit_sha\":\"{full_sha}\"");
        if existing.contains(&needle) || pending.contains(&needle) {
            skipped += 1;
            continue;
        }

        let authored_ts: i64 = match git_ok(&["show", "-s", "--format=%at", &full_sha]) {
            Some(s) => match s.parse() {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("claim-latency: unparseable authored_ts for {full_sha}: '{s}'");
                    return 1;
                }
            },
            None => {
                eprintln!("claim-latency: cannot read authored_ts for {full_sha}");
                return 1;
            }
        };
        let delta_s = observed_ts - authored_ts;
        let loc = diff_loc(&full_sha);

        pending.push_str(&format!(
            "{{\"commit_sha\":\"{full_sha}\",\"authored_ts\":{authored_ts},\"ci_observed_green_ts\":{observed_ts},\"delta_s\":{delta_s},\"diff_loc\":{loc}}}\n"
        ));
        appended += 1;
    }

    if !pending.is_empty() {
        match OpenOptions::new().create(true).append(true).open(&ledger) {
            Ok(mut f) => {
                if let Err(e) = f.write_all(pending.as_bytes()) {
                    eprintln!("claim-latency: append failed: {e}");
                    return 1;
                }
            }
            Err(e) => {
                eprintln!("claim-latency: cannot open ledger {}: {e}", ledger.display());
                return 1;
            }
        }
    }

    println!(
        "claim-latency-append: appended {appended}, skipped {skipped} (already present) -> {}",
        ledger.display()
    );
    0
}

// ===========================================================================
// Subcommand 2 — v5c-reexec  (port of scripts/v5c-reexec.sh)
// ===========================================================================

/// Red-line surface match — money / orders / event_log / auth·otp·jwt, case
/// insensitive. Native port of `grep -Ei 'money\.rs|order_machine\.rs|event_log\.rs|auth|otp|jwt'`.
fn is_redline(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains("money.rs")
        || p.contains("order_machine.rs")
        || p.contains("event_log.rs")
        || p.contains("auth")
        || p.contains("otp")
        || p.contains("jwt")
}

/// JSON array from repo paths / test ids (defensively strips stray quotes),
/// mirroring the bash `json_array()`.
fn json_array(items: &[String]) -> String {
    let mut s = String::from("[");
    let mut first = true;
    for item in items {
        if item.is_empty() {
            continue;
        }
        let cleaned = item.replace('"', "");
        if first {
            first = false;
        } else {
            s.push(',');
        }
        s.push('"');
        s.push_str(&cleaned);
        s.push('"');
    }
    s.push(']');
    s
}

/// Sum every integer N appearing as `<N> <keyword>` in the text — the native
/// equivalent of `grep -oE '[0-9]+ <keyword>' | awk '{s+=$1}'`. cargo prints
/// e.g. "42 passed;" so a trailing ';' after the keyword is tolerated.
fn sum_counts(text: &str, keyword: &str) -> i64 {
    let bytes = text.as_bytes();
    let kw = format!(" {keyword}");
    let mut sum = 0i64;
    let mut from = 0usize;
    while let Some(rel) = text[from..].find(&kw) {
        let pos = from + rel; // index of the space preceding the keyword
        let mut i = pos;
        while i > 0 && bytes[i - 1].is_ascii_digit() {
            i -= 1;
        }
        if i < pos {
            if let Ok(n) = text[i..pos].parse::<i64>() {
                sum += n;
            }
        }
        from = pos + kw.len();
    }
    sum
}

/// Extract failing test ids from a suite's output — native port of
/// `grep -hE '\.\.\. FAILED' | sed -E 's/^test ([^ ]+) \.\.\. FAILED.*/\1/'`.
/// A "... FAILED" line that does NOT match the `test <name> ...` shape is passed
/// through verbatim (sed's no-match passthrough).
fn parse_failing(text: &str, out: &mut Vec<String>) {
    for line in text.lines() {
        if !line.contains("... FAILED") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("test ") {
            if let Some(idx) = rest.find(" ... FAILED") {
                out.push(rest[..idx].to_string());
                continue;
            }
        }
        out.push(line.to_string());
    }
}

/// Resolve a ref to a full commit SHA via `git rev-parse --verify --quiet <ref>^{commit}`.
fn resolve_ref(r: &str) -> Option<String> {
    let spec = format!("{r}^{{commit}}");
    git_ok(&["rev-parse", "--verify", "--quiet", &spec]).filter(|s| !s.is_empty())
}

/// RAII worktree — `Drop` runs `git worktree remove --force`, the native idiom
/// for bash's `trap cleanup EXIT`: cleanup fires on normal return, early return,
/// or panic. Falls back to `rm -rf` if the git removal fails.
struct WorktreeGuard {
    path: PathBuf,
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let removed = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&self.path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !removed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Run one suite (`cargo test --offline --manifest-path <manifest>`) INSIDE the
/// worktree; return (exit_code, passed, failed, combined_stdout_stderr).
fn run_suite(worktree: &Path, manifest: &str) -> (i32, i64, i64, String) {
    let out = Command::new("cargo")
        .args(["test", "--offline", "--manifest-path", manifest])
        .current_dir(worktree)
        .output();
    match out {
        Ok(o) => {
            let mut combined = String::from_utf8_lossy(&o.stdout).into_owned();
            combined.push_str(&String::from_utf8_lossy(&o.stderr));
            let exit = o.status.code().unwrap_or(1);
            let passed = sum_counts(&combined, "passed");
            let failed = sum_counts(&combined, "failed");
            (exit, passed, failed, combined)
        }
        Err(e) => (1, 0, 0, format!("cargo spawn failed: {e}")),
    }
}

fn v5c_reexec(pos: &[String]) -> i32 {
    let base_ref = pos.first().cloned().unwrap_or_else(|| "origin/main".to_string());
    let head_ref = pos.get(1).cloned().unwrap_or_else(|| "HEAD".to_string());

    let head_sha = match resolve_ref(&head_ref) {
        Some(s) => s,
        None => {
            eprintln!("v5c-reexec: cannot resolve head '{head_ref}'");
            return 2;
        }
    };
    // base tolerates a missing origin/main in CI: fall back to `main`, then HEAD~1.
    let base_sha = match resolve_ref(&base_ref) {
        Some(s) => s,
        None => match resolve_ref("main").or_else(|| resolve_ref(&format!("{head_sha}~1"))) {
            Some(s) => s,
            None => {
                eprintln!("v5c-reexec: cannot resolve base '{base_ref}' (nor main / HEAD~1)");
                return 2;
            }
        },
    };

    // --- red-line detection over the diff range ---
    let changed = git_ok(&["diff", "--name-only", &base_sha, &head_sha]).unwrap_or_default();
    let redline_hits: Vec<String> =
        changed.lines().filter(|p| is_redline(p)).map(String::from).collect();

    if redline_hits.is_empty() {
        println!("V5C-VERDICT: SKIP");
        println!(
            "{{\"verdict\":\"SKIP\",\"signed\":false,\"base\":\"{base_sha}\",\"head\":\"{head_sha}\",\"red_line_paths\":[],\"reason\":\"no red-line path (money.rs/order_machine.rs/event_log.rs/auth) touched in base..head; V5-C re-exec is red-line-gated per BLUEPRINT-P01 §2.8\"}}"
        );
        return 0;
    }

    // --- clean independent worktree at the head SHA ---
    let worktree = std::env::temp_dir().join(format!(
        "v5c-reexec.{}.{}",
        std::process::id(),
        now_unix()
    ));
    let add_ok = Command::new("git")
        .args(["worktree", "add", "--detach"])
        .arg(&worktree)
        .arg(&head_sha)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !add_ok {
        eprintln!("v5c-reexec: git worktree add failed");
        // The dir may have been partially created; best-effort clean.
        let _ = fs::remove_dir_all(&worktree);
        return 2;
    }
    // From here on, the guard guarantees teardown on every exit path.
    let guard = WorktreeGuard { path: worktree.clone() };

    let (k_exit, k_pass, k_fail, k_log) = run_suite(&guard.path, "kernel/Cargo.toml");
    let (e_exit, e_pass, e_fail, e_log) = run_suite(&guard.path, "engine/Cargo.toml");

    // Failing test ids (empty on GREEN) — kernel log first, then engine.
    let mut failing: Vec<String> = Vec::new();
    parse_failing(&k_log, &mut failing);
    parse_failing(&e_log, &mut failing);

    let (verdict, exit_code) = if k_exit != 0 || e_exit != 0 {
        ("RED", 1)
    } else {
        ("GREEN", 0)
    };

    println!("V5C-VERDICT: {verdict}");
    println!(
        "{{\"verdict\":\"{verdict}\",\"signed\":false,\"base\":\"{base_sha}\",\"head\":\"{head_sha}\",\"red_line_paths\":{},\"suites\":{{\"kernel\":{{\"ran\":true,\"exit\":{k_exit},\"passed\":{k_pass},\"failed\":{k_fail}}},\"engine\":{{\"ran\":true,\"exit\":{e_exit},\"passed\":{e_pass},\"failed\":{e_fail}}}}},\"failing_tests\":{},\"note\":\"UNSIGNED (Phase 1); Phase 6 wraps this runner with ML-DSA key_K/key_V signatures\"}}",
        json_array(&redline_hits),
        json_array(&failing)
    );

    // `guard` drops HERE (function scope end) — before main calls process::exit,
    // so the worktree is always removed. Do not std::process::exit from within.
    drop(guard);
    exit_code
}

// ===========================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sub = args.get(1).map(String::as_str).unwrap_or("");
    let pos: &[String] = if args.len() > 2 { &args[2..] } else { &[] };

    let code = match sub {
        "claim-latency" => claim_latency(pos),
        "v5c-reexec" => v5c_reexec(pos),
        _ => {
            eprintln!("ci-truth: usage: ci-truth <claim-latency|v5c-reexec> [args]");
            eprintln!("  claim-latency [<sha> | <base> <head> | <base>..<head>]   append V5-B ledger entries");
            eprintln!("  v5c-reexec [<base>] [<head>]                             independent re-exec (default origin/main HEAD)");
            2
        }
    };

    // stdout is line-buffered (flushed per println!), but flush explicitly since
    // process::exit does not run buffer flushes or destructors.
    let _ = std::io::stdout().flush();
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- numstat summing (claim-latency diff_loc) ---
    #[test]
    fn numstat_sum_basic() {
        // added + deleted across two text files = 10+2 + 3+0.
        let out = "10\t2\tsrc/a.rs\n3\t0\tsrc/b.rs\n";
        assert_eq!(numstat_sum(out), 15);
    }

    #[test]
    fn numstat_sum_binary_and_blank() {
        // Binary rows show '-'/'-' (counted 0); the leading blank line from
        // `--format=` contributes 0. Only 5+4 counts.
        let out = "\n-\t-\tassets/logo.png\n5\t4\tsrc/c.rs\n";
        assert_eq!(numstat_sum(out), 9);
    }

    #[test]
    fn numstat_sum_empty_commit() {
        assert_eq!(numstat_sum(""), 0);
        assert_eq!(numstat_sum("\n\n"), 0);
    }

    // --- red-line path matcher (v5c-reexec gate) ---
    #[test]
    fn redline_matches_core_surfaces() {
        assert!(is_redline("kernel/src/money.rs"));
        assert!(is_redline("kernel/src/order_machine.rs"));
        assert!(is_redline("kernel/src/event_log.rs"));
        assert!(is_redline("apps/api/src/routes/customer/otp.ts"));
        assert!(is_redline("apps/api/src/auth/jwt.rs"));
        // case-insensitive
        assert!(is_redline("Kernel/Src/EVENT_LOG.rs"));
        assert!(is_redline("apps/AUTH/handler.ts"));
    }

    #[test]
    fn redline_ignores_non_core() {
        assert!(!is_redline("kernel/src/geo.rs"));
        assert!(!is_redline("docs/design/plan.md"));
        assert!(!is_redline("tools/ci-truth/src/main.rs"));
        // money without the .rs literal must NOT match (regex was money\.rs)
        assert!(!is_redline("packages/ui/moneyFormat.ts"));
    }

    // --- cargo count parsing (v5c-reexec suite tallies) ---
    #[test]
    fn sum_counts_summed_over_binaries() {
        let log = "test result: ok. 42 passed; 0 failed; 1 ignored\n\
                   test result: ok. 12 passed; 0 failed; 0 ignored\n";
        assert_eq!(sum_counts(log, "passed"), 54);
        assert_eq!(sum_counts(log, "failed"), 0);
    }

    #[test]
    fn sum_counts_with_failures() {
        let log = "test result: FAILED. 40 passed; 2 failed; 0 ignored\n";
        assert_eq!(sum_counts(log, "passed"), 40);
        assert_eq!(sum_counts(log, "failed"), 2);
    }

    // --- failing-test id extraction (v5c-reexec failing_tests) ---
    #[test]
    fn parse_failing_extracts_ids() {
        let log = "running 3 tests\n\
                   test money::tests::tax_overflow ... FAILED\n\
                   test money::tests::rounds_even ... ok\n\
                   test event_log::tests::fail_open ... FAILED\n\
                   test result: FAILED. 1 passed; 2 failed;\n";
        let mut out = Vec::new();
        parse_failing(log, &mut out);
        assert_eq!(
            out,
            vec![
                "money::tests::tax_overflow".to_string(),
                "event_log::tests::fail_open".to_string()
            ]
        );
    }

    #[test]
    fn parse_failing_empty_on_green() {
        let log = "test a ... ok\ntest b ... ok\ntest result: ok. 2 passed; 0 failed;\n";
        let mut out = Vec::new();
        parse_failing(log, &mut out);
        assert!(out.is_empty());
    }

    // --- json_array emission ---
    #[test]
    fn json_array_shapes() {
        assert_eq!(json_array(&[]), "[]");
        assert_eq!(
            json_array(&["kernel/src/money.rs".into(), "kernel/src/auth.rs".into()]),
            "[\"kernel/src/money.rs\",\"kernel/src/auth.rs\"]"
        );
        // stray quotes stripped; empty entries skipped.
        assert_eq!(json_array(&["a\"b".into(), "".into(), "c".into()]), "[\"ab\",\"c\"]");
    }
}
