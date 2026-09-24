use super::*;
use dowiz_hub::tables::from_json;
use dowiz_kernel::reservation::ReservationStatus::*;

const CEIL: usize = 256 * 1024;
const SLOT: i64 = 29_000_000;
const NOW: i64 = SLOT - 600; // ten hours before the sitting

fn plan() -> floor::Plan {
    from_json(
        r#"{"zones":[{"id":"terasa","name":"Terasa","tables":[
             {"n":1,"x":85,"y":110,"w":44,"h":44,"seats":4},
             {"n":2,"x":155,"y":130,"w":40,"h":40,"seats":2}]}]}"#,
    )
    .expect("the fixture plan must read")
}

fn guest(id: &str, phone_key: &str, table: Option<(&str, i64)>) -> NewBooking {
    NewBooking {
        id: id.into(),
        venue: "v1".into(),
        party: 2,
        slot_min: SLOT,
        occasion: String::new(),
        name: "Ana".into(),
        phone: "069 123 4567".into(),
        phone_key: Some(phone_key.into()),
        user_id: None,
        table: table.map(|(z, n)| (z.to_string(), n)),
        side: Side::Guest,
        now_ms: NOW * 60_000,
    }
}

fn book(t: &mut Table, b: &NewBooking) -> Result<(String, bool), Refusal> {
    write_new(t, b, &plan(), NOW).expect("the image write itself must not fail")
}

fn status_of(t: &Table, id: &str) -> String {
    fold_status(&events_of(t, id)).expect("history replays").as_str().to_string()
}

// ── a guest books ───────────────────────────────────────────────────────────

/// THE DEFECT THIS ROW CLOSES: a guest could not book at all. Written without
/// an account, it lands REQUESTED, signed CUSTOMER, holding its table.
#[test]
fn a_guest_booking_lands_requested_and_holds_its_table() {
    let mut t = Table::create(CEIL).unwrap();
    assert_eq!(book(&mut t, &guest("rsv_a", "k1", Some(("terasa", 1)))), Ok(("REQUESTED".into(), true)));
    assert_eq!(status_of(&t, "rsv_a"), "REQUESTED");
    assert_eq!(events_of(&t, "rsv_a")[0].actor, "CUSTOMER");
    assert_eq!(held_tables(&t).len(), 1, "a request holds its table");
    // Without a table is a booking too: the venue seats them where it likes.
    assert_eq!(book(&mut t, &guest("rsv_b", "k2", None)), Ok(("REQUESTED".into(), true)));
}

/// The same request retried is the same booking: no second write, no token.
#[test]
fn a_retry_is_answered_with_the_first_booking_and_writes_nothing() {
    let mut t = Table::create(CEIL).unwrap();
    book(&mut t, &guest("rsv_a", "k1", Some(("terasa", 1)))).unwrap();
    let root = t.root();
    assert_eq!(book(&mut t, &guest("rsv_a", "k1", Some(("terasa", 1)))), Ok(("REQUESTED".into(), false)));
    assert_eq!(t.root(), root, "a retry moved the image");
}

#[test]
fn a_table_already_held_is_refused_by_name() {
    let mut t = Table::create(CEIL).unwrap();
    book(&mut t, &guest("rsv_a", "k1", Some(("terasa", 1)))).unwrap();
    let (code, why) = book(&mut t, &guest("rsv_b", "k2", Some(("terasa", 1)))).unwrap_err();
    assert_eq!(code, 409);
    assert!(why.contains("table 1"), "{why}");
    // The positive twin: the next table over is free.
    assert!(book(&mut t, &guest("rsv_c", "k2", Some(("terasa", 2)))).is_ok());
}

/// ONE PHONE CANNOT FILL THE ROOM. The fourth live booking is refused; the
/// twin: once one is cancelled, the phone may book again.
#[test]
fn a_phone_past_its_cap_is_refused_until_one_is_cancelled() {
    let mut t = Table::create(CEIL).unwrap();
    for i in 0..MAX_OPEN_PER_PHONE {
        book(&mut t, &guest(&format!("rsv_{i}"), "k1", None)).expect("under the cap");
    }
    let (code, why) = book(&mut t, &guest("rsv_over", "k1", None)).unwrap_err();
    assert_eq!(code, 429, "{why}");
    // Another phone is not affected.
    assert!(book(&mut t, &guest("rsv_other", "k2", None)).is_ok());
    // Cancel one; the index moves with the record, and the cap frees.
    write_transition(&mut t, "rsv_0", CancelledByGuest, Side::Guest, "rsv_0", "", NOW * 60_000)
        .unwrap()
        .unwrap();
    assert!(book(&mut t, &guest("rsv_over", "k1", None)).is_ok());
}

/// The venue's own booking (a phone call) is not capped and lands CONFIRMED,
/// through REQUESTED, both events signed VENUE -- the fold replays it.
#[test]
fn the_venues_own_booking_lands_confirmed_and_is_signed_venue() {
    let mut t = Table::create(CEIL).unwrap();
    let mut b = guest("rsv_v", "k1", Some(("terasa", 2)));
    b.side = Side::Venue;
    assert_eq!(book(&mut t, &b), Ok(("CONFIRMED".into(), true)));
    let ev = events_of(&t, "rsv_v");
    assert_eq!(ev.iter().map(|e| e.to_status.as_str()).collect::<Vec<_>>(), ["REQUESTED", "CONFIRMED"]);
    assert!(ev.iter().all(|e| e.actor == "VENUE"));
    assert_eq!(status_of(&t, "rsv_v"), "CONFIRMED");
}

// ── the venue moves it ──────────────────────────────────────────────────────

/// Cancel is an EVENT, not an erasure: the record and its history stay, the
/// table is released, and the fold says CANCELLED_BY_VENUE.
#[test]
fn a_venue_cancel_appends_and_releases_the_table() {
    let mut t = Table::create(CEIL).unwrap();
    book(&mut t, &guest("rsv_a", "k1", Some(("terasa", 1)))).unwrap();
    let seq = write_transition(&mut t, "rsv_a", CancelledByVenue, Side::Venue, "u1", "closed", 1)
        .unwrap()
        .unwrap();
    assert_eq!(seq, 2);
    assert!(t.get(K_RSV, "rsv_a").is_some(), "the booking was erased");
    assert_eq!(events_of(&t, "rsv_a").len(), 2);
    assert_eq!(status_of(&t, "rsv_a"), "CANCELLED_BY_VENUE");
    assert!(held_tables(&t).is_empty(), "a cancelled booking still holds its table");
    // And the table is bookable again.
    assert!(book(&mut t, &guest("rsv_b", "k2", Some(("terasa", 1)))).is_ok());
}

#[test]
fn an_illegal_move_is_refused_and_writes_nothing() {
    let mut t = Table::create(CEIL).unwrap();
    book(&mut t, &guest("rsv_a", "k1", Some(("terasa", 1)))).unwrap();
    let root = t.root();
    let (code, _) = write_transition(&mut t, "rsv_a", Seated, Side::Venue, "u1", "", 1).unwrap().unwrap_err();
    assert_eq!(code, 409, "seating an unconfirmed request");
    assert_eq!(t.root(), root);
    // Twin: confirm, then seat.
    write_transition(&mut t, "rsv_a", Confirmed, Side::Venue, "u1", "", 1).unwrap().unwrap();
    assert_eq!(write_transition(&mut t, "rsv_a", Seated, Side::Venue, "u1", "", 2).unwrap(), Ok(3));
    assert_eq!(held_tables(&t).len(), 1, "a seated party still holds the table");
    assert_eq!(
        write_transition(&mut t, "rsv_nope", Confirmed, Side::Venue, "u1", "", 1).unwrap().unwrap_err().0,
        404
    );
}

/// The index is the record's, whoever writes it: a transition keeps every key
/// the record implies. Rebuilding from the records alone changes nothing.
#[test]
fn a_transition_keeps_the_index_the_record_implies() {
    let mut t = Table::create(CEIL).unwrap();
    let mut b = guest("rsv_a", "k1", Some(("terasa", 1)));
    b.user_id = Some("u9".into());
    book(&mut t, &b).unwrap();
    write_transition(&mut t, "rsv_a", Confirmed, Side::Venue, "u1", "", 1).unwrap().unwrap();
    let before = t.root();
    // Both kinds: a rebuild drops every owner list, and an event owns none.
    t.rebuild_index(&[K_RSV, K_EV], |kind, id, json| match kind {
        K_RSV => index_of(id, &serde_json::from_str(json).unwrap_or(Value::Null)),
        _ => Vec::new(),
    });
    assert_eq!(t.root(), before, "a writer and the indexer disagree");
    assert_eq!(t.scan("rsv.phone/k1/").len(), 1);
    assert_eq!(t.scan("rsv.user/u9/").len(), 1);
}

// ── the day's list ──────────────────────────────────────────────────────────

#[test]
fn the_days_list_is_the_days_with_the_venues_moves() {
    let mut t = Table::create(CEIL).unwrap();
    let mut late = guest("rsv_late", "k1", None);
    late.slot_min = SLOT + 120;
    book(&mut t, &late).unwrap();
    book(&mut t, &guest("rsv_early", "k2", Some(("terasa", 1)))).unwrap();
    let mut other_day = guest("rsv_tomorrow", "k3", None);
    other_day.slot_min = SLOT + 1440;
    book(&mut t, &other_day).unwrap();

    let rows = day_rows(&t, SLOT - 60, SLOT + 1380);
    let ids: Vec<&str> = rows.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["rsv_early", "rsv_late"], "the day, earliest first, nothing else");
    assert_eq!(rows[0]["status"], "REQUESTED");
    assert_eq!(rows[0]["name"], "Ana");
    assert_eq!(rows[0]["tableN"], 1);
    assert_eq!(rows[0]["next"], json!(["CONFIRMED", "DECLINED", "CANCELLED_BY_VENUE"]));
}

// ── the free-cancel window, wired (audit D29) ───────────────────────────────

/// A guest cancelling inside the venue's free window (120 minutes before the
/// sitting) is marked late on the event, the record and the console's row;
/// ten hours ahead is not. Neither is refused: late is a fact, not a penalty.
#[test]
fn a_guest_cancel_inside_the_free_window_is_marked_late() {
    let mut t = Table::create(CEIL).unwrap();
    book(&mut t, &guest("rsv_early", "k1", None)).unwrap();
    book(&mut t, &guest("rsv_late", "k2", None)).unwrap();
    write_transition(&mut t, "rsv_early", CancelledByGuest, Side::Guest, "g", "", NOW * 60_000).unwrap().unwrap();
    write_transition(&mut t, "rsv_late", CancelledByGuest, Side::Guest, "g", "", (SLOT - 30) * 60_000)
        .unwrap()
        .unwrap();
    let last = |id: &str| events_of(&t, id).last().map(|e| e.seq).unwrap();
    let ev = |id: &str| -> Value {
        serde_json::from_str(&t.get(K_EV, &ev_key(id, last(id))).unwrap()).unwrap()
    };
    assert_eq!(ev("rsv_early")["late"], json!(false), "ten hours ahead is free");
    assert_eq!(ev("rsv_late")["late"], json!(true), "thirty minutes ahead is late");
    let rows = day_rows(&t, SLOT - 60, SLOT + 60);
    let row = |id: &str| rows.iter().find(|r| r["id"] == id).cloned().unwrap();
    assert_eq!(row("rsv_late")["lateCancel"], json!(true));
    assert_eq!(row("rsv_early")["lateCancel"], json!(false));
    assert_eq!(row("rsv_late")["status"], "CANCELLED_BY_GUEST", "late is not refused");
}
