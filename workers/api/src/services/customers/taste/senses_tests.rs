//! The guest on the dish's axes (W-SENSE rows 3, 4, 6, 7), pinned: what an order adds, how it
//! fades, the context and the month it is filed under, and the owner's segment builder.

use super::builder::{self, Filter};
use super::senses::{self, month_of, At};
use super::*;

const D: i64 = 20_000; // 2024-10-04
fn smoky_crispy(qty: i64) -> Line {
    let s = dowiz_hub::sense::validate(&serde_json::json!({"taste": {"salty": 3}, "texture": {"crispy": 3}, "aroma": {"smoky": 3}})).unwrap();
    Line { tags: vec![], category: Some("rolls".into()), qty, sense: dowiz_hub::sense::vector(&s) }
}
fn sweet_soft(qty: i64) -> Line {
    let s = dowiz_hub::sense::validate(&serde_json::json!({"taste": {"sweet": 5}, "texture": {"soft": 3}})).unwrap();
    Line { tags: vec![], category: Some("desserts".into()), qty, sense: dowiz_hub::sense::vector(&s) }
}
fn at(keys: &[&str], when: &str) -> At {
    At { keys: keys.iter().map(|s| s.to_string()).collect(), when: when.into() }
}

#[test]
fn an_order_adds_the_dish_vector_and_old_weights_fade_by_half_in_sixty_days() {
    let p = apply_order(None, &[smoky_crispy(2)], None, D);
    assert_eq!(p.sense["a:smoky"], 2 * UNIT);
    assert_eq!(p.sense["x:crispy"], 2 * UNIT);
    assert_eq!(p.sense["t:salty"], 2 * UNIT * 600 / 1000, "salty 3 of 5 = 600 per mille");
    let q = apply_order(Some(p), &[sweet_soft(1)], None, D + 60);
    assert_eq!(q.sense["a:smoky"], UNIT, "two portions sixty days on count as one");
    assert_eq!(q.sense["t:sweet"], UNIT);
    // A dish that declares nothing adds nothing (no fake zeros), and no key outside the vocabulary.
    let none = apply_order(None, &[Line { qty: 3, ..Line::default() }], None, D);
    assert!(none.sense.is_empty() && none.months.is_empty());
    let odd = Line { qty: 1, sense: [("t:richness".to_string(), 1000)].into_iter().collect(), ..Line::default() };
    assert!(apply_order(None, &[odd], None, D).sense.is_empty());
}

#[test]
fn each_order_is_filed_under_its_context_and_its_weekday_band() {
    let p = apply_order_at(None, &[smoky_crispy(1)], None, D, Some(&at(&["band:evening", "day:weekday", "wx:rain"], "thu:evening")));
    let p = apply_order_at(Some(p), &[sweet_soft(1)], None, D, Some(&at(&["band:midday", "day:weekend", "wx:hot"], "sat:midday")));
    let p = apply_order_at(Some(p), &[smoky_crispy(1)], None, D, Some(&at(&["band:evening", "day:weekday", "wx:rain"], "thu:evening")));
    assert_eq!(p.ctx["wx:rain"]["a:smoky"], 2 * UNIT);
    assert!(!p.ctx["wx:rain"].contains_key("t:sweet"), "the sweet order was on a hot day");
    assert_eq!(p.ctx["wx:hot"]["t:sweet"], UNIT);
    assert_eq!((p.when["thu:evening"], p.when["sat:midday"]), (2, 1));
    assert_eq!(senses::main_band(&p).as_deref(), Some("evening"));
    // No context: nothing filed under one, the rest as before.
    let bare = apply_order(None, &[smoky_crispy(1)], None, D);
    assert!(bare.ctx.is_empty() && bare.when.is_empty() && !bare.sense.is_empty());
}

#[test]
fn a_snapshot_is_taken_each_month_and_twelve_are_kept() {
    assert_eq!(month_of(0), "1970-01");
    assert_eq!(month_of(D), "2024-10");
    assert_eq!(month_of(19_783), "2024-03", "2024-03-01");
    let mut p = None;
    for i in 0..15 {
        p = Some(apply_order(p, &[smoky_crispy(1)], None, D + i * 31));
    }
    let p = p.unwrap();
    assert_eq!(p.months.len(), senses::MONTHS_KEEP);
    let newest = p.months.keys().last().unwrap().clone();
    assert_eq!(newest, month_of(D + 14 * 31));
    assert_eq!(p.months[&newest]["a:smoky"], 1000, "per mille of the month's strongest");
    assert!(!p.months.contains_key("2024-10"), "the oldest went first");
}

#[test]
fn the_share_reads_the_server_fold_then_the_device_and_because_names_two() {
    let p = apply_order(None, &[smoky_crispy(1)], None, D);
    assert_eq!(senses::share(&p, "a:smoky"), 1000);
    assert_eq!(senses::share(&p, "t:salty"), 600);
    assert_eq!(senses::share(&p, "t:sweet"), 0);
    assert_eq!(senses::because(&p), ["a:smoky", "x:crispy"]);
    let mut s = SyncIn { v: 1, ..SyncIn::default() };
    s.sense.insert("x:creamy".into(), 800);
    let d = apply_order(None, &[], Some(&s), D);
    assert_eq!(senses::share(&d, "x:creamy"), 1000, "no fold yet: the device's vector, scaled to its top");
}

#[test]
fn the_device_sense_vector_is_closed_to_the_vocabulary() {
    let mut s = SyncIn { v: 1, ..SyncIn::default() };
    s.sense.insert("a:smoky".into(), 1000);
    assert!(s.validate().is_ok());
    let mut bad = s.clone();
    bad.sense.insert("allergen:fish".into(), 10);
    assert!(bad.validate().unwrap_err().contains("vocabulary"));
    let mut big = s.clone();
    big.sense.insert("x:crispy".into(), 1001);
    assert!(big.validate().is_err());
    assert!(serde_json::from_str::<SyncIn>(r#"{"v":1,"sense":{"t:spicy":3}}"#).unwrap().validate().is_ok());
}

#[test]
fn the_builder_takes_only_vocabulary_keys_and_known_words() {
    let ok = Filter { key: "a:smoky".into(), min: 500, not_seen_days: Some(4), min_orders: Some(2), band: Some("evening".into()), weather: Some("rain".into()) };
    assert!(ok.check().is_ok());
    for (f, says) in [
        (Filter { key: "allergen:gluten".into(), ..ok.clone() }, "vocabulary"),
        (Filter { key: "t:richness".into(), ..ok.clone() }, "vocabulary"),
        (Filter { min: 0, ..ok.clone() }, "min"),
        (Filter { not_seen_days: Some(0), ..ok.clone() }, "not_seen_days"),
        (Filter { min_orders: Some(0), ..ok.clone() }, "min_orders"),
        (Filter { band: Some("brunch".into()), ..ok.clone() }, "band"),
        (Filter { weather: Some("foggy".into()), ..ok.clone() }, "weather"),
    ] {
        assert!(f.check().unwrap_err().contains(says), "{f:?}");
    }
    assert!(serde_json::from_str::<Filter>(r#"{"key":"a:smoky","health":"diabetic"}"#).is_err(), "closed shape");
}

#[test]
fn loves_smoky_not_seen_four_days_orders_in_the_evening() {
    let rain_eve = at(&["band:evening", "day:weekday", "wx:rain"], "thu:evening");
    let lover = apply_order_at(None, &[smoky_crispy(2)], None, D, Some(&rain_eve));
    let lover = apply_order_at(Some(lover), &[smoky_crispy(1)], None, D + 1, Some(&rain_eve));
    let sweet = apply_order_at(None, &[sweet_soft(2)], None, D, Some(&rain_eve));
    let recent = apply_order_at(None, &[smoky_crispy(1)], None, D + 6, Some(&rain_eve));
    let lunch = apply_order_at(None, &[smoky_crispy(1)], None, D, Some(&at(&["band:midday", "day:weekday"], "thu:midday")));
    let f = Filter { key: "a:smoky".into(), min: 500, not_seen_days: Some(4), min_orders: None, band: Some("evening".into()), weather: None };
    let today = D + 6;
    assert!(builder::matches(&lover, &f, today));
    assert!(!builder::matches(&sweet, &f, today), "does not love smoky");
    assert!(!builder::matches(&recent, &f, today), "seen today");
    assert!(!builder::matches(&lunch, &f, today), "orders at midday");
    assert!(builder::matches(&lover, &Filter { weather: Some("rain".into()), ..f.clone() }, today));
    assert!(!builder::matches(&lover, &Filter { weather: Some("hot".into()), ..f.clone() }, today));
    assert!(!builder::matches(&lover, &Filter { min_orders: Some(3), ..f.clone() }, today));
    let all = vec![lover, sweet, recent, lunch];
    let r = builder::report(&all, &f, today);
    assert_eq!((r["count"].clone(), r["of"].clone()), (serde_json::json!(1), serde_json::json!(4)), "{r}");
    assert_eq!(r["busiest"], "thu:evening");
    assert_eq!(r["trend"].as_array().unwrap().len(), builder::TREND_MONTHS);
    let now = r["trend"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(now["month"], month_of(today));
    assert_eq!(now["count"], 3, "every smoky profile this month, whatever the recency: {r}");
    // An expired profile is not counted at all.
    let gone = builder::report(&all, &f, D + KEEP_DAYS + 10);
    assert_eq!(gone["of"], 0);
}
