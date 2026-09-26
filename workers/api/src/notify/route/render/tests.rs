//! The words of a message, per language, and the person cut to the group's level.
use super::*;
use serde_json::json;

#[test]
fn every_language_row_has_every_word_and_english_is_first() {
    assert_eq!(WORDS[0].lang, "en");
    assert!(langs().len() >= 4, "sq, en, uk and ru");
    for w in WORDS {
        for word in [w.pickup, w.delivery, w.table, w.low, w.daily, w.weekly, w.orders, w.test, w.gone] {
            assert!(!word.trim().is_empty(), "{} is missing a word", w.lang);
        }
    }
    assert!(speaks("ru") && !speaks("xx"));
    assert_eq!(words("xx").lang, "en");
}

#[test]
fn a_table_and_pickup_are_reworded_and_a_dish_called_delivery_is_not() {
    let t = "🍣 V — #1\n🍽 table 4\n\n1 × delivery cake — 5 ALL\n\ndiscount −5 ALL\n💰 0 ALL · cash";
    let out = ticket(t, "uk", Pii::None);
    assert!(out.contains("🍽 стіл 4"));
    assert!(out.contains("1 × delivery cake — 5 ALL"), "a line is re-worded only at its anchor");
    assert!(out.contains("знижка −5 ALL"));
    assert_eq!(ticket("🥡 pickup", "sq", Pii::None), "🥡 merr vetë");
    assert_eq!(ticket("🍽 in the venue (no table recorded)", "ru", Pii::None), "🍽 в заведении (стол не указан)");
    assert_eq!(ticket("delivery fee", "sq", Pii::None), "delivery fee", "not a number: not the fee line");
}

#[test]
fn english_at_full_is_the_identity() {
    let t = "🍣 V — #1\n👤 A B.\n🛵 delivery\n📍 X\n\n1 × Y — 1 ALL\n\ndelivery 3 ALL\ntip 1 ALL\n💰 5 ALL · cash";
    assert_eq!(ticket(t, "en", Pii::Full), t);
    assert_eq!(ticket(t, "en", Pii::Fulfil), t);
    assert!(!ticket(t, "en", Pii::None).contains("A B."));
}

#[test]
fn every_stock_shape_renders_without_a_customer() {
    let d = json!({ "items": [{ "name": "Rice", "on_hand": 900, "low_at": 1000, "unit": "g", "qty": 2, "expiry": "2026-01-20", "expected": 5, "observed": 3 }] });
    assert_eq!(event("stock.low", &d, "en"), "📉 running low\n- Rice: 900 g (≤ 1000)");
    assert_eq!(event("stock.expiring", &d, "en"), "⏳ expiring soon\n- Rice: 2 g · 2026-01-20");
    assert_eq!(event("stocktake.variance", &d, "sq"), "🧮 diferencë numërimi\n- Rice: 5 → 3 g");
    let r = json!({ "name": "Salmon", "qty": 5000, "unit": "g", "lot": "L7", "expiry": "2026-01-22" });
    assert_eq!(event("stock.received", &r, "en"), "📦 delivery received: Salmon 5000 g · L7 · 2026-01-22");
    assert_eq!(event("stock.received", &json!({ "name": "Nori", "qty": 3, "unit": "pc" }), "en"), "📦 delivery received: Nori 3 pc");
    assert_eq!(event("order.status", &json!({ "order": "abcdefghijk", "status": "ready" }), "en"), "🔔 order #abcdefgh → ready");
    assert_eq!(event("order.late", &json!({ "order": "abcdefghijk", "minutes": 12 }), "en"), "⏰ #abcdefgh waiting for 12 min");
    assert_eq!(event("anything.else", &json!({ "text": "hello" }), "en"), "hello");
}

#[test]
fn a_summary_lists_numbers_then_top_dishes_then_what_the_group_chose() {
    let s = Summary { venue: "Dubin".into(), orders: 12, revenue: "18000 ALL".into(), top: vec![("Maki".into(), 9), ("Sake".into(), 4)] };
    let t = digest("en", false, &s, &["📉 running low · - Rice".into()]);
    assert_eq!(t, "📊 Daily summary · Dubin\n12 orders · revenue 18000 ALL\n\nTop dishes:\n1. Maki × 9\n2. Sake × 4\n\nAlso:\n• 📉 running low · - Rice\n");
    let empty = Summary { venue: "D".into(), orders: 0, revenue: "0 ALL".into(), top: vec![] };
    assert_eq!(digest("sq", true, &empty, &[]), "📊 Përmbledhja e javës · D\n0 porosi · të ardhura 0 ALL\n");
}
