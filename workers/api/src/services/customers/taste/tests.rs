//! The guest's taste on the server, pinned: what an order adds, how it fades, the segment and its
//! rule, retention, and the device vector's closed shape.

use super::*;

const D: i64 = 20_000;
fn line(tags: &[&str], cat: &str, qty: i64) -> Line {
    Line { tags: tags.iter().map(|s| s.to_string()).collect(), category: Some(cat.into()), qty, ..Line::default() }
}

#[test]
fn an_order_adds_its_dishes_tags_and_categories() {
    let p = apply_order(None, &[line(&["salmon"], "rolls", 2), line(&["salmon", "hot"], "soups", 1)], None, D);
    assert_eq!((p.orders, p.portions, p.first_day, p.last_day), (1, 3, D, D));
    assert_eq!(p.tags["salmon"], 3 * UNIT);
    assert_eq!(p.tags["hot"], UNIT);
    assert_eq!((p.cats["rolls"], p.cats["soups"]), (2 * UNIT, UNIT));
}

#[test]
fn old_weights_fade_by_half_in_sixty_days() {
    let p = apply_order(None, &[line(&["salmon"], "rolls", 2)], None, D);
    let p = apply_order(Some(p), &[line(&["tuna"], "rolls", 1)], None, D + 60);
    assert_eq!(p.tags["salmon"], UNIT, "two portions, sixty days on, count as one");
    assert_eq!(p.tags["tuna"], UNIT);
    assert_eq!((p.orders, p.first_day, p.last_day), (2, D, D + 60));
}

#[test]
fn the_segment_is_recency_and_frequency_with_its_rule() {
    let one = apply_order(None, &[line(&["a"], "c", 1)], None, D);
    assert_eq!(segment(&one, D + 3).0, Segment::New);
    let two = apply_order(Some(one.clone()), &[line(&["a"], "c", 1)], None, D + 5);
    assert_eq!(segment(&two, D + 5 + AT_RISK_DAYS).0, Segment::Regular, "on the boundary still regular");
    assert_eq!(segment(&two, D + 5 + AT_RISK_DAYS + 1).0, Segment::AtRisk);
    assert_eq!(segment(&two, D + 5 + LAPSED_DAYS + 1).0, Segment::Lapsed);
    assert_eq!(segment(&one, D + 40).0, Segment::AtRisk, "a single order goes at risk too");
    let (_, why) = segment(&two, D + 10);
    assert!(why.contains("2 orders") && why.contains("5 days"), "{why}");
}

#[test]
fn a_profile_past_twelve_months_reads_as_absent_and_starts_afresh() {
    let p = apply_order(None, &[line(&["a"], "c", 5)], None, D);
    assert!(!expired(&p, D + KEEP_DAYS));
    assert!(expired(&p, D + KEEP_DAYS + 1));
    let q = apply_order(Some(p), &[line(&["b"], "c", 1)], None, D + KEEP_DAYS + 1);
    assert_eq!((q.orders, q.first_day), (1, D + KEEP_DAYS + 1));
    assert!(!q.tags.contains_key("a"));
    let counts = segment_counts([q.clone(), apply_order(None, &[], None, D)].iter(), D + KEEP_DAYS + 2);
    assert_eq!(counts["new"], 1, "the expired one is not counted: {counts:?}");
}

#[test]
fn the_device_vector_is_closed_small_and_bounded() {
    let mut s = SyncIn { v: 1, ..SyncIn::default() };
    s.tags.insert("salmon".into(), 1000);
    assert!(s.validate().is_ok());
    let over = SyncIn { tags: (0..13).map(|i| (format!("t{i}"), 1)).collect(), ..s.clone() };
    assert!(over.validate().is_err());
    let big = SyncIn { cats: [("c".to_string(), 1001)].into_iter().collect(), ..s.clone() };
    assert!(big.validate().is_err());
    assert!(SyncIn { v: 2, ..s.clone() }.validate().is_err());
    assert!(serde_json::from_str::<SyncIn>(r#"{"v":1,"tags":{},"cats":{},"events":[1]}"#).is_err(), "raw events cannot ride along");
    let p = apply_order(None, &[], Some(&s), D);
    assert_eq!(p.device.as_ref().map(|d| d.tags["salmon"]), Some(1000));
    assert_eq!(p.device_day, Some(D));
}

#[test]
fn the_view_shows_everything_held_and_round_trips() {
    let p = apply_order(None, &[line(&["salmon"], "rolls", 2)], None, D);
    let v = view(&p, D + 1);
    assert_eq!(v["segment"], "new");
    assert_eq!(v["tags"][0]["key"], "salmon");
    assert_eq!(v["keptUntilDay"], D + KEEP_DAYS);
    assert_eq!(parse(&serde_json::to_string(&p).unwrap()), Some(p));
    assert_eq!(parse("{not json"), None);
}

#[test]
fn a_line_reads_its_dish_tags_and_category() {
    let l = line_of(r#"{"id":"maki","categoryId":"rolls","tags":["salmon","hot"]}"#, 2);
    assert_eq!(l, Line { tags: vec!["salmon".into(), "hot".into()], category: Some("rolls".into()), qty: 2, id: "maki".into(), ..Line::default() });
}
