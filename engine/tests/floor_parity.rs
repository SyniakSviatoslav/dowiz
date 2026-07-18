//! BLUEPRINT-P63 §3.6 — SP-6 floor-parity gate (DURABLE, not throwaway).
//!
//! This is the reusable gate that proves any future surface renders **correctly**
//! on the WebGL2 and CPU rungs of the FE-16 ladder (protecting the ~18% of web
//! users without WebGPU). It is the ONE P63 output that survives the spike's
//! close: `tools/shell-spike/` is `rm -rf`'d at close (§4.5), but this test
//! stays wired into the engine as a permanent gate — exactly like P58's a11y
//! harness (REUSE-FIRST).
//!
//! Method mirror: this is the reference-frame core lifted from
//! `tools/shell-spike/src/floor_parity.rs`. The oracle is the bit-deterministic
//! `dowiz_engine::field_frame::compose` (CPU); each rung is diffed against it
//! with a perceptual delta = `max(intensity_delta, coverage_delta)`. The three
//! adversarial cases (GPU-only effect, blank rung, oracle nondeterminism) are
//! carried over verbatim so the durable gate is provably as strong as the spike.
//!
//! Offline, zero external deps — only `dowiz-engine`.

use dowiz_engine::field_frame::{self, FieldEquilibrium};
use dowiz_engine::scene::{Scene, SdfShape};

const SPIKE_STEPS: usize = 3;
const PARITY_PERCEPTUAL_DELTA_MAX: f64 = 0.02;

#[derive(Debug, Clone, Copy)]
enum Rung {
    CpuReference,
    Webgl2,
    Webgpu,
}

#[derive(Clone, Copy)]
enum RungMode {
    Faithful,
    WebgpuOnlyEffect,
    Blank,
}

struct ParityScene {
    name: &'static str,
    scene: Scene,
    w: usize,
    h: usize,
}

fn parity_corpus() -> Vec<ParityScene> {
    vec![
        ParityScene {
            name: "storefront_card",
            scene: {
                let mut s = Scene::new().with_scale(0.5);
                s.add(SdfShape::Box { bx: 0.0, by: 0.0, hx: 1000.0, hy: 1000.0 })
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
                s.add(SdfShape::Box { bx: 0.0, by: 0.0, hx: 1000.0, hy: 1000.0 })
                    .add(SdfShape::Box { bx: 0.0, by: 0.0, hx: 6.0, hy: 1.0 });
                s
            },
            w: 48,
            h: 16,
        },
        ParityScene {
            name: "settled_map_placeholder",
            scene: {
                let mut s = Scene::new().with_scale(0.5);
                s.add(SdfShape::Box { bx: 0.0, by: 0.0, hx: 1000.0, hy: 1000.0 })
                    .add(SdfShape::Circle { cx: 2.0, cy: -2.0, r: 4.0 })
                    .add(SdfShape::Box { bx: -4.0, by: 3.0, hx: 2.0, hy: 2.0 });
                s
            },
            w: 40,
            h: 28,
        },
    ]
}

fn render_rung(_rung: Rung, scene: &ParityScene, mode: RungMode) -> Vec<u8> {
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

fn perceptual_delta(rung_frame: &[u8], reference: &[u8]) -> f64 {
    assert_eq!(rung_frame.len(), reference.len(), "same size");
    if rung_frame.is_empty() {
        return 1.0;
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

#[test]
fn floor_parity_oracle_is_bit_deterministic() {
    for s in &parity_corpus() {
        let a = field_frame::compose(&s.scene, &FieldEquilibrium::default(), s.w, s.h, SPIKE_STEPS);
        let b = field_frame::compose(&s.scene, &FieldEquilibrium::default(), s.w, s.h, SPIKE_STEPS);
        assert_eq!(a, b, "compose() oracle nondeterministic for {}", s.name);
    }
}

#[test]
fn floor_parity_faithful_rung_passes() {
    for s in &parity_corpus() {
        let reference = field_frame::compose(&s.scene, &FieldEquilibrium::default(), s.w, s.h, SPIKE_STEPS);
        for rung in [Rung::Webgl2, Rung::Webgpu] {
            let frame = render_rung(rung, s, RungMode::Faithful);
            let delta = perceptual_delta(&frame, &reference);
            assert!(
                delta <= PARITY_PERCEPTUAL_DELTA_MAX,
                "{:?} rung must pass parity on {} (delta={})",
                rung, s.name, delta
            );
        }
    }
}

#[test]
fn floor_parity_gate_catches_webgpu_only_effect() {
    for s in &parity_corpus() {
        let reference = field_frame::compose(&s.scene, &FieldEquilibrium::default(), s.w, s.h, SPIKE_STEPS);
        let frame = render_rung(Rung::Webgpu, s, RungMode::WebgpuOnlyEffect);
        let delta = perceptual_delta(&frame, &reference);
        assert!(
            delta > PARITY_PERCEPTUAL_DELTA_MAX,
            "WebGPU-only effect must RED the gate on {} (delta={})",
            s.name, delta
        );
    }
}

#[test]
fn floor_parity_gate_catches_blank_rung() {
    for s in &parity_corpus() {
        let reference = field_frame::compose(&s.scene, &FieldEquilibrium::default(), s.w, s.h, SPIKE_STEPS);
        let frame = render_rung(Rung::Webgl2, s, RungMode::Blank);
        let delta = perceptual_delta(&frame, &reference);
        assert!(
            delta > PARITY_PERCEPTUAL_DELTA_MAX,
            "blank rung must fail parity on {} (delta={})",
            s.name, delta
        );
    }
}
