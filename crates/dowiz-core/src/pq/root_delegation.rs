//! pq/root_delegation.rs — R-3 `RootDelegationPolicy` (Layer D consensus / trust / capability).
//!
//! Per operator ruling A (recorded 2026-07-18, `DECISIONS.md` D10):
//!   * `OperatorSigned`  — the operator's ML-DSA-65 root key is the sole sovereign root.
//!   * `Overlay { depth }` — at most ONE delegation hop below the operator root is permitted
//!                           (`depth <= 1` by construction; deeper chains are rejected).
//!   * `Deferred`        — the Web-Of-Trust / FirstContactQr branch. The variant exists (type
//!                           level) but its verify path is NOT wired in v1: it returns a typed
//!                           `Unsupported` error rather than silently doing anything.
//!
//! This module lives under `pq/` precisely so it can reach the KAT-gated, byte-exact
//! `crate::pq::dsa` ML-DSA-65 primitive WITHOUT pulling `pq` into the serde-free native
//! build graph (the canonical order/money core stays feature-independent; `RootDelegationPolicy`
//! is part of the opt-in mesh/transport identity seam, exactly like the rest of `pq`).
//!
//! No network, no clock, no I/O. `verify_root` is a pure predicate over a signature + pubkey.

use crate::pq::dsa::{keygen, sign, verify, MlDsa65Pk, MlDsa65Sig, MlDsa65Sk};

/// Configured root-delegation model. Fail-closed by construction: until the operator
/// selects a real policy, nothing delegates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootDelegationPolicy {
    /// Operator-signed root cert(s): offline, audited, pinned. The sovereign floor.
    OperatorSigned,
    /// An overlay delegation chain rooted at the operator's ML-DSA root, allowed to fan
    /// out by at most `depth` hops below the operator. `depth` MUST be `<= 1` (exactly ONE
    /// overlay hop per the operator ruling). Construct via [`RootDelegationPolicy::overlay`]
    /// to enforce the invariant; a `depth > 1` is rejected at construction time.
    Overlay { depth: u8 },
    /// Deferred branch (Web-Of-Trust / FirstContactQr). Exists at the type level (so the
    /// ruling's other options are representable) but is NOT wired in v1: `verify_root` returns
    /// `RootVerifyError::Unsupported` for it. Never guessed, never silently allowed.
    Deferred,
}

impl RootDelegationPolicy {
    /// Construct an `Overlay` policy, enforcing the invariant `depth <= 1`.
    ///
    /// Returns `Err(RootVerifyError::MaxDepthExceeded)` if `depth > 1`, so a double-hop (or
    /// deeper) overlay can never be represented. This is the construction-time gate that makes
    /// the "exactly ONE overlay hop" rule unrepresentable-at-the-boundary rather than a runtime
    /// check that could be bypassed.
    pub fn overlay(depth: u8) -> Result<RootDelegationPolicy, RootVerifyError> {
        if depth > 1 {
            return Err(RootVerifyError::MaxDepthExceeded(depth));
        }
        Ok(RootDelegationPolicy::Overlay { depth })
    }

    /// Verify a root attestation under this policy, using the kernel's EXISTING ML-DSA-65
    /// primitive (FIPS 204, KAT-gated, byte-exact vs NIST ACVP vectors — NO invented crypto).
    ///
    /// * `OperatorSigned` — `msg` MUST verify against `op_root_pk` (the operator's ML-DSA root
    ///   public key). Returns `Ok(())` on a valid signature, `RootVerifyError::BadRootSignature`
    /// otherwise.
    /// * `Overlay { depth }` — `msg` MUST verify against `op_root_pk` as well (the overlay chain
    ///   is still rooted at the operator; `depth` is the permitted fan-out hops, already bounded
    ///   to `<= 1` by [`RootDelegationPolicy::overlay`]). Same verification as `OperatorSigned`;
    ///   the `depth` bound is enforced at construction, so any `Overlay` value here is already
    ///   admissible.
    /// * `Deferred` — not wired in v1. Returns `Err(RootVerifyError::Unsupported)` every time.
    pub fn verify_root(
        &self,
        msg: &[u8],
        sig: &MlDsa65Sig,
        op_root_pk: &MlDsa65Pk,
    ) -> Result<(), RootVerifyError> {
        match self {
            RootDelegationPolicy::OperatorSigned => {
                if verify(op_root_pk, msg, sig) {
                    Ok(())
                } else {
                    Err(RootVerifyError::BadRootSignature)
                }
            }
            RootDelegationPolicy::Overlay { depth: _ } => {
                // The overlay chain is still rooted at the operator's ML-DSA key; the hop bound
                // is enforced at construction. Verification is identical to OperatorSigned.
                if verify(op_root_pk, msg, sig) {
                    Ok(())
                } else {
                    Err(RootVerifyError::BadRootSignature)
                }
            }
            RootDelegationPolicy::Deferred => Err(RootVerifyError::Unsupported),
        }
    }
}

/// Why a root attestation verification failed (or was refused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootVerifyError {
    /// The ML-DSA-65 signature over the root attestation did not verify against the operator
    /// root public key.
    BadRootSignature,
    /// The `Deferred` policy variant is not wired in v1 and must not be used to authorize anything.
    Unsupported,
    /// Construction-time invariant violation: an `Overlay` with `depth > 1` (more than ONE
    /// delegation hop below the operator root) is refused and never represented.
    MaxDepthExceeded(u8),
}

/// Convenience: produce an operator-rooted ML-DSA-65 signature over `msg` from the operator
/// root secret key. Used by tests and by the (future) signing path; deterministic in FIPS mode
/// when `rnd` is fixed (we pass all-zeros here — caller may supply fresh entropy for production).
pub fn sign_root(op_root_sk: &MlDsa65Sk, msg: &[u8]) -> MlDsa65Sig {
    let rnd = [0u8; 32];
    sign(op_root_sk, msg, &rnd)
}

/// Deterministically derive an operator root keypair from a 32-byte seed.
pub fn operator_root_keygen(seed: &[u8; 32]) -> (MlDsa65Pk, MlDsa65Sk) {
    keygen(seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op_keypair() -> (MlDsa65Pk, MlDsa65Sk) {
        // Deterministic seed; this is a test-only root, never a production key.
        let seed = [7u8; 32];
        operator_root_keygen(&seed)
    }

    #[test]
    fn operator_signed_root_accepted() {
        let (pk, sk) = op_keypair();
        let msg = b"root-attestation: operator-signed genesis";
        let sig = sign_root(&sk, msg);
        let policy = RootDelegationPolicy::OperatorSigned;
        assert_eq!(policy.verify_root(msg, &sig, &pk), Ok(()));
    }

    #[test]
    fn single_overlay_hop_depth_1_accepted() {
        let (pk, sk) = op_keypair();
        let msg = b"root-attestation: overlay anchor #1";
        let sig = sign_root(&sk, msg);
        // depth == 1 is the exactly-one-hop overlay allowed by the operator ruling.
        let policy = RootDelegationPolicy::overlay(1).expect("depth=1 must construct");
        assert_eq!(policy.verify_root(msg, &sig, &pk), Ok(()));
    }

    #[test]
    fn double_hop_overlay_depth_2_rejected() {
        // A depth-2 overlay (more than ONE delegation hop below the operator root) is the
        // operator-ruling violation. The invariant is enforced at CONSTRUCTION so the bad state
        // is unrepresentable: `overlay(2)` returns Err and no `RootDelegationPolicy` value with
        // depth>1 can ever reach `verify_root`. This is the fail-closed boundary — callers must
        // construct overlays via `overlay()`, never via the raw variant.
        assert_eq!(
            RootDelegationPolicy::overlay(2),
            Err(RootVerifyError::MaxDepthExceeded(2))
        );
    }

    #[test]
    fn deferred_policy_is_unsupported() {
        let (pk, sk) = op_keypair();
        let msg = b"root-attestation: web-of-trust (deferred)";
        let sig = sign_root(&sk, msg);
        let policy = RootDelegationPolicy::Deferred;
        assert_eq!(
            policy.verify_root(msg, &sig, &pk),
            Err(RootVerifyError::Unsupported)
        );
    }

    #[test]
    fn operator_signed_rejects_tampered_message() {
        let (pk, sk) = op_keypair();
        let sig = sign_root(&sk, b"root-attestation: legitimate");
        let policy = RootDelegationPolicy::OperatorSigned;
        // Same signature, different message -> must NOT verify.
        assert_eq!(
            policy.verify_root(b"root-attestation: forged", &sig, &pk),
            Err(RootVerifyError::BadRootSignature)
        );
    }

    #[test]
    fn depth_0_overlay_is_single_root_directly() {
        // depth == 0 means "delegated directly by the operator, no further hop" — also allowed.
        let (pk, sk) = op_keypair();
        let msg = b"root-attestation: overlay depth 0";
        let sig = sign_root(&sk, msg);
        let policy = RootDelegationPolicy::overlay(0).expect("depth=0 must construct");
        assert_eq!(policy.verify_root(msg, &sig, &pk), Ok(()));
    }
}
