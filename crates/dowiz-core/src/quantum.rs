//! quantum.rs — minimal quantum-state primitives for the no_std core.
//!
//! Implements the single-qubit state |ψ⟩ = α|0⟩ + β|1⟩ (α, β ∈ ℂ, |α|²+|β|²=1),
//! the Bloch-sphere decomposition, the standard Clifford+rotation gate set
//! (Pauli X/Y/Z, Hadamard, S/T phase, RX/RY/RZ), Born-rule measurement, and a
//! two-qubit register with CNOT + Bell states (entanglement).
//!
//! All complex arithmetic routes through [`crate::complex::Complex`] and the
//! bit-exact real functions in [`crate::math`] (no `num-complex`, no std).
//!
//! Reference (AI-tools compendium, Part IV.6–8): superposition, entanglement,
//! measurement collapse, Bloch sphere, quantum circuit model.

use crate::complex::Complex;
use crate::tri_state::TriState;
use alloc::vec::Vec;

/// Phase factor e^{iφ} = cos φ + i sin φ.
fn phase(phi: f64) -> Complex {
    Complex::new(crate::math::cos(phi), crate::math::sin(phi))
}

/// Scale a complex number by a real scalar.
fn scale(z: Complex, s: f64) -> Complex {
    Complex::new(z.re * s, z.im * s)
}

/// A single qubit state |ψ⟩ = α|0⟩ + β|1⟩.
///
/// Amplitudes are stored un-normalized; call [`Qubit::normalize`] to project
/// back onto the unit sphere after composing non-unitary operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Qubit {
    pub alpha: Complex,
    pub beta: Complex,
}

impl Qubit {
    /// Raw constructor (no normalization — the caller owns the invariant).
    pub const fn new(alpha: Complex, beta: Complex) -> Self {
        Self { alpha, beta }
    }

    /// |0⟩ basis state.
    pub fn zero() -> Self {
        Self::new(Complex::new(1.0, 0.0), Complex::new(0.0, 0.0))
    }

    /// |1⟩ basis state.
    pub fn one() -> Self {
        Self::new(Complex::new(0.0, 0.0), Complex::new(1.0, 0.0))
    }

    /// Equal superposition |+⟩ = (|0⟩ + |1⟩)/√2.
    pub fn plus() -> Self {
        let s = crate::math::sqrt(0.5);
        Self::new(Complex::new(s, 0.0), Complex::new(s, 0.0))
    }

    /// Equal superposition |-⟩ = (|0⟩ − |1⟩)/√2.
    pub fn minus() -> Self {
        let s = crate::math::sqrt(0.5);
        Self::new(Complex::new(s, 0.0), Complex::new(-s, 0.0))
    }

    /// Superposition |i⟩ = (|0⟩ + i|1⟩)/√2.
    pub fn plus_i() -> Self {
        let s = crate::math::sqrt(0.5);
        Self::new(Complex::new(s, 0.0), Complex::new(0.0, s))
    }

    /// Superposition |−i⟩ = (|0⟩ − i|1⟩)/√2.
    pub fn minus_i() -> Self {
        let s = crate::math::sqrt(0.5);
        Self::new(Complex::new(s, 0.0), Complex::new(0.0, -s))
    }

    /// Euclidean norm √(|α|² + |β|²).
    pub fn norm(self) -> f64 {
        crate::math::hypot(self.alpha.abs(), self.beta.abs())
    }

    /// Project onto the unit sphere (divide by [`Qubit::norm`]).
    pub fn normalize(mut self) -> Self {
        let n = self.norm();
        if n != 0.0 {
            self.alpha = scale(self.alpha, 1.0 / n);
            self.beta = scale(self.beta, 1.0 / n);
        }
        self
    }

    /// Probability of measuring |0⟩ = |α|².
    pub fn prob0(self) -> f64 {
        self.alpha.re * self.alpha.re + self.alpha.im * self.alpha.im
    }

    /// Probability of measuring |1⟩ = |β|².
    pub fn prob1(self) -> f64 {
        self.beta.re * self.beta.re + self.beta.im * self.beta.im
    }

    /// Born-rule measurement in the computational basis.
    ///
    /// `p` is a uniform sample in [0, 1) supplied by the caller (keeps the
    /// function deterministic and testable). Returns `0` when `p < |α|²`, else
    /// `1`.
    pub fn measure(self, p: f64) -> u8 {
        if p < self.prob0() {
            0
        } else {
            1
        }
    }

    /// Bloch-sphere angles (θ, φ) with θ ∈ [0, π], φ ∈ [0, 2π).
    ///
    /// |ψ⟩ = cos(θ/2)|0⟩ + e^{iφ} sin(θ/2)|1⟩. θ is recovered from the
    /// magnitude 2·acos(|α|); φ is the relative phase arg(β) − arg(α).
    pub fn bloch(self) -> (f64, f64) {
        let r = self.alpha.abs().min(1.0);
        let theta = 2.0 * crate::math::acos(r);
        let phi = crate::math::rem_euclid(
            self.beta.arg() - self.alpha.arg(),
            2.0 * crate::constants::TAU,
        );
        (theta, phi)
    }

    /// Inner product ⟨ψ|φ⟩ = conj(α)·α′ + conj(β)·β′.
    pub fn inner(self, other: Qubit) -> Complex {
        self.alpha
            .conj()
            .mul(other.alpha)
            .add(self.beta.conj().mul(other.beta))
    }

    /// Fidelity |⟨ψ|φ⟩|² between two pure states.
    pub fn fidelity(self, other: Qubit) -> f64 {
        let ip = self.inner(other);
        ip.re * ip.re + ip.im * ip.im
    }

    // — Gate set (all unitary, all preserve the norm for normalized input) —

    /// Pauli-X (NOT / bit-flip): |0⟩ ↔ |1⟩.
    pub fn apply_x(mut self) -> Self {
        core::mem::swap(&mut self.alpha, &mut self.beta);
        self
    }

    /// Pauli-Y: Y|ψ⟩ = −iβ|0⟩ + iα|1⟩.
    pub fn apply_y(self) -> Self {
        // −i·β and i·α
        Self::new(
            Complex::new(self.beta.im, -self.beta.re),
            Complex::new(-self.alpha.im, self.alpha.re),
        )
    }

    /// Pauli-Z (phase-flip): Z|ψ⟩ = α|0⟩ − β|1⟩.
    pub fn apply_z(mut self) -> Self {
        self.beta = Complex::new(-self.beta.re, -self.beta.im);
        self
    }

    /// Hadamard: H|ψ⟩ = ((α+β)|0⟩ + (α−β)|1⟩)/√2.
    pub fn apply_h(self) -> Self {
        let s = crate::math::sqrt(0.5);
        Self::new(
            scale(self.alpha.add(self.beta), s),
            scale(self.alpha.sub(self.beta), s),
        )
    }

    /// S (π/2 phase): S|ψ⟩ = α|0⟩ + iβ|1⟩.
    pub fn apply_s(mut self) -> Self {
        self.beta = Complex::new(-self.beta.im, self.beta.re);
        self
    }

    /// T (π/4 phase): T|ψ⟩ = α|0⟩ + e^{iπ/4}β|1⟩.
    pub fn apply_t(mut self) -> Self {
        self.beta = phase(crate::constants::PI * 0.25).mul(self.beta);
        self
    }

    /// Rotation about X: RX(θ) = e^{−iθX/2}.
    pub fn apply_rx(self, theta: f64) -> Self {
        let c = crate::math::cos(theta * 0.5);
        let s = crate::math::sin(theta * 0.5);
        // α′ = c·α − i·s·β ; β′ = −i·s·α + c·β
        let i_s_beta = Complex::new(s * self.beta.im, -s * self.beta.re); // i·s·β
        let i_s_alpha = Complex::new(s * self.alpha.im, -s * self.alpha.re); // i·s·α
        Self::new(
            scale(self.alpha, c).sub(i_s_beta),
            scale(self.beta, c).sub(i_s_alpha),
        )
    }

    /// Rotation about Y: RY(θ) = e^{−iθY/2}.
    pub fn apply_ry(self, theta: f64) -> Self {
        let c = crate::math::cos(theta * 0.5);
        let s = crate::math::sin(theta * 0.5);
        // α′ = c·α − s·β ; β′ = s·α + c·β
        Self::new(
            scale(self.alpha, c).sub(scale(self.beta, s)),
            scale(self.alpha, s).add(scale(self.beta, c)),
        )
    }

    /// Rotation about Z: RZ(φ) = e^{−iφZ/2}.
    pub fn apply_rz(self, phi: f64) -> Self {
        // α′ = e^{−iφ/2}·α ; β′ = e^{iφ/2}·β
        Self::new(
            phase(-phi * 0.5).mul(self.alpha),
            phase(phi * 0.5).mul(self.beta),
        )
    }
}

/// A two-qubit register: amplitudes for |00⟩, |01⟩, |10⟩, |11⟩.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoQubit {
    /// a[0]=|00⟩, a[1]=|01⟩, a[2]=|10⟩, a[3]=|11⟩.
    pub a: [Complex; 4],
}

impl TwoQubit {
    pub const fn new(a: [Complex; 4]) -> Self {
        Self { a }
    }

    /// |00⟩ ground state.
    pub fn zero() -> Self {
        let mut a = [Complex::new(0.0, 0.0); 4];
        a[0] = Complex::new(1.0, 0.0);
        Self { a }
    }

    /// Euclidean norm √(Σ |a_i|²).
    pub fn norm(self) -> f64 {
        let s = self
            .a
            .iter()
            .fold(0.0, |acc, z| acc + z.re * z.re + z.im * z.im);
        crate::math::sqrt(s)
    }

    /// CNOT with qubit 0 as control, qubit 1 as target:
    /// |00⟩→|00⟩, |01⟩→|01⟩, |10⟩→|11⟩, |11⟩→|10⟩ (swaps a[2], a[3]).
    pub fn apply_cnot(mut self) -> Self {
        self.a.swap(2, 3);
        self
    }

    /// Maximally-entangled Bell state |Φ⁺⟩ = (|00⟩ + |11⟩)/√2,
    /// produced by H⊗I followed by CNOT on |00⟩.
    pub fn bell_phi_plus() -> Self {
        let s = crate::math::sqrt(0.5);
        let mut a = [Complex::new(0.0, 0.0); 4];
        a[0] = Complex::new(s, 0.0);
        a[3] = Complex::new(s, 0.0);
        Self { a }
    }

    /// Bell state |Ψ⁺⟩ = (|01⟩ + |10⟩)/√2.
    pub fn bell_psi_plus() -> Self {
        let s = crate::math::sqrt(0.5);
        let mut a = [Complex::new(0.0, 0.0); 4];
        a[1] = Complex::new(s, 0.0);
        a[2] = Complex::new(s, 0.0);
        Self { a }
    }

    /// Probability of measuring the computational-basis outcome `i` = |a_i|².
    pub fn prob(self, i: usize) -> f64 {
        let z = self.a[i];
        z.re * z.re + z.im * z.im
    }

    /// Born-rule measurement of the joint state: returns the basis index
    /// (0..4) chosen by the uniform sample `p ∈ [0, 1)`.
    pub fn measure(self, p: f64) -> usize {
        let mut acc = 0.0;
        for i in 0..4 {
            acc += self.prob(i);
            if p < acc {
                return i;
            }
        }
        3
    }
}

/// Quantum tri-state (qutrit): |ψ⟩ = a|True⟩ + b|False⟩ + c|Unknown⟩.
///
/// Generalizes [`TriState`] from three discrete outcomes to a continuous
/// superposition, so a system can hold *partial* information (e.g. 0.7|True⟩ +
/// 0.3|Unknown⟩) instead of a hard "Unknown". Born-rule measurement collapses
/// to one of the three classical outcomes; the logical operations (`not`, `and`,
/// `or`) are non-unitary and therefore collapse first (the superposition is the
/// storage/measurement representation, not a quantum circuit).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QTri {
    /// Amplitude of |True⟩.
    pub a: Complex,
    /// Amplitude of |False⟩.
    pub b: Complex,
    /// Amplitude of |Unknown⟩.
    pub c: Complex,
}

impl QTri {
    pub const fn new(a: Complex, b: Complex, c: Complex) -> Self {
        Self { a, b, c }
    }

    /// Pure |True⟩ basis state.
    pub fn t() -> Self {
        Self::new(
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
        )
    }

    /// Pure |False⟩ basis state.
    pub fn f() -> Self {
        Self::new(
            Complex::new(0.0, 0.0),
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 0.0),
        )
    }

    /// Pure |Unknown⟩ basis state.
    pub fn u() -> Self {
        Self::new(
            Complex::new(0.0, 0.0),
            Complex::new(0.0, 0.0),
            Complex::new(1.0, 0.0),
        )
    }

    /// Embed a classical [`TriState`] as a pure basis state.
    pub fn from_tri(t: TriState) -> Self {
        match t {
            TriState::True => Self::t(),
            TriState::False => Self::f(),
            TriState::Unknown => Self::u(),
        }
    }

    /// Born probability of measuring |True⟩ = |a|².
    pub fn prob_true(self) -> f64 {
        self.a.re * self.a.re + self.a.im * self.a.im
    }

    /// Born probability of measuring |False⟩ = |b|².
    pub fn prob_false(self) -> f64 {
        self.b.re * self.b.re + self.b.im * self.b.im
    }

    /// Born probability of measuring |Unknown⟩ = |c|².
    pub fn prob_unknown(self) -> f64 {
        self.c.re * self.c.re + self.c.im * self.c.im
    }

    /// Euclidean norm √(|a|² + |b|² + |c|²).
    pub fn norm(self) -> f64 {
        let s = self.prob_true() + self.prob_false() + self.prob_unknown();
        crate::math::sqrt(s)
    }

    /// Project onto the unit sphere.
    pub fn normalize(mut self) -> Self {
        let n = self.norm();
        if n != 0.0 {
            self.a = scale(self.a, 1.0 / n);
            self.b = scale(self.b, 1.0 / n);
            self.c = scale(self.c, 1.0 / n);
        }
        self
    }

    /// Born-rule measurement: collapse to a classical [`TriState`].
    ///
    /// `p` is a uniform sample in [0, 1) supplied by the caller (deterministic).
    pub fn measure(self, p: f64) -> TriState {
        let pt = self.prob_true();
        let pf = self.prob_false();
        if p < pt {
            TriState::True
        } else if p < pt + pf {
            TriState::False
        } else {
            TriState::Unknown
        }
    }

    /// Collapse to the dominant classical outcome (argmax of the probabilities).
    pub fn collapse(self) -> TriState {
        let pt = self.prob_true();
        let pf = self.prob_false();
        let pu = self.prob_unknown();
        if pt >= pf && pt >= pu {
            TriState::True
        } else if pf >= pu {
            TriState::False
        } else {
            TriState::Unknown
        }
    }

    /// Fail-closed resolution (mirrors [`TriState::resolve`]).
    pub fn resolve(self, default: bool) -> bool {
        self.collapse().resolve(default)
    }

    /// Logical NOT (Pauli-X on the True/False subspace): swaps a ↔ b.
    pub fn not(mut self) -> Self {
        core::mem::swap(&mut self.a, &mut self.b);
        self
    }

    /// Kleene AND: collapse both operands to classical, apply, re-embed.
    pub fn and(self, o: Self) -> Self {
        Self::from_tri(self.collapse().and(o.collapse()))
    }

    /// Kleene OR: collapse both operands to classical, apply, re-embed.
    pub fn or(self, o: Self) -> Self {
        Self::from_tri(self.collapse().or(o.collapse()))
    }
}

/// N-level quantum superposition |ψ⟩ = Σ c_i |i⟩.
///
/// The "quantum state everywhere" primitive for prediction: holds every
/// possible outcome simultaneously; an *oracle* (a boolean predicate marking
/// predicted-good outcomes) phase-flips the marked states, amplitude
/// amplification (Grover diffusion) boosts them, and measurement collapses to
/// the predicted consequence. Used at all levels to foresee consequences,
/// state changes, memory deltas, resource use, and time.
#[derive(Debug, Clone, PartialEq)]
pub struct QState {
    amps: Vec<Complex>,
}

impl QState {
    /// Uniform superposition |s⟩ = (1/√n) Σ |i⟩ over all n basis states.
    pub fn uniform(n: usize) -> Self {
        let a = Complex::new(1.0 / crate::math::sqrt(n as f64), 0.0);
        QState { amps: vec![a; n] }
    }

    /// Pure basis state |i⟩.
    pub fn basis(n: usize, i: usize) -> Self {
        let mut amps = vec![Complex::new(0.0, 0.0); n];
        amps[i] = Complex::new(1.0, 0.0);
        QState { amps }
    }

    pub fn len(&self) -> usize {
        self.amps.len()
    }

    /// Born probability of measuring outcome `i` = |c_i|².
    pub fn prob(&self, i: usize) -> f64 {
        let z = self.amps[i];
        z.re * z.re + z.im * z.im
    }

    /// Most-probable outcome (deterministic prediction — argmax of |c_i|²).
    pub fn argmax(&self) -> usize {
        let mut best = 0;
        let mut bp = self.prob(0);
        for i in 1..self.amps.len() {
            let p = self.prob(i);
            if p > bp {
                bp = p;
                best = i;
            }
        }
        best
    }

    /// Born-rule measurement: collapse to basis index `i` (deterministic via
    /// the uniform sample `p ∈ [0, 1)`).
    pub fn measure(&self, p: f64) -> usize {
        let mut acc = 0.0;
        for i in 0..self.amps.len() {
            acc += self.prob(i);
            if p < acc {
                return i;
            }
        }
        self.amps.len() - 1
    }

    /// One Grover iteration: oracle phase-flip on marked states, then diffusion
    /// (reflection about the mean amplitude). `oracle(i) == true` marks state i.
    pub fn grover_iterate<F: Fn(usize) -> bool>(&mut self, oracle: &F) {
        let n = self.amps.len();
        for i in 0..n {
            if oracle(i) {
                let z = self.amps[i];
                self.amps[i] = Complex::new(-z.re, -z.im);
            }
        }
        let (mut sr, mut si) = (0.0, 0.0);
        for i in 0..n {
            sr += self.amps[i].re;
            si += self.amps[i].im;
        }
        let mr = 2.0 * sr / n as f64;
        let mi = 2.0 * si / n as f64;
        for i in 0..n {
            let z = self.amps[i];
            self.amps[i] = Complex::new(mr - z.re, mi - z.im);
        }
    }

    /// Grover search: amplify the oracle-marked states over `iters` iterations
    /// from a uniform superposition. Optimal iters ≈ π/4 · √(n / marked).
    pub fn grover_search<F: Fn(usize) -> bool>(n: usize, oracle: &F, iters: usize) -> Self {
        let mut s = QState::uniform(n);
        for _ in 0..iters {
            s.grover_iterate(oracle);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn basis_states_are_normalized() {
        assert!(close(Qubit::zero().norm(), 1.0));
        assert!(close(Qubit::one().norm(), 1.0));
        assert!(close(Qubit::plus().norm(), 1.0));
        assert!(close(Qubit::plus_i().norm(), 1.0));
    }

    #[test]
    fn superposition_amplitudes() {
        let s = crate::math::sqrt(0.5);
        let p = Qubit::plus();
        assert!(close(p.alpha.re, s) && close(p.beta.re, s));
        let m = Qubit::minus();
        assert!(close(m.alpha.re, s) && close(m.beta.re, -s));
    }

    #[test]
    fn hadamard_maps_basis_to_superposition() {
        let hp = Qubit::zero().apply_h();
        assert!(close(hp.prob0(), 0.5) && close(hp.prob1(), 0.5));
        let hm = Qubit::one().apply_h();
        assert!(close(hm.prob0(), 0.5) && close(hm.prob1(), 0.5));
        // H² = I
        let id = Qubit::zero().apply_h().apply_h();
        assert!(close(id.alpha.re, 1.0) && close(id.beta.re, 0.0));
    }

    #[test]
    fn pauli_gates_act_on_basis() {
        assert_eq!(Qubit::zero().apply_x(), Qubit::one());
        assert_eq!(Qubit::one().apply_x(), Qubit::zero());
        // Z is the identity on |0⟩, a phase-flip on |1⟩
        assert!(close(Qubit::zero().apply_z().beta.re, 0.0));
        assert!(close(Qubit::one().apply_z().beta.re, -1.0));
    }

    #[test]
    fn measurement_is_deterministic() {
        assert_eq!(Qubit::zero().measure(0.5), 0);
        assert_eq!(Qubit::one().measure(0.5), 1);
        // equal superposition: p below 0.5 → 0, above → 1
        assert_eq!(Qubit::plus().measure(0.25), 0);
        assert_eq!(Qubit::plus().measure(0.75), 1);
    }

    #[test]
    fn bloch_sphere_angles() {
        let (t0, _) = Qubit::zero().bloch();
        assert!(close(t0, 0.0));
        let (t1, _) = Qubit::one().bloch();
        assert!(close(t1, crate::constants::PI));
        let (tp, pp) = Qubit::plus().bloch();
        assert!(close(tp, crate::constants::HALF_PI));
        assert!(close(pp, 0.0));
    }

    #[test]
    fn fidelity_of_states() {
        assert!(close(Qubit::zero().fidelity(Qubit::zero()), 1.0));
        assert!(close(Qubit::zero().fidelity(Qubit::one()), 0.0));
        assert!(close(Qubit::zero().fidelity(Qubit::plus()), 0.5));
        assert!(close(Qubit::plus().fidelity(Qubit::minus()), 0.0));
    }

    #[test]
    fn phase_gates_preserve_norm() {
        let t = Qubit::plus().apply_t();
        assert!(close(t.norm(), 1.0));
        let s = Qubit::plus().apply_s();
        assert!(close(s.norm(), 1.0));
    }

    #[test]
    fn rotations_are_unitary() {
        let r = Qubit::plus().apply_rx(0.3).apply_ry(0.7).apply_rz(0.2);
        assert!(close(r.norm(), 1.0));
    }

    #[test]
    fn cnot_produces_bell_entanglement() {
        // H⊗I on |00⟩ gives (|00⟩+|10⟩)/√2; CNOT gives (|00⟩+|11⟩)/√2.
        let bell = TwoQubit::bell_phi_plus();
        assert!(close(bell.prob(0), 0.5));
        assert!(close(bell.prob(1), 0.0));
        assert!(close(bell.prob(2), 0.0));
        assert!(close(bell.prob(3), 0.5));
        assert!(close(bell.norm(), 1.0));
        // CNOT flips the target when the control is |1⟩.
        let mut a = [Complex::new(0.0, 0.0); 4];
        a[2] = Complex::new(1.0, 0.0); // |10⟩
        let flipped = TwoQubit::new(a).apply_cnot();
        assert!(close(flipped.prob(3), 1.0)); // → |11⟩
    }

    #[test]
    fn two_qubit_measurement_partitions() {
        let bell = TwoQubit::bell_phi_plus();
        assert_eq!(bell.measure(0.25), 0);
        assert_eq!(bell.measure(0.75), 3);
    }

    #[test]
    fn qtri_round_trips_through_tristate() {
        assert_eq!(QTri::t().collapse(), TriState::True);
        assert_eq!(QTri::f().collapse(), TriState::False);
        assert_eq!(QTri::u().collapse(), TriState::Unknown);
        assert_eq!(QTri::from_tri(TriState::True).collapse(), TriState::True);
        assert_eq!(
            QTri::from_tri(TriState::Unknown).collapse(),
            TriState::Unknown
        );
    }

    #[test]
    fn qtri_probabilities_are_born_squares() {
        let q = QTri::new(
            Complex::new(1.0, 0.0),
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 0.0),
        )
        .normalize();
        assert!(close(q.prob_true(), 0.5));
        assert!(close(q.prob_false(), 0.5));
        assert!(close(q.prob_unknown(), 0.0));
        assert!(close(q.norm(), 1.0));
    }

    #[test]
    fn qtri_measurement_is_deterministic() {
        assert_eq!(QTri::t().measure(0.5), TriState::True);
        assert_eq!(QTri::f().measure(0.5), TriState::False);
        assert_eq!(QTri::u().measure(0.5), TriState::Unknown);
        let q = QTri::new(
            Complex::new(1.0, 0.0),
            Complex::new(1.0, 0.0),
            Complex::new(0.0, 0.0),
        )
        .normalize();
        assert_eq!(q.measure(0.25), TriState::True);
        assert_eq!(q.measure(0.75), TriState::False);
    }

    #[test]
    fn qtri_not_swaps_true_false() {
        assert_eq!(QTri::t().not().collapse(), TriState::False);
        assert_eq!(QTri::f().not().collapse(), TriState::True);
        assert_eq!(QTri::u().not().collapse(), TriState::Unknown);
    }

    #[test]
    fn qtri_logic_matches_kleene() {
        assert_eq!(QTri::t().and(QTri::u()).collapse(), TriState::Unknown);
        assert_eq!(QTri::t().and(QTri::f()).collapse(), TriState::False);
        assert_eq!(QTri::f().or(QTri::u()).collapse(), TriState::Unknown);
        assert_eq!(QTri::f().or(QTri::t()).collapse(), TriState::True);
        assert_eq!(QTri::u().not().collapse(), TriState::Unknown);
    }

    #[test]
    fn qstate_uniform_is_normalized() {
        let s = QState::uniform(4);
        assert_eq!(s.len(), 4);
        for i in 0..4 {
            assert!(close(s.prob(i), 0.25));
        }
    }

    #[test]
    fn grover_amplifies_marked_state_to_certainty() {
        // N=4, one marked state |2⟩: a single Grover iteration concentrates all
        // amplitude on it (optimal iteration count for N=4, M=1 is 1).
        let s = QState::grover_search(4, &|i| i == 2, 1);
        assert!(close(s.prob(2), 1.0), "prob(2)={}", s.prob(2));
        assert!(close(s.prob(0), 0.0));
        assert!(close(s.prob(1), 0.0));
        assert!(close(s.prob(3), 0.0));
        assert_eq!(s.argmax(), 2);
    }

    #[test]
    fn qstate_measurement_is_deterministic() {
        assert_eq!(QState::basis(4, 1).measure(0.5), 1);
        let u = QState::uniform(4);
        assert_eq!(u.measure(0.1), 0);
        assert_eq!(u.measure(0.6), 2);
        assert_eq!(u.measure(0.9), 3);
    }

    #[test]
    fn qstate_argmax_picks_dominant_outcome() {
        let s = QState::basis(3, 2);
        assert_eq!(s.argmax(), 2);
    }
}
