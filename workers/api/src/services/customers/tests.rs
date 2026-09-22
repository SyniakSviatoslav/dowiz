//! The fold, pinned against what it gets wrong. The mask that goes with it is
//! `dowiz_hub::redact`, tested where it lives.

use super::roll::*;
use serde_json::json;

// ── the fold ────────────────────────────────────────────────────────────────

fn order(phone: &str, name: &str, at: i64, total: i64, tip: i64, status: &str) -> serde_json::Value {
    json!({
        "contact": { "phone": phone, "name": name },
        "created_at_ms": at, "total": total, "tip": tip, "status": status
    })
}

fn rolled(orders: &[serde_json::Value], sort: Sort) -> Vec<Row> {
    roll(orders, |p| format!("k:{p}"), |n| n.to_string(), |p| p.to_string(), sort)
}

/// A REFUSED ORDER IS NOT MONEY TAKEN, AND THE TIP WENT TO THE COURIER. A
/// "spent" column that counted either would rank a generous customer, or one
/// the venue kept turning away, above a profitable one.
#[test]
fn a_refused_order_is_a_visit_without_money_and_the_tip_is_never_the_venues() {
    let os = [
        order("1", "A", 10, 1000, 200, "DELIVERED"),
        order("1", "A", 20, 5000, 0, "REJECTED"),
        order("1", "A", 30, 4000, 500, "CANCELLED"),
    ];
    let r = rolled(&os, Sort::Recent);
    assert_eq!(r.len(), 1, "one person");
    assert_eq!(r[0].orders, 3, "three visits, and the venue refused two of them");
    assert_eq!(r[0].spent, 800, "1000 less the 200 tip; the refused orders are nothing");
    assert_eq!(r[0].last_at, 30);
}

/// THE KEY IS WHERE THE SPELLING RULE LIVES, and the test below could not
/// reach it: `rolled` hands `roll` a pass-through closure, so the fold joins
/// on whatever string it is given and the real normalisation — in
/// `customer_key` — was never exercised by anything. The test above it even
/// says "two spellings that hash alike" and passed the SAME spelling twice.
///
/// One person writes their number four ways over a year. If each is a row, the
/// venue's customer list is four strangers who all live at the same address,
/// and the reveal audit is four trails.
#[test]
fn four_spellings_of_one_number_are_one_customer() {
    let secret = b"a-test-signing-key";
    let canonical = super::handlers::customer_key(secret, "+355691234567");
    for spelling in ["+355 69 123 45 67", "00355691234567", "355-69-1234567", "+355 (69) 1234567"] {
        assert_eq!(
            super::handlers::customer_key(secret, spelling),
            canonical,
            "{spelling:?} is the same person"
        );
    }
    // AND A DIFFERENT NUMBER IS A DIFFERENT PERSON, which is the half that
    // stops "normalise harder" from becoming "everyone is one customer".
    assert_ne!(super::handlers::customer_key(secret, "+355691234568"), canonical);

    // THE NATIONAL FORM IS STILL A SECOND HANDLE, and this asserts the KNOWN
    // GAP rather than hiding it: `069…` needs the venue's dialling code to
    // become `+35569…`, and guessing one is how a Kosovan number becomes an
    // Albanian customer. When that argument is added, this line changes to
    // `assert_eq!` in the same commit.
    assert_ne!(
        super::handlers::customer_key(secret, "0691234567"),
        canonical,
        "the national form is a known, written-down gap, not an accident"
    );
}

/// AND IT IS KEYED, WHICH A HASH OF A PHONE NUMBER HAS TO BE.
///
/// `storefront.rs` used a bare `sha256_hex(phone)` for the customer record
/// under a comment claiming the table could be joined "without holding the
/// number in the clear". An Albanian mobile is seven digits after a fixed
/// prefix: the whole space is seconds of brute force, and every row's number
/// falls out. A hash masks nothing when its inputs can be enumerated.
#[test]
fn the_same_number_under_two_secrets_is_two_different_handles() {
    let a = super::handlers::customer_key(b"one-venue-secret", "+355691234567");
    let b = super::handlers::customer_key(b"another-secret", "+355691234567");
    assert_ne!(a, b, "without the secret the handle is guessable from the number alone");
}

/// THE ROW IS THE PERSON, not the order. The key is what joins them, so two
/// spellings that hash alike are one row and the name shown is the one from
/// the order the venue saw first.
#[test]
fn one_person_is_one_row_however_many_orders_they_placed() {
    let os = [
        order("+355 69 1", "Arben", 10, 100, 0, "DELIVERED"),
        order("+355 69 1", "A.", 20, 200, 0, "DELIVERED"),
    ];
    let r = rolled(&os, Sort::Recent);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "Arben", "the first name seen, not the latest");
    assert_eq!(r[0].spent, 300);
}

/// AN ORDER WITHOUT A PHONE IS NOT A PERSON. The number is optional by
/// operator decision, so such an order is simply not attributable — and it
/// must not become a row keyed on an empty string, which would gather every
/// anonymous order in the venue's history into one fictitious customer.
#[test]
fn an_order_with_no_phone_is_skipped_and_never_becomes_one_fictitious_customer() {
    let os = [
        json!({ "contact": { "name": "A" }, "created_at_ms": 10, "total": 100, "status": "DELIVERED" }),
        json!({ "created_at_ms": 20, "total": 100, "status": "DELIVERED" }),
        order("1", "B", 30, 100, 0, "DELIVERED"),
    ];
    let r = rolled(&os, Sort::Recent);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "B");
}

/// TIES BREAK ON RECENCY, or the list wobbles between two requests that asked
/// the same question.
#[test]
fn every_sort_breaks_its_ties_on_who_was_here_last() {
    let os = [
        order("old", "O", 10, 1000, 0, "DELIVERED"),
        order("new", "N", 99, 1000, 0, "DELIVERED"),
    ];
    for sort in [Sort::Spent, Sort::Orders] {
        let r = rolled(&os, sort);
        assert_eq!(r[0].name, "N", "{sort:?}: equal on the measure, later in time");
    }
    assert_eq!(rolled(&os, Sort::Recent)[0].name, "N");
}

/// THE SORTS ACTUALLY SORT, and they are different questions.
#[test]
fn spent_and_orders_are_two_different_orderings() {
    let os = [
        order("big", "B", 10, 9000, 0, "DELIVERED"),
        order("often", "F", 20, 100, 0, "DELIVERED"),
        order("often", "F", 30, 100, 0, "DELIVERED"),
        order("often", "F", 40, 100, 0, "DELIVERED"),
    ];
    assert_eq!(rolled(&os, Sort::Spent)[0].name, "B");
    assert_eq!(rolled(&os, Sort::Orders)[0].name, "F");
    assert_eq!(rolled(&os, Sort::Recent)[0].name, "F", "newest first by default");
}

/// AN UNKNOWN `sort=` IS THE DEFAULT, not an error and not an empty list: a
/// console with a stale query string still answers the useful question.
#[test]
fn an_unknown_sort_falls_back_to_newest_first() {
    assert_eq!(Sort::of(None), Sort::Recent);
    assert_eq!(Sort::of(Some("")), Sort::Recent);
    assert_eq!(Sort::of(Some("SPENT")), Sort::Recent, "and it is case-sensitive");
    assert_eq!(Sort::of(Some("spent")), Sort::Spent);
    assert_eq!(Sort::of(Some("orders")), Sort::Orders);
}

// ── the record: what a fold cannot know ─────────────────────────────────────
//
// §3.1 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22. THE RULE THE WHOLE MODULE
// IS JUDGED BY IS `mod.rs`'s: "the venue holds exactly what it held before".
// A CRM by definition makes it hold more, so every field has to be something
// the venue would otherwise write on a paper card by the till — and nothing
// that repeats a fold, because a stored copy is a second number that can
// disagree, and the one that disagrees is always the stored one.

use super::record::{merge, Card};

fn card() -> Card {
    Card::default()
}

fn field<'a>(j: &'a str, k: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(j).ok()?.get(k).cloned()
}

/// THE ALLOW-LIST IS THE RECORD. `orders`, `spent`, `last_at`, `name` and
/// `phone` are folds over the order log; a record that also held them would
/// eventually disagree with the fold, and the console would show two numbers
/// for one question.
#[test]
fn the_record_never_holds_what_the_fold_already_knows() {
    let mut c = card();
    c.note = Some("always asks for extra ginger".into());
    let out = merge(r#"{"id":"cust_1","phone_hash":"k1","name":"Arben","created_at_ms":10}"#, &c, 99)
        .expect("a note is allowed");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(|s| s.as_str()).collect();
    for banned in ["orders", "spent", "last_at", "name", "phone", "balance", "stamps", "tier", "score"] {
        assert!(!keys.contains(&banned), "{banned} is a fold or a rating, not a record: {keys:?}");
    }
    // The three the placement path owns survive an owner's edit.
    assert_eq!(field(&out, "id").unwrap(), "cust_1");
    assert_eq!(field(&out, "phone_hash").unwrap(), "k1");
    assert_eq!(field(&out, "created_at_ms").unwrap(), 10, "not reset by an edit");
    assert_eq!(field(&out, "updated_at_ms").unwrap(), 99, "the request's clock, not a handler's");
}

#[test]
fn a_note_longer_than_the_paper_card_is_refused() {
    let mut c = card();
    c.note = Some("ë".repeat(281));
    assert!(merge("{}", &c, 1).is_err(), "281 CHARACTERS, not bytes");
    c.note = Some("ë".repeat(280));
    assert!(merge("{}", &c, 1).is_ok());
}

/// A TAG IS NOT A TIER (`CLAUDE.md:102`). The closed list is the whole defence:
/// a free-text tag field is where `vip`, `difficult` and `bad payer` arrive,
/// and then the venue is rating a participant.
#[test]
fn a_tag_outside_the_closed_list_is_refused() {
    let mut c = card();
    for ranking in ["premium", "difficult", "gold", "bad-payer"] {
        c.tags = Some(vec![ranking.into()]);
        assert!(merge("{}", &c, 1).is_err(), "{ranking} would be a rating of a person");
    }
    c.tags = Some(vec!["regular".into(), "office_lunch".into()]);
    let out = merge("{}", &c, 1).expect("two tags from the list");
    assert_eq!(field(&out, "tags").unwrap(), serde_json::json!(["regular", "office_lunch"]));
}

#[test]
fn the_same_tag_twice_is_one_tag() {
    let mut c = card();
    c.tags = Some(vec!["regular".into(), "regular".into()]);
    let out = merge("{}", &c, 1).unwrap();
    assert_eq!(field(&out, "tags").unwrap(), serde_json::json!(["regular"]));
}

/// THE ONE FIELD THAT PREVENTS HARM, and the reason it is a code list: a
/// customer recorded as allergic to `shelfish` matches no dish filter, so the
/// check that was supposed to protect them silently passes.
#[test]
fn an_allergen_outside_the_eu_fourteen_is_refused() {
    let mut c = card();
    c.allergens = Some(vec!["shelfish".into()]);
    assert!(merge("{}", &c, 1).is_err());
    c.allergens = Some(vec!["crustaceans".into(), "milk".into()]);
    let out = merge("{}", &c, 1).expect("two of the fourteen");
    assert_eq!(field(&out, "allergens").unwrap(), serde_json::json!(["crustaceans", "milk"]));
}

/// THE YEAR IS THE FIELD THAT MAKES THE RECORD SENSITIVE. A greeting needs the
/// day; age needs the year, and nothing here needs age.
#[test]
fn a_birthday_with_a_year_is_refused_and_so_is_a_date_that_does_not_exist() {
    let mut c = card();
    for bad in ["1990-05-02", "05-02-1990", "13-01", "00-10", "02-30", "5-2", "tomorrow"] {
        c.birthday_md = Some(bad.into());
        assert!(merge("{}", &c, 1).is_err(), "{bad} is not MM-DD");
    }
    for good in ["05-02", "01-01", "12-31", "02-29"] {
        c.birthday_md = Some(good.into());
        assert!(merge("{}", &c, 1).is_ok(), "{good} is a day of the year");
    }
}

#[test]
fn a_language_the_venue_does_not_speak_is_refused() {
    let mut c = card();
    c.lang = Some("pt".into());
    assert!(merge("{}", &c, 1).is_err());
    for l in dowiz_hub::consent::LANGS {
        c.lang = Some(l.into());
        assert!(merge("{}", &c, 1).is_ok(), "{l} is one of the three the storefront speaks");
    }
}

/// A FIELD NOT SENT IS A FIELD NOT TOUCHED, and an EMPTY one is a deletion.
/// Without the difference, a console that renders one tab and saves it wipes
/// the allergens entered on another.
#[test]
fn an_absent_field_is_untouched_and_an_empty_one_clears() {
    let mut c = card();
    c.note = Some("extra ginger".into());
    c.allergens = Some(vec!["milk".into()]);
    let one = merge("{}", &c, 1).unwrap();

    let mut c2 = card();
    c2.lang = Some("sq".into());
    let two = merge(&one, &c2, 2).unwrap();
    assert_eq!(field(&two, "note").unwrap(), "extra ginger", "an absent field is not a deletion");
    assert_eq!(field(&two, "allergens").unwrap(), serde_json::json!(["milk"]));

    let mut c3 = card();
    c3.note = Some("  ".into());
    c3.allergens = Some(vec![]);
    let three = merge(&two, &c3, 3).unwrap();
    assert!(field(&three, "note").is_none(), "an empty note is a deletion");
    assert!(field(&three, "allergens").is_none(), "and so is an empty list");
    assert_eq!(field(&three, "lang").unwrap(), "sq", "the untouched one stays");
}

/// THE BODY IS A CLOSED SHAPE. A console that posts `spent` gets a 400 rather
/// than a silently ignored field, because a field that is accepted and dropped
/// is how a number nobody stores ends up believed.
#[test]
fn a_body_that_carries_a_fold_is_refused_before_it_is_merged() {
    assert!(serde_json::from_str::<Card>(r#"{"note":"x"}"#).is_ok());
    for banned in [r#"{"spent":100}"#, r#"{"orders":3}"#, r#"{"name":"A"}"#, r#"{"tier":"gold"}"#] {
        assert!(serde_json::from_str::<Card>(banned).is_err(), "{banned} is not a record field");
    }
}

/// A TABLE NUMBER IS A SHORT LABEL. Free text here is where a second note
/// would live, with none of the note's length rule.
#[test]
fn the_usual_table_is_a_label_not_a_paragraph() {
    let mut c = card();
    c.usual_table = Some("7".into());
    assert!(merge("{}", &c, 1).is_ok());
    c.usual_table = Some("x".repeat(17));
    assert!(merge("{}", &c, 1).is_err());
}
