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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasteReason {
    Spoiled,
    Dropped,
    /// Made and not sold.
    Unsold,
    /// Food back from a delivery refused at the door (P1-4).
    Returned,
    /// Eaten by the staff, or given as a sample.
    StaffMeal,
}

impl WasteReason {
    pub fn as_str(self) -> &'static str {
        match self {
            WasteReason::Spoiled => "spoiled",
            WasteReason::Dropped => "dropped",
            WasteReason::Unsold => "unsold",
            WasteReason::Returned => "returned",
            WasteReason::StaffMeal => "staff_meal",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "spoiled" => Some(WasteReason::Spoiled),
            "dropped" => Some(WasteReason::Dropped),
            "unsold" => Some(WasteReason::Unsold),
            "returned" => Some(WasteReason::Returned),
            "staff_meal" => Some(WasteReason::StaffMeal),
            _ => None,
        }
    }

    /// The whole closed set, in wire words -- what a refusal names.
    pub const ALL: [WasteReason; 5] = [
        WasteReason::Spoiled,
        WasteReason::Dropped,
        WasteReason::Unsold,
        WasteReason::Returned,
        WasteReason::StaffMeal,
    ];

    /// "spoiled, dropped, unsold, returned, staff_meal", for a 400's text.
    pub fn allowed_words() -> String {
        Self::ALL.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(", ")
    }
}

/// §4.1's event family, plus the signer on the two a PERSON causes.
///
/// `by` IS THE SIGNER'S PERSON ID, AND `""` MEANS "RECORDED BEFORE SIGNERS".
/// Every `Wasted` and `Stocktake` written before 2026-09-23 has no `by` on the
/// wire; it decodes as `""` and FOLDS EXACTLY AS IT ALWAYS DID -- a venue's
/// shelf is the fold of its whole history, so a fold that refused the old
/// records would refuse every order the venue takes afterwards. The signer is
/// required of NEW writes only, at the write door ([`StockLog::append`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockEvent {
    /// restock: on_hand += qty
    Received { item: String, qty: Qty },
    /// order placed: reserved += qty
    Reserved { item: String, qty: Qty, order_id: String },
    /// prep started: on_hand -= qty, reserved -= qty
    Consumed { item: String, qty: Qty, order_id: String },
    /// cancel or reject before prep: reserved -= qty
    Released { item: String, qty: Qty, order_id: String },
    /// spoilage or a dropped tray: on_hand -= qty
    Wasted { item: String, qty: Qty, reason: WasteReason, by: String },
    /// a human counted: on_hand := observed, and the conservation basis resets
    Stocktake { item: String, observed: Qty, stocktake_id: String, by: String },
    /// A DISH ALREADY SERVED, recorded after the fact (an ebills till import,
    /// BLUEPRINT-EBILLS §6.4): on_hand -= qty, and it MAY GO NEGATIVE.
    ///
    /// Never refused for the shelf, unlike every other subtraction here. The
    /// food has left the kitchen; refusing the record would make the ledger
    /// claim a portion is on the shelf that a guest has eaten. A negative
    /// level IS the report -- the next `Reserved` sees nothing available (the
    /// automated 86 still holds), and the owner's view shows the shortfall
    /// until a delivery or a count settles it. Holds no reservation.
    Served { item: String, qty: Qty, order_id: String },
    /// FOOD BACK FROM A DOOR (P1-4, BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.4),
    /// one per line the order `Consumed`, written once per order by the
    /// owner's choice -- never inferred.
    ///
    /// `resell`: on_hand += qty. The dish goes back on the shelf, and the next
    /// order's `Consumed` takes it again.
    /// Otherwise it is WASTE and on_hand DOES NOT MOVE: `Consumed` already took
    /// these ingredients at PREPARING. §2.4 names a `Wasted` per line here;
    /// that subtracts on_hand a second time for the same food, and is refused
    /// outright whenever the unreserved shelf is short of the order's BOM.
    /// This record is what the waste report counts instead.
    ///
    /// `by` is the courier who brought it back; `chosen_by` the person who
    /// decided. It carries its order, so "was this order's choice made" is a
    /// question the stock log answers alone.
    Returned { item: String, qty: Qty, order_id: String, resell: bool, by: String, chosen_by: String },
    /// A `Served` draw REVERSED: the till voided the sale (`changedStatus:
    /// CANCELLED`, BLUEPRINT-EBILLS §6.6). on_hand += qty. Refused unless the
    /// order has at least `qty` of `item` served and not yet reversed -- a
    /// void puts back only what its sale took, and never twice.
    Unserved { item: String, qty: Qty, order_id: String },
    /// PREP (research 2026-09-26 §3.4): `qty` of `item` went onto the board
    /// raw and `out` came off it after `stage` -- cleaning or cooking.
    ///
    /// `into`: the supply that came off, when it is a DIFFERENT one the
    /// kitchen stocks (whole salmon -> salmon fillet): `item` loses `qty`,
    /// `into` gains `out`. `None` is a MEASUREMENT only -- the shelf is still
    /// counted gross, so nothing moves and the record is the yield. Signed.
    Produced { item: String, qty: Qty, out: Qty, stage: PrepStage, into: Option<String>, by: String },
    /// THE SUPPLY WAS DELETED from the nomenclature (W-NOM, 2026-09-28): its
    /// shelf, holds, lots and cost leave every fold; the records before stay.
    /// NOT a write-off -- no quantity, no waste, no value. Signed.
    Removed { item: String, by: String },
    /// A PRODUCTION ACT'S INPUT (акт приготування, W-PF2 R2): `qty` of `item`
    /// went into batch `act` of the semi-finished product `into`, by its card.
    /// on_hand -= qty and MAY GO NEGATIVE: what was cooked is recorded, as
    /// `Consumed` and `Served` are. May be ZERO with a `uq` (the carry).
    Cooked { item: String, qty: Qty, into: String, act: String, by: String },
    /// A PRODUCTION ACT'S OUTPUT: `qty` of the semi-finished `item` onto the
    /// shelf, WEIGHED -- so the item is counted from here. `planned`: what
    /// the card makes from the inputs; `gross`: their grams (0 unknown), so
    /// the loss on cooking is `gross - qty`. Its value rides on the record.
    Made { item: String, qty: Qty, planned: Qty, gross: Qty, act: String, by: String },
}

/// Which loss a [`StockEvent::Produced`] measured: raw -> net (cleaning) or
/// net -> out (cooking). The supply's `cleanPm` / `cookPm` are its defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrepStage {
    Clean,
    Cook,
}

impl PrepStage {
    pub fn as_str(self) -> &'static str {
        match self {
            PrepStage::Clean => "clean",
            PrepStage::Cook => "cook",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "clean" => Some(PrepStage::Clean),
            "cook" => Some(PrepStage::Cook),
            _ => None,
        }
    }
}

impl StockEvent {
    pub fn item(&self) -> &str {
        match self {
            StockEvent::Received { item, .. }
            | StockEvent::Reserved { item, .. }
            | StockEvent::Consumed { item, .. }
            | StockEvent::Released { item, .. }
            | StockEvent::Wasted { item, .. }
            | StockEvent::Stocktake { item, .. }
            | StockEvent::Served { item, .. }
            | StockEvent::Returned { item, .. }
            | StockEvent::Unserved { item, .. }
            | StockEvent::Produced { item, .. }
            | StockEvent::Removed { item, .. }
            | StockEvent::Cooked { item, .. }
            | StockEvent::Made { item, .. } => item,
        }
    }

    pub fn order_id(&self) -> Option<&str> {
        match self {
            StockEvent::Reserved { order_id, .. }
            | StockEvent::Consumed { order_id, .. }
            | StockEvent::Released { order_id, .. }
            | StockEvent::Served { order_id, .. }
            | StockEvent::Returned { order_id, .. }
            | StockEvent::Unserved { order_id, .. } => Some(order_id),
            _ => None,
        }
    }
}

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

impl StockLedger {
    pub fn level(&self, item: &str) -> StockLevel {
        self.levels
            .iter()
            .find(|(i, _)| i == item)
            .map(|(_, l)| *l)
            .unwrap_or_default()
    }

    pub fn available(&self, item: &str) -> Qty {
        self.level(item).available()
    }

    /// Whether this item's shelf has ever been measured (received or counted).
    /// An uncounted item is reserved and consumed like any other -- its
    /// `on_hand` goes below zero -- but never refuses an order: its level is
    /// "what was used since nobody looked", not what is there.
    pub fn is_counted(&self, item: &str) -> bool {
        self.counted.iter().any(|i| i == item)
    }

    /// Every item this ledger knows about, sorted, so two folds of one log
    /// produce byte-identical output (I4).
    pub fn items(&self) -> Vec<(String, StockLevel)> {
        let mut out = self.levels.clone();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn level_mut(&mut self, item: &str) -> &mut StockLevel {
        if let Some(pos) = self.levels.iter().position(|(i, _)| i == item) {
            return &mut self.levels[pos].1;
        }
        self.levels.push((item.to_string(), StockLevel::default()));
        let last = self.levels.len() - 1;
        &mut self.levels[last].1
    }

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

    fn served_qty(&self, order_id: &str, item: &str) -> Qty {
        self.served.get(&(order_id.to_string(), item.to_string())).copied().unwrap_or(0)
    }

    /// What an order has served and not had reversed, `(item, qty)`, sorted:
    /// exactly the `Unserved` a void of it emits.
    pub fn served_of(&self, order_id: &str) -> Vec<(String, Qty)> {
        // The map is ordered by (order, item): one order's lines are one
        // contiguous, item-sorted range.
        self.served
            .range((order_id.to_string(), String::new())..)
            .take_while(|((o, _), _)| o == order_id)
            .map(|((_, i), q)| (i.clone(), *q))
            .collect()
    }

    fn held(&self, order_id: &str, item: &str) -> Qty {
        self.open.get(&(order_id.to_string(), item.to_string())).copied().unwrap_or(0)
    }

    fn mark_counted(&mut self, item: &str) {
        if !self.is_counted(item) {
            self.counted.push(item.to_string());
        }
    }

    fn take_held(&mut self, order_id: &str, item: &str, qty: Qty) {
        let key = (order_id.to_string(), item.to_string());
        if let Some(q) = self.open.get_mut(&key) {
            *q -= qty;
            if *q <= 0 {
                self.open.remove(&key);
            }
        }
    }

    fn apply(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        self.decide(ev)?;
        match ev {
            StockEvent::Received { item, qty } => {
                // THE FIRST DELIVERY ARMS THE ITEM AT WHAT CAME IN, as it
                // always did: an uncounted item's negative is use nobody
                // measured against, not a debt this box of salmon pays.
                let was_counted = self.is_counted(item);
                self.mark_counted(item);
                let l = self.level_mut(item);
                if !was_counted {
                    l.on_hand = l.on_hand.max(0);
                }
                l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Reserved { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.reserved = l.reserved.checked_add(*qty).ok_or(StockError::Overflow)?;
                *self.open.entry((order_id.clone(), item.clone())).or_insert(0) += qty;
            }
            StockEvent::Consumed { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
                l.reserved = l.reserved.checked_sub(*qty).ok_or(StockError::Overflow)?;
                self.take_held(order_id, item, *qty);
            }
            StockEvent::Released { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.reserved = l.reserved.checked_sub(*qty).ok_or(StockError::Overflow)?;
                self.take_held(order_id, item, *qty);
            }
            StockEvent::Wasted { item, qty, .. } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Stocktake { item, observed, .. } => {
                // The basis resets here (I2): everything before this count
                // stops contributing to on_hand. Reservations survive, because
                // they are promises to customers, not a property of the shelf.
                self.mark_counted(item);
                let l = self.level_mut(item);
                l.on_hand = *observed;
            }
            StockEvent::Served { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
                let q = self.served.entry((order_id.clone(), item.clone())).or_insert(0);
                *q = q.saturating_add(*qty);
            }
            StockEvent::Unserved { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
                let key = (order_id.clone(), item.clone());
                if let Some(q) = self.served.get_mut(&key) {
                    *q -= qty;
                    if *q <= 0 {
                        self.served.remove(&key);
                    }
                }
            }
            StockEvent::Returned { item, qty, resell, .. } => {
                if *resell {
                    let l = self.level_mut(item);
                    l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
                }
            }
            StockEvent::Produced { item, qty, out, into, .. } => {
                if let Some(to) = moved_into(item, into).map(str::to_string) {
                    let l = self.level_mut(item);
                    l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
                    // What came off the board was weighed: a measurement.
                    self.mark_counted(&to);
                    let l = self.level_mut(&to);
                    l.on_hand = l.on_hand.checked_add(*out).ok_or(StockError::Overflow)?;
                }
            }
            StockEvent::Removed { item, .. } => self.forget(item),
            StockEvent::Cooked { item, qty, .. } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_sub(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Made { item, qty, .. } => {
                // The batch was weighed: a measurement, like a delivery's.
                let was_counted = self.is_counted(item);
                self.mark_counted(item);
                let l = self.level_mut(item);
                if !was_counted {
                    l.on_hand = l.on_hand.max(0);
                }
                l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
        }
        Ok(())
    }

    /// Pure, checked, deterministic. The only way to obtain a ledger.
    pub fn fold(events: &[StockEvent]) -> Result<StockLedger, StockError> {
        let mut led = StockLedger::default();
        for ev in events {
            led.apply(ev)?;
        }
        Ok(led)
    }

    /// I3, checked over a whole log: every reservation is eventually matched.
    ///
    /// Returns the ones that are not. A stranded reservation is stock a venue
    /// believes it owes to an order that ended -- the slow leak that makes a
    /// kitchen think it is out of something it has.
    pub fn stranded(&self) -> Vec<(String, String, Qty)> {
        let mut out: Vec<(String, String, Qty)> = self
            .open
            .iter()
            .map(|((o, i), q)| (o.clone(), i.clone(), *q))
            .collect();
        out.sort();
        out
    }
}

/// The supply a `Produced` moves stock into, or `None` for a measurement
/// (no `into`, or `into` naming the input itself).
pub fn moved_into<'a>(item: &str, into: &'a Option<String>) -> Option<&'a str> {
    into.as_deref().filter(|t| !t.is_empty() && *t != item)
}

/// THE SIGNER RULE, for a NEW event: a write-off or a count names a person.
///
/// Deliberately NOT in `decide`, which `fold` runs over the whole history: an
/// unsigned record written before signers existed must keep folding, or the
/// venue's shelf -- and every order that reserves against it -- stops.
pub fn signed(ev: &StockEvent) -> Result<(), StockError> {
    match ev {
        StockEvent::Wasted { by, .. }
        | StockEvent::Stocktake { by, .. }
        | StockEvent::Produced { by, .. }
        | StockEvent::Removed { by, .. }
        | StockEvent::Cooked { by, .. }
        | StockEvent::Made { by, .. }
            if by.trim().is_empty() =>
        {
            Err(StockError::Unsigned)
        }
        StockEvent::Returned { chosen_by, .. } if chosen_by.trim().is_empty() => Err(StockError::Unsigned),
        _ => Ok(()),
    }
}

/// The signer of an event, or `None` for one no person signs (the order
/// lifecycle's) and for a write-off recorded before signers existed.
pub fn signer(ev: &StockEvent) -> Option<&str> {
    match ev {
        StockEvent::Wasted { by, .. }
        | StockEvent::Stocktake { by, .. }
        | StockEvent::Produced { by, .. }
        | StockEvent::Removed { by, .. }
        | StockEvent::Cooked { by, .. }
        | StockEvent::Made { by, .. }
            if !by.is_empty() =>
        {
            Some(by)
        }
        StockEvent::Returned { chosen_by, .. } if !chosen_by.is_empty() => Some(chosen_by),
        _ => None,
    }
}

// ── serialisation, so the log survives a restart ────────────────────────────

pub fn encode(ev: &StockEvent) -> String {
    match ev {
        StockEvent::Received { item, qty } => {
            format!(r#"{{"k":"received","item":"{}","qty":{qty}}}"#, esc(item))
        }
        StockEvent::Reserved { item, qty, order_id } => format!(
            r#"{{"k":"reserved","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Consumed { item, qty, order_id } => format!(
            r#"{{"k":"consumed","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Released { item, qty, order_id } => format!(
            r#"{{"k":"released","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Wasted { item, qty, reason, by } => format!(
            r#"{{"k":"wasted","item":"{}","qty":{qty},"reason":"{}","by":"{}"}}"#,
            esc(item),
            reason.as_str(),
            esc(by)
        ),
        StockEvent::Stocktake { item, observed, stocktake_id, by } => format!(
            r#"{{"k":"stocktake","item":"{}","observed":{observed},"id":"{}","by":"{}"}}"#,
            esc(item),
            esc(stocktake_id),
            esc(by)
        ),
        StockEvent::Served { item, qty, order_id } => format!(
            r#"{{"k":"served","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Unserved { item, qty, order_id } => format!(
            r#"{{"k":"unserved","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Returned { item, qty, order_id, resell, by, chosen_by } => format!(
            r#"{{"k":"returned","item":"{}","qty":{qty},"order":"{}","resell":{},"by":"{}","chosen_by":"{}"}}"#,
            esc(item),
            esc(order_id),
            i64::from(*resell),
            esc(by),
            esc(chosen_by)
        ),
        StockEvent::Produced { item, qty, out, stage, into, by } => format!(
            r#"{{"k":"produced","item":"{}","qty":{qty},"out":{out},"stage":"{}","into":"{}","by":"{}"}}"#,
            esc(item),
            stage.as_str(),
            esc(into.as_deref().unwrap_or("")),
            esc(by)
        ),
        StockEvent::Removed { item, by } => format!(r#"{{"k":"removed","item":"{}","by":"{}"}}"#, esc(item), esc(by)),
        StockEvent::Cooked { item, qty, into, act, by } => format!(
            r#"{{"k":"cooked","item":"{}","qty":{qty},"into":"{}","act":"{}","by":"{}"}}"#,
            esc(item),
            esc(into),
            esc(act),
            esc(by)
        ),
        StockEvent::Made { item, qty, planned, gross, act, by } => format!(
            r#"{{"k":"made","item":"{}","qty":{qty},"planned":{planned},"gross":{gross},"act":"{}","by":"{}"}}"#,
            esc(item),
            esc(act),
            esc(by)
        ),
    }
}

pub fn decode(rec: &str) -> Option<StockEvent> {
    let item = str_field(rec, "item")?;
    match str_field(rec, "k")?.as_str() {
        "received" => Some(StockEvent::Received { item, qty: int_field(rec, "qty")? }),
        "reserved" => Some(StockEvent::Reserved {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "consumed" => Some(StockEvent::Consumed {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "released" => Some(StockEvent::Released {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "wasted" => Some(StockEvent::Wasted {
            item,
            qty: int_field(rec, "qty")?,
            reason: WasteReason::from_str(&str_field(rec, "reason")?)?,
            // Absent on every record written before 2026-09-23: "" = unsigned.
            by: str_field(rec, "by").unwrap_or_default(),
        }),
        "stocktake" => Some(StockEvent::Stocktake {
            item,
            observed: int_field(rec, "observed")?,
            stocktake_id: str_field(rec, "id")?,
            // Absent on every record written before 2026-09-23: "" = unsigned.
            by: str_field(rec, "by").unwrap_or_default(),
        }),
        "served" => Some(StockEvent::Served {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "unserved" => Some(StockEvent::Unserved {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "returned" => Some(StockEvent::Returned {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
            resell: match int_field(rec, "resell")? { 0 => false, 1 => true, _ => return None },
            by: str_field(rec, "by")?,
            chosen_by: str_field(rec, "chosen_by")?,
        }),
        "produced" => Some(StockEvent::Produced {
            item,
            qty: int_field(rec, "qty")?,
            out: int_field(rec, "out")?,
            stage: PrepStage::from_str(&str_field(rec, "stage")?)?,
            into: str_field(rec, "into").filter(|t| !t.is_empty()),
            by: str_field(rec, "by")?,
        }),
        "removed" => Some(StockEvent::Removed { item, by: str_field(rec, "by")? }),
        "cooked" => Some(StockEvent::Cooked {
            item,
            qty: int_field(rec, "qty")?,
            into: str_field(rec, "into")?,
            act: str_field(rec, "act")?,
            by: str_field(rec, "by")?,
        }),
        "made" => Some(StockEvent::Made {
            item,
            qty: int_field(rec, "qty")?,
            planned: int_field(rec, "planned")?,
            gross: int_field(rec, "gross")?,
            act: str_field(rec, "act")?,
            by: str_field(rec, "by")?,
        }),
        _ => None,
    }
}

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
}

/// Room for roughly a year of a single venue's stock events.
pub const DEFAULT_STOCK_BYTES: usize = 8 * 1024 * 1024;

impl StockLog {
    /// What this image has spent. See [`crate::Usage`].
    ///
    /// GROWS RATHER THAN REFUSING: `write` doubles the image and copies the
    /// chain when the arena fills, retrying up to six times. Measured at 7168
    /// cells growing to 523264 over four thousand events with no refusal, so
    /// this reading predicts a doubling rather than a failure.
    /// How many events the log holds, from the root's counter. Survives
    /// `grow()` unchanged, which the store generation does not.
    pub fn len(&self) -> usize {
        EvLog::len(&self.store)
    }

    pub fn usage(&self) -> crate::Usage {
        let cap = self.store.capacity_cells();
        crate::usage_of_kind(&self.store, cap, true)
    }

    pub fn create() -> Result<Self, crate::HubError> {
        Self::create_sized(DEFAULT_STOCK_BYTES)
    }

    /// A log that starts at a chosen size.
    ///
    /// The default is a year of a venue's movements, which is right on a disk
    /// and wrong on a Worker: 8 MiB is nine D1 chunks read and written on every
    /// order that reserves an ingredient. The log grows itself when an append
    /// does not fit, so a small start costs a few doublings and nothing else.
    pub fn create_sized(bytes: usize) -> Result<Self, crate::HubError> {
        let mut store = Store::create_bytes(bytes)?;
        EvLog::init_bytes(&mut store)?;
        Ok(StockLog { store, clock: None, every: checkpoint::CHECKPOINT_EVERY, grew: false })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, crate::HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(crate::HubError::NotAHub);
        }
        // AND REFUSES ONE THAT LOST RECORDS ON THE WAY HERE (W-AUDIT S3,
        // 2026-09-27), as `Hub::load` and `LogImage::load` have since the
        // short-read defect. This loader alone trusted the superblock: a stock
        // image cut short LOADED, folded a partial history, and -- because
        // `append` re-folds the whole log first -- a `Consumed` whose
        // `Reserved` was in the lost tail turned every later stock write into
        // a `Linkage` refusal. Refusing here is what a caller can act on.
        crate::chain_is_whole(&store)?;
        Ok(StockLog { store, clock: None, every: checkpoint::CHECKPOINT_EVERY, grew: false })
    }

    /// FULL CAPACITY: `grow()` doubles from this length, so it keeps the
    /// zeros. Persist `to_bytes_trimmed`.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.store.to_bytes()
    }

    /// The image without the unused tail of its arena. Reloads identical:
    /// `Store::from_bytes` pads the zeros back from the superblock's capacity.
    pub fn to_bytes_trimmed(&self) -> Vec<u8> {
        self.store.to_bytes_trimmed()
    }

    /// Every event, OLDEST FIRST.
    ///
    /// `EvLog::walk` returns newest first -- the root points at the last record
    /// and each links back to its predecessor -- which is right for "show me
    /// what just happened" and catastrophic for a fold. Applied in that order a
    /// reservation lands before the delivery that made it possible, and the
    /// ledger refuses its own history: the first symptom was a log that
    /// replayed as `OutOfStock` for stock it plainly had.
    ///
    /// A fold is defined over time moving forwards. The reversal belongs here,
    /// once, rather than at each of the three call sites.
    pub fn events(&self) -> Vec<StockEvent> {
        let mut out: Vec<StockEvent> = EvLog::walk(&self.store)
            .into_iter()
            .filter_map(|r| decode(&String::from_utf8_lossy(&r.payload)))
            .collect();
        out.reverse();
        out
    }

    /// One record, chained to the previous. The chain is what makes the log
    /// tamper-evident: editing any event changes every content id after it,
    /// which is I4's other half. Takes the bytes, so a test can lay down an
    /// OLDER encoding and prove the fold still reads them, and a checkpoint
    /// is written through the same door as an event.
    fn write_payload(&mut self, payload: Vec<u8>) -> Result<(), StockError> {
        let prev = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
        // CHAINED, which is what the comment above has always claimed: the id
        // commits to the previous record, so editing an event breaks every id
        // after it. It hashed the payload alone until 2026-09-20.
        let id = crate::content_id_chained(&prev, &payload);
        let rec = Record {
            id,
            prev,
            // Stock events are the venue's own, recorded by the hub rather than
            // signed by a person; the actor slot is zero rather than borrowing
            // an identity that did not act.
            actor_pubkey: [0u8; 32],
            // The root's own counter, not a walk: `events().len()` decoded the
            // whole stock log on every single delivery just to number it.
            actor_seq: EvLog::len(&self.store) as u64,
            payload,
        };
        // THE IMAGE GROWS RATHER THAN REFUSING, like the order log. A shelf
        // that cannot record a delivery because its arena is full is a kitchen
        // that stops being able to sell -- the refusal path reads this ledger.
        // ── APPEND AND TIP ARE ONE COMMIT ──
        //
        // They used to be two, and they failed differently: measured, on a
        // nearly full arena the RECORD still fits while the tip update does
        // not. `walk` follows the store's object chain rather than the tip
        // hash, so a record written without its tip is already IN the chain --
        // re-appending it after growing counted it twice, which is exactly
        // what the first version of this did (3002 deliveries recorded for
        // 3000 made).
        //
        // `append_tip_bytes` allocates the record and the new root in ONE
        // transaction: either both fit or nothing is committed and the arena
        // cursor has not moved. So a failure leaves no record to double-count,
        // and growing and trying again is the whole recovery.
        let mut placed = EvLog::append_tip_bytes(&mut self.store, &rec).is_ok();
        for _ in 0..6 {
            if placed {
                break;
            }
            self.grow().map_err(|_| StockError::Malformed)?;
            placed = EvLog::append_tip_bytes(&mut self.store, &rec).is_ok();
        }
        if !placed {
            return Err(StockError::Malformed);
        }
        Ok(())
    }

    /// Double the image and copy the chain across, oldest first.
    ///
    /// `walk` is newest-first, so the copy is reversed: appending in the wrong
    /// order leaves every `prev` pointing at a record that does not exist yet,
    /// which is a heap of orphans that still looks like a log.
    fn grow(&mut self) -> Result<(), crate::HubError> {
        let mut records = EvLog::walk(&self.store);
        records.reverse();
        let bigger = self.store.to_bytes().len().saturating_mul(2).max(64 * 1024);
        let mut fresh = Store::create_bytes(bigger)?;
        EvLog::init_bytes(&mut fresh)?;
        let mut last = None;
        for r in &records {
            EvLog::append_bytes(&mut fresh, r)?;
            last = Some(r.id);
        }
        if let Some(id) = last {
            EvLog::set_tip_bytes(&mut fresh, &id)?;
        }
        // Swapped in only once the whole copy succeeded: a partial grow that
        // replaced the store would lose the ledger to save space.
        self.store = fresh;
        self.grew = true;
        Ok(())
    }

    /// The shelf NOW: the newest checkpoint plus the records after it (R7),
    /// equal to `StockLedger::fold(&self.events())` by the checkpoint's law.
    pub fn ledger(&self) -> Result<StockLedger, StockError> {
        self.fold_tail(true, false).map(|f| f.0)
    }

    /// Append one event, AFTER the ledger has agreed to it.
    ///
    /// `decide` runs against the current fold and nothing is written when it
    /// refuses -- the fail-closed gate §4 specifies. An event that would break
    /// an invariant never reaches the log, so a replay of the log can never
    /// reconstruct an impossible state.
    ///
    /// A new write-off or count must be SIGNED ([`signed`]); the history it is
    /// decided against need not be.
    pub fn append(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        self.append_all(std::slice::from_ref(ev))
    }

    /// Append several as ONE decision.
    ///
    /// §4's "one commit, not two": an order's reservations fold atomically. If
    /// the third dish in a basket is out of stock, the first two must not be
    /// reserved -- otherwise a refused order silently holds ingredients that
    /// nothing will ever release.
    pub fn append_all(&mut self, evs: &[StockEvent]) -> Result<(), StockError> {
        // Decided against a ledger that accumulates the batch, so two lines of
        // one order competing for the same ingredient are caught here rather
        // than by the second one failing after the first was written.
        let with: Vec<(StockEvent, meta::Meta)> = evs.iter().map(|e| (e.clone(), meta::Meta::default())).collect();
        self.commit(&with, false).map(|_| ())
    }
}

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

// ── what a dish is made of ──────────────────────────────────────────────────

/// One line of a dish's bill of materials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomLine {
    pub supply: String,
    /// How much ONE portion uses, in the supply's base unit -- WHOLE units;
    /// for a leaf of a semi-finished tree, `uq` rounded to the nearest.
    pub qty: Qty,
    /// The same in MILLIONTHS of the base unit (`prep::MICRO`): exact for a
    /// leaf expanded through a semi-finished card (`prep::for_ledger` writes
    /// `{"supply","uq"}`), `qty × 10^6` for a whole line. What `draws_for`
    /// books and `cost::CostBook::dish_cost` prices.
    pub uq: i64,
}

impl BomLine {
    /// A whole-unit line, as every stored dish writes it.
    pub fn whole(supply: impl Into<String>, qty: Qty) -> BomLine {
        BomLine { supply: supply.into(), qty, uq: qty.saturating_mul(prep_micro()) }
    }
}

/// `prep::MICRO`, named here so this file's readers do not import the tree.
const fn prep_micro() -> i64 {
    crate::prep::MICRO
}

/// Read a product's recipe out of its catalogue record.
///
/// A product with no `bom` is not an error and not a problem: plenty of things
/// a venue sells -- a bottle of water, a dessert bought in -- have no recipe
/// worth tracking, and those simply never reserve anything. Stock control that
/// demands every item be modelled before any item can be sold is stock control
/// nobody switches on.
pub fn bom_of(product_json: &str) -> Vec<BomLine> {
    let mut out = Vec::new();
    // `"bom":[{"supply":"salmon","qty":40}, ...]` -- the array `"bom"` holds,
    // brackets walked outside strings (`modifiers::array_of`, W-AUDIT S7).
    let Some(body) = crate::modifiers::array_of(product_json, "bom") else { return out };
    for chunk in crate::modifiers::split_objects(body) {
        let Some(supply) = crate::minijson::str_field(&chunk, "supply") else { continue };
        if supply.is_empty() {
            continue;
        }
        // A leaf line (`uq`, millionths) or a whole line (`qty`). A line with
        // neither, or a fraction written as `qty`, is not a line.
        if let Some(uq) = crate::minijson::int_field(&chunk, "uq") {
            if uq > 0 {
                out.push(BomLine { supply, qty: (uq + prep_micro() / 2).div_euclid(prep_micro()), uq });
            }
            continue;
        }
        let Some(qty) = crate::minijson::int_field(&chunk, "qty") else { continue };
        if qty > 0 {
            out.push(BomLine::whole(supply, qty));
        }
    }
    out
}

/// `bom_of`, read from the catalogue's `bom` block when one is present (row DG7).
///
/// The block is the catalogue projection (`block::encode::project`), rebuilt
/// with every catalogue generation; a product it does not have -- one written
/// after the block was folded -- falls back to its JSON, which stays the
/// writers' format. Both readers give equal lines (`block::tests::bom_of_block_equals_json`).
pub fn bom_of_product(blocks: Option<&crate::block::view::Catalogue<'_>>, product_id: &str, product_json: &str) -> Vec<BomLine> {
    match blocks.and_then(|c| c.bom_of(product_id)) {
        Some(lines) => lines,
        None => bom_of(product_json),
    }
}

/// The stock events one order's lines imply.
///
/// `lines` is `(product_json, quantity_ordered)`. Quantities MULTIPLY: two
/// portions of a roll using forty grams of salmon reserve eighty, and getting
/// that wrong is how a kitchen runs out mid-service while the ledger says it is
/// fine.
///
/// Lines for the same supply are SUMMED rather than emitted separately, so a
/// basket with two different rolls that both use salmon is checked against the
/// total it actually needs.
pub fn reservations_for(order_id: &str, lines: &[(String, i64)]) -> Vec<StockEvent> {
    let mut totals: Vec<(String, Qty)> = Vec::new();
    for (product_json, qty_ordered) in lines {
        if *qty_ordered <= 0 {
            continue;
        }
        for line in bom_of(product_json) {
            let need = line.qty.saturating_mul(*qty_ordered);
            match totals.iter_mut().find(|(s, _)| *s == line.supply) {
                Some((_, t)) => *t = t.saturating_add(need),
                None => totals.push((line.supply.clone(), need)),
            }
        }
    }
    // Sorted, so the same basket always produces the same event sequence and
    // two hubs replaying it agree byte for byte.
    totals.sort_by(|a, b| a.0.cmp(&b.0));
    totals
        .into_iter()
        .map(|(supply, qty)| StockEvent::Reserved {
            item: supply,
            qty,
            order_id: order_id.to_string(),
        })
        .collect()
}

/// Turn an order's reservations into consumption or release.
///
/// Derived from what the LEDGER is holding for that order rather than
/// recomputed from the basket: if the menu changed between placing and
/// cooking, the recipe may have too, and releasing a different quantity from
/// the one that was reserved is how a reservation gets stranded.
pub fn settle(ledger: &StockLedger, order_id: &str, consume: bool) -> Vec<StockEvent> {
    ledger
        .stranded()
        .into_iter()
        .filter(|(o, _, _)| o == order_id)
        .map(|(_, item, qty)| {
            if consume {
                StockEvent::Consumed { item, qty, order_id: order_id.to_string() }
            } else {
                StockEvent::Released { item, qty, order_id: order_id.to_string() }
            }
        })
        .collect()
}

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
