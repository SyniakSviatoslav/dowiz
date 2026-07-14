//! FE-03 — Fixed-timestep accumulator loop.
//!
//! RED→GREEN GATE (per blueprint): simulate a stutter (e.g. 50ms injected frame
//! time). The RED variable-dt integrator sees 0.05 → diverges (field tested
//! divergent at 0.05). The GREEN fixed-timestep loop clamps frame time and only
//! ever feeds the integrator the compile-constant `DT_STABLE`; interpolation
//! (`alpha = accum / DT`) keeps render smooth at 60fps while physics steps at
//! a fixed 50Hz.
//!
//! Guards: `MAX_FRAME` clamp (spiral-of-death), `MAX_SUBSTEPS` cap, `dt` is a
//! compile-const equal to the kernel's `DT_STABLE`
//! (`dowiz-kernel/src/lib.rs::DT_STABLE` — the authoritative source of truth).

/// Fixed physics step — MUST equal `dowiz-kernel::DT_STABLE`
/// (`kernel/src/lib.rs`) so the integrator never sees a divergent dt (kernel is
/// the authority on stability). The kernel pins its value with
/// `dt_stable_is_authoritative`; this mirror pin catches drift on the engine
/// side. If you change one, change both.
pub const DT_STABLE: f32 = 0.02;

/// Clamp for a single rendered frame time (guards spiral-of-death).
pub const MAX_FRAME: f32 = 0.25;

/// Cap on substeps consumed per rendered frame.
pub const MAX_SUBSTEPS: u32 = 5;

/// Records every `dt` actually handed to the integrator — the falsifiable
/// evidence that the loop honors the fixed-step contract under stutter.
pub struct FixedTimestep {
    accumulator: f32,
    /// Every dt the integrator was called with this run (should all be DT_STABLE).
    seen_dts: Vec<f32>,
    /// Interpolation factor for the most recent render.
    pub last_alpha: f32,
    /// True if the MAX_FRAME clamp engaged (a dropped frame was capped).
    pub clamped: bool,
}

impl FixedTimestep {
    pub fn new() -> Self {
        FixedTimestep {
            accumulator: 0.0,
            seen_dts: Vec::new(),
            last_alpha: 0.0,
            clamped: false,
        }
    }

    /// Advance one rendered frame given `frame_time` (seconds, real wall clock,
    /// possibly janky). `step` is called with the FIXED dt; `render(alpha)` is
    /// called once with the interpolation factor.
    pub fn frame<F, R>(&mut self, frame_time: f32, mut step: F, mut render: R)
    where
        F: FnMut(f32),
        R: FnMut(f32),
    {
        // 1) clamp frame time to avoid spiral-of-death.
        let ft = if frame_time > MAX_FRAME {
            self.clamped = true;
            MAX_FRAME
        } else {
            frame_time
        };
        self.accumulator += ft;

        // 2) consume fixed steps — integrator ONLY ever sees DT_STABLE.
        let mut substeps = 0u32;
        while self.accumulator >= DT_STABLE && substeps < MAX_SUBSTEPS {
            step(DT_STABLE);
            self.seen_dts.push(DT_STABLE);
            self.accumulator -= DT_STABLE;
            substeps += 1;
        }
        // If we hit the substep cap, drop the leftover accumulator (keep stable).
        if substeps == MAX_SUBSTEPS && self.accumulator > DT_STABLE {
            self.accumulator = 0.0;
        }

        // 3) interpolation factor for smooth render between prev/curr.
        self.last_alpha = (self.accumulator / DT_STABLE).clamp(0.0, 1.0);
        render(self.last_alpha);
    }

    /// Max dt actually delivered to the integrator this run.
    pub fn max_seen_dt(&self) -> f32 {
        self.seen_dts.iter().cloned().fold(0.0_f32, f32::max)
    }

    /// Min dt actually delivered to the integrator this run.
    pub fn min_seen_dt(&self) -> f32 {
        self.seen_dts.iter().cloned().fold(f32::INFINITY, f32::min)
    }
}

impl Default for FixedTimestep {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RED→GREEN: under a 50ms stutter, the integrator ONLY ever sees DT_STABLE
    // (0.02). A variable-dt loop would hand it 0.05 → diverge.
    #[test]
    fn stutter_never_feeds_divergent_dt() {
        let mut loop_ = FixedTimestep::new();
        let mut steps = 0u32;
        // A janky 50ms frame followed by smooth 16.6ms frames.
        let frames = [0.050_f32, 0.0166, 0.0166, 0.0166, 0.050, 0.0166];
        for ft in frames {
            loop_.frame(ft, |_dt| steps += 1, |_a| {});
        }
        assert_eq!(
            loop_.max_seen_dt(),
            DT_STABLE,
            "integrator never receives a divergent dt (RED path would see 0.05)"
        );
        assert_eq!(
            loop_.min_seen_dt(),
            DT_STABLE,
            "every step is exactly DT_STABLE"
        );
        assert!(steps > 0);
    }

    // Smooth 60fps: alpha interpolates, dt stays fixed.
    #[test]
    fn smooth_60fps_fixed_step_with_interpolation() {
        let mut loop_ = FixedTimestep::new();
        let mut alphas = Vec::new();
        for _ in 0..10 {
            loop_.frame(1.0 / 60.0, |_dt| {}, |a| alphas.push(a));
        }
        assert!(
            alphas.iter().all(|&a| (0.0..=1.0).contains(&a)),
            "interpolation alpha stays within [0,1]"
        );
        assert_eq!(loop_.max_seen_dt(), DT_STABLE);
    }

    // Spiral-of-death guard: a pathological 10s frame must not loop forever.
    #[test]
    fn pathological_frame_does_not_infinite_loop() {
        let mut loop_ = FixedTimestep::new();
        let mut steps = 0u32;
        loop_.frame(10.0, |_dt| steps += 1, |_a| {});
        assert!(
            steps <= MAX_SUBSTEPS,
            "substep cap honored — no spiral of death"
        );
        assert!(loop_.clamped);
    }

    // Mirror pin for the kernel↔engine DT_STABLE contract. The authoritative
    // value lives in `dowiz-kernel::DT_STABLE` (kernel/src/lib.rs). This catches
    // drift on the engine side; the kernel's `dt_stable_is_authoritative` catches
    // it on the kernel side. Both must stay 0.02 (50 Hz) or the integrator
    // desyncs from the kernel's route-kinematics sampling cadence.
    #[test]
    fn dt_stable_matches_kernel_contract() {
        assert_eq!(DT_STABLE, 0.02);
        assert_eq!((1.0 / DT_STABLE as f64).round() as u32, 50);
    }
}
