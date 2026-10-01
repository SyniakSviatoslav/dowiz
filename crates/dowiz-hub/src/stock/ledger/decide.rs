//! THE GATE: `decide` runs before anything is written, and a refusal here is
//! final -- oversell is impossible by structure, not avoided by procedure.

use super::*;

impl StockLedger {
    /// §4.1's fail-closed gate, run BEFORE anything is written.
    ///
    /// Nothing is persisted when this refuses -- the Law pole of the commit
    /// error, never retried. That is what makes oversell structurally
    /// impossible rather than procedurally avoided.
    pub fn decide(&self, ev: &StockEvent) -> Result<(), StockError> {
        // "Always > 0" applies to every quantity except an observed count,
        // which may legitimately be zero: a shelf can be empty.
        match ev {
            StockEvent::Stocktake { observed, .. } => {
                if *observed < 0 {
                    return Err(StockError::NotPositive { qty: *observed });
                }
            }
            // A deletion moves no quantity, and deleting nothing is not wrong.
            StockEvent::Removed { .. } => return Ok(()),
            _ => {
                let q = match ev {
                    StockEvent::Received { qty, .. }
                    | StockEvent::Reserved { qty, .. }
                    | StockEvent::Consumed { qty, .. }
                    | StockEvent::Released { qty, .. }
                    | StockEvent::Wasted { qty, .. }
                    | StockEvent::Served { qty, .. }
                    | StockEvent::Returned { qty, .. }
                    | StockEvent::Unserved { qty, .. }
                    | StockEvent::Produced { qty, .. }
                    | StockEvent::Cooked { qty, .. }
                    | StockEvent::Made { qty, .. } => *qty,
                    StockEvent::Stocktake { .. } | StockEvent::Removed { .. } => unreachable!(),
                };
                // AN ORDER LINE MAY BE ZERO (SPEC-SEMI-FINISHED §c): a draw of
                // 0.3 g of salt through a semi-finished card books 0 whole
                // grams THIS sale and must still be a record, or the carried
                // remainder forgets it and every later draw rounds to 0 for
                // ever. Everything a person types (a delivery, a write-off, a
                // prep) keeps §4's "Always > 0".
                let order_line = matches!(
                    ev,
                    StockEvent::Reserved { .. } | StockEvent::Consumed { .. } | StockEvent::Released { .. } | StockEvent::Served { .. } | StockEvent::Unserved { .. } | StockEvent::Cooked { .. }
                );
                if q < 0 || (q == 0 && !order_line) {
                    return Err(StockError::NotPositive { qty: q });
                }
            }
        }

        let lvl = self.level(ev.item());
        match ev {
            StockEvent::Received { qty, .. } | StockEvent::Returned { qty, resell: true, .. } => {
                lvl.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Reserved { item, qty, .. } => {
                // I1. THE AUTOMATED 86: there is no flag to set and no race to
                // lose, because the refusal is the mechanism -- for an item
                // somebody has counted. An uncounted zero is not a measurement.
                if self.is_counted(item) && *qty > lvl.available() {
                    return Err(StockError::OutOfStock {
                        item: item.clone(),
                        wanted: *qty,
                        available: lvl.available(),
                    });
                }
            }
            // A zero line consumes or releases nothing: nothing to check.
            StockEvent::Consumed { qty: 0, .. } | StockEvent::Released { qty: 0, .. } => {}
            StockEvent::Consumed { item, qty, order_id } => {
                let held = self.held(order_id, item);
                if held == 0 {
                    return Err(StockError::Linkage(format!(
                        "{order_id} has no open reservation for {item}"
                    )));
                }
                if *qty > held {
                    return Err(StockError::Linkage(format!(
                        "{order_id} reserved {held} of {item}, cannot consume {qty}"
                    )));
                }
                // NO SHELF CHECK HERE. The reservation was the check, at the
                // moment the customer was promised the dish; a later move of the
                // order (to PREPARING, or straight to IN_DELIVERY) must never be
                // refused because the shelf moved since. What was cooked is
                // recorded, as `Served` is, whatever the shelf now says.
            }
            StockEvent::Released { item, qty, order_id } => {
                let held = self.held(order_id, item);
                if held == 0 {
                    return Err(StockError::Linkage(format!(
                        "{order_id} has nothing reserved of {item} to release"
                    )));
                }
                if *qty > held {
                    return Err(StockError::Linkage(format!(
                        "{order_id} holds {held} of {item}, cannot release {qty}"
                    )));
                }
            }
            StockEvent::Wasted { item, qty, .. } => {
                // Waste comes off the shelf, and what is reserved is still
                // owed to somebody. Wasting into a reservation would let a
                // kitchen bin a portion it has already promised. An uncounted
                // shelf has no measured level to protect, so it only records.
                if self.is_counted(item) && *qty > lvl.on_hand.saturating_sub(lvl.reserved) {
                    return Err(StockError::OutOfStock {
                        item: item.clone(),
                        wanted: *qty,
                        available: lvl.available(),
                    });
                }
            }
            StockEvent::Stocktake { item, observed, .. } => {
                // A count below what is already promised cannot be honoured.
                // Refusing makes the person recount; accepting would make the
                // ledger claim it owes more than it has.
                if *observed < lvl.reserved {
                    return Err(StockError::Linkage(format!(
                        "{item}: {} is reserved, an observed count of {observed} cannot be right",
                        lvl.reserved
                    )));
                }
            }
            // §6.4: what was served is recorded, whatever the shelf says. Only
            // the arithmetic can refuse it.
            StockEvent::Served { qty, .. } => {
                lvl.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Unserved { item, qty, order_id } => {
                let had = self.served_qty(order_id, item);
                if *qty > had {
                    return Err(StockError::Linkage(format!(
                        "{order_id} has {had} of {item} served, cannot put back {qty}"
                    )));
                }
                lvl.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
            // Waste of food `Consumed` already took: the shelf does not move,
            // so there is nothing for it to refuse.
            StockEvent::Returned { resell: false, .. } | StockEvent::Removed { .. } => {}
            StockEvent::Produced { item, qty, out, into, .. } => {
                if *out < 0 {
                    return Err(StockError::NotPositive { qty: *out });
                }
                // Moving stock takes it off the unspoken-for shelf, as waste
                // does; a measurement moves nothing and refuses nothing.
                if let Some(to) = moved_into(item, into) {
                    if self.is_counted(item) && *qty > lvl.available() {
                        return Err(StockError::OutOfStock { item: item.clone(), wanted: *qty, available: lvl.available() });
                    }
                    self.level(to).on_hand.checked_add(*out).ok_or(StockError::Overflow)?;
                }
            }
            // What went into a batch was used, whatever the shelf says (the act
            // takes a ready product only as far as the shelf has it).
            StockEvent::Cooked { qty, .. } => {
                lvl.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Made { qty, planned, gross, .. } => {
                if *planned <= 0 || *gross < 0 {
                    return Err(StockError::NotPositive { qty: (*planned).min(*gross) });
                }
                lvl.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
        }
        Ok(())
    }
}
