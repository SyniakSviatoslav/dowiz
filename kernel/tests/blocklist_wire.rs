//! P74 (M2/M3) wiring gate — `SignedBlocklist::sign` and `is_flagged` must be
//! reachable through a REAL operator subscription surface
//! (`blocklist::BlocklistSubscriptions`), not only through unit tests.
//!
//! RED before wiring: `BlocklistSubscriptions` does not exist → this test fails to
//! compile. GREEN after wiring: a signed list verifies and enters the subscribed
//! set, `flagged_reason` resolves a flagged actor to its `ReportReason`, and a
//! forged (tampered) list fails verification and never enters the set.

use dowiz_kernel::blocklist::{AbuseBlocklist, BlockedActor, BlocklistSubscriptions};
use dowiz_kernel::moderation::ReportReason;
use dowiz_kernel::ports::agent::cap::{RefSigner, SignatureVerifier};

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
fn p74_signed_list_subscribes_and_flags_actor() {
    let v = RefSigner;
    let (cpk, ppk) = pubs_of([2u8; 32], [3u8; 32]);
    let signed = BlocklistSubscriptions::publish(sample_list(1), &v, &[2u8; 32], &[3u8; 32]);

    let mut subs = BlocklistSubscriptions::new();
    assert!(
        subs.subscribe(signed, &v, &cpk, &ppk),
        "valid signed list must be admitted to the subscribed set"
    );
    assert_eq!(subs.subscribed_len(), 1);
    // is_flagged resolves a flagged actor to its ReportReason over the verified set.
    assert_eq!(subs.flagged_reason(&[11u8; 32]), Some(ReportReason::Fraud));
    assert_eq!(
        subs.flagged_reason(&[33u8; 32]),
        Some(ReportReason::Harassment)
    );
    // An unflagged actor resolves to None (no score, just absence).
    assert_eq!(subs.flagged_reason(&[99u8; 32]), None);
}

#[test]
fn p74_forged_list_fails_verify_and_is_not_subscribed() {
    let v = RefSigner;
    let (cpk, ppk) = pubs_of([2u8; 32], [3u8; 32]);
    let mut signed = BlocklistSubscriptions::publish(sample_list(1), &v, &[2u8; 32], &[3u8; 32]);
    // Tamper: append an entry AFTER signing → canonical bytes change → sig fails.
    signed.list.entries.push(BlockedActor {
        actor: [44u8; 32],
        reason: ReportReason::Fraud,
        evidence: None,
    });

    let mut subs = BlocklistSubscriptions::new();
    assert!(
        !subs.subscribe(signed, &v, &cpk, &ppk),
        "forged/tampered list must fail verification"
    );
    assert_eq!(subs.subscribed_len(), 0, "forged list never enters the set");
    // The tampered-in actor is NOT flagged because its list never subscribed.
    assert_eq!(subs.flagged_reason(&[44u8; 32]), None);
}
