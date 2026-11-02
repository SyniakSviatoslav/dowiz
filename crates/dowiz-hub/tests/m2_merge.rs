//! W-MG-M2: two offline devices, one genesis, no server, real ML-DSA-65. The honest merge first, then
//! every door an attacker with the file (or with a key) tries, each to its named end. Files are
//! hand-edited only through `atk_forge`, with no private key, unless a test says it signs.
mod atk_forge;

use atk_forge::*;
use einstore::evlog::doc::keys::KeyOp;
use einstore::evlog::doc::{self, merge_body, DocError, Ids, MergeView, Report, Row, Trust, MERGE_MARK, STATE_COMMUTES, STATE_OBSTRUCTED};
use einstore::Store;
use dowiz_hub::doc_seal::merge::{merge_report, MergeError, Merged, Recheck};
use dowiz_hub::doc_seal::{DeviceKey, HubIds, MlDsa65, SealedHub};
use dowiz_hub::stock::StockEvent;
use dowiz_hub::{EventKind, HubError};

fn keys() -> (DeviceKey, DeviceKey) {
    (DeviceKey::from_seed(&[0xA1; 32]), DeviceKey::from_seed(&[0xB2; 32]))
}
fn received(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.to_string(), qty }
}
fn served(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Served { item: item.to_string(), qty, order_id: order.to_string() }
}
fn opened(bytes: &[u8], seed: u8) -> SealedHub {
    let mut sh = SealedHub::load(bytes).unwrap();
    sh.set_entropy([seed; 32]);
    sh
}
fn trimmed(sh: &SealedHub) -> Vec<u8> {
    sh.to_bytes_trimmed()
}

/// The shared past: A receives one salmon and ten rice, admits B. Then the devices part.
fn genesis(a: &DeviceKey, b: &DeviceKey) -> Vec<u8> {
    let mut sh = SealedHub::create(a).unwrap();
    sh.set_entropy([1; 32]);
    sh.sync_stock(&[received("salmon", 1), received("rice", 10)], a, 1).unwrap();
    sh.commit_keyop(&KeyOp::Admit(b.key()), a, 2).unwrap();
    trimmed(&sh)
}
/// A file after `key` synced `moves` offline.
fn offline(split: &[u8], seed: u8, key: &DeviceKey, moves: &[StockEvent]) -> Vec<u8> {
    let mut sh = opened(split, seed);
    sh.sync_stock(moves, key, 100).unwrap();
    trimmed(&sh)
}
/// Two copies that commute, A a salmon-free side, B a rice-free one.
fn calm(a: &DeviceKey, b: &DeviceKey) -> (Vec<u8>, Vec<u8>) {
    let s = genesis(a, b);
    (offline(&s, 2, a, &[served("rice", 2, "o1"), served("rice", 1, "o2")]), offline(&s, 3, b, &[received("tuna", 4)]))
}
/// The last salmon sold on both.
fn oversold(a: &DeviceKey, b: &DeviceKey) -> (Vec<u8>, Vec<u8>) {
    let s = genesis(a, b);
    (offline(&s, 2, a, &[served("salmon", 1, "o1")]), offline(&s, 3, b, &[served("salmon", 1, "o2")]))
}
fn report(bytes: &[u8]) -> Report {
    SealedHub::load(bytes).unwrap().verify().unwrap()
}
fn refused(sh: &mut SealedHub, theirs: &[u8], key: &DeviceKey) -> DocError {
    let before = trimmed(sh);
    match sh.merge(theirs, key, 500) {
        Err(MergeError::Refused(e)) => {
            assert_eq!(trimmed(sh), before, "a refusal writes nothing");
            e
        }
        other => panic!("not refused: {other:?}"),
    }
}
fn ids_of(bytes: &[u8]) -> std::collections::BTreeSet<[u8; 32]> {
    read(bytes).recs.iter().map(|r| r.id).collect()
}
fn heads_of(bytes: &[u8]) -> Vec<([u8; 32], u64, [u8; 32])> {
    report(bytes).heads.iter().map(|h| (h.actor_id, h.seq, h.tip)).collect()
}

#[test]
fn two_offline_devices_merge_and_every_seal_survives() {
    let (a, b) = keys();
    let (fa, fb) = calm(&a, &b);
    let (ca, cb) = (report(&fa).counters, report(&fb).counters);
    let mut sh = opened(&fa, 4);
    assert_eq!(sh.merge(&fb, &a, 500).unwrap(), Merged::Sealed { records: 1, conflicts: Vec::new() });
    let merged = trimmed(&sh);
    let r = report(&merged);
    let c = r.counters;
    assert!(r.holds(), "{c:?}");
    assert_eq!((c.records, c.merges, c.merges_bad, c.forks, c.broken, c.seals_bad, c.unsealed), (ca.records + 1 + 1, 1, 0, 0, 0, 0, 0), "{c:?}");
    assert_eq!(c.seals_ok, ca.seals_ok + 1 + 1, "theirs carried, one for the MERGE, none re-signed: {ca:?} {cb:?} {c:?}");
    assert_eq!(heads_of(&merged).len(), 2);
    let keep = read(&merged);
    for side in [&fa, &fb] {
        for s in read(side).chains.iter().flatten() {
            assert!(keep.chains.iter().flatten().any(|x| x.sig == s.sig), "the seal of seq {} survived byte for byte", s.seq);
        }
    }
    let lines = sh.merge_lines();
    assert_eq!(lines.len(), 1);
    assert_eq!((lines[0].seq, lines[0].recheck), (report(&fa).heads.iter().find(|h| h.actor_id == a.id()).unwrap().seq + 1, Recheck::Same));
}

/// A and B each merge the other from the same two files: two MERGEs, then one more turn each takes
/// the other's MERGE in without a third. Same records, same heads; nothing storms.
#[test]
fn two_merges_meet_and_end() {
    let (a, b) = keys();
    let (fa, fb) = calm(&a, &b);
    let mut x = opened(&fa, 4);
    x.merge(&fb, &a, 500).unwrap();
    let mut y = opened(&fb, 5);
    y.merge(&fa, &b, 501).unwrap();
    let (mx, my) = (trimmed(&x), trimmed(&y));
    assert_eq!(x.merge(&my, &a, 502).unwrap(), Merged::Adopted { records: 1 });
    assert_eq!(y.merge(&mx, &b, 503).unwrap(), Merged::Adopted { records: 1 });
    let (ex, ey) = (trimmed(&x), trimmed(&y));
    assert_eq!(ids_of(&ex), ids_of(&ey));
    let (hx, hy) = (heads_of(&ex), heads_of(&ey));
    assert_eq!(hx, hy);
    assert!(report(&ex).holds() && report(&ey).holds());
    assert_eq!((report(&ex).counters.merges, report(&ey).counters.merges), (2, 2));
    assert_eq!(x.merge(&ey, &a, 504).unwrap(), Merged::Nothing);
    assert_eq!(y.merge(&ex, &b, 505).unwrap(), Merged::Nothing);
}

/// The last salmon: named, by item, and NOT resolved: both sales stay in the file.
#[test]
fn the_conflict_is_named_and_nothing_is_dropped() {
    let (a, b) = keys();
    let (fa, fb) = oversold(&a, &b);
    let mut sh = opened(&fa, 4);
    let Merged::Sealed { conflicts, .. } = sh.merge(&fb, &a, 500).unwrap() else { panic!("no MERGE") };
    assert_eq!(conflicts.len(), 1, "{conflicts:?}");
    let merged = trimmed(&sh);
    assert!(report(&merged).holds());
    let line = &sh.merge_lines()[0];
    let v = line.view.as_ref().unwrap();
    assert_eq!((v.state, line.recheck), (STATE_OBSTRUCTED, Recheck::Same));
    let text = String::from_utf8(v.conflicts[0].clone()).unwrap();
    assert!(text.contains(r#""kind":"oversold""#) && text.contains(r#""item":"salmon""#), "{text}");
    let moved: Vec<_> = sh.stock_events().into_iter().filter(|e| matches!(e, StockEvent::Served { .. })).collect();
    assert_eq!(moved.len(), 2, "both sales stay; a person decides");
    let (words, clean) = merge_report(&sh, true);
    assert!(clean && words.iter().any(|l| l.starts_with("conflict ") && l.contains("salmon: на двох пристроях")), "{words:?}");
}

#[test]
fn a_fast_forward_writes_no_merge() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    let fb = offline(&s, 3, &b, &[received("tuna", 4)]);
    let mut sh = opened(&s, 4);
    assert_eq!(sh.merge(&fb, &a, 500).unwrap(), Merged::Adopted { records: 1 });
    assert_eq!(report(&trimmed(&sh)).counters.merges, 0);
    assert!(report(&trimmed(&sh)).holds());
    let mut other = opened(&fb, 5);
    assert_eq!(other.merge(&s, &b, 501).unwrap(), Merged::Nothing, "the older copy has nothing");
}

// ── THE ATTACKS ──────────────────────────────────────────────────────────────────────────────

fn sealed_merge(a: &DeviceKey, b: &DeviceKey) -> Vec<u8> {
    let (fa, fb) = oversold(a, b);
    let mut sh = opened(&fa, 4);
    sh.merge(&fb, a, 500).unwrap();
    trimmed(&sh)
}

fn honest(a: &DeviceKey) -> Vec<[u8; 32]> {
    vec![a.id()]
}

/// ONE BIT OF THE MERGE, no key: the conflict list edited. The id stops hashing.
#[test]
fn a_forged_merge_body_does_not_hold() {
    let (a, b) = keys();
    let m = sealed_merge(&a, &b);
    let roots = honest(&a);
    let mut s = round_trip(&m, &roots);
    let p = pos_of(&s, MERGE_MARK);
    let last = s.recs[p].payload.len() - 1;
    s.recs[p].payload[last] ^= 1;
    let r = only("m2-merge-bit", &write(&s), &roots).unwrap();
    assert!(!r.holds() && r.counters.broken == 1 && r.counters.merges == 0, "{:?}", r.counters);
}

/// A MERGE WITH A RIGHT ID AND NO SEAL, no key: it is one more record nobody signed.
#[test]
fn an_unsealed_merge_declares_nothing() {
    let (a, b) = keys();
    let (fa, fb) = calm(&a, &b);
    let mut sh = opened(&fa, 4);
    sh.merge(&fb, &a, 500).unwrap();
    let m = trimmed(&sh);
    let roots = honest(&a);
    let mut s = round_trip(&m, &roots);
    let top = s.chains.iter().flatten().filter(|x| x.actor_id == a.id()).last().unwrap().clone();
    let mut rec = forged(a.id(), top.seq + 1, top.tip, 0x31);
    rec.payload.truncate(16);
    rec.payload.push(dowiz_hub::doc_seal::KIND_DECL);
    rec.payload.extend_from_slice(&merge_body(&MergeView { ours: Vec::new(), theirs: Vec::new(), state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() }));
    rec.id = HubIds.chained(&rec);
    s.recs.push(rec);
    let r = only("m2-unsealed-merge", &write(&s), &roots).unwrap();
    assert!(!r.holds() && r.counters.unsealed == 1 && r.counters.merges == 1, "{:?}", r.counters);
}

/// A REAL KEY SIGNS A LIE: it claims to have seen a record no file here holds.
#[test]
fn a_merge_over_a_record_the_file_lacks_is_merges_bad() {
    let (a, b) = keys();
    let (fa, _) = calm(&a, &b);
    let ghost = Row { actor_id: b.id(), seq: 9, tip: [0x77; 32] };
    let rows: Vec<Row> = report(&fa).heads.iter().map(|h| Row { actor_id: h.actor_id, seq: h.seq, tip: h.tip }).collect();
    let mut sh = opened(&fa, 4);
    sh.declare(&merge_body(&MergeView { ours: rows.clone(), theirs: vec![ghost], state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() }), &a, 500).unwrap();
    let r = report(&trimmed(&sh));
    assert!(!r.holds() && r.counters.merges == 1 && r.counters.merges_bad == 1 && r.counters.seals_bad == 0, "{:?}", r.counters);
    // The same lie on the wire of a merge: their file does not hold, so nothing is glued.
    let mut ours = opened(&fa, 6);
    let lie = trimmed(&sh);
    assert_eq!(refused(&mut ours, &lie, &a), DocError::NotWhole { ours: false });
}

/// A body with the mark and not a MERGE: a key holder cannot hide behind a parse.
#[test]
fn a_merge_that_does_not_parse_does_not_hold() {
    let (a, b) = keys();
    let mut sh = opened(&genesis(&a, &b), 4);
    sh.declare(MERGE_MARK, &a, 500).unwrap();
    let r = report(&trimmed(&sh));
    assert!(!r.holds() && r.counters.merges_bad == 1, "{:?}", r.counters);
    assert_eq!(sh.merge_lines()[0].recheck, Recheck::Differs);
}

/// A MERGED TAIL DROPPED WITH ITS SEALS AND ITS ROW, no key: alone that is a shorter true file
/// (`k3_union` pins it). With the MERGE still there its row names the hole.
#[test]
fn a_merged_tail_dropped_is_named_by_the_merge() {
    let (a, b) = keys();
    let m = sealed_merge(&a, &b);
    let roots = honest(&a);
    let mut s = round_trip(&m, &roots);
    let seq_of_b = s.recs.iter().filter(|r| r.actor_pubkey == b.id()).count();
    assert_eq!(seq_of_b, 1);
    s.recs.retain(|r| r.actor_pubkey != b.id());
    s.chains.retain(|c| c[0].actor_id != b.id());
    let r = only("m2-tail-dropped", &write(&s), &roots).unwrap();
    assert!(!r.holds() && r.counters.merges_bad == 1 && r.counters.seals_bad == 0 && r.counters.unsealed == 0, "{:?}", r.counters);
}

/// A KEY HOLDER HIDES THE CONFLICT: everything `verify` judges stands, and the rows fix both
/// sides, so the same fold gives the conflict the MERGE left out.
#[test]
fn a_signed_merge_that_hides_the_conflict_reads_differs() {
    let (a, b) = keys();
    let (fa, fb) = oversold(&a, &b);
    let mut sh = opened(&fa, 4);
    sh.merge(&fb, &a, 500).unwrap();
    let honest_view = sh.merge_lines()[0].view.clone().unwrap();
    let mut lie = honest_view.clone();
    lie.conflicts.clear();
    lie.state = STATE_COMMUTES;
    sh.declare(&merge_body(&lie), &a, 501).unwrap();
    let r = report(&trimmed(&sh));
    assert!(r.holds() && r.counters.merges == 2, "the file alone cannot say: {:?}", r.counters);
    let lines = sh.merge_lines();
    assert_eq!((lines[0].recheck, lines[1].recheck), (Recheck::Same, Recheck::Differs));
    assert!(!merge_report(&sh, true).1, "doc_verify's DOES NOT HOLD");
}

/// REPLAY. The same file merged twice changes nothing, and an old MERGE's record laid in again is a
/// fork of its own device.
#[test]
fn a_replayed_merge_is_nothing_or_a_fork() {
    let (a, b) = keys();
    let (fa, fb) = calm(&a, &b);
    let mut sh = opened(&fa, 4);
    sh.merge(&fb, &a, 500).unwrap();
    let m = trimmed(&sh);
    assert_eq!(sh.merge(&fb, &a, 501).unwrap(), Merged::Nothing);
    assert_eq!(sh.merge(&fa, &a, 502).unwrap(), Merged::Nothing);
    assert_eq!(trimmed(&sh), m);
    let roots = honest(&a);
    let mut s = round_trip(&m, &roots);
    let p = pos_of(&s, MERGE_MARK);
    let again = s.recs[p].clone();
    s.recs.push(again);
    let r = only("m2-replayed-record", &write(&s), &roots).unwrap();
    assert!(!r.holds() && r.counters.forks == 1, "{:?}", r.counters);
}

#[test]
fn a_file_merged_with_itself_writes_nothing() {
    let (a, b) = keys();
    let (fa, _) = calm(&a, &b);
    let mut sh = opened(&fa, 4);
    assert_eq!(sh.merge(&fa, &a, 500).unwrap(), Merged::Nothing);
    assert_eq!(trimmed(&sh), fa);
}

#[test]
fn two_different_genesis_are_refused() {
    let (a, b) = keys();
    let other = DeviceKey::from_seed(&[0xC3; 32]);
    let mut sh = opened(&genesis(&a, &b), 4);
    assert_eq!(refused(&mut sh, &genesis(&other, &b), &a), DocError::ForeignRoot);
    // ONE key, two documents: same roots, and the first record says it.
    let s2 = {
        let mut x = SealedHub::create(&a).unwrap();
        x.set_entropy([9; 32]);
        x.sync_stock(&[received("salt", 1)], &a, 1).unwrap();
        trimmed(&x)
    };
    assert!(matches!(refused(&mut sh, &s2, &a), DocError::Fork { seq: 1, .. }));
}

/// A device restored from a backup: one key, two histories.
#[test]
fn one_device_on_two_copies_is_a_fork() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    let x = offline(&s, 2, &a, &[served("rice", 1, "x")]);
    let y = offline(&s, 3, &a, &[served("rice", 1, "y")]);
    let mut sh = opened(&x, 4);
    let e = refused(&mut sh, &y, &a);
    assert!(matches!(e, DocError::Fork { actor, .. } if actor == a.id()), "{e:?}");
}

/// A MERGE BY A REVOKED KEY, three ways.
#[test]
fn a_revoked_key_merges_nothing() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    let fb = offline(&s, 3, &b, &[received("tuna", 4)]);
    let bseq = 1i64;
    // (1) A saw B's seq 1 and revokes after it, naming that record. B takes that in, no MERGE owed,
    // and B's next write is refused. A REVOKE names only what the revoker holds (`tam_tests::h2`).
    let mut ha = opened(&s, 4);
    assert_eq!(ha.merge(&fb, &a, 150).unwrap(), Merged::Adopted { records: 1 });
    ha.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: bseq, tip: ha.tip_at(&b.id(), bseq) }, &a, 200).unwrap();
    ha.sync_stock(&[received("rice", 1)], &a, 201).unwrap();
    let fa = trimmed(&ha);
    assert!(report(&fa).holds());
    let mut hb = opened(&fb, 5);
    assert_eq!(hb.merge(&fa, &b, 301).unwrap(), Merged::Adopted { records: 2 });
    let why = format!("{:?}", hb.sync_stock(&[received("tuna", 1)], &b, 302).unwrap_err());
    assert!(why.contains("revoked"), "{why}");
    // (2) B wrote past the REVOKE, offline, in its own file: refused by name.
    let fb2 = offline(&fb, 6, &b, &[received("tuna", 1)]);
    let mut hc = opened(&fa, 7);
    assert_eq!(refused(&mut hc, &fb2, &a), DocError::RevokedTail { actor: b.id(), after_seq: bseq });
    // (3) B's own file knows nothing of the REVOKE, so B signs a MERGE there: that same tail.
    let mut own = opened(&fb, 8);
    own.declare(&merge_body(&MergeView { ours: Vec::new(), theirs: Vec::new(), state: STATE_COMMUTES, seg_root: [0; 32], conflicts: Vec::new() }), &b, 302).unwrap();
    assert!(report(&trimmed(&own)).holds(), "alone, B's file cannot know");
    let mut hd = opened(&fa, 9);
    assert_eq!(refused(&mut hd, &trimmed(&own), &a), DocError::RevokedTail { actor: b.id(), after_seq: bseq });
    // (4) The table zeroed by a forger does not bring the key back: `verify` folds the records.
    let roots = honest(&a);
    let mut s4 = round_trip(&fa, &roots);
    zero_table(&mut s4);
    let r = only("m2-table-zeroed", &write(&s4), &roots).unwrap();
    assert!(!r.holds() && r.counters.keys_disagree == 1, "{:?}", r.counters);
    let mut he = opened(&fb, 10);
    let e = refused(&mut he, &write(&s4), &b);
    assert_eq!(e, DocError::NotWhole { ours: false });
}

const PHONE: &str = "+355691111111";

#[test]
fn an_erasure_on_one_side_is_refused_and_no_text_comes_back() {
    let (a, b) = keys();
    let mut sh = opened(&genesis(&a, &b), 2);
    let json = format!(r#"{{"status":"new","contact":{{"phone":"{PHONE}"}}}}"#);
    sh.append(EventKind::Placed, "p1", &json, &a, 3).unwrap();
    let s = trimmed(&sh);
    let mut gone = opened(&s, 3);
    gone.forget(|e| e.order_json.contains(PHONE).then(|| e.order_json.replace(PHONE, "")), &[5; 32], &a, 4).unwrap();
    let fa = trimmed(&gone);
    let fb = offline(&s, 4, &b, &[received("tuna", 1)]);
    let had_it = |f: &[u8]| f.windows(PHONE.len()).any(|w| w == PHONE.as_bytes());
    assert!(!had_it(&fa) && had_it(&fb), "the control");
    assert_eq!(refused(&mut gone, &fb, &a), DocError::ForgetOneSide);
    assert!(!had_it(&trimmed(&gone)), "the text did not come back");
    let mut other = opened(&fb, 5);
    assert_eq!(refused(&mut other, &fa, &b), DocError::ForgetOneSide, "and the older copy does not take half an erasure");
}

#[test]
fn a_cut_on_one_side_is_refused_and_a_shared_cut_is_the_shared_past() {
    let (a, b) = keys();
    let mut sh = opened(&genesis(&a, &b), 2);
    sh.sync_stock(&[received("salt", 1)], &a, 3).unwrap();
    sh.sync_stock(&[received("oil", 1)], &a, 4).unwrap();
    let s = trimmed(&sh);
    let fb = offline(&s, 3, &b, &[received("tuna", 1)]);
    let mut cutter = opened(&s, 4);
    cutter.cut(3, &a, 5).unwrap();
    let cut = trimmed(&cutter);
    let e = refused(&mut cutter, &fb, &a);
    assert!(matches!(e, DocError::CutOneSide { .. }), "{e:?}");
    // the ADMIT stays above the cut, so B's first record still chains
    let fa2 = offline(&cut, 5, &a, &[received("rice", 1)]);
    let fb2 = offline(&cut, 6, &b, &[received("tuna", 1)]);
    let mut x = opened(&fa2, 7);
    assert!(matches!(x.merge(&fb2, &a, 6).unwrap(), Merged::Sealed { .. }));
    let r = report(&trimmed(&x));
    assert!(r.holds() && r.counters.merges == 1 && r.counters.segments == 1, "{:?}", r.counters);
    assert_eq!(x.merge_lines()[0].recheck, Recheck::Unchecked);
}

#[test]
fn a_file_that_does_not_hold_is_not_glued() {
    let (a, b) = keys();
    let (fa, fb) = calm(&a, &b);
    let roots = honest(&a);
    let mut s = round_trip(&fb, &roots);
    let top = s.recs.len() - 1;
    s.recs[top].payload[20] ^= 1;
    let mut sh = opened(&fa, 4);
    assert_eq!(refused(&mut sh, &write(&s), &a), DocError::NotWhole { ours: false });
}

/// Trust: `theirs` answers to OUR roots, never its own table: a row zeroed into a root is refused
/// before anything is judged.
#[test]
fn their_table_cannot_name_a_new_root() {
    let (a, b) = keys();
    let (fa, fb) = calm(&a, &b);
    let mut s = read(&fb);
    s.keys.iter_mut().for_each(|k| k.1 = None);
    let mut sh = opened(&fa, 4);
    assert_eq!(refused(&mut sh, &write(&s), &a), DocError::ForeignRoot);
}

/// An anchor over a record only one side had follows it: the id did not change.
#[test]
fn an_anchor_travels_with_its_record() {
    let (a, b) = keys();
    let (fa, fb) = calm(&a, &b);
    let mut sb = opened(&fb, 5);
    let tip = sb.tip().unwrap();
    sb.anchor(1, &tip, 150, b"proof").unwrap();
    let mut sh = opened(&fa, 4);
    sh.merge(&trimmed(&sb), &a, 500).unwrap();
    let s = read(&trimmed(&sh));
    assert_eq!(s.anchors.len(), 1);
    assert!(s.recs.iter().any(|r| r.id == s.anchors[0].over), "`over` names a record of the union");
}

/// K2's price: many movements, one seal; and it merges like any other.
#[test]
fn one_seal_per_sync() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    let moves: Vec<StockEvent> = (0..5).map(|i| served("rice", 1, &format!("o{i}"))).collect();
    let fb = offline(&s, 3, &b, &moves);
    let fa = offline(&s, 2, &a, &[served("rice", 1, "a")]);
    let before = report(&fa).counters.seals_ok;
    let mut sh = opened(&fa, 4);
    sh.merge(&fb, &a, 500).unwrap();
    let c = report(&trimmed(&sh)).counters;
    assert_eq!((c.seals_ok, c.records), (before + 1 + 1, 4 + 5 + 1), "{c:?}");
    let refusal = opened(&s, 5).sync_stock(&[], &a, 1);
    assert!(matches!(refusal, Err(HubError::Store(_))), "{refusal:?}");
}

/// The library's own door refuses the same things with no hub in the way.
#[test]
fn the_verifier_trusts_nothing_the_merge_wrote() {
    let (a, b) = keys();
    let m = sealed_merge(&a, &b);
    let roots = honest(&a);
    let r = doc::verify(&Store::from_bytes(&m), &HubIds, &MlDsa65, Trust::Only(&roots)).unwrap();
    assert!(r.holds() && r.admitted == 2, "{r:?}");
    let wrong = [[9u8; 32]];
    let r = doc::verify(&Store::from_bytes(&m), &HubIds, &MlDsa65, Trust::Only(&wrong)).unwrap();
    assert!(!r.holds() && r.counters.seals_ok == 0, "{:?}", r.counters);
}
