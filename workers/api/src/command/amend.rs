//! AMEND A ROUND: lines added, removed, re-counted or comped, and the table it
//! stands at — as one signed `Amended` event, in one object turn.
//!
//! THE DECIDER MOVED to `dowiz_hub::room::amend` (D7 phase 1, BLUEPRINT-
//! OPERATIONAL-BLIND-SPOTS §2.11), where `bebop-wasm --features decide` runs the
//! same code for a tablet that is offline. Its header holds the rules (the
//! intent form, `base_seq`, the record outranking the permission). This file
//! is what every caller here already names, so none of them changed.
//!
//! THE TWO ADAPTERS BELOW exist only because `hubdo.rs` still defines its own
//! `OrderView` (the hand-back is `pub use dowiz_hub::room::view::OrderView;`
//! there). They copy the view into the hub's type and call the hub; once the
//! hand-back lands the copy is of the same type and both can be deleted.

pub use dowiz_hub::room::amend::{next_seq, AmendIn, AmendOut, Op};

use super::Refused;
use crate::hubdo::OrderView;
use serde_json::Value;
#[cfg(test)]
use {super::room_rules::VoidReason, serde_json::json};

/// The hub's view of the same order: the same four fields.
pub(crate) fn hub_view(v: &OrderView) -> dowiz_hub::room::view::OrderView {
    dowiz_hub::room::view::OrderView {
        order_id: v.order_id.clone(),
        kind: v.kind,
        seq: v.seq,
        order_json: v.order_json.clone(),
    }
}

/// `dowiz_hub::room::amend::apply`: the ops over a round, in memory.
pub fn apply(current: &OrderView, input: &AmendIn) -> Result<(Value, bool), Refused> {
    dowiz_hub::room::amend::apply(&hub_view(current), input)
}

/// `dowiz_hub::room::amend::decide`: THE WHOLE AMENDMENT, over two images
/// already in memory, written by the caller only if this returns `Ok`.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    stock: &mut dowiz_hub::stock::StockLog,
    current: Option<&OrderView>,
    input: &AmendIn,
) -> Result<(Value, String, u64), Refused> {
    dowiz_hub::room::amend::decide(hub, stock, current.map(hub_view).as_ref(), input)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod agrees;
