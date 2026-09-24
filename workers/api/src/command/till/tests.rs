//! The till's refusals, each beside its positive twin, and the fold across
//! the till log and the order log (BLUEPRINT-POS-THE-ROOM §2.5, G2).

use super::*;
use dowiz_hub::logimage::Entry;

const T0: i64 = 1_790_000_000_000;

fn money(pairs: &[(&str, i64)]) -> Money {
    pairs.iter().map(|(c, n)| (c.to_string(), *n)).collect()
}

fn open(float: &[(&str, i64)], at: i64) -> Cmd {
    Cmd::Open(OpenIn { location_id: "v1".into(), till_id: "main".into(), float: money(float), by: "p1".into(), now_ms: at })
}

fn mv(pay_in: bool, cur: &str, amount: i64, at: i64) -> Cmd {
    let i = MoveIn {
        location_id: "v1".into(),
        till_id: "main".into(),
        currency: cur.into(),
        amount,
        reason: "change from the bank".into(),
        source: None,
        by: "p1".into(),
        now_ms: at,
    };
    if pay_in { Cmd::PayIn(i) } else { Cmd::PayOut(i) }
}

fn count(observed: &[(&str, i64)], at: i64) -> Cmd {
    Cmd::Count(CountIn { location_id: "v1".into(), till_id: "main".into(), observed: money(observed), by: "p1".into(), now_ms: at })
}

fn close(at: i64) -> Cmd {
    Cmd::Close(CloseIn { location_id: "v1".into(), till_id: "main".into(), by: "p1".into(), now_ms: at })
}

fn cash(order: &str, at: i64, cur: &str, amount: i64) -> CashIn {
    CashIn { order_id: order.into(), at, currency: cur.into(), amount }
}

/// Run commands through `decide` and the fold, exactly as the object does:
/// each record appended only if decide accepted it.
fn run(cmds: &[Cmd], cash: &[CashIn]) -> (Vec<Entry>, Vec<Result<(), Refused>>) {
    let mut log: Vec<Entry> = Vec::new();
    let mut said = Vec::new();
    for c in cmds {
        let ps = periods(&log).expect("folds");
        match decide(&ps, cash, c) {
            Ok((kind, subject, rec)) => {
                log.push(Entry { kind: kind.into(), subject, json: rec.to_string(), seq: log.len() as u64 });
                said.push(Ok(()));
            }
            Err(r) => said.push(Err(r)),
        }
    }
    (log, said)
}

fn last(said: &[Result<(), Refused>]) -> &Result<(), Refused> {
    said.last().expect("a command ran")
}

#[test]
fn a_till_opens_with_a_float_in_two_currencies() {
    let (log, said) = run(&[open(&[("ALL", 10_000), ("EUR", 5_000)], T0)], &[]);
    assert!(said[0].is_ok());
    let ps = periods(&log).unwrap();
    assert_eq!(ps[0].float, money(&[("ALL", 10_000), ("EUR", 5_000)]));
    assert_eq!(ps[0].closed_at, None);
}

#[test]
fn a_second_open_while_one_is_open_is_refused_and_after_a_close_it_lands() {
    let (_, said) = run(&[open(&[], T0), open(&[], T0 + 1)], &[]);
    assert!(matches!(last(&said), Err(Refused::Conflict(_))));
    let (_, said) = run(&[open(&[], T0), count(&[], T0 + 1), close(T0 + 2), open(&[], T0 + 3)], &[]);
    assert!(said.iter().all(Result::is_ok), "{said:?}");
}

#[test]
fn a_float_in_an_unknown_currency_or_negative_is_invalid_and_a_zero_float_lands() {
    let (_, said) = run(&[open(&[("XYZ", 100)], T0)], &[]);
    assert!(matches!(last(&said), Err(Refused::Invalid(_))));
    let (_, said) = run(&[open(&[("ALL", -1)], T0)], &[]);
    assert!(matches!(last(&said), Err(Refused::Invalid(_))));
    let (_, said) = run(&[open(&[("ALL", 0)], T0)], &[]);
    assert!(last(&said).is_ok());
}

#[test]
fn an_unsigned_event_is_invalid_and_a_signed_one_lands() {
    let mut c = open(&[], T0);
    if let Cmd::Open(i) = &mut c {
        i.by = "  ".into();
    }
    assert!(matches!(decide(&[], &[], &c), Err(Refused::Invalid(_))));
    assert!(decide(&[], &[], &open(&[], T0)).is_ok());
}

#[test]
fn a_till_id_with_odd_bytes_is_invalid_and_a_plain_one_lands() {
    let mut c = open(&[], T0);
    if let Cmd::Open(i) = &mut c {
        i.till_id = "a\"b".into();
    }
    assert!(matches!(decide(&[], &[], &c), Err(Refused::Invalid(_))));
    assert!(decide(&[], &[], &open(&[], T0)).is_ok());
}

#[test]
fn money_moved_with_no_till_open_is_refused_and_with_one_open_lands() {
    let (_, said) = run(&[mv(true, "ALL", 500, T0)], &[]);
    assert!(matches!(last(&said), Err(Refused::Conflict(_))));
    let (log, said) = run(&[open(&[], T0), mv(true, "ALL", 500, T0 + 1), mv(false, "EUR", 200, T0 + 2)], &[]);
    assert!(said.iter().all(Result::is_ok));
    let p = &periods(&log).unwrap()[0];
    assert_eq!(p.pay_in, money(&[("ALL", 500)]));
    assert_eq!(p.pay_out, money(&[("EUR", 200)]));
}

#[test]
fn a_pay_in_of_zero_without_a_reason_or_in_no_currency_is_invalid() {
    let (_, said) = run(&[open(&[], T0), mv(true, "ALL", 0, T0 + 1)], &[]);
    assert!(matches!(last(&said), Err(Refused::Invalid(_))));
    let (_, said) = run(&[open(&[], T0), mv(true, "GBP", 10, T0 + 1)], &[]);
    assert!(matches!(last(&said), Err(Refused::Invalid(_))));
    let mut c = mv(false, "ALL", 10, T0 + 1);
    if let Cmd::PayOut(i) = &mut c {
        i.reason = String::new();
    }
    let (_, said) = run(&[open(&[], T0), c], &[]);
    assert!(matches!(last(&said), Err(Refused::Invalid(_))));
    let (_, said) = run(&[open(&[], T0), mv(true, "ALL", 1, T0 + 1)], &[]);
    assert!(last(&said).is_ok(), "the positive twin: 1 lek with a reason lands");
}

#[test]
fn an_event_for_a_till_that_is_not_the_open_one_is_refused() {
    let mut c = count(&[], T0 + 1);
    if let Cmd::Count(i) = &mut c {
        i.till_id = "bar".into();
    }
    let (_, said) = run(&[open(&[], T0), c], &[]);
    assert!(matches!(last(&said), Err(Refused::Conflict(_))));
    let (_, said) = run(&[open(&[], T0), count(&[], T0 + 1)], &[]);
    assert!(last(&said).is_ok());
}

#[test]
fn closing_an_uncounted_drawer_is_refused_and_a_counted_one_closes() {
    let (_, said) = run(&[open(&[], T0), close(T0 + 1)], &[]);
    assert!(matches!(last(&said), Err(Refused::Conflict(_))));
    let (_, said) = run(&[close(T0)], &[]);
    assert!(matches!(last(&said), Err(Refused::Conflict(_))), "nothing open to close");
    let (_, said) = run(&[open(&[], T0), count(&[], T0 + 1), close(T0 + 2)], &[]);
    assert!(last(&said).is_ok());
}

/// THE EQUATION, per currency: a lek pile and a euro pile, cash landing in
/// the pile it was PAID in, and the close recording counted − expected.
#[test]
fn the_close_records_over_short_per_currency() {
    let paid = [cash("r1", T0 + 10, "ALL", 1_500), cash("r2", T0 + 20, "EUR", 2_000)];
    let (log, said) = run(
        &[
            open(&[("ALL", 10_000), ("EUR", 5_000)], T0),
            mv(true, "ALL", 4_500, T0 + 30),
            mv(false, "EUR", 1_000, T0 + 40),
            count(&[("ALL", 15_900), ("EUR", 6_000)], T0 + 50),
            close(T0 + 60),
        ],
        &paid,
    );
    assert!(said.iter().all(Result::is_ok), "{said:?}");
    let p = &periods(&log).unwrap()[0];
    // ALL: 10000 + 1500 + 4500 = 16000, counted 15900 → −100.
    // EUR: 5000 + 2000 − 1000 = 6000, counted 6000 → 0.
    assert_eq!(expected(p, &paid).unwrap(), money(&[("ALL", 16_000), ("EUR", 6_000)]));
    assert_eq!(p.over_short, Some(money(&[("ALL", -100), ("EUR", 0)])));
}

/// A pile expected and not counted at all is short by all of it — a missing
/// currency in the count is not "nothing to say".
#[test]
fn a_currency_left_out_of_the_count_is_short_by_all_of_it() {
    let paid = [cash("r1", T0 + 10, "EUR", 2_000)];
    let (log, _) = run(&[open(&[("ALL", 1_000)], T0), count(&[("ALL", 1_000)], T0 + 20), close(T0 + 30)], &paid);
    assert_eq!(periods(&log).unwrap()[0].over_short, Some(money(&[("ALL", 0), ("EUR", -2_000)])));
}

#[test]
fn cash_outside_every_period_is_reported_and_cash_inside_is_not() {
    let paid = [cash("early", T0 - 5, "ALL", 700), cash("in", T0 + 5, "ALL", 300), cash("late", T0 + 99, "ALL", 1)];
    let (log, _) = run(&[open(&[], T0), count(&[("ALL", 300)], T0 + 10), close(T0 + 20)], &paid);
    let r = report(&periods(&log).unwrap(), &paid).unwrap();
    let names: Vec<_> = r.outside.iter().map(|c| c.order_id.as_str()).collect();
    assert_eq!(names, ["early", "late"]);
    assert!(!r.open);
    assert_eq!(r.periods[0].cash_paid, money(&[("ALL", 300)]));
    assert_eq!(r.periods[0].period.over_short, Some(money(&[("ALL", 0)])));
}

#[test]
fn cash_payments_are_read_from_the_orders_in_the_currency_they_were_paid_in() {
    let orders = [
        serde_json::json!({"id": "r1", "location_id": "v1", "payments": [
            {"method": "cash", "amount": 2000, "currency": "EUR", "amount_in_order_currency": 1950, "at": T0},
            {"method": "card", "amount": 500, "at": T0},
            {"method": "cash", "amount": 300, "at": T0 + 1},
        ]}),
        serde_json::json!({"id": "x", "location_id": "other", "payments": [{"method": "cash", "amount": 9, "at": T0}]}),
    ];
    let c = cash_payments(&orders, "v1", "ALL");
    assert_eq!(c, vec![cash("r1", T0, "EUR", 2000), cash("r1", T0 + 1, "ALL", 300)]);
}

#[test]
fn an_unreadable_till_record_is_loud_not_skipped() {
    let bad = [Entry { kind: "till.pay_out".into(), subject: "main".into(), json: r#"{"at":1}"#.into(), seq: 0 }];
    assert!(periods(&bad).is_err(), "a pay-out with no till open is an error, never a skipped line");
    let odd = [
        Entry { kind: OPENED.into(), subject: "main".into(), json: r#"{"float":{},"by":"p","at":1}"#.into(), seq: 0 },
        Entry { kind: "till.bribe".into(), subject: "main".into(), json: r#"{"by":"p","at":2}"#.into(), seq: 1 },
    ];
    assert!(periods(&odd).is_err());
    assert!(periods(&odd[..1]).is_ok(), "the positive twin: the opened record alone folds");
}

/// THE SHAPE LAW 10 READS (`e2e/gates/conservation.mjs`): the report's JSON
/// carries every key the gate folds, flat on each period. A rename here
/// without the gate would make the law silently skip every period.
#[test]
fn the_report_serialises_the_keys_law_10_reads() {
    let paid = [cash("r1", T0 + 10, "EUR", 2_000)];
    let (log, _) = run(&[open(&[("ALL", 100)], T0), count(&[("ALL", 100), ("EUR", 2_000)], T0 + 20), close(T0 + 30)], &paid);
    let v = serde_json::to_value(report(&periods(&log).unwrap(), &paid).unwrap()).unwrap();
    assert_eq!(v["open"], serde_json::json!(false));
    assert!(v["outside"].is_array());
    let p = &v["periods"][0];
    for k in ["till_id", "opened_at", "closed_at", "float", "cash_paid", "pay_in", "pay_out", "counted", "over_short"] {
        assert!(p.get(k).is_some(), "period lacks {k}: {p}");
    }
    assert_eq!(p["cash_paid"], serde_json::json!({"EUR": 2000}));
    assert_eq!(p["over_short"], serde_json::json!({"ALL": 0, "EUR": 0}));
}

/// D31 (G5): THE Z REPORT CLOSES AGAINST A STALE COUNT. The count was taken at
/// +20, a 1 500 sale went into the drawer at +25, and the close at +30 used
/// the +20 count against `expected` up to +30: a false 1 500 shortfall.
#[test]
fn a_close_after_cash_moved_since_the_count_is_refused() {
    let paid = [cash("r1", T0 + 25, "ALL", 1_500)];
    let (_, said) = run(&[open(&[("ALL", 1_000)], T0), count(&[("ALL", 1_000)], T0 + 20), close(T0 + 30)], &paid);
    assert!(matches!(last(&said), Err(Refused::Conflict(m)) if m.contains("count")), "{said:?}");
    // A pay-out after the count is cash moving too.
    let (_, said) = run(
        &[open(&[("ALL", 1_000)], T0), count(&[("ALL", 1_000)], T0 + 20), mv(false, "ALL", 100, T0 + 25), close(T0 + 30)],
        &[],
    );
    assert!(matches!(last(&said), Err(Refused::Conflict(_))), "{said:?}");
}

/// The twin: count again after the sale and the close lands, even. A sale
/// AFTER the close's own instant is not in this period and does not block it.
#[test]
fn a_close_after_a_fresh_count_lands() {
    let paid = [cash("r1", T0 + 25, "ALL", 1_500), cash("r9", T0 + 99, "ALL", 7)];
    let (log, said) = run(
        &[open(&[("ALL", 1_000)], T0), count(&[("ALL", 1_000)], T0 + 20), count(&[("ALL", 2_500)], T0 + 26), close(T0 + 30)],
        &paid,
    );
    assert!(said.iter().all(Result::is_ok), "{said:?}");
    assert_eq!(periods(&log).unwrap()[0].over_short, Some(money(&[("ALL", 0)])));
}

/// D12 (G5): cash handed back on a completed refund leaves the drawer, per
/// currency pile, at the instant it was handed back. Before, the drawer
/// "expected" the refunded 2 000 EUR and closed short by exactly that.
#[test]
fn cash_handed_back_on_a_refund_leaves_expected_in_its_own_pile() {
    let orders = [serde_json::json!({"id": "r1", "location_id": "v1", "status": "COMPENSATED_REFUND",
        "payments": [
            {"method": "cash", "amount": 2000, "currency": "EUR", "amount_in_order_currency": 1950, "at": T0 + 5},
            {"method": "card", "amount": 500, "at": T0 + 5},
        ],
        "refund": {"owed": 2450, "returned": {"by": "p1", "at": T0 + 15}}})];
    let c = cash_payments(&orders, "v1", "ALL");
    assert_eq!(c, vec![cash("r1", T0 + 5, "EUR", 2000), cash("r1", T0 + 15, "EUR", -2000)]);
    let (log, said) = run(&[open(&[("ALL", 100)], T0), count(&[("ALL", 100)], T0 + 20), close(T0 + 30)], &c);
    assert!(said.iter().all(Result::is_ok), "{said:?}");
    assert_eq!(periods(&log).unwrap()[0].over_short, Some(money(&[("ALL", 0), ("EUR", 0)])));
}

/// The twin: a refund not yet handed back (REFUNDING, no `returned`) is still
/// in the drawer, and the till expects it.
#[test]
fn cash_not_yet_handed_back_is_still_expected() {
    let orders = [serde_json::json!({"id": "r1", "location_id": "v1", "status": "REFUNDING",
        "payments": [{"method": "cash", "amount": 700, "at": T0 + 5}], "refund": {"owed": 700}})];
    assert_eq!(cash_payments(&orders, "v1", "ALL"), vec![cash("r1", T0 + 5, "ALL", 700)]);
}
