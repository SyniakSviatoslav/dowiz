use super::{checked_plan, table_key, table_verdict};
use dowiz_hub::tables::{from_json, Held, Plan, DWELL_MIN};
use serde_json::json;

/// Two tables in one room, one of them a four-top.
fn plan() -> Plan {
    from_json(
        r#"{"zones":[{"id":"terasa","name":"Тераса","tables":[
             {"n":1,"x":85,"y":110,"w":44,"h":44,"seats":4},
             {"n":2,"x":155,"y":130,"w":40,"h":40,"seats":2}]}]}"#,
    )
    .expect("the fixture plan must read")
}

fn held(n: i64, slot_min: i64, reservation: &str) -> Held {
    Held { zone: "terasa".into(), n, slot_min, reservation: reservation.into() }
}

/// THE DEFECT THIS CLOSES, as one test. Before the floor existed a
/// reservation was a party size and a time, so the SAME TABLE at the SAME
/// MINUTE was accepted twice and the venue found out when two parties
/// walked in. Deleting the `floor::holder` arm of `table_verdict` turns
/// this red; nothing else in the suite notices.
#[test]
fn the_same_table_at_the_same_minute_is_refused_the_second_time() {
    let (p, slot) = (plan(), 29_000_000i64);
    // The first booking meets an empty floor and is allowed.
    assert_eq!(table_verdict(&p, &[], "terasa", 1, 2, slot, "rsv_first"), None);
    // The second one, for the same table and slot, is not.
    let why = table_verdict(
        &p,
        &[held(1, slot, "rsv_first")],
        "terasa",
        1,
        2,
        slot,
        "rsv_second",
    )
    .expect("a table already held must be REFUSED, not booked twice");
    // AND THE REFUSAL NAMES THE TABLE. "not available" sends a guest away
    // without telling them table 2 is free.
    assert!(why.contains("table 1"), "the refusal must name the table: {why}");
    assert!(why.contains("terasa"), "and the zone: {why}");
    assert!(why.contains(&slot.to_string()), "and the slot that took it: {why}");
    // The next table over is still free at that minute.
    assert_eq!(table_verdict(&p, &[held(1, slot, "rsv_first")], "terasa", 2, 2, slot, "x"), None);
}

/// OCCUPIED IS PER SLOT. The same table, two hours later, is free.
#[test]
fn a_table_held_at_one_hour_is_bookable_at_another() {
    let (p, slot) = (plan(), 29_000_000i64);
    let booked = [held(1, slot, "rsv_first")];
    assert!(table_verdict(&p, &booked, "terasa", 1, 2, slot + 15, "x").is_some(),
            "fifteen minutes later is the same sitting");
    assert_eq!(
        table_verdict(&p, &booked, "terasa", 1, 2, slot + DWELL_MIN, "x"),
        None,
        "past the dwell the table has turned over"
    );
}

/// A RETRIED REQUEST IS NOT A DOUBLE BOOKING. `create` derives the
/// reservation id from the caller's `requestId`, so a retry arrives
/// holding its own table -- and must not be refused by it.
#[test]
fn a_reservation_does_not_collide_with_its_own_hold() {
    let (p, slot) = (plan(), 29_000_000i64);
    assert_eq!(
        table_verdict(&p, &[held(1, slot, "rsv_me")], "terasa", 1, 2, slot, "rsv_me"),
        None
    );
}

#[test]
fn a_party_too_large_for_the_table_is_refused_by_name() {
    let (p, slot) = (plan(), 29_000_000i64);
    let why = table_verdict(&p, &[], "terasa", 2, 4, slot, "x").expect("a deuce seats two");
    assert!(why.contains("table 2") && why.contains("seats 2"), "{why}");
    assert_eq!(table_verdict(&p, &[], "terasa", 1, 4, slot, "x"), None, "the four-top takes it");
}

#[test]
fn a_table_that_is_not_on_the_plan_is_refused() {
    let (p, slot) = (plan(), 29_000_000i64);
    assert!(table_verdict(&p, &[], "terasa", 9, 2, slot, "x").unwrap().contains("no table 9"));
    assert!(table_verdict(&p, &[], "zala", 1, 2, slot, "x").unwrap().contains("no table 1"));
}

/// THE KEYS THE HOLD IS MADE OF. They sort as strings and the scan in
/// `held_tables` parses the slot back out of them, so the padding is
/// load-bearing exactly as it is for `ev_key`.
#[test]
fn table_keys_sort_numerically_and_parse_back() {
    let mut keys: Vec<String> = [2i64, 10, 1].iter().map(|n| table_key("z", *n, 100)).collect();
    keys.sort();
    assert_eq!(keys, vec![table_key("z", 1, 100), table_key("z", 2, 100), table_key("z", 10, 100)]);
    let parts: Vec<&str> = keys[2].split('/').collect();
    assert_eq!(parts[0], "rsv.tbl");
    assert_eq!(parts[1], "z");
    assert_eq!(parts[2].parse::<i64>().unwrap(), 10, "the padding does not need stripping");
    assert_eq!(parts[3].parse::<i64>().unwrap(), 100);
}

/// ONE TABLE'S HOLDS ARE ONE PREFIX. `held_tables` scans `rsv.tbl/` whole,
/// but the key must still keep one zone's tables out of another's range.
#[test]
fn one_zones_tables_cannot_be_read_as_anothers() {
    assert!(table_key("terasa", 1, 5).starts_with("rsv.tbl/terasa/"));
    assert!(!table_key("mala", 1, 5).starts_with("rsv.tbl/terasa/"));
}

// ── the owner's editor saves through `checked_plan` ─────────────────────────

fn table(n: i64, seats: i64) -> serde_json::Value {
    json!({ "n": n, "x": 100, "y": 100, "w": 40, "h": 40, "seats": seats })
}

/// The editor's positive twin: two zones, a table number reused ACROSS zones
/// (the terrace's 1 and the hall's 1 are two tables), seats in range.
#[test]
fn a_plan_the_editor_draws_is_accepted() {
    let zones = vec![
        json!({ "id": "terasa", "name": "Terasa", "tables": [table(1, 4), table(2, 2)] }),
        json!({ "id": "salla", "name": "Salla", "tables": [table(1, 6)] }),
    ];
    assert_eq!(checked_plan(&zones), Ok((2, 3)));
    // An empty list is the feature switched off, not a refusal.
    assert_eq!(checked_plan(&[]), Ok((0, 0)));
}

/// UNIQUE ZONE + NUMBER: two table 1s in one zone would be two tables the
/// guest and the waiter mean differently. Refused whole, naming the zone.
#[test]
fn two_tables_with_one_number_in_a_zone_are_refused() {
    let zones = vec![json!({ "id": "terasa", "name": "Terasa", "tables": [table(1, 4), table(1, 2)] })];
    let why = checked_plan(&zones).unwrap_err();
    assert!(why.contains("terasa") && why.contains("two tables numbered 1"), "{why}");
}

/// SEATS > 0: a table with no seats is a typo that would hide a table.
#[test]
fn a_table_without_seats_is_refused() {
    let zones = vec![json!({ "id": "terasa", "name": "Terasa", "tables": [table(3, 0)] })];
    let why = checked_plan(&zones).unwrap_err();
    assert!(why.contains("table 3") && why.contains("0 seats"), "{why}");
}

#[test]
fn a_zone_without_a_name_or_a_duplicate_zone_is_refused() {
    assert!(checked_plan(&[json!({ "id": "a", "name": "", "tables": [] })]).is_err());
    let twice = vec![
        json!({ "id": "a", "name": "A", "tables": [] }),
        json!({ "id": "a", "name": "B", "tables": [] }),
    ];
    assert!(checked_plan(&twice).unwrap_err().contains("two zones share"));
}
