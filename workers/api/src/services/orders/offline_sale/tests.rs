//! THE OFFLINE SALE'S RULES, against the real pricer, a real `Hub` and
//! `StockLog` in memory, and the real fiscal queue. Tested in-memory: the
//! live proof is `tools/live-proof/probes/feature-offline-sale.mjs`.

use super::*;
use crate::exceptions::alert::Voice;
use crate::fiscal::queue::{health, DEADLINE_MS};
use crate::fiscal::wire::{at_placement, AtPlacement, Config};
use crate::hubdo::OrderView;
use crate::outbox::Entry;
use crate::services::ordering::pricing::{price_basket, Want};
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::RatePpm;
use dowiz_hub::stock::{StockLevel, StockLog};
use dowiz_hub::Hub;
use serde_json::{json, Value};

const LOC: &str = "loc-qa";
const T0: i64 = 1_791_000_000_000; // 2026-10-03, about
const PARITY: &str = include_str!("parity.json");

fn fixture() -> Value {
    serde_json::from_str(PARITY).expect("parity.json is JSON")
}

fn lookup_of(fx: &Value) -> impl Fn(&str) -> Option<String> + '_ {
    move |id: &str| fx["products"].get(id).map(Value::to_string)
}

// ── row 5: kernel-equal prices, offline vs online, ONE fixture ─────────────

/// The online pricer answers every basket of `parity.json` exactly as the
/// fixture says; `public/room/offline-sale.test.mjs` holds the tablet to the
/// same file, so the two cannot drift apart without one side going red.
#[test]
fn the_online_pricer_answers_the_shared_parity_fixture() {
    let fx = fixture();
    let look = lookup_of(&fx);
    let baskets = fx["baskets"].as_array().expect("baskets");
    assert!(baskets.len() >= 10, "the fixture lost its baskets");
    for b in baskets {
        let lines: Vec<(String, i64)> = b["lines"].as_array().unwrap().iter()
            .map(|l| (l[0].as_str().unwrap().to_string(), l[1].as_i64().unwrap())).collect();
        let got = match price_basket(&look, lines.iter().map(|(p, q)| Want { product_id: p, modifier_ids: &[], quantity: *q })) {
            Ok(basket) => json!({ "ok": true, "units": basket.lines.iter().map(|l| l.unit_price).collect::<Vec<_>>(), "total": basket.subtotal }),
            Err(r) => json!({ "ok": false, "refusal": rules::refusal_kind(&r) }),
        };
        assert_eq!(got, b["expect"], "basket {:?}", b["name"]);
    }
}

// ── the body ────────────────────────────────────────────────────────────────

fn sale(key: &str, lines: &[(&str, i64, i64)], at: i64) -> SaleIn {
    let lines: Vec<LineIn> = lines.iter().map(|(p, q, u)| LineIn { product_id: (*p).into(), quantity: *q, unit_price: *u, name: String::new() }).collect();
    let total = lines.iter().fold(0i64, |a, l| a.wrapping_add(l.quantity.wrapping_mul(l.unit_price)));
    SaleIn { location_id: LOC.into(), sale_key: key.into(), sold_at_ms: at, currency: "ALL".into(), method: "cash".into(), lines, total, menu_version: Some(7) }
}

#[test]
fn a_well_formed_cash_sale_is_accepted() {
    assert_eq!(rules::check(&sale("key-0001", &[("p_water", 2, 150)], T0)), Ok(()));
}

#[test]
fn a_malformed_sale_is_refused_by_name() {
    let mut s = sale("key-0001", &[("p_water", 2, 150)], T0);
    s.total = 299;
    assert!(rules::check(&s).unwrap_err().contains("not the lines' 300"));
    let mut s = sale("key-0001", &[("p_water", 1, 150)], T0);
    s.method = "card".into();
    assert!(rules::check(&s).unwrap_err().contains("cash"));
    assert!(rules::check(&sale("short", &[("p_water", 1, 150)], T0)).is_err(), "a key under 8 characters");
    assert!(rules::check(&sale("key 0001!", &[("p_water", 1, 150)], T0)).is_err(), "a key outside the alphabet");
    assert!(rules::check(&sale("key-0001", &[], T0)).is_err(), "no lines");
    assert!(rules::check(&sale("key-0001", &[("p_refill", 1, 0)], T0)).unwrap_err().contains("takes money"));
    assert!(rules::check(&sale("key-0001", &[("p_water", 100, 150)], T0)).is_err(), "quantity 100");
    assert!(rules::check(&sale("key-0001", &[("p_water", 1, -1)], T0)).is_err(), "a negative price");
    assert!(rules::check(&sale("key-0001", &[("p_water", 99, i64::MAX / 50)], T0)).unwrap_err().contains("overflows"));
    let mut s = sale("key-0001", &[("p_water", 1, 150)], T0);
    s.lines[0].name = "x".repeat(121);
    assert!(rules::check(&s).unwrap_err().contains("120"), "a name is bounded before it is stored");
    let mut s = sale("key-0001", &[("p_water", 1, 150)], T0);
    s.currency = "lek".into();
    assert!(rules::check(&s).is_err(), "a currency that is not a code");
}

#[test]
fn the_tablet_clock_is_believed_inside_the_window_and_replaced_outside_it() {
    assert_eq!(rules::when(T0 - 3_600_000, T0), (T0 - 3_600_000, None), "an hour offline");
    assert_eq!(rules::when(T0 + 60_000, T0), (T0, None), "a minute ahead is the same instant, never after the sync");
    let (at, c) = rules::when(T0 + rules::AHEAD_MS + 1, T0);
    assert_eq!((at, c.unwrap().kind.as_str()), (T0, "clock"));
    assert_eq!(rules::when(T0 - rules::MAX_OFFLINE_MS, T0).1, None, "exactly a week is still believed");
    let (at, c) = rules::when(T0 - rules::MAX_OFFLINE_MS - 1, T0);
    assert_eq!((at, c.unwrap().kind.as_str()), (T0, "clock"));
}

// ── the envelope: the sale keeps the price the guest paid ──────────────────

#[test]
fn a_price_moved_while_offline_keeps_the_paid_price_and_is_flagged() {
    let fx = fixture();
    let s = sale("key-0002", &[("p_miso", 2, 350), ("p_nigiri", 1, 700), ("p_water", 1, 150)], T0);
    let env = rules::envelope(&s, "staff-1", T0, T0 + 5_000, &lookup_of(&fx), vec![]);
    let units: Vec<i64> = env["items"].as_array().unwrap().iter().map(|i| i["unit_price"].as_i64().unwrap()).collect();
    assert_eq!(units, vec![350, 700, 150], "history: the paid prices stand");
    let kinds: Vec<(&str, &str)> = env["offline"]["conflicts"].as_array().unwrap().iter()
        .map(|c| (c["kind"].as_str().unwrap(), c["product_id"].as_str().unwrap())).collect();
    assert_eq!(kinds, vec![("price_changed", "p_miso"), ("off_sale", "p_nigiri")]);
    assert_eq!(env["offline"]["conflicts"][0]["now"], 400);
    // LAW 3 and the till's payment.
    assert_eq!((env["total"].as_i64(), env["subtotal"].as_i64(), env["discount"].as_i64()), (Some(1550), Some(1550), Some(0)));
    assert_eq!(env["payments"], json!([{ "by": "staff-1", "amount": 1550, "method": "cash", "currency": "ALL", "at": T0 }]));
    assert_eq!((env["status"].as_str(), env["payment_status"].as_str()), (Some("PICKED_UP"), Some("paid")));
    assert_eq!(env["id"], "offline:key-0002");
    assert_eq!(env["offline"]["fiscal_deadline_ms"], T0 + DEADLINE_MS);
    assert!(crate::services::ordering::channel::of(&env).is_ok());
}

#[test]
fn an_unchanged_basket_syncs_with_no_conflict() {
    let fx = fixture();
    let env = rules::envelope(&sale("key-0003", &[("p_water", 1, 150)], T0), "staff-1", T0, T0, &lookup_of(&fx), vec![]);
    assert_eq!(env["offline"]["conflicts"], json!([]));
    assert_eq!(env["items"][0]["name"], "QA Water");
}

// ── the object's turn: replay twice = one sale; two tablets; the shelf ─────

const ROLL: &str = r#"{"id":"p-roll","name":"Sake roll","price":250,"available":true,"bom":[{"supply":"salmon","qty":40}]}"#;
const TAX: VenueTax = VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

struct Venue {
    hub: Hub,
    stock: StockLog,
}

impl Venue {
    fn new() -> Venue {
        Venue { hub: Hub::create_sized(64 * 1024).unwrap(), stock: StockLog::create_sized(64 * 1024).unwrap() }
    }
    fn listed(&self) -> Vec<OrderView> {
        crate::hubstore::orders_state(&self.hub).into_iter().map(OrderView::of_event).collect()
    }
    fn sync(&mut self, s: &SaleIn, tax: Result<Option<VenueTax>, String>, now: i64) -> sync::Decided {
        let look = |id: &str| (id == "p-roll").then(|| ROLL.to_string());
        let env = rules::envelope(s, "staff-1", s.sold_at_ms, now, &look, vec![]);
        let input = SyncIn {
            order_id: rules::order_id(&s.sale_key), envelope: env,
            bom_lines: rules::bom_lines(s, &look), sold_at_ms: s.sold_at_ms, now_ms: now,
        };
        let listed = self.listed();
        self.stock.set_clock(now);
        sync::decide(&mut self.hub, &mut self.stock, &listed, &tax, "ALL", &input).expect("synced")
    }
}

#[test]
fn a_sale_replayed_twice_is_one_sale() {
    let mut v = Venue::new();
    let s = sale("key-0004", &[("p-roll", 2, 250)], T0);
    let first = v.sync(&s, Ok(Some(TAX)), T0 + 1_000);
    assert!(!first.replayed);
    let events = v.hub.len();
    let second = v.sync(&s, Ok(Some(TAX)), T0 + 9_000);
    assert!(second.replayed, "the replay found its own order");
    assert_eq!(v.hub.len(), events, "nothing was appended");
    assert_eq!(v.listed().len(), 1);
    assert_eq!(second.stored, v.listed()[0].order_json);
}

#[test]
fn two_tablets_offline_at_once_both_sync_without_loss() {
    let mut v = Venue::new();
    v.sync(&sale("tablet-a-0001", &[("p-roll", 1, 250)], T0), Ok(None), T0 + 2_000);
    v.sync(&sale("tablet-b-0001", &[("p-roll", 3, 250)], T0 + 500), Ok(None), T0 + 3_000);
    let mut ids: Vec<String> = v.listed().into_iter().map(|o| o.order_id).collect();
    ids.sort();
    assert_eq!(ids, vec!["offline:tablet-a-0001", "offline:tablet-b-0001"]);
}

#[test]
fn the_shelf_moves_by_the_recipe_at_the_sync_and_a_short_shelf_never_refuses() {
    let mut v = Venue::new();
    v.sync(&sale("key-0005", &[("p-roll", 2, 250)], T0), Ok(None), T0 + 1_000);
    let led = v.stock.ledger().unwrap();
    assert_eq!(led.level("salmon"), StockLevel { on_hand: -80, reserved: 0 }, "2 rolls x 40 g served from an empty shelf: drawn, not refused");
}

#[test]
fn the_sale_is_stamped_inclusive_with_the_order_time_and_its_seq() {
    let mut v = Venue::new();
    let d = v.sync(&sale("key-0006", &[("p-roll", 2, 250)], T0 - 7_200_000), Ok(Some(TAX)), T0);
    let o: Value = serde_json::from_str(&d.stored).unwrap();
    assert_eq!(o["total"], 500, "the stamp did not add tax to what the guest paid");
    assert_eq!(o["tax"]["groups"][0]["rate_ppm"], 200_000);
    assert_eq!(o["created_at_ms"], T0 - 7_200_000, "the original time");
    assert_eq!(v.listed()[0].seq, (T0 - 7_200_000) as u64);
    let excl = VenueTax { inclusive: false, ..TAX };
    let d = v.sync(&sale("key-0007", &[("p-roll", 1, 250)], T0), Ok(Some(excl)), T0);
    let o: Value = serde_json::from_str(&d.stored).unwrap();
    assert_eq!(o["total"], 250);
    assert!(d.conflicts.iter().any(|c| c.kind == "tax"), "an exclusive venue is said, not silently re-totalled");
}

#[test]
fn an_order_id_that_is_not_the_sales_own_is_refused() {
    let mut v = Venue::new();
    let s = sale("key-0008", &[("p-roll", 1, 250)], T0);
    let env = rules::envelope(&s, "staff-1", T0, T0, &|_| None, vec![]);
    let input = SyncIn { order_id: "ord-1".into(), envelope: env, bom_lines: vec![], sold_at_ms: T0, now_ms: T0 };
    assert!(sync::decide(&mut v.hub, &mut v.stock, &[], &Ok(None), "ALL", &input).is_err());
}

#[test]
fn the_cash_goes_into_the_drawer_open_at_the_sale_and_none_when_it_was_closed() {
    let p: crate::command::till::Period = serde_json::from_value(json!({
        "till_id": "main", "opened_at": T0 - 3_600_000, "opened_by": "s1", "float": {}, "pay_in": {}, "pay_out": {},
        "closed_at": T0 - 60_000 })).expect("a period");
    assert_eq!(sync::till_at(std::slice::from_ref(&p), T0 - 120_000), Some("main".into()));
    assert_eq!(sync::till_at(std::slice::from_ref(&p), T0), None, "after the close: no drawer, never a guessed one");
    let fx = fixture();
    let mut env = rules::envelope(&sale("key-0014", &[("p_water", 1, 150)], T0), "s1", T0, T0, &lookup_of(&fx), vec![]);
    sync::into_till(&mut env, Some("main".into()));
    assert_eq!(env["payments"][0]["till_id"], "main");
    let mut bare = rules::envelope(&sale("key-0015", &[("p_water", 1, 150)], T0), "s1", T0, T0, &lookup_of(&fx), vec![]);
    sync::into_till(&mut bare, None);
    assert!(bare["payments"][0].get("till_id").is_none());
}

// ── row 2: the fiscal queue, 48 h from the SALE, at the edge ───────────────

fn queued(v: &mut Venue, key: &str, sold_at: i64, now: i64) -> Entry {
    let d = v.sync(&sale(key, &[("p-roll", 1, 250)], sold_at), Ok(Some(TAX)), now);
    match at_placement(&Config::From(0), &serde_json::from_str(&d.stored).unwrap(), "ALL", sold_at) {
        AtPlacement::Queued(e) => e,
        other => panic!("not queued: {other:?}"),
    }
}

#[test]
fn the_fiscal_deadline_is_48_hours_from_the_sale_at_the_edge() {
    let mut v = Venue::new();
    let sold = T0 - 3 * 3600 * 1000; // synced three hours after the sale
    let e = queued(&mut v, "key-0009", sold, T0);
    assert_eq!(e.queued_at_ms, sold, "issued at the sale, not at the sync");
    assert_eq!(rules::deadline(sold), sold + 172_800_000);
    let h = health(std::slice::from_ref(&e), sold + DEADLINE_MS - 1);
    assert_eq!((h.backlog, h.overdue.len(), h.first_deadline), (1, 0, Some(sold + 172_800_000)));
    assert_eq!(health(std::slice::from_ref(&e), sold + DEADLINE_MS).overdue.len(), 1, "AT the deadline it has passed");
}

// ── row 2: an overdue sale alerts ONCE ─────────────────────────────────────

fn voice() -> Voice<'static> {
    Voice { venue: LOC, zone: dowiz_hub::tz::DEFAULT, lang: "sq" }
}

#[test]
fn an_overdue_offline_sale_alerts_once() {
    let mut v = Venue::new();
    let e = queued(&mut v, "key-0010", T0, T0);
    let entries = vec![e.clone()];
    let none = |_: &str| false;
    assert_eq!(overdue::next_alert(&entries, &none), Some(T0 + DEADLINE_MS));
    let (early, marks) = overdue::due(&entries, &none, T0 + DEADLINE_MS - 1, &voice(), "-100123");
    assert!(early.is_empty() && marks.is_empty(), "before the deadline: nothing");
    let (due, marks) = overdue::due(&entries, &none, T0 + DEADLINE_MS, &voice(), "-100123");
    assert_eq!((due.len(), marks.clone()), (1, vec![e.id.clone()]));
    assert_eq!(due[0].kind, "telegram");
    assert!(due[0].text.contains("48 orë"), "the venue's own language: {}", due[0].text);
    let marked = |id: &str| marks.iter().any(|m| m == id);
    let (again, more) = overdue::due(&entries, &marked, T0 + DEADLINE_MS + 3_600_000, &voice(), "-100123");
    assert!(again.is_empty() && more.is_empty(), "once");
    assert_eq!(overdue::next_alert(&entries, &marked), None, "the alarm is not re-armed for it");
}

#[test]
fn with_no_alert_chat_the_overdue_sale_is_still_marked_once() {
    let mut v = Venue::new();
    let entries = vec![queued(&mut v, "key-0011", T0, T0)];
    let (due, marks) = overdue::due(&entries, &|_| false, T0 + DEADLINE_MS, &voice(), " ");
    assert!(due.is_empty());
    assert_eq!(marks.len(), 1, "marked, or the alarm would wake every minute for ever");
}

#[test]
fn only_offline_sales_are_alerted_and_the_text_speaks_four_languages() {
    let other = Entry::new("u1".into(), crate::fiscal::queue::KIND, "ord-9".into(), "{}".into(), T0);
    assert!(overdue::offline(&[other]).is_empty());
    let ids = vec!["offline:abcdefgh-1".to_string()];
    for (l, w) in [("sq", "NIVF"), ("en", "deadline"), ("uk", "строк"), ("ru", "срок")] {
        assert!(overdue::text(l, &ids).contains(w) && overdue::text(l, &ids).contains("abcdefgh"), "{l}");
    }
}

// ── the owner's pane ───────────────────────────────────────────────────────

#[test]
fn the_pane_counts_names_the_oldest_and_marks_the_overdue_red() {
    let mut v = Venue::new();
    let a = queued(&mut v, "key-0012", T0 - DEADLINE_MS - 1, T0);
    let _ = queued(&mut v, "key-0013", T0 - 1_000, T0);
    let orders: Vec<Value> = v.listed().iter().map(|o| serde_json::from_str(&o.order_json).unwrap()).collect();
    let marked = |id: &str| id == a.id;
    let p = ledger::pane(&orders, &[a.clone()], &marked, T0, false);
    assert_eq!((p.count, p.oldest_sold_at_ms, p.overdue, p.send_enabled), (2, Some(T0 - DEADLINE_MS - 1), 1, false));
    assert_eq!((p.items[0].fiscal, p.items[0].overdue, p.items[0].alerted), ("queued", true, true));
    assert_eq!((p.items[1].fiscal, p.items[1].overdue), ("not_queued", false), "its own deadline still stands");
}

// ── row 5: SEND_ENABLED=false holds, and the queue path has no network ─────

#[test]
fn sending_stays_off_and_nothing_on_the_offline_path_can_reach_the_network() {
    assert!(!crate::fiscal::SEND_ENABLED, "operator 2026-09-24: import-only; do not flip it here");
    let sources = [
        include_str!("../offline_sale.rs"), include_str!("rules.rs"), include_str!("sync.rs"),
        include_str!("overdue.rs"), include_str!("ledger.rs"), include_str!("handler.rs"),
        include_str!("../../../hubdo/room/offline.rs"),
    ];
    for (i, src) in sources.iter().enumerate() {
        let code: String = src.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        for word in ["Fetch", "fetch(", "Transport", "ebills_fire", "venue_minute", "fiscal_plan", "ebills::fetch"] {
            assert!(!code.contains(word), "source {i} names {word}: the offline path must not reach the network");
        }
    }
}
