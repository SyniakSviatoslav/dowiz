//! W-MG-ATK4, ATTACK 4: S2 UNDER ID v2. A stock id binds `seq` = the record's position, computed
//! on the wire as `ch.n - th.len()`. Stale bases, two tablets, a legacy image then a flip: every
//! case asks whether a position, an id or a tablet's landing can go wrong.

use bebop_store::evlog::{EvLog, FLAG_NONCE_DERIVED, FLAG_SHA256};
use bebop_store::nodekey::sha256;
use bebop_store::Store;
use dowiz_hub::glue::merge::merge;
use dowiz_hub::glue::{glue, tablet, State as Gs};
use dowiz_hub::stock::{StockEvent, StockLog};
use std::collections::BTreeSet;

const WASTE: &str = r#"{"k":"wasted","item":"salmon","qty":1,"reason":"spoiled","by":"ana"}"#;

fn hub(sha: bool) -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.set_entropy([5; 32]);
    s.set_sha256(sha);
    s.set_clock(1000);
    s.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    s
}

fn records(s: &StockLog) -> Vec<bebop_store::evlog::Record> {
    let mut w = EvLog::walk(&Store::from_bytes(&s.to_bytes_trimmed()));
    w.reverse();
    w
}

/// One sync: send everything above `base`, adopt, rebase. Answers the tablet's image, new base.
fn sync(hub: &mut StockLog, img: &[u8], ctx: &str, base: &[u8; 32]) -> (Vec<u8>, [u8; 32]) {
    let t = tablet::tail(img, ctx, base).unwrap();
    let bytes = hub.to_bytes_trimmed();
    let r = merge(&bytes, hub, &t).unwrap();
    assert!(r.law(t.recs.len()), "{r:?}");
    (tablet::rebase(img, base, false, t.recs.len(), &r.entries, &r.tip).unwrap(), r.tip)
}

fn whole(hub: &StockLog) {
    let recs = records(hub);
    let ids: BTreeSet<[u8; 32]> = recs.iter().map(|r| r.id).collect();
    assert_eq!(ids.len(), recs.len(), "no id reused");
    let mut prev = [0u8; 32];
    for (pos, r) in recs.iter().enumerate() {
        assert_eq!(r.prev, prev, "the chain links at {pos}");
        assert_eq!(r.actor_seq, pos as u64, "seq is the position at {pos}");
        prev = r.id;
    }
}

/// Held: two tablets from one base, a hub write between them, each one stale in turn.
#[test]
fn held_two_tablets_one_base_keep_every_position() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let (b, _) = tablet::record(&img, "bar", WASTE, 2001).unwrap();
    let (a2, atip) = sync(&mut h, &a, "kitchen", &base);
    h.append(&StockEvent::Received { item: "salmon".into(), qty: 1 }).unwrap();
    let (b2, btip) = sync(&mut h, &b, "bar", &base);
    whole(&h);
    assert_eq!(btip, h.tip_id());
    assert_eq!(StockLog::load(&b2).unwrap().tip_id(), h.tip_id(), "b lands on the hub's tip");
    let (a3, _) = tablet::record(&a2, "kitchen", WASTE, 3000).unwrap();
    let (a4, tip) = sync(&mut h, &a3, "kitchen", &atip);
    whole(&h);
    assert_eq!(StockLog::load(&a4).unwrap().tip_id(), tip);
    assert_eq!(glue(&h.to_bytes_trimmed(), &a4, None).unwrap().state(), Gs::Same);
}

/// Held: records written after the tail left are carried over the rebase and adopted next time.
#[test]
fn held_unsent_records_survive_a_rebase() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let t = tablet::tail(&a, "kitchen", &base).unwrap();
    let (late, _) = tablet::record(&a, "kitchen", WASTE, 2500).unwrap();
    h.append(&StockEvent::Received { item: "salmon".into(), qty: 1 }).unwrap();
    let bytes = h.to_bytes_trimmed();
    let r = merge(&bytes, &mut h, &t).unwrap();
    let reb = tablet::rebase(&late, &base, false, t.recs.len(), &r.entries, &r.tip).unwrap();
    let (_, tip) = sync(&mut h, &reb, "kitchen", &r.tip);
    assert_eq!(tip, h.tip_id());
    whole(&h);
}

/// Held: a legacy shelf, a bare tablet tail, then the flip: the hub rewrites the tablet's record
/// with a nonce and the current tablet code replays it onto the hub's tip.
#[test]
fn held_legacy_image_then_flip_then_a_bare_tail() {
    let mut h = hub(false);
    let base = h.tip_id();
    let mut bare = StockLog::load(&h.to_bytes_trimmed()).unwrap();
    bare.set_sha256(false);
    bare.append(&StockEvent::Received { item: "salmon".into(), qty: 2 }).unwrap();
    let img = bare.to_bytes_trimmed();
    h.set_sha256(true);
    h.append(&StockEvent::Received { item: "salmon".into(), qty: 1 }).unwrap();
    let (reb, tip) = sync(&mut h, &img, "kitchen", &base);
    assert_eq!(StockLog::load(&reb).unwrap().tip_id(), tip);
    whole(&h);
    let flags: Vec<i64> = records(&h).iter().map(|r| r.flags & FLAG_SHA256).collect();
    assert_eq!(flags[0], 0);
    assert!(flags[1..].iter().all(|f| *f != 0), "{flags:?}");
}

/// H11. A tablet has no seed. Rebased over a hub record its `prev` moved, so `stock_flags` no longer
/// sees the derived nonce and the flag is off, while the bytes stay (every adopted record goes back
/// as an index). The `glued` note keeps the count, so `derived` does not undercount.
#[test]
fn a_derived_nonce_is_counted_across_a_rebase() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    h.append(&StockEvent::Received { item: "salmon".into(), qty: 1 }).unwrap();
    let (reb, tip) = sync(&mut h, &a, "kitchen", &base);
    assert_eq!(StockLog::load(&reb).unwrap().tip_id(), tip, "the tablet lands");
    let rec = records(&h).into_iter().find(|r| String::from_utf8_lossy(&r.payload).contains("wasted")).unwrap();
    let (nonce, body) = rec.payload.split_at(16);
    let mut m = b"dowiz/nonce/derived".to_vec();
    m.extend_from_slice(&base);
    m.extend_from_slice(body);
    assert_eq!(nonce, dowiz_hub::crypto::b64url_encode(&sha256(&m)[..12]).as_bytes(), "control: derived from the OLD prev");
    assert_eq!(rec.flags & FLAG_NONCE_DERIVED, 0, "control: the flag cannot survive");
    assert_eq!(h.derived(), 1, "and it is counted");
}

/// Twin: a fast-forward keeps the tablet's bytes, flag and all: `derived` counts it honestly.
#[test]
fn a_fast_forward_keeps_the_tablets_record_flagged() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let want = StockLog::load(&a).unwrap().tip_id();
    let _ = sync(&mut h, &a, "kitchen", &base);
    let rec = records(&h).into_iter().find(|r| String::from_utf8_lossy(&r.payload).contains("wasted")).unwrap();
    assert_eq!(rec.id, want, "the same id on both sides");
    assert_ne!(rec.flags & FLAG_NONCE_DERIVED, 0);
    assert_eq!(h.derived(), 1, "counted once, by its flag");
}

/// H10. Each tablet is its own context (`tablet.js` gives a new stalk its own ctx): the same
/// movement in the same ms from two of them is two movements. One context sending the same tail
/// twice IS a retry, by contract, and is adopted once (`merge`).
#[test]
fn two_tablets_lose_no_movement() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen-a1", WASTE, 2000).unwrap();
    let (b, _) = tablet::record(&img, "kitchen-b2", WASTE, 2000).unwrap();
    let _ = sync(&mut h, &a, "kitchen-a1", &base);
    let _ = sync(&mut h, &b, "kitchen-b2", &base);
    let n = records(&h).iter().filter(|r| String::from_utf8_lossy(&r.payload).contains(r#""k":"wasted""#)).count();
    assert_eq!(n, 2, "two real write-offs, both on the shelf");
}

/// H2. The reply was lost and the tablet wrote one more record: the retry is a longer tail from the
/// same base, its `ktip` is new. Measured live on ac5ff0c4: 2 movements took 3 from the shelf.
/// Every adoption's `glued` note names a point on the tablet's chain, so the old records are known.
#[test]
fn a_longer_retry_adopts_each_movement_once() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let t1 = tablet::tail(&a, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    let r = merge(&bytes, &mut h, &t1).unwrap();
    assert_eq!(r.merged, 1, "control");
    // the reply never lands; the tablet still stands on `base`
    let (b, _) = tablet::record(&a, "kitchen", WASTE, 3000).unwrap();
    let t2 = tablet::tail(&b, "kitchen", &base).unwrap();
    assert_eq!(t2.recs.len(), 2);
    let bytes = h.to_bytes_trimmed();
    let r = merge(&bytes, &mut h, &t2).unwrap();
    let left = h.ledger().unwrap().items().into_iter().find(|(i, _)| i == "salmon").unwrap().1.on_hand;
    assert_eq!(left, 98, "HOLE: two write-offs took {} from the shelf: {r:?}", 100 - left);
    assert_eq!((r.skipped, r.merged), (1, 1), "one known, one new");
    assert!(r.law(2), "the merge law counts the skipped");
    let reb = tablet::rebase(&b, &base, false, 2, &r.entries, &r.tip).unwrap();
    assert_eq!(StockLog::load(&reb).unwrap().tip_id(), h.tip_id(), "and the tablet lands after all");
}

/// H2, twice lost: three tails, each longer, each adopted once.
#[test]
fn two_lost_replies_adopt_once() {
    let mut h = hub(true);
    let (base, mut im) = (h.tip_id(), h.to_bytes_trimmed());
    for (i, at) in [2000, 3000, 4000].into_iter().enumerate() {
        im = tablet::record(&im, "kitchen", WASTE, at).unwrap().0;
        let t = tablet::tail(&im, "kitchen", &base).unwrap();
        let bytes = h.to_bytes_trimmed();
        let r = merge(&bytes, &mut h, &t).unwrap();
        assert_eq!((r.skipped, r.merged), (i, 1), "sync {i}");
    }
    assert_eq!(h.ledger().unwrap().items().into_iter().find(|(i, _)| i == "salmon").unwrap().1.on_hand, 97);
    whole(&h);
}

/// H2 on an unflipped hub, whose tablet writes bare: same rule, same count.
#[test]
fn a_longer_retry_on_a_legacy_hub_adopts_once() {
    let mut h = hub(false);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let t1 = tablet::tail(&a, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    merge(&bytes, &mut h, &t1).unwrap();
    let (b, _) = tablet::record(&a, "kitchen", WASTE, 3000).unwrap();
    let t2 = tablet::tail(&b, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    let r = merge(&bytes, &mut h, &t2).unwrap();
    assert_eq!((r.skipped, r.merged), (1, 1));
}

/// Twin: another context's tail is not this context's retry.
#[test]
fn a_prefix_of_another_context_is_not_skipped() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let t1 = tablet::tail(&a, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    merge(&bytes, &mut h, &t1).unwrap();
    let (b, _) = tablet::record(&a, "bar", WASTE, 3000).unwrap();
    let t2 = tablet::tail(&b, "bar", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    let r = merge(&bytes, &mut h, &t2).unwrap();
    assert_eq!(r.skipped, 0);
}
