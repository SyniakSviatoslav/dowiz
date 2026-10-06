//! Deterministic stock, implementing `DELIVERY-EDGE-CASES-AND-DETERMINISTIC-
//! INVENTORY-2026-07-17.md` §4 verbatim.
//!
//! The design was written and never built; the completeness audit lists it as
//! "Fully DESIGNED ... no P-number owns it". This is that module. Nothing here
//! is invented: the three mechanisms it composes -- integer checked arithmetic
//! on conserved quantities, state as a pure fold over an append-only log, and
//! executable conservation checks -- are the ones the kernel already proves for
//! money.
//!
//! THE COUNT IS ALWAYS `fold(events)`, never a stored counter. A counter can
//! drift from its own history and there is no way to tell which is right; a
//! fold cannot, because there is only one of it.
//!
//! THE AUTOMATED 86 IS A REFUSAL, NOT A FLAG. §4's I1 says an order reserving
//! more than is available is refused with a typed `OutOfStock`, and that
//! refusal IS the automatic stop-listing. Nothing has to notice a level hit
//! zero and go and change a boolean -- which is the version that races with the
//! next order and oversells.

use crate::minijson::{esc, int_field, str_field};
use std::collections::BTreeMap;

/// Quantity in the item's base unit -- pieces, grams, millilitres. Integer,
/// because a conserved quantity that can be 0.30000000000000004 is not
/// conserved.
pub type Qty = i64;

mod event;
pub use event::{moved_into, signed, signer, PrepStage, StockEvent, WasteReason};

#[derive(Debug)]
pub enum StockError {
    /// I1: the event would drive a level negative, or reserve past on_hand.
    /// The typed refusal §4 calls "the automated 86".
    OutOfStock { item: String, wanted: Qty, available: Qty },
    /// A quantity that is not a quantity. §4: "Always > 0".
    NotPositive { qty: Qty },
    /// Checked arithmetic said no. Degrade rather than fabricate.
    Overflow,
    /// I3: a reservation released twice, or consumed after release.
    Linkage(String),
    /// The record on disk is not one this can read.
    Malformed,
    /// A NEW `Wasted` or `Stocktake` with nobody's name on it. Refused at the
    /// write door only; an old unsigned record still folds.
    Unsigned,
}

impl std::fmt::Display for StockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StockError::OutOfStock { item, wanted, available } => {
                write!(f, "{item}: {wanted} wanted, {available} available")
            }
            StockError::NotPositive { qty } => write!(f, "{qty} is not a quantity"),
            StockError::Overflow => write!(f, "the arithmetic overflowed"),
            StockError::Linkage(m) => write!(f, "{m}"),
            StockError::Malformed => write!(f, "unreadable stock record"),
            StockError::Unsigned => write!(f, "a write-off or a count names who made it"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StockLevel {
    pub on_hand: Qty,
    pub reserved: Qty,
}

impl StockLevel {
    /// What a new order may take. `on_hand` includes what is already spoken
    /// for, so selling against it is how a kitchen promises the same portion
    /// twice.
    pub fn available(&self) -> Qty {
        self.on_hand.saturating_sub(self.reserved)
    }
}

/// A PROJECTION. Rebuilt by fold, never edited in place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StockLedger {
    levels: Vec<(String, StockLevel)>,
    /// Open reservations, for I3. `(order_id, item)` -> qty.
    ///
    /// A MAP, NOT A VEC (W-AUDIT M1, 2026-09-27). Both this and `served` were
    /// `Vec`s scanned with `position` on every event, so the fold every stock
    /// write runs first was O(n^2) in the log: 20 000 `Served` lines -- a busy
    /// room's year through the e-bills import, and nothing ever removes one
    /// -- folded in 896 ms native release. The map is ordered, so every
    /// reading that iterates it is as deterministic as it was.
    open: BTreeMap<(String, String), Qty>,
    /// Served and not reversed, `(order_id, item)` -> qty: what a void may
    /// put back. Empty for every log written before `served` existed.
    served: BTreeMap<(String, String), Qty>,
    /// Items whose shelf somebody has actually measured: a delivery or a
    /// count. Only these can refuse an order. A venue that has written its
    /// recipes but not yet counted its shelf would otherwise have every dish
    /// refused at checkout for stock it plainly has -- dubin-sushi was, from
    /// 2026-09-25, with 75 of 76 supplies at a never-counted zero.
    ///
    /// EVERY ORDER STILL TAKES ITS INGREDIENTS OFF THE SHELF (operator,
    /// 2026-09-26): an uncounted item's `on_hand` moves with every draw and
    /// goes NEGATIVE -- "needs a count", never an error -- and only the
    /// refusal waits for a measurement. Until then it said zero and hid how
    /// much the kitchen had used.
    counted: Vec<String>,
}

mod ledger;

mod codec;
pub use codec::{decode, encode};

#[cfg(test)]
#[path = "stock/fold/tests.rs"]
mod tests;

// ── the log ─────────────────────────────────────────────────────────────────

use bebop_store::evlog::{EvLog, Record};
use bebop_store::Store;

/// The append-only stock log.
///
/// AN EVENT LOG, not a table of levels, and that is the whole design: §4 says
/// the count is ALWAYS `fold(events)` and never a stored counter that can drift
/// from its own history. Appending is O(1) -- one object and a root relink --
/// which is why this is an EvLog rather than the eager KV the catalogue uses.
/// A restaurant emits a stock event per dish per order; a layout that rewrites
/// itself on every write would be quadratic by dinner service.
pub struct StockLog {
    store: Store,
    /// The request clock, when the caller set one ([`StockLog::set_clock`]):
    /// every record written then carries `"at"`.
    clock: Option<i64>,
    /// A checkpoint is written once this many records follow the last one
    /// ([`checkpoint::CHECKPOINT_EVERY`]).
    every: usize,
    /// The image was rewritten (`grow`) since the last checkpoint.
    grew: bool,
    /// Ids of the records quarantined for a failed crc at load (W-CRC policy: an append
    /// log serves the rest). Empty on every healthy image, so no reader pays for it.
    bad: Vec<[u8; 32]>,
}

/// Room for roughly a year of a single venue's stock events.
pub const DEFAULT_STOCK_BYTES: usize = 8 * 1024 * 1024;

mod log;

/// The signer rule and the pre-signer records (A11, 2026-09-23).
#[cfg(test)]
#[path = "stock/tests.rs"]
mod signer_tests;

/// §6.4: a served dish is recorded even past an empty shelf, and a log
/// written before `served` existed folds exactly as it did.
#[cfg(test)]
#[path = "stock/served_tests.rs"]
mod served_tests;

/// P1-4: food back from a door, resold or wasted, and old logs unchanged.
#[cfg(test)]
#[path = "stock/returned_tests.rs"]
mod returned_tests;

/// I5: prep batches -- a measurement, or stock moving between supplies.
#[cfg(test)]
#[path = "stock/produced_tests.rs"]
mod produced_tests;

/// W-NOM: a supply deleted from the nomenclature leaves every fold.
#[path = "stock/removed.rs"]
mod removed;
#[cfg(test)]
#[path = "stock/removed_tests.rs"]
mod removed_tests;

/// The owner's choice on a door-refused order, already made? Any `Returned`
/// for it says yes. And what the kitchen took for it: Σ `Consumed` per item,
/// in first-seen order -- the lines a `Returned` is written against.
pub fn returned_lines(log: &StockLog, order_id: &str) -> (bool, Vec<(String, Qty)>) {
    let mut chosen = false;
    let mut lines: Vec<(String, Qty)> = Vec::new();
    for ev in log.events() {
        match &ev {
            StockEvent::Returned { order_id: o, .. } if o == order_id => chosen = true,
            StockEvent::Consumed { item, qty, order_id: o } if o == order_id => {
                match lines.iter_mut().find(|(i, _)| i == item) {
                    Some(l) => l.1 += qty,
                    None => lines.push((item.clone(), *qty)),
                }
            }
            _ => {}
        }
    }
    (chosen, lines)
}

/// The shortfalls: every item whose level a served dish drove below zero.
/// Sorted, like `items`, so two folds report the same list.
pub fn short(ledger: &StockLedger) -> Vec<(String, Qty)> {
    ledger.items().into_iter().filter(|(_, l)| l.on_hand < 0).map(|(i, l)| (i, l.on_hand)).collect()
}

#[cfg(test)]
#[path = "stock/log/tests.rs"]
mod log_tests;

mod bom;
pub use bom::{bom_of, bom_of_product, reservations_for, settle, BomLine};

#[cfg(test)]
#[path = "stock/bom/tests.rs"]
mod bom_tests;

/// Cost that follows purchases (§2.10): priced receipts on this log, WAC fold.
pub mod cost;
/// When, which lot, from whom: the keys a record carries besides its movement.
pub mod meta;
/// ONE pass over the raw log: every record dated, valued and folded.
pub mod journal;
/// Lots on hand, first-expiry-first-out.
pub mod lots;
/// The fold's state as a record in the chain: fold = checkpoint + tail (R7).
pub mod checkpoint;
/// The carried remainder of fractional draws, and the exact write door.
pub mod carry;
pub use carry::{draws_for, Draw};
/// A basket whose dishes reach a semi-finished product kept ready.
pub mod basket;
/// The production act: a batch of a semi-finished product cooked ahead.
pub mod act;
/// Records in the chain that never move the shelf: supplier cards, orders sent (W-STOCK P5).
pub mod notes;
/// Where a supply is -- kitchen, bar, freezer -- and transfers between them (P12).
pub mod storages;
/// Raw-fish freezing records and the HACCP traceability export (P13).
pub mod haccp;

/// W-AUDIT S7 (2026-09-27): the recipe is read through brackets inside names
/// and never from the next array in the record.
#[cfg(test)]
mod bom_audit_tests {
    use super::bom_of;

    #[test]
    fn the_recipe_is_read_whole_and_only_from_its_own_array() {
        let lines = bom_of(r#"{"bom":[{"supply":"a]","qty":1},{"supply":"rice","qty":90}]}"#);
        assert_eq!(lines.iter().map(|l| (l.supply.as_str(), l.qty)).collect::<Vec<_>>(), vec![("a]", 1), ("rice", 90)]);
        assert!(bom_of(r#"{"bom":null,"ingredients":[{"supply":"x","qty":5}]}"#).is_empty(), "null is not the next array");
        assert!(bom_of(r#"{"name":"bom","ingredients":[{"supply":"x","qty":5}]}"#).is_empty(), "a value is not the key");
        assert!(bom_of(r#"{"bom":[{"supply":"salmon","qty":40.5}]}"#).is_empty(), "a fractional qty is refused, not truncated");
        let nested = bom_of(r#"{"modifierGroups":[{"options":[{"id":"x"}]}],"bom":[{"supply":"nori","qty":1,"tags":["a","b"]},{"supply":"rice","qty":90}]}"#);
        assert_eq!(nested.len(), 2, "a nested array inside a line does not end the recipe: {nested:?}");
    }
}

/// P12's old-image rule as a golden (W-STORE).
#[cfg(test)]
#[path = "stock/oldimage_tests.rs"]
mod oldimage_tests;

/// Lost sales: a basket the shelf refused, as a note that moves nothing (A13, W-LOST).
pub mod refused;
