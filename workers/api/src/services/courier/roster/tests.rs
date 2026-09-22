//! The roster's rules, each pinned against the defect it came from.

use super::view::*;
use serde_json::json;

/// AN OPEN SHIFT AND A RECORD WRITTEN BEFORE THE FIELD EXISTED are the same
/// thing, because the shift is ended by WRITING `ended_at_ms`. Reading a
/// missing key as "ended" would take every legacy courier off shift at once.
#[test]
fn a_shift_is_open_until_it_is_ended_and_an_absent_field_is_not_an_ending() {
    assert!(shift_is_open(Some(r#"{"started_at_ms":1}"#)));
    assert!(shift_is_open(Some(r#"{"started_at_ms":1,"ended_at_ms":null}"#)));
    assert!(!shift_is_open(Some(r#"{"started_at_ms":1,"ended_at_ms":9}"#)));
    assert!(!shift_is_open(None), "no record is no shift");
    assert!(!shift_is_open(Some("not json")), "and neither is a damaged one");
}

/// THE ID IS THE ID. This row carried the phone number in `id` for its whole
/// life, and an assignment made from it found no courier: the console was
/// showing a key nothing else in the system answered to.
#[test]
fn a_roster_row_carries_the_courier_id_and_the_phone_separately() {
    let rec = json!({
        "phone_encrypted": "+355691234567",
        "full_name_encrypted": "Arben",
        "status": "active",
    });
    let row = roster_row("cou_7", &rec, true);
    assert_eq!(row["id"], "cou_7");
    assert_eq!(row["phone"], "+355691234567");
    assert_eq!(row["name"], "Arben");
    assert_eq!(row["active"], true);
    assert_eq!(row["onShift"], true);
}

/// `active` IS A COMPARISON, NOT A TRUTHINESS TEST: any other status —
/// suspended, invited, whatever is added next — is not active, and the row
/// must not report it as such.
#[test]
fn only_the_active_status_is_active() {
    for (status, want) in [("active", true), ("suspended", false), ("", false), ("ACTIVE", false)] {
        let row = roster_row("c", &json!({ "status": status }), false);
        assert_eq!(row["active"], want, "status {status:?}");
    }
}

/// A SPENT INVITE IS A COURIER and appears in the roster instead; leaving it
/// in this list was an owner seeing two entries for one person and cancelling
/// the wrong one.
#[test]
fn a_used_or_revoked_invite_is_no_longer_outstanding() {
    let base = json!({ "invited_name": "Besa", "expires_at_ms": 100, "created_at_ms": 1 });
    assert!(invite_row("i1", &base, 0).is_some());
    let mut used = base.clone();
    used["used_at_ms"] = json!(50);
    assert!(invite_row("i1", &used, 0).is_none());
    let mut revoked = base.clone();
    revoked["revoked_at_ms"] = json!(50);
    assert!(invite_row("i1", &revoked, 0).is_none());
    // AN EXPLICIT NULL IS NOT A SPENDING. The record is written with both
    // fields null, and reading null as "used" would empty the list.
    let mut fresh = base.clone();
    fresh["used_at_ms"] = json!(null);
    fresh["revoked_at_ms"] = json!(null);
    assert!(invite_row("i1", &fresh, 0).is_some());
}

/// AN EXPIRED INVITE IS STILL LISTED, MARKED. It is the answer to "they say
/// the code does not work", and an owner who cannot see it has run out will
/// send the same dead code again.
#[test]
fn an_expired_invite_is_listed_and_says_so() {
    let rec = json!({ "invited_name": "Besa", "expires_at_ms": 100, "created_at_ms": 1 });
    assert_eq!(invite_row("i1", &rec, 99).unwrap()["expired"], false);
    // The boundary belongs to expiry: at the millisecond it expires, it has.
    assert_eq!(invite_row("i1", &rec, 100).unwrap()["expired"], true);
    assert_eq!(invite_row("i1", &rec, 101).unwrap()["expired"], true);
}

/// EIGHT DIGITS, COUNTED RATHER THAN MATCHED, because the console sends what a
/// person typed. A pattern rule here would refuse a number that works.
#[test]
fn a_phone_is_counted_in_digits_and_a_name_may_not_be_blank() {
    assert_eq!(
        invite_fields(" +355 69 123 4567 ", " Arben "),
        Ok(("+355 69 123 4567".into(), "Arben".into())),
        "the spaces inside a number are the owner's business"
    );
    assert_eq!(invite_fields("(069) 12-34-567", "A").map(|x| x.0), Ok("(069) 12-34-567".into()));
    assert_eq!(invite_fields("1234567", "A"), Err("that does not look like a phone number"));
    assert_eq!(invite_fields("+355 69 123 4567", "   "), Err("who is this code for?"));
}

/// THE CODE IS MINTED FROM THE PLATFORM'S BYTES AND THE HUB'S ALPHABET, which
/// is the shape that fixed "no randomness available" — a Worker has no
/// `/dev/urandom`, and an owner hiring somebody met that as a 500.
#[test]
fn a_code_is_sixteen_characters_of_the_hubs_own_alphabet() {
    let hex = "0123456789abcdef".repeat(4); // 32 bytes, as two UUIDs give
    let minted = code_from_entropy(&hex, |c| format!("sha({c})")).expect("32 bytes is enough");
    assert_eq!(minted.code.chars().count(), 16);
    assert!(minted.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()), "{}", minted.code);
    assert_eq!(minted.hash, format!("sha({})", minted.code), "the digest is of the code itself");
}

/// TOO LITTLE ENTROPY IS NOTHING, never a short code. Sixteen characters from a
/// 32-symbol alphabet is 80 bits; eight would be 40, and nothing in the answer
/// would look different.
#[test]
fn too_little_entropy_mints_nothing() {
    assert!(code_from_entropy("00112233", |c| c.into()).is_none(), "4 bytes");
    assert!(code_from_entropy("", |c| c.into()).is_none());
    // A stray non-hex pair is DROPPED, not guessed at -- and dropping enough of
    // them takes the entropy below the floor, where it is refused.
    let mut hex = "zz".to_string();
    hex.push_str(&"ab".repeat(15));
    assert!(code_from_entropy(&hex, |c| c.into()).is_none(), "15 good pairs is not 16");
}
