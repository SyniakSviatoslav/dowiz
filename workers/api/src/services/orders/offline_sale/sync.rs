//! PURE. The object's one turn for a synced offline sale: is it here already,
//! the tax stamped as the guest paid it, the shelf drawn by the recipe, one
//! `Placed` with the sale's own instant. The caller writes only on `Ok`, and
//! only when something was appended.
//!
//! THE TAX IS STAMPED INCLUSIVE. The tablet's total is what the guest handed
//! over for the shelf prices (an Albanian consumer price includes VAT), so the
//! stamp may not add tax to it afterwards: `tax_block::stamp` rewrites an
//! exclusive venue's total, which would record money nobody took. A venue set
//! to exclusive prices is stamped inclusive here and the conflict says so.
//! A venue with no tax configured gets no block (the fiscal document then
//! refuses `NoTax`, named in the owner's pane); a malformed tax setting is a
//! conflict, never a refused sale.

use super::{Conflict, SyncIn};
use crate::command::Refused;
use crate::hubdo::OrderView;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_hub::stock::StockLog;
use dowiz_hub::{EventKind, Hub};
use serde_json::{json, Value};

/// What the turn came to.
#[derive(Debug, Clone, PartialEq)]
pub struct Decided {
    /// The order as stored (or as it already stood, on a replay).
    pub stored: String,
    /// Nothing appended: the sale was already in the log.
    pub replayed: bool,
    pub conflicts: Vec<Conflict>,
}

fn conflicts_of(env: &Value) -> Vec<Conflict> {
    env.pointer("/offline/conflicts").and_then(|c| serde_json::from_value(c.clone()).ok()).unwrap_or_default()
}

/// THE DRAWER THE CASH WENT INTO: the till period open at the sale's instant,
/// if one was. Its id goes on the payment, so the drawer's expected cash
/// counts the sale (`command::till::fold::cash_payments`); with no drawer open
/// the payment carries none, and the exceptions report says `cash_outside_till`
/// -- the truth, not a guessed drawer.
pub fn till_at(periods: &[crate::command::till::Period], sold_at: i64) -> Option<String> {
    periods.iter().rev().find(|p| crate::command::till::fold::within(p, sold_at)).map(|p| p.till_id.clone())
}

/// Put the drawer on the sale's one payment.
pub fn into_till(envelope: &mut Value, till: Option<String>) {
    if let (Some(t), Some(p)) = (till, envelope.pointer_mut("/payments/0")) {
        p["till_id"] = json!(t);
    }
}

/// The turn. `tax` is the venue's tax at the SALE's instant; `currency` the
/// venue's own.
pub fn decide(
    hub: &mut Hub,
    stock: &mut StockLog,
    listed: &[OrderView],
    tax: &Result<Option<VenueTax>, String>,
    currency: &str,
    input: &SyncIn,
) -> Result<Decided, Refused> {
    if !input.order_id.starts_with(super::PREFIX) || input.envelope.get("id").and_then(Value::as_str) != Some(input.order_id.as_str()) {
        return Err(Refused::Invalid("an offline sale's order id is offline:<key>, in the envelope too".into()));
    }
    // EXACTLY ONCE: the replay finds its own order and appends nothing.
    if let Some(o) = listed.iter().find(|o| o.order_id == input.order_id) {
        let held: Value = serde_json::from_str(&o.order_json).unwrap_or(Value::Null);
        return Ok(Decided { stored: o.order_json.clone(), replayed: true, conflicts: conflicts_of(&held) });
    }
    let mut env = input.envelope.clone();
    let mut extra: Vec<Conflict> = Vec::new();
    if env.get("currency").and_then(Value::as_str) != Some(currency) {
        extra.push(Conflict::said("currency", format!("the sale is in {}, the venue in {currency}", env["currency"].as_str().unwrap_or("?"))));
    }
    match tax {
        Ok(Some(v)) => {
            if !v.inclusive {
                extra.push(Conflict::said("tax", "the venue's prices are set exclusive; the sale was stamped inclusive, as the guest paid it"));
            }
            let inclusive = VenueTax { inclusive: true, ..*v };
            if let Err(why) = crate::services::ordering::tax_block::stamp(&mut env, &inclusive, 0, 0, 0) {
                extra.push(Conflict::said("tax", why));
            }
        }
        Ok(None) => {}
        Err(why) => extra.push(Conflict::said("tax", why.clone())),
    }
    // THE SHELF, AT THE SYNC (row 3): `Served` draws, which the ledger never
    // refuses for a short shelf -- the food left the kitchen hours ago.
    let draws = dowiz_hub::stock::draws_for(&input.order_id, &input.bom_lines);
    if !draws.is_empty() {
        match stock.append_served_draws(&draws) {
            // THE COST STAMP FROM THE SAME FOLD, as a placement's (R4,
            // `command::place::cost`): the food cost reads this sale like any other.
            Ok((book, at)) => crate::command::place::cost::stamp_lines(&mut env, &input.bom_lines, &book, at, &dowiz_hub::prep::stocked::Shelf::new()),
            Err(e) => extra.push(Conflict::said("stock", e.to_string())),
        }
    }
    let mut conflicts = conflicts_of(&env);
    conflicts.extend(extra);
    env["offline"]["conflicts"] = json!(conflicts);
    // THE CHANNEL IS STAMPED BY THE ONE FUNCTION THAT WRITES IT, before this
    // second `Placed` append site (`tools/gates/channel-closed.sh`, placed=2):
    // a sale signed by staff is `console`, never a word the body chose.
    use crate::services::ordering::channel;
    channel::stamp(&mut env, channel::for_placement(true)).map_err(|e| Refused::Invalid(e.to_string()))?;
    let body = env.to_string();
    hub.append(EventKind::Placed, &input.order_id, &body, input.sold_at_ms.max(0) as u64, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok(Decided { stored: body, replayed: false, conflicts })
}
