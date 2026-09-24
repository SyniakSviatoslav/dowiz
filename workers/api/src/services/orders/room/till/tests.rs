//! The till's door: the body a person sends, and what a person is shown.

use super::*;
use crate::command::till::{self, CashIn, CountIn, Money, OpenIn, COUNTED, OPENED, PAY_IN};
use dowiz_hub::logimage::Entry;

const T0: i64 = 1_790_000_000_000;

fn money(pairs: &[(&str, i64)]) -> Money {
    pairs.iter().map(|(c, n)| (c.to_string(), *n)).collect()
}

/// Fold a till through the real decide and fold, then build the object's
/// answer for its last record — the exact value `room/till.rs` returns.
fn out_after(cmds: &[Cmd], cash: &[CashIn]) -> TillOut {
    let mut log: Vec<Entry> = Vec::new();
    let mut kind = "";
    for c in cmds {
        let (k, subject, rec) = till::decide(&till::periods(&log).unwrap(), cash, c).expect("accepted");
        log.push(Entry { kind: k.into(), subject, json: rec.to_string(), seq: log.len() as u64 });
        kind = k;
    }
    let ps = till::periods(&log).unwrap();
    let period = till::report(&ps[ps.len() - 1..], cash).unwrap().periods.remove(0);
    TillOut { kind: kind.into(), till_id: "main".into(), period, generation: 1 }
}

fn opened() -> Cmd {
    Cmd::Open(OpenIn { location_id: "v1".into(), till_id: "main".into(), float: money(&[("ALL", 10_000)]), by: "p1".into(), now_ms: T0 })
}

fn counted(n: i64) -> Cmd {
    Cmd::Count(CountIn { location_id: "v1".into(), till_id: "main".into(), observed: money(&[("ALL", n)]), by: "p1".into(), now_ms: T0 + 20 })
}

fn closed() -> Cmd {
    command(Verb::Close, json!({"location_id": "v1"}), "p1", T0 + 30).unwrap()
}

const CASH: [CashIn; 0] = [];

/// THE BLIND COUNT: the count's answer carries what was entered and nothing
/// to compare it with — and neither does any other answer before the close.
#[test]
fn a_count_answer_carries_no_expected_figure() {
    let out = out_after(&[opened(), counted(9_900)], &CASH);
    assert_eq!(out.kind, COUNTED);
    assert_eq!(out.period.expected, money(&[("ALL", 10_000)]), "the object knows it");
    let shown = answer(&out);
    for hidden in ["expected", "over_short", "cash_paid", "pay_in", "pay_out"] {
        assert!(shown.get(hidden).is_none(), "{hidden} leaked into a blind count: {shown}");
    }
    assert_eq!(shown["counted"], json!({"ALL": 9_900}));
}

#[test]
fn no_answer_before_the_close_carries_the_expected_figure() {
    let pay_in = command(Verb::PayIn, json!({"location_id": "v1", "currency": "ALL", "amount": 1, "reason": "peek"}), "p1", T0 + 5).unwrap();
    for (cmds, kind) in [(vec![opened()], OPENED), (vec![opened(), pay_in], PAY_IN)] {
        let shown = answer(&out_after(&cmds, &CASH));
        assert_eq!(shown["kind"], json!(kind));
        assert!(shown.get("expected").is_none() && shown.get("over_short").is_none(), "{kind}: {shown}");
    }
}

/// THE POSITIVE TWIN: the close is the Z report and says it all.
#[test]
fn the_close_answer_carries_expected_and_over_short() {
    let cash = [CashIn { order_id: "r1".into(), at: T0 + 10, currency: "ALL".into(), amount: 1_500 }];
    let shown = answer(&out_after(&[opened(), counted(11_400), closed()], &cash));
    assert_eq!(shown["expected"], json!({"ALL": 11_500}));
    assert_eq!(shown["over_short"], json!({"ALL": -100}));
    assert_eq!(shown["cash_paid"], json!({"ALL": 1_500}));
    assert_eq!(shown["open"], json!(false));
}

/// The signer and the clock are the Worker's; a body that claims either is
/// overwritten, and a body with no till names the venue's one drawer.
#[test]
fn the_body_cannot_sign_or_date_a_command() {
    let body = json!({"location_id": "v1", "by": "someone-else", "now_ms": 1, "observed": {"ALL": 5}});
    let Cmd::Count(c) = command(Verb::Count, body, "p1", T0).unwrap() else { unreachable!("a count") };
    assert_eq!((c.by.as_str(), c.now_ms, c.till_id.as_str()), ("p1", T0, DEFAULT_TILL));
    let Cmd::Count(c) = command(Verb::Count, json!({"location_id": "v1", "till_id": "bar", "observed": {}}), "p1", T0).unwrap() else {
        unreachable!("a count")
    };
    assert_eq!(c.till_id, "bar", "a named till is kept");
}

#[test]
fn a_body_missing_its_fields_is_refused_and_a_whole_one_is_a_command() {
    assert!(command(Verb::PayOut, json!({"location_id": "v1", "amount": 5}), "p1", T0).is_err(), "no currency, no reason");
    assert!(command(Verb::Count, json!("not an object"), "p1", T0).is_err());
    assert!(matches!(
        command(Verb::PayOut, json!({"location_id": "v1", "currency": "EUR", "amount": 5, "reason": "ice"}), "p1", T0),
        Ok(Cmd::PayOut(_))
    ));
}

#[test]
fn every_verb_has_its_own_idempotency_route() {
    let all = [Verb::Open, Verb::Count, Verb::Close, Verb::PayIn, Verb::PayOut];
    let routes: std::collections::BTreeSet<_> = all.iter().map(|v| v.route()).collect();
    assert_eq!(routes.len(), all.len());
    assert!(routes.iter().all(|r| r.starts_with("staff.till_")));
}

#[test]
fn a_tips_period_is_the_drawers_and_an_open_till_ends_now() {
    const DAY: i64 = 50;
    assert_eq!(tips_period(Some("100"), Some("200"), T0, DAY), Ok((100, 200)));
    assert_eq!(tips_period(Some("100"), None, T0, DAY), Ok((100, T0)), "an open till reads up to now");
    assert!(tips_period(Some("x"), None, T0, DAY).is_err());
    assert!(tips_period(Some("100"), Some("y"), T0, DAY).is_err());
    assert!(tips_period(Some("300"), Some("200"), T0, DAY).is_err(), "ends before it starts");
    assert_eq!(tips_period(Some("200"), Some("200"), T0, DAY), Ok((200, 200)), "an instant is a period");
}

/// NO TILL, STILL TIPS (live walk 2026-09-24): a card-only day opens no drawer,
/// and the phone that asks names no start. The period is then the venue's day
/// so far -- never a refusal, which left the day's tips unreachable.
#[test]
fn with_no_drawer_the_tips_period_is_the_venues_day() {
    let today = dowiz_hub::tz::start_of_local_day_ms(dowiz_hub::tz::from_settings(Some("Europe/Tirane"), None), T0);
    assert!(today <= T0 && T0 - today < 25 * 3_600_000);
    assert_eq!(tips_period(None, None, T0, today), Ok((today, T0)));
    assert_eq!(tips_period(None, Some("200"), T0, 100), Ok((100, 200)));
    assert!(tips_period(None, Some("50"), T0, 100).is_err(), "still: ends before it starts");
}
