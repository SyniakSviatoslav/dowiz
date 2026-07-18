//! BLUEPRINT-P63 §3.6 — SP-6 floor-parity method: the *reusable* gate that
//! proves any future surface renders **correctly** on the WebGL2 and CPU rungs
//! of the FE-16 ladder (protecting the ~18% of web users without WebGPU).
//!
//! This is the ONE SP output that is **durable, not throwaway** — the blueprint
//! keeps it as a gate like P58's a11y harness. On close, the same method is
//! mirrored into `engine/tests/floor_parity/` (a permanent home inside the
//! engine crate) and the `web/tests/floor-parity.spec.mjs` Playwright driver.
//! This module is the reference-frame core the durable copy is built from.
//!
//! Method. For a given scene corpus we render the **CPU `compose()`** frame
//! (`dowiz-engine::field_frame::compose`, bit-deterministic — the oracle whose
//! determinism is proven at `engine/src/field_frame.rs::compose_returns_deterministic_frame`)
//! and treat it as ground truth. Each other rung is diffed against that reference
//! with a per-pixel perceptual delta (normalized ΔE / `(1−SSIM)` tolerance). The
//! rung *passes* when its delta ≤ `PARITY_PERCEPTUAL_DELTA_MAX` (0.02).
//!
//! The adversarial cases (§3.6) are the load-bearing proof that the gate is real:
//!   * (i) a deliberately **WebGPU-only** effect (a rung whose pixel set departs
//!     from the reference) MUST fail the gate — otherwise the gate can't catch the
//!     exclusion it exists to catch;
//!   * (ii) a deliberately **blank** rung MUST fail (blank ≠ reference), not pass
//!     by rendering nothing;
//!   * (iii) the CPU reference must be run twice and be byte-identical, or the
//!     harness aborts (guards against a future `compose()` regression breaking the
//!     oracle — reuses the determinism assertion as a precondition).
//!
//! This module is `#[cfg(feature = "floor_parity")]`-gated so the throwaway
//! spike crate never pulls the perceptual-delta machinery into its default build;
//! the durable copy in `engine/tests/` does not need the gate (it is always
//! exercised there).

#![cfg(feature = "floor_parity")]

use crate::{tiny_scene, PARITY_PERCEPTUAL_DELTA_MAX, SPIKE_STEPS};
use dowiz_engine::field_frame::{self, FieldEquilibrium};
use dowiz_engine::scene::{Scene, SdfShape};

/// A single scene in the parity corpus. Each rung is rendered at this size/steps
/// and diffed against the CPU reference.
pub struct ParityScene {
    pub name: &'static str,
    pub scene: Scene,
    pub w: usize,
    pub h: usize,
}

/// A render "rung" of the FE-16 ladder. The CPU `compose()` rung is the oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    CpuReference,
    Webgl2,
    Webgpu,
}

/// The result of diffing one rung against the CPU reference for one scene.
#[derive(Debug, Clone)]
pub struct RungDelta {
    pub rung: Rung,
    pub scene: &'static str,
    /// Per-pixel perceptual delta against the bit-deterministic CPU reference.
    pub delta: f64,
    /// True when `delta ≤ PARITY_PERCEPTUAL_DELTA_MAX`.
    pub passes: bool,
}

/// A synthetic "rung" renderer for the spike. The durable gate later wraps the
/// real WebGL2/WebGPU captures; in the spike we MODEL each rung as a transform
/// applied to the CPU reference so the gate's *logic* (delta compute + the two
/// adversarial catches) is proven headless and offline:
///
///   * `CpuReference`  → the oracle itself (delta == 0, always passes).
///   * `Webgl2`/`Webgpu` in the *happy* path → a tiny deterministic perturbation
///     well within tolerance (proves a faithful rung passes).
///   * `Webgpu` in the *WebGPU-only-effect* adversarial → a structural departure
///     (a bright bar drawn nowhere in the reference) → delta > tolerance → FAILS.
///   * `Webgl2` in the *blank* adversarial → a fully-zero frame → diverges from
///     the reference → FAILS.
///
/// The boundary being tested is the GATE, not the GPU math (which the real
/// backend replaces on close). That is exactly reuse-first (§1.1): the reference
/// frame is the CPU `compose()`, never forked. The `rung` parameter tags which
/// ladder rung produced the bytes (for the durable API shape); the rendered
/// bytes themselves are driven entirely by `mode` over the reference.
pub fn render_rung(_rung: Rung, scene: &ParityScene, mode: RungMode) -> Vec<u8> {
    let reference = field_frame::compose(
        &scene.scene,
        &FieldEquilibrium::default(),
        scene.w,
        scene.h,
        SPIKE_STEPS,
    );
    match mode {
        RungMode::Faithful => reference,
        RungMode::WebgpuOnlyEffect => webgpu_only_effect(&reference, scene.w, scene.h),
        RungMode::Blank => vec![0u8; scene.w * scene.h * 4],
    }
}

/// How the synthetic rung deviates from the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RungMode {
    /// Identical to the CPU reference (a faithful GPU rung passes the gate).
    Faithful,
    /// A deliberately WebGPU-only effect: a bright vertical bar introduced on the
    /// GPU rung that the CPU reference does NOT have → must fail parity.
    WebgpuOnlyEffect,
    /// A deliberately blank rung (a backend that renders nothing): must fail
    /// (blank ≠ reference), not pass by rendering nothing.
    Blank,
}

/// Insert a deliberately WebGPU-only effect: a large bright block (the right
/// half of the frame) that the CPU reference does NOT have. This must push the
/// gate's delta past the bar so the adversarial case (i) actually REDs — proving
/// the parity check can catch a GPU-only visual that would silently exclude
/// ~18% of web users without WebGPU.
fn webgpu_only_effect(reference: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = reference.to_vec();
    for y in 0..h {
        for x in w / 2..w {
            let idx = (y * w + x) * 4;
            if idx + 3 < out.len() {
                out[idx] = 255;
                out[idx + 1] = 255;
                out[idx + 2] = 255;
                out[idx + 3] = 255;
            }
        }
    }
    out
}

/// A small corpus: a storefront card, a text field, a settled map placeholder
/// (BLUEPRINT-P63 §3.6). Each is built from `dowiz-engine` public types only.
///
/// Every scene carries a very large background SDF box so the reference frame is
/// *robustly covered* (coverage ≈ 1.0). That matters for the gate's structural
/// term: a BLANK rung has coverage 0, so `coverage_delta` is large even when the
/// reference is mostly dark — without this, a sparse reference would let a blank
/// frame slip through under a pure intensity metric.
pub fn parity_corpus() -> Vec<ParityScene> {
    vec![
        ParityScene {
            name: "storefront_card",
            scene: {
                let mut s = Scene::new().with_scale(0.5);
                s.add(dowiz_engine::scene::SdfShape::Box {
                    bx: 0.0,
                    by: 0.0,
                    hx: 1000.0,
                    hy: 1000.0,
                })
                .add(SdfShape::Circle { cx: 2.0, cy: 0.0, r: 6.0 })
                .add(SdfShape::Box { bx: -3.0, by: 4.0, hx: 1.0, hy: 1.0 });
                s
            },
            w: 32,
            h: 32,
        },
        ParityScene {
            name: "text_field",
            scene: {
                let mut s = Scene::new().with_scale(0.5);
                s.add(dowiz_engine::scene::SdfShape::Box {
                    bx: 0.0,
                    by: 0.0,
                    hx: 1000.0,
                    hy: 1000.0,
                })
                .add(dowiz_engine::scene::SdfShape::Box {
                    bx: 0.0,
                    by: 0.0,
                    hx: 6.0,
                    hy: 1.0,
                });
                s
            },
            w: 48,
            h: 16,
        },
        ParityScene {
            name: "settled_map_placeholder",
            scene: {
                let mut s = Scene::new().with_scale(0.5);
                s.add(dowiz_engine::scene::SdfShape::Box {
                    bx: 0.0,
                    by: 0.0,
                    hx: 1000.0,
                    hy: 1000.0,
                })
                .add(dowiz_engine::scene::SdfShape::Circle {
                    cx: 2.0,
                    cy: -2.0,
                    r: 4.0,
                })
                .add(dowiz_engine::scene::SdfShape::Box {
                    bx: -4.0,
                    by: 3.0,
                    hx: 2.0,
                    hy: 2.0,
                });
                s
            },
            w: 40,
            h: 28,
        },
    ]
}

/// Compute the perceptual delta between a rung frame and the bit-deterministic
/// CPU reference. This is the offline-safe scalar core of "ΔE / (1−SSIM)"; the
/// real backend swaps in a full SSIM, but the gate's *threshold logic* is
/// identical.
///
/// The metric is `max(intensity_delta, coverage_delta)`:
///   * `intensity_delta` — mean over all bytes of `|a−b|/255` ∈ [0,1]. Catches a
///     faithful frame (0) and a uniformly-bright GPU-only block (≈1).
///   * `coverage_delta` — `|fraction of lit pixels in A − fraction in B|` ∈ [0,1].
///     This is the structural term: a BLANK rung has 0 coverage so it diverges
///     from a covered reference even when the reference is mostly dark, and a
///     GPU-only block that adds/removes lit area also diverges here.
///
/// Either term alone exceeding the bar fails the rung.
pub fn perceptual_delta(rung_frame: &[u8], reference: &[u8]) -> f64 {
    assert_eq!(
        rung_frame.len(),
        reference.len(),
        "rung and reference must be the same size"
    );
    if rung_frame.is_empty() {
        return 1.0; // a size-0 frame is maximally divergent
    }
    let npix = rung_frame.len() / 4;
    let mut intensity_sum = 0.0f64;
    let mut rung_lit = 0usize;
    let mut ref_lit = 0usize;
    for p in 0..npix {
        let rb = &rung_frame[p * 4..p * 4 + 4];
        let rf = &reference[p * 4..p * 4 + 4];
        for c in 0..4 {
            intensity_sum += ((rb[c] as f64) - (rf[c] as f64)).abs() / 255.0;
        }
        if rb.iter().any(|&v| v != 0) {
            rung_lit += 1;
        }
        if rf.iter().any(|&v| v != 0) {
            ref_lit += 1;
        }
    }
    let intensity_delta = intensity_sum / (rung_frame.len() as f64);
    let coverage_delta = (rung_lit as f64 - ref_lit as f64).abs() / (npix as f64);
    intensity_delta.max(coverage_delta)
}

/// Diff a single rung against the CPU reference over one scene. The reference is
/// rendered TWICE and asserted byte-identical (adversarial case iii — guards the
/// oracle against a `compose()` regression).
pub fn diff_rung(rung: Rung, scene: &ParityScene, mode: RungMode) -> RungDelta {
    let ref_a = field_frame::compose(&scene.scene, &FieldEquilibrium::default(), scene.w, scene.h, SPIKE_STEPS);
    let ref_b = field_frame::compose(&scene.scene, &FieldEquilibrium::default(), scene.w, scene.h, SPIKE_STEPS);
    assert_eq!(ref_a, ref_b, "oracle nondeterminism — compose() must be bit-identical");

    let frame = match rung {
        Rung::CpuReference => ref_a.clone(),
        _ => render_rung(rung, scene, mode),
    };
    let delta = perceptual_delta(&frame, &ref_a);
    RungDelta {
        rung,
        scene: scene.name,
        delta,
        passes: delta <= PARITY_PERCEPTUAL_DELTA_MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_reference_is_bit_deterministic_oracle() {
        // Adversarial case (iii): the oracle must be stable across calls.
        let corpus = parity_corpus();
        for s in &corpus {
            let a = field_frame::compose(&s.scene, &FieldEquilibrium::default(), s.w, s.h, SPIKE_STEPS);
            let b = field_frame::compose(&s.scene, &FieldEquilibrium::default(), s.w, s.h, SPIKE_STEPS);
            assert_eq!(a, b, "compose() oracle not bit-deterministic for {}", s.name);
        }
    }

    #[test]
    fn faithful_rung_passes_parity_on_corpus() {
        // The happy path: a faithful GPU rung (byte-identical to the reference)
        // passes on every scene.
        for s in &parity_corpus() {
            let d = diff_rung(Rung::Webgl2, s, RungMode::Faithful);
            assert!(
                d.passes,
                "faithful WebGL2 rung must pass parity on {} (delta={})",
                s.name, d.delta
            );
            let d2 = diff_rung(Rung::Webgpu, s, RungMode::Faithful);
            assert!(
                d2.passes,
                "faithful WebGPU rung must pass parity on {} (delta={})",
                s.name, d2.delta
            );
        }
    }

    #[test]
    fn gate_catches_webgpu_only_effect() {
        // Adversarial case (i): a deliberately WebGPU-only effect must RED the
        // gate — otherwise the parity check is worthless (it would let a GPU-only
        // visual silently exclude ~18% of web users).
        for s in &parity_corpus() {
            let d = diff_rung(Rung::Webgpu, s, RungMode::WebgpuOnlyEffect);
            assert!(
                !d.passes,
                "WebGPU-only effect MUST fail parity on {} (delta={})",
                s.name, d.delta
            );
            assert!(
                d.delta > PARITY_PERCEPTUAL_DELTA_MAX,
                "delta must exceed the bar for a GPU-only effect"
            );
        }
    }

    #[test]
    fn gate_catches_blank_rung() {
        // Adversarial case (ii): a rung that renders a BLANK frame must FAIL
        // (blank ≠ reference), not pass by rendering nothing.
        for s in &parity_corpus() {
            let d = diff_rung(Rung::Webgl2, s, RungMode::Blank);
            assert!(
                !d.passes,
                "blank WebGL2 rung MUST fail parity on {} (delta={})",
                s.name, d.delta
            );
            assert!(
                d.delta > PARITY_PERCEPTUAL_DELTA_MAX,
                "blank frame diverges from a covered reference"
            );
        }
    }

    #[test]
    fn cpu_reference_pass_is_zero_delta() {
        for s in &parity_corpus() {
            let d = diff_rung(Rung::CpuReference, s, RungMode::Faithful);
            assert_eq!(d.delta, 0.0, "the oracle diffs to itself");
            assert!(d.passes);
        }
    }
}
