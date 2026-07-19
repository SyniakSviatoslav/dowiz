#![cfg(feature = "pq")]

//! P68 — kernel-local supervisor drive wiring (RED→GREEN).
//!
//! Wires `hub_supervisor::seal`/`open` (M1/M2) + `decide_promote` (M5/M6) into a
//! real kernel-local drive that seals a backup of the pre-promote state, walks
//! the snapshot → health → flip state machine, and restores byte-identically on
//! rollback. Before this wiring, `drive_promote`/`drive_restore` did not exist
//! (zero production callers) ⇒ the test fails to compile (RED). After wiring it
//! compiles and passes (GREEN).

use dowiz_kernel::hub_supervisor::{
    decide_promote, drive_promote, drive_restore, MemStateStore, PromoteStep,
    RecipientPubKey, RecipientSet, SeededRng, Slot, StateStore, UpdateError, UpdateState, Version,
};
use dowiz_kernel::pq::x25519::x25519;

// RFC 7748 §5 X25519 basepoint: u = 9.
const X25519_BASEPOINT: [u8; 32] = [9u8; 32];

// Deterministic vendor keypair (scalar = [seed;32], pub = X25519(scalar, 9)).
fn vendor_keypair(seed: u8) -> ([u8; 32], [u8; 32]) {
    let sec = [seed; 32];
    let pubk = x25519(&sec, &X25519_BASEPOINT);
    (sec, pubk)
}

// Build a vendor recipient set from a single keypair seed.
fn recipients(seed: u8) -> RecipientSet {
    let (_sec, pubk) = vendor_keypair(seed);
    RecipientSet::from_vendor_config(vec![RecipientPubKey::from_vendor_config(pubk)]).unwrap()
}

#[test]
fn drive_promote_seals_then_restores_byte_identical() {
    let mut store = MemStateStore::new();
    store.append_event(b"v1-state-a");
    store.append_event(b"v1-state-b");
    let pre_epoch = store.chain_tip();

    let mut rng = SeededRng::new(0xc0ffee);
    let outcome = drive_promote(
        &mut store,
        &UpdateState::Migrated {
            into: Slot::B,
            version: Version("2.0.0".into()),
        },
        &None,
        &Version("2.0.0".into()),
        Slot::B,
        &recipients(0x11),
        &mut rng,
        true, // seal BEFORE promote — the mandatory ordering
    )
    .expect("drive_promote");

    assert_eq!(outcome.to_slot, Slot::B);
    let sealed = outcome
        .backup
        .expect("a backup must travel with the promote");

    // Rollback: re-open the sealed backup with the vendor identity and apply it.
    let (vsec, vpub) = vendor_keypair(0x11);
    let restored = drive_restore(&mut store, &sealed, &vsec, &vpub).expect("drive_restore");

    // The restored epoch must equal the pre-promote epoch, byte-for-byte.
    assert_eq!(restored, pre_epoch);
    assert_eq!(store.chain_tip(), pre_epoch);
}

#[test]
fn drive_promote_refuses_without_seal_before() {
    // The blueprint's age-snapshot-before-promote invariant: a promote that
    // bypasses the seal (seal_before_promote = false) MUST be refused — there is
    // no safe rollback if we never backed up the pre-promote state.
    let mut store = MemStateStore::new();
    store.append_event(b"e1");
    let mut rng = SeededRng::new(0xbeef);
    let res = drive_promote(
        &mut store,
        &UpdateState::Migrated {
            into: Slot::B,
            version: Version("2.0.0".into()),
        },
        &None,
        &Version("2.0.0".into()),
        Slot::B,
        &recipients(0x22),
        &mut rng,
        false, // FORBIDDEN: no seal before promote
    );
    assert!(matches!(res, Err(UpdateError::SnapshotFailed)));
}

#[test]
fn drive_promote_requires_snapshot_before_flip() {
    // From Idle (no migration yet) decide_promote refuses to reach FlipSymlink,
    // so drive_promote cannot promote — the mandatory pre-promote snapshot is
    // enforced by the state machine, not by a flag.
    let mut store = MemStateStore::new();
    store.append_event(b"e1");
    let mut rng = SeededRng::new(0x1234);
    let res = drive_promote(
        &mut store,
        &UpdateState::Idle {
            current: Slot::A,
            pinned: None,
        },
        &None,
        &Version("2.0.0".into()),
        Slot::B,
        &recipients(0x33),
        &mut rng,
        true,
    );
    assert!(matches!(res, Err(_)));
    // The decide step from Idle must refuse (no snapshot taken).
    let step = decide_promote(
        &UpdateState::Idle {
            current: Slot::A,
            pinned: None,
        },
        &None,
        &Version("2.0.0".into()),
    );
    assert!(matches!(step, PromoteStep::Refuse(_)));
}
