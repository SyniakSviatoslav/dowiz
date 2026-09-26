//! PURE. "Table 5", said aloud, found in the room -- or refused.
//!
//! The room is `command::sitting::room`'s cards, the same the waiter's screen
//! draws, so a voice command and a tap look at one list. A table number finds
//! the sitting whose table IS that number; failing that, the one whose table's
//! only digits are it ("T5", "Terrace 5"). Two sittings at one number, or two
//! rounds a command could mean, are refused: the screen is where a waiter picks
//! between them.

use serde_json::Value;

/// A round, and the version an edit or a payment must quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundRef {
    pub id: String,
    pub seq: u64,
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
fn int(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}
fn rounds(sitting: &Value) -> &[Value] {
    sitting.get("rounds").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}
fn round_ref(r: &Value) -> RoundRef {
    RoundRef { id: s(r, "id").to_string(), seq: r.get("seq").and_then(Value::as_u64).unwrap_or(0) }
}

/// The sittings at a table: its exact name first (case aside), then -- for a
/// number -- the tables whose only digits are that number.
pub fn at_table<'a>(sittings: &'a [Value], want: &str) -> Vec<&'a Value> {
    let want = want.trim().to_lowercase();
    let exact: Vec<&Value> = sittings.iter().filter(|x| s(x, "table").trim().to_lowercase() == want).collect();
    if !exact.is_empty() || want.is_empty() || !want.chars().all(|c| c.is_ascii_digit()) {
        return exact;
    }
    sittings
        .iter()
        .filter(|x| {
            let digits: String = s(x, "table").chars().filter(char::is_ascii_digit).collect();
            digits == want
        })
        .collect()
}

/// The one sitting at a table, or none; two is refused.
fn one_sitting<'a>(sittings: &'a [Value], table: &str) -> Result<Option<&'a Value>, &'static str> {
    match at_table(sittings, table).as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(*one)),
        _ => Err("two_sittings"),
    }
}

/// The round a dish may still be added to at a table: one the kitchen has
/// not taken (`PENDING`/`CONFIRMED`, `room/logic.js` stage "before") and not
/// paid. `None` = there is none, so a new round is the answer.
pub fn editable(sittings: &[Value], table: &str) -> Result<Option<RoundRef>, &'static str> {
    let Some(sit) = one_sitting(sittings, table)? else { return Ok(None) };
    let open: Vec<&Value> = rounds(sit)
        .iter()
        .filter(|r| matches!(s(r, "status"), "PENDING" | "CONFIRMED") && s(r, "payment_status") != "paid")
        .collect();
    match open.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(round_ref(one))),
        _ => Err("two_rounds"),
    }
}

/// What is still owed on one round, in minor units: its total less what its
/// payments settled (the card's `paid`), never below zero.
pub fn owed(r: &Value) -> i64 {
    if s(r, "payment_status") == "paid" {
        return 0;
    }
    (int(r, "total") - int(r, "paid")).max(0)
}

/// May this round take a payment now? `pay::decide`'s refusals, read from the
/// card: money not kept (a refused status) or on its way back, a guest's round
/// nobody has confirmed, nothing owed.
fn payable_round(r: &Value) -> bool {
    let st = s(r, "status");
    let guest_waiting = st == "PENDING" && s(r, "placed_by") == dowiz_hub::room::pay::GUEST;
    crate::services::orders::status::took_money(st) && st != "REFUNDING" && !guest_waiting && owed(r) > 0
}

/// The one round at a table that "paid" can mean, and what it owes.
pub fn payable(sittings: &[Value], table: &str) -> Result<(RoundRef, i64), &'static str> {
    let Some(sit) = one_sitting(sittings, table)? else { return Err("no_table") };
    let due: Vec<&Value> = rounds(sit).iter().filter(|r| payable_round(r)).collect();
    match due.as_slice() {
        [] => Err("nothing_owed"),
        [one] => Ok((round_ref(one), owed(one))),
        _ => Err("two_rounds"),
    }
}

/// The room at a glance, for "status": open tables, rounds waiting.
pub fn glance(sittings: &[Value]) -> (usize, usize) {
    let waiting = sittings.iter().flat_map(|x| rounds(x)).filter(|r| s(r, "status") == "PENDING").count();
    (sittings.len(), waiting)
}

#[cfg(test)]
mod tests;
