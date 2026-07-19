//! P59 wiring gate — `capability_cert::verify_chain_hybrid` must be reachable
//! through a REAL dowiz-side production caller on the owner-surface claim/request
//! path (`owner_surface::verify_claim_cap_chain`), not only through unit tests.
//!
//! RED before wiring: `verify_claim_cap_chain` does not exist → this test fails to
//! compile. GREEN after wiring: a valid anchor-rooted hybrid chain verifies `Ok`,
//! and a forged (wrong-issuer-secret) link is refused `BadSignature`.

use dowiz_kernel::capability_cert::{AlgSuite, CertDelegation, RevocationStore, SelfSignedRoot};
use dowiz_kernel::ports::agent::cap::{AnchorRoster, Capability, RefSigner, SignatureVerifier};
use dowiz_kernel::ports::agent::scope::{Action, Resource, Scope};
use dowiz_kernel::ports::owner_surface::{verify_claim_cap_chain, OwnerSurfaceError};

fn scope() -> Scope {
    Scope::single(Resource::Route, Action::Send)
}

struct Party {
    cls_seed: [u8; 32],
    pq_seed: [u8; 32],
    cls_pub: [u8; 32],
    pq_pub: Vec<u8>,
}
impl Party {
    fn new(v: &RefSigner, i: u8) -> Self {
        let cls_seed = [i; 32];
        let pq_seed = [i.wrapping_add(100); 32];
        Party {
            cls_pub: v.classical_public(&cls_seed),
            pq_pub: v.pq_public(&pq_seed),
            cls_seed,
            pq_seed,
        }
    }
}

#[test]
fn p59_valid_chain_verifies_through_owner_caller() {
    let v = RefSigner;
    let owner = Party::new(&v, 9);
    let hub = Party::new(&v, 2);
    let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
    let mut roster = AnchorRoster::new();
    roster.enroll(&owner.cls_pub);
    // Owner → hub single-hop child, may_delegate = false (P59 §2.4).
    let link = CertDelegation::sign(
        &v,
        &owner.cls_seed,
        &owner.pq_seed,
        owner.cls_pub,
        owner.pq_pub.clone(),
        hub.cls_pub,
        hub.pq_pub.clone(),
        scope(),
        scope(),
        false,
        AlgSuite::MlDsa65Ed25519,
        99999,
        [1u8; 8],
    );
    let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 99999);
    let store = RevocationStore::new();
    assert_eq!(
        verify_claim_cap_chain(&v, &roster, &store, &root, &[link], &cap, 0),
        Ok(())
    );
}

#[test]
fn p59_forged_link_rejected_through_owner_caller() {
    let v = RefSigner;
    let owner = Party::new(&v, 9);
    let attacker = Party::new(&v, 5);
    let hub = Party::new(&v, 2);
    let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 99999);
    let mut roster = AnchorRoster::new();
    roster.enroll(&owner.cls_pub);
    // Forgery: claim the owner as issuer but sign under the attacker's secret keys.
    let forged = CertDelegation::sign(
        &v,
        &attacker.cls_seed,
        &attacker.pq_seed,
        owner.cls_pub, // claiming owner as issuer
        owner.pq_pub.clone(),
        hub.cls_pub,
        hub.pq_pub.clone(),
        scope(),
        scope(),
        false,
        AlgSuite::MlDsa65Ed25519,
        99999,
        [7u8; 8],
    );
    let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 99999);
    let store = RevocationStore::new();
    assert_eq!(
        verify_claim_cap_chain(&v, &roster, &store, &root, &[forged], &cap, 0),
        Err(OwnerSurfaceError::BadSignature)
    );
}
