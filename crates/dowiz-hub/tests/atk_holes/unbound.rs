//! What `holds` says nothing about, though a reader takes it for covered.
use super::*;
use bebop_store::evlog::doc::keys::{revoke_body, Key, KeyOp};
use bebop_store::evlog::doc::FLAG_TREE_IS_FOLD;
use dowiz_hub::Hub;

/// HOLE 7. `ids.body` skipped ONE byte of any kind, so `effective` read a declaration out of an
/// ORDER. `'b'` is 98: a 98-byte order id spelling the rest of the mark was a REVOKE, signed by
/// the device that took the order. (A FORGOTTEN now lists raw digests, no text; the first form
/// of this test spelled one.) Key X has an all-text id, so its name fits in an order id; it
/// never seals, the point is the fold: unruled, the order revokes it and `keys_disagree` is 1.
#[test]
fn hole_7_an_order_id_reads_as_a_declaration() {
    let a = key(0xA1);
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    let x = Key { actor_id: [b'x'; 32], ..key(0xC3).key() };
    sh.commit_keyop(&KeyOp::Admit(x), &a, 1).unwrap();
    let mut body = revoke_body(&[b'x'; 32], 0x7878_7878_7878_7878);
    body.resize(99, b'x');
    let id = String::from_utf8(body[1..].to_vec()).unwrap();
    sh.append(EventKind::Placed, &id, MOVE, &a, 3).unwrap();
    let r = only("h7-order-spells-a-revoke", &sh.to_bytes_trimmed(), &[a.id()]).unwrap();
    assert_eq!(r.counters.keys_disagree, 0, "an order declares nothing: {:?}", r.counters);
    assert!(r.holds(), "{:?}", r.counters);
}

/// HOLE 8. A DOC's TREE is read by every reader's `get` and by nothing in `verify`: any object at
/// cell 2, any FLAGS, holds true. Nothing writes a TREE into a DOC today, so the rule is cheap
/// now: a file whose data is in a tree no signature covers must not hold.
#[test]
fn hole_8_a_tree_no_signature_covers() {
    let a = key(0xA1);
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 2);
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    s.tree = Some(vec![4, 0xBAD, 0xBAD]);
    s.flags |= FLAG_TREE_IS_FOLD;
    must_not_hold("h8-tree-and-flag", &write(&s), &roots);
}

/// HOLE 9. Under a seal over many records (`seal_hub`) a seal binds the tip and `prev`, and the
/// id binds `prev ‖ payload`: not the record's `actor` cell. Who wrote a pre-seal record is free.
#[test]
fn hole_9_the_actor_cell_under_one_seal_is_free() {
    let a = key(0xA1);
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    (1..=4u64).for_each(|i| drop(hub.append(EventKind::Placed, &format!("o{i}"), MOVE, i, [0u8; 32]).unwrap()));
    let mut sh = SealedHub::seal_hub(&hub.to_bytes_trimmed(), &a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 1);
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let s = round_trip(&bytes, &roots);
    let mut x = s.clone();
    x.recs[0].actor_pubkey = [0x77; 32];
    must_not_hold("h9-actor-relabelled", &write(&x), &roots);
    let mut x = s.clone();
    x.recs[0].actor_seq = 77;
    must_not_hold("h9-seq-moved", &write(&x), &roots);
}
