//! `markov_attractor` — native-kernel CLI for the self-improvement loop detector.
//!
//! Replaces `tools/loop-signals/markov_attractor.py` (now deleted — the kernel's
//! `markov` module IS the port, with VbM parity tests against the Python's own
//! 12-case corpus). This binary links `dowiz_kernel` directly and emits the exact
//! JSON contract the Python produced (`json.dumps(analyze(toks))`), so `check.sh`
//! and downstream consumers keep working unchanged. In-process, zero interpreter.
//!
//! Usage:
//!   cat events.txt | markov_attractor        # one token per line on stdin
//!   markov_attractor --selftest               # parity self-check vs known verdicts
//!
//! PMU companion (roadmap item 27, classifier-input half): the `analyze_detailed` call is
//! window-bracketed with a before/after [`PmuStation`] snapshot, and the verdict + PMU delta
//! are logged as ONE FDR record (`name: "markov_verdict"`) via `fdr::emit_verdict_pmu`. The
//! stdout JSON contract above is UNCHANGED (the FDR record goes to the ring/stderr, never
//! stdout), and `analyze_detailed` stays a pure function — the PMU data rides alongside the
//! verdict, it is not a classifier input. Durable capture is opt-in via `DOWIZ_FDR_DIR`; with
//! no sink installed the emit is a zero-cost no-op, so `check.sh`'s behavior is unchanged.

use dowiz_kernel::fdr::pmu::PmuStation;
use dowiz_kernel::markov::{analyze_detailed, Verdict};
use std::io::Read;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        selftest();
        return;
    }

    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() {
        // fail-open: never break the hook
        println!("{{\"verdict\": \"HEALTHY\", \"reason\": \"analyzer error\"}}");
        exit(0);
    }
    let toks: Vec<&str> = buf
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    // Opt-in durable FDR capture (roadmap item 27). Default: no sink ⇒ the emit below is a
    // no-op and check.sh sees the identical stdout with zero extra cost.
    // `fdr::init`/`FdrConfig` are native-only (see fdr/mod.rs cfg gates) — this bin is a
    // native CLI and is never meant to build for wasm32, but `cargo build --target
    // wasm32-unknown-unknown` still compiles every `[[bin]]` in the crate by default.
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(dir) = std::env::var("DOWIZ_FDR_DIR") {
        if !dir.is_empty() {
            let _ = dowiz_kernel::fdr::init(dowiz_kernel::fdr::FdrConfig {
                stderr: false,
                ring_dir: Some(std::path::PathBuf::from(dir)),
                ..Default::default()
            });
        }
    }

    // Window-bracket the classification with a before/after PMU snapshot. `analyze_detailed`
    // is called exactly as before (inside the closure); the delta rides alongside the verdict.
    let station = PmuStation::new();
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
    let (r, pmu_delta) = station.bracket(|| analyze_detailed(&toks));

    // Hand-rolled JSON — matches the Python key order exactly (kernel is serde-free).
    let eigs_json: Vec<String> = r
        .eigs
        .iter()
        .map(|(re, im)| format!("[{:.3}, {:.3}]", re, im))
        .collect();
    let stat_json: Vec<String> = r
        .stationary
        .iter()
        .map(|(k, v)| format!("\"{}\": {:.4}", esc(k), v))
        .collect();
    let alpha_json: Vec<String> = r
        .alphabet
        .iter()
        .map(|s| format!("\"{}\"", esc(s)))
        .collect();

    println!(
        "{{\"verdict\": \"{}\", \"reason\": \"{}\", \"events\": {}, \
         \"alphabet\": [{}], \"entropy_rate_bits\": {:.4}, \"escape_mass\": {:.4}, \
         \"drift\": {:.4}, \"has_failure\": {}, \"slem\": {:.4}, \"period\": {}, \
         \"eigs\": [{}], \"stationary\": {{{}}}}}",
        r.verdict_str(),
        esc(&r.reason),
        r.report.events,
        alpha_json.join(", "),
        r.report.entropy_rate_bits,
        r.report.escape_mass,
        r.report.drift,
        r.report.has_failure,
        r.report.slem,
        r.report.period,
        eigs_json.join(", "),
        stat_json.join(", ")
    );

    // Companion FDR record: the verdict string + the window's PMU delta, on ONE record.
    // No-op unless a sink was installed above (DOWIZ_FDR_DIR); never touches stdout.
    // `emit_verdict_pmu` is native-only — see cfg note above.
    #[cfg(not(target_arch = "wasm32"))]
    dowiz_kernel::fdr::emit_verdict_pmu("markov_verdict", r.verdict_str(), pmu_delta);
}

/// Minimal JSON string escape — now the single `fdr::json` authority (roadmap items 4+29
/// absorbed the two coexisting escapers into one). Byte-identical to the deleted local
/// `esc()` body (same match arms, same capacity); golden-pinned in `fdr::json` tests and by
/// this CLI's own `--selftest` output being unchanged.
fn esc(s: &str) -> String {
    dowiz_kernel::fdr::json::escape(s)
}

/// RED→GREEN self-check: the Python's headline verdicts must hold in the kernel.
fn selftest() {
    let healthy_rhythm = analyze_detailed(&rep(&["edit", "run_ok"], 8));
    assert_eq!(healthy_rhythm.report.verdict, Verdict::Healthy);

    let lc = analyze_detailed(&rep(&["edit", "run_fail"], 8));
    assert_eq!(lc.report.verdict, Verdict::LimitCycle);
    assert!(lc.report.period, "2-cycle must show period signal");

    let sa = analyze_detailed(&lcg_walk(&["edit", "edit_fail", "run_fail"], 40, 1));
    assert_eq!(sa.report.verdict, Verdict::StrangeAttractor);
    assert_eq!(sa.report.escape_mass, 0.0);
    assert!(sa.report.has_failure);

    let cold = analyze_detailed(&["edit", "run_fail", "edit"]);
    assert_eq!(cold.report.verdict, Verdict::Healthy);

    println!(
        "SELFTEST PASS: HEALTHY={:?} LIMIT_CYCLE={:?} STRANGE={:?} cold={:?}",
        healthy_rhythm.report.verdict, lc.report.verdict, sa.report.verdict, cold.report.verdict
    );
}

fn rep(p: &[&'static str], times: usize) -> Vec<&'static str> {
    let mut v = Vec::new();
    for _ in 0..times {
        v.extend_from_slice(p);
    }
    v
}

fn lcg_walk(alphabet: &[&'static str], n: usize, seed: u64) -> Vec<&'static str> {
    let mut x = seed;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        x = (1103515245u64.wrapping_mul(x).wrapping_add(12345)) & 0x7FFF_FFFF;
        out.push(alphabet[((x >> 16) as usize) % alphabet.len()]);
    }
    out
}
