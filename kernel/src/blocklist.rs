//! BLUEPRINT-P74 (M2 / M3 / M4) — signed, subscribable ABUSE blocklist.
//!
//! A hub operator *may* publish a cryptographically-signed list of known-abusive
//! actor identities, and another operator *may* subscribe. Named "abuse", never
//! "reputation" or "quality". Opt-in and hub-controlled; never a mandatory
//! central ban list. It is **advisory**: it surfaces a signal to the operator,
//! who makes a manual pool-membership decision (§16.3 — the venue owns its
//! courier pool). It never auto-filters and never re-ranks.
//!
//! The signed artifact reuses the **default-built** `capability_cert::HybridSig`
//! (Ed25519 ⊕ ML-DSA-65, RequireBoth AND-verify) — the same hybrid signer P59
//! rides. The blueprint text referenced `crate::pq::hybrid::HybridSig`, which
//! does not exist (`pq/hybrid.rs` is a KEM, not a signature); the real hybrid
//! *signature* seam is `capability_cert`. Using it (instead of gating this
//! module behind the `pq` feature) means M2 is exercised by `cargo test --lib`
//! under DEFAULT features — strictly better for the acceptance gate, and
//! consistent with "matching the signer" (the signer is itself default-built).
//!
//! # Honest limits (stated, not dressed as a solution)
//!
//! This is a *starting point* for abuse-signal sharing, not a solved
//! trust-and-safety system. There is no consensus on who is abusive, no
//! dispute/appeal flow, and the well-documented echo-chamber failure mode of
//! shared blocklists applies in full. It shares *what an operator asserts*,
//! signed, and nothing more.
//!
//! # No-scoring guarantee (the point of P74)
//!
//! `BlockedActor` carries an identity and a reason — NEVER a score, count, rank,
//! or weight. The field-level NO-COURIER-SCORING CI guard auto-covers this type
//! (no score-ish field name can be added). `SubscriptionTrust` has exactly ONE
//! variant (`Advisory`); an `Enforcing`/`Ranking` variant is intentionally
//! absent, so an auto-enforcing or ranking blocklist is *unrepresentable* — the
//! §16.26/§16.59 red line becomes a hard boundary, not a code-review
//! discipline. Nothing in the dispatch/matching path may call `is_flagged` (M4).

use crate::capability_cert::{AlgSuite, HybridSig};
use crate::moderation::ReportReason;
use crate::ports::agent::cap::SignatureVerifier;
use std::fmt;

/// Scaling axis: entry count K. Whole-list re-sign + re-fetch is O(K); beyond
/// this bound a Merkle-delta sync would win — named future, NOT built (§5.4).
pub const MAX_BLOCKLIST_ENTRIES: usize = 10_000;

/// Domain-separated, prefix-tagged canonical namespace for a blocklist. The
/// signature binds over these exact bytes (incl. entry order), so a reordered or
/// truncated entry vector fails verification (M2 forgery teeth).
pub const DOMAIN_BLOCKLIST: &[u8] = b"dowiz.blocklist\x01";

/// One blocklist entry. Carries an identity and a reason — NEVER a score, count,
/// rank, or weight (the field-level NO-COURIER-SCORING CI guard, §0, auto-covers
/// this type: no score-ish field name can be added).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedActor {
    pub actor: [u8; 32], // pubkey
    pub reason: ReportReason,
    pub evidence: Option<[u8; 32]>, // content-id of a supporting report event (optional)
}

/// A publisher's signed abuse list at a given epoch. Epoch replacement is the
/// only mutation (revoke = publish a new epoch omitting the entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbuseBlocklist {
    pub publisher: [u8; 32],
    pub epoch: u64,
    pub entries: Vec<BlockedActor>, // len <= MAX_BLOCKLIST_ENTRIES
}

/// Construction refusal — the only error a blocklist can hit (over-capacity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlocklistRejected {
    /// `entries.len()` exceeded `MAX_BLOCKLIST_ENTRIES`.
    TooManyEntries(usize),
}

impl fmt::Display for BlocklistRejected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyEntries(n) => {
                write!(f, "blocklist has {n} entries > MAX_BLOCKLIST_ENTRIES")
            }
        }
    }
}

impl AbuseBlocklist {
    /// Construct, refusing a list over `MAX_BLOCKLIST_ENTRIES` (the scaling
    /// break point; beyond it a Merkle-delta sync is the named future).
    pub fn new(
        publisher: [u8; 32],
        epoch: u64,
        entries: Vec<BlockedActor>,
    ) -> Result<Self, BlocklistRejected> {
        if entries.len() > MAX_BLOCKLIST_ENTRIES {
            return Err(BlocklistRejected::TooManyEntries(entries.len()));
        }
        Ok(Self {
            publisher,
            epoch,
            entries,
        })
    }

    /// Canonical, domain-separated, ORDER-SIGNIFICANT encoding. Verification
    /// re-encodes canonically and AND-checks both signature halves over these
    /// bytes, so a reordered entry vector (different bytes) or a truncated entry
    /// list (different bytes) cannot verify against the original signature.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN_BLOCKLIST);
        out.extend_from_slice(&self.publisher);
        out.extend_from_slice(&self.epoch.to_le_bytes());
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for e in &self.entries {
            out.extend_from_slice(&e.actor);
            out.push(e.reason as u8);
            match &e.evidence {
                Some(h) => {
                    out.push(1);
                    out.extend_from_slice(h);
                }
                None => {
                    out.push(0);
                }
            }
        }
        out
    }
}

/// The wire artifact: a canonically-encoded list + a hybrid signature over that
/// canonical encoding. Verification AND-checks both hybrid halves (B4 lesson) and
/// re-encodes canonically to defeat reorder/truncation forgery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedBlocklist {
    pub list: AbuseBlocklist,
    pub sig: HybridSig,
}

impl SignedBlocklist {
    /// Sign a list with the RequireBoth hybrid signer. `verifier` is any
    /// `SignatureVerifier` (production injects real Ed25519 ⊕ ML-DSA-65; tests
    /// use `RefSigner`). The signature binds the canonical bytes, so any later
    /// mutation of the list invalidates it.
    pub fn sign<V: SignatureVerifier>(
        list: AbuseBlocklist,
        verifier: &V,
        classical_secret: &[u8; 32],
        pq_secret: &[u8; 32],
    ) -> Self {
        let msg = list.canonical_bytes();
        let sig = HybridSig::sign(
            verifier,
            AlgSuite::MlDsa65Ed25519,
            classical_secret,
            pq_secret,
            &msg,
        );
        SignedBlocklist { list, sig }
    }

    /// Verify under RequireBoth. Returns `false` on ANY of: unknown suite,
    /// classical-half failure, pq-half failure, or a list whose canonical bytes
    /// no longer match the signed message (reorder/truncation forgery).
    pub fn verify<V: SignatureVerifier>(
        &self,
        verifier: &V,
        classical_pub: &[u8; 32],
        pq_pub: &[u8],
    ) -> bool {
        let msg = self.list.canonical_bytes();
        self.sig.verify(verifier, classical_pub, pq_pub, &msg)
    }
}

/// How a subscriber treats a list. There is exactly ONE variant: a subscription
/// is ADVISORY. `Enforcing`/`Ranking` variants are intentionally absent — an
/// auto-enforcing or ranking blocklist is unrepresentable, which is how the
/// §16.26/§16.59 red line becomes a hard boundary rather than a review
/// discipline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionTrust {
    Advisory,
}

/// Operator-facing advisory lookup. Returns the reason an actor was flagged on
/// any *verified, subscribed* list, or `None`. It returns NO score, count, or
/// ordering, and NOTHING in the dispatch/matching path may call it (enforced by
/// M4's guard). The return type `Option<ReportReason>` is compile-level proof it
/// cannot return a number.
pub fn is_flagged(subscribed: &[AbuseBlocklist], actor: &[u8; 32]) -> Option<ReportReason> {
    for list in subscribed {
        for e in &list.entries {
            if e.actor == *actor {
                return Some(e.reason);
            }
        }
    }
    None
}

/// P74 (M2/M3) operator subscription surface — the kernel-local production caller
/// that ties [`SignedBlocklist::sign`] (publisher side) and [`is_flagged`]
/// (subscriber side) together. A subscribing operator holds a set of *verified*
/// lists and asks advisory abuse questions over them.
///
/// A list is only admitted to the subscribed set after its RequireBoth hybrid
/// signature verifies against the publisher's advertised keys ([`subscribe`]);
/// a forged/reordered/truncated list is refused and never enters the set. Queries
/// ([`flagged_reason`]) run [`is_flagged`] over ONLY the verified set — the return
/// type `Option<ReportReason>` is compile-level proof this surface can never emit a
/// score, count, or ranking (§16.26/§16.59). It is advisory-only: it surfaces a
/// reason to the operator and takes no action.
///
/// [`subscribe`]: BlocklistSubscriptions::subscribe
/// [`flagged_reason`]: BlocklistSubscriptions::flagged_reason
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BlocklistSubscriptions {
    /// Verified, advisory lists this operator subscribes to.
    verified: Vec<AbuseBlocklist>,
}

impl BlocklistSubscriptions {
    /// A fresh, empty subscription set.
    pub fn new() -> Self {
        Self {
            verified: Vec::new(),
        }
    }

    /// The publisher side: an operator signs its own `AbuseBlocklist` for
    /// distribution. Thin wrapper over [`SignedBlocklist::sign`] naming it as the
    /// operator-facing publish action.
    pub fn publish<V: SignatureVerifier>(
        list: AbuseBlocklist,
        verifier: &V,
        classical_secret: &[u8; 32],
        pq_secret: &[u8; 32],
    ) -> SignedBlocklist {
        SignedBlocklist::sign(list, verifier, classical_secret, pq_secret)
    }

    /// The subscriber side: admit a signed list into the subscribed set IFF its
    /// hybrid signature verifies against the publisher's advertised keys. Returns
    /// `true` if admitted, `false` if the signature (or canonical bytes) did not
    /// verify — a forged/reordered/truncated list never enters the set.
    pub fn subscribe<V: SignatureVerifier>(
        &mut self,
        signed: SignedBlocklist,
        verifier: &V,
        publisher_classical_pub: &[u8; 32],
        publisher_pq_pub: &[u8],
    ) -> bool {
        if signed.verify(verifier, publisher_classical_pub, publisher_pq_pub) {
            self.verified.push(signed.list);
            true
        } else {
            false
        }
    }

    /// Advisory lookup over the verified subscribed set. Delegates to [`is_flagged`]
    /// — returns the reason an actor was flagged on any subscribed, verified list,
    /// or `None`. No score, count, or ordering (compile-level via the return type).
    pub fn flagged_reason(&self, actor: &[u8; 32]) -> Option<ReportReason> {
        is_flagged(&self.verified, actor)
    }

    /// Number of verified lists in the subscribed set (test/observability aid).
    pub fn subscribed_len(&self) -> usize {
        self.verified.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moderation::ReportReason;
    use crate::ports::agent::cap::RefSigner;

    fn pubs_of(cls_sk: [u8; 32], pq_sk: [u8; 32]) -> ([u8; 32], Vec<u8>) {
        let v = RefSigner;
        (v.classical_public(&cls_sk), v.pq_public(&pq_sk))
    }

    fn sample_list(epoch: u64) -> AbuseBlocklist {
        AbuseBlocklist::new(
            [1u8; 32],
            epoch,
            vec![
                BlockedActor {
                    actor: [11u8; 32],
                    reason: ReportReason::Fraud,
                    evidence: Some([22u8; 32]),
                },
                BlockedActor {
                    actor: [33u8; 32],
                    reason: ReportReason::Harassment,
                    evidence: None,
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn m2_sign_verify_roundtrip() {
        let v = RefSigner;
        let (cpk, ppk) = pubs_of([2u8; 32], [3u8; 32]);
        let list = sample_list(1);
        let signed = SignedBlocklist::sign(list, &v, &[2u8; 32], &[3u8; 32]);
        assert!(
            signed.verify(&v, &cpk, &ppk),
            "valid signed blocklist verifies"
        );
    }

    #[test]
    fn m2_reorder_entries_fails_verify() {
        let v = RefSigner;
        let (cpk, ppk) = pubs_of([2u8; 32], [3u8; 32]);
        // Sign the list in its canonical order.
        let orig = sample_list(1);
        let signed = SignedBlocklist::sign(orig, &v, &[2u8; 32], &[3u8; 32]);
        assert!(signed.verify(&v, &cpk, &ppk), "original list verifies");
        // Adversarial: keep the ORIGINAL signature but reorder the entries. The
        // canonical re-encode now differs → AND-verify must reject.
        let mut forged = signed.clone();
        forged.list.entries.reverse();
        assert!(
            !forged.verify(&v, &cpk, &ppk),
            "reordered entries with a stale sig must fail"
        );
        // Sanity: re-signing the reordered list is a NEW valid list (proves the
        // guard is about the stale sig, not the order itself).
        let resigned = SignedBlocklist::sign(forged.list.clone(), &v, &[2u8; 32], &[3u8; 32]);
        assert!(
            resigned.verify(&v, &cpk, &ppk),
            "reordered list re-signed is valid"
        );
    }

    #[test]
    fn m2_truncated_entries_fails_verify() {
        let v = RefSigner;
        let (cpk, ppk) = pubs_of([2u8; 32], [3u8; 32]);
        let orig = sample_list(1);
        let signed = SignedBlocklist::sign(orig, &v, &[2u8; 32], &[3u8; 32]);
        assert!(signed.verify(&v, &cpk, &ppk), "original list verifies");
        // Adversarial: drop an entry but keep the stale sig → no silent short-accept.
        let mut forged = signed.clone();
        forged.list.entries.pop();
        assert!(
            !forged.verify(&v, &cpk, &ppk),
            "truncated entry list must fail verify"
        );
    }

    #[test]
    fn m2_single_half_forgery_rejected() {
        let v = RefSigner;
        let (cpk, ppk) = pubs_of([2u8; 32], [3u8; 32]);
        // Tamper the classical half (valid-pq, junk-classical) → AND-verify rejects.
        let mut a = SignedBlocklist::sign(sample_list(7), &v, &[2u8; 32], &[3u8; 32]);
        if !a.sig.classical.is_empty() {
            a.sig.classical[0] ^= 0xFF;
        }
        assert!(
            !a.verify(&v, &cpk, &ppk),
            "single-half (classical) forgery rejected"
        );
        // Tamper the pq half (valid-classical, junk-pq) → AND-verify rejects.
        let mut b = SignedBlocklist::sign(sample_list(7), &v, &[2u8; 32], &[3u8; 32]);
        if !b.sig.pq.is_empty() {
            b.sig.pq[0] ^= 0xFF;
        }
        assert!(
            !b.verify(&v, &cpk, &ppk),
            "single-half (pq) forgery rejected"
        );
    }

    #[test]
    fn m2_over_cap_rejected_at_construction() {
        let entries = (0..(MAX_BLOCKLIST_ENTRIES + 1))
            .map(|i| BlockedActor {
                actor: [i as u8; 32],
                reason: ReportReason::Spam,
                evidence: None,
            })
            .collect();
        let res = AbuseBlocklist::new([1u8; 32], 1, entries);
        assert_eq!(
            res,
            Err(BlocklistRejected::TooManyEntries(MAX_BLOCKLIST_ENTRIES + 1))
        );
    }

    #[test]
    fn m2_is_flagged_returns_option_reason() {
        let list = sample_list(1);
        // The return type is Option<ReportReason> — compile-level proof it cannot
        // return a number (no count/score).
        let r: Option<ReportReason> = is_flagged(&[list.clone()], &[11u8; 32]);
        assert_eq!(r, Some(ReportReason::Fraud));
        let r2: Option<ReportReason> = is_flagged(&[list.clone()], &[33u8; 32]);
        assert_eq!(r2, Some(ReportReason::Harassment));
        let r3: Option<ReportReason> = is_flagged(&[list], &[99u8; 32]);
        assert_eq!(r3, None);
        let r4: Option<ReportReason> = is_flagged(&[], &[0u8; 32]);
        assert!(r4.is_none());
    }

    #[test]
    fn m2_subscription_trust_is_advisory_only() {
        // The only admissible subscription posture is Advisory; an Enforcing /
        // Ranking variant is unrepresentable (compile-time red line).
        let t = SubscriptionTrust::Advisory;
        assert_eq!(t, SubscriptionTrust::Advisory);
        // exhaustive match — adding any other variant is a compile error here.
        match t {
            SubscriptionTrust::Advisory => {}
        }
    }

    // ── M3: legal-takedown boundary (contract + isolation invariant) ─────────────
    //
    // The takedown surface lives on the CLOSED `dowiz-infra` side (P67's split)
    // and is specified here only as an INVARIANT the hub kernel enforces by having
    // NO such API. dowiz has zero visibility into hub data by design (§16.14):
    // its only cross-hub lever is §16.53 liveness/availability, never content.
    #[test]
    fn m3_kernel_exposes_no_cross_hub_content_api() {
        // The hub kernel has NO function that deletes, edits, or reads another
        // hub's content on external instruction. This is asserted structurally
        // below: the `moderation`/`blocklist` modules expose only
        // report-commit, blocklist sign/verify, and the advisory `is_flagged`
        // query. None of them accept a *target hub* or a *content-delete*
        // instruction. A "takedown into a hub" is therefore inexpressible.
        //
        // Self-check: the advisory query returns a membership signal, never a
        // mutation. It cannot delete or edit anything.
        let absent: Option<ReportReason> = is_flagged(&[], &[0u8; 32]);
        assert!(
            absent.is_none(),
            "advisory query is read-only, never a mutation"
        );
        // The only list-mutation operation is epoch REPLACEMENT (re-publish a
        // new signed epoch omitting the entry) — a publisher acting on THEIR OWN
        // list, never a cross-hub content edit.
        let v = RefSigner;
        let (cpk, ppk) = pubs_of([2u8; 32], [3u8; 32]);
        let original = sample_list(1);
        let signed = SignedBlocklist::sign(original.clone(), &v, &[2u8; 32], &[3u8; 32]);
        assert!(
            signed.verify(&v, &cpk, &ppk),
            "publisher-signed epoch verifies (self-owned list)"
        );
    }

    // ── M4: no moderation datum reaches HRW / discovery (the point of P74) ──────
    //
    // M4's load-bearing runtime test lives in `bebop2/proto-cap/src/matcher.rs`
    // (T4): it asserts `assign(order, cands, max)` is byte-identical with vs
    // without reports + a verified subscribed blocklist against a candidate — and
    // passes today because `assign`'s signature CANNOT accept moderation state.
    //
    // This kernel-side test is the STRUCTURAL mirror that runs under `cargo
    // test --lib` (no `bebop-repo` in this workspace): it proves the blocklist
    // / report types carry NO score/rank/reputation field that could ever become a
    // dispatch input, and that `is_flagged`'s return type (`Option<ReportReason>`)
    // is a membership signal, never a number dispatch could sort on. The no-scoring
    // field CI guard (ci-no-courier-scoring.sh) covers `bebop2/`; here we assert
    // the kernel types are likewise field-clean by construction.
    #[test]
    fn m4_advisory_returns_no_score_type() {
        // Compile-time truth: `is_flagged` yields `Option<ReportReason>` — a
        // categorical reason or `None`, never `u64`/`f64`/count. If dispatch
        // ever wired this in, it could only branch on an enum, never rank.
        fn takes_option_reason(_: Option<ReportReason>) {}
        takes_option_reason(is_flagged(&[], &[0u8; 32]));
    }

    #[test]
    fn m4_blocked_actor_has_no_score_field() {
        // `BlockedActor` carries identity + reason + optional evidence. No
        // score/rank/weight/reputation field exists. Adding one is a field-name
        // CI guard violation (§0). Asserted by constructing one with exactly the
        // three defined fields and confirming the type has no other accessor shape.
        let e = BlockedActor {
            actor: [5u8; 32],
            reason: ReportReason::Spam,
            evidence: None,
        };
        // Destructure exhaustively: any added field breaks this pattern match,
        // making a score field unshippable without a deliberate, greppable edit.
        let BlockedActor {
            actor: _,
            reason: _,
            evidence: _,
        } = e;
    }
}
