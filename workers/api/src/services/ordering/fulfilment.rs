//! PURE. How an order reaches the person who ordered it.
//!
//! THERE WERE THREE WAYS AND THE CODE KNEW ABOUT TWO, in four places, each
//! written as `== Some("pickup")`:
//!
//!   * `courier.rs` — a pickup is not offered to a courier;
//!   * `live_eta.rs` — a pickup has no travel time;
//!   * `services/analytics/fold.rs` — the pickup tally;
//!   * `notify.rs` — 🥡 or 🛵 on the kitchen's ticket.
//!
//! Four copies of the question "is this going out of the building?", and every
//! one of them gets a THIRD kind wrong in a different way: a table order would
//! have been offered to a courier, given a delivery ETA, counted as a delivery
//! in the takings, and announced with a scooter. That is the shape `a18025d4`
//! (fourteen copies of a status set) and `a18886d4` (two pricers) already cost
//! this codebase, so the predicate lives here once and the four sites ask it.
//!
//! `dine_in` IS NOT `pickup` WITH A TABLE. They agree on everything the four
//! sites above ask and differ on what they REQUIRE: a pickup needs nothing but
//! a person to come, and a table order cannot exist without a table. Folding
//! them together would let an order be placed for "the restaurant" with no
//! table on it, which is an order nobody can carry anywhere.
//!
//! THE STOCK IS THE SAME STOCK, and that needs no code at all — it is why it is
//! worth saying. `place_command::decide` reserves ingredients from the venue's
//! one ledger before it looks at anything else, and it never reads the
//! fulfilment. A table order and a delivery order compete for the last portion
//! of salmon on equal terms, by construction rather than by arrangement. The
//! test at the bottom is there so a later change cannot quietly separate them.

/// Every way an order can reach its customer. A CLOSED SET, checked on the way
/// in.
///
/// IT WAS OPEN, and that was a live defect: `kind` came off the wire as a free
/// string, and only `"delivery"` and `"pickup"` were ever compared against. A
/// basket sent with `kind: "banana"` was accepted, charged the full delivery
/// fee, and given no address to deliver to — because the address check fires
/// only on the exact word `"delivery"`.
pub const KINDS: [&str; 3] = ["delivery", "pickup", "dine_in"];

pub fn known(kind: &str) -> bool {
    KINDS.contains(&kind)
}

/// What an order of this kind cannot be placed without.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Needs {
    /// Somewhere to take it.
    Address,
    /// A table in the venue.
    Table,
    /// Nothing: the customer is coming to the counter.
    Nothing,
}

pub fn needs(kind: &str) -> Needs {
    match kind {
        "delivery" => Needs::Address,
        "dine_in" => Needs::Table,
        _ => Needs::Nothing,
    }
}

/// Does this order leave the building?
///
/// THE ONE QUESTION THE FOUR SITES WERE EACH ASKING SEPARATELY. A courier, a
/// travel ETA and the delivery tally are all downstream of it, and only
/// `delivery` answers yes. Written as an allow-list rather than
/// `!= "pickup"`, so the NEXT kind is refused by default instead of silently
/// inheriting a scooter.
pub fn leaves_the_building(kind: &str) -> bool {
    kind == "delivery"
}

/// What this order pays for its journey.
///
/// A TABLE PAYS NO DELIVERY FEE, and neither does a pickup: the fee buys a
/// courier's trip, and there is no trip. The free-delivery threshold is checked
/// FIRST and for every kind, because a venue that has set one has said "over
/// this much, no fee", and a kind that never had one is already at zero.
pub fn fee(kind: &str, subtotal: i64, free_over: Option<i64>, delivery_fee: i64) -> i64 {
    match free_over {
        Some(th) if subtotal >= th => 0,
        _ if !leaves_the_building(kind) => 0,
        _ => delivery_fee,
    }
}

/// The kind an order envelope carries.
///
/// ABSENT MEANS `delivery`, and that is a compatibility rule with a date on it:
/// every order placed before this module existed carries a kind, but orders
/// restored from an archive or an old backup may not, and the folds that read
/// this must not turn a missing field into an unknown kind that no branch
/// handles. It is the ONE place the pointer is spelled out; four files used to
/// spell it themselves.
pub fn of(envelope: &serde_json::Value) -> &str {
    envelope
        .pointer("/fulfilment/kind")
        .and_then(serde_json::Value::as_str)
        .filter(|k| known(k))
        .unwrap_or("delivery")
}

/// The table an order names, if it is a table order.
pub fn table_of(envelope: &serde_json::Value) -> Option<&str> {
    envelope
        .pointer("/fulfilment/table")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests;
