use super::*;
use serde_json::json;

fn round(id: &str, status: &str, total: i64, paid: i64) -> Value {
    json!({ "id": id, "seq": 7, "status": status, "total": total, "paid": paid, "payment_status": if paid >= total && total > 0 { "paid" } else { "unpaid" } })
}
fn sitting(table: &str, rounds: Vec<Value>) -> Value {
    json!({ "sitting_id": format!("s-{table}"), "table": table, "rounds": rounds })
}

#[test]
fn a_table_is_found_by_its_name_then_by_its_digits() {
    let room = vec![sitting("5", vec![]), sitting("T12", vec![]), sitting("Terrace 3", vec![])];
    assert_eq!(at_table(&room, "5").len(), 1);
    assert_eq!(at_table(&room, "12")[0]["table"], "T12");
    assert_eq!(at_table(&room, "3")[0]["table"], "Terrace 3");
    assert!(at_table(&room, "9").is_empty());
    // "5" exactly beats "T5" by digits.
    let both = vec![sitting("T5", vec![]), sitting("5", vec![])];
    assert_eq!(at_table(&both, "5").len(), 1);
    assert_eq!(at_table(&both, "5")[0]["table"], "5");
    // A named table is found by its name, any case; a name is not a number.
    let named = vec![sitting("Bar", vec![]), sitting("Bar 2", vec![])];
    assert_eq!(at_table(&named, "bar").len(), 1);
    assert!(at_table(&named, "Terrace").is_empty());
    assert!(at_table(&named, "").is_empty());
}

#[test]
fn a_dish_goes_to_the_one_round_the_kitchen_has_not_taken() {
    let room = vec![sitting("5", vec![round("a", "PREPARING", 900, 0), round("b", "PENDING", 500, 0)])];
    assert_eq!(editable(&room, "5"), Ok(Some(RoundRef { id: "b".into(), seq: 7 })));
    // Every round already in the kitchen: a NEW round is the answer.
    let cooking = vec![sitting("5", vec![round("a", "PREPARING", 900, 0)])];
    assert_eq!(editable(&cooking, "5"), Ok(None));
    // No sitting at the table: a new round too.
    assert_eq!(editable(&room, "8"), Ok(None));
    // A paid round is not added to.
    let paid = vec![sitting("5", vec![round("a", "CONFIRMED", 900, 900)])];
    assert_eq!(editable(&paid, "5"), Ok(None));
}

#[test]
fn two_rounds_or_two_sittings_are_refused_not_guessed() {
    let two = vec![sitting("5", vec![round("a", "PENDING", 1, 0), round("b", "CONFIRMED", 1, 0)])];
    assert_eq!(editable(&two, "5"), Err("two_rounds"));
    let twice = vec![sitting("T5", vec![]), sitting("Bar 5", vec![])];
    assert_eq!(editable(&twice, "5"), Err("two_sittings"));
    assert_eq!(payable(&twice, "5"), Err("two_sittings"));
}

#[test]
fn paid_finds_the_one_round_that_owes_and_how_much() {
    let room = vec![sitting("5", vec![round("a", "DELIVERED", 1500, 1500), round("b", "READY", 2000, 500)])];
    assert_eq!(payable(&room, "5"), Ok((RoundRef { id: "b".into(), seq: 7 }, 1500)));
}

#[test]
fn paid_refuses_what_pay_would_refuse() {
    assert_eq!(payable(&[], "5"), Err("no_table"));
    let settled = vec![sitting("5", vec![round("a", "DELIVERED", 1500, 1500)])];
    assert_eq!(payable(&settled, "5"), Err("nothing_owed"));
    let refused = vec![sitting("5", vec![round("a", "REJECTED", 1500, 0)])];
    assert_eq!(payable(&refused, "5"), Err("nothing_owed"));
    let mut guest = round("g", "PENDING", 800, 0);
    guest["placed_by"] = json!(dowiz_hub::room::pay::GUEST);
    assert_eq!(payable(&[sitting("5", vec![guest])], "5"), Err("nothing_owed"));
    let two = vec![sitting("5", vec![round("a", "READY", 1, 0), round("b", "READY", 1, 0)])];
    assert_eq!(payable(&two, "5"), Err("two_rounds"));
}

#[test]
fn owed_never_goes_below_zero() {
    assert_eq!(owed(&round("a", "READY", 1000, 1200)), 0);
    assert_eq!(owed(&json!({ "total": 1000, "paid": 1200, "payment_status": "unpaid" })), 0);
    assert_eq!(owed(&json!({ "total": 1000, "paid": 400 })), 600);
}

#[test]
fn the_glance_counts_tables_and_waiting_rounds() {
    let room = vec![sitting("5", vec![round("a", "PENDING", 1, 0), round("b", "READY", 1, 0)]), sitting("6", vec![round("c", "PENDING", 1, 0)])];
    assert_eq!(glance(&room), (2, 2));
    assert_eq!(glance(&[]), (0, 0));
}
