//! Supplier aliases through the venue object's turn: a confirmed invoice line is
//! remembered on the supplier's card, the next request writes only what
//! changed, an empty item forgets, and the NIPT finds the card again.

use super::super::suppliers::CARD;
use super::super::turn::{run as turn, supplies_of, StockTurnIn};
use super::*;

const NOW: i64 = 1_790_000_000_000;

fn input(kind: &str, card: Value) -> StockTurnIn {
    let supplies = supplies_of(vec![
        ("salmon".into(), json!({ "name": "Salmon", "unit": "g" }).to_string()),
        ("rice".into(), json!({ "name": "Rice", "unit": "g" }).to_string()),
    ]);
    StockTurnIn { kind: kind.into(), body: json!({ "card": card }), by: "p_anna".into(), now_ms: NOW, today: 20261004, supplies, currency: String::new() }
}

fn with_card() -> StockLog {
    let mut l = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut l, &input(CARD, json!({ "name": "Peshku i Detit" })), false).unwrap();
    l
}

fn alias(l: &mut StockLog, c: Value) -> Result<Value, Bad> {
    turn(l, &input(KIND, c), false).map(|r| r.0)
}

#[test]
fn a_confirmed_line_is_remembered_and_the_same_line_again_writes_nothing() {
    let mut l = with_card();
    let before = l.len();
    let r = alias(&mut l, json!({ "supplier": "peshku-i-detit", "nipt": " k12345678a ", "lines": [
        { "text": "  Salmon  File Norvegjeze ", "item": "salmon" }, { "text": "Oriz sushi", "item": "rice" } ] })).unwrap();
    assert_eq!(r["written"], 3, "the NIPT and two lines");
    let k = &fold(&l)["peshku-i-detit"];
    assert_eq!(k.nipt, "K12345678A", "upper case, trimmed");
    assert_eq!(k.lines.get("salmon file norvegjeze").map(String::as_str), Some("salmon"), "the key is lower case, one space");
    assert_eq!(l.len(), before + 3);
    let again = alias(&mut l, json!({ "supplier": "peshku-i-detit", "nipt": "K12345678A", "lines": [{ "text": "salmon file norvegjeze", "item": "salmon" }] })).unwrap();
    assert_eq!(again["written"], 0, "nothing changed, nothing written");
    assert_eq!(l.len(), before + 3, "the log did not grow");
    assert_eq!(l.ledger().unwrap(), dowiz_hub::stock::StockLedger::default(), "an alias moves no stock");
}

#[test]
fn a_changed_line_is_rewritten_and_an_empty_item_forgets_it() {
    let mut l = with_card();
    alias(&mut l, json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "Oriz", "item": "salmon" }] })).unwrap();
    let r = alias(&mut l, json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "oriz", "item": "rice" }] })).unwrap();
    assert_eq!((r["written"].as_i64(), r["known"]["lines"]["oriz"].as_str()), (Some(1), Some("rice")), "the newest word wins");
    alias(&mut l, json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "ORIZ", "item": "" }] })).unwrap();
    assert!(fold(&l)["peshku-i-detit"].lines.is_empty(), "forgotten");
}

#[test]
fn what_is_not_an_alias_is_refused_and_nothing_is_written() {
    let mut l = with_card();
    let before = l.len();
    for (bad, status) in [
        (json!({ "supplier": "nobody", "lines": [{ "text": "x", "item": "rice" }] }), 404),
        (json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "x", "item": "caviar" }] }), 404),
        (json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "   ", "item": "rice" }] }), 400),
        (json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "x".repeat(TEXT_MAX + 1), "item": "rice" }] }), 400),
        (json!({ "supplier": "peshku-i-detit", "nipt": "12345678" }), 400),
        (json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "x", "item": "rice", "price": 5 }] }), 400),
        (json!({ "supplier": "peshku-i-detit", "phone": "+355 69 000" }), 400),
        (json!({ "supplier": "peshku-i-detit", "lines": vec![json!({ "text": "x", "item": "rice" }); LINES_MAX + 1] }), 400),
    ] {
        assert_eq!(alias(&mut l, bad.clone()).map_err(|e| e.0).err(), Some(status), "{bad}");
    }
    // One good line beside a bad one: the request is refused whole.
    assert!(alias(&mut l, json!({ "supplier": "peshku-i-detit", "lines": [{ "text": "a", "item": "rice" }, { "text": "b", "item": "caviar" }] })).is_err());
    assert_eq!(l.len(), before, "nothing written");
    // The positive twin of each: the same shapes, correct.
    assert!(alias(&mut l, json!({ "supplier": "peshku-i-detit", "nipt": "L01234567B", "lines": [{ "text": "x".repeat(TEXT_MAX), "item": "rice" }] })).is_ok());
}

#[test]
fn a_supplier_keeps_a_bounded_number_of_aliases() {
    let mut known = BTreeMap::new();
    known.insert("s".to_string(), Known { nipt: String::new(), lines: (0..PER_SUPPLIER_MAX).map(|i| (format!("t{i}"), "rice".to_string())).collect() });
    let one_more = AliasIn { supplier: "s".into(), nipt: None, lines: vec![LineIn { text: "new".into(), item: "rice".into() }] };
    assert_eq!(plan(one_more, &known, |_| true, |_| true).map_err(|e| e.0), Err(400));
    let a_change = AliasIn { supplier: "s".into(), nipt: None, lines: vec![LineIn { text: "t1".into(), item: "salmon".into() }] };
    assert_eq!(plan(a_change, &known, |_| true, |_| true).map(|n| n.len()), Ok(1), "changing one already kept is fine");
}

#[test]
fn the_key_and_the_nipt_rules() {
    assert_eq!(key("  Salmon\tFILE   Norvegjeze "), "salmon file norvegjeze");
    assert_eq!(key("Djathë Kaçkavall"), "djathë kaçkavall");
    assert_eq!(nipt("k12345678a").as_deref(), Some("K12345678A"));
    for bad in ["", "K1234567A", "K123456789A", "112345678A", "K12345678", "K1234S678A"] {
        assert_eq!(nipt(bad), None, "{bad}");
    }
}
