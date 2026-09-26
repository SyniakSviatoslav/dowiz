//! The router (T4): one event, every group that wants it, each in its own
//! language and with its own amount of the customer. Every refusal has a twin.
use super::*;
use groups::Pii;
use dowiz_hub::tz::{Dst, Zone};
use serde_json::json;

const UTC: Zone = Zone { standard_minutes: 0, dst: Dst::None };
const TIRANA_WINTER: Zone = Zone { standard_minutes: 60, dst: Dst::Eu };
/// 2026-01-15 00:00 UTC, a Thursday.
const T0: i64 = 1_768_435_200_000;
const MIN: i64 = 60_000;

fn group(id: &str, chat: &str, lang: &str, pii: Pii, evs: &[(&str, Mode)]) -> Group {
    let mut g = Group::fresh(id.into(), chat.into(), None, id.into(), "group".into(), lang);
    g.pii = pii;
    g.subs = evs.iter().map(|(e, m)| (e.to_string(), *m)).collect();
    g
}

const TICKET: &str = "🍣 Dubin — #abcdefgh\n👤 Arben H. +355691\n🛵 delivery\n📍 Rruga 1\n📝 no wasabi\n\n2 × Sake — 1500 ALL\n\ndelivery 300 ALL\ntip 100 ALL\n💰 1900 ALL · cash";

fn order(lines: Vec<Value>) -> Entry {
    routed("o1/route".into(), "order.placed", &json!({ "ticket": TICKET, "lines": lines, "amend": false }), T0)
}

#[test]
fn one_order_two_groups_two_entries_each_with_its_own_customer_and_language() {
    let kitchen = group("kitchen", "-1001", "sq", Pii::None, &[("order.placed", Mode::Now)]);
    let owners = group("owners", "-2002", "en", Pii::Full, &[("order.placed", Mode::Now)]);
    let plan = fan_out(&order(vec![]), &[kitchen, owners], UTC, T0);
    assert_eq!(plan.send.len(), 2);
    let (k, o) = (&plan.send[0], &plan.send[1]);
    assert_eq!((k.id.as_str(), k.to.as_str(), k.kind.as_str()), ("o1/route/g/kitchen", "-1001", "telegram"));
    assert!(!k.text.contains("Arben") && !k.text.contains("Rruga") && !k.text.contains("wasabi") && !k.text.contains("+355"));
    assert!(k.text.contains("🛵 dërgesë") && k.text.contains("dërgesa 300 ALL") && k.text.contains("bakshish 100 ALL"));
    assert!(k.text.contains("2 × Sake — 1500 ALL"), "a dish name is never touched");
    assert_eq!(o.text, TICKET, "full + English = today's ticket, byte for byte");
    assert_eq!(o.to, "-2002");
}

#[test]
fn fulfil_keeps_what_a_courier_needs() {
    let courier = group("riders", "-3", "en", Pii::Fulfil, &[("order.placed", Mode::Now)]);
    let t = &fan_out(&order(vec![]), &[courier], UTC, T0).send[0].text;
    assert!(t.contains("📍 Rruga 1") && t.contains("+355691") && t.contains("📝 no wasabi"));
}

#[test]
fn a_group_that_did_not_ask_or_is_paused_gets_nothing() {
    let mut off = group("a", "-1", "en", Pii::Full, &[("stock.low", Mode::Now)]);
    assert!(fan_out(&order(vec![]), &[off.clone()], UTC, T0).send.is_empty());
    off.subs.insert("order.placed".into(), Mode::Now);
    off.state = State::Muted;
    assert!(fan_out(&order(vec![]), &[off.clone()], UTC, T0).send.is_empty());
    off.state = State::Active;
    assert_eq!(fan_out(&order(vec![]), &[off], UTC, T0).send.len(), 1);
}

#[test]
fn a_topic_is_the_target_and_the_forum_id_travels() {
    let mut g = group("k", "-100123", "en", Pii::None, &[("order.placed", Mode::Now)]);
    g.thread = Some(42);
    assert_eq!(fan_out(&order(vec![]), &[g], UTC, T0).send[0].to, "-100123:42");
}

#[test]
fn quiet_hours_hold_a_stock_message_and_never_an_order() {
    let mut g = group("k", "-1", "en", Pii::None, &[("order.placed", Mode::Now), ("stock.low", Mode::Now)]);
    g.quiet = Some(Window { from: 23 * 60, to: 7 * 60 });
    let at_2am = T0 + 2 * 60 * MIN;
    let low = routed("s1".into(), "stock.low", &json!({ "data": { "items": [{ "name": "Rice", "on_hand": 900, "low_at": 1000, "unit": "g" }] } }), at_2am);
    let p = fan_out(&low, &[g.clone()], UTC, at_2am);
    assert_eq!(p.send[0].next_at_ms, T0 + 7 * 60 * MIN, "waits until 07:00");
    assert!(p.send[0].text.contains("Rice: 900 g (≤ 1000)"));
    let p = fan_out(&order(vec![]), &[g.clone()], UTC, at_2am);
    assert_eq!(p.send[0].next_at_ms, at_2am, "an order rings through");
    let at_noon = T0 + 12 * 60 * MIN;
    assert_eq!(fan_out(&low, &[g], UTC, at_noon).send[0].next_at_ms, at_noon);
}

#[test]
fn a_quiet_window_wraps_midnight_in_the_venues_own_time() {
    let w = Window { from: 23 * 60, to: 7 * 60 };
    // 22:30 UTC is 23:30 in Tirana in winter: inside, ends 07:00 local = 06:00 UTC next day.
    let t = T0 + (22 * 60 + 30) * MIN;
    assert_eq!(quiet_until(w, TIRANA_WINTER, t), Some(T0 + (24 + 6) * 60 * MIN));
    assert_eq!(quiet_until(w, UTC, t), None, "22:30 in UTC is before the window");
    assert_eq!(quiet_until(w, UTC, T0 + 7 * 60 * MIN), None, "`to` is exclusive");
    let day = Window { from: 60, to: 120 };
    assert_eq!(quiet_until(day, UTC, T0 + 90 * MIN), Some(T0 + 120 * MIN));
    assert_eq!(quiet_until(day, UTC, T0 + 30 * MIN), None);
}

#[test]
fn a_summary_choice_is_a_digest_line_not_a_message() {
    let g = group("o", "-1", "uk", Pii::None, &[("stock.wasted", Mode::Digest)]);
    let e = routed("w1".into(), "stock.wasted", &json!({ "data": { "name": "Salmon", "qty": 200, "unit": "g", "reason": "spoiled" } }), T0);
    let p = fan_out(&e, &[g], UTC, T0);
    assert!(p.send.is_empty());
    assert_eq!(p.digest, vec![("o".to_string(), "🗑 списано: Salmon 200 g · spoiled".to_string())]);
}

fn line(name: &str, bar: bool) -> Value {
    let mut l = json!({ "name": name, "quantity": 1 });
    if bar {
        l["station"] = json!("bar");
    }
    l
}

#[test]
fn a_bar_group_gets_only_the_bar_lines_and_nothing_when_there_are_none() {
    let mut kitchen = group("main", "-1", "en", Pii::Full, &[("order.placed", Mode::Now)]);
    kitchen.station = Some("kitchen".into());
    let mut bar = group("bar", "-2", "en", Pii::Full, &[("order.placed", Mode::Now)]);
    bar.station = Some("bar".into());
    let owners = group("owners", "-3", "en", Pii::Full, &[("order.placed", Mode::Now)]);
    let groups = [kitchen, bar, owners];
    let p = fan_out(&order(vec![line("Maki", false), line("Beer", true)]), &groups, UTC, T0);
    assert_eq!(p.send.len(), 3);
    assert!(p.send[0].text.contains("[kitchen]\n1 × Maki") && !p.send[0].text.contains("Beer"));
    assert!(p.send[1].text.contains("[bar]\n1 × Beer") && !p.send[1].text.contains("Maki"));
    assert_eq!(p.send[2].text, TICKET, "a group with no station gets the whole order");
    let p = fan_out(&order(vec![line("Maki", false)]), &groups, UTC, T0);
    assert_eq!(p.send.iter().map(|e| e.to.as_str()).collect::<Vec<_>>(), vec!["-1", "-3"], "no bar line, no bar ticket");
    assert_eq!(p.send[0].text, TICKET, "unsplit, the kitchen gets today's ticket");
}

#[test]
fn an_amendment_rings_only_its_added_lines() {
    let g = group("k", "-1", "sq", Pii::None, &[("order.amended", Mode::Now)]);
    let e = routed("o1/amend/2/route".into(), "order.amended", &json!({ "ticket": "+ #abcdefgh — table 4", "lines": [line("Maki", false)], "amend": true }), T0);
    assert_eq!(fan_out(&e, &[g], UTC, T0).send[0].text, "+ #abcdefgh — tavolina 4\n\n1 × Maki\n");
}

#[test]
fn a_legacy_venue_rings_byte_identical_tickets_and_a_venue_with_groups_one_route() {
    let mut s = dowiz_hub::settings::Settings::create().unwrap();
    s.set("notify.telegram.chat", "123");
    let lines = vec![line("Maki", false)];
    let old: Vec<Entry> = crate::bell_route::telegram_tickets("o1", TICKET, &lines, None, "123", "")
        .into_iter()
        .map(|t| Entry::new(t.id, "telegram", t.to, t.text, T0))
        .collect();
    assert_eq!(bell(&s, "o1", TICKET, &lines, None, T0), old);
    assert_eq!(old[0].id, "o1/telegram");
    s.set(groups::KEY_GROUPS, "[]");
    let new = bell(&s, "o1", TICKET, &lines, Some(3), T0);
    assert_eq!(new.len(), 1);
    assert_eq!((new[0].id.as_str(), new[0].kind.as_str(), new[0].to.as_str()), ("o1/amend/3/route", ROUTE_KIND, "order.amended"));
}

#[test]
fn an_alert_goes_to_the_legacy_chat_as_today_or_routes_with_every_language() {
    let mut s = dowiz_hub::settings::Settings::create().unwrap();
    assert_eq!(alert_target(&s), "");
    s.set("notify.telegram.chat", " 55 ");
    assert_eq!(alert_target(&s), "55");
    let words = |l: &str| format!("alert[{l}]");
    let e = alert_entry("x".into(), "55", &words, T0);
    assert_eq!((e.kind.as_str(), e.to.as_str(), e.text.as_str()), ("telegram", "55", "alert[]"));
    s.set(groups::KEY_GROUPS, "[]");
    assert_eq!(alert_target(&s), "@order.exception");
    let e = alert_entry("x".into(), &alert_target(&s), &words, T0);
    assert_eq!((e.kind.as_str(), e.to.as_str()), (ROUTE_KIND, "order.exception"));
    let g = group("o", "-1", "uk", Pii::None, &[("order.exception", Mode::Now)]);
    assert_eq!(fan_out(&e, &[g], UTC, T0).send[0].text, "alert[uk]");
    let g = group("o", "-1", "zz", Pii::None, &[("order.exception", Mode::Now)]);
    assert_eq!(fan_out(&e, &[g], UTC, T0).send[0].text, "alert[en]", "an unknown language falls back to English");
}

#[test]
fn a_customer_message_carries_its_words_only_to_a_full_group() {
    let e = routed("m1".into(), "inbox.message", &json!({ "data": { "channel": "whatsapp", "peer": "Ana", "text": "table for 4?" } }), T0);
    let full = group("o", "-1", "en", Pii::Full, &[("inbox.message", Mode::Now)]);
    let none = group("k", "-2", "en", Pii::None, &[("inbox.message", Mode::Now)]);
    let p = fan_out(&e, &[full, none], UTC, T0);
    assert_eq!(p.send[0].text, "💬 whatsapp · Ana\ntable for 4?");
    assert!(!p.send[1].text.contains("Ana") && !p.send[1].text.contains("table for 4"));
}

#[test]
fn an_event_with_nothing_to_say_sends_nothing() {
    let g = group("o", "-1", "en", Pii::Full, &[("system.alert", Mode::Now)]);
    let e = routed("z".into(), "system.alert", &json!({ "texts": {} }), T0);
    assert!(fan_out(&e, &[g], UTC, T0).send.is_empty());
}

#[test]
fn the_next_summary_is_the_next_such_minute_and_the_weekly_is_a_monday() {
    // Thursday 10:00 UTC; the summary is at 09:00 -> Friday 09:00.
    let now = T0 + 10 * 60 * MIN;
    assert_eq!(next_at(9 * 60, false, UTC, now), T0 + (24 + 9) * 60 * MIN);
    assert_eq!(next_at(11 * 60, false, UTC, now), T0 + 11 * 60 * MIN);
    // Monday 2026-01-19 09:00 UTC.
    assert_eq!(next_at(9 * 60, true, UTC, now), 1_768_780_800_000 + 9 * 60 * MIN);
    // Tirana winter: 09:00 local is 08:00 UTC.
    assert_eq!(next_at(9 * 60, false, TIRANA_WINTER, T0), T0 + 8 * 60 * MIN);
}

#[test]
fn a_group_is_owed_a_summary_when_it_asked_or_put_anything_in_one() {
    let a = group("a", "-1", "en", Pii::None, &[("digest.daily", Mode::Now), ("digest.weekly", Mode::Now)]);
    let b = group("b", "-2", "en", Pii::None, &[("stock.low", Mode::Digest)]);
    let c = group("c", "-3", "en", Pii::None, &[("order.placed", Mode::Now)]);
    let mut d = a.clone();
    d.id = "d".into();
    d.state = State::Left;
    let keys: Vec<String> = dues(&[a, b, c, d], UTC, T0).into_iter().map(|(k, _)| k).collect();
    assert_eq!(keys, vec!["a/daily", "a/weekly", "b/daily"]);
}

#[test]
fn the_catalogue_is_closed_and_orders_ring_through_quiet_hours() {
    assert!(events::get("order.placed").is_some() && events::get("nope").is_none());
    assert!(!events::holds_in_quiet("order.placed"));
    assert!(events::holds_in_quiet("stock.low"));
    assert!(events::holds_in_quiet("unknown.thing"), "unknown waits rather than waking someone");
    let mut keys: Vec<&str> = events::EVENTS.iter().map(|e| e.key).collect();
    keys.dedup();
    assert_eq!(keys.len(), events::EVENTS.len());
}
