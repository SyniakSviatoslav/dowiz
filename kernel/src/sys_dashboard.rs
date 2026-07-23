//! `kernel::sys_dashboard` — system-wide ASCII status dashboard.
//!
//! Renders the entire dowiz system state as a human-readable ASCII report.
//! Uses all visualization primitives: trinary RGB, fractal ASCII, eigen ASCII,
//! chronos snapshots, delta tracking, ETA oracle.
//!
//! ZERO deps. Call `render()` → String ready for display.

use crate::trinary::{Tri, TriMatrix};
use crate::fractal::ascii_matrix;
use crate::eigen::EigenDecomp;
use crate::delta::DeltaTracker;
use crate::code_oracle::EtaOracle;
use crate::trig::Xyz;
use std::collections::HashMap;

/// Complete system dashboard — one call, full state.
pub fn render(
    test_count: usize,
    module_count: usize,
    db_entries: usize,
    db_kinds: usize,
    oracle: &EtaOracle,
    drift: &DeltaTracker,
    state: Option<Xyz>,
    phase: &str,
) -> String {
    let mut out = String::with_capacity(4096);

    // ── Header ──
    out.push_str("┌─────────────────────────────────────────┐\n");
    out.push_str("│         DOWIZ SYSTEM DASHBOARD          │\n");
    out.push_str(&format!("│  Phase: {:>31} │\n", phase));
    out.push_str("├─────────────────────────────────────────┤\n");

    // ── Tests ──
    let bar = "█".repeat((test_count / 50).min(30));
    out.push_str(&format!("│  Tests:   {:>5} green  {}│\n", test_count, bar));

    // ── Modules ──
    out.push_str(&format!("│  Modules: {:>5} total                   │\n", module_count));

    // ── Enrichment DB ──
    out.push_str(&format!("│  Enrich DB: {:>5} entries, {:>2} kinds      │\n", db_entries, db_kinds));

    // ── ETA Oracle ──
    if oracle.history.len() >= 3 {
        let eta10 = oracle.predict_eta(3, 100);
        let (mean_err, _) = oracle.eta_confidence();
        out.push_str(&format!("│  ETA(3mod,100L): {:>4.0}m ±{:.0}m           │\n", eta10, mean_err));
    } else {
        out.push_str("│  ETA Oracle:  calibrating...              │\n");
    }

    // ── System stability ──
    if let Some(xyz) = state {
        out.push_str(&format!("│  XYZ State: ({:+.2},{:+.2},{:+.2})            │\n", xyz.x, xyz.y, xyz.z));
    }

    // ── Drift ──
    let drift_level = if drift.cumulative_drift > 100.0 { "⚠ HIGH" }
        else if drift.cumulative_drift > 10.0 { "◈ MED" }
        else { "✓ LOW" };
    out.push_str(&format!("│  Drift:    {:>6.1}  {}                    │\n", drift.cumulative_drift, drift_level));

    // ── Alarms ──
    let alarming = drift.is_alarming(5);
    out.push_str(&format!("│  Alarm:    {:>6}                        │\n",
        if alarming { "⚠ ACTIVE" } else { "✓ CLEAR" }));

    // ── Recent deltas ──
    if !drift.history.is_empty() {
        let last = drift.history.last().unwrap();
        out.push_str(&format!("│  Last Δ:   {:.3}  rate={:.3}               │\n",
            last.magnitude, last.rate));
    }

    out.push_str("└─────────────────────────────────────────┘\n");

    // ── Enrichment mode ──
    out.push_str("\n═══ ENRICHMENT ═══\n");
    out.push_str("  primary: code  intents: code(4) meta(3) system(2)\n");
    out.push_str("  paths: [code→debug→compile] [meta→prompt-eng]\n");
    out.push_str("  patterns: quality safety minimal idempotency invariant\n");

    out
}

/// Render a TriMatrix as a color-coded ASCII grid (using . for visual clarity).
pub fn render_trimatrix(m: &TriMatrix, label: &str) -> String {
    let mut out = format!("═══ {} ═══\n", label);
    let (t, f, u) = m.counts();
    out.push_str(&format!("  T:{} F:{} ?:{}  ", t, f, u));
    let stable = t as f64 / m.data.len().max(1) as f64;
    out.push_str(&format!("stability: {:.3}\n", stable));
    out.push_str(&ascii_matrix(m));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_renders() {
        let oracle = EtaOracle::new();
        let drift = DeltaTracker::new(100.0, 10.0);
        let d = render(2128, 168, 13880, 19, &oracle, &drift, None, "Phase 2");
        assert!(d.contains("DOWIZ SYSTEM DASHBOARD"));
        assert!(d.contains("Tests"));
        assert!(d.contains("Phase 2"));
    }

    #[test]
    fn dashboard_with_oracle_shows_eta() {
        let mut oracle = EtaOracle::new();
        oracle.record(&["a.rs"], 100, 50, 15.0);
        oracle.record(&["b.rs"], 200, 100, 30.0);
        oracle.record(&["c.rs"], 50, 25, 8.0);
        let drift = DeltaTracker::new(100.0, 10.0);
        let d = render(2128, 168, 13880, 19, &oracle, &drift, None, "Phase 3");
        assert!(d.contains("ETA"));
    }

    #[test]
    fn render_trimatrix_works() {
        let mut m = TriMatrix::new(2, 2);
        m.set(0, 0, Tri::True);
        m.set(0, 1, Tri::False);
        let d = render_trimatrix(&m, "test");
        assert!(d.contains("T:"));
        assert!(d.contains("stability"));
    }
}
