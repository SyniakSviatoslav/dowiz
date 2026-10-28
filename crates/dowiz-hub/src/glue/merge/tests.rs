//! SHEAF S2 pass marks (R-BEBOPDB G.5 row S2), every case named, no generator, no seed (memory
//! `no-fuzzers-in-this-repo`):
//!   - a day offline, replayed: every movement merged, the tablet lands on the hub's tip, and the
//!     tail costs <= 100 KB on the wire (`a_day_offline_merges_every_movement_under_100_kb`);
//!   - conflicts shown to a person, never lost: `len == merged + held` and a decision per
//!     obstruction (`the_last_salmon_is_a_decision_until_a_person_closes_it`,
//!     `a_record_the_hub_refuses_is_held_verbatim_not_dropped`);
//!   - no-tracking both ways (`wire/tests.rs` for the door; here: nothing guest-shaped comes back).

use super::super::desk;
use super::super::tablet;
use super::super::wire::Tail;
use super::*;
use crate::stock::{StockEvent, StockLog};

pub(crate) const ITEMS: [&str; 8] = ["salmon", "tuna", "rice", "nori", "avocado", "eel", "shrimp", "cheese"];
pub(crate) const T0: i64 = 1_791_000_000_000;

fn res(item: &str, qty: i64, o: &str) -> StockEvent {
    StockEvent::Reserved { item: item.into(), qty, order_id: o.into() }
}
fn con(item: &str, qty: i64, o: &str) -> StockEvent {
    StockEvent::Consumed { item: item.into(), qty, order_id: o.into() }
}

/// A hub with the eight items counted (salmon at `salmon`, the rest at 5000) and `orders`
/// online orders of one portion each, over the seven other items.
pub(crate) fn hub(salmon: i64, orders: usize) -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(T0);
    let first: Vec<StockEvent> = ITEMS.iter().map(|i| StockEvent::Received { item: (*i).into(), qty: if *i == "salmon" { salmon } else { 5000 } }).collect();
    log.append_all(&first).unwrap();
    online(&mut log, "base", orders);
    log
}

/// `n` online orders on the hub: reserve and consume one portion, the seven non-salmon items in turn.
pub(crate) fn online(log: &mut StockLog, tag: &str, n: usize) {
    for k in 0..n {
        let (it, o) = (ITEMS[1 + k % 7], format!("{tag}-{k}"));
        log.append_all(&[res(it, 1, &o), con(it, 1, &o)]).unwrap();
    }
}

/// One sync, as the object and the tablet do it: tail out, merge, rebase.
pub(crate) fn sync(hub: &mut StockLog, image: &[u8], base: [u8; 32]) -> (Vec<u8>, [u8; 32], Reply, Tail) {
    let t = tablet::tail(image, "kitchen", &base).unwrap();
    let bytes = hub.to_bytes_trimmed();
    let r = merge(&bytes, hub, &t).unwrap();
    let img = tablet::rebase(image, &base, false, t.recs.len(), &r.entries, &r.tip).unwrap();
    (img, r.tip, r, t)
}

/// A tablet that has synced once: the hub's image, its tip as base.
pub(crate) fn bootstrapped(h: &mut StockLog) -> (Vec<u8>, [u8; 32]) {
    let (img, base, r, _) = sync(h, &[], [0u8; 32]);
    assert!(!r.replay && r.merged == 0 && r.grants == ITEMS.len(), "a first sync grants every counted item: {r:?}");
    (img, base)
}

fn rec(img: &[u8], k: usize, line: &str) -> Vec<u8> {
    tablet::record(img, "kitchen", line, T0 + k as i64 * 60_000).unwrap().0
}

/// What `serde_json` makes of a list of records: the bytes the request body carries.
fn json_len(recs: &[String]) -> usize {
    serde_json::to_string(recs).unwrap().len()
}

#[test]
fn a_day_offline_merges_every_movement_under_100_kb() {
    let mut h = hub(5000, 1000);
    let (mut img, base) = bootstrapped(&mut h);
    // THE DAY: 400 kitchen movements offline (a delivery every tenth, waste and sales between),
    // while the hub takes 100 online orders (200 records) on the same items.
    for k in 0..400 {
        let it = ITEMS[1 + k % 7];
        let line = match k % 10 {
            0 => format!(r#"{{"k":"received","item":"{it}","qty":40}}"#),
            1 | 2 => format!(r#"{{"k":"wasted","item":"{it}","qty":1,"reason":"spoiled","by":"chef"}}"#),
            _ => format!(r#"{{"k":"served","item":"{it}","qty":1,"order":"k-{k}"}}"#),
        };
        img = rec(&img, k, &line);
    }
    online(&mut h, "day", 100);
    let before = h.len();
    let (img2, tip, r, t) = sync(&mut h, &img, base);
    assert_eq!(t.recs.len(), 400);
    assert!(r.law(400), "every record merged or held: {} + {}", r.merged, r.held.len());
    assert_eq!((r.merged, r.held.len()), (400, 0), "a compatible day merges every movement");
    assert!(r.conflicts.is_empty(), "no obstruction on a stocked day: {:?}", r.conflicts);
    let extra = h.len() - before - 400 - r.grants - 1;
    assert!(extra <= 1, "besides the movements, the grants and the glued note only a due checkpoint is written: {extra}");
    assert_eq!(tip, h.tip_id());
    let back = StockLog::load(&img2).unwrap();
    assert_eq!(back.tip_id(), h.tip_id(), "the tablet replays the reply onto the hub's own tip");
    assert_eq!(back.since(&[0u8; 32]), h.since(&[0u8; 32]), "byte for byte the hub's records");
    assert_eq!(back.ledger().unwrap(), h.ledger().unwrap());
    let up = json_len(&t.recs);
    let down = r.wire_down();
    println!("S2 day: up {up} B json ({} B records), down {down} B, hub tail {}, merged {}", r.wire_up, r.entries.len(), r.merged);
    assert!(up <= 100_000, "a day's tail costs {up} B on the wire, over 100 KB");
    let echoes = r.entries.iter().filter(|e| matches!(e, Entry::Echo(_))).count();
    assert_eq!(echoes, 400, "every adopted record goes back as its index, never its bytes");
}

#[test]
fn the_last_salmon_is_a_decision_until_a_person_closes_it() {
    // X5 over the wire: 17 salmon; the tablet sells 2 offline, the hub sells 16 online.
    let mut h = hub(17, 50);
    let (mut img, base) = bootstrapped(&mut h);
    img = rec(&img, 0, r#"{"k":"served","item":"salmon","qty":2,"order":"k-s"}"#);
    h.append_all(&[res("salmon", 16, "hub-s"), con("salmon", 16, "hub-s")]).unwrap();
    let (_, _, r, _) = sync(&mut h, &img, base);
    assert!(r.law(1));
    assert_eq!(r.merged, 1, "the sale happened: it is history, merged");
    assert!(r.conflicts.iter().any(|c| matches!(c, Conflict::Oversold { item, .. } if item == "salmon")), "{:?}", r.conflicts);
    let open = desk::open(&h);
    let d = open.iter().find(|d| d.kind == "oversold").expect("the oversale is on the decision list");
    assert_eq!((d.item.as_str(), d.ctx.as_str(), d.qty), ("salmon", "kitchen", Some(-1)));
    assert!(!desk::close(&mut h, "no-such-id", "chef").unwrap(), "an unknown id closes nothing");
    assert!(desk::close(&mut h, &d.did, "chef").unwrap());
    assert!(desk::open(&h).iter().all(|x| x.did != d.did), "closed by a person, gone from the list");
    assert!(!desk::close(&mut h, &d.did, "chef").unwrap(), "closing twice writes once");
}

#[test]
fn a_record_the_hub_refuses_is_held_verbatim_not_dropped() {
    // The tablet consumes order o-1's hold; the hub, online, released it meanwhile.
    let mut h = hub(5000, 10);
    h.append_all(&[res("tuna", 2, "o-1")]).unwrap();
    let (mut img, base) = bootstrapped(&mut h);
    img = rec(&img, 0, r#"{"k":"consumed","item":"tuna","qty":2,"order":"o-1"}"#);
    img = rec(&img, 1, r#"{"k":"wasted","item":"rice","qty":3,"reason":"dropped","by":"chef"}"#);
    h.append_all(&[StockEvent::Released { item: "tuna".into(), qty: 2, order_id: "o-1".into() }]).unwrap();
    let (img2, _, r, t) = sync(&mut h, &img, base);
    assert!(r.law(2), "{} + {} != 2", r.merged, r.held.len());
    assert_eq!((r.merged, r.held.len()), (1, 1));
    assert_eq!(r.held[0].0, 0, "the consume is the one held");
    let d = desk::open(&h).into_iter().find(|d| d.kind == "held").expect("held is a decision");
    assert_eq!(d.rec, t.recs[0], "the record is kept verbatim inside its decision");
    assert_eq!(StockLog::load(&img2).unwrap().tip_id(), h.tip_id(), "the tablet still lands on the hub's tip");
}

#[test]
fn the_same_tail_twice_is_adopted_once() {
    let mut h = hub(5000, 10);
    let (mut img, base) = bootstrapped(&mut h);
    img = rec(&img, 0, r#"{"k":"wasted","item":"rice","qty":1,"reason":"spoiled","by":"chef"}"#);
    let t = tablet::tail(&img, "kitchen", &base).unwrap();
    let r1 = merge(&h.to_bytes_trimmed(), &mut h, &t).unwrap();
    let n = h.len();
    let r2 = merge(&h.to_bytes_trimmed(), &mut h, &t).unwrap();
    assert!(!r1.replay && r2.replay);
    assert_eq!(h.len(), n, "a retried request writes nothing");
    assert_eq!((r2.tip, &r2.entries), (r1.tip, &r1.entries), "and answers what the first did");
}

#[test]
fn nothing_guest_shaped_comes_back_to_a_kitchen() {
    let mut h = hub(17, 30);
    let (mut img, base) = bootstrapped(&mut h);
    img = rec(&img, 0, r#"{"k":"served","item":"salmon","qty":2,"order":"k-s"}"#);
    h.append_all(&[res("salmon", 16, "hub-s"), con("salmon", 16, "hub-s")]).unwrap();
    let (_, _, r, _) = sync(&mut h, &img, base);
    for e in &r.entries {
        if let Entry::Rec(s) = e {
            for k in ["phone", "email", "customer", "guest", "address", "taste", "profile"] {
                assert!(!s.contains(&format!("\"{k}\"")), "a guest key `{k}` in the hub's tail: {s}");
            }
        }
    }
}

/// THE MEASUREMENT (release, `--ignored`): one merge at 2000 hub records + 200 hub + 400 tablet.
#[test]
#[ignore]
fn measure_merge_2000_200_400() {
    let mut h = hub(5000, 1000);
    let (mut img, base) = bootstrapped(&mut h);
    for k in 0..400 {
        img = rec(&img, k, &format!(r#"{{"k":"served","item":"{}","qty":1,"order":"k-{k}"}}"#, ITEMS[1 + k % 7]));
    }
    online(&mut h, "day", 100);
    let t = tablet::tail(&img, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    let (mut us, mut next) = (Vec::new(), Vec::new());
    for _ in 0..9 {
        let mut l = StockLog::load(&bytes).unwrap();
        let t0 = std::time::Instant::now();
        let r = merge(&bytes, &mut l, &t).unwrap();
        us.push(t0.elapsed().as_micros());
        assert_eq!(r.merged, 400);
        // WHAT THE DEFERRED CHECKPOINT COSTS THE NEXT WRITE: one delivery through the write door.
        let t0 = std::time::Instant::now();
        l.append(&StockEvent::Received { item: "rice".into(), qty: 1 }).unwrap();
        next.push(t0.elapsed().as_micros());
    }
    us.sort();
    next.sort();
    println!("S2 merge 2000+200+400: median {} us of 9 [{}..{}]", us[4], us[0], us[8]);
    println!("S2 next write after it (pays the deferred checkpoint): median {} us [{}..{}]", next[4], next[0], next[8]);
}

/// THE PROFILE (release, `--ignored`): `merge`'s phases one by one, on the same input as
/// `measure_merge_2000_200_400`, each the median of 9.
#[test]
#[ignore]
fn profile_merge_phases() {
    use std::time::Instant;
    let mut h = hub(5000, 1000);
    let (mut img, base) = bootstrapped(&mut h);
    for k in 0..400 {
        img = rec(&img, k, &format!(r#"{{"k":"served","item":"{}","qty":1,"order":"k-{k}"}}"#, ITEMS[1 + k % 7]));
    }
    online(&mut h, "day", 100);
    let t = tablet::tail(&img, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    let names = ["load", "since#1", "ledger+glue_tail", "adopt_from", "regrant", "glued_note", "since#2(reply, whole)"];
    let mut us = vec![Vec::new(); names.len()];
    for _ in 0..9 {
        let mut i = 0;
        let mut lap = |us: &mut Vec<Vec<u128>>, t0: Instant| { us[i].push(t0.elapsed().as_micros()); i += 1; };
        let t0 = Instant::now();
        let mut l = StockLog::load(&bytes).unwrap();
        lap(&mut us, t0);
        let t0 = Instant::now();
        let _ = l.since(&t.base).unwrap();
        lap(&mut us, t0);
        let t0 = Instant::now();
        let start = l.ledger_since().unwrap();
        let _ = crate::glue::wire::glue_tail(&bytes, &l, &start.0, &t).unwrap();
        lap(&mut us, t0);
        let t0 = Instant::now();
        let (a, led) = l.adopt_from(&t.recs, start).unwrap();
        assert_eq!(a.merged, 400);
        lap(&mut us, t0);
        let t0 = Instant::now();
        let _ = regrant(&mut l, "kitchen", &led).unwrap();
        lap(&mut us, t0);
        let t0 = Instant::now();
        l.append_note(GLUED, r#"{"ctx":"kitchen"}"#).unwrap();
        lap(&mut us, t0);
        let t0 = Instant::now();
        let _ = l.since(&t.base).unwrap();
        lap(&mut us, t0);
    }
    for (n, v) in names.iter().zip(us.iter_mut()) {
        v.sort();
        println!("S2 phase {n}: median {} us [{}..{}]", v[4], v[0], v[8]);
    }
}

/// THE SMALL SYNC (release, `--ignored`): a tablet that syncs every five minutes sends ~20.
#[test]
#[ignore]
fn measure_merge_small_sync_2000_20() {
    let mut h = hub(5000, 1000);
    let (mut img, base) = bootstrapped(&mut h);
    for k in 0..20 {
        img = rec(&img, k, &format!(r#"{{"k":"served","item":"{}","qty":1,"order":"k-{k}"}}"#, ITEMS[1 + k % 7]));
    }
    online(&mut h, "day", 5);
    let t = tablet::tail(&img, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    let mut us = Vec::new();
    for _ in 0..9 {
        let mut l = StockLog::load(&bytes).unwrap();
        let t0 = std::time::Instant::now();
        let r = merge(&bytes, &mut l, &t).unwrap();
        us.push(t0.elapsed().as_micros());
        assert_eq!(r.merged, 20);
    }
    us.sort();
    println!("S2 merge small 2000+10+20: median {} us of 9 [{}..{}]", us[4], us[0], us[8]);
}

/// Sub-phases of `adopt_from` and `regrant` (release, `--ignored`).
#[test]
#[ignore]
fn profile_adopt_and_regrant_parts() {
    use std::time::Instant;
    let mut h = hub(5000, 1000);
    let (mut img, base) = bootstrapped(&mut h);
    for k in 0..400 {
        img = rec(&img, k, &format!(r#"{{"k":"served","item":"{}","qty":1,"order":"k-{k}"}}"#, ITEMS[1 + k % 7]));
    }
    online(&mut h, "day", 100);
    let t = tablet::tail(&img, "kitchen", &base).unwrap();
    let bytes = h.to_bytes_trimmed();
    let med = |mut v: Vec<u128>| { v.sort(); v[v.len() / 2] };
    let (mut led, mut nockp, mut ckp, mut rights, mut raw, mut usage) = (vec![], vec![], vec![], vec![], vec![], String::new());
    for _ in 0..9 {
        let mut l = StockLog::load(&bytes).unwrap();
        let t0 = Instant::now();
        let _ = l.ledger().unwrap();
        led.push(t0.elapsed().as_micros());
        let t0 = Instant::now();
        let _ = l.rights();
        rights.push(t0.elapsed().as_micros());
        let t0 = Instant::now();
        let _ = l.since(&[0u8; 32]).unwrap();
        raw.push(t0.elapsed().as_micros());
        let u0 = format!("{:?}", l.usage());
        l.set_checkpoint_every(usize::MAX);
        let t0 = Instant::now();
        let start = l.ledger_since().unwrap();
        l.adopt_from(&t.recs, start).unwrap();
        nockp.push(t0.elapsed().as_micros());
        usage = format!("{u0} -> {:?}", l.usage());
        let t0 = Instant::now();
        l.checkpoint_now().unwrap();
        ckp.push(t0.elapsed().as_micros());
    }
    println!("S2 sub: ledger {} us, rights {} us, walk-all {} us, adopt(no ckpt) {} us, checkpoint {} us; usage {usage}", med(led), med(rights), med(raw), med(nockp), med(ckp));
}
