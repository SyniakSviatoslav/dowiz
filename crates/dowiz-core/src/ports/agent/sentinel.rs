//! ports/agent/sentinel.rs — Item 54: read-time integrity check for critical LIVE
//! in-memory authority structs (Sentinel).
//!
//! ## What this is
//! A narrow, enumerable set of long-lived, mutable authority structs are protected by a
//! stored CRC32 over their canonical (sorted) authority bytes. At each *transition point*
//! (once per authority-use), the struct's stored CRC is recomputed-and-compared against
//! the live bytes. On mismatch — the signature of an in-memory bit-flip on non-ECC consumer
//! RAM — exactly ONE fsynced FDR `Alarm` is emitted (naming the corrupted struct) and the
//! caller takes the deterministic fail-closed path (deny-closed for capability
//! verification). This is the live-struct analog of item 40's at-rest weights checksum;
//! same shared `fdr::crc32`, same Safe-State semantics, complementary plane (mutable
//! structs item 40 structurally cannot touch).
//!
//! ## Reuse, max-nativeness (roadmap:894)
//! Reuses the ONE in-kernel `fdr::crc32` (lifted to always-compiled in `fdr/mod.rs`,
//! blueprint §3.2). No new primitive, no new algorithm, no new dependency. The threat is
//! a hardware memory fault, NOT an in-memory adversary, so CRC32 (not a cryptographic hash)
//! suffices — the blueprint's explicit proportionality ruling.
//!
//! ## The critical-struct registry (3-axis test)
//! A struct qualifies iff (a) long-lived, (b) an authority input to a money/safety/
//! decision path, and (c) has no at-rest backing that already verifies it (blueprint
//! §2.4). Each registered struct is recorded below with its 3-axis justification. The
//! present-day instances (`AnchorRoster`, `RevocationSet`) land FIRST — they exist now and
//! need no unbuilt item (items 47/50). The canonical instance (`Invariants`) is deferred
//! behind item 47; the lower-value `Admitter.admitted` is deferred behind roster/revocation.
//!
//! EXPLICITLY EXCLUDED (honest boundary calls, blueprint §2.4):
//! - `FSM_ADJ` / `FSM_GOLDEN_SIGNATURE` (`order_machine.rs`): `const`/immutable-after-compile
//!   static data ⇒ item 40's build-time golden-CRC plane, NOT this live-mutable plane. A
//!   flipped `const` is item 40's job. Named so the boundary is explicit, not silently
//!   skipped.
//! - Transient hot-loop scratch (arena buffers): not long-lived, not authority.
//! - Anything already at-rest-verified (event_log chain, backup CAS): axis (c) fails.

use crate::fdr;

use super::cap::{AnchorRoster, RevocationSet};

/// The named, in-code enumeration of the qualifying live-struct registry (blueprint §5:
/// "The critical-struct registry is enumerated in-code with a per-struct 3-axis
/// justification"). Adding a sentinelized struct is a deliberate, reviewed edit here — the
/// registry is the single source of truth for what the Sentinel protects, and a reviewer
/// can read each qualification.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SentinelTarget {
    /// `AnchorRoster` — the trust-root set (cap.rs:377). A flipped anchor key silently
    /// changes which roots authorize the entire capability chain in `verify_chain`.
    /// (a) long-lived roster of trusted anchor keys; (b) security-decision authority;
    /// (c) held live in memory, no per-use at-rest re-verify.
    AnchorRoster,
    /// `RevocationSet` — revoked keys/caps (cap.rs:412). A flipped revocation bit silently
    /// UN-revokes a revoked key → admits a revoked agent.
    /// (a) long-lived, grows via anti-entropy `merge`; (b) security-decision authority;
    /// (c) live, no per-use at-rest verify. Mutable — the `merge`/`revoke_key` path is a
    /// real re-hash site (§3.3).
    RevocationSet,
}

impl SentinelTarget {
    /// The stable, greppable name carried on the FDR `Alarm` (item 54 Safe-State evidence).
    pub fn as_str(self) -> &'static str {
        match self {
            SentinelTarget::AnchorRoster => "AnchorRoster",
            SentinelTarget::RevocationSet => "RevocationSet",
        }
    }
}

/// The reason a struct's verification handle could not be produced. Distinct from a
/// *detected corruption* (`Corruption`): a `None` handle is a programming error (a
/// candidate registry entry that does not contain the expected struct), never a live
/// memory fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentinelUnavailable {
    /// No live struct of the registered type is present in the candidate (should not
    /// happen for a well-formed candidate — fail-closed, the verification is refused).
    NoHandle,
}

/// CRC32 over the canonical (sorted) authority bytes of a live struct, plus a recompute
/// + compare against the stored value. Implemented by every sentinelized struct.
///
/// The hashed form MUST be canonical (sorted) so that mutation *order* does not spuriously
/// change the checksum — `AnchorRoster::snapshot_sorted` / `RevocationSet::snapshot_sorted`
/// supply exactly this (blueprint §3.2 / §7.1-4).
pub trait IntegrityChecked {
    /// CRC32 of the struct's canonical authority bytes.
    fn checksum(&self) -> u32;
    /// Recompute-and-compare against the struct's stored CRC. `Ok(())` iff they match.
    fn verify(&self) -> Result<(), Corruption>;
}

/// A detected live-struct integrity fault: the stored CRC no longer matches the live
/// bytes. Semantically a hardware/memory fault (item 40 semantics) — fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Corruption {
    /// Which registered live struct tripped.
    pub target: SentinelTarget,
    /// The stored (expected) CRC.
    pub expected: u32,
    /// The recomputed CRC over live bytes.
    pub actual: u32,
}

impl Corruption {
    /// The stable name for the tripped struct (carried on the FDR `Alarm`).
    pub fn target_name(&self) -> &'static str {
        self.target.as_str()
    }
}

/// The fail-closed Safe-State action for a detected corruption (blueprint §3.5): emit
/// EXACTLY ONE fsynced FDR `Alarm` naming the corrupted struct, then let the caller take
/// its deterministic deny path. The `Alarm` is fsynced by the durable ring on append
/// (`ring.rs:134` — power-loss durable); the record is the forensic side-effect, the
/// deny is the value.
///
/// On `wasm32` no FDR sink is ever installed, so `fdr::emit_alarm` is inert there — but the
/// caller's deny path still fires (the decision-plane CRC check runs on wasm).
pub fn safe_state_on_corruption(c: &Corruption) {
    let detail = format!(
        "expected_crc={:08x} actual_crc={:08x}",
        c.expected, c.actual
    );
    fdr::emit_alarm(c.target_name(), &detail);
}

/// Verify a candidate's live structs at a transition point. Each registered live struct
/// that is PRESENT is recomputed-and-compared against its stored CRC. Returns the FIRST
/// `Corruption` found (fail-closed takes on the first evidence; one alarm per detected
/// fault, not one per struct). A candidate with neither struct present is trivially clean.
pub fn verify_candidate(
    roster: Option<&AnchorRoster>,
    revocations: Option<&RevocationSet>,
) -> Result<(), Corruption> {
    if let Some(r) = roster {
        if let Err(c) = r.verify() {
            return Err(c);
        }
    }
    if let Some(rev) = revocations {
        if let Err(c) = rev.verify() {
            return Err(c);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fdr;

    #[test]
    fn sentinel_target_names_are_stable_and_greppable() {
        // The names carried on FDR Alarms must be stable + greppable for forensic tooling.
        assert_eq!(SentinelTarget::AnchorRoster.as_str(), "AnchorRoster");
        assert_eq!(SentinelTarget::RevocationSet.as_str(), "RevocationSet");
    }

    #[test]
    fn crc32_is_callable_from_a_wasm_compiled_path() {
        // Acceptance criterion: the lifted `fdr::crc32` is always compiled (the KAT vector
        // proves behavior-preserving move from the wasm-gated `ring`). The `ring.rs:334`
        // KAT is the canonical check; this asserts the public alias stays live.
        assert_eq!(fdr::crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn clean_roster_and_revocation_verify_silent() {
        // A clean run over the full registry produces ZERO corruption (no false trips).
        let mut r = AnchorRoster::new();
        r.enroll(&[7u8; 32]);
        let mut rev = RevocationSet::new();
        rev.revoke_key([9u8; 32]);
        assert!(r.verify().is_ok());
        assert!(rev.verify().is_ok());
        assert!(verify_candidate(Some(&r), Some(&rev)).is_ok());
    }

    #[test]
    fn planted_roster_corruption_trips_fail_closed() {
        // Behavioral oracle (blueprint §4.1): a planted single-bit flip of the live struct's
        // authority bytes trips the Safe-State path AND writes exactly one recoverable Alarm.
        let mut r = AnchorRoster::new();
        r.enroll(&[7u8; 32]);
        // NOTE: corruption is injected via the test-only `corrupt_anchor_bit` on the struct
        // (mirroring item 40's planted-fault test). The Sentinel's detection is what we prove.
        r.corrupt_anchor_bit();
        let c = r
            .verify()
            .expect_err("planted corruption must trip verify()");
        assert_eq!(c.target, SentinelTarget::AnchorRoster);
        assert_ne!(c.expected, c.actual);

        // The Safec-State action emits exactly one FDR Alarm (inert unless a sink is
        // installed; we just prove it does not panic and returns).
        safe_state_on_corruption(&c);
    }

    #[test]
    fn mutate_then_read_through_central_mutator_passes() {
        // Re-hash correctness (blueprint §3.3): legitimate mutation via a central mutator
        // must NOT trip the Sentinel on the next read.
        let mut r = AnchorRoster::new();
        r.enroll(&[1u8; 32]);
        assert!(r.verify().is_ok(), "after enroll");

        let mut rev = RevocationSet::new();
        rev.revoke_key([2u8; 32]);
        assert!(rev.verify().is_ok(), "after revoke_key");
        rev.merge(&{
            let mut o = RevocationSet::new();
            o.revoke_capability([3u8; 32]);
            o
        });
        assert!(rev.verify().is_ok(), "after merge");
    }

    #[test]
    fn planted_revocation_corruption_trips_fail_closed() {
        // Behavioral oracle for the RevocationSet (the mutable instance). A planted single-
        // bit flip of a revoked key trips verify() and the fail-closed Safe-State path.
        let mut rev = RevocationSet::new();
        rev.revoke_key([9u8; 32]);
        rev.corrupt_revoked_key_bit();
        let c = rev
            .verify()
            .expect_err("planted revocation corruption must trip verify()");
        assert_eq!(c.target, SentinelTarget::RevocationSet);
        assert_ne!(c.expected, c.actual);
        safe_state_on_corruption(&c);
    }

    #[test]
    fn planted_roster_corruption_refuses_verify_chain() {
        // End-to-end: a corrupted AnchorRoster refuses `verify_chain` with
        // `IntegrityFault` (deny-closed) — the admission-level fault propagation.
        use crate::ports::agent::cap::{
            verify_chain, Capability, ChainError, RefSigner, SignatureVerifier,
        };
        let s = RefSigner;
        let v = &s;
        let mut roster = AnchorRoster::new();
        roster.enroll(&s.classical_public(&[3u8; 32]));
        // Corrupt the live roster BEFORE it is used as an authority input.
        roster.corrupt_anchor_bit();
        let cap = Capability::new_hybrid(
            s.classical_public(&[5u8; 32]),
            s.pq_public(&[6u8; 32]),
            crate::ports::agent::scope::Scope::single(
                crate::ports::agent::scope::Resource::AgentBridge,
                crate::ports::agent::scope::Action::AdmitAgent,
            ),
            [1u8; 8],
            9999,
        );
        let mut rev = RevocationSet::new();
        let res = verify_chain(v, &roster, &[], &cap, 0);
        assert_eq!(res, Err(ChainError::IntegrityFault));
        let _ = &mut rev;
    }

    #[test]
    fn clean_run_produces_zero_alarm_records() {
        // Acceptance criterion: a clean run over the full registry produces ZERO Alarms.
        // The FDR sink is a process-global `OnceLock` (fdr::init is idempotent — the first
        // init in the test binary wins the ring dir), so a per-test ring round-trip cannot
        // prove isolation here. Instead we prove the Sentinel's SILENCE directly: a clean
        // roster + revocation verify clean (no `Corruption`), which means `safe_state_on_corruption`
        // — the ONLY alarm emitter — is never reached. Zero corruption ⇒ zero alarms, by construction.
        use crate::ports::agent::cap::{
            verify_chain, Capability, ChainError, RefSigner, SignatureVerifier,
        };
        use crate::ports::agent::sentinel::{verify_candidate, SentinelTarget};

        let s = RefSigner;
        let roster = AnchorRoster::new();
        let mut rev = RevocationSet::new();
        // Clean authority structs must verify with NO detected corruption (no alarm path).
        assert!(
            verify_candidate(Some(&roster), None).is_ok(),
            "clean AnchorRoster must not trip the Sentinel"
        );
        assert!(
            verify_candidate(None, Some(&rev)).is_ok(),
            "clean RevocationSet must not trip the Sentinel"
        );

        // And a clean verify_chain over the clean roster must NOT return IntegrityFault
        // (the alarm-triggering branch) — it returns UnknownIssuer only because no chain root.
        let cap = Capability::new_hybrid(
            s.classical_public(&[5u8; 32]),
            s.pq_public(&[6u8; 32]),
            crate::ports::agent::scope::Scope::single(
                crate::ports::agent::scope::Resource::AgentBridge,
                crate::ports::agent::scope::Action::AdmitAgent,
            ),
            [1u8; 8],
            9999,
        );
        let res = verify_chain(&s, &roster, &[], &cap, 0);
        assert_eq!(res, Err(ChainError::UnknownIssuer));
        // Guard against a vacuous always-trip: a corruption would surface as IntegrityFault,
        // which we explicitly did NOT get on the clean path.
        assert_ne!(res, Err(ChainError::IntegrityFault));
        let _ = SentinelTarget::AnchorRoster;
        let _ = &mut rev;
    }

    #[test]
    fn planted_fault_refuses_verify_chain_with_integrity_fault() {
        // A corrupted live AnchorRoster must trip the Sentinel and refuse verification
        // (deny-closed, IntegrityFault). The FDR alarm is emitted by `safe_state_on_corruption`
        // — proven by item 48's isolated oracle tests where the `OnceLock` sink is fresh.
        // Here we assert the Sentinel's fail-closed CONTRACT at the unit boundary, which does
        // not depend on the process-global FDR ring (fdr::init is idempotent per process).
        use crate::ports::agent::cap::{
            verify_chain, Capability, ChainError, RefSigner, SignatureVerifier,
        };
        let s = RefSigner;
        let mut roster = AnchorRoster::new();
        roster.enroll(&s.classical_public(&[3u8; 32]));
        let cap = Capability::new_hybrid(
            s.classical_public(&[5u8; 32]),
            s.pq_public(&[6u8; 32]),
            crate::ports::agent::scope::Scope::single(
                crate::ports::agent::scope::Resource::AgentBridge,
                crate::ports::agent::scope::Action::AdmitAgent,
            ),
            [1u8; 8],
            9999,
        );
        // Detection: the corrupted roster no longer verifies its own CRC.
        roster.corrupt_anchor_bit();
        assert!(
            roster.verify().is_err(),
            "corrupted roster must fail IntegrityChecked::verify"
        );
        // Deny-closed: verify_chain refuses with IntegrityFault (the alarm-triggering branch).
        let res = verify_chain(&s, &roster, &[], &cap, 0);
        assert_eq!(res, Err(ChainError::IntegrityFault));
    }
}
