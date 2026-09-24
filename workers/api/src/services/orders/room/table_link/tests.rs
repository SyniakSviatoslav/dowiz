//! A9: the table's code, the guest's seat, the old token, the printed QR.
//! Every test calls the real function; none re-states the rule.

use super::*;
use crate::services::orders::room::placer::{guest_seat, plan_of, Guest};
use crate::services::orders::room::table_qr;
use dowiz_hub::tables::{Shape, Table, Zone};
use serde_json::json;

const KEY: &[u8] = b"0123456789abcdef0123456789abcdef";
const LOC: &str = "sushi-durres";

fn t(n: i64) -> Table {
    Table { n, x: 40 * n, y: 50, w: 30, h: 30, seats: 4, shape: Shape::Rect }
}
fn plan() -> Plan {
    Plan {
        zones: vec![
            Zone { id: "salla".into(), name: "Salla".into(), tables: (1..=4).map(t).collect() },
            Zone { id: "terasa".into(), name: "Terasa".into(), tables: vec![t(7)] },
        ],
    }
}
fn view(id: &str, o: Value) -> OrderView {
    OrderView { order_id: id.into(), kind: 1, seq: 10, order_json: o.to_string() }
}
fn round(id: &str, sitting: &str, table: &str, status: &str, total: i64, paid: i64, at: i64) -> Value {
    let payments: Vec<Value> = if paid > 0 { vec![json!({"amount": paid, "method": "cash", "at": at + 5})] } else { vec![] };
    json!({"id": id, "location_id": LOC, "sitting_id": sitting, "status": status, "total": total,
           "payments": payments, "created_at_ms": at, "fulfilment": {"kind": "dine_in", "table": table}})
}

// ── the signature ───────────────────────────────────────────────────────────

#[test]
fn a_signed_table_code_verifies() {
    let code = table_param(KEY, LOC, "salla", 3);
    assert_eq!(verify_table(KEY, LOC, &plan(), &code), Ok(("salla".to_string(), 3)));
    assert_eq!(table_sig(KEY, LOC, "salla", 3).len(), SIG_HEX);
    let url = table_url("sushi-durres.dowiz.org", KEY, LOC, "salla", 3);
    assert_eq!(url, format!("https://sushi-durres.dowiz.org/store/?t={code}"));
}

#[test]
fn a_tampered_signature_is_refused() {
    let code = table_param(KEY, LOC, "salla", 3);
    // the same signature moved to the next table
    let moved = code.replacen("salla.3.", "salla.4.", 1);
    assert_eq!(verify_table(KEY, LOC, &plan(), &moved), Err(LinkRefused::BadSignature));
    // one hex digit flipped
    let last = code.chars().last().unwrap();
    let flipped = format!("{}{}", &code[..code.len() - 1], if last == '0' { '1' } else { '0' });
    assert_eq!(verify_table(KEY, LOC, &plan(), &flipped), Err(LinkRefused::BadSignature));
    // signed for ANOTHER venue, presented here
    let elsewhere = table_param(KEY, "dubin-durres", "salla", 3);
    assert_eq!(verify_table(KEY, LOC, &plan(), &elsewhere), Err(LinkRefused::BadSignature));
    // signed with another key
    let other = table_param(b"another-key-another-key-another!!", LOC, "salla", 3);
    assert_eq!(verify_table(KEY, LOC, &plan(), &other), Err(LinkRefused::BadSignature));
}

#[test]
fn a_table_not_on_the_plan_is_refused_even_signed() {
    let gone = table_param(KEY, LOC, "salla", 9);
    assert_eq!(verify_table(KEY, LOC, &plan(), &gone), Err(LinkRefused::NoSuchTable));
    let zone_gone = table_param(KEY, LOC, "bar", 1);
    assert_eq!(verify_table(KEY, LOC, &plan(), &zone_gone), Err(LinkRefused::NoSuchTable));
    // and the twin: the same venue's table 7 on the terrace stands
    let here = table_param(KEY, LOC, "terasa", 7);
    assert!(verify_table(KEY, LOC, &plan(), &here).is_ok());
}

#[test]
fn a_malformed_code_is_refused() {
    for bad in ["", "salla", "salla.3", "salla.x.0123456789abcdef", "salla.3.0123", "salla.3.0123456789ABCDEF",
                "salla.3.0123456789abcdef.x", ".3.0123456789abcdef"] {
        assert_eq!(verify_table(KEY, LOC, &plan(), bad), Err(LinkRefused::Malformed), "{bad:?}");
    }
}

// ── the live sitting ────────────────────────────────────────────────────────

#[test]
fn the_live_sitting_is_the_open_one_at_that_table() {
    let listed = vec![
        view("a", round("a", "old-sitting-1", "salla:3", "PICKED_UP", 900, 900, 1)), // paid up: history
        view("b", round("b", "live-sitting-2", "3", "CONFIRMED", 500, 0, 10)),        // bare n: salla 3
        view("c", round("c", "other-table-3", "salla:4", "PENDING", 500, 0, 20)),
    ];
    assert_eq!(live_sitting(&plan(), &listed, "salla", 3).as_deref(), Some("live-sitting-2"));
    assert_eq!(live_sitting(&plan(), &listed, "salla", 4).as_deref(), Some("other-table-3"));
    assert_eq!(live_sitting(&plan(), &listed, "salla", 1), None);
    // table 7 exists in one zone only, so a bare "7" is the terrace's
    let t7 = vec![view("d", round("d", "terrace-sit-4", "7", "PREPARING", 100, 0, 5))];
    assert_eq!(live_sitting(&plan(), &t7, "terasa", 7).as_deref(), Some("terrace-sit-4"));
}

// ── the guest's seat (placer::guest_seat) ───────────────────────────────────

#[test]
fn a_guest_naming_a_sitting_is_refused() {
    let code = Some(("salla".to_string(), 3));
    let got = guest_seat("dine_in", Some("live-sitting-2"), code, None, || Some("fresh-sitting".into()));
    assert_eq!(got.unwrap_err().0, 400);
    // even with no table code at all
    assert_eq!(guest_seat("delivery", Some("abcdefgh12"), None, None, || None).unwrap_err().0, 400);
    // twin: the same guest without the raw sitting is seated
    assert!(guest_seat("dine_in", None, Some(("salla".into(), 3)), None, || Some("fresh-sitting".into())).is_ok());
}

#[test]
fn a_guest_with_a_table_code_joins_the_live_sitting() {
    let listed = vec![view("b", round("b", "live-sitting-2", "salla:3", "CONFIRMED", 500, 0, 10))];
    let code = table_param(KEY, LOC, "salla", 3);
    let (zone, n) = verify_table(KEY, LOC, &plan(), &code).unwrap();
    let live = live_sitting(&plan(), &listed, &zone, n);
    let got = guest_seat("dine_in", None, Some((zone, n)), live, || panic!("must not mint when a sitting is live"));
    assert_eq!(got, Ok(Some(Guest { sitting_id: "live-sitting-2".into(), table: "salla:3".into() })));
}

#[test]
fn a_guest_at_an_empty_table_opens_a_new_sitting() {
    let listed = vec![view("a", round("a", "old-sitting-1", "salla:3", "PICKED_UP", 900, 900, 1))];
    let live = live_sitting(&plan(), &listed, "salla", 3);
    assert_eq!(live, None);
    let got = guest_seat("dine_in", None, Some(("salla".into(), 3)), live, || Some("fresh-sitting".into()));
    assert_eq!(got, Ok(Some(Guest { sitting_id: "fresh-sitting".into(), table: "salla:3".into() })));
}

#[test]
fn a_table_code_on_a_delivery_is_refused_and_no_code_is_no_guest() {
    assert_eq!(guest_seat("delivery", None, Some(("salla".into(), 3)), None, || Some("x".into())).unwrap_err().0, 400);
    assert_eq!(guest_seat("delivery", None, None, None, || Some("x".into())), Ok(None));
}

#[test]
fn the_plan_is_read_from_the_venue_record() {
    let rec = json!({"id": LOC, "floor_plan": {"zones": [{"id": "salla", "name": "Salla",
        "tables": [{"n": 3, "x": 40, "y": 40, "w": 30, "h": 30, "seats": 4, "shape": "rect"}]}]}});
    let p = plan_of(&rec.to_string());
    assert!(p.find("salla", 3).is_some());
    assert!(plan_of("{}").is_empty());
}

// ── the customer token (auth::Claims::Customer) ─────────────────────────────

#[test]
fn an_old_customer_token_without_sitting_id_still_decodes() {
    let old = r#"{"role":"customer","sub":"o1","order_id":"o1","location_id":"v","iat":1,"exp":2}"#;
    let c: crate::auth::Claims = serde_json::from_str(old).expect("a token minted before A9 must parse");
    assert!(matches!(c, crate::auth::Claims::Customer { sitting_id: None, .. }));
    // and an ordinary order's token serialises exactly as before: no new field
    assert_eq!(serde_json::to_string(&c).unwrap(), old);
    // twin: a guest's token carries its sitting both ways
    let with = crate::auth::Claims::Customer {
        sub: "o2".into(), order_id: "o2".into(), location_id: "v".into(),
        sitting_id: Some("sit-1".into()), iat: 1, exp: 2,
    };
    let back: crate::auth::Claims = serde_json::from_str(&serde_json::to_string(&with).unwrap()).unwrap();
    assert_eq!(back, with);
}

// ── the printed code (table_qr) ─────────────────────────────────────────────

#[test]
fn every_table_on_the_plan_gets_a_code_that_carries_its_link() {
    let list = table_qr::entries(&plan(), "sushi-durres.dowiz.org", KEY, LOC);
    assert_eq!(list.len(), 5);
    let first = &list[0];
    assert_eq!(first["zone"], "salla");
    assert_eq!(first["n"], 1);
    let url = first["url"].as_str().unwrap();
    let t = url.split("?t=").nth(1).unwrap();
    assert_eq!(verify_table(KEY, LOC, &plan(), t), Ok(("salla".to_string(), 1)));
    let svg = first["svg"].as_str().unwrap();
    assert!(svg.starts_with("<svg") && svg.contains("<path d=\"M"), "{svg}");
}

#[test]
fn the_qr_svg_is_deterministic_and_differs_per_table() {
    let a = table_qr::svg("https://a.dowiz.org/store/?t=salla.1.0123456789abcdef").unwrap();
    let b = table_qr::svg("https://a.dowiz.org/store/?t=salla.2.0123456789abcdef").unwrap();
    assert_eq!(a, table_qr::svg("https://a.dowiz.org/store/?t=salla.1.0123456789abcdef").unwrap());
    assert_ne!(a, b);
    // a text no QR version can hold is refused, not truncated
    assert!(table_qr::svg(&"x".repeat(8000)).is_none());
}
