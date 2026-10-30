//! THE REAL SEAL: ML-DSA-65 (FIPS 204) through the `Signer` / `Verifier` seams, from
//! `dowiz-core` as a DEV-dependency only -- the library stays MIT OR Apache-2.0 with no
//! signature code; `cargo tree -e no-dev` of this crate shows bebop-image alone.
//!
//! The actor id convention here is `sha256(pk)` (R-MAINGOAL-A §1.1); dowiz-hub's adapter
//! decides it for production and this crate never derives it.

use super::keys::{Key, KeyOp, ALG_ML_DSA_65};
use super::tests::{key_of, Dev};
use super::verify::{verify, Trust};
use super::{Doc, DocError, Signer, Verifier};
use crate::Store;
use dowiz_core::pq::dsa::{keygen, sign, verify_internal_bytes, MlDsa65Pk, MlDsa65Sk, PUBLICKEYBYTES, SIGNATUREBYTES};
use dowiz_core::sha256_hw::sha256_hw as sha256;

struct MlDsa {
    pk: MlDsa65Pk,
    sk: MlDsa65Sk,
    id: [u8; 32],
}
impl MlDsa {
    fn from_seed(seed: [u8; 32]) -> Self {
        let (pk, sk) = keygen(&seed);
        let id = sha256(&pk.bytes);
        MlDsa { pk, sk, id }
    }
}
impl Signer for MlDsa {
    fn actor_id(&self) -> [u8; 32] {
        self.id
    }
    fn alg(&self) -> i64 {
        ALG_ML_DSA_65
    }
    fn public_key(&self) -> Vec<u8> {
        self.pk.bytes.clone()
    }
    /// Deterministic (FIPS 204 `rnd` = 0^32), as the published price root is signed.
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        sign(&self.sk, msg, &[0u8; 32]).bytes
    }
}
struct MlDsaVerifier;
impl Verifier for MlDsaVerifier {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        alg == ALG_ML_DSA_65 && pk.len() == PUBLICKEYBYTES && sig.len() == SIGNATUREBYTES && verify_internal_bytes(pk, msg, sig)
    }
}

fn st_digest(s: &str) -> i64 {
    let h = sha256(s.as_bytes());
    u32::from_be_bytes([h[28], h[29], h[30], h[31]]) as i64
}

/// The five pinned digests are `st_digest` of their layout strings (FORMAT §5), with the
/// two the format already pins as the control.
#[test]
fn digests_are_st_digest_of_their_layouts() {
    assert_eq!(st_digest("KV{i64,ref [i64],ref [i64],ref [i64],ref [i64]}"), 610082063, "control: DIGEST_KV_ROOT");
    assert_eq!(st_digest("OUT{i64,i64}"), 514297014, "control: DIGEST_OUT");
    assert_eq!(st_digest(super::LAYOUT_DOC), super::DIGEST_DOC);
    assert_eq!(st_digest(super::seal::LAYOUT_SEAL), super::seal::DIGEST_SEAL);
    assert_eq!(st_digest(super::seal::LAYOUT_HEADS), super::seal::DIGEST_HEADS);
    assert_eq!(st_digest(super::keys::LAYOUT_KEY), super::keys::DIGEST_KEY);
    assert_eq!(st_digest(super::keys::LAYOUT_KEYS), super::keys::DIGEST_KEYS);
}

/// The seal is 429 cells (15 + 3 309 / 8 rounded up) and the key 253 (9 + 1 952 / 8), as
/// R-MAINGOAL-A §1.2 sized them; a changed tip under a REAL signature is `seals_bad`.
#[test]
fn ml_dsa_65_seals_and_a_changed_tip_is_bad() {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut owner = Dev::new(MlDsa::from_seed([7u8; 32]));
    Doc::init_bytes(&mut st, 0, &[key_of(&owner.s)]).unwrap();
    owner.commit(&mut st, b"first", None).unwrap();
    owner.commit(&mut st, b"second", None).unwrap();
    let view = Doc::open(&st).unwrap();
    let head_seal = super::seal::read_heads(&st, view.heads)[0].seal.unwrap();
    assert_eq!(st.obj_len(head_seal), 15 + 414, "SEAL cells");
    let key_at = super::keys::key_rows(&st, view.keys)[0].1;
    assert_eq!(st.obj_len(key_at), 9 + 244, "KEY cells");

    let rep = verify(&st, &super::tests::ToyIds, &MlDsaVerifier, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad), (2, 0), "control: {rep:?}");

    let heads = view.heads.unwrap();
    let clean = st.to_bytes_trimmed();
    st.put_cell(heads, 1 + 5, st.get(heads, 1 + 5) ^ 1);
    st.seal(heads);
    let rep = verify(&st, &super::tests::ToyIds, &MlDsaVerifier, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad), (1, 1), "changed tip: {rep:?}");

    // W-MG-M0's finding, under the real signature: an edit of the FIRST record repaired under
    // the id scheme (every id and prev recomputed, no gap) moves both tips -- both seals bad.
    let mut st = Store::from_bytes(&clean);
    super::tests::repair_edit(&mut st, 0);
    let rep = verify(&st, &super::tests::ToyIds, &MlDsaVerifier, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.chained, rep.counters.broken), (2, 0), "no gap: {rep:?}");
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad), (0, 2), "repaired edit: {rep:?}");
}

/// REVOKE after seq k breaks k+1 under ML-DSA-65: the owner admits B, B seals 1..2, the
/// owner revokes B after 1; B's second seal is bad and B's third commit is refused.
#[test]
fn ml_dsa_65_revoke_after_k_breaks_k_plus_1() {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut owner = Dev::new(MlDsa::from_seed([7u8; 32]));
    let mut b = Dev::new(MlDsa::from_seed([9u8; 32]));
    Doc::init_bytes(&mut st, 0, &[key_of(&owner.s)]).unwrap();
    let admit = KeyOp::Admit(Key { valid_from: 1, ..key_of(&b.s) });
    owner.commit(&mut st, &admit.body(), Some(&admit)).unwrap();
    b.commit(&mut st, b"b-1", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    let rep = verify(&st, &super::tests::ToyIds, &MlDsaVerifier, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad, rep.admitted), (3, 0, 2), "control: {rep:?}");

    let revoke = KeyOp::Revoke { actor_id: b.s.id, after_seq: 1 };
    owner.commit(&mut st, &revoke.body(), Some(&revoke)).unwrap();
    let rep = verify(&st, &super::tests::ToyIds, &MlDsaVerifier, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad), (3, 1), "B's seal 2 is past the revoke: {rep:?}");
    assert_eq!(b.commit(&mut st, b"b-3", None), Err(DocError::Revoked { after_seq: 1, seq: 3 }));
}
