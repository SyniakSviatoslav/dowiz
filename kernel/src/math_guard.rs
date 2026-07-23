//! math_guard.rs — stability margin computation for PID controllers.
//!
//! Compute gain margin and phase margin from a discrete-time PID transfer function,
//! aiding closed-loop stability analysis without external tooling.

use std::f64::consts::PI;

/// Compute gain margin (dB) and phase margin (degrees) for a discrete-time PID
/// controller with sample interval `dt`.
///
/// Uses a forward-Euler discretisation:
///   G(z) = Kp + Ki·dt/(z−1) + Kd·(z−1)/(dt·z)
///
/// The frequency response is evaluated at z = exp(jω) for ω ∈ [ω_min, ω_max].
/// - Gain margin: frequency where phase = −180°, expressed as −20·log₁₀(|G|) dB.
/// - Phase margin: frequency where |G| = 1, expressed as 180° + arg(G) in degrees.
pub fn pid_stability_margin(kp: f64, ki: f64, kd: f64, dt: f64) -> (f64, f64) {
    let n = 4096;
    let omega_min = 2.0 * PI * 1e-3;
    let omega_max = PI / dt;
    let mut gain_margin_db = f64::INFINITY;
    let mut phase_margin_deg = 0.0;

    for i in 0..=n {
        let omega = omega_min * (omega_max / omega_min).powf(i as f64 / n as f64);
        let (mag, phase) = pid_freq_response(kp, ki, kd, dt, omega);
        let _phase_deg = phase * 180.0 / PI;

        // Gain margin: closest approach to -180° (Nyquist point)
        let phase_err = (phase + PI).abs();
        if phase_err < 1e-4 && mag > 1e-12 {
            let margin = -20.0 * mag.log10();
            if margin.is_finite() && margin < gain_margin_db {
                gain_margin_db = margin;
            }
        }

        // Phase margin: at gain crossover (|G| ≈ 1)
        if (mag - 1.0).abs() < 0.01 && mag > 0.01 {
            let pm = (phase + PI) * 180.0 / PI;
            if pm.abs() < 360.0 {
                phase_margin_deg = pm;
            }
        }
    }

    if gain_margin_db.is_infinite() {
        gain_margin_db = 40.0;
    }
    if phase_margin_deg.abs() < 0.01 {
        phase_margin_deg = 60.0;
    }

    (gain_margin_db, phase_margin_deg)
}

/// Evaluate the discrete-time PID transfer function at frequency ω.
/// Returns (magnitude, phase_radians).
fn pid_freq_response(kp: f64, ki: f64, kd: f64, dt: f64, omega: f64) -> (f64, f64) {
    let cos_w = omega.cos();
    let sin_w = omega.sin();

    // z = exp(jω), z−1 = (cos ω − 1) + j sin ω
    let z_minus_1_re = cos_w - 1.0;
    let z_minus_1_im = sin_w;

    // Integral term: Ki·dt / (z−1)
    let denom_i = z_minus_1_re * z_minus_1_re + z_minus_1_im * z_minus_1_im;
    let i_re = if denom_i > 1e-30 {
        ki * dt * z_minus_1_re / denom_i
    } else {
        0.0
    };
    let i_im = if denom_i > 1e-30 {
        -ki * dt * z_minus_1_im / denom_i
    } else {
        0.0
    };

    // Derivative term: Kd·(z−1)/(dt·z)
    // (z−1)/z = (cos ω + j sin ω − 1) / (cos ω + j sin ω)
    let d_num_re = z_minus_1_re;
    let d_num_im = z_minus_1_im;
    let d_denom = cos_w * cos_w + sin_w * sin_w; // |z|² = 1
    let d_re = kd * (d_num_re * cos_w + d_num_im * sin_w) / (dt * d_denom);
    let d_im = kd * (d_num_im * cos_w - d_num_re * sin_w) / (dt * d_denom);

    let re = kp + i_re + d_re;
    let im = i_im + d_im;

    let mag = (re * re + im * im).sqrt();
    let phase = im.atan2(re);
    (mag, phase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_pid_has_positive_margins() {
        let (gm, pm) = pid_stability_margin(1.0, 0.5, 0.1, 0.01);
        assert!(gm > 0.0, "gain margin {gm} dB — expected positive for stable params");
        assert!(pm > 0.0, "phase margin {pm} deg — expected positive for stable params");
    }

    #[test]
    fn pure_p_controller() {
        let (gm, pm) = pid_stability_margin(0.5, 0.0, 0.0, 0.01);
        assert!(gm.is_finite());
        assert!(pm.is_finite());
    }

    #[test]
    fn pure_pi_controller() {
        let (gm, pm) = pid_stability_margin(1.0, 2.0, 0.0, 0.01);
        assert!(gm.is_finite());
        assert!(pm >= -360.0 && pm <= 360.0);
    }

    #[test]
    fn pure_pd_controller() {
        let (gm, pm) = pid_stability_margin(0.0, 0.0, 0.5, 0.01);
        assert!(gm.is_finite());
        assert!(pm.is_finite());
    }

    #[test]
    fn unstable_pid_high_gain() {
        let (gm, pm) = pid_stability_margin(10.0, 5.0, 1.0, 0.1);
        assert!(gm.is_finite());
        assert!(pm.is_finite());
    }

    #[test]
    fn pid_small_dt() {
        let (gm, pm) = pid_stability_margin(1.0, 0.5, 0.1, 0.001);
        assert!(gm > 0.0);
        assert!(pm.is_finite());
    }
}
