//! PURE. The rules every room command shares: who may touch a round in which
//! status, why a line may leave it, what a line set costs, and how the shelf
//! follows the lines. `amend`, `transfer` and `pay` are each a sequence of
//! these; putting them in one place is what keeps the three from becoming
//! three pricers of the same round.

use super::Refused;
use serde_json::{json, Value};

/// Why a line left a round, or why it was not charged. A CLOSED set, the
/// `WasteReason` pattern (`dowiz_hub::stock`): a reason is a word the owner can
/// count at midnight, and a free-text field is a word nobody can.
///
/// `Other` carries its text, bounded, for the case the set did not foresee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoidReason {
    Mistake,
    GuestChanged,
    Unavailable,
    Dropped,
    Other(String),
}

/// The longest `other:` text kept. A reason is a line on a report, not a note.
pub const OTHER_MAX_CHARS: usize = 140;

impl VoidReason {
    pub fn parse(s: &str) -> Option<VoidReason> {
        match s.trim() {
            "mistake" => Some(VoidReason::Mistake),
            "guest_changed" => Some(VoidReason::GuestChanged),
            "unavailable" => Some(VoidReason::Unavailable),
            "dropped" => Some(VoidReason::Dropped),
            other => {
                let text = other.strip_prefix("other:")?.trim();
                (!text.is_empty() && text.chars().count() <= OTHER_MAX_CHARS)
                    .then(|| VoidReason::Other(text.to_string()))
            }
        }
    }

    pub fn word(&self) -> String {
        match self {
            VoidReason::Mistake => "mistake".into(),
            VoidReason::GuestChanged => "guest_changed".into(),
            VoidReason::Unavailable => "unavailable".into(),
            VoidReason::Dropped => "dropped".into(),
            VoidReason::Other(t) => format!("other:{t}"),
        }
    }
}

/// Where a round is in the kitchen's life, as far as changing it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// PENDING or CONFIRMED: nothing has been cooked, the lines are still an
    /// intention and the shelf still holds them as reservations.
    BeforeKitchen,
    /// PREPARING or READY: the ingredients were `Consumed`. A line may only
    /// leave with a reason and the `void` capability, and the shelf is left
    /// alone — the food is gone whether it was eaten or dropped.
    InKitchen,
    /// Anything else: collected, rejected, cancelled, delivered. Not a round
    /// that can be changed; ending an accepted one is a refund.
    Over,
}

pub fn stage(status: &str) -> Stage {
    match status {
        "PENDING" | "CONFIRMED" => Stage::BeforeKitchen,
        "PREPARING" | "READY" => Stage::InKitchen,
        _ => Stage::Over,
    }
}

/// The guard every command that changes a round's money runs first.
///
/// THE CONFIRMED FRAME (BLUEPRINT §4.5 (2)). A round whose bill is settled is
/// never amended, whatever the signer holds: after payment, taking money off a
/// round is a refund, and a refund is not this command.
pub fn changeable(order: &Value, location_id: &str, by: &str) -> Result<Stage, Refused> {
    if by.trim().is_empty() {
        return Err(Refused::Invalid("a change to a round names who made it".into()));
    }
    if order.get("location_id").and_then(Value::as_str) != Some(location_id) {
        return Err(Refused::NotFound);
    }
    if order.get("payment_status").and_then(Value::as_str) == Some("paid") {
        return Err(Refused::Conflict("this round is paid; taking money off it now is a refund".into()));
    }
    let status = order.get("status").and_then(Value::as_str).unwrap_or("");
    match stage(status) {
        Stage::Over => Err(Refused::Conflict(format!("this round is {status} and cannot be changed"))),
        s => Ok(s),
    }
}

/// One line's money, checked. `unit_price` is what the catalogue said when the
/// line was priced (`services::ordering::pricing`), carried on the line.
pub fn line_amount(line: &Value) -> Result<i64, Refused> {
    let unit = line.get("unit_price").and_then(Value::as_i64).unwrap_or(0);
    let qty = line.get("quantity").and_then(Value::as_i64).unwrap_or(0);
    unit.checked_mul(qty).ok_or_else(|| Refused::Invalid("a line's amount overflows".into()))
}

/// Σ lines. THE KERNEL'S OWN RULE — `apply_event_logic` recomputes `subtotal`
/// from `items` on the next transition — so this is the same sum, taken now,
/// not a second pricer: no price is looked up here, only multiplied.
pub fn subtotal_of(items: &[Value]) -> Result<i64, Refused> {
    items.iter().try_fold(0i64, |acc, l| {
        acc.checked_add(line_amount(l)?).ok_or_else(|| Refused::Invalid("the subtotal overflows".into()))
    })
}

/// The round's money after its lines or its discount moved:
/// `total = subtotal − discount + delivery_fee + tip`, the placement's own
/// equation (`command::place`), with every term an integer.
///
/// Refused when the discount would exceed the lines, and when more has already
/// been paid against the round than it would now cost — that difference would
/// be money the venue owes back, which is a refund and not an amendment.
pub fn reprice(order: &mut Value, items: Vec<Value>, discount: i64) -> Result<(), Refused> {
    if items.is_empty() {
        return Err(Refused::Conflict("a round with no lines left is a cancellation, not an amendment".into()));
    }
    let subtotal = subtotal_of(&items)?;
    if discount > subtotal {
        return Err(Refused::Conflict("the discount would be larger than the lines".into()));
    }
    let fee = order.get("delivery_fee").and_then(Value::as_i64).unwrap_or(0);
    let tip = order.get("tip").and_then(Value::as_i64).unwrap_or(0);
    let total = subtotal - discount + fee + tip;
    let paid = order.get("paid").and_then(Value::as_i64).unwrap_or(0);
    if paid > total {
        return Err(Refused::Conflict(format!("{paid} has been paid and the round would cost {total}")));
    }
    order["items"] = Value::Array(items);
    order["subtotal"] = json!(subtotal);
    order["discount"] = json!(discount);
    order["total"] = json!(total);
    Ok(())
}

/// Append one entry to a list field on the order (`amended`, `adjustments`).
/// A list is replaced whole by the delta fold, so the object writes the whole
/// list back with the new entry at the end.
pub fn push(order: &mut Value, field: &str, entry: Value) {
    let mut list = order.get(field).and_then(Value::as_array).cloned().unwrap_or_default();
    list.push(entry);
    order[field] = Value::Array(list);
}

/// THE SHELF FOLLOWS THE LINES, for a round the kitchen has not started.
///
/// Everything this order holds is released and the new line set is reserved,
/// in one batch, against the ledger in memory. Reservations are keyed by order
/// id, so I3 holds on the id: every `Reserved` is still matched by exactly one
/// later `Consumed` or `Released`. `boms` is the catalogue record of each
/// product the Worker saw; a line whose product it did not send reserves
/// nothing, exactly as placement treats a dish with no recipe.
pub fn restock(
    stock: &mut dowiz_hub::stock::StockLog,
    order_id: &str,
    items: &[Value],
    boms: &[(String, String)],
) -> Result<(), Refused> {
    let led = stock.ledger().map_err(|e| Refused::Stock(e.to_string()))?;
    let mut evs = dowiz_hub::stock::settle(&led, order_id, false);
    let lines: Vec<(String, i64)> = items
        .iter()
        .filter_map(|l| {
            let pid = l.get("product_id").and_then(Value::as_str)?;
            let rec = boms.iter().find(|(p, _)| p == pid)?.1.clone();
            Some((rec, l.get("quantity").and_then(Value::as_i64).unwrap_or(0)))
        })
        .collect();
    evs.extend(dowiz_hub::stock::reservations_for(order_id, &lines));
    if evs.is_empty() {
        return Ok(());
    }
    stock.append_all(&evs).map_err(|e| Refused::Stock(e.to_string()))
}
