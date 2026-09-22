//! PURE. What a roster row says, what an invite row says, and what an invite
//! is allowed to be minted from.
//!
//! EVERY FUNCTION HERE HAS A DEFECT BEHIND IT. The id that was really a phone
//! number, the spent invite that kept being offered, the expired code an owner
//! could not see had expired, and the code minted from randomness a Worker
//! does not have. They were all inside a handler that needs a Durable Object
//! to run, which is why none of them had a test.

use serde_json::{json, Value};

use crate::identity_store::{i_of, s_of};

/// Is this venue's stored shift record an OPEN one?
///
/// A shift with no `ended_at_ms` is open; a missing record is no shift at all.
/// AN ABSENT KEY AND AN EXPLICIT NULL MEAN THE SAME THING here and that is not
/// an accident: the shift is ended by writing the field, so a record written
/// before that field existed is still an open shift.
pub fn shift_is_open(stored: Option<&str>) -> bool {
    stored
        .and_then(|j| serde_json::from_str::<Value>(j).ok())
        .filter(|x| x.get("ended_at_ms").is_none_or(Value::is_null))
        .is_some()
}

/// One person on the roster.
///
/// THE ID IS THE ID. This row once carried the phone number in the `id` field,
/// and an assignment made from it found no courier — the console was showing a
/// key that nothing else in the system answered to.
pub fn roster_row(id: &str, rec: &Value, on_shift: bool) -> Value {
    json!({
        "id": id,
        "phone": s_of(rec, "phone_encrypted"),
        "name": s_of(rec, "full_name_encrypted"),
        "active": s_of(rec, "status") == "active",
        "onShift": on_shift,
    })
}

/// One outstanding invite, or `None` if it is no longer outstanding.
///
/// A SPENT OR REVOKED INVITE IS NOT AN INVITE: it has become a courier, and it
/// appears in the roster instead. An EXPIRED one is still listed, marked — the
/// owner needs to see that the code they sent has run out, because that is the
/// answer to "they say it does not work".
pub fn invite_row(id: &str, rec: &Value, now_ms: i64) -> Option<Value> {
    let spent = |k: &str| rec.get(k).is_some_and(|v| !v.is_null());
    if spent("used_at_ms") || spent("revoked_at_ms") {
        return None;
    }
    let until = i_of(rec, "expires_at_ms");
    Some(json!({
        "id": id,
        "name": s_of(rec, "invited_name"),
        "madeMs": i_of(rec, "created_at_ms"),
        "untilMs": until,
        "expired": now_ms >= until,
    }))
}

/// What an owner typed, checked and tidied, or the refusal in the owner's own
/// words.
///
/// EIGHT DIGITS, COUNTED RATHER THAN MATCHED, because the console sends what a
/// person typed: `+355 69 123 4567`, brackets, dashes and all. A stricter rule
/// here would refuse a number that works.
pub fn invite_fields(phone: &str, name: &str) -> Result<(String, String), &'static str> {
    let phone = phone.trim().to_string();
    if phone.chars().filter(char::is_ascii_digit).count() < 8 {
        return Err("that does not look like a phone number");
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("who is this code for?");
    }
    Ok((phone, name))
}

/// A minted code and the digest that is all the platform keeps of it.
pub struct InviteCode {
    pub code: String,
    pub hash: String,
}

/// Turn platform entropy into an invite code.
///
/// THE BYTES ARE THE PLATFORM'S, THE ALPHABET IS THE HUB'S. `new_invite_code`
/// reads `/dev/urandom`, which a Worker does not have, so it failed with "no
/// randomness available" while an owner was trying to hire somebody. Two UUIDs
/// give 32 hex-encoded bytes and the hub renders sixteen of them through the
/// one alphabet both implementations share, so a code minted at the edge is
/// indistinguishable from one minted natively.
///
/// The dashes are stripped by the caller; anything that is not a hex pair is
/// dropped rather than guessed at, and too little entropy is `None` rather
/// than a short code.
pub fn code_from_entropy(hex: &str, digest: impl Fn(&str) -> String) -> Option<InviteCode> {
    let raw: Vec<u8> = hex
        .as_bytes()
        .chunks(2)
        .filter_map(|c| u8::from_str_radix(std::str::from_utf8(c).ok()?, 16).ok())
        .collect();
    let code = dowiz_hub::roster::invite_code_from(&raw)?;
    let hash = digest(&code);
    Some(InviteCode { code, hash })
}
