//! PURE. WHEN A VENUE MAY SEND (card L70 HARD LIMITS), and the refund's
//! cancel entry.
//!
//! A VENUE SENDS ONLY WHEN ALL HOLD, and each one that does not is NAMED:
//!   * `fiscal.since_ms` is set (`wire::config`);
//!   * `fiscal.ebills.armed` is exactly "yes" -- written ONLY by the owner's
//!     fiscal route, and only with the confirmation phrase typed (`arm`);
//!   * `fiscal.ebills.sale_unit` names a sale unit (EBILLS-WRITE-PATH §1.6:
//!     at this venue the SPA always attaches one; a table-less sale is
//!     first-send unknown #1);
//!   * the venue's ebills link is healthy: configured, not halted, and its
//!     last firing worked (`LinkState`).
//!
//! THE FOUR KEYS ARE DELIBERATELY NOT IN `settings/known.rs`. A declared key
//! is editable from the generic settings pane, which would arm a legal act
//! without the confirmation that names it; undeclared, the generic route
//! refuses them ("unknown setting") and the fiscal route is the only writer.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::wire::Config;
use crate::hubdo::OrderView;

pub const ARMED: &str = "fiscal.ebills.armed";
pub const SALE_UNIT: &str = "fiscal.ebills.sale_unit";
pub const FEE_ITEM: &str = "fiscal.ebills.fee_item";
pub const CANCEL_ARMED: &str = "fiscal.ebills.cancel_armed";
/// What the owner types to arm a venue: the consequence, in words.
pub const CONFIRM: &str = "SEND FISCAL INVOICES TO EBILLS";

/// The link as the sender needs it, read from the venue's `ebills` image.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LinkState {
    pub usable: bool,
    pub halted: bool,
    pub failures: u32,
    pub last_ok_ms: i64,
}

/// The sender's settings as read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arming {
    pub armed: bool,
    pub sale_unit: String,
    pub fee_item: String,
    pub cancel_armed: bool,
}

/// From the settings image's raw values (`Settings::get`).
pub fn arming(get: &dyn Fn(&str) -> Option<String>) -> Arming {
    let yes = |k: &str| get(k).is_some_and(|v| v.trim() == "yes");
    let text = |k: &str| get(k).map(|v| v.trim().to_string()).unwrap_or_default();
    Arming { armed: yes(ARMED), sale_unit: text(SALE_UNIT), fee_item: text(FEE_ITEM), cancel_armed: yes(CANCEL_ARMED) }
}

/// Every reason this venue may not send now. EMPTY MEANS ARMED.
pub fn not_ready(since: &Config, a: &Arming, link: &LinkState) -> Vec<String> {
    let mut why = Vec::new();
    if !matches!(since, Config::From(_)) {
        why.push("fiscal.since_ms is not set: nothing is queued".to_string());
    }
    if !a.armed {
        why.push("sending is not armed: the owner arms it in the fiscal pane".into());
    }
    if a.sale_unit.is_empty() {
        why.push("no eBills sale unit is chosen".into());
    }
    if !link.usable {
        why.push("the ebills link is not configured".into());
    } else if link.halted {
        why.push("the ebills link is halted".into());
    } else if link.failures > 0 || link.last_ok_ms == 0 {
        why.push("the ebills link's last read did not work".into());
    }
    why
}

/// THE GATE the object runs every firing: the plan when ALL hold, else the
/// reasons -- and then nothing is claimed, so nothing can be sent.
pub fn gated_plan(
    since: &Config,
    a: &Arming,
    link: &LinkState,
    entries: &[crate::outbox::Entry],
    intents: &[super::ebills_sender::Intent],
    now_ms: i64,
) -> Result<(Vec<super::ebills_sender::Item>, Vec<super::ebills_sender::Intent>), Vec<String>> {
    let why = not_ready(since, a, link);
    if !why.is_empty() {
        return Err(why);
    }
    Ok(super::ebills_sender::plan(entries, intents, now_ms))
}

/// The owner's change to the four keys (`POST /api/owner/fiscal/ebills`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArmIn {
    #[serde(default)]
    pub armed: Option<bool>,
    #[serde(default)]
    pub confirm: Option<String>,
    #[serde(default)]
    pub sale_unit: Option<String>,
    #[serde(default)]
    pub fee_item: Option<String>,
    #[serde(default)]
    pub cancel_armed: Option<bool>,
}

fn plain(v: &str, max: usize) -> bool {
    v.len() <= max && v.chars().all(|c| c.is_ascii_alphanumeric() || " -_.".contains(c))
}

/// The settings writes the owner asked for, or why not. ARMING -- sending,
/// or the (not yet built) cancel -- needs the phrase typed exactly;
/// DISARMING needs nothing, so switching it off is never harder than on.
pub fn arm(input: &ArmIn) -> Result<Vec<(&'static str, String)>, String> {
    let confirmed = input.confirm.as_deref().map(str::trim) == Some(CONFIRM);
    let mut out = Vec::new();
    for (key, on) in [(ARMED, input.armed), (CANCEL_ARMED, input.cancel_armed)] {
        match on {
            Some(true) if !confirmed => return Err(format!("to arm, type exactly: {CONFIRM}")),
            Some(true) => out.push((key, "yes".to_string())),
            Some(false) => out.push((key, String::new())),
            None => {}
        }
    }
    for (key, v, max) in [(SALE_UNIT, &input.sale_unit, 32), (FEE_ITEM, &input.fee_item, 64)] {
        if let Some(v) = v.as_deref().map(str::trim) {
            if !plain(v, max) {
                return Err(format!("{key}: letters, digits, space, - _ . and at most {max}"));
            }
            out.push((key, v.to_string()));
        }
    }
    Ok(out)
}

/// THE CANCEL ENTRY's record kind in the `fiscal` image (card item 4).
pub const CANCEL: &str = "fiscal.cancel";

/// A whole-sale cancel owed to the tax authority: the order was refunded
/// after its document was registered. The body is §4.1's, built at send
/// time from the sale as read -- and SENDING IT IS NOT BUILT: the allow-list
/// has no `PUT /api/sales-cancel` (card: exactly one write constructor), the
/// shape is unverified, and `fiscal.ebills.cancel_armed` only records intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CancelEntry {
    pub order_id: String,
    pub sale_id: i64,
    pub fic: String,
    pub reason: String,
    pub queued_at_ms: i64,
    /// §4.1's wrapper minus the sale: `{reason, withoutfiscalization:false, saleType:"NORMAL"}`.
    pub shape: Value,
}

/// The refunds that owe a cancel and have none queued yet.
pub fn cancels_owed(orders: &[OrderView], queued: &dyn Fn(&str) -> bool, now_ms: i64) -> Vec<CancelEntry> {
    let mut out = Vec::new();
    for v in orders {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        let status = o.get("status").and_then(Value::as_str).unwrap_or("");
        let fic = o.pointer("/fiscal/fic").and_then(Value::as_str).unwrap_or("");
        let sale_id = o.pointer("/fiscal/sale_id").and_then(Value::as_i64).unwrap_or(0);
        if !matches!(status, "REFUNDING" | "COMPENSATED_REFUND") || fic.is_empty() || sale_id < 1 || queued(&v.order_id) {
            continue;
        }
        let reason = o.pointer("/refund/reason").and_then(Value::as_str).unwrap_or("refund").to_string();
        out.push(CancelEntry {
            order_id: v.order_id.clone(),
            sale_id,
            fic: fic.to_string(),
            shape: serde_json::json!({ "reason": reason, "withoutfiscalization": false, "saleType": "NORMAL" }),
            reason,
            queued_at_ms: now_ms,
        });
    }
    out
}

#[cfg(test)]
mod tests;
