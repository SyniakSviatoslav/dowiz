//! THE SEAL'S SIGNATURE, held from outside `einfile-sig-mldsa` (W-MG-SIG): what the hub really wrote
//! into an image and a TSA mark is external pure ML-DSA-65 with the empty context, read back
//! with `dowiz_core::pq::dsa`, a second implementation that shares no code with `fips204`.

use super::tests::device;
use super::tsa::{self, PlatformTsa};
use super::{actor_id, SealedHub};
use crate::EventKind;
use einstore::evlog::doc::seal::{message, read_heads, read_seal};
use einstore::evlog::doc::{Anchor, AnchorCheck, Doc, Verdict, ALG_ML_DSA_65, KIND_PLATFORM_TSA};
use einstore::Store;
use dowiz_core::pq::dsa;

const JSON: &str = r#"{"id":"o1"}"#;

/// A real seal out of a real image: it verifies externally over its own message, and the
/// interface this format left does not accept it. `seals_ok` alone could not tell the two apart.
#[test]
fn a_seal_in_an_image_is_external_pure_ml_dsa() {
    let key = device(7);
    let mut sh = SealedHub::create(&key).unwrap();
    sh.append(EventKind::Placed, "o1", JSON, &key, 1).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let st = Store::from_bytes(&bytes);
    let view = Doc::open(&st).unwrap();
    let head = read_heads(&st, view.heads)[0];
    let seal = read_seal(&st, head.seal.unwrap()).unwrap();
    let prev_tip = seal.prev.map_or([0u8; 32], |at| read_seal(&st, at).unwrap().tip);
    let msg = message(&seal.actor_id, seal.seq, &seal.tip, seal.claimed_ms, &prev_tip);
    assert_eq!(seal.alg, ALG_ML_DSA_65);
    assert!(dsa::verify_pure_bytes(key.public_key(), &msg, &seal.sig), "the oracle verifies the hub's seal");
    assert!(!dsa::verify_internal_bytes(key.public_key(), &msg, &seal.sig), "and not as Sign_internal");
    assert!(sh.verify().unwrap().holds(), "the hub agrees");
}

/// The same for anchor kind 1, and a mark signed the old way is `Bad` under the same key.
#[test]
fn a_tsa_mark_is_external_pure_ml_dsa_and_an_old_one_is_bad() {
    let seed = [42u8; 32];
    let platform = super::DeviceKey::from_seed(&seed);
    let (over, time_ms) = ([9u8; 32], 1_730_000_000_000);
    let proof = tsa::mark(&platform, &over, time_ms);
    let (pk, sig) = proof.split_at(dsa::PUBLICKEYBYTES);
    let msg = tsa::message(KIND_PLATFORM_TSA, &over, time_ms);
    assert_eq!(pk, platform.public_key());
    assert!(dsa::verify_pure_bytes(pk, &msg, sig), "the oracle verifies the mark");

    let keys = [pk.to_vec()];
    let checker = PlatformTsa { keys: &keys };
    let anchor = |proof: Vec<u8>| Anchor { at: 0, kind: KIND_PLATFORM_TSA, over, time_ms, prev: None, proof };
    assert!(matches!(checker.check(&anchor(proof.clone())), Verdict::Ok { .. }), "control");

    let (_, sk) = dsa::keygen_bytes(&seed);
    let old = dsa::sign_internal_bytes(&sk, &msg, &[0u8; dsa::RNDBYTES]);
    let mut stale = pk.to_vec();
    stale.extend_from_slice(&old);
    assert!(matches!(checker.check(&anchor(stale)), Verdict::Bad(_)), "a Sign_internal mark");
}

/// The hub's `actor_id` is the crate's, and it still answers what the hub's sha256 answers.
#[test]
fn actor_id_did_not_move() {
    let pk = device(7);
    assert_eq!(actor_id(pk.public_key()), einstore::nodekey::sha256(pk.public_key()));
    assert_eq!(pk.id(), actor_id(pk.public_key()));
}
