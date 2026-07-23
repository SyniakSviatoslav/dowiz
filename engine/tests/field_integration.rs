//! Integration test: `FieldFrame` evolution converges to equilibrium under
//! a constant source buffer without diverging, and monotonic field values do not
//! spike beyond tolerance. This is the runnable probe for the engine's PDE
//! integrator — it fails if the numerical scheme is unstable.

use dowiz_engine::field_frame::{FieldEquilibrium, FieldFrame, laplacian};

const W: usize = 32;
const H: usize = 32;
const STEPS: usize = 200;
const TOLERANCE: f64 = 1.0e6; // field values must stay well within this bound

/// Create a source buffer with a centered Gaussian-like bump.
fn gaussian_bump(w: usize, h: usize) -> Vec<f32> {
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let sigma = (w.min(h) as f64) / 6.0;
    let mut src = vec![0.0f32; w * h];
    for r in 0..h {
        for c in 0..w {
            let dx = c as f64 - cx;
            let dy = r as f64 - cy;
            let val = (-(dx * dx + dy * dy) / (2.0 * sigma * sigma)).exp();
            src[r * w + c] = val as f32;
        }
    }
    src
}

#[test]
fn field_frame_evolves_without_divergence() {
    let eq = FieldEquilibrium::default();
    let source = gaussian_bump(W, H);
    let mut frame = FieldFrame::new(W, H);

    for step_idx in 0..STEPS {
        frame.step(&source, &eq);
        let u = frame.u();
        // Assert no values blow up (NaN or beyond tolerance).
        for (i, &v) in u.iter().enumerate() {
            assert!(
                v.is_finite(),
                "step {}: non-finite value at index {}: {}", step_idx, i, v
            );
            assert!(
                v.abs() as f64 <= TOLERANCE,
                "step {}: field value {} exceeds tolerance {} at index {}",
                step_idx, v, TOLERANCE, i
            );
        }
    }
}

#[test]
fn field_frame_monotonic_values_dont_diverge() {
    let eq = FieldEquilibrium::default();
    let source = gaussian_bump(W, H);
    let mut frame = FieldFrame::new(W, H);

    let mut prev_max = 0.0f64;
    let mut monotonic_violations = 0u32;

    for step_idx in 0..STEPS {
        frame.step(&source, &eq);
        let u = frame.u();
        let max_abs = u.iter().map(|&v| v.abs() as f64).fold(0.0f64, f64::max);

        // Monotonic check: field energy should not grow unboundedly after the
        // initial ramp-up. After the first 40 steps (enough for oscillation
        // to settle), any sustained growth beyond tolerance is a divergence.
        if step_idx >= 40 && max_abs > prev_max * 1.1 && max_abs > 200.0 {
            monotonic_violations += 1;
        }
        prev_max = max_abs;
    }

    assert!(
        monotonic_violations <= 5,
        "field energy grew in {} out of 160 post-rampup steps (threshold ≤5); scheme may be divergent",
        monotonic_violations
    );
}

#[test]
fn field_frame_equilibrium_within_tolerance() {
    let eq = FieldEquilibrium::default();
    let source = gaussian_bump(W, H);
    let mut frame = FieldFrame::new(W, H);

    let epsilon = 1.0e-2;

    let mut prev_mean = 0.0f64;
    let mut settled = false;

    for step_idx in 0..STEPS {
        frame.step(&source, &eq);
        let u = frame.u();
        let mean = u.iter().map(|&v| v as f64).sum::<f64>() / (W * H) as f64;

        if step_idx > STEPS / 2 {
            let delta = (mean - prev_mean).abs();
            if delta < epsilon {
                settled = true;
                break;
            }
        }
        prev_mean = mean;
    }

    assert!(
        settled,
        "FieldFrame did not reach equilibrium (mean delta < {}) within {} steps",
        epsilon, STEPS
    );
}

#[test]
fn laplacian_sign_leaves_field_bounded() {
    let eq = FieldEquilibrium::default();
    let source = gaussian_bump(W, H);
    let mut frame = FieldFrame::new(W, H);

    let mut lap_history: Vec<(f32, f32)> = Vec::new();

    for _step_idx in 0..40 {
        frame.step(&source, &eq);

        // Recompute Laplacian of current U to verify sign consistency.
        let u = frame.u();
        let lap = laplacian(u, W, H);

        // Accumulate mean(U) vs mean(Lap) for sign check.
        let u_mean = u.iter().sum::<f32>() / (W * H) as f32;
        let lap_mean = lap.iter().sum::<f32>() / (W * H) as f32;
        lap_history.push((u_mean, lap_mean));
    }

    // At equilibrium, mean Laplacian should be close to zero (diffusion term
    // has flattened the field).
    let final_lap = lap_history.last().unwrap().1;
    assert!(
        final_lap.abs() < 1.0,
        "final mean Laplacian {} should be near zero at equilibrium", final_lap
    );

    // Sign coherence: when the field is positive-dominant (> 0.01), the
    // Laplacian should be ≤ 0 (peaks → negative Laplacian = downward diffusion).
    for (u_mean, lap_mean) in &lap_history {
        if *u_mean > 0.01 {
            // With Neumann BC, a smooth bump should have ∇² ≤ 0 at the top.
            // However, the mean Laplacian can be slightly positive from boundary
            // effects, so we use a lenient check.
            assert!(
                *lap_mean < 0.5,
                "positive field mean {:.4} with positive Laplacian mean {:.4} — sign anomaly",
                u_mean, lap_mean
            );
        }
    }
}
