//! What the floor knows. Every test here is a sentence about a restaurant,
//! not about a parser.

use super::*;

/// A plan in the shape the prototype's `ZONES` const holds, minus `occ` --
/// which is the point of the model and is asserted below.
const PLAN: &str = r#"{"zones":[
  {"id":"terasa","name":"Тераса","tables":[
    {"n":1,"x":85,"y":110,"shape":"rect","w":44,"h":44,"seats":4},
    {"n":2,"x":155,"y":130,"shape":"rect","w":40,"h":40,"seats":2},
    {"n":6,"x":290,"y":245,"shape":"rect","w":66,"h":44,"seats":4}]},
  {"id":"mala","name":"Мала зала","tables":[
    {"n":3,"x":140,"y":330,"shape":"circle","w":50,"h":50,"seats":2}]}]}"#;

fn plan() -> Plan {
    from_json(PLAN).expect("the prototype's own shape must read")
}

fn held(zone: &str, n: i64, slot_min: i64) -> Held {
    Held { zone: zone.into(), n, slot_min, reservation: format!("rsv_{zone}_{n}_{slot_min}") }
}

// ── the model ───────────────────────────────────────────────────────────────

#[test]
fn the_prototypes_own_zones_read_back_whole() {
    let p = plan();
    assert_eq!(p.zones.len(), 2);
    assert_eq!(p.zones[0].name, "Тераса", "the venue's own word, not a key");
    assert_eq!(p.zones[0].tables.len(), 3);
    assert_eq!(p.find("terasa", 6).map(|t| t.w), Some(66), "the long four-top");
    assert_eq!(p.find("mala", 3).map(|t| t.shape), Some(Shape::Circle));
    assert_eq!(p.find("terasa", 9), None, "a table that is not there");
}

#[test]
fn no_plan_is_an_empty_plan_and_not_an_error() {
    assert!(from_json("{}").expect("an absent plan is not a refusal").is_empty());
    assert!(from_json(r#"{"zones":[]}"#).unwrap().is_empty());
    assert!(
        from_json(r#"{"zones":[{"id":"a","name":"A","tables":[]}]}"#).unwrap().is_empty(),
        "a named room with no tables in it yet is still no plan to book from"
    );
    assert!(!plan().is_empty());
}

/// THE SHAPE OF THE WHOLE MODULE, as a test. If occupancy were a field on a
/// table, this plan would carry it -- and the same JSON would mean "taken" at
/// every hour of every day.
#[test]
fn a_plan_carries_no_occupancy_at_all() {
    let p = from_json(
        r#"{"zones":[{"id":"z","name":"Z","tables":[
             {"n":1,"x":50,"y":50,"w":40,"h":40,"seats":2,"occ":true}]}]}"#,
    )
    .expect("an `occ` the prototype wrote is ignored, not refused");
    let t = p.find("z", 1).unwrap();
    assert_eq!(*t, Table { n: 1, x: 50, y: 50, w: 40, h: 40, seats: 2, shape: Shape::Rect });
    // And it is free or taken only once a SLOT is named.
    assert!(!availability(&p, &[], 2, 1_000, DWELL_MIN)[0].occupied);
    assert!(availability(&p, &[held("z", 1, 1_000)], 2, 1_000, DWELL_MIN)[0].occupied);
}

// ── refusals: a plan is refused whole, and the refusal names the table ──────

fn refuse(doc: &str) -> PlanError {
    from_json(doc).expect_err("this plan must be refused")
}

#[test]
fn a_table_missing_a_field_stops_the_parse_rather_than_vanishing() {
    let e = refuse(r#"{"zones":[{"id":"z","name":"Z","tables":[{"n":1,"x":5,"y":5,"w":9}]}]}"#);
    assert_eq!(e, PlanError::TableFieldMissing { zone: "z".into(), field: "h" });
    assert!(e.message().contains("\"h\""), "{}", e.message());
}

#[test]
fn two_tables_with_one_number_are_refused_because_the_waiter_cannot_tell_them_apart() {
    let e = refuse(
        r#"{"zones":[{"id":"z","name":"Z","tables":[
             {"n":4,"x":50,"y":50,"w":40,"h":40,"seats":2},
             {"n":4,"x":150,"y":50,"w":40,"h":40,"seats":2}]}]}"#,
    );
    assert_eq!(e, PlanError::TableNumberTwice { zone: "z".into(), n: 4 });
    assert!(e.message().contains("two tables numbered 4"), "{}", e.message());
}

#[test]
fn two_zones_with_one_id_are_refused() {
    let e = refuse(
        r#"{"zones":[{"id":"z","name":"A","tables":[]},{"id":"z","name":"B","tables":[]}]}"#,
    );
    assert_eq!(e, PlanError::ZoneIdTwice { id: "z".into() });
}

/// A ZONE ID BECOMES PART OF A STORAGE KEY. A slash in it would make one
/// zone's tables scan as another's.
#[test]
fn a_zone_id_that_could_break_a_key_is_refused() {
    assert_eq!(
        refuse(r#"{"zones":[{"id":"a/b","name":"A","tables":[]}]}"#),
        PlanError::ZoneIdBadChar { id: "a/b".into() }
    );
    assert_eq!(
        refuse(r#"{"zones":[{"id":"","name":"A","tables":[]}]}"#),
        PlanError::ZoneIdMissing
    );
    assert_eq!(
        refuse(r#"{"zones":[{"id":"Terasa","name":"A","tables":[]}]}"#),
        PlanError::ZoneIdBadChar { id: "Terasa".into() },
        "case is not a difference a key should depend on"
    );
}

#[test]
fn a_zone_with_no_name_has_nothing_to_put_on_its_tab() {
    assert_eq!(
        refuse(r#"{"zones":[{"id":"z","tables":[]}]}"#),
        PlanError::ZoneNameMissing { id: "z".into() }
    );
}

#[test]
fn seats_outside_the_venues_own_range_are_refused() {
    let doc = |s: i64| {
        format!(
            r#"{{"zones":[{{"id":"z","name":"Z","tables":[
                 {{"n":1,"x":50,"y":50,"w":40,"h":40,"seats":{s}}}]}}]}}"#
        )
    };
    assert_eq!(refuse(&doc(0)), PlanError::TableSeats { zone: "z".into(), n: 1, seats: 0 });
    assert_eq!(refuse(&doc(-2)), PlanError::TableSeats { zone: "z".into(), n: 1, seats: -2 });
    assert_eq!(
        refuse(&doc(MAX_SEATS + 1)),
        PlanError::TableSeats { zone: "z".into(), n: 1, seats: MAX_SEATS + 1 }
    );
    assert!(from_json(&doc(MAX_SEATS)).is_ok(), "the largest party the kernel seats");
    assert!(from_json(&doc(1)).is_ok());
}

/// A TABLE NOBODY CAN TAP IS NOT A TABLE. The prototype's plan is 390x446 and
/// a table is drawn from its CENTRE, so the edges are checked both ways.
#[test]
fn a_table_drawn_off_the_edge_of_the_room_is_refused() {
    let at = |x: i64, y: i64| {
        format!(
            r#"{{"zones":[{{"id":"z","name":"Z","tables":[
                 {{"n":1,"x":{x},"y":{y},"w":40,"h":40,"seats":2}}]}}]}}"#
        )
    };
    assert_eq!(refuse(&at(10, 100)), PlanError::TableOffPlan { zone: "z".into(), n: 1 });
    assert_eq!(refuse(&at(PLAN_W - 10, 100)), PlanError::TableOffPlan { zone: "z".into(), n: 1 });
    assert_eq!(refuse(&at(100, 10)), PlanError::TableOffPlan { zone: "z".into(), n: 1 });
    assert_eq!(refuse(&at(100, PLAN_H - 10)), PlanError::TableOffPlan { zone: "z".into(), n: 1 });
    assert!(from_json(&at(20, 20)).is_ok(), "exactly in the corner is on the plan");
    assert!(from_json(&at(PLAN_W - 20, PLAN_H - 20)).is_ok());
}

#[test]
fn a_round_table_is_round() {
    assert_eq!(
        refuse(
            r#"{"zones":[{"id":"z","name":"Z","tables":[
                 {"n":1,"x":50,"y":50,"w":60,"h":40,"seats":2,"shape":"circle"}]}]}"#
        ),
        PlanError::TableNotACircle { zone: "z".into(), n: 1 }
    );
}

/// AN UNKNOWN SHAPE IS A RECTANGLE, not a refusal: a plan drawn by a later
/// owner surface that learns "booth" must still open every table it names.
#[test]
fn an_unknown_shape_draws_as_a_rectangle() {
    let p = from_json(
        r#"{"zones":[{"id":"z","name":"Z","tables":[
             {"n":1,"x":50,"y":50,"w":40,"h":40,"seats":2,"shape":"booth"}]}]}"#,
    )
    .expect("a shape from the future is not a broken plan");
    assert_eq!(p.find("z", 1).unwrap().shape, Shape::Rect);
    assert_eq!(Shape::Rect.as_str(), "rect");
    assert_eq!(Shape::Circle.as_str(), "circle");
}

#[test]
fn a_plan_larger_than_the_bound_is_refused_rather_than_truncated() {
    let mut t = String::new();
    for n in 1..=(MAX_TABLES + 1) {
        if n > 1 {
            t.push(',');
        }
        t.push_str(&format!(r#"{{"n":{n},"x":50,"y":50,"w":40,"h":40,"seats":2}}"#));
    }
    let e = refuse(&format!(r#"{{"zones":[{{"id":"z","name":"Z","tables":[{t}]}}]}}"#));
    assert_eq!(e, PlanError::TooManyTables { found: MAX_TABLES + 1 });
}

// ── who may sit where, and when ─────────────────────────────────────────────

#[test]
fn a_table_seats_a_party_no_larger_than_its_seats() {
    let p = plan();
    let deuce = p.find("terasa", 2).unwrap();
    assert!(deuce.seats_party(1));
    assert!(deuce.seats_party(2));
    assert!(!deuce.seats_party(3), "three people do not fit at a two-top");
    assert!(!deuce.seats_party(0), "a party of nobody is not a party");
    assert!(!deuce.seats_party(-1));
    assert!(p.find("terasa", 1).unwrap().seats_party(2), "a four-top seats a deuce");
}

/// THE WHOLE POINT, as one test: the same table, two hours apart.
#[test]
fn the_same_table_is_free_at_one_hour_and_taken_at_another() {
    let p = plan();
    let eighteen = 29_000_000i64; // any minute; the arithmetic is what matters
    let twenty = eighteen + 120;
    let booked = [held("terasa", 1, twenty)];
    let at = |slot| {
        availability(&p, &booked, 2, slot, DWELL_MIN)
            .into_iter()
            .find(|s| s.zone == "terasa" && s.n == 1)
            .unwrap()
            .occupied
    };
    assert!(!at(eighteen), "free at 18:00");
    assert!(at(twenty), "taken at 20:00");
}

/// A DWELL, NOT AN EQUALITY. 19:00 and 19:15 are the same table for the same
/// evening; an `==` on the slot would have sold it twice.
#[test]
fn two_bookings_inside_the_dwell_collide_and_one_past_it_does_not() {
    let booked = [held("terasa", 1, 1_000)];
    let h = |slot| holder(&booked, "terasa", 1, slot, DWELL_MIN).is_some();
    assert!(h(1_000), "the same minute");
    assert!(h(1_000 + DWELL_MIN - 1), "one minute short of the dwell");
    assert!(h(1_000 - DWELL_MIN + 1), "and symmetric, before it");
    assert!(!h(1_000 + DWELL_MIN), "the dwell has run out");
    assert!(!h(1_000 - DWELL_MIN));
}

#[test]
fn a_hold_belongs_to_one_table_in_one_zone() {
    let booked = [held("terasa", 1, 1_000)];
    assert!(holder(&booked, "terasa", 1, 1_000, DWELL_MIN).is_some());
    assert!(holder(&booked, "terasa", 2, 1_000, DWELL_MIN).is_none(), "the next table over");
    assert!(
        holder(&booked, "mala", 1, 1_000, DWELL_MIN).is_none(),
        "table 1 in another room is another table"
    );
}

/// THE REFUSAL HAS TO NAME SOMETHING, so the answer is the booking.
#[test]
fn the_holder_is_the_booking_that_took_it() {
    let booked = [held("terasa", 1, 1_000), held("terasa", 1, 5_000)];
    let got = holder(&booked, "terasa", 1, 1_030, DWELL_MIN).expect("held");
    assert_eq!(got.reservation, "rsv_terasa_1_1000");
    assert_eq!(got.slot_min, 1_000);
}

/// WHICH BOOKINGS HOLD A TABLE AT ALL. Every wire status the kernel knows is
/// named here, so a status added to `ReservationStatus` without a decision
/// about the floor shows up as a failing test rather than as a table that
/// silently stays held for ever.
#[test]
fn a_finished_or_abandoned_booking_releases_its_table() {
    for s in ["REQUESTED", "CONFIRMED", "SEATED"] {
        assert!(holds_table(s), "{s} still has the table");
    }
    for s in ["COMPLETED", "DECLINED", "CANCELLED_BY_GUEST", "CANCELLED_BY_VENUE", "NO_SHOW"] {
        assert!(!holds_table(s), "{s} must give the table back");
    }
    assert!(!holds_table(""), "an unreadable status holds nothing");
    assert!(!holds_table("PENDING"), "an ORDER status is not a reservation status");
}

/// A GUEST MUST SEE THE TABLES THEY CANNOT HAVE. An availability answer that
/// omitted them would draw an empty room.
#[test]
fn every_table_appears_in_the_answer_however_unavailable() {
    let p = plan();
    let a = availability(&p, &[held("terasa", 1, 1_000)], 4, 1_000, DWELL_MIN);
    assert_eq!(a.len(), 4, "three on the terrace, one in the small room");
    let s = |zone: &str, n: i64| {
        a.iter().find(|s| s.zone == zone && s.n == n).unwrap_or_else(|| panic!("{zone}/{n}"))
    };
    assert!(s("terasa", 1).occupied && !s("terasa", 1).too_small);
    assert!(!s("terasa", 2).occupied && s("terasa", 2).too_small, "a two-top for four");
    assert!(!s("terasa", 6).occupied && !s("terasa", 6).too_small, "the free four-top");
    assert!(s("mala", 3).too_small);
}
