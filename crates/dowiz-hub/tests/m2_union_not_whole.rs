//! W-MG-M2's last guard with real ML-DSA-65, through `SealedHub::merge`: each file holds and the two
//! together do not. `keyed` IS NOT MONOTONE (`doc/merge.rs`): an actor with no KEY row is not judged,
//! so a record of it sits in a file that holds; the other side admits that actor, and in the union
//! its KEY row judges that record. Every check before the write passes, and only the verify of the
//! union sees it. The stranger's record is written with `atk_forge`, no private key used.
mod atk_forge;

use atk_forge::*;
use einstore::evlog::doc::keys::KeyOp;
use einstore::evlog::doc::DocError;
use dowiz_hub::doc_seal::merge::{refusal_name, MergeError, Merged};
use dowiz_hub::doc_seal::{DeviceKey, SealedHub};
use dowiz_hub::stock::StockEvent;

fn received(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.to_string(), qty }
}

fn opened(bytes: &[u8], seed: u8) -> SealedHub {
    let mut sh = SealedHub::load(bytes).unwrap();
    sh.set_entropy([seed; 32]);
    sh
}

fn holds(bytes: &[u8]) -> bool {
    SealedHub::load(bytes).unwrap().verify().unwrap().holds()
}

/// The shared past: A receives, admits B, the devices part. Then, on B's side only, a stranger's
/// record at `seq` that B's next record chains onto, so B's seal reaches it.
fn two_sides(a: &DeviceKey, b: &DeviceKey, stranger: &DeviceKey, seq: u64) -> (Vec<u8>, Vec<u8>) {
    let mut sh = SealedHub::create(a).unwrap();
    sh.set_entropy([1; 32]);
    sh.sync_stock(&[received("rice", 10)], a, 1).unwrap();
    sh.commit_keyop(&KeyOp::Admit(b.key()), a, 2).unwrap();
    let split = sh.to_bytes_trimmed();
    let mut s = read(&split);
    let tip = s.recs.last().unwrap().id;
    s.recs.push(forged(stranger.key().actor_id, seq, tip, 0x77));
    let mut theirs = opened(&write(&s), 3);
    theirs.sync_stock(&[received("tuna", 4)], b, 100).unwrap();
    (split, theirs.to_bytes_trimmed())
}

/// Seq 7 is off the stranger's chain once it is judged; seq 1 is above any seal of its key.
#[test]
fn a_key_on_one_side_judges_the_other_sides_stranger_and_the_union_is_refused() {
    let (a, b, c) = (DeviceKey::from_seed(&[0xA1; 32]), DeviceKey::from_seed(&[0xB2; 32]), DeviceKey::from_seed(&[0xC3; 32]));
    for seq in [7u64, 1] {
        let (split, theirs) = two_sides(&a, &b, &c, seq);
        let mut ours = opened(&split, 2);
        ours.commit_keyop(&KeyOp::Admit(c.key()), &a, 100).unwrap();
        assert!(holds(&ours.to_bytes_trimmed()) && holds(&theirs), "seq {seq}: each side holds");
        let before = ours.to_bytes_trimmed();
        let e = ours.merge(&theirs, &a, 500).expect_err("the union does not hold");
        assert_eq!(e, MergeError::Refused(DocError::UnionNotWhole), "seq {seq}");
        assert_eq!(refusal_name(&e), "UNION_NOT_WHOLE: each file holds and the two together do not");
        assert_eq!(ours.to_bytes_trimmed(), before, "seq {seq}: a refusal writes nothing");
    }
}

/// THE CONTROL: the same two files with the stranger left unadmitted. Nothing else differs, and they
/// merge: the record alone is not what the union refuses.
#[test]
fn the_same_stranger_with_no_key_on_either_side_merges() {
    let (a, b, c) = (DeviceKey::from_seed(&[0xA1; 32]), DeviceKey::from_seed(&[0xB2; 32]), DeviceKey::from_seed(&[0xC3; 32]));
    for seq in [7u64, 1] {
        let (split, theirs) = two_sides(&a, &b, &c, seq);
        let mut ours = opened(&split, 2);
        ours.sync_stock(&[received("salt", 2)], &a, 100).unwrap();
        assert!(matches!(ours.merge(&theirs, &a, 500), Ok(Merged::Sealed { .. })), "seq {seq}");
        assert!(holds(&ours.to_bytes_trimmed()), "seq {seq}");
    }
}
