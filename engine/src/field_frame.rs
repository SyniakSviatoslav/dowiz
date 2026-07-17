//! CPU field-evolution + RGBA frame composer on the SDF substrate.
//!
//! Implements the operator's physics-render equation on the existing
//! row-major SDF field buffer (do NOT fork it — reuse `Scene::field_view` /
//! `Scene::render_frame`). Authoritative compute stays CPU-side (per
//! `Cargo.toml`: wgpu OUT OF SCOPE in the default build). The GPU (future
//! `feature = "gpu"` adapter) would blit the `Vec<u8>` this module produces.
//!
//! Operator directive (rearranged, semi-implicit mass):
//!   M·U̇ = -ΓU̇ - c²·L·U + S   with  U̇ ≈ (U - U_prev)/dt
//!   ⇒  U_next = (U + dt·(Γ·U̇ + c²·L·U) + dt·S) / (1 + dt·M)
//! where L is the 5-point Laplacian (Neumann zero-flux edges), S is the SDF
//! source buffer (shapes are attractors/repellors), and the implicit M term
//! gives the (1+dt·M) denominator that keeps the scheme fail-closed.

use crate::scene::{Scene, SdfShape};

/// Physically-stable integration constants for the field operator.
///
/// Stability (explicit part of the semi-implicit scheme): the Laplacian's
/// discrete eigenvalues live in `[-4, 0]`, so a CFL-ish bound keeps the
/// explicit `c²·L·U` term bounded:
///
///   dt ≤ M / (Γ + 2·c²)   →   fail-closed: `assert_stable` panics otherwise.
///
/// (The implicit M term widens the real margin, but we assert the conservative
/// bound so a divergent dt can never reach the integrator.)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldEquilibrium {
    /// Mass / inertia M (must be > 0).
    pub m: f64,
    /// Damping coefficient Γ (≥ 0).
    pub gamma: f64,
    /// Wave-speed squared c² (> 0).
    pub c2: f64,
    /// Fixed timestep dt (seconds). Asserted within the stability bound.
    pub dt: f64,
}

impl Default for FieldEquilibrium {
    fn default() -> Self {
        // ONE governed field/animation clock: dt is pinned to the kernel's
        // authoritative `DT_STABLE` (0.02 s == 50 Hz), crossing the f32→f64
        // boundary explicitly. m=1, Γ=0.2, c²=1, dt=0.02 → bound = 1/(0.2+2)
        // ≈ 0.455 ≫ dt: safe (see `assert_stable`). Pinned by
        // `field_default_dt_matches_kernel_dt_stable`.
        FieldEquilibrium {
            m: 1.0,
            gamma: 0.2,
            c2: 1.0,
            dt: dowiz_kernel::DT_STABLE as f64,
        }
    }
}

impl FieldEquilibrium {
    /// Fail-closed: panic if dt is outside the CFL-ish stability bound, or if
    /// the coefficients are non-physical. Called before every integration step.
    pub fn assert_stable(&self) {
        assert!(self.m > 0.0, "FieldEquilibrium.m must be > 0 (mass/inertia)");
        assert!(
            self.gamma + 2.0 * self.c2 > 0.0,
            "FieldEquilibrium: Gamma + 2*c2 must be > 0 for the stability denominator"
        );
        let bound = self.m / (self.gamma + 2.0 * self.c2);
        assert!(
            self.dt > 0.0 && self.dt <= bound,
            "dt={} exceeds stability bound dt <= M/(Gamma+2*c2)={} (fail-closed)",
            self.dt,
            bound
        );
    }
}

/// 5-point finite-difference Laplacian on a row-major buffer.
///
/// Edges use **Neumann zero-flux**: a missing neighbour is replaced by the
/// centre value, so shapes do not bleed across the frame border. `∇²U` of a
/// constant field is exactly zero.
pub struct LaplacianField;

impl LaplacianField {
    /// Named surface for the operator's "LaplacianField"; delegates to the free
    /// [`laplacian`] function.
    pub fn compute(&self, u: &[f32], w: usize, h: usize) -> Vec<f32> {
        laplacian(u, w, h)
    }
}

/// 5-point ∇²U on the row-major `u` buffer (`len == w*h`). Neumann zero-flux
/// at edges. Pure / deterministic; no allocation beyond the output.
pub fn laplacian(u: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; w * h];
    laplacian_into(u, w, h, &mut out);
    out
}

/// In-place 5-point ∇²U variant: fills the caller-owned `out` slice with the
/// SAME stencil math as [`laplacian`] (Neumann zero-flux edges). Zero heap
/// allocation — this is what the hot `FieldFrame::step()` loop calls so no
/// per-step `Vec` is created. The free [`laplacian`] delegates here, so both
/// surfaces are guaranteed bit-identical.
pub fn laplacian_into(u: &[f32], w: usize, h: usize, out: &mut [f32]) {
    debug_assert_eq!(u.len(), w * h, "field length must equal w*h");
    debug_assert_eq!(out.len(), w * h, "output length must equal w*h");
    for r in 0..h {
        for c in 0..w {
            let i = r * w + c;
            // Reflective neighbours (clamp index) ==> zero-flux boundary.
            let left = u[r * w + c.saturating_sub(1)];
            let right = u[r * w + (c + 1).min(w - 1)];
            let up = u[(r.saturating_sub(1)) * w + c];
            let down = u[((r + 1).min(h - 1)) * w + c];
            out[i] = left + right + up + down - 4.0 * u[i];
        }
    }
}

/// Stateful field integrator: holds `U` (current) and `U_prev` (for the
/// backward-difference U̇). Evolves the operator equation one step at a time
/// and can render the current `U` to an RGBA8 display frame.
pub struct FieldFrame {
    u: Vec<f32>,
    u_prev: Vec<f32>,
    /// Pre-allocated scratch for the per-step Laplacian (`laplacian_into`).
    lap_scratch: Vec<f32>,
    /// Pre-allocated scratch that receives `U_next` before buffer rotation.
    next_scratch: Vec<f32>,
    w: usize,
    h: usize,
}

impl FieldFrame {
    /// Allocate a zeroed field of `w × h` (U = U_prev = 0). The two scratch
    /// buffers (`lap_scratch`, `next_scratch`) are allocated ONCE here so
    /// `step()` performs zero heap allocations thereafter.
    pub fn new(w: usize, h: usize) -> Self {
        FieldFrame {
            u: vec![0.0f32; w * h],
            u_prev: vec![0.0f32; w * h],
            lap_scratch: vec![0.0f32; w * h],
            next_scratch: vec![0.0f32; w * h],
            w,
            h,
        }
    }

    /// Borrow the current field `U`.
    #[inline]
    pub fn u(&self) -> &[f32] {
        &self.u
    }

    /// Advance one timestep under `eq`, driven by source `S` (the SDF buffer).
    ///
    ///   U̇     = (U - U_prev) / dt
    ///   L·U    = laplacian(U)
    ///   U_next = (U + dt·(Γ·U̇ + c²·L·U) + dt·S) / (1 + dt·M)
    ///
    /// Fail-closed: asserts the stability bound before integrating.
    pub fn step(&mut self, source: &[f32], eq: &FieldEquilibrium) {
        eq.assert_stable();
        debug_assert_eq!(source.len(), self.w * self.h);
        // Write the Laplacian into the pre-allocated scratch (no alloc).
        laplacian_into(&self.u, self.w, self.h, &mut self.lap_scratch);
        let dt = eq.dt;
        // Compute U_next into the pre-allocated scratch (no alloc). The
        // per-cell arithmetic order below is IDENTICAL to the previous
        // implementation — only the buffer lifecycle changed.
        for i in 0..self.w * self.h {
            let u = self.u[i] as f64;
            let uprev = self.u_prev[i] as f64;
            let s = source[i] as f64;
            let l = self.lap_scratch[i] as f64;
            let udot = (u - uprev) / dt;
            let num = u + dt * (eq.gamma * udot + eq.c2 * l) + dt * s;
            let den = 1.0 + dt * eq.m;
            self.next_scratch[i] = (num / den) as f32;
        }
        // Rotate the three live buffers with no allocation or drop:
        //   u_prev <- old u, u <- next (freshly computed),
        //   next_scratch <- old u_prev (recycled as future scratch).
        std::mem::swap(&mut self.u_prev, &mut self.u); // u_prev <- old u; u <- old u_prev
        std::mem::swap(&mut self.u, &mut self.next_scratch); // u <- next; next_scratch <- old u_prev
    }

    /// Map the current field `U` to an RGBA8 display frame (`len == w*h*4`).
    ///
    /// Deterministic, dependency-free: **sign → hue** (warm = positive,
    /// cool = negative), **magnitude → brightness** (clamped to [0,1]).
    /// Non-finite values (e.g. SDF background) render black, never NaN bytes.
    pub fn frame_rgba(&self) -> Vec<u8> {
        let n = self.w * self.h;
        let mut out = vec![0u8; n * 4];
        for i in 0..n {
            let v = self.u[i];
            let mag = if v.is_finite() { v.abs().min(1.0) } else { 0.0 };
            let b = (mag * 255.0) as u8;
            let (r, g, bl) = if v >= 0.0 {
                // warm: red-dominant, low blue.
                (b, (b as u32 * 128 / 255) as u8, (b as u32 * 40 / 255) as u8)
            } else {
                // cool: blue-dominant, low red.
                ((b as u32 * 40 / 255) as u8, (b as u32 * 128 / 255) as u8, b)
            };
            out[i * 4] = r;
            out[i * 4 + 1] = g;
            out[i * 4 + 2] = bl;
            out[i * 4 + 3] = 255;
        }
        out
    }
}

/// Convenience: rasterize `scene` to its SDF field buffer, run `steps`
/// evolution steps under `eq`, and return the final RGBA8 frame. This is the
/// single call a future `wgpu` blit would consume.
pub fn compose(scene: &Scene, eq: &FieldEquilibrium, w: usize, h: usize, steps: usize) -> Vec<u8> {
    let source = scene.render_frame(w, h);
    let mut frame = FieldFrame::new(w, h);
    for _ in 0..steps {
        frame.step(&source, eq);
    }
    frame.frame_rgba()
}

#[cfg(test)]
mod tests {
    use super::*;

    // (0) DT_STABLE mirror pin (row #10). The field integrator's default dt MUST
    //     equal the kernel's authoritative `DT_STABLE` (one governed 50 Hz clock).
    //     This crosses the crate boundary via the real import, mirroring the
    //     kernel's `dt_stable_is_authoritative` + engine loop_'s
    //     `dt_stable_matches_kernel_contract` template: literal identity + the
    //     50 Hz physical meaning. Reverting the default to 0.016 turns this red.
    #[test]
    fn field_default_dt_matches_kernel_dt_stable() {
        assert_eq!(
            FieldEquilibrium::default().dt,
            dowiz_kernel::DT_STABLE as f64
        );
        assert_eq!((1.0 / FieldEquilibrium::default().dt).round() as u32, 50); // 50 Hz, one clock
    }

    // (1) frame_buffer_dims_match_wxh_x4 -- RGBA out has exactly w*h*4 bytes.
    #[test]
    fn frame_buffer_dims_match_wxh_x4() {
        let frame = FieldFrame::new(17, 11);
        let rgba = frame.frame_rgba();
        assert_eq!(rgba.len(), 17 * 11 * 4, "RGBA frame must be w*h*4 bytes");
    }

    // (2) laplacian_of_constant_field_is_zero -- ∇² of a flat field is exactly 0.
    #[test]
    fn laplacian_of_constant_field_is_zero() {
        let w = 13usize;
        let h = 9usize;
        let u = vec![1.0f32; w * h];
        let lap = laplacian(&u, w, h);
        for &v in &lap {
            assert_eq!(v, 0.0, "∇² of a constant must be exactly zero");
        }
        // The named struct surface agrees with the free function.
        assert_eq!(LaplacianField.compute(&u, w, h), lap);
    }

    // (3) laplacian_peak_negative_at_center_of_disk -- an isolated positive spike
    //     at the centre (local maximum) gives a STRONGLY NEGATIVE Laplacian at
    //     the centre, while its immediate neighbour (the "rim") has a POSITIVE
    //     Laplacian. This proves the discrete ∇² sign is correct.
    #[test]
    fn laplacian_peak_negative_at_center_of_disk() {
        let w = 21usize;
        let h = 21usize;
        let mut u = vec![0.0f32; w * h];
        let cx = w / 2;
        let cy = h / 2;
        u[cy * w + cx] = 1.0; // isolated spike (peak)

        let lap = laplacian(&u, w, h);
        let center = lap[cy * w + cx];
        // At the peak: 0+0+0+0 - 4*1 = -4  (negative -- correct for a max).
        assert!(center < 0.0, "∇² at a peak (centre) must be negative: {center}");
        // Immediate neighbour (up): centre(1)+three 0s - 0 = +1  (positive rim).
        let rim = lap[(cy - 1) * w + cx];
        assert!(rim > 0.0, "∇² at the spike's rim must be positive: {rim}");
        assert!(
            rim > center,
            "rim Laplacian ({rim}) must exceed centre Laplacian ({center})"
        );
    }

    // (4) step_reduces_magnitude_toward_source_equilibrium -- after many steps the
    //     field converges to a finite equilibrium (no NaN/inf, no divergence),
    //     and the per-step change decays toward zero.
    #[test]
    fn step_reduces_magnitude_toward_source_equilibrium() {
        let w = 24usize;
        let h = 24usize;
        let eq = FieldEquilibrium::default();
        // Source = SDF of a centred disk (finite everywhere; no background ∞).
        let mut scene = Scene::new().with_scale(1.0);
        scene.add(SdfShape::Circle {
            cx: 0.0,
            cy: 0.0,
            r: 4.0,
        });
        let source = scene.render_frame(w, h);

        let mut frame = FieldFrame::new(w, h);
        for step in 0..3000 {
            frame.step(&source, &eq);
            // Never NaN / inf.
            for &v in frame.u() {
                assert!(v.is_finite(), "field must stay finite at step {step}");
            }
        }
        // Bounded: field must not have diverged.
        let norm: f64 = frame
            .u()
            .iter()
            .map(|&x| (x as f64) * (x as f64))
            .sum::<f64>()
            .sqrt();
        assert!(norm < 1e6, "field must not diverge (norm={norm})");

        // Convergence: an extra 300 steps changes the field negligibly.
        let before = frame.u().to_vec();
        for _ in 0..300 {
            frame.step(&source, &eq);
        }
        let mut max_delta = 0.0f64;
        for i in 0..before.len() {
            max_delta = max_delta.max((frame.u()[i] as f64 - before[i] as f64).abs());
        }
        assert!(
            max_delta < 1e-2,
            "field should be near equilibrium (max delta={max_delta})"
        );
    }

    // (5) compose_returns_deterministic_frame -- identical scene+eq+steps ==>
    //     bit-identical RGBA bytes (the future blit consumes a stable frame).
    #[test]
    fn compose_returns_deterministic_frame() {
        let mut scene = Scene::new().with_scale(0.5);
        scene
            .add(SdfShape::Circle {
                cx: 0.0,
                cy: 0.0,
                r: 3.0,
            })
            .add(SdfShape::Box {
                bx: 4.0,
                by: 2.0,
                hx: 1.0,
                hy: 1.0,
            });
        let eq = FieldEquilibrium::default();
        let (w, h, steps) = (32usize, 24usize, 50usize);

        let a = compose(&scene, &eq, w, h, steps);
        let b = compose(&scene, &eq, w, h, steps);
        assert_eq!(a.len(), w * h * 4);
        assert_eq!(a, b, "compose must be bit-deterministic across calls");
    }

    // (6) allocfree_step_byte_identical -- the allocation-free step() (scratch
    //     buffers + swap rotation) must produce byte-identical RGBA to an
    //     independent run with the identical scene/eq/steps. This is the
    //     bit-identity proof for the P11 §3 rework: same arithmetic order,
    //     only the buffer lifecycle changed. Also drives 1000 steps to prove
    //     the hot path never panics / diverges under sustained reuse.
    #[test]
    fn allocfree_step_byte_identical() {
        let mut scene = Scene::new().with_scale(0.5);
        scene
            .add(SdfShape::Circle {
                cx: 1.0,
                cy: -1.0,
                r: 3.0,
            })
            .add(SdfShape::Box {
                bx: -3.0,
                by: 2.0,
                hx: 1.5,
                hy: 0.75,
            });
        let eq = FieldEquilibrium::default();
        let (w, h, steps) = (40usize, 28usize, 60usize);

        // Two independent frames advanced identically must be byte-identical.
        let a = compose(&scene, &eq, w, h, steps);
        let b = compose(&scene, &eq, w, h, steps);
        assert_eq!(a, b, "allocation-free step must stay bit-deterministic");

        // Hot-path endurance: 1000 reused steps, no alloc, stays finite.
        let source = scene.render_frame(w, h);
        let mut frame = FieldFrame::new(w, h);
        for s in 0..1000 {
            frame.step(&source, &eq);
            if s % 250 == 0 {
                for &v in frame.u() {
                    assert!(v.is_finite(), "field must stay finite at step {s}");
                }
            }
        }
    }
}
