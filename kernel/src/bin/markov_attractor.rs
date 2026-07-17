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
    let toks: Vec<&str> = buf.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let r = analyze_detailed(&toks);

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
    let alpha_json: Vec<String> = r.alphabet.iter().map(|s| format!("\"{}\"", esc(s))).collect();

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
}

/// Minimal JSON string escape (handles the quotes/backslashes the reason text uses).
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            _ => o.push(c),
        }
    }
    o
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
        healthy_rhythm.report.verdict,
        lc.report.verdict,
        sa.report.verdict,
        cold.report.verdict
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
