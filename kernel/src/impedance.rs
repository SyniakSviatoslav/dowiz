//! impedance.rs — C2: circuit/impedance as a resource framework (Master-Integration).
//!
//! LENS (from the plan, made executable): software flow is NOT a resistor — it
//! is a driven transmission line where we WANT reflection coefficient |ρ| < 1
//! with margin, and we GATE on it (backpressure) rather than matching power.
//! The plan is explicit that literal "max-power-transfer impedance matching"
//! misleads here; the real invariant is `ρ < 1 − margin` (stable, no runaway
//! reflection) and a two-phase write→verify→drop discipline.
//!
//! We implement the kernel-side primitive: given an arrival process described by
//! its load ratio `ρ = λ/μ` (offered / service rate) and a burstiness factor
//! `k` (peak/average, a "surge impedance"), compute the effective reflection
//! coefficient and a hard backpressure verdict. Deterministic, offline, std-only.

/// Reflection coefficient of a driven line with load ratio ρ and surge factor k.
/// Model: ρ drives utilization; k is the mismatch (burstiness). A smooth,
/// monotonic proxy for "how much load bounces back" — bounded in [0,1) for
/// stable ρ<1, →1 as ρ→1 (the line saturates and reflects everything).
pub fn reflection_coefficient(rho: f64, k: f64) -> f64 {
    if rho <= 0.0 {
        return 0.0;
    }
    let r = rho.min(1.0);
    let burst = k.max(1.0); // burstiness ≥ 1 (k<1 clamped); worse ⇒ more reflection
    // ρ²·burstiness: light load ⇒ small (ρ=0.3 → 0.09), saturation ⇒ →1
    // (ρ→1, burst=1 ⇒ 1), burstiness amplifies a moderate load. Bounded <1.
    (r * r * burst).min(0.99999)
}

/// Backpressure verdict for a flow with offered/service ratio `rho`, burstiness
/// `k`, and required stability `margin` (e.g. 0.1). Stable ⇔ |ρ_eff| < 1−margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowGate {
    /// ρ with margin ⇒ admit (no backpressure).
    Admit,
    /// reflection too high ⇒ apply backpressure (drop/shed/queuescale).
    Backpressure,
}

pub fn gate(rho: f64, k: f64, margin: f64) -> FlowGate {
    let rho_eff = reflection_coefficient(rho, k);
    if rho_eff < 1.0 - margin {
        FlowGate::Admit
    } else {
        FlowGate::Backpressure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HAND ORACLE 1 — light load, no burst: ρ=0.3, k=1 ⇒ low reflection ⇒ admit.
    #[test]
    fn light_load_admits() {
        let r = reflection_coefficient(0.3, 1.0);
        assert!(r < 0.1, "got {}", r);
        assert_eq!(gate(0.3, 1.0, 0.1), FlowGate::Admit);
    }

    /// HAND ORACLE 2 — saturation: ρ→1 reflects almost everything ⇒ backpressure.
    #[test]
    fn saturation_backpressures() {
        let r = reflection_coefficient(0.999, 1.0);
        assert!(r > 0.9, "got {}", r);
        assert_eq!(gate(0.999, 1.0, 0.1), FlowGate::Backpressure);
    }

    /// HAND ORACLE 3 — burstiness degrades a moderate load: ρ=0.7,k=3 should
    /// backpressure where ρ=0.7,k=1 admitted.
    #[test]
    fn burstiness_worsens() {
        let calm = reflection_coefficient(0.7, 1.0);
        let bursty = reflection_coefficient(0.7, 3.0);
        assert!(bursty > calm, "bursty {} not > calm {}", bursty, calm);
        assert_eq!(gate(0.7, 1.0, 0.1), FlowGate::Admit);
        assert_eq!(gate(0.7, 3.0, 0.1), FlowGate::Backpressure);
    }

    /// Monotonic in ρ: more load ⇒ more reflection (for fixed k).
    #[test]
    fn monotonic_in_rho() {
        let a = reflection_coefficient(0.2, 1.5);
        let b = reflection_coefficient(0.6, 1.5);
        let c = reflection_coefficient(0.9, 1.5);
        assert!(a < b && b < c);
    }

    /// Zero offered load ⇒ zero reflection (idle line reflects nothing).
    #[test]
    fn zero_load_zero_reflection() {
        assert_eq!(reflection_coefficient(0.0, 5.0), 0.0);
    }

    /// Determinism.
    #[test]
    fn deterministic() {
        let a = gate(0.8, 2.0, 0.05);
        let b = gate(0.8, 2.0, 0.05);
        assert_eq!(a, b);
    }
}
