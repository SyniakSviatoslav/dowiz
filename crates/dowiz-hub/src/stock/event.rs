//! WHAT HAPPENED TO THE SHELF: the stock event family, the reasons and stages
//! it carries, and the signer rule a person-caused event answers to.

use super::*;

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
