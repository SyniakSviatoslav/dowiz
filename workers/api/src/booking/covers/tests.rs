use super::*;
use crate::booking::guest::Side;
use crate::booking::store::{write_new, write_transition, NewBooking};
use dowiz_kernel::reservation::ReservationStatus::*;

const CEIL: usize = 256 * 1024;
const SLOT: i64 = 29_000_000;
const NOW: i64 = SLOT - 600;

fn plan() -> dowiz_hub::tables::Plan {
    dowiz_hub::tables::from_json(r#"{"zones":[{"id":"z","name":"Z","tables":[{"n":1,"x":85,"y":110,"w":44,"h":44,"seats":8}]}]}"#).unwrap()
}

fn book(t: &mut Table, id: &str, party: i64, slot: i64) {
    let b = NewBooking {
        id: id.into(),
        venue: "v".into(),
        party,
        slot_min: slot,
        occasion: String::new(),
        name: "Ana".into(),
        phone: "069 123 4567".into(),
        phone_key: Some(format!("k-{id}")),
        user_id: None,
        table: None,
        side: Side::Guest,
        now_ms: NOW * 60_000,
    };
    write_new(t, &b, &plan(), NOW).unwrap().unwrap();
}

#[test]
fn the_kitchen_cooks_for_guests_still_coming_in_the_window() {
    let mut t = Table::create(CEIL).unwrap();
    book(&mut t, "a", 4, SLOT);
    book(&mut t, "b", 2, SLOT + 60);
    book(&mut t, "c", 3, SLOT + 90);
    book(&mut t, "late", 6, SLOT + 2000);
    write_transition(&mut t, "b", Confirmed, Side::Venue, "u", "", NOW * 60_000).unwrap().unwrap();
    write_transition(&mut t, "c", CancelledByGuest, Side::Guest, "c", "", NOW * 60_000).unwrap().unwrap();
    assert_eq!(between(&t, SLOT, SLOT + 1440), Covers { guests: 6, unreadable: 0 }, "a requested 4 + b confirmed 2; c cancelled, late outside");
    assert_eq!(between(&t, SLOT + 1440, SLOT + 2880).guests, 6, "the next day: only late");
    assert!(coming(Seated) && !coming(Completed) && !coming(NoShow) && !coming(Declined) && !coming(CancelledByVenue));
}

#[test]
fn no_image_is_no_bookings_and_a_broken_one_is_said() {
    assert_eq!(of_image(None, 0, 1), Ok(Covers::default()));
    assert!(of_image(Some(b"not a table"), 0, 1).is_err());
    let mut t = Table::create(CEIL).unwrap();
    book(&mut t, "a", 5, SLOT);
    let bytes = t.to_bytes().unwrap();
    assert_eq!(of_image(Some(&bytes), SLOT, SLOT + 1).map(|c| c.guests), Ok(5));
}
