//! W-MG-ATK5: THE ATTACKS THAT HELD. These are green and stay green; each is one row of the table.
mod atk5_common;
mod atk_forge;

use atk5_common::*;
use atk_forge::read;
use einstore::evlog::doc::keys::KeyOp;
use einstore::evlog::doc::{parse_merge, DocError, Ids, MERGE_MARK};
use dowiz_hub::doc_seal::merge::{MergeError, Merged, Recheck};
use dowiz_hub::doc_seal::{HubIds, SealedHub};
use dowiz_hub::stock::{decode, encode};
use std::collections::BTreeSet;

fn ids(f: &[u8]) -> BTreeSet<[u8; 32]> {
    read(f).recs.iter().map(|r| r.id).collect()
}
fn heads(f: &[u8]) -> Vec<([u8; 32], u64, [u8; 32])> {
    SealedHub::load(f).unwrap().verify().unwrap().heads.iter().map(|h| (h.actor_id, h.seq, h.tip)).collect()
}
fn sigs(f: &[u8]) -> BTreeSet<Vec<u8>> {
    read(f).chains.iter().flatten().map(|s| s.sig.clone()).collect()
}

/// THREE DEVICES, THREE ORDERS: every record and every seal of every file, one set of heads.
#[test]
fn three_devices_in_three_orders_converge_with_nothing_dropped() {
    let s = genesis(&[received("rice", 10)]);
    let (fa, fb, fc) = (offline(&s, 2, &a(), &[served("rice", 1, "a")]), offline(&s, 3, &b(), &[received("tuna", 4)]), offline(&s, 4, &c(), &[received("oil", 2)]));
    let (ab, _) = merged(&fa, &fb, &a(), 5);
    let abc = merged(&ab, &fc, &a(), 6).0;
    let (bc, _) = merged(&fb, &fc, &b(), 7);
    let bca = merged(&bc, &fa, &b(), 8).0;
    let (ca, _) = merged(&fc, &fa, &c(), 9);
    let cab = merged(&ca, &fb, &c(), 10).0;
    let e1 = merged(&merged(&abc, &bca, &a(), 11).0, &cab, &a(), 12).0;
    let e2 = merged(&merged(&bca, &cab, &b(), 13).0, &abc, &b(), 14).0;
    let e3 = merged(&merged(&cab, &abc, &c(), 15).0, &bca, &c(), 16).0;
    assert!(holds(&e1) && holds(&e2) && holds(&e3));
    assert_eq!(ids(&e1), ids(&e2));
    assert_eq!(ids(&e2), ids(&e3));
    let (mut h1, mut h2, mut h3) = (heads(&e1), heads(&e2), heads(&e3));
    h1.sort();
    h2.sort();
    h3.sort();
    assert!(h1 == h2 && h2 == h3, "heads");
    for f in [&fa, &fb, &fc, &abc, &bca, &cab] {
        assert!(ids(f).is_subset(&ids(&e1)) && sigs(f).is_subset(&sigs(&e1)), "a record or a seal dropped");
    }
    assert_eq!((merged(&e1, &e2, &a(), 17).1, merged(&e2, &e3, &b(), 18).1), (Merged::Nothing, Merged::Nothing), "no storm");
}

/// Commuting tails give one level either way round, and recheck reads same in both files.
#[test]
fn commuting_tails_fold_to_one_level_either_way() {
    let s = genesis(&[received("rice", 10), received("salmon", 5)]);
    let (fa, fb) = (offline(&s, 2, &a(), &[served("rice", 1, "a"), served("salmon", 1, "b")]), offline(&s, 3, &b(), &[served("rice", 2, "c"), received("salmon", 1)]));
    let (x, y) = (merged(&fa, &fb, &a(), 4).0, merged(&fb, &fa, &b(), 5).0);
    assert_eq!(fold(&x).unwrap().items(), fold(&y).unwrap().items());
    assert_eq!((audit(&x), audit(&y)), ((vec![Recheck::Same], true), (vec![Recheck::Same], true)));
}

/// Two devices admit one new device, offline: two ADMITs, one KEY, no clash.
#[test]
fn two_admits_of_one_device_merge() {
    let s = genesis(&[received("rice", 1)]);
    let k = dowiz_hub::doc_seal::DeviceKey::from_seed(&[0xD4; 32]);
    let mut x = opened(&s, 2);
    x.commit_keyop(&KeyOp::Admit(k.key()), &a(), 3).unwrap();
    let mut y = opened(&s, 3);
    y.commit_keyop(&KeyOp::Admit(k.key()), &b(), 3).unwrap();
    let fy = trimmed(&y);
    assert!(matches!(x.merge(&fy, &a(), 500).unwrap(), Merged::Sealed { .. }));
    assert!(holds(&trimmed(&x)));
}

/// Every record of one conflict stays, in the words a person reads, both sides named.
#[test]
fn a_recount_keeps_both_movements() {
    let s = genesis(&[received("salmon", 5)]);
    let (fa, fb) = (offline(&s, 2, &a(), &[counted("salmon", 3, "n")]), offline(&s, 3, &b(), &[served("salmon", 1, "o")]));
    let (f, said) = merged(&fa, &fb, &a(), 4);
    assert!(named(&said).unwrap().contains("Recount"), "{said:?}");
    assert_eq!(SealedHub::load(&f).unwrap().stock_events().len(), 3);
}

/// A body whose counts claim more than the bytes: refused by the parse, no allocation.
#[test]
fn counts_in_a_merge_body_are_bounded_by_the_body() {
    for n in [u64::MAX, 1 << 40, 1 << 20] {
        let mut b = MERGE_MARK.to_vec();
        b.extend_from_slice(&n.to_le_bytes());
        assert!(parse_merge(&b).is_none());
        let mut b = MERGE_MARK.to_vec();
        b.extend_from_slice(&[0; 16]);
        b.push(3);
        b.extend_from_slice(&[0; 48]);
        b.extend_from_slice(&n.to_le_bytes());
        assert!(parse_merge(&b).is_none());
    }
}

/// Item names a kitchen can type survive the line, so no honest movement reads as nothing.
#[test]
fn item_names_round_trip_the_stock_line() {
    for item in ["лосось", "a\"b", "a\\b", "a\nb", "a\tb", "{\"k\":\"x\"}", "", " ", "🍣"] {
        let ev = served(item, 1, item);
        assert_eq!(decode(&encode(&ev)).as_ref(), Some(&ev), "{item:?}");
    }
}

/// A revoked merger: refused by name, the file stands, on either order of arguments.
#[test]
fn the_ours_side_is_judged_too() {
    let s = genesis(&[received("rice", 1)]);
    let (fa, fb) = (offline(&s, 2, &a(), &[received("rice", 1)]), offline(&s, 3, &b(), &[received("rice", 1)]));
    let mut damaged = read(&fa);
    let top = damaged.recs.len() - 1;
    damaged.recs[top].payload[20] ^= 1;
    let mut sh = opened(&atk_forge::write(&damaged), 4);
    assert_eq!(sh.merge(&fb, &a(), 500).unwrap_err(), MergeError::Refused(DocError::NotWhole { ours: true }));
    assert!(HubIds.body(0, &[]).is_empty());
}
