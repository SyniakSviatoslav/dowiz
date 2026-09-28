//! A SUPPLY DELETED FROM THE NOMENCLATURE (W-NOM, 2026-09-28): what
//! [`super::StockEvent::Removed`] does to the shelf's fold.
//!
//! THE LOG KEEPS ITS HISTORY; THE FOLD FORGETS THE ITEM. Every record before
//! the deletion stays in the chain (a delivery that happened is a fact), and
//! the fold drops what it knew of the item: its level, its "counted" mark,
//! every order's hold on it and every till sale a void could put back. So a
//! supply re-created later under the same id starts from nothing, instead of
//! inheriting the old shelf -- which is what `Catalog::remove_supply` warned
//! about. The lots follow on their own (the journal reconciles them to the
//! shelf, now zero) and the cost book drops its pool (`cost.rs`).
//!
//! AN ORDER THAT HELD IT STILL MOVES: `settle` derives from the holds, and
//! there is none left for this item, so the order cooks or cancels with the
//! lines that still exist -- nothing is consumed, and nothing is refused.

use super::StockLedger;

impl StockLedger {
    pub(super) fn forget(&mut self, item: &str) {
        self.levels.retain(|(i, _)| i != item);
        self.counted.retain(|i| i != item);
        self.open.retain(|(_, i), _| i != item);
        self.served.retain(|(_, i), _| i != item);
    }
}
