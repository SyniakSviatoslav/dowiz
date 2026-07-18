//! BP-08 `admit()` — deterministic intake compiler for the dowiz kernel.
//!
//! `admit()` runs a three-stage, cheapest-first gate over an [`EtalonSpec`]
//! and either returns a resolved [`Witness`] or a typed [`IntakeError`]:
//!
//!   1. **UNSAT ladder** — Tier A (structural O(n)) → Tier B (AC-3
//!      arc-consistency) → Tier C (SMT QF_LIA/QF_LRA). Tier A and Tier B are
//!      implemented in full using only `std`; Tier C is a documented stub that
//!      FAILS-CLOSED (returns [`IntakeError::Undecidable`]) for any hard /
//!      nonlinear constraint, because no SMT solver dependency is permitted.
//!   2. **Under-determined** — degree-of-freedom / entropy / AllSAT-2 check
//!      (F2). A spec with `dof > 0` (≥ 2 admissible models) is rejected so the
//!      system never silently picks one.
//!   3. **Non-reproducible verify** — (a) static purity denylist scan of the
//!      verify-expression source, then (b) a dynamic idempotence probe
//!      (K ≥ 2 evaluations under a perturbed nuisance environment). Divergence
//!      or a banned token FORCE a human bypass (F3).
//!
//! All checks are deterministic and std-only.

/// Maximum width of a bounded integer range that `admit()` will materialize as
/// a `Vec` during the emptiness probe (Tier A). Specs with a wider range are
/// treated as unbounded for emptiness (a bounded-but-huge range is never empty
/// by structure) — caps memory at O(n·|enum|) and prevents an adversarial
/// `min:0, max:1_000_000_000` from allocating a multi-GB `Vec` (FEYNMAN-09).
const MAX_ENUM_WIDTH: i64 = 4096;
use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// Public surface
// ─────────────────────────────────────────────────────────────────────────────

/// Resolution tier at which a check fired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    A,
    B,
    C,
}

/// Sub-classification of an ill-conditioned feasible region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IllKind {
    /// A singleton admissible value pinned exactly on a constraint endpoint
    /// (zero numeric margin) — fragile to perturbation.
    KnifeEdge,
    /// A feasible region that is real but contains only a tiny number of
    /// models (a "sliver" of the search space).
    FeasibleSliver,
}

/// Deterministic intake failure. Every variant is stable (stable codes) so the
/// decide-law fold can stop-at-first and return at a fixed position.
#[derive(Debug, Clone, PartialEq)]
pub enum IntakeError {
    /// F1 — the spec is unsatisfiable.
    Unsatisfiable {
        tier: Tier,
        rule_or_core: String,
        fields: Vec<String>,
    },
    /// F2 — the spec admits more than one model (under-determined).
    UnderDetermined {
        free_fields: Vec<String>,
        dof: i64,
        entropy: f64,
    },
    /// Ill-conditioned feasible region (knife-edge or feasible sliver).
    IllConditioned { kind: IllKind },
    /// F3 — the verify expression cannot be reproduced deterministically.
    NonReproducibleVerify { source: String },
    /// SMT timeout / nonlinear constraint → route to a human reviewer.
    /// NEVER claim SAT on timeout (fail-closed).
    Undecidable { reason: String },
}

/// A single typed field constraint in an [`EtalonSpec`].
#[derive(Debug, Clone, PartialEq)]
pub struct FieldSpec {
    pub name: String,
    /// Inclusive lower bound (None ⇒ unbounded below).
    pub min: Option<i64>,
    /// Inclusive upper bound (None ⇒ unbounded above).
    pub max: Option<i64>,
    /// Discrete allowed set; if present, admissible values must be in this set.
    pub enum_values: Option<Vec<i64>>,
    /// Field must be present / take a value.
    pub required: bool,
    /// Field is disallowed (must not be bound). Conflicts with `required`.
    pub forbidden: bool,
    /// Forced singleton admissible value (a binding).
    pub pinned: Option<i64>,
}

impl FieldSpec {
    pub fn new(name: impl Into<String>) -> Self {
        FieldSpec {
            name: name.into(),
            min: None,
            max: None,
            enum_values: None,
            required: false,
            forbidden: false,
            pinned: None,
        }
    }
}

/// Binary comparison constraint between two fields (indices into
/// [`EtalonSpec::fields`]), used by the Tier-B AC-3 pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// A cross-field algebraic rule consumed by arc-consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSpec {
    pub a: usize,
    pub b: usize,
    pub op: BinOp,
}

/// The intake specification handed to [`admit`].
///
/// `verify` is the source string of the verification predicate (kept for the
/// static purity audit). `verify_fn` is an optional *actual* evaluator used by
/// the dynamic idempotence probe; it receives a nuisance-environment seed and
/// must return the same bits for every seed when the predicate is pure.
///
/// Note: not `Clone` — `Box<dyn Fn>` is not `Clone`. `admit()` takes the spec
/// by reference, so cloning is unnecessary.
pub struct EtalonSpec {
    pub fields: Vec<FieldSpec>,
    pub rules: Vec<RuleSpec>,
    pub verify: String,
    pub verify_fn: Option<Box<dyn Fn(u64) -> bool>>,
    /// Set when the spec contains hard / nonlinear constraints that only a
    /// full SMT solver (Tier C) could settle. Triggers the documented stub.
    pub nonlinear: bool,
}

impl std::fmt::Debug for EtalonSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EtalonSpec")
            .field("fields", &self.fields)
            .field("rules", &self.rules)
            .field("verify", &self.verify)
            .field("verify_fn", &"<closure>")
            .field("nonlinear", &self.nonlinear)
            .finish()
    }
}

/// A resolved, admissible model produced when the spec is admitted.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Witness {
    pub values: BTreeMap<String, i64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal domain model
// ─────────────────────────────────────────────────────────────────────────────

/// Admissible domain for one field.
/// `None` ⇒ unbounded (infinite); `Some(vec)` ⇒ finite admissible set (sorted,
/// de-duplicated). Empty `Some(vec![])` ⇒ no admissible value.
type Domain = Option<Vec<i64>>;

const SLIVER_MAX: u128 = 8;

/// Banned tokens for the static purity denylist. Matching any substring in the
/// lower-cased verify source forces a human bypass (F3).
const BANNED_TOKENS: &[&str] = &[
    "rand",
    "random",
    "rng",
    "clock",
    "time",
    "now",
    "date",
    "network",
    "fetch",
    "http",
    "https",
    "llm",
    "gpt",
    "openai",
    "mutable",
    "mutex",
    "refcell",
    "cell",
    "volatile",
    "nondet",
    "instant",
    "systemtime",
    "socket",
];

// ─────────────────────────────────────────────────────────────────────────────
// Tier A — structural O(n) unsatisfiability
// ─────────────────────────────────────────────────────────────────────────────

/// Run Tier-A structural checks. Returns `Some(IntakeError::Unsatisfiable)`
/// on the first contradiction found, else `None`.
fn tier_a_unsat(spec: &EtalonSpec) -> Option<IntakeError> {
    for f in &spec.fields {
        // required ∧ forbidden is inherently contradictory.
        if f.required && f.forbidden {
            return Some(IntakeError::Unsatisfiable {
                tier: Tier::A,
                rule_or_core: "required_forbidden".into(),
                fields: vec![f.name.clone()],
            });
        }
        // Empty interval: min > max (e.g. minimum:10, maximum:5).
        if let (Some(lo), Some(hi)) = (f.min, f.max) {
            if lo > hi {
                return Some(IntakeError::Unsatisfiable {
                    tier: Tier::A,
                    rule_or_core: "empty_interval".into(),
                    fields: vec![f.name.clone()],
                });
            }
        }
        // Need a contradiction check only when an actual constraint is present.
        let constrained =
            f.min.is_some() || f.max.is_some() || f.enum_values.is_some() || f.pinned.is_some();
        if !constrained {
            continue;
        }
        // Build the admissible intersection.
        let mut domain: Vec<i64> = Vec::new();
        // Start from range (or the pinned value, or the enum set).
        if let Some(v) = f.pinned {
            domain.push(v);
        } else if let Some(ev) = &f.enum_values {
            domain = ev.clone();
        } else if let (Some(lo), Some(hi)) = (f.min, f.max) {
            // Only enumerate bounded ranges for the emptiness probe. Cap the
            // enumeration width so an adversarial spec (min:0, max:1e9) cannot
            // allocate a multi-GB Vec (FEYNMAN-09): beyond the cap we treat the
            // range as effectively unbounded for emptiness — a bounded-but-huge
            // range is never empty by structure, so this preserves correctness
            // while bounding memory to O(n·|enum|).
            if hi - lo <= MAX_ENUM_WIDTH {
                domain = (lo..=hi).collect();
            } else {
                continue;
            }
        } else {
            // Unbounded range with no enum/pinned cannot be empty by structure.
            continue;
        }
        // Intersect with enum.
        if let Some(ev) = &f.enum_values {
            domain.retain(|v| ev.contains(v));
        }
        // Intersect with range.
        if let Some(lo) = f.min {
            domain.retain(|v| *v >= lo);
        }
        if let Some(hi) = f.max {
            domain.retain(|v| *v <= hi);
        }
        if domain.is_empty() {
            let core = if f.pinned.is_some() {
                "const_notin_range"
            } else if f.enum_values.as_ref().map_or(false, |e| e.is_empty()) {
                "empty_enum"
            } else {
                "empty_enum"
            };
            return Some(IntakeError::Unsatisfiable {
                tier: Tier::A,
                rule_or_core: core.into(),
                fields: vec![f.name.clone()],
            });
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier B — AC-3 arc-consistency over binary rules
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the bounded admissible domain for every field (finite enumeration
/// only where the field is bounded; `None` ⇒ unbounded / skipped in AC-3).
fn bounded_domains(spec: &EtalonSpec) -> Vec<Domain> {
    spec.fields
        .iter()
        .map(|f| {
            if f.forbidden && !f.required {
                return Some(Vec::new()); // excluded field: no value
            }
            if let Some(v) = f.pinned {
                return Some(vec![v]);
            }
            // Enumerate only when both bounds are present (finite) AND the width
            // is within the materialization cap (FEYNMAN-09). A wider range is
            // returned as `None` (unbounded for AC-3) — it is never empty by
            // structure, so skipping it preserves correctness.
            if let (Some(lo), Some(hi)) = (f.min, f.max) {
                if hi - lo <= MAX_ENUM_WIDTH {
                    let mut d: Vec<i64> = (lo..=hi).collect();
                    if let Some(ev) = &f.enum_values {
                        d.retain(|v| ev.contains(v));
                    }
                    Some(d)
                } else {
                    None
                }
            } else {
                None // unbounded — not enumerated here
            }
        })
        .collect()
}

/// Does value `vi` (from domain of `a`) have *some* supporting `vj` in `dj`
/// satisfying `vi op vj`?
fn supported(vi: i64, op: BinOp, dj: &[i64]) -> bool {
    match op {
        BinOp::Eq => dj.contains(&vi),
        BinOp::Ne => dj.iter().any(|&vj| vj != vi),
        BinOp::Lt => dj.iter().any(|&vj| vi < vj),
        BinOp::Le => dj.iter().any(|&vj| vi <= vj),
        BinOp::Gt => dj.iter().any(|&vj| vi > vj),
        BinOp::Ge => dj.iter().any(|&vj| vi >= vj),
    }
}

/// Run AC-3. Returns the revised finite domains (only over bounded fields), or
/// an `Unsatisfiable` error if any domain is wiped out. Unbounded fields
/// involved in a rule are skipped (conservative: AC-3 cannot reason about an
/// infinite domain without an SMT solver — that is Tier C's job).
fn tier_b_ac3(spec: &EtalonSpec, mut domains: Vec<Domain>) -> Result<Vec<Domain>, IntakeError> {
    // Arcs (a,b) for every rule a op b.
    let mut queue: std::collections::VecDeque<(usize, usize, BinOp)> =
        spec.rules.iter().map(|r| (r.a, r.b, r.op)).collect();

    while let Some((i, j, op)) = queue.pop_front() {
        // Skip rules touching an unbounded domain (cannot enumerate).
        let (di, dj) = match (&domains[i], &domains[j]) {
            (Some(a), Some(b)) => (a.clone(), b.clone()),
            _ => continue,
        };
        // Revise domain i.
        let revised: Vec<i64> = di
            .iter()
            .copied()
            .filter(|&vi| supported(vi, op, &dj))
            .collect();
        if revised.len() != di.len() {
            if revised.is_empty() {
                return Err(IntakeError::Unsatisfiable {
                    tier: Tier::B,
                    rule_or_core: "ac3_domain_wipeout".into(),
                    fields: vec![spec.fields[i].name.clone(), spec.fields[j].name.clone()],
                });
            }
            domains[i] = Some(revised);
            // Re-enqueue arcs (k, i) for all rules whose rhs is i.
            for r in &spec.rules {
                if r.b == i {
                    queue.push_back((r.a, r.b, r.op));
                }
            }
        }
    }
    Ok(domains)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier C — SMT stub (documented, fail-closed)
// ─────────────────────────────────────────────────────────────────────────────

/// Tier-C SMT (QF_LIA / QF_LRA) solver stub.
///
/// No external SMT dependency is permitted in this std-only crate, so any hard
/// or nonlinear constraint is routed to a human reviewer. **Fail-closed**: this
/// never reports SAT on a problem it cannot solve.
pub fn tier_c_smt_stub(_spec: &EtalonSpec) -> IntakeError {
    IntakeError::Undecidable {
        reason: "Tier-C SMT (QF_LIA/QF_LRA) solver stub: no external solver dependency is \
                 permitted (std-only kernel). Hard / nonlinear constraints must be routed to a \
                 human reviewer. FAIL-CLOSED — never claims SAT."
            .into(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Under-determined (F2) — DOF / entropy / AllSAT-2
// ─────────────────────────────────────────────────────────────────────────────

/// Given the post-AC-3 finite domains (unbounded ⇒ `None`), classify each
/// field and detect under-determination.
fn check_under_determined(spec: &EtalonSpec, domains: &[Domain]) -> Result<(), IntakeError> {
    let mut free_fields: Vec<String> = Vec::new();
    let mut entropy: f64 = 0.0;
    let mut model_count: u128 = 1;
    let mut any_unbounded = false;

    for (i, f) in spec.fields.iter().enumerate() {
        if f.forbidden && !f.required {
            continue; // excluded field
        }
        match &domains[i] {
            None => {
                // Unbounded admissible set ⇒ free, infinite entropy / models.
                free_fields.push(f.name.clone());
                any_unbounded = true;
                entropy = f64::INFINITY;
            }
            Some(d) if d.len() <= 1 => {
                // Singleton (or empty-excluded) ⇒ binding, 0 DOF, ln(1)=0.
                if !d.is_empty() {
                    model_count = model_count.saturating_mul(d.len() as u128);
                }
            }
            Some(d) => {
                // Finite domain with ≥ 2 values ⇒ free.
                free_fields.push(f.name.clone());
                entropy += (d.len() as f64).ln();
                model_count = model_count.saturating_mul(d.len() as u128);
            }
        }
    }

    let dof = free_fields.len() as i64;
    // Under-determined iff dof > 0 (≥ 1 free field) ⇔ ≥ 2 admissible models
    // (AllSAT-2 would find a distinct second model).
    if dof > 0 {
        return Err(IntakeError::UnderDetermined {
            free_fields,
            dof,
            entropy,
        });
    }

    // Ill-conditioned: a real but tiny feasible region (finite & small).
    if !any_unbounded && model_count >= 2 && model_count <= SLIVER_MAX {
        return Err(IntakeError::IllConditioned {
            kind: IllKind::FeasibleSliver,
        });
    }

    Ok(())
}

/// Detect a knife-edge: a singleton admissible value pinned exactly on a range
/// endpoint with zero margin.
fn check_knife_edge(spec: &EtalonSpec) -> Result<(), IntakeError> {
    for f in &spec.fields {
        if f.forbidden && !f.required {
            continue;
        }
        if let Some(v) = f.pinned {
            if let Some(lo) = f.min {
                if v == lo {
                    return Err(IntakeError::IllConditioned {
                        kind: IllKind::KnifeEdge,
                    });
                }
            }
            if let Some(hi) = f.max {
                if v == hi {
                    return Err(IntakeError::IllConditioned {
                        kind: IllKind::KnifeEdge,
                    });
                }
            }
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Non-reproducible verify (F3)
// ─────────────────────────────────────────────────────────────────────────────

/// (a) static denylist scan, then (b) dynamic idempotence probe (K ≥ 2 evals
/// under a perturbed nuisance seed). Either failure ⇒ `NonReproducibleVerify`.
fn check_verify(spec: &EtalonSpec) -> Result<(), IntakeError> {
    let lower = spec.verify.to_lowercase();
    for tok in BANNED_TOKENS {
        if lower.contains(tok) {
            return Err(IntakeError::NonReproducibleVerify {
                source: spec.verify.clone(),
            });
        }
    }
    // Dynamic idempotence probe: K evals under perturbed nuisance env.
    if let Some(f) = &spec.verify_fn {
        const K: u64 = 3;
        let mut prev: Option<bool> = None;
        for nuis in 0..K {
            let r = f(nuis);
            if let Some(p) = prev {
                if p != r {
                    return Err(IntakeError::NonReproducibleVerify {
                        source: spec.verify.clone(),
                    });
                }
            }
            prev = Some(r);
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Witness synthesis (determinate specs only)
// ─────────────────────────────────────────────────────────────────────────────

/// Pick a representative admissible value for each field and build a [`Witness`].
fn synthesize_witness(spec: &EtalonSpec, domains: &[Domain]) -> Witness {
    let mut values = BTreeMap::new();
    for (i, f) in spec.fields.iter().enumerate() {
        if f.forbidden && !f.required {
            continue;
        }
        let v = if let Some(p) = f.pinned {
            p
        } else if let Some(d) = &domains[i] {
            *d.first().unwrap_or(&0)
        } else {
            0 // unbounded free field ⇒ canonical 0 (only reached if determinate)
        };
        values.insert(f.name.clone(), v);
    }
    Witness { values }
}

// ─────────────────────────────────────────────────────────────────────────────
// admit() — the public intake compiler
// ─────────────────────────────────────────────────────────────────────────────

/// Deterministic intake compiler (BP-08).
///
/// Order of the cheapest-first gate:
/// 1. Tier-A structural UNSAT → Tier-B AC-3 UNSAT → (Tier-C stub if `nonlinear`).
/// 2. Under-determined (F2).
/// 3. Non-reproducible verify (F3).
/// 4. Synthesis of the [`Witness`].
pub fn admit(spec: &EtalonSpec) -> Result<Witness, IntakeError> {
    // 1a. Tier A — structural O(n).
    if let Some(e) = tier_a_unsat(spec) {
        return Err(e);
    }

    // 1b. Tier B — AC-3 arc-consistency over bounded domains.
    let mut domains = bounded_domains(spec);
    domains = tier_b_ac3(spec, domains)?;

    // 1c. Tier C — only for hard / nonlinear constraints (stub, fail-closed).
    if spec.nonlinear {
        return Err(tier_c_smt_stub(spec));
    }

    // 2. Under-determined (F2) — reject ≥ 2 admissible models.
    check_under_determined(spec, &domains)?;

    // 2b. Ill-conditioned knife-edge (zero-margin singleton).
    check_knife_edge(spec)?;

    // 3. Non-reproducible verify (F3) — static + dynamic.
    check_verify(spec)?;

    // 4. Determinate ⇒ synthesize the witness.
    Ok(synthesize_witness(spec, &domains))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — Red→Green gate
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── RED: Tier-A empty interval (minimum:10, maximum:5) ──────────────────
    #[test]
    fn red_unsat_empty_interval() {
        let spec = EtalonSpec {
            fields: vec![FieldSpec {
                name: "amount".into(),
                min: Some(10),
                max: Some(5),
                ..FieldSpec::new("amount")
            }],
            rules: vec![],
            verify: String::new(),
            verify_fn: None,
            nonlinear: false,
        };
        match admit(&spec) {
            Err(IntakeError::Unsatisfiable {
                tier: Tier::A,
                rule_or_core,
                ..
            }) if rule_or_core == "empty_interval" => {}
            other => panic!(
                "expected Tier-A empty_interval Unsatisfiable, got {:?}",
                other
            ),
        }
    }

    // ── RED: under-determined (required field, no constraint) ───────────────
    #[test]
    fn red_under_determined() {
        let spec = EtalonSpec {
            fields: vec![FieldSpec {
                name: "quantity".into(),
                required: true,
                ..FieldSpec::new("quantity")
            }],
            rules: vec![],
            verify: String::new(),
            verify_fn: None,
            nonlinear: false,
        };
        match admit(&spec) {
            Err(IntakeError::UnderDetermined { dof, .. }) if dof > 0 => {}
            other => panic!("expected UnderDetermined with dof>0, got {:?}", other),
        }
    }

    // ── RED: impure verify (banned token) ───────────────────────────────────
    #[test]
    fn red_impure_verify() {
        let spec = EtalonSpec {
            fields: vec![FieldSpec {
                name: "x".into(),
                pinned: Some(1),
                ..FieldSpec::new("x")
            }],
            rules: vec![],
            // banned token "rand"
            verify: "rand()>0.5".into(),
            verify_fn: None,
            nonlinear: false,
        };
        match admit(&spec) {
            Err(IntakeError::NonReproducibleVerify { source }) if source == "rand()>0.5" => {}
            other => panic!("expected NonReproducibleVerify, got {:?}", other),
        }
    }

    // ── GREEN: pure verify (3 identical evals) ──────────────────────────────
    #[test]
    fn green_pure_verify() {
        let spec = EtalonSpec {
            fields: vec![
                FieldSpec {
                    name: "total".into(),
                    pinned: Some(10),
                    ..FieldSpec::new("total")
                },
                FieldSpec {
                    name: "lines".into(),
                    pinned: Some(10),
                    ..FieldSpec::new("lines")
                },
            ],
            rules: vec![],
            // clean token — passes static denylist
            verify: "(total==sum(lines))".into(),
            // pure evaluator: ignores nuisance env, always reproducible
            verify_fn: Some(Box::new(|_nuis: u64| true)),
            nonlinear: false,
        };
        let w = admit(&spec).expect("pure verify spec should admit");
        assert_eq!(w.values.get("total"), Some(&10));
        assert_eq!(w.values.get("lines"), Some(&10));
    }

    // ── Tier-B AC-3 UNSAT demonstration (x<y and y<x over [1,3]) ────────────
    #[test]
    fn ac3_unsat_mutual_order() {
        let spec = EtalonSpec {
            fields: vec![
                FieldSpec {
                    name: "x".into(),
                    min: Some(1),
                    max: Some(3),
                    ..FieldSpec::new("x")
                },
                FieldSpec {
                    name: "y".into(),
                    min: Some(1),
                    max: Some(3),
                    ..FieldSpec::new("y")
                },
            ],
            rules: vec![
                RuleSpec {
                    a: 0,
                    b: 1,
                    op: BinOp::Lt,
                }, // x < y
                RuleSpec {
                    a: 1,
                    b: 0,
                    op: BinOp::Lt,
                }, // y < x
            ],
            verify: String::new(),
            verify_fn: None,
            nonlinear: false,
        };
        match admit(&spec) {
            Err(IntakeError::Unsatisfiable {
                tier: Tier::B,
                rule_or_core,
                ..
            }) if rule_or_core == "ac3_domain_wipeout" => {}
            other => panic!(
                "expected Tier-B ac3_domain_wipeout Unsatisfiable, got {:?}",
                other
            ),
        }
    }

    // ── Tier-C stub fails closed ────────────────────────────────────────────
    #[test]
    fn tier_c_stub_undecidable() {
        let spec = EtalonSpec {
            fields: vec![FieldSpec::new("z")],
            rules: vec![],
            verify: String::new(),
            verify_fn: None,
            nonlinear: true,
        };
        assert!(matches!(
            tier_c_smt_stub(&spec),
            IntakeError::Undecidable { .. }
        ));
        // and admit() routes to it for nonlinear specs
        assert!(matches!(admit(&spec), Err(IntakeError::Undecidable { .. })));
    }

    // ── Determinate admit produces a Witness ────────────────────────────────
    #[test]
    fn determinate_admit_witness() {
        let spec = EtalonSpec {
            fields: vec![
                FieldSpec {
                    name: "a".into(),
                    pinned: Some(2),
                    ..FieldSpec::new("a")
                },
                FieldSpec {
                    name: "b".into(),
                    min: Some(5),
                    max: Some(5),
                    ..FieldSpec::new("b")
                },
            ],
            rules: vec![],
            verify: "(a==2)".into(),
            verify_fn: Some(Box::new(|_| true)),
            nonlinear: false,
        };
        let w = admit(&spec).expect("determinate spec should admit");
        assert_eq!(w.values.get("a"), Some(&2));
        assert_eq!(w.values.get("b"), Some(&5));
    }

    // ── required ∧ forbidden → Tier-A UNSAT ─────────────────────────────────
    #[test]
    fn required_forbidden_unsat() {
        let spec = EtalonSpec {
            fields: vec![FieldSpec {
                name: "f".into(),
                required: true,
                forbidden: true,
                ..FieldSpec::new("f")
            }],
            rules: vec![],
            verify: String::new(),
            verify_fn: None,
            nonlinear: false,
        };
        match admit(&spec) {
            Err(IntakeError::Unsatisfiable {
                tier: Tier::A,
                rule_or_core,
                ..
            }) if rule_or_core == "required_forbidden" => {}
            other => panic!("expected required_forbidden Unsatisfiable, got {:?}", other),
        }
    }
}
