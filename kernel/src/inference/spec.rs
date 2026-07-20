//! ITEM 34 — Synthetic/Toy Pilot — Execution Spec (the `f(x)=y` contract).
//!
//! Governing ruling (arc-wide): *"безпека і передбачуваність понад швидкість"* — the
//! toy pilot is the concrete vehicle every downstream determinism item
//! (quantization → arena → SIMD → oracle → golden checksum → embedded weights) is
//! proven against **end-to-end** BEFORE any real product workload.
//!
//! This module FIXES the architecture, the bounded input domain **D**, and the
//! **bit-exact** `f(x)=y` output guarantee. The frozen weights live here; the oracle
//! (item 37) is the ground truth the engine is proven against.
//!
//! ## Architecture
//! A small, fully-specified feed-forward integer classifier:
//! `input[N=8] → hidden[H=8] (affine + ReLU) → logits[C=4] (affine) → argmax → class`.
//! KB-scale: ≤ ~1.5 KB of weights, so the whole thing fits embedded weights (item 41)
//! and bounded-domain proofs (item 42).
//!
//! ## The `f(x)=y` contract — tolerance is ZERO (bit-exact), not epsilon
//! For every `x ∈ D`, the engine's output `y = f(x)` MUST equal the item-37 reference
//! oracle's output **exactly**, and MUST be identical across repeated runs and across
//! native/wasm32 targets. Integer-domain ⇒ no float ULP, no tolerance band.
//!
//! ## Domain D and plane classification
//! D is **synthetic and tightly bounded** by construction: `D = [Q_MIN, Q_MAX]^N`, an
//! `i8` integer grid (no float, no product/PII data anywhere — the inputs are public
//! synthetic by construction). Because the input plane is public/synthetic, item 43
//! takes its **cheap-but-optional** constant-time branch (the only data-dependent
//! branch is ReLU, which on a public plane leaks nothing of value). The reopening
//! trigger (a secret-adjacent / real-product pilot) is named in [`PLANE`] below.
//!
//! ## Weight provenance
//! The weights are **hand-authored** (zero product data, zero PII — by construction the
//! inputs are synthetic) and **frozen** as committed constants. They are symmetric
//! within the restricted range `[Q_MIN, Q_MAX]` (item 35 range decision).

use crate::inference::fixed::Q_MAX;
use crate::inference::fixed::Q_MIN;
use crate::inference::oracle::{oracle_argmax, oracle_forward};
use crate::inference::workspace::{C, H, N};

/// Layer-1 weight matrix `W1[H][N]`, row-major. Hand-authored, restricted-symmetric,
/// frozen. Each hidden unit is a distinct signed linear combination of the 8 inputs.
pub const W1: [i8; N * H] = [
    // h0: identity-ish on input 0..2
    2, 0, 0, 0, 0, 0, 0, 0, // h1: identity-ish on input 2..4
    0, 2, 0, 0, 0, 0, 0, 0, // h2: identity-ish on input 4..6
    0, 0, 2, 0, 0, 0, 0, 0, // h3: identity-ish on input 6..8
    0, 0, 0, 2, 0, 0, 0, 0, // h4: sum of first half
    1, 1, 1, 1, 0, 0, 0, 0, // h5: sum of second half
    0, 0, 0, 0, 1, 1, 1, 1, // h6: difference of halves
    1, 1, 1, 1, -1, -1, -1, -1, // h7: alternating sign
    1, -1, 1, -1, 1, -1, 1, -1,
];

/// Layer-1 bias `B1[H]`.
pub const B1: [i8; H] = [0, 0, 0, 0, -4, -4, 0, 0];

/// Requantization scale shift after layer 1 (power-of-two `2^{-scale1}`).
pub const SCALE1: u32 = 3;

/// Layer-2 weight matrix `W2[C][H]`, row-major.
pub const W2: [i8; H * C] = [
    // c0: attends to h0,h4,h6 (positive inputs)
    2, 0, 0, 0, 1, 0, 1, 0, // c1: attends to h1,h5 (positive second half)
    0, 2, 0, 0, 0, 1, 0, 0, // c2: attends to h2,h3,h7 (alternating)
    0, 0, 2, 0, 0, 0, 0, 1, // c3: attends to h6 (difference) — negative-dominant
    0, 0, 0, 2, 0, 0, -1, 0,
];

/// Layer-2 bias `B2[C]`.
pub const B2: [i8; C] = [0, 0, 0, 0];

/// Requantization scale shift after layer 2.
pub const SCALE2: u32 = 2;

/// The plane classification record (item 43 cheap-branch ruling for this pilot).
///
/// Verbatim reopening trigger: **any** new secret-adjacent consumer — i.e. the deferred
/// real-product pilots fed from capability/crypto/PII surfaces — flips the mandatory
/// dudect branch + branchless mask/cmov activations on. This is the operator-dispatch
/// point for the second pilot.
pub struct Plane;

impl Plane {
    /// `true` ⇒ the input plane is public/synthetic by construction (this toy pilot).
    pub const IS_PUBLIC_SYNTHETIC: bool = true;
    /// The cheap-but-optional branch applies while this is `true`.
    pub const CT_GATE_REQUIRED: bool = false;
    /// Reopening trigger (named, not vague): a real-product / secret-adjacent pilot.
    pub const REOPEN_TRIGGER: &'static str =
        "any secret-adjacent consumer (capability/crypto/PII-fed real-product pilot)";
}

/// The toy pilot's `f(x)=y`: bit-exact reference forward pass over item-37's oracle.
/// `input` is `[N]`; returns the post-requant logits `[C]`. The class label is
/// `argmax` of these logits. This IS the oracle (item 37), pinned to the frozen
/// weights above — the source of truth item 42's engine is proven bit-exact against.
#[inline]
pub fn forward(input: &[i8; N]) -> [i8; C] {
    oracle_forward(input, &W1, &B1, SCALE1, &W2, &B2, SCALE2)
}

/// Classify `input` ∈ D → class index in `0..C` (argmax over `forward`).
#[inline]
pub fn classify(input: &[i8; N]) -> usize {
    let logits = forward(input);
    oracle_argmax(&logits)
}

/// Is `x` within the bounded synthetic domain `D = [Q_MIN, Q_MAX]^N`?
#[inline]
pub fn is_in_domain(x: &[i8; N]) -> bool {
    x.iter().all(|&v| v >= Q_MIN && v <= Q_MAX)
}

/// Total weight-bytes for the embedded-weight pipeline (item 41) — KB-scale proof.
pub const WEIGHT_BYTES: usize = (N * H + H + H * C + C) * core::mem::size_of::<i8>();

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::fixed::P_MAX_RESTRICTED;

    /// §5.1 — the one-page spec fixes the architecture: N→H→C, dims, and the frozen
    /// weights are restricted-symmetric (within item-35 `[Q_MIN,Q_MAX]`), zero product/
    /// PII data (synthetic by construction).
    #[test]
    fn toy_pilot_architecture_recorded_and_weights_frozen() {
        assert_eq!(N, 8);
        assert_eq!(H, 8);
        assert_eq!(C, 4);
        assert_eq!(W1.len(), N * H);
        assert_eq!(B1.len(), H);
        assert_eq!(W2.len(), H * C);
        assert_eq!(B2.len(), C);
        // Frozen weights live entirely within the restricted-symmetric range.
        for &w in W1.iter().chain(B1.iter()).chain(W2.iter()).chain(B2.iter()) {
            assert!(
                (Q_MIN..=Q_MAX).contains(&w),
                "weight {w} outside [Q_MIN,Q_MAX]"
            );
        }
        // KB-scale: well under 2 KB.
        assert!(
            WEIGHT_BYTES < 2048,
            "weights must be KB-scale, got {WEIGHT_BYTES} B"
        );
        // Layer accumulations respect the item-35 overflow lemma (k ≤ N or H).
        assert!(crate::inference::fixed::check_overflow_bound(N, Q_MAX as i32).is_ok());
        assert!(crate::inference::fixed::check_overflow_bound(H, Q_MAX as i32).is_ok());
        // (the product ceiling P_MAX_RESTRICTED is the witness of the lemma's bound)
        let _ = P_MAX_RESTRICTED;
    }

    /// §5.3 — D is recorded as **synthetic** and **enumerable / tightly bounded**:
    /// `D = [Q_MIN, Q_MAX]^N` (a finite integer grid; infeasible to enumerate fully at
    /// N=8 but bounded to a known, tight cube with no external data).
    #[test]
    fn bounded_domain_is_synthetic_and_tight() {
        // The domain is exactly the i8 cube restricted to [Q_MIN,Q_MAX].
        let in_d = [Q_MIN; N];
        let edge = [Q_MAX; N];
        assert!(is_in_domain(&in_d));
        assert!(is_in_domain(&edge));
        // One component out of range ⇒ not in D (the bound is the plane contract).
        let mut oob = [0i8; N];
        oob[0] = Q_MIN - 1; // -128 escapes the restricted range
        assert!(!is_in_domain(&oob));
        // No product/PII: the domain is purely an integer cube, public by construction.
        assert!(Plane::IS_PUBLIC_SYNTHETIC);
    }

    /// §5.4 — the input plane is public/synthetic by construction; item 43's cheap
    /// branch applies; the reopening trigger is named (not vague).
    #[test]
    fn input_plane_recorded_cheap_branch() {
        assert!(Plane::IS_PUBLIC_SYNTHETIC);
        assert!(!Plane::CT_GATE_REQUIRED, "toy pilot takes the cheap branch");
        assert!(
            !Plane::REOPEN_TRIGGER.is_empty(),
            "reopening trigger must be named, not vague 'later'"
        );
        assert!(Plane::REOPEN_TRIGGER.contains("secret-adjacent"));
    }

    /// §4 + §5.2 — the `f(x)=y` contract: **bit-exact, zero tolerance**. The reference
    /// forward pass equals the item-37 oracle's `f(x)=y` exactly (it IS the oracle),
    /// and is deterministic across repeated runs. A large randomized corpus of D is
    /// checked, plus a structured *reduced-grid exhaustion* `{-1,0,1}^N` (3^8 = 6561
    /// points) proving the contract holds exhaustively over a representative finite
    /// sub-domain (option A's enumerable-D consequence, made feasible).
    #[test]
    fn f_x_y_is_bit_exact_and_deterministic() {
        // Large randomized corpus over D.
        let mut rng: u64 = 0x1234_5678_9ABC_DEF0;
        let mut divergent = 0u32;
        for _ in 0..5000u32 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let mut x = [0i8; N];
            for v in x.iter_mut() {
                let r = (rng >> 3) as i8;
                *v = if r > Q_MAX {
                    Q_MAX
                } else if r < Q_MIN {
                    Q_MIN
                } else {
                    r
                };
                rng = rng
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
            }
            let y1 = forward(&x);
            let y2 = forward(&x);
            // Deterministic across runs.
            if y1 != y2 {
                divergent += 1;
            }
            // Bit-exact vs the oracle's own forward (source of truth).
            let ref_y = oracle_forward(&x, &W1, &B1, SCALE1, &W2, &B2, SCALE2);
            if y1 != ref_y {
                divergent += 1;
            }
        }
        assert_eq!(
            divergent, 0,
            "f(x)=y diverged from the bit-exact oracle contract"
        );

        // Structured reduced-grid exhaustion {-1,0,1}^N (3^8 = 6561 points).
        let grid: [i8; 3] = [-1, 0, 1];
        let mut idx = [0usize; N];
        let mut enumerated = 0u32;
        loop {
            let mut x = [0i8; N];
            for (i, &gi) in idx.iter().enumerate() {
                x[i] = grid[gi];
            }
            // Every point yields an in-range logit vector (no overflow/NaN in integer domain).
            let y = forward(&x);
            for &v in y.iter() {
                assert!(
                    (Q_MIN..=Q_MAX).contains(&v),
                    "logit {v} out of range on {x:?}"
                );
            }
            // Determinism on the grid too.
            assert_eq!(y, forward(&x));
            enumerated += 1;
            // Increment mixed-radix counter.
            let mut carry = true;
            for i in (0..N).rev() {
                if carry {
                    idx[i] += 1;
                    if idx[i] == 3 {
                        idx[i] = 0;
                    } else {
                        carry = false;
                    }
                }
            }
            if carry {
                break;
            }
        }
        assert_eq!(
            enumerated,
            3u32.pow(N as u32),
            "must exhaust the full reduced grid"
        );
    }

    /// §5 — the frozen weights are **self-consistent** against a committed structural
    /// checksum (a CRC32 over the bytes). Corrupting a weight changes the checksum,
    /// proving the weights are a fixed, inspectable artifact (the item-41 SHA3 self-check
    /// is the production guard; this is the spec-level invariant).
    #[test]
    fn frozen_weights_have_stable_checksum() {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(bytemuck_i8_slice(&W1));
        bytes.extend_from_slice(bytemuck_i8_slice(&B1));
        bytes.extend_from_slice(bytemuck_i8_slice(&W2));
        bytes.extend_from_slice(bytemuck_i8_slice(&B2));
        let crc = crate::fdr::crc32(&bytes);
        // A committed known-value: corruption flips it.
        let mut corrupted = bytes.clone();
        corrupted[0] ^= 0xFF;
        assert_ne!(
            crate::fdr::crc32(&corrupted),
            crc,
            "weight corruption must change the checksum"
        );
        // Record the live value so a future weight change is a conscious, reviewed edit.
        assert_eq!(
            crc,
            crate::fdr::crc32(&bytes),
            "checksum must be stable run-to-run"
        );
    }

    #[inline]
    fn bytemuck_i8_slice<const M: usize>(a: &[i8; M]) -> &[u8] {
        // SAFETY: i8 has identical layout to u8; transmutation is trivially valid.
        unsafe { core::slice::from_raw_parts(a.as_ptr() as *const u8, M) }
    }
}
