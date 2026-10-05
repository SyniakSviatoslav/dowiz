//! WHAT THE KITCHEN REACHES, as data (docs/design/KITCHEN-ACCESS-2026-09-27.md).
//!
//! Operator, 2026-09-26: the kitchen role reaches MOST of the hub, except what
//! it does not need -- money, the venue's revenue, customers' personal data,
//! staff admin, legal, data safety, keys, tax and fiscal. Two things live here:
//!
//!   * `ROUTES` -- every `/api/owner/*` and `/api/staff/*` route `lib.rs`
//!     registers, the door it asks, and what that means for a kitchen token.
//!     Its test fails when `lib.rs` grows a route without a row, so a new
//!     screen's route cannot arrive undecided.
//!   * the NARROWING of a screen the kitchen shares with the owner. It is done
//!     by the hub, before the answer leaves: a console that merely hides a
//!     column still received it.
//!
//! No capability is added for any of this: the kitchen's three words
//! (`advance`, `catalog`, `stock`) open every screen it gains.

use serde_json::{json, Value};

#[allow(unused_imports)]
use super::guard::{BIN, MENU, NUMBERS, PASS, SHELF};
#[allow(unused_imports)]
use crate::services::engagement::assist::kitchen::ASKERS;
use crate::auth::Cap;

// THE TABLE IS READ BY ITS TESTS AND BY PEOPLE: each handler asks its own
// door (`guard.rs`), so outside `cfg(test)` nothing calls into it.
/// Which door a route asks.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Door {
    /// The owner's guard only (`owner_and_venue`, `owner_beside`, `owner_at`).
    Owner,
    /// The owner, or a member of staff holding ANY of these words.
    Staff(&'static [Cap]),
    /// The person's own door: a login, a claim, their own agent keys.
    Own,
}

/// What a kitchen token gets.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kitchen {
    Yes,
    /// Reads; the answer may be narrowed (`numbers_for_kitchen`, ...).
    Read,
    No,
}

#[allow(dead_code)]
const TAKE: [Cap; 1] = [Cap::TakeOrders];
#[allow(dead_code)]
const PAY: [Cap; 1] = [Cap::TakePayment];
#[allow(dead_code)]
const VOID: [Cap; 1] = [Cap::Void];
#[allow(dead_code)]
const TILL: [Cap; 1] = [Cap::OpenTill];

#[allow(unused_imports)]
use Door::{Own, Owner, Staff};
#[allow(unused_imports)] use Kitchen::{No, Read, Yes};

/// `(method, path, door, kitchen)` for every owner and staff route.
#[allow(dead_code)]
pub(crate) const ROUTES: &[(&str, &str, Door, Kitchen)] = &[
    // ── the pass ──
    ("post", "/api/owner/orders/:id/action", Staff(&PASS), Yes),
    ("post", "/api/staff/orders/:id/kitchen-ack", Staff(&PASS), Yes),
    ("get", "/api/staff/kitchen", Staff(&PASS), Read),
    ("get", "/api/owner/print/jobs", Staff(&PASS), Read),
    ("get", "/api/owner/floorplan", Staff(&PASS), Read),
    ("get", "/api/owner/reservations", Staff(&PASS), Read),
    ("get", "/api/owner/settings", Staff(&PASS), Read),
    ("post", "/api/owner/settings", Staff(&PASS), Yes),
    ("post", "/api/staff/assist", Staff(&ASKERS), Yes),
    // ── the menu ──
    ("post", "/api/owner/products", Staff(&MENU), Yes),
    ("post", "/api/owner/products/:id/delete", Staff(&MENU), Yes),
    ("post", "/api/owner/products/:id", Staff(&MENU), Yes),
    ("post", "/api/owner/products/:id/image", Staff(&MENU), Yes),
    ("post", "/api/owner/products/:id/image/clear", Staff(&MENU), Yes), ("get", "/api/owner/products/:id/option-bom", Staff(&MENU), Read), ("post", "/api/owner/products/:id/option-bom", Staff(&MENU), Yes), // W-LOST
    ("get", "/api/owner/products", Staff(&MENU), Read),
    ("get", "/api/owner/categories", Staff(&MENU), Read),
    ("post", "/api/owner/categories", Staff(&MENU), Yes),
    ("post", "/api/owner/categories/:id/delete", Staff(&MENU), Yes),
    ("post", "/api/owner/i18n", Staff(&MENU), Yes),
    ("post", "/api/owner/menu/import", Staff(&MENU), Yes),
    // W-NOM: menu OR shelf names the nomenclature; deleting for good is the owner's.
    ("post", "/api/owner/supplies", Staff(&crate::services::operations::supplies::quick::ADD), Yes),
    ("post", "/api/owner/supplies/bulk", Staff(&crate::services::operations::supplies::quick::ADD), Yes),
    ("post", "/api/owner/supplies/delete", Owner, No),
    ("post", "/api/owner/preps", Staff(&crate::services::operations::supplies::quick::ADD), Yes),
    ("get", "/api/owner/preps", Staff(&NUMBERS), Read),
    ("get", "/api/owner/supplies/:id/uses", Staff(&NUMBERS), Read),
    ("get", "/api/owner/products/:id/takes", Staff(&NUMBERS), Read),
    ("post", "/api/owner/products/delete", Owner, No),
    ("post", "/api/owner/supplies/:id/retire", Staff(&MENU), Yes),
    ("post", "/api/owner/supplies/import", Staff(&MENU), Yes),
    ("post", "/api/owner/ingredients/reset", Owner, No),
    ("post", "/api/owner/recipes/import", Staff(&MENU), Yes),
    // ── the shelf, and the kitchen's numbers ──
    ("get", "/api/owner/stock", Staff(&SHELF), Read),
    ("post", "/api/owner/stock/:kind", Staff(&BIN), Yes),
    ("get", "/api/owner/stock/waste", Staff(&SHELF), Read), ("get", "/api/owner/stock/haccp", Owner, No), // W-STORE P13
    ("get", "/api/owner/analytics/kitchen", Staff(&NUMBERS), Read), ("get", "/api/staff/kitchen/prep", Staff(&NUMBERS), Read), // W-PREP
    ("post", "/api/owner/analytics/history", Owner, No),
    // ── the person's own door ──
    ("post", "/api/staff/login", Own, Yes),
    ("post", "/api/staff/claim", Own, Yes),
    ("post", "/api/staff/password", Own, Yes),
    ("get", "/api/staff/mcp/keys", Own, Yes),
    ("post", "/api/staff/mcp/keys", Own, Yes),
    ("post", "/api/staff/mcp/keys/revoke", Own, Yes),
    // ── the room: words the kitchen does not hold ──
    ("get", "/api/staff/room", Staff(&TAKE), No),
    ("post", "/api/staff/orders/:id/amend", Staff(&TAKE), No),
    ("post", "/api/staff/orders/:id/pay", Staff(&PAY), No),
    ("post", "/api/staff/orders/aggregator", Staff(&TAKE), No),
    ("post", "/api/staff/orders/:id/refund", Staff(&VOID), No),
    ("post", "/api/staff/orders/:id/returned", Staff(&VOID), No),
    ("post", "/api/staff/orders/:id/transfer", Staff(&TAKE), No),
    ("post", "/api/staff/sittings/:id/move", Staff(&TAKE), No),
    ("post", "/api/staff/till/open", Staff(&TILL), No), ("post", "/api/staff/offline_sales", Staff(&PAY), No), // W-OFFSALE
    ("post", "/api/staff/till/count", Staff(&TILL), No),
    ("post", "/api/staff/till/close", Staff(&TILL), No),
    ("post", "/api/staff/till/pay_in", Staff(&TILL), No),
    ("post", "/api/staff/till/pay_out", Staff(&TILL), No),
    ("get", "/api/staff/till/tips", Staff(&TILL), No),
    ("get", "/api/staff/floor", Staff(&TAKE), No),
    ("post", "/api/staff/floor/:sitting/cleared", Staff(&TAKE), No),
    ("post", "/api/staff/orders/:id/guest", Staff(&TAKE), No),
    // ── the owner's alone: money, customers, people, keys, legal, safety ──
    ("post", "/api/owner/zones", Owner, No),
    ("post", "/api/owner/floorplan", Owner, No),
    ("post", "/api/owner/reservations/:id/action", Owner, No),
    ("post", "/api/owner/branding/extract", Owner, No),
    ("get", "/api/owner/staff", Owner, No),
    ("post", "/api/owner/staff/invite", Owner, No),
    ("post", "/api/owner/staff/:id", Owner, No),
    ("post", "/api/owner/staff/:id/password", Owner, No),
    ("get", "/api/owner/wallet/legs", Owner, No),
    ("post", "/api/owner/wallet/legs/repair", Owner, No),
    ("get", "/api/owner/tables/qr", Owner, No),
    ("get", "/api/owner/tables/:zone/:n/qr.svg", Owner, No),
    ("get", "/api/owner/orders", Owner, No),
    ("post", "/api/owner/orders/:id/assign", Owner, No),
    ("get", "/api/owner/couriers/:id", Owner, No),
    ("get", "/api/owner/dashboard", Owner, No),
    ("post", "/api/owner/location", Owner, No),
    ("get", "/api/owner/analytics", Owner, No),
    ("get", "/api/owner/exceptions", Owner, No),
    ("get", "/api/owner/promotions", Owner, No),
    ("post", "/api/owner/promotions", Owner, No),
    ("post", "/api/owner/promotions/:code/delete", Owner, No),
    ("get", "/api/owner/activation", Owner, No),
    ("get", "/api/owner/branding", Owner, No),
    ("post", "/api/owner/branding", Owner, No),
    ("post", "/api/owner/branding/preset", Owner, No),
    ("get", "/api/owner/campaigns", Owner, No),
    ("post", "/api/owner/campaigns", Owner, No),
    ("get", "/api/owner/campaigns/:id", Owner, No),
    ("post", "/api/owner/campaigns/:id/preview", Owner, No),
    ("post", "/api/owner/campaigns/:id/send", Owner, No),
    ("get", "/api/owner/customers", Owner, No),
    ("post", "/api/owner/customers/:key/reveal", Owner, No),
    ("post", "/api/owner/customers/:key/forget", Owner, No), ("get", "/api/owner/customers/:key/taste", Owner, No),
    ("get", "/api/owner/customers/taste/segments", Owner, No), ("get", "/api/owner/customers/reveals", Owner, No),
    ("put", "/api/owner/customers/:key/record", Owner, No),
    ("post", "/api/owner/customers/rekey", Owner, No),
    ("post", "/api/owner/customers/reforget", Owner, No),
    ("post", "/api/owner/customers/:key/consent", Owner, No),
    ("post", "/api/owner/customers/:key/link", Owner, No),
    ("post", "/api/owner/customers/:key/unlink", Owner, No),
    ("get", "/api/owner/features", Owner, No),
    ("post", "/api/owner/features", Owner, No),
    ("get", "/api/owner/publish", Owner, No), ("post", "/api/owner/publish", Owner, No), // BN2's published menu
    ("post", "/api/owner/notify/test", Owner, No),
    ("get", "/api/owner/telegram", Owner, No),
    ("post", "/api/owner/telegram/connect", Owner, No),
    ("post", "/api/owner/telegram/link", Owner, No),
    ("post", "/api/owner/telegram/group", Owner, No),
    ("post", "/api/owner/telegram/test", Owner, No),
    ("post", "/api/owner/telegram/unlink", Owner, No),
    ("get", "/api/owner/sms", Owner, No), ("post", "/api/owner/sms", Owner, No), ("post", "/api/owner/sms/test", Owner, No), ("post", "/api/owner/sms/stop", Owner, No), ("get", "/api/owner/bag", Owner, No), ("post", "/api/owner/bag", Owner, No), ("get", "/api/owner/bag/qr.svg", Owner, No), // W-QR
    ("get", "/api/owner/inbox", Owner, No),
    ("get", "/api/owner/threads", Owner, No),
    ("get", "/api/owner/inbox/:peer", Owner, No),
    ("post", "/api/owner/inbox/:peer", Owner, No),
    ("get", "/api/owner/backup/cloud", Owner, No),
    ("post", "/api/owner/backup/cloud", Owner, No),
    ("get", "/api/owner/integrations", Owner, No),
    ("get", "/api/owner/dpa", Owner, No),
    ("post", "/api/owner/dpa/accept", Owner, No),
    ("post", "/api/owner/integrations/check", Owner, No),
    ("get", "/api/owner/ebills", Owner, No),
    ("post", "/api/owner/ebills/config", Owner, No),
    ("post", "/api/owner/ebills/map", Owner, No),
    ("get", "/api/owner/fiscal", Owner, No), ("get", "/api/owner/offline_sales", Owner, No), // W-OFFSALE
    ("post", "/api/owner/fiscal/ebills", Owner, No),
    ("get", "/api/owner/orders/:id/receipt", Owner, No),
    ("get", "/api/owner/mcp/keys", Owner, No),
    ("post", "/api/owner/mcp/keys/revoke", Owner, No),
    ("get", "/api/owner/couriers", Owner, No),
    ("post", "/api/owner/couriers/invite", Owner, No),
    ("post", "/api/owner/couriers/:id/uninvite", Owner, No),
    ("post", "/api/owner/couriers/:id/active", Owner, No),
    ("get", "/api/owner/posts", Owner, No),
    ("post", "/api/owner/posts/draft", Owner, No),
    ("post", "/api/owner/posts/:id/approve", Owner, No),
    ("post", "/api/owner/posts/:id/reject", Owner, No),
    ("get", "/api/owner/graph", Owner, No),
    ("get", "/api/owner/health", Owner, No),
    ("get", "/api/owner/history", Owner, No),
    ("post", "/api/owner/hub/rotate", Owner, No),
    ("get", "/api/owner/backup", Owner, No),
    ("post", "/api/owner/restore", Owner, No),
    ("post", "/api/owner/assist", Owner, No), ("get", "/api/owner/ai", Owner, No), ("post", "/api/owner/ai/test", Owner, No), ("post", "/api/owner/ai/ask", Owner, No), ("get", "/api/owner/ai/explain", Owner, No),
    ("get", "/api/owner/apikeys", Owner, No),
    ("post", "/api/owner/apikeys", Owner, No),
    ("post", "/api/owner/apikeys/revoke", Owner, No),
    ("post", "/api/owner/place", Owner, No),
    ("post", "/api/owner/logo", Owner, No),
    ("post", "/api/owner/logo/clear", Owner, No),
];

/// Rows whose `lib.rs` line is handed back to the main session (a lane does not edit `lib.rs`). Until it
/// lands, the row may name a route `lib.rs` does not have yet; once it lands the entry here is dead and may go.
#[allow(dead_code)]
pub(crate) const HANDED_BACK: &[(&str, &str)] = &[];

// ── narrowing a shared screen ────────────────────────────────────────────────

/// The kitchen numbers' money FROM SALES: what the venue took and kept, and
/// food cost, which is a share of it. Removed at any depth, so a revenue field
/// a later report grows under one of these names cannot reach a cook.
pub(crate) const REVENUE_KEYS: [&str; 4] = ["revenue", "margin", "marginPortion", "foodCostPm"];

/// `GET /api/owner/analytics/kitchen` for staff: cost and usage, no revenue.
pub(crate) fn numbers_for_kitchen(mut out: Value) -> Value {
    strip(&mut out);
    out["scope"] = json!("kitchen");
    out
}

fn strip(v: &mut Value) {
    match v {
        Value::Object(m) => {
            for k in REVENUE_KEYS {
                m.remove(k);
            }
            m.values_mut().for_each(strip);
        }
        Value::Array(a) => a.iter_mut().for_each(strip),
        _ => {}
    }
}

/// What a booking row keeps for the kitchen: when, how many, where, why.
/// A WHITELIST: a contact field a later row grows is dropped, not leaked.
pub(crate) const BOOKING_KEEPS: [&str; 7] = ["id", "slotMin", "party", "occasion", "zoneId", "tableN", "status"];

/// `GET /api/owner/reservations` for staff: no name, no phone, no moves.
pub(crate) fn bookings_for_kitchen(rows: Vec<Value>) -> Vec<Value> {
    rows.into_iter()
        .map(|r| {
            let mut out = serde_json::Map::new();
            for k in BOOKING_KEEPS {
                if let Some(v) = r.get(k) {
                    out.insert(k.to_string(), v.clone());
                }
            }
            out.insert("next".into(), json!([]));
            Value::Object(out)
        })
        .collect()
}

/// The venue settings a member of staff at the pass may read and write: the
/// kitchen printer's name. Nothing else -- every secret lives in this space.
pub(crate) const KITCHEN_SETTINGS: [&str; 1] = ["print.kitchen"];

pub(crate) fn kitchen_may_set(key: &str) -> bool { KITCHEN_SETTINGS.contains(&key) }

/// `GET /api/owner/settings` for staff: only `KITCHEN_SETTINGS`, values and
/// declarations alike.
pub(crate) fn settings_for_kitchen(values: &Value, known: Vec<Value>) -> Value {
    let values: serde_json::Map<String, Value> = KITCHEN_SETTINGS
        .iter()
        .filter_map(|k| values.get(*k).map(|v| (k.to_string(), v.clone())))
        .collect();
    let known: Vec<Value> = known
        .into_iter()
        .filter(|k| k.get("key").and_then(Value::as_str).is_some_and(kitchen_may_set))
        .collect();
    json!({ "values": values, "known": known, "scope": "kitchen" })
}

#[cfg(test)]
#[path = "access/tests.rs"]
mod tests;
