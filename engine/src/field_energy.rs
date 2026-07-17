//! field_energy.rs — BLUEPRINT-E1 kernel↔engine seam gate (TEST-ONLY module).
//!
//! Two organs that had never met are wired here:
//!   * the kernel's `noether::lyapunov_nonincreasing` energy checker, and
//!   * the engine's `field_frame` semi-implicit integrator.
//!
//! It delivers the two falsifiable guarantees §0 of the blueprint names:
//!   HALF A — the field integrator's Dirichlet energy is monotone NON-INCREASING
//!            per step (a physical, Lyapunov certificate the tree lacked), and it
//!            is CAUGHT when an artificially-broken integrator pumps energy in.
//!   HALF B — the SIGN-PIN: `field_frame::laplacian` (the `−(D−A)` grid stencil)
//!            is bound to the kernel's `+(D−A)` reference operator by a
//!            red-provable test, retiring the last unpinned mirror at the seam.
//!
//! The whole module is `#[cfg(test)]` (declared so in `lib.rs`): it changes no
//! runtime contract — the incidence operator and the energy check are the
//! test-side reference oracle, never on `FieldFrame::step`'s hot path.

use crate::field_frame::{self, FieldEquilibrium, FieldFrame};
use crate::scene::{Scene, SdfShape};
use dowiz_kernel::csr::{Csr, LaplacianKind};
use dowiz_kernel::incidence::Incidence;
use dowiz_kernel::noether::{invariant_drift, lyapunov_nonincreasing};

// ── Lattice builders — the SAME 4-connectivity `w×h` grid `field_frame` walks ──

/// Grid node index (row-major), matching `field_frame::laplacian`.
#[inline]
fn idx(r: usize, c: usize, w: usize) -> usize {
    r * w + c
}

/// The grid as a CSR graph — undirected edges DOUBLED per `Csr::from_edges`'s
/// contract. 4-connectivity, no wraparound (matches the stencil's real neighbours).
fn lattice_csr(w: usize, h: usize) -> Csr {
    let mut edges = Vec::new();
    for r in 0..h {
        for c in 0..w {
            let i = idx(r, c, w);
            if c + 1 < w {
                let j = idx(r, c + 1, w);
                edges.push((i, j, 1.0));
                edges.push((j, i, 1.0));
            }
            if r + 1 < h {
                let j = idx(r + 1, c, w);
                edges.push((i, j, 1.0));
                edges.push((j, i, 1.0));
            }
        }
    }
    Csr::from_edges(w * h, &edges)
}

/// The same grid as an oriented incidence — ONE tuple per undirected pair.
fn lattice_incidence(w: usize, h: usize) -> Incidence {
    let mut edges = Vec::new();
    for r in 0..h {
        for c in 0..w {
            let i = idx(r, c, w);
            if c + 1 < w {
                edges.push((i, idx(r, c + 1, w), 1.0));
            }
            if r + 1 < h {
                edges.push((i, idx(r + 1, c, w), 1.0));
            }
        }
    }
    Incidence::from_edges(w * h, &edges)
}

// ── Energy functional pieces ──────────────────────────────────────────────

/// Dirichlet energy `½·xᵀ·(L x)` computed THROUGH the existing CSR operator
/// (never a fresh Laplacian). `kind` selects the convention; the field-integrator
/// gate uses `Unnormalized` (the `+(D−A)` well matching the stencil's negation).
fn dirichlet_energy(x: &[f64], graph: &Csr, kind: LaplacianKind) -> f64 {
    let mut lx = vec![0.0f64; x.len()];
    graph.laplacian_spmv(x, &mut lx, kind);
    0.5 * x.iter().zip(&lx).map(|(&a, &b)| a * b).sum::<f64>()
}

/// The blueprint's exact energy `E(U,U̇) = ½‖U̇‖² + ½c²⟨U,L₊U⟩ − ⟨S,U⟩` on the
/// packed state `z = [U ‖ U_prev]`, with `U̇ = (U − U_prev)/dt` (the SAME backward
/// difference the integrator uses at `field_frame.rs:154`) and `L₊` the POSITIVE
/// `(D−A)` operator (so the potential is a genuine well `½c²Σ_edges(U_i−U_j)² ≥ 0`).
fn field_energy(z: &[f64], source: &[f64], eq: &FieldEquilibrium, graph: &Csr) -> f64 {
    let n = source.len();
    let (u, uprev) = z.split_at(n);
    let dt = eq.dt;
    let kinetic = 0.5
        * u.iter()
            .zip(uprev)
            .map(|(&a, &b)| {
                let ud = (a - b) / dt;
                ud * ud
            })
            .sum::<f64>();
    let potential = eq.c2 * dirichlet_energy(u, graph, LaplacianKind::Unnormalized);
    let coupling: f64 = u.iter().zip(source).map(|(&a, &s)| a * s).sum();
    kinetic + potential - coupling
}

/// Faithful f32-mirroring transcription of `FieldFrame::step` on the packed state,
/// parameterized so the §4.5 non-vacuousness mutants can be injected:
///   * `stencil_sign = +1.0` reproduces the real diffusion `+c²·(−(D−A))U`;
///     `−1.0` is the anti-diffusion break `+c²·(+(D−A))U`.
///   * `gamma` is the damping; `−eq.gamma` is the anti-damping break.
/// With `(+1.0, eq.gamma)` this is BIT-IDENTICAL to `FieldFrame::step` (proven by
/// `field_step_transcription_matches_real_integrator`).
fn field_step_packed(
    z: &[f64],
    source: &[f32],
    eq: &FieldEquilibrium,
    w: usize,
    h: usize,
    stencil_sign: f32,
    gamma: f64,
) -> Vec<f64> {
    let n = w * h;
    let (uz, upz) = z.split_at(n);
    let u32: Vec<f32> = uz.iter().map(|&v| v as f32).collect();
    let uprev32: Vec<f32> = upz.iter().map(|&v| v as f32).collect();
    let lap = field_frame::laplacian(&u32, w, h); // the real f32 grid stencil
    let dt = eq.dt;
    let mut unext = vec![0.0f32; n];
    for i in 0..n {
        let u = u32[i] as f64;
        let uprev = uprev32[i] as f64;
        let s = source[i] as f64;
        let l = (stencil_sign as f64) * (lap[i] as f64);
        let udot = (u - uprev) / dt;
        let num = u + dt * (gamma * udot + eq.c2 * l) + dt * s;
        let den = 1.0 + dt * eq.m;
        unext[i] = (num / den) as f32;
    }
    let mut out = Vec::with_capacity(2 * n);
    out.extend(unext.iter().map(|&v| v as f64));
    out.extend(u32.iter().map(|&v| v as f64));
    out
}

/// A finite SDF source (centred disk) on the `w×h` grid — the same driver the
/// existing bounded-and-converges test uses.
fn disk_source(w: usize, h: usize) -> Vec<f32> {
    let mut scene = Scene::new().with_scale(1.0);
    scene.add(SdfShape::Circle {
        cx: 0.0,
        cy: 0.0,
        r: 4.0,
    });
    scene.render_frame(w, h)
}

// ── tol_E — PINNED from a known-good run, not a magic number (BLUEPRINT §2b) ──
//
// Empirical calibration (diagnostic, this file's git history): on the default
// `FieldEquilibrium` (m=1, Γ=0.2, c²=1, dt=0.02) driven by the centred-disk SDF
// source on a 12×12 grid, after a 4-step warm-up past the source-switch-on
// impulse, the LARGEST per-step change over the next ~400 steps is
// −1.09e-4 — i.e. every step STRICTLY DECREASES the energy. `tol_E` therefore
// only has to absorb IEEE rounding in ‖U̇‖²/⟨U,L₊U⟩; 1e-6 is far below the
// −1e-4 strict-decrease margin and ~8 orders below the +2e2 growth the broken
// variants (§4.5) inject, keeping the gate sharp AND non-vacuous.
const TOL_E: f64 = 1e-6;
// Warm-up skips the switch-on impulse (source off→on at t=0 forces energy in as
// U ramps from 0) and the reconstructed-velocity spike U̇=(U−0)/dt (BLUEPRINT §2b
// item ii). The Lyapunov claim is over the DISSIPATIVE evolution, not the forcing
// transient.
const WARMUP: usize = 4;
const GATE_STEPS: usize = 200;
const GRID: (usize, usize) = (12, 12);

/// Warm a packed state `z = [U ‖ U_prev]` past the switch-on transient.
fn warmed_state(w: usize, h: usize, source: &[f32], eq: &FieldEquilibrium) -> Vec<f64> {
    let mut z = vec![0.0f64; 2 * w * h];
    for _ in 0..WARMUP {
        z = field_step_packed(&z, source, eq, w, h, 1.0, eq.gamma);
    }
    z
}

// ═════════════════════════════════════════════════════════════════════════
// HALF B — THE SIGN-PIN (BLUEPRINT §2a #3 / §4.2), the missing tripwire.
// ═════════════════════════════════════════════════════════════════════════

/// `field_frame::laplacian` (the `−(D−A)` grid stencil) is the exact NEGATION of
/// the kernel's `+(D−A)` incidence reference on the shared lattice — pinned with
/// the minus sign VISIBLE, on interior nodes (Neumann-boundary honest mask).
///
/// The test is a real tripwire, proven three ways in-place (RED against each):
///   (1) the correct `stencil == −incidence.laplacian` holds (GREEN);
///   (2) the NAIVE shared-convention form `stencil == +incidence.laplacian` is
///       FALSE at interior nodes — i.e. the opposite-sign split genuinely exists
///       (this is the assertion §4.2 says "is red against live code");
///   (3) the reference is non-trivial (non-constant field ⇒ some interior node
///       has a non-zero Laplacian), so the sign actually matters.
/// Independently (see report): flipping `field_frame.rs:103` to `4u−Σ` turns this
/// pin RED — the divergence this test would have caught.
#[test]
fn sign_pin_field_frame_stencil_is_negative_incidence() {
    let (w, h) = (11usize, 9usize);
    // Deterministic, non-constant field with EXACT f32 integer values (0..4) so
    // the f32 stencil sum and the f64 (D−A) agree to the bit — the pin is razor
    // sharp, no float-slop hiding a sign error.
    let u32: Vec<f32> = (0..w * h)
        .map(|i| {
            let (r, c) = (i / w, i % w);
            ((r * 7 + c * 3) % 5) as f32
        })
        .collect();
    let u64: Vec<f64> = u32.iter().map(|&v| v as f64).collect();

    let stencil = field_frame::laplacian(&u32, w, h); // −(D−A)U, f32
    let inc = lattice_incidence(w, h);
    let lap_pos = inc.laplacian(&u64); // +(D−A)U, f64

    let mut nonzero_seen = false;
    let mut naive_would_fail = false;
    for r in 1..h - 1 {
        for c in 1..w - 1 {
            let i = idx(r, c, w);
            let s = stencil[i] as f64;
            // (1) the PIN, minus sign visible.
            assert!(
                (s - (-lap_pos[i])).abs() <= 1e-9,
                "sign-pin RED at interior ({r},{c}): stencil={s} != -incidence={}",
                -lap_pos[i]
            );
            if lap_pos[i].abs() > 1e-9 {
                nonzero_seen = true;
                // (2) the naive `+` form MUST fail here (the split exists).
                if (s - lap_pos[i]).abs() > 1e-9 {
                    naive_would_fail = true;
                }
            }
        }
    }
    assert!(
        nonzero_seen,
        "field must be non-constant for the sign to matter"
    );
    assert!(
        naive_would_fail,
        "the naive shared-convention assertion (stencil == +incidence) must be \
         FALSE somewhere — otherwise the split does not exist and the pin is vacuous"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// HALF A — the packed-state transcription is FAITHFUL to the real integrator.
// ═════════════════════════════════════════════════════════════════════════

/// The `field_step_packed(_, +1.0, eq.gamma)` transcription used by the energy
/// gate is BIT-IDENTICAL to the real `FieldFrame::step` — so the Lyapunov check
/// is verifying the ACTUAL integrator, not a plausible fiction. Driven from the
/// same zero state under the same disk source for many steps.
#[test]
fn field_step_transcription_matches_real_integrator() {
    let (w, h) = GRID;
    let n = w * h;
    let eq = FieldEquilibrium::default();
    let source = disk_source(w, h);

    let mut frame = FieldFrame::new(w, h);
    let mut z = vec![0.0f64; 2 * n];
    for step in 0..300 {
        frame.step(&source, &eq);
        z = field_step_packed(&z, &source, &eq, w, h, 1.0, eq.gamma);
        for i in 0..n {
            assert_eq!(
                frame.u()[i] as f64,
                z[i],
                "transcription diverged from FieldFrame::step at step {step}, cell {i}"
            );
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════
// HALF A — the energy Lyapunov gate itself (BLUEPRINT §4.4).
// ═════════════════════════════════════════════════════════════════════════

/// On `FieldEquilibrium::default()` (Γ=0.2>0) driven by a finite SDF source, the
/// packed-state Dirichlet energy is monotone NON-INCREASING per step within
/// `TOL_E` — `noether::lyapunov_nonincreasing == true` — and `invariant_drift`
/// equals the total dissipation `E(x0) − E(x_final)`, a positive bounded number.
#[test]
fn field_energy_is_monotone_nonincreasing() {
    let (w, h) = GRID;
    let eq = FieldEquilibrium::default();
    let source_f32 = disk_source(w, h);
    let source_f64: Vec<f64> = source_f32.iter().map(|&v| v as f64).collect();
    let graph = lattice_csr(w, h);

    let x0 = warmed_state(w, h, &source_f32, &eq);

    let update = |z: &[f64]| field_step_packed(z, &source_f32, &eq, w, h, 1.0, eq.gamma);
    let potential = |z: &[f64]| field_energy(z, &source_f64, &eq, &graph);

    // The certificate: energy never spontaneously grows.
    assert!(
        lyapunov_nonincreasing(&x0, &update, &potential, GATE_STEPS, TOL_E),
        "field integrator energy must be monotone non-increasing (Γ>0, fixed S)"
    );

    // invariant_drift == total dissipation E(x0) − E(x_final) (monotone ⇒ Σ|ΔE| = drop).
    let drift = invariant_drift(&x0, &update, &potential, GATE_STEPS);
    let mut x_final = x0.clone();
    for _ in 0..GATE_STEPS {
        x_final = update(&x_final);
    }
    let dissipated = potential(&x0) - potential(&x_final);
    assert!(dissipated > 0.0, "energy must actually dissipate (drop > 0)");
    assert!(dissipated.is_finite() && dissipated < 1e6, "dissipation bounded");
    assert!(
        (drift - dissipated).abs() <= 1e-6,
        "invariant_drift {drift} must equal total dissipation {dissipated}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// HALF A — NON-VACUOUSNESS: an artificially-broken integrator is CAUGHT.
// (BLUEPRINT §4.5, mirroring noether::catches_euler_energy_drift.)
// ═════════════════════════════════════════════════════════════════════════

/// (a) Flipping the stencil sign gives anti-diffusion `+c²(D−A)U` — the exact
/// failure a caller crossing the unpinned seam with the `+` operator would hit.
/// The largest-eigenvalue modes now GROW, energy increases, and the gate returns
/// `false`. We also assert an explicit early increase so the proof is robust even
/// if the field later overflows to a non-comparable NaN.
#[test]
fn energy_gate_catches_anti_diffusion() {
    let (w, h) = GRID;
    let eq = FieldEquilibrium::default();
    let source_f32 = disk_source(w, h);
    let source_f64: Vec<f64> = source_f32.iter().map(|&v| v as f64).collect();
    let graph = lattice_csr(w, h);
    let x0 = warmed_state(w, h, &source_f32, &eq);

    let broken = |z: &[f64]| field_step_packed(z, &source_f32, &eq, w, h, -1.0, eq.gamma);
    let potential = |z: &[f64]| field_energy(z, &source_f64, &eq, &graph);

    assert!(
        !lyapunov_nonincreasing(&x0, &broken, &potential, GATE_STEPS, TOL_E),
        "anti-diffusion pumps energy in — the gate MUST reject it (non-vacuous)"
    );
    // Prove the rejection is a GENUINE FINITE energy increase (not NaN vacuity):
    // walk the trajectory and locate the first step whose energy rise exceeds
    // TOL_E, asserting BOTH energies there are finite. (`lyapunov` can only return
    // false via exactly such a comparison — NaN>tol is false — so this simply
    // exhibits the step it tripped on.)
    let mut x = x0.clone();
    let mut e_prev = potential(&x);
    let mut found = false;
    for _ in 0..GATE_STEPS {
        let x_next = broken(&x);
        let e_next = potential(&x_next);
        if e_next - e_prev > TOL_E {
            assert!(
                e_prev.is_finite() && e_next.is_finite(),
                "the detected energy increase must be finite (genuine growth, not overflow)"
            );
            found = true;
            break;
        }
        x = x_next;
        e_prev = e_next;
    }
    assert!(found, "anti-diffusion must exhibit a finite energy increase");
}

/// (b) Negating Γ gives anti-damping (Γ=−0.2) — the sign-flipped damping the
/// bounded-and-converges test cannot see. It pumps energy in; the gate returns
/// `false`.
#[test]
fn energy_gate_catches_anti_damping() {
    let (w, h) = GRID;
    let eq = FieldEquilibrium::default();
    let source_f32 = disk_source(w, h);
    let source_f64: Vec<f64> = source_f32.iter().map(|&v| v as f64).collect();
    let graph = lattice_csr(w, h);
    let x0 = warmed_state(w, h, &source_f32, &eq);

    let broken = |z: &[f64]| field_step_packed(z, &source_f32, &eq, w, h, 1.0, -eq.gamma);
    let potential = |z: &[f64]| field_energy(z, &source_f64, &eq, &graph);

    assert!(
        !lyapunov_nonincreasing(&x0, &broken, &potential, GATE_STEPS, TOL_E),
        "anti-damping (Γ=-0.2) pumps energy in — the gate MUST reject it (non-vacuous)"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Normalized-branch coverage sanity (the bridge.rs:125 operator), complementing
// the kernel-side `parity_incidence_reference_matches_csr_normalized`.
// ═════════════════════════════════════════════════════════════════════════

/// The Dirichlet energy is a genuine well (≥0) for BOTH the Unnormalized `(D−A)`
/// operator (the integrator's) AND the Normalized operator the live trigger
/// caller uses — so the energy plumbing is exercised on the branch bridge.rs:125
/// actually calls, not only the originally-scoped Unnormalized case.
#[test]
fn dirichlet_energy_nonnegative_both_conventions() {
    let (w, h) = (6usize, 6usize);
    let graph = lattice_csr(w, h);
    let x: Vec<f64> = (0..w * h).map(|i| ((i % 7) as f64) - 3.0).collect();
    let un = dirichlet_energy(&x, &graph, LaplacianKind::Unnormalized);
    let nm = dirichlet_energy(&x, &graph, LaplacianKind::Normalized);
    assert!(un >= -1e-12, "Unnormalized Dirichlet energy must be ≥0, got {un}");
    assert!(nm >= -1e-12, "Normalized Dirichlet energy must be ≥0, got {nm}");
    // A non-constant field has strictly positive Dirichlet energy in both.
    assert!(un > 1e-6 && nm > 1e-6, "non-constant field ⇒ strictly positive well");
}
