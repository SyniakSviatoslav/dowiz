//! W-MG-ATK2: `verify` never asks that a device's seqs climb, and `covered` is a number, not a walk.
//! A record at a seq no record holds, at or below a key's seals, counts as sealed by that key.
//! The writers start at 1 (`Doc::commit_bytes`: `want = head.seq + 1`), so seq 0 is always free.
//! RED ON PURPOSE until `verify` closes it. `Trust::Only`, real ML-DSA-65.
mod atk_forge;

use atk_forge::*;
use bebop_store::evlog::doc::keys::{Key, KeyOp};
use bebop_store::evlog::doc::seal::{message, Seal};
use bebop_store::evlog::doc::{Ids, Signer, ALG_ML_DSA_65};
use bebop_store::evlog::{Record, FLAG_SHA256};
use dowiz_hub::doc_seal::{DeviceKey, HubIds, SealedHub, KIND_KEYOP};
use dowiz_hub::EventKind;

const MOVE: &str = r#"{"item":"salmon","delta":-1,"unit":"portion"}"#;

fn key(seed: u8) -> DeviceKey {
    DeviceKey::from_seed(&[seed; 32])
}
fn orders(sh: &mut SealedHub, key: &DeviceKey, tag: &str, n: u8) {
    (0..n).for_each(|i| drop(sh.append(EventKind::Placed, &format!("{tag}{i}"), MOVE, key, i as i64 + 1).unwrap()));
}
/// An ADMIT of `who` as a record of `actor` at `seq` after `prev`: its id right, no key used.
fn admit_rec(actor: Id, seq: u64, prev: Id, who: &DeviceKey) -> Record {
    let mut payload = vec![0x41u8; 16];
    payload.push(KIND_KEYOP);
    payload.extend_from_slice(&KeyOp::Admit(who.key()).body());
    let mut r = Record { id: [0; 32], prev, actor_pubkey: actor, actor_seq: seq, flags: FLAG_SHA256, payload };
    r.id = HubIds.chained(&r);
    r
}
fn seal(k: &DeviceKey, seq: u64, tip: Id, prev_tip: Id) -> Seal {
    Seal { at: 0, actor_id: k.id(), seq, tip, claimed_ms: 1, prev: None, time: None, alg: ALG_ML_DSA_65, sig: k.sign(&message(&k.id(), seq, &tip, 1, &prev_tip)) }
}
fn verdict(name: &str, bytes: &[u8], roots: &[Id], hole: &str) {
    if let Ok(r) = only(name, bytes, roots) {
        assert!(!r.holds(), "HOLE {hole}: {name} got holds=true: {:?}", r.counters);
    }
}

/// THE FILE AND NO KEY OF THE FILE. A record in the owner's name at seq 0 admits the forger:
/// `admit_more` asks `covered >= seq` and the id is the forger's to compute. The forger's own seal
/// over a record chained on it is what makes `unsealed` reach it. From here F is an admitted key:
/// any order, any REVOKE, any TRUNCATE. The owner never signed anything new.
#[test]
fn a_forger_with_no_key_of_the_file_admits_his_own() {
    let (a, f) = (key(0xA1), key(0xF0));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 3);
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(only("atk2-control", &bytes, &roots).unwrap().holds());
    let tip = s.recs.last().unwrap().id;
    let x = admit_rec(a.id(), 0, tip, &f);
    let y = forged(f.id(), 1, x.id, 0x55);
    let top = seal(&f, 1, y.id, [0; 32]);
    s.keys.push((Key { admit_rec: None, ..f.key() }, Some(x.id)));
    s.recs.push(x);
    s.recs.push(y);
    s.chains.push(vec![top]);
    let f = write(&s);
    verdict("atk2-mint", &f, &roots, "A no-key mint");
    assert_eq!(only("atk2-mint", &f, &roots).unwrap().admitted, 1, "a record the counters call broken admits nobody");
}

/// THE THIEF OF A REVOKED KEY. The REVOKE says after 2; seq 0 is at or below 2, so a new record
/// written after the REVOKE seals clean. `valid_at` is about a number the thief picks.
#[test]
fn a_revoked_key_seals_a_new_record_at_seq_zero() {
    let (a, b) = (key(0xA1), key(0xB2));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 1);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 2).unwrap();
    orders(&mut sh, &b, "b", 2);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 2 }, &a, 3).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(only("atk2-rev-control", &bytes, &roots).unwrap().holds(), "everything B sealed sits at or below the REVOKE");
    let last = chain_of(&mut s, &b.id()).last().unwrap().tip;
    let y = forged(b.id(), 0, last, 0x66);
    let top = seal(&b, 0, y.id, last);
    chain_of(&mut s, &b.id()).push(top);
    s.recs.push(y);
    verdict("atk2-revoked-seq0", &write(&s), &roots, "B revoked key writes on");
}

/// THE SAME DOOR, MADE PERMANENT: at seq 0 the revoked key admits a new key, which no REVOKE names.
#[test]
fn a_revoked_key_admits_a_fresh_key_at_seq_zero() {
    let (a, b, c) = (key(0xA1), key(0xB2), key(0xC3));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 1);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 2).unwrap();
    orders(&mut sh, &b, "b", 2);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 2 }, &a, 3).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    let last = chain_of(&mut s, &b.id()).last().unwrap().tip;
    let x = admit_rec(b.id(), 0, last, &c);
    let top = seal(&b, 0, x.id, last);
    chain_of(&mut s, &b.id()).push(top);
    let y = forged(c.id(), 1, x.id, 0x77);
    let z = forged(c.id(), 2, y.id, 0x78);
    let cc = vec![seal(&c, 1, y.id, [0; 32]), seal(&c, 2, z.id, y.id)];
    s.keys.push((Key { admit_rec: None, ..c.key() }, Some(x.id)));
    s.recs.extend([x, y, z]);
    s.chains.push(cc);
    verdict("atk2-revoked-admits", &write(&s), &roots, "B2 revoked key mints a successor");
}

/// A REVOKE on an off-seq record revokes nobody, and its seal is bad, so `covered` learns nothing.
#[test]
fn a_revoke_at_seq_zero_revokes_nobody() {
    let (a, b) = (key(0xA1), key(0xB2));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 3);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 2).unwrap();
    orders(&mut sh, &b, "b", 2);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 2 }, &a, 3).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    let last = chain_of(&mut s, &b.id()).last().unwrap().tip;
    let mut payload = vec![0x41u8; 16];
    payload.push(KIND_KEYOP);
    payload.extend_from_slice(&KeyOp::Revoke { actor_id: a.id(), after_seq: 1 }.body());
    let mut x = Record { id: [0; 32], prev: last, actor_pubkey: b.id(), actor_seq: 0, flags: FLAG_SHA256, payload };
    x.id = HubIds.chained(&x);
    let top = seal(&b, 0, x.id, last);
    chain_of(&mut s, &b.id()).push(top);
    s.recs.push(x);
    let r = only("atk2-revoke-seq0", &write(&s), &roots).unwrap();
    assert!(!r.holds());
    assert_eq!((r.counters.seals_ok, r.counters.seals_bad, r.counters.broken), (7, 1, 1), "A's seals stand: {:?}", r.counters);
}

/// A device's chain starts at 1: an admitted device whose FIRST record is seq 0 is off its seq.
#[test]
fn a_device_does_not_start_at_seq_zero() {
    let (a, b) = (key(0xA1), key(0xB2));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 1);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 2).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    let tip = s.recs.last().unwrap().id;
    let y = forged(b.id(), 0, tip, 0x66);
    s.chains.push(vec![seal(&b, 0, y.id, [0; 32])]);
    s.recs.push(y);
    verdict("atk2-first-seq0", &write(&s), &roots, "a first record at seq 0");
}
