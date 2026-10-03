//! THE OFFLINE CASH SALE (lane W-OFFSALE, row OF3; operator decision D8,
//! 2026-10-03: "offline cash sale with the 48 h fiscal rule: YES").
//!
//! WHAT IT IS. The venue's internet is down; a guest at the counter wants to
//! pay cash for a dish. The room tablet prices the basket from the catalogue it
//! cached (`public/room/offline-sale.js`, the same rule as
//! `services::ordering::pricing` for a dish without options, pinned by ONE
//! fixture both sides read), takes the cash, prints a paper receipt that says
//! "pa NIVF -- do te fiskalizohet brenda 48 oreve", and queues the sale in its
//! outbox under the key minted at the tap. When the network returns the outbox
//! replays it HERE, exactly once, into the venue's object as an ordinary order
//! with its ORIGINAL time.
//!
//! WHY NO CRDT (research §5 (a)). The sale is an INTENT the object accepts; it
//! never edits another order. Two tablets offline at once each send their own
//! sales under their own keys; the object appends both, in arrival order.
//!
//! IT IS HISTORY, SO IT IS NEVER REFUSED FOR WHAT CHANGED. The money changed
//! hands. A dish taken off sale, a price moved, a recipe short on the shelf, a
//! tablet clock that lies -- each is RECORDED beside the sale as a conflict for
//! the owner (`offline.conflicts`), and the sale keeps the price the guest
//! paid. Only a malformed body (a bug, not a fact) is refused, 400.
//!
//! EXACTLY ONCE, TWO WAYS. The order id is `offline:<sale_key>`, so a replay
//! after the idempotency window has expired still finds its own order in the
//! log and appends nothing (`sync::decide`); inside the window the
//! idempotency layer answers the first call's body.
//!
//! THE FISCAL QUEUE. Every synced sale enters the venue's `fiscal` image with
//! its deadline 48 h FROM THE SALE (Law 87/2019 art. 29, `fiscal::queue`), at
//! every venue -- the paper receipt promised it. SENDING STAYS OFF
//! (`fiscal::SEND_ENABLED = false`, operator 2026-09-24): nothing in this
//! module reaches the network. An entry past its deadline alerts ONCE through
//! the exception alert path (`overdue`).
//!
//! - `rules`:   PURE, the body's checks, the re-pricing into conflicts, the envelope
//! - `sync`:    PURE, the object's one turn over the log and the shelf
//! - `overdue`: PURE, the once-only alert for a sale past its 48 h
//! - `ledger`:  PURE, the owner's pane: count, oldest, overdue, conflicts
//! - `handler`: the two routes

use serde::{Deserialize, Serialize};

pub mod handler;
pub mod ledger;
pub mod overdue;
pub mod rules;
pub mod sync;

#[cfg(test)]
mod tests;

/// The id prefix every offline sale's order carries.
pub const PREFIX: &str = "offline:";

/// One line as the tablet sold it. `unit_price` IS what the guest paid for one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LineIn {
    pub product_id: String,
    pub quantity: i64,
    pub unit_price: i64,
    /// The name the tablet showed, for a dish the catalogue no longer has.
    #[serde(default)]
    pub name: String,
}

/// `POST /api/staff/offline_sales` -- one sale, as the tablet decided it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaleIn {
    pub location_id: String,
    /// Minted AT THE TAP (`lib/outbox.js newKey`); also the Idempotency-Key.
    pub sale_key: String,
    /// The tablet's clock at the tap. Checked against the server's (`rules::when`).
    pub sold_at_ms: i64,
    pub currency: String,
    /// `"cash"`: a card needs the network by nature (research §5 (a)).
    pub method: String,
    pub lines: Vec<LineIn>,
    /// Σ quantity × unit_price, as the receipt printed it.
    pub total: i64,
    /// The cached menu's `menuVersion` the sale was priced from.
    #[serde(default)]
    pub menu_version: Option<i64>,
}

/// Something that changed between the tap and the sync, kept for the owner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Conflict {
    /// `price_changed`, `off_sale`, `unknown`, `no_price`, `options`,
    /// `unreadable`, `clock`, `currency`, `tax`, `stock`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    /// What the guest paid for one (line conflicts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid: Option<i64>,
    /// The catalogue's price at the sync, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<i64>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub said: String,
}

impl Conflict {
    pub fn line(kind: &str, product_id: &str, paid: i64, now: Option<i64>) -> Self {
        Conflict { kind: kind.into(), product_id: Some(product_id.into()), paid: Some(paid), now, said: String::new() }
    }
    pub fn said(kind: &str, said: impl Into<String>) -> Self {
        Conflict { kind: kind.into(), product_id: None, paid: None, now: None, said: said.into() }
    }
}

/// What the Worker hands the object: the envelope it built, what the shelf
/// draws for it, and the two clocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncIn {
    pub order_id: String,
    pub envelope: serde_json::Value,
    /// (the product's ledger record, quantity): `stock::draws_for`'s input.
    pub bom_lines: Vec<(String, i64)>,
    /// The sale's instant, already checked (`rules::when`).
    pub sold_at_ms: i64,
    /// The sync's instant: the shelf moves now.
    pub now_ms: i64,
}

/// What the object answers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncOut {
    pub order_id: String,
    /// `true` when this sale was already in the log: nothing was appended.
    pub replayed: bool,
    pub sold_at_ms: i64,
    /// `sold_at_ms + fiscal::queue::DEADLINE_MS`.
    pub fiscal_deadline_ms: i64,
    /// `queued`, `not_owed`, or `refused: <why>` / `error: <why>`.
    pub fiscal: String,
    pub conflicts: Vec<Conflict>,
}
