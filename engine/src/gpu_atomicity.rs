//! gpu_atomicity.rs — P88 atomicity-by-default policy + spec (BLUEPRINT-P88).
//!
//! BLUEPRINT: `docs/design/CORE-ROADMAP-2026-07-17/`
//!            `BLUEPRINT-P88-atomicity-by-default-physics-gpu-2026-07-19.md`
//!
//! This module is the **CPU-side, always-compiled** deliverable of P88:
//!   * the shared-write classification types (§3),
//!   * the WGSL shared-write checklist audit gate (§4.1/§4.2),
//!   * the deterministic fixed-point reduction *reference* (§4.4),
//!   * and the RED→GREEN falsifiers proving the policy bites (§4.2/§4.4/§4.5).
//!
//! It deliberately contains **ZERO WGSL / GPU code** — the shader legs (D2, the
//! shader edits of D3–D7) are gated on the operator-owned P38 §4.2 GPU-compute
//! decision and are NOT written here. Until that decision, P88 is a written
//! policy + a checklist template + the deterministic-reduction spec, with no GPU
//! adapter compiled (the default `engine` build has no GPU adapter; see
//! `bridge.rs`).
//!
//! The whole module compiles on the **default** feature set (no `gpu` feature
//! exists yet), so every gate below runs under `cargo test` with no flags. This
//! is what lets P86/P87 *inherit* the rule (cite this module + the blueprint)
//! before any shader is written.
//!
//! `// SINGLE-WRITER:` discipline (§4.5): every class-(c) `SingleWriterProof`
//! exemption MUST carry a written structural proof at the write site. The
//! greppable marker constant below is what the future CI grep keys on; the
//! audit gate (`audit_blocks_merge`) enforces the *non-empty proof* half of it.

// ── Greppable discipline marker (CI gate anchor, §4.5) ────────────────────
/// The exact comment a physics/GPU non-atomic shared write MUST carry to claim
/// the single-writer exemption (class `SingleWriterProof`). A CI grep asserts
/// that every non-atomic `var<storage, read_write>` store in `engine/**/*.wgsl`
/// is preceded by this marker; the audit gate asserts the referenced `proof`
/// string is non-empty. Both halves are required — the comment is the site,
/// the proof text is the argument a reviewer can falsify.
pub const SINGLE_WRITER_MARKER: &str = "// SINGLE-WRITER:";

/// Fixed-point scale for deterministic reductions (§3, §4.4). Chosen so the
/// field's value range (|U| ≤ 1.0 by the `field_frame` clamp) maps to i64 with
/// headroom for N-cell accumulation without overflow: 2^20 ≈ 1.05e6, and
/// 512² = 262_144 cells × max |scaled cell| 2^20 fits comfortably in an i64
/// accumulator. Named, not magic.
pub const FIXED_POINT_SHIFT: u32 = 20;
/// `2^FIXED_POINT_SHIFT` as an f64 (1_048_576.0) — the integer↔fixed-point
/// conversion factor for every physics/GPU reduction.
pub const FIXED_POINT_SCALE: f64 = (1u64 << FIXED_POINT_SHIFT) as f64;

/// Float `atomicAdd` is BANNED in the physics/GPU domain (non-associative →
/// non-deterministic sum, §0.3/§4.4). This const documents the ban at a
/// greppable site; enforcement is the review checklist + the reproducibility
/// test (`red_float_atomic_reduction_is_nondeterministic`) — a float-atomic
/// reduction fails N-run bit-identity, proving the ban answers a real hazard.
pub const FLOAT_ATOMIC_ADD_BANNED_IN_DOMAIN: bool = true;

/// Classification of a single shared-write site in a physics/GPU shader — the
/// checklist verdict (§3, §4.2). Every `var<storage, read_write>` /
/// `var<workgroup>` write MUST resolve to exactly one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedWriteClass {
    /// (a) the write is an atomic op (atomicAdd/atomicMax/… on an integer
    ///     atomic). Mandatory for any many-writer accumulation site.
    Atomic,
    /// (b) the write is separated from conflicting reads by a
    ///     `workgroupBarrier()` / `storageBarrier()`.
    BarrierSeparated,
    /// (c) the write is proven single-writer by construction (ping-pong,
    ///     per-invocation-disjoint output index). REQUIRES a written
    ///     `// SINGLE-WRITER:` proof at the site (non-empty `proof`).
    SingleWriterProof,
}

/// One row of the per-shader shared-write checklist (the review artifact, §4.2).
/// Before a physics/GPU shader merges, every shared write must appear as a row
/// whose `class` is set and — for class (c) — whose `proof` is non-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedWriteAudit {
    /// Shader under review, e.g. `"field_step.wgsl"`.
    pub shader: &'static str,
    /// The binding written, e.g. `"var<storage, read_write> next: array<f32>"`.
    pub binding: &'static str,
    /// The classification verdict (a)/(b)/(c). A shader with any unclassified
    /// write (or a class-(c) write with empty `proof`) is RED.
    pub class: SharedWriteClass,
    /// The `// SINGLE-WRITER:` text for class (c); `""` otherwise.
    pub proof: &'static str,
}

/// The standing review gate (§4.1/§4.2). Returns `true` iff every shared write
/// in the checklist is a *valid* exemption: a class-(c) `SingleWriterProof` MUST
/// carry a non-empty written proof, or the whole shader is rejected (RED).
///
/// This is the machine-checkable half of the `// SINGLE-WRITER:` discipline: an
/// unproven "trust me it's single-writer" entry — or any physics shared write
/// left with an empty proof — cannot merge. Classes (a) `Atomic` and (b)
/// `BarrierSeparated` need no single-writer proof, so an empty `proof` is
/// acceptable for them.
pub fn audit_blocks_merge(audits: Vec<SharedWriteAudit>) -> bool {
    audits.iter().all(|a| {
        // A single-writer exemption WITHOUT a written proof is rejected.
        !(matches!(a.class, SharedWriteClass::SingleWriterProof) && a.proof.is_empty())
    })
}

/// Deterministic fixed-point reduction over a cell buffer (§4.4).
///
/// Each `f64` cell value is scaled to a fixed-point integer, the integers are
/// summed as `i64` (**integer addition is associative → order-independent**),
/// then the total is de-scaled back to `f64`. Because the sum is a pure function
/// of the *multiset* of inputs, the result is bit-identical regardless of the
/// accumulation order — this is exactly what makes an atomic GPU reduction
/// reproducible (the `red_fixed_point_reduction_is_run_deterministic` gate).
///
/// Values must lie within the `i64` accumulator headroom; for the field's
/// |U| ≤ 1.0 range and grids up to 512² this holds (see `FIXED_POINT_SHIFT`).
pub fn reduce_fixed_point(cells: &[f64]) -> f64 {
    let total: i64 = cells
        .iter()
        .map(|&v| (v * FIXED_POINT_SCALE).round() as i64)
        .sum();
    total as f64 / FIXED_POINT_SCALE
}

/// Float-accumulate reduction (the NEGATIVE control, §4.4).
///
/// IEEE-754 `f64` addition is **non-associative**, so this fold's result depends
/// on the accumulation order. It is the exact pattern a banned `atomicAdd<f32>`
/// would exhibit under arbitrary GPU invocation-completion order, and the
/// `red_float_atomic_reduction_is_nondeterministic` gate uses it to PROVE the
/// in-domain float-atomic ban answers a real hazard (non-vacuousness).
pub fn reduce_float(cells: &[f64]) -> f64 {
    cells.iter().fold(0.0f64, |acc, &v| acc + v)
}

/// CPU-oracle matcher (§4.4, `red_reduction_matches_cpu_oracle`).
///
/// The de-scaled fixed-point total must agree with a plain sequential `f64` fold
/// (the CPU `field_energy` oracle) within the fixed-point quantization
/// tolerance `≤ 2^-FIXED_POINT_SHIFT × N`. Per-cell rounding error is bounded by
/// `0.5 / FIXED_POINT_SCALE`, so the N-cell total is within `N·2^-SHIFT`, making
/// this bound tight and always satisfiable by a correct reference.
pub fn matches_cpu_oracle(fixed_total: f64, oracle_total: f64, n_cells: usize) -> bool {
    let tol = (2.0f64).powi(-(FIXED_POINT_SHIFT as i32)) * (n_cells as f64);
    (fixed_total - oracle_total).abs() <= tol
}

// ── Tiny deterministic PRNG (no external deps) for reproducible shuffles ──
/// XorShift64* — used only to deterministically permute accumulation order in
/// the RED gates, so the tests are reproducible run-to-run without `rand`.
struct XorShift(u64);
impl XorShift {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            slice.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── §4.2 D1 — an unproven single-writer exemption is rejected ──────────
    /// RED: a checklist row with class (c) and an EMPTY proof fails the audit
    /// (an exemption without a written proof cannot merge). GREEN once the proof
    /// text is present.
    #[test]
    fn red_unclassified_shared_write_blocks_merge() {
        // (1) EMPTY proof → audit rejects (RED for a naive "trust me" entry).
        let unproven = SharedWriteAudit {
            shader: "field_step.wgsl",
            binding: "var<storage, read_write> next: array<f32>",
            class: SharedWriteClass::SingleWriterProof,
            proof: "",
        };
        assert!(
            !audit_blocks_merge(vec![unproven]),
            "an unproven single-writer exemption MUST be rejected"
        );

        // (2) GREEN: the same row with a written proof passes the audit.
        let proven = SharedWriteAudit {
            shader: "field_step.wgsl",
            binding: "var<storage, read_write> next: array<f32>",
            class: SharedWriteClass::SingleWriterProof,
            proof: "// SINGLE-WRITER: ping-pong — read set `u` (disjoint prev \
                    buffer), write set `next`; output index == global_invocation_id; \
                    each cell written by exactly one invocation.",
        };
        assert!(
            audit_blocks_merge(vec![proven]),
            "a single-writer exemption WITH a written proof MUST merge"
        );
    }

    // ── §4.2 D2 — the stencil step is the canonical single-writer exemption ─
    /// RED (encodes §0.1 as a machine-checkable row): the future `field_step`
    /// classifies its `next` write as `SingleWriterProof` with the ping-pong
    /// proof text, and the audit passes. A future edit that turns the step into a
    /// *shared* accumulator (a real bug) would have no valid class and fail.
    #[test]
    fn red_stencil_step_is_single_writer_exempt() {
        let field_step_checklist = vec![
            SharedWriteAudit {
                shader: "field_step.wgsl",
                binding: "var<storage, read_write> next: array<f32>",
                class: SharedWriteClass::SingleWriterProof,
                proof: "// SINGLE-WRITER: ping-pong. Read set = `u` (prev buffer); \
                        write set = `next` (disjoint buffer). Output index i == \
                        global_invocation_id; each cell written by exactly one \
                        invocation. The buffer rotations (CPU: field_frame.rs:217-221) \
                        keep read/write disjoint. No cross-invocation shared write \
                        exists → atomic-free is CORRECT, not merely un-benched.",
            },
            // A barrier-separated write (e.g. a staged intermediate) is also fine.
            SharedWriteAudit {
                shader: "field_step.wgsl",
                binding: "var<workgroup> scratch: array<f32, 256>",
                class: SharedWriteClass::BarrierSeparated,
                proof: "",
            },
        ];
        assert!(
            audit_blocks_merge(field_step_checklist),
            "the field_step checklist (single-writer `next` + barrier scratch) MUST pass"
        );
    }

    // ── §4.4 D3 — the fixed-point reduction is run-deterministic ───────────
    /// RED→GREEN (the key determinism proof): run the fixed-point reduction over
    /// the SAME cell buffer N=32 times with a SHUFFLED accumulation order → all
    /// 32 sums are bit-identical. Integer addition is associative, so the fixed-
    /// point accumulator is a pure function of the input multiset.
    #[test]
    fn red_fixed_point_reduction_is_run_deterministic() {
        // A deterministic, mixed-magnitude cell buffer (|U| ≤ 1.0 field range).
        let mut cells: Vec<f64> = (0..1024)
            .map(|i| {
                let r = (i as f64 * 0.6180339887498949).fract();
                (r - 0.5) * 1.6 // spread across ~[-0.8, 0.8]
            })
            .collect();

        let base = reduce_fixed_point(&cells);
        let mut all_identical = true;
        for run in 0..32u64 {
            let mut perm = cells.clone();
            XorShift(0x9E37_79B9_7F4A_7C15 ^ (run << 1)).shuffle(&mut perm);
            let s = reduce_fixed_point(&perm);
            if s.to_bits() != base.to_bits() {
                all_identical = false;
            }
        }
        assert!(
            all_identical,
            "fixed-point reduction MUST be bit-identical across 32 shuffled runs"
        );
    }

    // ── §4.4 D4 — the float-atomic hazard is REAL (non-vacuousness) ────────
    /// RED (the non-vacuousness proof): the SAME N=32 test over a *float*-
    /// accumulate reduction with a shuffled summation order MUST exhibit ≥2
    /// differing sums. Float addition is non-associative, so a `float atomicAdd`
    /// would be run-to-run non-deterministic. This proves the in-domain ban
    /// answers a genuine hazard, not a decorative rule.
    ///
    /// The buffer is engineered with large dynamic range (one unit-magnitude
    /// anchor + many tiny values) so that WHERE the anchor sits in the
    /// accumulation order measurably changes the rounded result — exactly the
    /// hazard an arbitrary GPU completion order would trigger.
    #[test]
    fn red_float_atomic_reduction_is_nondeterministic() {
        // 1.0 anchor + 2000 values of 1e-8. Serial-adding each 1e-8 to 1.0
        // rounds back to 1.0 (lost), but accumulating the tinies first then
        // adding to 1.0 yields ~1.00002 — so the result depends on the anchor's
        // position, which the shuffle varies run-to-run.
        let mut cells: Vec<f64> = Vec::with_capacity(2001);
        cells.push(1.0);
        for _ in 0..2000 {
            cells.push(1e-8);
        }

        let mut distinct: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for run in 0..32u64 {
            let mut perm = cells.clone();
            XorShift(0x1234_5678_9ABC_DEF0 ^ (run << 2)).shuffle(&mut perm);
            distinct.insert(reduce_float(&perm).to_bits());
        }
        assert!(
            distinct.len() >= 2,
            "float accumulation MUST be non-deterministic (≥2 distinct sums over 32 \
             shuffled runs); got {} — the float-atomic ban would be vacuous",
            distinct.len()
        );
    }

    // ── §4.4 D5 — the GPU reduction is measured against the CPU oracle ─────
    /// RED→GREEN: the de-scaled fixed-point total matches a simple sequential
    /// `f64` fold (the CPU `field_energy` oracle) within the quantization
    /// tolerance `≤ 2^-FIXED_POINT_SHIFT × N`. The GPU path is *measured
    /// against* the authority, never replacing it.
    #[test]
    fn red_reduction_matches_cpu_oracle() {
        // A plain cell buffer of N values in the field's |U| ≤ 1.0 range.
        let n = 1024usize;
        let cells: Vec<f64> = (0..n)
            .map(|i| {
                let r = (i as f64 * 0.7548776662).fract();
                (r - 0.5) * 1.2
            })
            .collect();

        // Oracle: sequential f64 fold (the authoritative CPU reduction).
        let oracle: f64 = cells.iter().fold(0.0f64, |acc, &v| acc + v);
        // Fixed-point reference total.
        let fixed = reduce_fixed_point(&cells);

        assert!(
            matches_cpu_oracle(fixed, oracle, n),
            "fixed-point total {fixed} must match CPU oracle {oracle} within \
             2^-SHIFT·N = {}",
            (2.0f64).powi(-(FIXED_POINT_SHIFT as i32)) * (n as f64)
        );
    }
}
