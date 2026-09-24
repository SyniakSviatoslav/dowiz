//! One order as a projection carries it, and how a decider finds the order
//! it is about in an image. MOVED FROM `workers/api/src/hubdo.rs` (D7 phase 1).

use crate::{Event, Hub};

/// One order as a projection carries it: the fold, and the two facts about the
/// event that produced it.
///
/// This is what crosses the Worker↔object hop instead of the image. A console
/// poll used to ship the whole log -- every order the venue has ever taken --
/// so that the Worker could throw away all but the last day of it.
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct OrderView {
    pub order_id: String,
    /// `dowiz_hub::EventKind` as its byte, because the enum is not serialisable
    /// and the number is what the log itself stores.
    pub kind: u8,
    pub seq: u64,
    /// The FOLDED order, as JSON text. Text rather than a `Value` because every
    /// consumer parses it themselves and re-serialising it here would be a
    /// second encoding of the same bytes.
    pub order_json: String,
}

impl OrderView {
    pub fn of_event(e: Event) -> Self {
        Self::of(e)
    }

    pub fn of(e: Event) -> Self {
        OrderView { order_id: e.order_id, kind: e.kind as u8, seq: e.seq, order_json: e.order_json }
    }
}

/// THE ORDER A COMMAND IS ABOUT, folded from the image: what the object's
/// `orders_view` (the Worker's `hubstore::orders_state`) yields for this id,
/// computed for ONE id.
///
/// The same three rules as that projection, because a decider fed a different
/// `current` would decide differently: events OLDEST FIRST, non-order events
/// skipped (`EventKind::is_order`), and the view carries the NEWEST event's
/// kind and seq with the folded state as its JSON. An order whose fold is null
/// (every payload unreadable) is not there, exactly as the projection drops it.
/// The Worker's `command/amend/agrees.rs` holds the two to one delta.
pub fn current(hub: &Hub, order_id: &str) -> Option<OrderView> {
    let mut state = serde_json::Value::Null;
    let mut newest: Option<Event> = None;
    for e in hub.events_oldest_first() {
        if !e.kind.is_order() || e.order_id != order_id {
            continue;
        }
        state = super::delta::fold_one(state, &e.order_json);
        newest = Some(e);
    }
    let mut e = newest?;
    if state.is_null() {
        return None;
    }
    e.order_json = state.to_string();
    Some(OrderView::of(e))
}

#[cfg(test)]
mod tests;
