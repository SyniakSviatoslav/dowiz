//! The tablet's three doors: a sale past its rights is flagged, never refused; the reply replays
//! onto the hub's tip or is refused; records written after the tail went are carried.

use super::super::merge::tests::{bootstrapped, hub, sync, T0};
use super::super::merge::{merge, Entry};
use super::super::wire::Tail;
use super::*;

#[test]
fn a_sale_past_the_tablets_rights_is_recorded_provisional() {
    // 17 salmon: the first sync grants the kitchen 3/4 of it, 12.
    let mut h = hub(17, 5);
    let (img, _) = bootstrapped(&mut h);
    let (img, p1) = record(&img, "kitchen", r#"{"k":"served","item":"salmon","qty":12,"order":"k-1"}"#, T0).unwrap();
    assert!(!p1, "inside its rights: an ordinary sale");
    let (img, p2) = record(&img, "kitchen", r#"{"k":"served","item":"salmon","qty":1,"order":"k-2"}"#, T0).unwrap();
    assert!(p2, "past its rights: recorded, flagged");
    assert_eq!(StockLog::load(&img).unwrap().provisional_sales().len(), 1);
}

#[test]
fn a_movement_the_shelf_refuses_on_the_tablet_writes_nothing() {
    let mut h = hub(5000, 5);
    let (img, _) = bootstrapped(&mut h);
    let e = record(&img, "kitchen", r#"{"k":"consumed","item":"tuna","qty":1,"order":"never-held"}"#, T0).unwrap_err();
    assert!(matches!(e, TabErr::Stock(_)), "{e:?}");
    assert!(matches!(record(&img, "kitchen", r#"{"k":"teleported"}"#, T0), Err(TabErr::Movement)));
    assert!(record(&img, "kitchen", r#"{"k":"received","item":"tuna","qty":3}"#, T0).is_ok(), "the twin");
}

#[test]
fn records_written_after_the_tail_went_are_carried_across_the_rebase() {
    let mut h = hub(5000, 5);
    let (img, base) = bootstrapped(&mut h);
    let (img, _) = record(&img, "kitchen", r#"{"k":"received","item":"rice","qty":5}"#, T0).unwrap();
    let t = tail(&img, "kitchen", &base).unwrap();
    let r = merge(&h.to_bytes_trimmed(), &mut h, &t).unwrap();
    // While the request was in flight the tablet wrote one more.
    let (img, _) = record(&img, "kitchen", r#"{"k":"received","item":"nori","qty":2}"#, T0 + 1).unwrap();
    let after = rebase(&img, &base, false, t.recs.len(), &r.entries, &r.tip).unwrap();
    let back = StockLog::load(&after).unwrap();
    let top = back.since(&r.tip).expect("the hub's tip is in the rebased chain");
    assert_eq!(top.len(), 1);
    assert!(top[0].contains("\"nori\""), "{top:?}");
    // And it is sent at the next sync, merged.
    let (_, _, r2, _) = sync(&mut h, &after, r.tip);
    assert_eq!(r2.merged, 1);
}

#[test]
fn a_reply_that_does_not_land_on_the_hubs_tip_is_refused() {
    let mut h = hub(5000, 5);
    let (img, base) = bootstrapped(&mut h);
    let (img, _) = record(&img, "kitchen", r#"{"k":"received","item":"rice","qty":5}"#, T0).unwrap();
    let t = tail(&img, "kitchen", &base).unwrap();
    let r = merge(&h.to_bytes_trimmed(), &mut h, &t).unwrap();
    assert!(rebase(&img, &base, false, 1, &r.entries, &r.tip).is_ok(), "the twin");
    let mut lost = r.entries.clone();
    lost.pop();
    assert!(matches!(rebase(&img, &base, false, 1, &lost, &r.tip), Err(TabErr::Reply(_))), "a record missing from the reply");
    let past = vec![Entry::Echo(5)];
    assert!(matches!(rebase(&img, &base, false, 1, &past, &r.tip), Err(TabErr::Reply(_))), "an echo past what was sent");
}

#[test]
fn a_tablet_whose_base_the_hub_lost_starts_over_and_keeps_its_records() {
    let mut h = hub(5000, 5);
    let (img, base) = bootstrapped(&mut h);
    let (img, _) = record(&img, "kitchen", r#"{"k":"received","item":"rice","qty":5}"#, T0).unwrap();
    // The owner reset the shelf: the hub's log starts again, the tablet's base is gone.
    let mut h = hub(5000, 2);
    let t = tail(&img, "kitchen", &base).unwrap();
    let e = merge(&h.to_bytes_trimmed(), &mut h, &t).unwrap_err();
    assert_eq!(e.why, super::super::Why::NoCommonAncestor);
    // Start over: the hub's whole log, the tablet's one record carried on top, sent next time.
    let fresh = Tail { ctx: "kitchen".into(), base: [0u8; 32], tip: [0u8; 32], recs: Vec::new() };
    let r = merge(&h.to_bytes_trimmed(), &mut h, &fresh).unwrap();
    let after = rebase(&img, &base, true, 0, &r.entries, &r.tip).unwrap();
    let (_, _, r2, t2) = sync(&mut h, &after, r.tip);
    assert_eq!((t2.recs.len(), r2.merged), (1, 1), "the record written before the reset is not lost");
}

#[test]
fn the_shelf_the_sheet_steps_through_is_the_tablets_own() {
    let mut h = hub(17, 5);
    let (img, _) = bootstrapped(&mut h);
    let (img, _) = record(&img, "kitchen", r#"{"k":"wasted","item":"salmon","qty":2,"reason":"spoiled","by":"chef"}"#, T0).unwrap();
    let got = items(&img).unwrap();
    assert_eq!(got.iter().find(|(i, _)| i == "salmon").map(|x| x.1), Some(15), "17 less the 2 written offline");
    assert!(got.windows(2).all(|w| w[0].0 < w[1].0), "sorted, so two tablets list one order");
    assert!(matches!(items(b"not an image"), Err(TabErr::Image)), "the twin: a broken image is named");
}
