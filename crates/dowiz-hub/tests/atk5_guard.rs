//! W-MG-ATK5: THE LAST GUARD OF `merge_into`, `verify` over the union, and the input that reaches
//! it. `revoked` says "each file holds, so every REVOKE in it is effective there", and that is
//! false for one record: a REVOKE body on a PRE-SEAL record, whose actor has no key. `verify`
//! ignores it (`effective`: not admitted). `revoked` folds it. Both files hold; the union's table
//! names a REVOKE nothing signed; `keys_disagree` is 1, and the answer blames OUR file.
//! Nothing was signed by a forger: the record lies below the first seal.
mod atk5_common;
mod atk_forge;

use atk5_common::*;
use atk_forge::forged;
use einstore::evlog::doc::keys::{revoke_body, KeyOp};
use einstore::evlog::doc::{Doc, DocError, Ids, Signer, FLAG_SHA256_IDS};
use einstore::Store;
use dowiz_hub::doc_seal::merge::{MergeError, Merged};
use dowiz_hub::doc_seal::{HubIds, SealedHub, KIND_KEYOP, MIN_SEALED_BYTES};

/// Genesis over one pre-seal record that says "REVOKE B after 1", then A admits B and C.
fn over_a_dead_revoke() -> Vec<u8> {
    let (ka, kb) = (a(), b());
    let mut r = forged([9; 32], 1, [0; 32], 0x21);
    r.payload.truncate(16);
    r.payload.push(KIND_KEYOP);
    r.payload.extend_from_slice(&revoke_body(&kb.id(), 1, &[0; 32]));
    r.id = HubIds.chained(&r);
    let mut st = Store::create_bytes(MIN_SEALED_BYTES).unwrap();
    Doc::init_over_bytes(&mut st, FLAG_SHA256_IDS, &[ka.key()], &[r]).unwrap();
    let mut sh = SealedHub::load(&st.to_bytes_trimmed()).unwrap();
    sh.set_entropy([1; 32]);
    sh.sync_stock(&[received("rice", 10)], &ka, 1).unwrap();
    sh.commit_keyop(&KeyOp::Admit(kb.key()), &ka, 2).unwrap();
    trimmed(&sh)
}

#[test]
fn a_dead_revoke_does_not_reach_the_last_guard() {
    let s = over_a_dead_revoke();
    let (fa, fb) = (offline(&s, 2, &a(), &[served("rice", 1, "a")]), offline(&s, 3, &b(), &[received("tuna", 1)]));
    assert!(holds(&fa) && holds(&fb), "the control: each file holds, B is not revoked in either");
    let mut sh = opened(&fa, 4);
    match sh.merge(&fb, &a(), 500) {
        Ok(Merged::Sealed { .. }) => {}
        Err(MergeError::Refused(DocError::NotWhole { ours })) => panic!("HOLE 10: the last guard, reached: two files that hold, NotWhole {{ ours: {ours} }}"),
        other => panic!("{other:?}"),
    }
}

/// B writes one more line: the same dead record now reads as a REVOKE B never had.
#[test]
fn a_dead_revoke_does_not_refuse_an_honest_tail() {
    let s = over_a_dead_revoke();
    let fa = offline(&s, 2, &a(), &[served("rice", 1, "a")]);
    let fb = offline(&offline(&s, 3, &b(), &[received("tuna", 1)]), 5, &b(), &[received("tuna", 1)]);
    assert!(holds(&fa) && holds(&fb));
    let mut sh = opened(&fa, 4);
    let got = sh.merge(&fb, &a(), 500);
    assert!(matches!(got, Ok(Merged::Sealed { .. })), "HOLE 10b: {got:?}");
}
