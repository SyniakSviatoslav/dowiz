//! One session in memory: one MERGE, a break at every message, each refusal by name.

use super::*;
use crate::toys::*;
use crate::{frames, Assembler};
use bebop_log::doc::keys::Key;
use bebop_log::doc::{forgotten_body, KeyOp};

/// Carry messages until nobody has one, starting with `first`'s hello. `carry` is what the wire does
/// to each message. Returns how many messages moved.
fn meet_with(i: &mut Peer<'_>, r: &mut Peer<'_>, carry: &dyn Fn(Vec<u8>) -> Vec<u8>) -> usize {
    let mut msg = i.hello().unwrap();
    let mut n = 0;
    loop {
        n += 1;
        assert!(n < 64, "a session that does not end");
        let wire = carry(msg);
        let step = if n % 2 == 1 { r.on(&wire).unwrap() } else { i.on(&wire).unwrap() };
        match step.send {
            Some(m) => msg = m,
            None => return n,
        }
    }
}

fn meet(i: &mut Peer<'_>, r: &mut Peer<'_>) -> usize {
    meet_with(i, r, &|m| m)
}

fn two_wrote<'a>(g: &[u8], a: &'a Toy, b: &'a Toy) -> (Peer<'a>, Peer<'a>) {
    let (mut pa, mut pb) = (peer(g, a), peer(g, b));
    write(&mut pa.store, a, b"from a", None);
    write(&mut pb.store, b, b"from b", None);
    (pa, pb)
}

/// §3's seven steps: FRONTIER, TAIL, TAIL, ACK, and exactly one MERGE, on the initiator.
#[test]
fn both_wrote_one_session_one_merge() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = two_wrote(&g, &a, &b);
    assert_eq!(meet(&mut pa, &mut pb), 4);
    let (ra, rb) = (report(&pa.store), report(&pb.store));
    assert!(ra.holds() && rb.holds(), "{:?} {:?}", ra.counters, rb.counters);
    assert_eq!((ra.counters.merges, rb.counters.merges), (1, 1), "the same one MERGE on both");
    assert_eq!(pa.front().unwrap(), pb.front().unwrap());
}

#[test]
fn again_writes_nothing() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = two_wrote(&g, &a, &b);
    meet(&mut pa, &mut pb);
    let before = pa.store.to_bytes();
    assert_eq!(meet(&mut pa, &mut pb), 2, "FRONTIER, ACK");
    assert_eq!(pa.store.to_bytes(), before);
}

/// One side wrote: one history, no MERGE, either side first.
#[test]
fn one_side_wrote_needs_no_merge() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    for initiator_wrote in [false, true] {
        let (mut pa, mut pb) = (peer(&g, &a), peer(&g, &b));
        write(&mut pb.store, &b, b"only b", None);
        if initiator_wrote {
            meet(&mut pb, &mut pa);
        } else {
            meet(&mut pa, &mut pb);
        }
        assert_eq!(report(&pa.store).counters.merges, 0);
        assert_eq!(pa.front().unwrap(), pb.front().unwrap());
    }
}

/// THE STORM: both said hello at once, so both tails cross and each side glues. Two MERGEs at most,
/// and a tail of nothing but MERGEs owes none.
#[test]
fn two_hellos_at_once_stop_at_two_merges() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = two_wrote(&g, &a, &b);
    let (ha, hb) = (pa.hello().unwrap(), pb.hello().unwrap());
    let (ta, tb) = (pa.on(&hb).unwrap().send.unwrap(), pb.on(&ha).unwrap().send.unwrap());
    let (sa, sb) = (pa.on(&tb).unwrap(), pb.on(&ta).unwrap());
    assert!(sa.adopted.merged && sb.adopted.merged);
    meet(&mut pa, &mut pb);
    assert!(report(&pa.store).counters.merges <= 2);
    assert!(report(&pa.store).holds());
    assert_eq!(pa.front().unwrap(), pb.front().unwrap());
    let before = pb.store.to_bytes();
    meet(&mut pb, &mut pa);
    assert_eq!(pb.store.to_bytes(), before, "no more");
}

/// A BREAK AT EVERY MESSAGE: drop the session after `cut` messages, say hello again, and the end is
/// the same document with at most one MERGE owed for each glue that finished.
#[test]
fn a_break_at_any_message_recovers_by_hello() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    for cut in 1..=3usize {
        let (mut pa, mut pb) = two_wrote(&g, &a, &b);
        let mut msg = pa.hello().unwrap();
        for n in 1..=cut {
            let step = if n % 2 == 1 { pb.on(&msg).unwrap() } else { pa.on(&msg).unwrap() };
            msg = step.send.unwrap();
        }
        meet(&mut pa, &mut pb);
        assert_eq!(pa.front().unwrap(), pb.front().unwrap(), "cut after {cut}");
        assert_eq!(report(&pa.store).counters.merges, 1, "cut after {cut}");
        assert!(report(&pa.store).holds() && report(&pb.store).holds(), "cut after {cut}");
    }
}

/// The same session over a data channel's small frames: the session cannot tell.
#[test]
fn small_frames_change_nothing() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = two_wrote(&g, &a, &b);
    let (mut qa, mut qb) = two_wrote(&g, &a, &b);
    let n = meet_with(&mut pa, &mut pb, &|m| {
        let mut asm = Assembler::new(1 << 23);
        let fs = frames(&m, 256, 0, &ToyIds).unwrap();
        let mut out: Vec<Vec<u8>> = Vec::new();
        for f in &fs {
            out.extend(asm.feed(f, &ToyIds).unwrap());
        }
        out.pop().unwrap()
    });
    assert_eq!(n, meet(&mut qa, &mut qb));
    assert_eq!(pa.store.to_bytes(), qa.store.to_bytes());
}

/// A TAIL PAST A MESSAGE arrives in parts, each a position; nothing is lost, nothing is left over.
#[test]
fn a_tail_past_the_budget_arrives_in_parts() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = (peer(&g, &a), peer(&g, &b));
    for i in 0..7 {
        write(&mut pb.store, &b, format!("b{i}").as_bytes(), None);
    }
    write(&mut pa.store, &a, b"a", None);
    let tight = Budget { seals: 2, ..Budget::default() };
    (pa.budget, pb.budget) = (tight, tight);
    meet(&mut pa, &mut pb);
    assert_eq!(pa.front().unwrap(), pb.front().unwrap());
    let (ra, rb) = (report(&pa.store), report(&pb.store));
    assert!(ra.holds() && rb.holds(), "{:?}", ra.counters);
    assert!(ra.counters.merges >= 1, "the cost is named in `doc/clip.rs`: {}", ra.counters.merges);
}

/// A tail built for a place we never held gets our FRONTIER, and the next one lands.
#[test]
fn a_tail_for_a_place_we_never_held_is_answered_with_where_we_stand() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = (peer(&g, &a), peer(&g, &b));
    write(&mut pb.store, &b, b"one", None);
    let base = pb.front().unwrap();
    write(&mut pb.store, &b, b"two", None);
    write(&mut pb.store, &b, b"three", None);
    write(&mut pa.store, &b, b"one", None);
    let mut t = Doc::tail_of(&pb.store, &base, &ToyIds).unwrap();
    t.recs.remove(0);
    let step = pa.on(&crate::encode_tail(&t, &pa.budget).unwrap()).unwrap();
    assert!(step.refused.is_none());
    let Ok(crate::Msg::Frontier(f)) = crate::decode(&step.send.unwrap(), &pa.budget, &ToyIds) else { panic!("not a FRONTIER") };
    assert_eq!(f, pa.front().unwrap());
}

/// `refused`, the peer's `need`, and the file as it was: each named refusal, one positive twin above.
fn refused_by_name(mut r: Peer<'_>, mut i: Peer<'_>, want: DocError, name: &str) {
    let before = r.store.to_bytes();
    let step = r.on(&i.hello().unwrap()).unwrap();
    let step = match step.refused {
        Some(_) => step,
        None => {
            let t = i.on(&step.send.unwrap()).unwrap();
            r.on(&t.send.unwrap()).unwrap()
        }
    };
    let got = step.refused.unwrap();
    assert_eq!(core::mem::discriminant(&got), core::mem::discriminant(&want), "{got:?}");
    assert_eq!(r.store.to_bytes(), before, "a refusal writes nothing");
    let told = i.on(&step.send.unwrap()).unwrap();
    assert_eq!(told.need, Some(Need::Refused { name: name.to_string() }));
}

#[test]
fn a_fork_is_refused_by_name() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = (peer(&g, &a), peer(&g, &b));
    write(&mut pa.store, &b, b"this", None);
    write(&mut pb.store, &b, b"that", None);
    refused_by_name(pa, pb, DocError::Fork { actor: B, seq: 0, ours: [0; 32], theirs: [0; 32] }, "Fork");
}

#[test]
fn another_document_is_refused_by_name() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let mut st = Store::create_bytes(1 << 20).unwrap();
    Doc::init_bytes(&mut st, 0, &[key_of(C)]).unwrap();
    let c = toy(C);
    write(&mut st, &c, b"elsewhere", None);
    refused_by_name(peer(&g, &a), peer(&st.to_bytes(), &c), DocError::ForeignRoot, "ForeignRoot");
}

/// ERASURE DOMINATES (§12): a peer whose FORGOTTEN list differs gets no record of ours at all.
#[test]
fn past_an_erasure_difference_not_one_record_leaves() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = (peer(&g, &a), peer(&g, &b));
    let erased = bebop_log::doc::Erased { id: [1; 32], digest: [2; 32] };
    let at = Store::from_bytes(&g);
    write(&mut pa.store, &a, &forgotten_body(&[9; 32], &[erased]), None);
    write(&mut pb.store, &b, b"plaintext", None);
    assert_ne!(pa.front().unwrap().forgotten_hash, Doc::front(&at, &ToyIds).unwrap().forgotten_hash);
    let step = pb.on(&pa.hello().unwrap()).unwrap();
    assert_eq!(step.refused, Some(DocError::ForgetOneSide));
    assert!(!step.send.as_ref().unwrap().windows(9).any(|w| w == b"plaintext"), "no bytes of it left");
}

#[test]
fn a_cut_on_one_side_asks_for_the_archive() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, pb) = (peer(&g, &a), peer(&g, &b));
    let mut cut = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&pa.store, &mut cut, 1, &ToyIds).unwrap();
    write(&mut cut, &a, &seg.truncate_body(), None);
    pa.store = cut;
    let step = pa.on(&pb.hello().unwrap()).unwrap();
    assert_eq!(step.refused, Some(DocError::CutOneSide { root: seg.root }));
    let Ok(crate::Msg::Need(Need::Archive { seg_root })) = crate::decode(&step.send.unwrap(), &pa.budget, &ToyIds) else { panic!("no archive asked") };
    assert_eq!(seg_root, seg.root);
}

/// Bytes that are no message are `Err`, and answer nothing: a stranger gets no reply to probe.
#[test]
fn bytes_that_are_no_message_answer_nothing() {
    let a = toy(A);
    let mut p = peer(&genesis(&[&a]), &a);
    assert_eq!(p.on(b"hello").err(), Some(SyncError::Mark("message")));
}

/// A MERGE's conflicts come from the device, and `Obstructed` follows from them.
#[test]
fn a_named_conflict_is_sealed_as_obstructed() {
    let (mut a, b) = (toy(A), toy(B));
    a.named = vec![b"the last salmon".to_vec()];
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = two_wrote(&g, &a, &b);
    meet(&mut pa, &mut pb);
    let recs = bebop_log::EvLog::walk_until_at(&pa.store, Doc::open(&pa.store).unwrap().log, |_| false);
    let view = recs.iter().find_map(|r| bebop_log::doc::parse_merge(&r.payload)).unwrap();
    assert_eq!((view.state, view.conflicts), (STATE_OBSTRUCTED, vec![b"the last salmon".to_vec()]));
}

/// A revoked device's keys are the file's business: an ADMIT and a REVOKE travel by the same door.
#[test]
fn a_key_change_travels_in_the_tail() {
    let (a, b, c) = (toy(A), toy(B), toy(C));
    let g = genesis(&[&a, &b]);
    let (mut pa, mut pb) = (peer(&g, &a), peer(&g, &b));
    let op = KeyOp::Admit(key_of(C));
    write(&mut pa.store, &a, &op.body(), Some(&op));
    write(&mut pa.store, &c, b"from c", None);
    let op = KeyOp::Revoke { actor_id: C, after_seq: 1, tip: Doc::tip_at(&pa.store, &C, 1).unwrap() };
    write(&mut pa.store, &a, &op.body(), Some(&op));
    meet(&mut pa, &mut pb);
    let keys: Vec<Key> = bebop_log::doc::keys::read_keys(&pb.store, Doc::open(&pb.store).unwrap().keys);
    assert_eq!(keys.iter().find(|k| k.actor_id == C).map(|k| k.revoked_at), Some(1));
    assert!(report(&pb.store).holds());
}

/// AN IMAGE THAT TURNS OUT TOO SMALL IS RETRIED LARGER: a conflict list past the first guess.
#[test]
fn a_merge_past_the_first_image_retries_larger_and_up_to_the_cap() {
    let (mut a, b) = (toy(A), toy(B));
    a.named = vec![vec![7u8; 3 << 20]];
    let g = genesis(&[&a, &b]);
    let (mut pa, pb) = two_wrote(&g, &a, &b);
    let t = Doc::tail_of(&pb.store, &pa.front().unwrap(), &ToyIds).unwrap();
    let mut capped = peer(&pa.store.to_bytes(), &a);
    capped.budget.image = 1 << 20;
    let before = capped.store.to_bytes();
    assert!(matches!(capped.adopt(&t), Err(SyncError::Doc(DocError::Store(StoreError::ArenaFull { .. })))), "the cap is named");
    assert_eq!(capped.store.to_bytes(), before, "a refusal writes nothing");
    assert!(pa.adopt(&t).unwrap().merged);
    assert!(report(&pa.store).holds());
}

/// `Step` tells the caller what moved; a wire-level repeat moves nothing.
#[test]
fn the_same_tail_twice_writes_nothing() {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let (mut pa, pb) = (peer(&g, &a), { let mut p = peer(&g, &b); write(&mut p.store, &b, b"one", None); p });
    let m = crate::encode_tail(&Doc::tail_of(&pb.store, &pa.front().unwrap(), &ToyIds).unwrap(), &pa.budget).unwrap();
    assert_eq!(pa.on(&m).unwrap().adopted.records, 1);
    let before = pa.store.to_bytes();
    assert_eq!(pa.on(&m).unwrap().adopted, Adopted::default());
    assert_eq!(pa.store.to_bytes(), before);
}
