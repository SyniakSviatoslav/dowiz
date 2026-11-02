// SPDX-License-Identifier: MIT OR Apache-2.0
//! `fips204` against `dowiz_core::pq::dsa`, two implementations with no code in common: same keys
//! from one seed, same signature bytes for one message, and either accepts what the other
//! makes and refuses what the other refuses. `dsa` is a dev-dependency only.

use einfile_log::doc::{Signer as _, Verifier as _, ALG_ML_DSA_65};
use einfile_sig_mldsa::{actor_id, DeviceKey, MlDsa65, PK_BYTES, SEED_BYTES, SIG_BYTES};
use dowiz_core::pq::dsa;
use dowiz_core::sha256_hw::sha256_hw as sha256;

/// Stable test input, derived so nothing here is a hand-typed constant.
fn seed(i: u8) -> [u8; SEED_BYTES] {
    sha256(&[b's', i])
}

fn message(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i as u8).wrapping_mul(31).wrapping_add(7)).collect()
}

/// Empty, one byte, a seal's message (124), a TSA mark's (62), past one SHAKE block, a long one.
const LENS: [usize; 6] = [0, 1, 62, 124, 137, 5000];

#[test]
fn the_two_implementations_make_the_same_bytes() {
    assert_eq!((PK_BYTES, SIG_BYTES, SEED_BYTES), (dsa::PUBLICKEYBYTES, dsa::SIGNATUREBYTES, dsa::SEEDBYTES));
    for i in 0..8u8 {
        let key = DeviceKey::from_seed(&seed(i));
        let (pk, sk) = dsa::keygen_bytes(&seed(i));
        assert_eq!(key.public_key(), pk.as_slice(), "seed {i}: pk");
        assert_eq!(key.id(), sha256(&pk), "seed {i}: the actor id is sha256(pk)");
        assert_eq!(MlDsa65.actor_of(&pk), Some(key.id()));
        for len in LENS {
            let msg = message(len);
            let ours = key.sign(&msg);
            let theirs = dsa::sign_pure_bytes(&sk, &msg, &[0u8; dsa::RNDBYTES]);
            assert_eq!(ours, theirs, "seed {i}, {len} bytes: signature");
            assert!(dsa::verify_pure_bytes(&pk, &msg, &ours), "seed {i}, {len} bytes: oracle verifies ours");
            assert!(MlDsa65.verify(ALG_ML_DSA_65, &pk, &msg, &theirs), "seed {i}, {len} bytes: we verify the oracle's");
        }
    }
}

/// One flipped bit in each region of a signature (c-tilde, z, the hint block's tail) and in the
/// key and the message: both sides refuse, each after a control that both accept.
#[test]
fn the_two_implementations_refuse_the_same_changes() {
    let key = DeviceKey::from_seed(&seed(9));
    let pk = key.public_key().to_vec();
    let msg = message(124);
    let sig = key.sign(&msg);
    let both = |pk: &[u8], msg: &[u8], sig: &[u8]| (MlDsa65.verify(ALG_ML_DSA_65, pk, msg, sig), dsa::verify_pure_bytes(pk, msg, sig));
    assert_eq!(both(&pk, &msg, &sig), (true, true), "control");
    for at in [0, 47, 48, 1500, 3253, 3254, SIG_BYTES - 7, SIG_BYTES - 1] {
        let mut bad = sig.clone();
        bad[at] ^= 1;
        assert_eq!(both(&pk, &msg, &bad), (false, false), "sig byte {at}");
    }
    for at in [0, 31, 32, PK_BYTES - 1] {
        let mut bad = pk.clone();
        bad[at] ^= 1;
        assert_eq!(both(&bad, &msg, &sig), (false, false), "pk byte {at}");
    }
    let mut bad = msg.clone();
    bad[0] ^= 1;
    assert_eq!(both(&pk, &bad, &sig), (false, false), "message");
}

/// THE FORMAT MOVED: a seal made the way this tree made them before W-MG-SIG, `Sign_internal`
/// over the bare message, is refused by name of what it is, and its external twin passes.
#[test]
fn an_internal_signature_is_not_a_seal() {
    let (pk, sk) = dsa::keygen_bytes(&seed(3));
    let msg = message(124);
    let old = dsa::sign_internal_bytes(&sk, &msg, &[0u8; dsa::RNDBYTES]);
    assert!(dsa::verify_internal_bytes(&pk, &msg, &old), "control: it is a good internal signature");
    assert!(!MlDsa65.verify(ALG_ML_DSA_65, &pk, &msg, &old), "an old seal must not pass");
    let new = dsa::sign_pure_bytes(&sk, &msg, &[0u8; dsa::RNDBYTES]);
    assert!(MlDsa65.verify(ALG_ML_DSA_65, &pk, &msg, &new), "the positive twin");
    assert!(!dsa::verify_internal_bytes(&pk, &msg, &new), "and the new one is not an internal one");
}

#[test]
fn a_wrong_algorithm_or_length_is_false_and_its_twin_is_true() {
    let key = DeviceKey::from_seed(&seed(1));
    let (pk, msg) = (key.public_key().to_vec(), b"ein-seal/1".to_vec());
    let sig = key.sign(&msg);
    assert!(MlDsa65.verify(ALG_ML_DSA_65, &pk, &msg, &sig), "control");
    assert!(!MlDsa65.verify(ALG_ML_DSA_65 + 1, &pk, &msg, &sig), "alg");
    assert!(!MlDsa65.verify(ALG_ML_DSA_65, &pk[1..], &msg, &sig), "short pk");
    assert!(!MlDsa65.verify(ALG_ML_DSA_65, &pk, &msg, &sig[1..]), "short sig");
    assert!(!MlDsa65.verify(ALG_ML_DSA_65, &pk, &msg, &[sig.as_slice(), &[0]].concat()), "long sig");
    assert_eq!(key.alg(), ALG_ML_DSA_65);
}

/// sha256 itself: `actor_id` against FIPS 180-4's "abc" and against the other implementation.
#[test]
fn actor_id_is_sha256() {
    let abc: Vec<String> = actor_id(b"abc").iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(abc.concat(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(actor_id(&message(4096)), sha256(&message(4096)));
}
