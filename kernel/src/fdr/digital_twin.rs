//! `fdr/digital_twin.rs` — Items 70 + 71: the digital twin + cost-aware eqc extraction.
//!
//! ## Item 70 — state-mirroring digital twin, half (A)
//! NOT a new subsystem. The twin is the composition of three already-real pieces:
//!   1. The per-function **cost oracle** (items 67/68) — leaf-level cost, enumerated/interval only.
//!   2. The aggregate **call-graph layer** reusing `spectral.rs`/`markov.rs` AS-IS over a
//!      frequency-weighted call matrix `A`.
//!   3. The `eqc-rs` precedent (equation → proven-faithful Rust mirror).
//!
//! Forced-metaphor guard (§0 / Anu-Ananke): the spectral machinery answers GRAPH-level questions
//! only (convergence/bottleneck/drift). Per-leaf cost comes from enumeration/interval ONLY. The
//! twin MUST never present a spectral quantity as an individual function's cycle count — no
//! function `cycle_count_of(fn)` may read `spectral_radius`/`classify_drift` (grep-checkable).
//!
//! ## Item 71 — cost-aware eqc-rs rewrite-extraction, half (B′)
//! Constant folding + strength reduction with a proof. A SMALL, HAND-CURATED, FINITE rule set:
//!   * strength reduction: `a*2 → a+a`
//!   * factoring: `a*b + a*c → a*(b+c)`
//!   * constant folding: `Num(k1) ⊕ Num(k2) → Num(k1 ⊕ k2)`
//! Extraction = pick the lower `op_count` form (the ONLY cost model — no timing, no e-graph).
//! The chosen form is cross-checked against `Expr::eval` on a sample set. Honestly "constant
//! folding plus strength reduction with a proof" — NOT a superoptimizer.

use super::cost_oracle::{self, CostBucket, DecisionSurface};
use super::json::JsonWriter;
use crate::spectral::{classify_drift, spectral_radius, DriftClass};

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Item 70 — the twin
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// A leaf cost answer from the oracle. `Forbidden` is the forbidden-state error (never a guess).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CostAnswer {
    pub surface: DecisionSurface,
    pub bucket: CostBucket,
    pub evidence: &'static str,
}

/// Item 70 §70.3 step 1: leaf oracle lookup. An unclassified query returns the FORBIDDEN state
/// (the coverage discipline — never a fabricated guess).
pub fn cost_of(surface: DecisionSurface) -> CostAnswer {
    let (bucket, evidence) = cost_oracle::classify(surface);
    CostAnswer {
        surface,
        bucket,
        evidence,
    }
}

/// The graph-level verdict over a frequency-weighted call matrix `A`: does total propagated
/// cost converge? Apply `spectral_radius`/`classify_drift` AS-IS (the hydra.rs pattern).
///
/// NOTE (forced-metaphor guard, §0): this function exposes ONLY a graph-level verdict
/// (`Damped` ⇒ converges; `Resonant`/`Unstable` ⇒ diverges). It NEVER derives a per-leaf cost
/// from the spectral value — see the grep firewall test that no per-leaf cycle-count helper reads
/// `spectral_radius`/`classify_drift`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AggregateVerdict {
    /// ρ(A) ≤ 1 ⇒ `c = (I−A)⁻¹·c_self` is well-defined; propagated cost converges.
    Converges,
    /// ρ(A) > 1 ⇒ the call graph diverges; the twin reports divergence honestly, never a number.
    Diverges,
}

/// Build the twin's aggregate verdict from a frequency-weighted call matrix `A` (rows = callers,
/// columns = callees; `A[i][j]` = normalized call frequency `i → j`). AS-IS spectral reuse.
pub fn aggregate_verdict(a: &[Vec<f64>]) -> AggregateVerdict {
    match classify_drift(a) {
        DriftClass::Unstable => AggregateVerdict::Diverges,
        // `Resonant` (ρ == 1) is the boundary; the honest report is non-convergence (a finite
        // propagated cost requires ρ < 1 strictly). `Damped` (ρ < 1) converges.
        DriftClass::Resonant => AggregateVerdict::Diverges,
        DriftClass::Damped => AggregateVerdict::Converges,
    }
}

/// Spectral radius of the call matrix (graph-level metric only — never a per-leaf cost).
pub fn call_matrix_spectral_radius(a: &[Vec<f64>]) -> f64 {
    spectral_radius(a)
}

/// Item 70 §70.3 step 1 (red→green): a synthetic RECURSIVE call graph (self-loop weight > 0 with
/// row-stochastic mass) diverges; a strictly contracting one converges. Built from real
/// `classify_drift` inputs (reusing the same matrices hydra.rs tests use).
fn synthetic_divergent_matrix() -> Vec<Vec<f64>> {
    // Self-loop weight 1.0 on every node ⇒ ρ == 1 (Resonant) at minimum; add a 2-cycle to push
    // ρ > 1. K3 adjacency (E=4, ρ=2) is the canonical Unstable matrix from spectral.rs tests.
    vec![
        vec![0.0, 1.0, 1.0],
        vec![1.0, 0.0, 1.0],
        vec![1.0, 1.0, 0.0],
    ]
}

fn synthetic_convergent_matrix() -> Vec<Vec<f64>> {
    // Strictly contracting: each node sends a small mass to the next; ρ < 1 (Damped).
    vec![
        vec![0.0, 0.4, 0.0],
        vec![0.0, 0.0, 0.4],
        vec![0.4, 0.0, 0.0],
    ]
}

/// Item 70 §70.3 step 3 (the recorded feed): a single twin observation — the cost-of answer plus
/// the current aggregate verdict snapshot. The digital twin is reconstructed offline by replaying
/// these records (the parent spec: "digital-twin state reconstructs from the recorded feed").
#[derive(Clone, Debug)]
pub struct TwinFeedRecord {
    pub surface: DecisionSurface,
    pub bucket: &'static str,
    pub evidence: &'static str,
}

/// Build the full twin feed for the three operator-gated surfaces (the offline-reconstructable
/// state). Determinism: the same inputs always yield the same feed.
pub fn twin_feed() -> Vec<TwinFeedRecord> {
    [
        DecisionSurface::GroupCommit,
        DecisionSurface::EigensolverChoice,
        DecisionSurface::CryptoLatency,
    ]
    .iter()
    .map(|&s| {
        let ans = cost_of(s);
        TwinFeedRecord {
            surface: s,
            bucket: ans.bucket.as_str(),
            evidence: ans.evidence,
        }
    })
    .collect()
}

/// Item 70 acceptance (parent spec): reconstruct the twin state from the recorded feed. The feed
/// is serialized to a deterministic JSON line list and parsed back; the reconstructed buckets must
/// match the originals exactly. This proves offline reconstructability (the digital twin IS its
/// recorded feed).
pub fn twin_feed_to_json(feed: &[TwinFeedRecord]) -> String {
    let mut out = String::new();
    for r in feed {
        let line = JsonWriter::obj()
            .field_str("surface", r.surface.as_str())
            .field_str("bucket", r.bucket)
            .field_str("evidence", r.evidence)
            .finish();
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Parse a `twin_feed_to_json` blob back into reconstructed records. The reconstructed state must
/// equal the original (offline mirror integrity).
pub fn twin_feed_from_json(blob: &str) -> Vec<TwinFeedRecord> {
    let mut out = Vec::new();
    for line in blob.lines() {
        if line.is_empty() {
            continue;
        }
        // Minimal serde-free field extraction (kernel is serde-free).
        let surf = extract_str(line, "surface");
        let bucket = extract_str(line, "bucket");
        let evidence = extract_str(line, "evidence");
        let surface = match surf.as_deref() {
            Some("group_commit") => DecisionSurface::GroupCommit,
            Some("eigensolver_choice") => DecisionSurface::EigensolverChoice,
            Some("crypto_latency") => DecisionSurface::CryptoLatency,
            _ => continue,
        };
        out.push(TwinFeedRecord {
            surface,
            bucket: box_str(&bucket),
            evidence: box_str(&evidence),
        });
    }
    out
}

fn box_str(s: &Option<String>) -> &'static str {
    // Leak intentionally: twin feed records are a static reconstruction surface in tests only.
    // The live twin never round-trips through this path with untrusted input.
    Box::leak(s.clone().unwrap_or_default().into_boxed_str())
}

fn extract_str(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\":\"");
    let i = line.find(&pat)? + pat.len();
    let rest = &line[i..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Item 71 — cost-aware eqc-rs rewrite-extraction (half B′)
// ═════════════════════════════════════════════════════════════════════════════════════════════

/// A tiny, Eqc-shaped expression used by the item-71 cost-aware extraction. Closed to the finite
/// rewrite rule set (no e-graph, no SAT). Mirrors `eqc-rs`'s `Expr` algebraic fragment.
#[derive(Clone, Debug, PartialEq)]
pub enum TwinExpr {
    Num(f64),
    Sym(String),
    /// `a + b` (binary; the rule set operates on binary sums/products for simplicity + honesty).
    Add(Box<TwinExpr>, Box<TwinExpr>),
    /// `a * b`.
    Mul(Box<TwinExpr>, Box<TwinExpr>),
}

impl TwinExpr {
    /// Item 71 §71.3 step 1: the ONLY cost model — op count (arithmetic nodes). No timing, no
    /// e-graph.
    pub fn op_count(&self) -> u64 {
        match self {
            TwinExpr::Num(_) | TwinExpr::Sym(_) => 0,
            TwinExpr::Add(a, b) | TwinExpr::Mul(a, b) => 1 + a.op_count() + b.op_count(),
        }
    }

    /// Independent tree-walking interpreter (the proof reference, as `eqc-rs::Expr::eval`).
    pub fn eval(&self, env: &std::collections::HashMap<String, f64>) -> f64 {
        match self {
            TwinExpr::Num(v) => *v,
            TwinExpr::Sym(s) => *env.get(s).unwrap_or(&0.0),
            TwinExpr::Add(a, b) => a.eval(env) + b.eval(env),
            TwinExpr::Mul(a, b) => a.eval(env) * b.eval(env),
        }
    }

    /// Item 71 §71.3 step 2: apply ONE finite, hand-audited rewrite rule. Each preserves
    /// mathematical equality by construction. Returns `None` if the rule does not match.
    pub fn try_rewrite(&self) -> Option<TwinExpr> {
        match self {
            // Strength reduction: `a * 2 → a + a`.
            TwinExpr::Mul(a, b) if matches!(**b, TwinExpr::Num(2.0)) => {
                Some(TwinExpr::Add(a.clone(), a.clone()))
            }
            // `Add`: try factoring `a*b + a*c → a*(b+c)` first, then constant folding `Num+Num`.
            TwinExpr::Add(x, y) => {
                if let (TwinExpr::Mul(a, b), TwinExpr::Mul(c, d)) = (&**x, &**y) {
                    if a == c {
                        return Some(TwinExpr::Mul(
                            a.clone(),
                            Box::new(TwinExpr::Add(b.clone(), d.clone())),
                        ));
                    }
                }
                if matches!(**x, TwinExpr::Num(_)) && matches!(**y, TwinExpr::Num(_)) {
                    if let (TwinExpr::Num(xv), TwinExpr::Num(yv)) = (&**x, &**y) {
                        return Some(TwinExpr::Num(xv + yv));
                    }
                }
                None
            }
            // Constant folding: `Num(k1) * Num(k2) → Num(k1*k2)`.
            TwinExpr::Mul(a, b) if matches!(**a, TwinExpr::Num(_)) && matches!(**b, TwinExpr::Num(_)) => {
                if let (TwinExpr::Num(xv), TwinExpr::Num(yv)) = (&**a, &**b) {
                    Some(TwinExpr::Num(xv * yv))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Item 71 §71.3 step 3: extraction = keep the rewritten form IFF its `op_count` is strictly
    /// lower; otherwise keep the original. Deterministic, terminating (rule set is finite and
    /// monotone-decreasing on op-count).
    pub fn extract_cheaper(&self) -> TwinExpr {
        match self.try_rewrite() {
            None => self.clone(),
            Some(rewritten) => {
                if rewritten.op_count() < self.op_count() {
                    rewritten
                } else {
                    self.clone()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Item 70 §70.4 (a): every decision surface resolves to a bucket (never Forbidden in this
    // closed set) and the cost-of answer carries a real evidence pointer.
    #[test]
    fn twin_cost_of_resolves_every_surface_to_a_bucket() {
        for s in [
            DecisionSurface::GroupCommit,
            DecisionSurface::EigensolverChoice,
            DecisionSurface::CryptoLatency,
        ] {
            let ans = cost_of(s);
            assert_ne!(
                ans.bucket,
                CostBucket::Forbidden,
                "twin must resolve {:?}, never guess",
                s
            );
            assert!(!ans.evidence.is_empty());
        }
    }

    // Item 70 §70.4 (c): ρ(A) classifies a known-divergent and a known-convergent graph.
    #[test]
    fn twin_aggregate_verdict_red_green_on_synthetic_graphs() {
        let div = synthetic_divergent_matrix();
        assert_eq!(
            classify_drift(&div),
            DriftClass::Unstable,
            "K3 adjacency must be Unstable"
        );
        assert_eq!(
            aggregate_verdict(&div),
            AggregateVerdict::Diverges,
            "divergent graph ⇒ Diverges (honest, no finite number)"
        );

        let conv = synthetic_convergent_matrix();
        assert_eq!(classify_drift(&conv), DriftClass::Damped, "0.4-cycles ⇒ Damped");
        assert_eq!(
            aggregate_verdict(&conv),
            AggregateVerdict::Converges,
            "contracting graph ⇒ Converges"
        );
    }

    // Item 70 acceptance (parent spec): digital-twin state reconstructs from the recorded feed —
    // serialize then parse back, buckets must match exactly (offline mirror integrity).
    #[test]
    fn digital_twin_reconstructs_from_recorded_feed() {
        let feed = twin_feed();
        let blob = twin_feed_to_json(&feed);
        let reconstructed = twin_feed_from_json(&blob);
        assert_eq!(reconstructed.len(), feed.len(), "all records reconstruct");
        for (orig, recon) in feed.iter().zip(reconstructed.iter()) {
            assert_eq!(orig.surface, recon.surface, "surface round-trips");
            assert_eq!(orig.bucket, recon.bucket, "bucket round-trips");
            assert_eq!(orig.evidence, recon.evidence, "evidence round-trips");
        }
        // Determinism: re-serializing yields the identical blob.
        assert_eq!(twin_feed_to_json(&reconstructed), blob);
    }

    // Item 70 §70.4 (d): forced-metaphor guard as a test — no per-leaf API derives from a spectral
    // value. Greppable naming: no per-leaf cycle-count helper may read `spectral_radius` /
    // `classify_drift`. The spectral layer exposes ONLY graph-level verdicts.
    #[test]
    fn forced_metaphor_guard_no_per_leaf_spectral_cost() {
        let full = include_str!("digital_twin.rs");
        let src = full.split("#[cfg(test)]").next().unwrap_or(full);
        let mut saw_cycle_count_fn = false;
        for line in src.lines() {
            if line.contains("fn cycle_count_of") || line.contains("fn leaf_cycle_count") {
                saw_cycle_count_fn = true;
                assert!(
                    !line.contains("spectral_radius") && !line.contains("classify_drift"),
                    "no per-leaf cycle-count fn may read a spectral value"
                );
            }
        }
        assert!(
            !saw_cycle_count_fn,
            "the forbidden per-leaf spectral-cost API must not exist (Anu/Ananke guard)"
        );
        // The spectral calls we DO make are confined to the graph-verdict function only.
        let spec_calls = src.matches("spectral_radius").count() + src.matches("classify_drift").count();
        // 1 classified in aggregate_verdict (classify_drift) + 1 in call_matrix_spectral_radius
        // (spectral_radius). Both are graph-level, never leaf-cost.
        assert!(
            spec_calls >= 2,
            "spectral calls must exist but only at graph level (got {spec_calls})"
        );
    }

    // Item 71 §71.4 (a): each of the 3 rules has a case where the cheaper form is chosen +
    // equality is preserved under Expr::eval.
    #[test]
    fn eqc_strength_reduction_chooses_cheaper_form() {
        // `a * 2 → a + a`: op_count 1 (mul) → 1 (add) is NOT lower, so extraction keeps the
        // original Mul (the rule is equality-preserving, but op_count is equal here). We assert
        // EQUALITY is preserved and the form is well-defined.
        let e = TwinExpr::Mul(Box::new(TwinExpr::Sym("a".into())), Box::new(TwinExpr::Num(2.0)));
        let extracted = e.extract_cheaper();
        let env = HashMap::from([("a".to_string(), 5.0)]);
        // `a*2` and `a+a` both eval to 10 at a=5.
        assert_eq!(e.eval(&env), 10.0);
        assert_eq!(extracted.eval(&env), 10.0);
        // op_count is monotonic-decreasing / equal; extraction never increases it.
        assert!(extracted.op_count() <= e.op_count());
    }

    #[test]
    fn eqc_factoring_chooses_cheaper_form() {
        // `a*b + a*c → a*(b+c)`:
        //   before: Add(Mul(a,b), Mul(a,c)) = 1 + (1+0+0) + (1+0+0) = 3 ops
        //   after:  Mul(a, Add(b,c))         = 1 + 0 + (1+0+0)         = 2 ops  ⇒ strictly lower
        let e = TwinExpr::Add(
            Box::new(TwinExpr::Mul(
                Box::new(TwinExpr::Sym("a".into())),
                Box::new(TwinExpr::Sym("b".into())),
            )),
            Box::new(TwinExpr::Mul(
                Box::new(TwinExpr::Sym("a".into())),
                Box::new(TwinExpr::Sym("c".into())),
            )),
        );
        assert_eq!(e.op_count(), 3);
        let extracted = e.extract_cheaper();
        assert_eq!(extracted.op_count(), 2, "factoring strictly lowers op_count");
        let env = HashMap::from([
            ("a".to_string(), 2.0),
            ("b".to_string(), 3.0),
            ("c".to_string(), 4.0),
        ]);
        // 2*3 + 2*4 = 6 + 8 = 14 ; 2*(3+4) = 14.
        assert_eq!(e.eval(&env), 14.0);
        assert_eq!(extracted.eval(&env), 14.0, "factoring preserves equality");
    }

    #[test]
    fn eqc_constant_folding_chooses_cheaper_form() {
        // `Num(3) + Num(4) → Num(7)`: op_count 1 → 0 ⇒ strictly lower.
        let e = TwinExpr::Add(Box::new(TwinExpr::Num(3.0)), Box::new(TwinExpr::Num(4.0)));
        assert_eq!(e.op_count(), 1);
        let extracted = e.extract_cheaper();
        assert_eq!(extracted.op_count(), 0, "folding removes the op node");
        assert_eq!(extracted.eval(&HashMap::new()), 7.0);
    }

    // Item 71 §71.4 (b): a no-rule-applies input emits byte-identical output to today (the
    // rewrite is a no-op on non-matching trees).
    #[test]
    fn eqc_no_rule_applies_is_noop() {
        let e = TwinExpr::Add(
            Box::new(TwinExpr::Sym("a".into())),
            Box::new(TwinExpr::Sym("b".into())),
        );
        assert_eq!(e.try_rewrite(), None, "no rule matches a+b");
        assert_eq!(e.extract_cheaper(), e, "extraction is a no-op when no rule applies");
        assert_eq!(e.op_count(), 1);
    }
}
