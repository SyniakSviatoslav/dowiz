//! Each movement through `plan` and `apply` onto a REAL log: a priced, lotted
//! delivery (I2), a valued write-off (I4), a count session (I4) and a prep
//! (I5). Every refusal beside its twin, and a refusal writes nothing.

use super::*;

const NOW: i64 = 1_790_000_000_000;
const TODAY: i64 = 20260926;

fn body(raw: &str) -> StockMoveIn {
    serde_json::from_str(raw).expect("a well-formed body")
}
fn no_shelf(_: &str) -> Option<i64> {
    None
}
fn run(log: &mut StockLog, kind: &str, raw: &str) -> Result<Value, String> {
    let p = plan(kind, body(raw), "p_owner", NOW, TODAY, no_shelf).map_err(|(c, m)| format!("{c} {m}"))?;
    p.apply(log).map_err(|e| e.to_string())
}

/// I2's CHECK through the request: 1 kg at 1,000 then 1 kg at 1,200 is a
/// WAC of 1,100; the paper's supplier, invoice, lot and date ride on the
/// record, dated and signed; the total form is the same price.
#[test]
fn a_priced_delivery_moves_the_average_and_keeps_its_paper() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(NOW);
    let r = run(&mut log, "received", r#"{"item":"rice","qty":1000,"unitCost":1000,"per":1000,"supplier":"Agro","doc":"F-12","lot":"L1","expiry":"2026-10-03"}"#).unwrap();
    assert_eq!(r["lines"][0]["value"], json!(1000));
    run(&mut log, "received", r#"{"item":"rice","qty":1000,"total":1200}"#).unwrap();
    assert_eq!(log.cost_book().wac("rice", 1000), Some(1100));
    let j = log.journal().unwrap();
    let m = &j.entries[0].meta;
    assert_eq!((m.supplier.as_deref(), m.doc.as_deref(), m.lot.as_deref(), m.expiry), (Some("Agro"), Some("F-12"), Some("L1"), Some(20261003)));
    assert_eq!((m.at, m.by.as_deref()), (Some(NOW), Some("p_owner")));
}

/// Half a price, a negative one and a date that is not one are 400s, and the
/// log did not move. A supply's shelf life dates an undated delivery.
#[test]
fn a_bad_price_or_date_is_refused_and_a_shelf_life_dates_the_rest() {
    for raw in [
        r#"{"item":"rice","qty":10,"unitCost":100}"#,
        r#"{"item":"rice","qty":10,"per":1000}"#,
        r#"{"item":"rice","qty":10,"total":-1}"#,
        r#"{"item":"rice","qty":10,"total":5,"unitCost":1,"per":1}"#,
        r#"{"item":"rice","qty":10,"expiry":"2026-02-30"}"#,
    ] {
        assert!(matches!(plan("received", body(raw), "p", NOW, TODAY, no_shelf), Err((400, _))), "{raw}");
    }
    let p = plan("received", body(r#"{"item":"fish","qty":10}"#), "p", NOW, TODAY, |_| Some(3)).unwrap();
    assert_eq!(p.lines[0].1.expiry, Some(20260929), "today + 3 days");
    let p = plan("received", body(r#"{"item":"fish","qty":10,"expiry":"2026-09-27"}"#), "p", NOW, TODAY, |_| Some(3)).unwrap();
    assert_eq!(p.lines[0].1.expiry, Some(20260927), "the label wins");
}

/// I4: a write-off is stored WITH its value at the average of its moment.
#[test]
fn a_write_off_stores_its_value() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    run(&mut log, "received", r#"{"item":"tuna","qty":1000,"unitCost":3000,"per":1000}"#).unwrap();
    let r = run(&mut log, "wasted", r#"{"item":"tuna","qty":200,"reason":"spoiled","lot":"L9"}"#).unwrap();
    assert_eq!(r["lines"][0]["value"], json!(600));
    let raw = log.journal().unwrap();
    assert_eq!((raw.entries[1].meta.value, raw.entries[1].meta.lot.as_deref()), (Some(600), Some("L9")));
    // Twin: more than the shelf is refused and nothing is written.
    assert!(run(&mut log, "wasted", r#"{"item":"tuna","qty":900,"reason":"spoiled"}"#).is_err());
    assert_eq!(log.len(), 2);
}

/// I4: A SESSION IS ONE DECISION. Every line carries the session id and the
/// shelf it was compared with; the answer is the drift per line and its
/// value. One impossible line (below what is reserved) writes NOTHING.
#[test]
fn a_count_session_is_all_or_nothing_with_its_drift() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    run(&mut log, "received", r#"{"item":"rice","qty":1000,"unitCost":2000,"per":1000}"#).unwrap();
    run(&mut log, "received", r#"{"item":"nori","qty":50}"#).unwrap();
    log.append(&StockEvent::Reserved { item: "nori".into(), qty: 20, order_id: "o1".into() }).unwrap();
    let before = log.len();
    let bad = r#"{"lines":[{"item":"rice","observed":900},{"item":"nori","observed":5}]}"#;
    assert!(run(&mut log, "count", bad).is_err());
    assert_eq!(log.len(), before, "the short line refused the whole session");

    let r = run(&mut log, "count", r#"{"session":"inv-1","lines":[{"item":"rice","observed":900},{"item":"nori","observed":55}]}"#).unwrap();
    assert_eq!(r["lines"][0], json!({ "item": "rice", "expected": 1000, "observed": 900, "drift": -100, "value": -200 }));
    assert_eq!((r["lines"][1]["drift"].clone(), r["lines"][1]["value"].clone()), (json!(5), Value::Null));
    assert_eq!(r["value"], json!(-200));
    let j = log.journal().unwrap();
    let last: Vec<_> = j.entries.iter().rev().take(2).collect();
    assert!(last.iter().all(|e| e.meta.session.as_deref() == Some("inv-1")));
    assert_eq!(last[1].meta.expected, Some(1000));
    assert_eq!(j.ledger.level("rice").on_hand, 900);
}

/// A session with no lines, a line counted twice, or a negative count is a
/// 400 before anything is decided.
#[test]
fn a_session_that_is_not_one_is_a_400() {
    for raw in [
        r#"{"lines":[]}"#,
        r#"{}"#,
        r#"{"lines":[{"item":"rice","observed":1},{"item":" rice ","observed":2}]}"#,
        r#"{"lines":[{"item":"rice","observed":-1}]}"#,
        r#"{"lines":[{"item":"","observed":1}]}"#,
    ] {
        assert!(matches!(plan("count", body(raw), "p", NOW, TODAY, no_shelf), Err((400, _))), "{raw}");
    }
    let p = plan("count", body(r#"{"lines":[{"item":"rice","observed":0}]}"#), "p", NOW, TODAY, no_shelf).unwrap();
    assert_eq!(p.lines[0].1.session.as_deref(), Some("st_1790000000000"), "a minted session id");
}

/// I5: a prep measurement answers its yield; into another supply it moves
/// stock; a stage outside the two is a 400; the output is a supply to check.
#[test]
fn a_prep_answers_its_yield_and_names_what_it_touches() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    run(&mut log, "received", r#"{"item":"salmon","qty":5000}"#).unwrap();
    let r = run(&mut log, "produced", r#"{"item":"salmon","qty":5000,"out":2750}"#).unwrap();
    assert_eq!(r["lines"][0]["yieldPm"], json!(550));
    assert_eq!(log.ledger().unwrap().level("salmon").on_hand, 5000, "a measurement moves nothing");
    let p = plan("produced", body(r#"{"item":"salmon","qty":1000,"out":560,"stage":"clean","into":"fillet"}"#), "p", NOW, TODAY, no_shelf).unwrap();
    assert_eq!(p.items(), vec!["salmon".to_string(), "fillet".to_string()]);
    p.apply(&mut log).unwrap();
    assert_eq!(log.ledger().unwrap().level("fillet").on_hand, 560);
    assert!(matches!(plan("produced", body(r#"{"item":"salmon","qty":1,"out":1,"stage":"fry"}"#), "p", NOW, TODAY, no_shelf), Err((400, _))));
    assert!(matches!(plan("produced", body(r#"{"item":"salmon","qty":1}"#), "p", NOW, TODAY, no_shelf), Err((400, _))));
}

/// A text field is bounded; a body still cannot name its signer.
#[test]
fn text_is_bounded_and_the_signer_is_not_the_callers() {
    let long = format!(r#"{{"item":"rice","qty":1,"supplier":"{}"}}"#, "x".repeat(81));
    assert!(matches!(plan("received", body(&long), "p", NOW, TODAY, no_shelf), Err((400, _))));
    assert!(serde_json::from_str::<StockMoveIn>(r#"{"item":"rice","qty":1,"by":"p_boss"}"#).is_err());
}
