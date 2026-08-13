//! `kernel::sys_dashboard` — system-wide ASCII status dashboard.
//!
//! Renders the entire dowiz system state as a human-readable ASCII report.
//! Uses all visualization primitives: trinary RGB, fractal ASCII, eigen ASCII,
//! chronos snapshots, delta tracking, ETA oracle.
//!
//! ZERO deps. Call `render()` → String ready for display.

use crate::trinary::{Tri, TriMatrix};
use crate::fractal::ascii_matrix;
use crate::delta::DeltaTracker;
use crate::code_oracle::EtaOracle;
use crate::trig::Xyz;

/// Drift severity bucket — the rewrite-law replacement for a drift if/else chain.
/// A decision is a table index (branchless bool-cast), not an if-chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum DriftBucket {
    Low = 0,
    Med = 1,
    High = 2,
}

impl crate::lut::LutKey for DriftBucket {
    const LUT_SIZE: usize = 3;
    fn discriminant(self) -> u8 {
        self as u8
    }
}

impl DriftBucket {
    /// Branchless bucket from cumulative drift.
    /// `(drift > 100.0) as usize + (drift > 10.0) as usize` yields
    /// 0 = Low (≤10), 1 = Med (10..100], 2 = High (>100) — two bool casts,
    /// one array index, zero branches.
    const BUCKETS: [DriftBucket; 3] = [DriftBucket::Low, DriftBucket::Med, DriftBucket::High];

    #[inline(always)]
    fn from_drift(drift: f64) -> DriftBucket {
        let idx = (drift > 100.0) as usize + (drift > 10.0) as usize;
        Self::BUCKETS[idx]
    }
}

/// Compile-time label table for drift buckets — the decision is data, not code.
const DRIFT_LABEL_LUT: crate::lut::Lut<DriftBucket, &'static str, 3> =
    crate::lut::Lut::new(["✓ LOW", "◈ MED", "⚠ HIGH"]);

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
    let drift_level = DRIFT_LABEL_LUT.get(DriftBucket::from_drift(drift.cumulative_drift));
    out.push_str(&format!(
        "│  Drift:    {:>6.1}  {}                    │\n",
        drift.cumulative_drift, drift_level
    ));

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

/// Render the drift history as a glyph sparkline (Phase C / F rewrite law:
/// numeric series render via pixel glyphs, not raw ASCII bars).
pub fn render_drift_sparkline(drift: &DeltaTracker) -> String {
    let series: Vec<f64> = drift.history.iter().map(|d| d.magnitude).collect();
    crate::glyph_dashboard::render_sparkline(&series)
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

    #[test]
    fn drift_bucket_branchless_agrees_with_thresholds() {
        // The LUT bucket must match the classic threshold semantics.
        assert_eq!(DriftBucket::from_drift(5.0), DriftBucket::Low);
        assert_eq!(DriftBucket::from_drift(10.0), DriftBucket::Low);
        assert_eq!(DriftBucket::from_drift(50.0), DriftBucket::Med);
        assert_eq!(DriftBucket::from_drift(100.0), DriftBucket::Med);
        assert_eq!(DriftBucket::from_drift(100.5), DriftBucket::High);
        assert_eq!(DriftBucket::from_drift(10_000.0), DriftBucket::High);

        assert_eq!(DRIFT_LABEL_LUT.get(DriftBucket::Low), "✓ LOW");
        assert_eq!(DRIFT_LABEL_LUT.get(DriftBucket::Med), "◈ MED");
        assert_eq!(DRIFT_LABEL_LUT.get(DriftBucket::High), "⚠ HIGH");
    }

    #[test]
    fn drift_sparkline_renders_block_glyphs() {
        let mut drift = DeltaTracker::new(100.0, 10.0);
        // Push a few deltas so the history is non-empty.
        let a = vec![0.0, 1.0, 2.0];
        let b = vec![0.1, 1.5, 2.5];
        let d = crate::delta::Delta::between(&a, 0, &b, 1);
        drift.observe(d);
        let s = render_drift_sparkline(&drift);
        assert!(!s.is_empty());
        assert!(s.chars().any(|c| ('\u{2581}'..='\u{2588}').contains(&c)));
    }
}
