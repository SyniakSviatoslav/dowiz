//! agent-governance-wasm — Phase-1 WASM binding for the dowiz agent-governance layer.
//!
//! This crate replaces the deleted TypeScript `agent-governance/*.ts` (resonator.ts removed,
//! 0 refs) with a thin Rust/WASM host around the from-scratch, zero-dep [`bebop2_core`]
//! kernels. Every heavy primitive (spectral eigen-solvers, resonator loop, basis projections,
//! belief entropy, hashing) lives in `bebop2-core`; this crate only marshals JS↔Rust values via
//! `wasm-bindgen`.
//!
//! Blueprint kernel coverage (BP-01 .. BP-06):
//!
//! | Kernel            | Status | Wrapper                | bebop2-core source                |
//! |-------------------|--------|------------------------|-----------------------------------|
//! | BP-01 resonator   | ✅     | [`resonate`]           | `resonator::run_resonator`        |
//! | BP-02 geodesic    | ✅     | [`geodesic`]           | `algebra::geodesic_distance`      |
//! | BP-03 eigensolver | ✅     | [`spectral_radius`]    | `lyapunov::eigenvalues_general`   |
//! | BP-04 diffusion   | ✅*    | [`entropy`]            | `active::entropy` (belief/entropy)|
//! | BP-05 PID governor| ⚠️ GAP | —                      | not in bebop2-core (see below)    |
//! | BP-06 ledger      | ✅     | [`ledger_step`]        | `hash::sha3_256` (append-only)    |
//!
//! \* BP-04 "diffusion" is surfaced here as the belief-entropy primitive that the diffusion /
//! free-energy kernel is built on (`active::entropy`). The full CSR-Laplacian matvec /
//! `field::propagate` spectral propagator is available in `bebop2-core` but is intentionally NOT
//! wrapped in this thin shim (it requires constructing a `LaplacianSpectrum` from an edge list,
//! which is a graph-construction concern, not a single-call kernel).
//!
//! ## BP-05 gap (documented, not faked)
//!
//! The autonomous PID + ICIR telemetry governor (BP-05) does **not** exist as a `bebop2-core`
//! kernel. It lives in the separate host-agent crate `crates/bebop/src/governor.rs` (the `bebop`
//! agent), which is NOT part of the `bebop2-core` dependency graph this crate links against.
//! Wrapping it would require depending on that host crate, which is out of scope for a
//! bebop2-core-only WASM binding. [`governor_step`] is therefore intentionally NOT exported.
//! When the governor is moved into `bebop2-core` (or exposed via a host-crate FFI), this crate
//! can add a one-line `#[wasm_bindgen]` wrapper. No placeholder/fake governor is emitted to
//! avoid presenting unverified control logic to callers.
//!
//! ## BP-06 ledger
//!
//! There is no dedicated "entropy ledger" struct in `bebop2-core`, but the building block exists:
//! [`bebop2_core::hash::sha3_256`]. [`ledger_step`] implements a genuine deterministic,
//! append-only, hash-chained ledger entry (the standard collision-resistant accumulator) using
//! the real bebop2-core SHA3-256 — no RNG, no clock, reproducible.

use wasm_bindgen::prelude::*;

// ── BP-01: resonator ─────────────────────────────────────────────────────────
//
// `run_resonator` takes its per-tick actors as bare `fn` pointers (no captures). To let the
// caller-supplied `coeff` (per-tick relaxation toward the ground) flow into the `generate` fn,
// we stage it in a `thread_local` cell. Single-threaded on wasm32 and on the native test runner;
// this shim never re-enters `run_resonator`, so there is no aliasing hazard.
use std::cell::RefCell;

thread_local! {
    static RELAX_COEFF: RefCell<f64> = const { RefCell::new(0.9) };
}

fn resonate_generate(s: &Vec<f64>) -> Vec<f64> {
    let c = RELAX_COEFF.with(|c| *c.borrow());
    s.iter().map(|x| x * c).collect()
}

fn resonate_reflect(proposed: &Vec<f64>, refv: &Vec<f64>) -> (Vec<f64>, f64) {
    // Strong, high-quality reflector: pull 99.999% of the way to the reference each tick.
    let refined: Vec<f64> = proposed
        .iter()
        .zip(refv.iter())
        .map(|(p, r)| p + 0.99999 * (r - p))
        .collect();
    let err = refined
        .iter()
        .zip(refv.iter())
        .map(|(p, r)| (p - r).powi(2))
        .sum::<f64>()
        .sqrt();
    (refined, (1.0 / (1.0 + err)).clamp(0.0, 1.0))
}

fn resonate_supervise(_refined: &Vec<f64>, _refv: &Vec<f64>, _q: f64) -> bool {
    true
}

/// BP-01 — drive the closed-loop resonator toward the immutable ground (the origin) and return
/// the converged state vector.
///
/// `field` is the initial state; `coeff` is the per-tick relaxation factor applied by the
/// generator (`x *= coeff`). The reference is the zero vector (the "ground" the loop settles on).
/// Lyapunov guard + drift-rollback are active (see `bebop2_core::resonator`).
#[wasm_bindgen]
pub fn resonate(field: &[f64], coeff: f64) -> Vec<f64> {
    RELAX_COEFF.with(|c| *c.borrow_mut() = coeff);
    let reference = bebop2_core::resonator::Reference {
        value: vec![0.0f64; field.len()],
    };
    let initial = field.to_vec();
    let actors = bebop2_core::resonator::Actors {
        generate: resonate_generate,
        reflect: resonate_reflect,
        supervise: resonate_supervise,
    };
    let cfg = bebop2_core::resonator::LoopConfig::default();
    let res = bebop2_core::resonator::run_resonator(
        &reference,
        initial,
        &actors,
        &bebop2_core::resonator::L2Metric,
        &cfg,
    );
    res.final_state
}

// ── BP-03: eigensolver (spectral radius) ─────────────────────────────────────

/// BP-03 — spectral radius ρ = max|λ| of a real `n×n` matrix `a` (row-major), returned as f64.
/// Uses the general (non-symmetric) Francis double-shift QR eigensolver, so rotational/complex
/// modes are handled correctly.
#[wasm_bindgen]
pub fn spectral_radius(a: &[f64], n: usize) -> f64 {
    let ev = bebop2_core::lyapunov::eigenvalues_general(a, n);
    ev.iter().map(|c| c.norm()).fold(0.0f64, f64::max)
}

// ── BP-02: geodesic (angular distance) ───────────────────────────────────────

/// BP-02 — great-circle (geodesic) distance `d_g = arccos(⟨a,b⟩)` between two equal-length
/// vectors on the unit sphere, in `[0, π]`. A true metric (unlike `1−cos`).
#[wasm_bindgen]
pub fn geodesic(a: &[f64], b: &[f64]) -> f64 {
    bebop2_core::algebra::geodesic_distance(a, b)
}

// ── BP-04: diffusion / belief entropy ────────────────────────────────────────

/// BP-04 — Shannon entropy `H = −Σ b_i ln b_i` of a (normalized) belief distribution. This is the
/// entropy primitive the diffusion / free-energy kernel (`active`) is built on. A zero probability
/// mass contributes `0·ln0 := 0` (limit).
#[wasm_bindgen]
pub fn entropy(b: &[f64]) -> f64 {
    bebop2_core::active::entropy(b)
}

// ── BP-06: entropy ledger (hash-chained append-only accumulator) ─────────────

/// BP-06 — append `entry` to a deterministic hash-chained ledger and return the new head hash.
///
/// `prev_hash` is the previous ledger head (empty for the genesis entry); the new head is
/// `SHA3-256(prev_hash ‖ entry)`. Collision-resistant and reproducible (no RNG / clock). This is a
/// genuine ledger built on `bebop2_core::hash::sha3_256` (there is no separate ledger struct in
/// bebop2-core, so the accumulator is constructed here from the real primitive).
#[wasm_bindgen]
pub fn ledger_step(prev_hash: &[u8], entry: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(prev_hash.len() + entry.len());
    buf.extend_from_slice(prev_hash);
    buf.extend_from_slice(entry);
    bebop2_core::hash::sha3_256(&buf).to_vec()
}

// ── Native unit tests ────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonate_relaxes_to_ground() {
        // coeff 0.5 + strong reflector ⇒ converges to (near) the origin well under the fuse.
        let out = resonate(&[10.0, -5.0, 3.0, 7.0], 0.5);
        assert_eq!(out.len(), 4);
        for &x in &out {
            assert!(x.abs() < 1e-3, "state component did not converge: {x}");
        }
    }

    #[test]
    fn resonate_coeff_kept_positive_diverges_slowly_but_is_bounded() {
        // coeff 1.0 + strong reflector still pulls to ground; stays finite, no NaN/inf.
        let out = resonate(&[1.0, 2.0], 1.0);
        for &x in &out {
            assert!(x.is_finite(), "non-finite state: {x}");
        }
    }

    #[test]
    fn spectral_radius_of_scaled_identity() {
        // 2×2: 0.5·I ⇒ both eigenvalues 0.5 ⇒ ρ = 0.5.
        let a = [0.5f64, 0.0, 0.0, 0.5];
        let r = spectral_radius(&a, 2);
        assert!((r - 0.5).abs() < 1e-9, "expected 0.5, got {r}");
    }

    #[test]
    fn spectral_radius_of_swap_is_one() {
        // Ã = [[0,1],[1,0]] (swap) ⇒ eigenvalues ±1 ⇒ ρ = 1.
        let a = [0.0f64, 1.0, 1.0, 0.0];
        let r = spectral_radius(&a, 2);
        assert!((r - 1.0).abs() < 1e-12, "expected 1.0, got {r}");
    }

    #[test]
    fn geodesic_identical_is_zero_and_perp_is_half_pi() {
        let x = [1.0f64, 0.0, 0.0];
        let y = [0.0, 1.0, 0.0];
        assert!(geodesic(&x, &x).abs() < 1e-12, "identical ⇒ 0");
        let d = geodesic(&x, &y);
        assert!(
            (d - std::f64::consts::FRAC_PI_2).abs() < 1e-12,
            "perpendicular ⇒ π/2, got {d}"
        );
    }

    #[test]
    fn entropy_of_uniform_matches_bits() {
        // Uniform over 4 outcomes ⇒ H = ln(4) ≈ 1.38629.
        let b = [0.25f64, 0.25, 0.25, 0.25];
        let h = entropy(&b);
        assert!(
            (h - 4.0 * 0.25 * 0.25f64.ln().abs()).abs() < 1e-9,
            "got {h}"
        );
    }

    #[test]
    fn entropy_zero_mass_contributes_nothing() {
        let b = [1.0f64, 0.0, 0.0];
        let h = entropy(&b);
        assert!(
            h.abs() < 1e-12,
            "degenerate belief has zero entropy, got {h}"
        );
    }

    #[test]
    fn ledger_chains_and_is_deterministic() {
        let h0: Vec<u8> = Vec::new();
        let h1 = ledger_step(&h0, &[1, 2, 3]);
        let h2 = ledger_step(&h1, &[4, 5, 6]);
        assert_eq!(h1.len(), 32);
        assert_eq!(h2.len(), 32);
        assert_ne!(h1, h2, "distinct entries must change the head");
        // determinism: same inputs ⇒ same head
        let h1b = ledger_step(&h0, &[1, 2, 3]);
        assert_eq!(h1, h1b, "ledger step must be reproducible");
    }
}

/// A3 — EIGENSOLVER PARITY GATE. Two independent general (non-symmetric) real eigensolvers exist
/// with NO parity gate between them: dowiz-kernel's Faddeev-LeVerrier + Durand-Kerner
/// (`dowiz_kernel::spectral`) and bebop2-core's Hessenberg + Francis-QR
/// (`bebop2_core::lyapunov::eigenvalues_general`). That is the exact dual-authority silent-drift
/// hazard the kernel exists to kill (cf. `kernel/src/markov.rs`, which already closed the Python↔Rust
/// version of it). This crate is the only bridge that already links both, so it hosts the gate:
/// it cross-checks BOTH solvers against each other AND against analytic ground truth on shared
/// fixtures; if either implementation drifts, it goes RED. Native `cargo test` only (dev-dep on the
/// kernel; never in the wasm build).
#[cfg(test)]
mod eigensolver_parity {
    const TOL: f64 = 1e-6;

    fn flat(rows: &[Vec<f64>]) -> Vec<f64> {
        rows.iter().flatten().copied().collect()
    }
    fn sort_pairs(v: &mut [(f64, f64)]) {
        v.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap()
                .then(a.1.partial_cmp(&b.1).unwrap())
        });
    }
    fn kernel_eigs(rows: &[Vec<f64>]) -> Vec<(f64, f64)> {
        let mut v: Vec<(f64, f64)> = dowiz_kernel::spectral::eigenvalues(rows)
            .iter()
            .map(|c| (c.re, c.im))
            .collect();
        sort_pairs(&mut v);
        v
    }
    fn bebop_eigs(rows: &[Vec<f64>]) -> Vec<(f64, f64)> {
        let f = flat(rows);
        let mut v: Vec<(f64, f64)> = bebop2_core::lyapunov::eigenvalues_general(&f, rows.len())
            .iter()
            .map(|c| (c.re, c.im))
            .collect();
        sort_pairs(&mut v);
        v
    }
    fn assert_close(a: &[(f64, f64)], b: &[(f64, f64)], msg: &str) {
        assert_eq!(
            a.len(),
            b.len(),
            "{msg}: eigenvalue COUNT differs {a:?} vs {b:?}"
        );
        for (x, y) in a.iter().zip(b.iter()) {
            assert!(
                (x.0 - y.0).abs() < TOL && (x.1 - y.1).abs() < TOL,
                "{msg}: eigenvalue drift {x:?} vs {y:?}"
            );
        }
    }
    fn analytic(mut w: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        sort_pairs(&mut w);
        w
    }

    // GREEN: FL+DK ↔ Francis-QR ↔ analytic, on real-spectrum fixtures.
    #[test]
    fn fl_dk_matches_francis_qr_real_spectrum() {
        // diag(2,3,5) → {2,3,5}
        let m = vec![
            vec![2.0, 0.0, 0.0],
            vec![0.0, 3.0, 0.0],
            vec![0.0, 0.0, 5.0],
        ];
        assert_close(&kernel_eigs(&m), &bebop_eigs(&m), "diag: kernel↔bebop2");
        assert_close(
            &kernel_eigs(&m),
            &analytic(vec![(2.0, 0.0), (3.0, 0.0), (5.0, 0.0)]),
            "diag: ↔analytic",
        );

        // upper-triangular → eigenvalues are the diagonal {1,4,-3}
        let m = vec![
            vec![1.0, 2.0, 0.0],
            vec![0.0, 4.0, 5.0],
            vec![0.0, 0.0, -3.0],
        ];
        assert_close(
            &kernel_eigs(&m),
            &bebop_eigs(&m),
            "triangular: kernel↔bebop2",
        );
        assert_close(
            &kernel_eigs(&m),
            &analytic(vec![(1.0, 0.0), (4.0, 0.0), (-3.0, 0.0)]),
            "triangular: ↔analytic",
        );

        // companion [[0,-2],[1,-3]] → λ²+3λ+2 → {-1,-2}
        let m = vec![vec![0.0, -2.0], vec![1.0, -3.0]];
        assert_close(
            &kernel_eigs(&m),
            &bebop_eigs(&m),
            "companion: kernel↔bebop2",
        );
        assert_close(
            &kernel_eigs(&m),
            &analytic(vec![(-1.0, 0.0), (-2.0, 0.0)]),
            "companion: ↔analytic",
        );
    }

    // GREEN: complex-conjugate spectrum — the case a symmetric-only solver gets WRONG.
    // The 2-D rotation [[0,-1],[1,0]] has eigenvalues ±i; both GENERAL solvers must agree.
    #[test]
    fn complex_conjugate_eigenvalues_agree() {
        let m = vec![vec![0.0, -1.0], vec![1.0, 0.0]];
        assert_close(&kernel_eigs(&m), &bebop_eigs(&m), "rotation: kernel↔bebop2");
        assert_close(
            &kernel_eigs(&m),
            &analytic(vec![(0.0, -1.0), (0.0, 1.0)]),
            "rotation: ↔analytic",
        );
    }

    // GREEN: spectral radius (the scalar the wasm surface returns) — parity kernel FL+DK vs the
    // live `spectral_radius` wasm export (bebop2 QR), cross-checked against analytic ρ.
    #[test]
    fn spectral_radius_parity_kernel_vs_wasm_surface() {
        let fixtures: [(Vec<Vec<f64>>, f64); 3] = [
            (
                vec![
                    vec![2.0, 0.0, 0.0],
                    vec![0.0, 3.0, 0.0],
                    vec![0.0, 0.0, 5.0],
                ],
                5.0,
            ),
            (vec![vec![0.0, -2.0], vec![1.0, -3.0]], 2.0),
            (vec![vec![0.0, -1.0], vec![1.0, 0.0]], 1.0),
        ];
        for (m, rho) in &fixtures {
            let k = dowiz_kernel::spectral::spectral_radius(m);
            let b = crate::spectral_radius(&flat(m), m.len()); // wasm surface → bebop2 QR
            assert!((k - b).abs() < TOL, "ρ kernel {k} ≠ bebop2 {b}");
            assert!((k - rho).abs() < TOL, "ρ {k} ≠ analytic {rho}");
        }
    }
}
