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
}

impl WasteReason {
    pub fn as_str(self) -> &'static str {
        match self {
            WasteReason::Spoiled => "spoiled",
            WasteReason::Dropped => "dropped",
            WasteReason::Unsold => "unsold",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "spoiled" => Some(WasteReason::Spoiled),
            "dropped" => Some(WasteReason::Dropped),
            "unsold" => Some(WasteReason::Unsold),
            _ => None,
        }
    }
}

/// §4.1's event family, unchanged.
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
    Wasted { item: String, qty: Qty, reason: WasteReason },
    /// a human counted: on_hand := observed, and the conservation basis resets
    Stocktake { item: String, observed: Qty, stocktake_id: String },
}

impl StockEvent {
    pub fn item(&self) -> &str {
        match self {
            StockEvent::Received { item, .. }
            | StockEvent::Reserved { item, .. }
            | StockEvent::Consumed { item, .. }
            | StockEvent::Released { item, .. }
            | StockEvent::Wasted { item, .. }
            | StockEvent::Stocktake { item, .. } => item,
        }
    }

    pub fn order_id(&self) -> Option<&str> {
        match self {
            StockEvent::Reserved { order_id, .. }
            | StockEvent::Consumed { order_id, .. }
            | StockEvent::Released { order_id, .. } => Some(order_id),
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
#[derive(Debug, Clone, Default)]
pub struct StockLedger {
    levels: Vec<(String, StockLevel)>,
    /// Open reservations, for I3. `(order_id, item)` -> qty.
    open: Vec<((String, String), Qty)>,
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
            _ => {
                let q = match ev {
                    StockEvent::Received { qty, .. }
                    | StockEvent::Reserved { qty, .. }
                    | StockEvent::Consumed { qty, .. }
                    | StockEvent::Released { qty, .. }
                    | StockEvent::Wasted { qty, .. } => *qty,
                    StockEvent::Stocktake { .. } => unreachable!(),
                };
                if q <= 0 {
                    return Err(StockError::NotPositive { qty: q });
                }
            }
        }

        let lvl = self.level(ev.item());
        match ev {
            StockEvent::Received { qty, .. } => {
                lvl.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Reserved { item, qty, .. } => {
                // I1. THE AUTOMATED 86: there is no flag to set and no race to
                // lose, because the refusal is the mechanism.
                if *qty > lvl.available() {
                    return Err(StockError::OutOfStock {
                        item: item.clone(),
                        wanted: *qty,
                        available: lvl.available(),
                    });
                }
            }
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
                if *qty > lvl.on_hand {
                    return Err(StockError::OutOfStock {
                        item: item.clone(),
                        wanted: *qty,
                        available: lvl.on_hand,
                    });
                }
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
                // kitchen bin a portion it has already promised.
                if *qty > lvl.on_hand.saturating_sub(lvl.reserved) {
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
        }
        Ok(())
    }

    fn held(&self, order_id: &str, item: &str) -> Qty {
        self.open
            .iter()
            .find(|((o, i), _)| o == order_id && i == item)
            .map(|(_, q)| *q)
            .unwrap_or(0)
    }

    fn take_held(&mut self, order_id: &str, item: &str, qty: Qty) {
        if let Some(pos) = self
            .open
            .iter()
            .position(|((o, i), _)| o == order_id && i == item)
        {
            self.open[pos].1 -= qty;
            if self.open[pos].1 <= 0 {
                self.open.remove(pos);
            }
        }
    }

    fn apply(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        self.decide(ev)?;
        match ev {
            StockEvent::Received { item, qty } => {
                let l = self.level_mut(item);
                l.on_hand = l.on_hand.checked_add(*qty).ok_or(StockError::Overflow)?;
            }
            StockEvent::Reserved { item, qty, order_id } => {
                let l = self.level_mut(item);
                l.reserved = l.reserved.checked_add(*qty).ok_or(StockError::Overflow)?;
                let key = (order_id.clone(), item.clone());
                match self.open.iter().position(|(k, _)| *k == key) {
                    Some(p) => self.open[p].1 += qty,
                    None => self.open.push((key, *qty)),
                }
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
                let l = self.level_mut(item);
                l.on_hand = *observed;
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
        StockEvent::Wasted { item, qty, reason } => format!(
            r#"{{"k":"wasted","item":"{}","qty":{qty},"reason":"{}"}}"#,
            esc(item),
            reason.as_str()
        ),
        StockEvent::Stocktake { item, observed, stocktake_id } => format!(
            r#"{{"k":"stocktake","item":"{}","observed":{observed},"id":"{}"}}"#,
            esc(item),
            esc(stocktake_id)
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
        }),
        "stocktake" => Some(StockEvent::Stocktake {
            item,
            observed: int_field(rec, "observed")?,
            stocktake_id: str_field(rec, "id")?,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recv(item: &str, q: Qty) -> StockEvent {
        StockEvent::Received { item: item.into(), qty: q }
    }
    fn res(item: &str, q: Qty, o: &str) -> StockEvent {
        StockEvent::Reserved { item: item.into(), qty: q, order_id: o.into() }
    }
    fn con(item: &str, q: Qty, o: &str) -> StockEvent {
        StockEvent::Consumed { item: item.into(), qty: q, order_id: o.into() }
    }
    fn rel(item: &str, q: Qty, o: &str) -> StockEvent {
        StockEvent::Released { item: item.into(), qty: q, order_id: o.into() }
    }

    #[test]
    fn the_happy_path_conserves() {
        let led = StockLedger::fold(&[
            recv("salmon", 1000),
            res("salmon", 200, "ord_1"),
            con("salmon", 200, "ord_1"),
        ])
        .expect("fold");
        assert_eq!(led.level("salmon"), StockLevel { on_hand: 800, reserved: 0 });
        assert_eq!(led.available("salmon"), 800);
        assert!(led.stranded().is_empty());
    }

    /// I1, and the sentence §4 builds the whole design around: the refusal IS
    /// the automatic stop-listing. Nothing sets a flag; there is nothing to race.
    #[test]
    fn reserving_more_than_is_available_is_refused_and_that_is_the_86() {
        let led = StockLedger::fold(&[recv("salmon", 100), res("salmon", 60, "ord_1")]).unwrap();
        assert_eq!(led.available("salmon"), 40);
        match led.decide(&res("salmon", 41, "ord_2")) {
            Err(StockError::OutOfStock { wanted, available, .. }) => {
                assert_eq!((wanted, available), (41, 40));
            }
            other => panic!("expected OutOfStock, got {other:?}"),
        }
        // Exactly what is left is fine; one more is not.
        assert!(led.decide(&res("salmon", 40, "ord_2")).is_ok());
    }

    /// Two orders cannot be promised the same portion. This is the oversell the
    /// design exists to make structurally impossible.
    #[test]
    fn the_same_portion_cannot_be_promised_twice() {
        let led = StockLedger::fold(&[recv("uni", 2), res("uni", 2, "ord_1")]).unwrap();
        assert_eq!(led.available("uni"), 0, "all of it is spoken for");
        assert_eq!(led.level("uni").on_hand, 2, "and it is still on the shelf");
        assert!(led.decide(&res("uni", 1, "ord_2")).is_err());
    }

    /// I1 non-negativity, across every event that subtracts.
    #[test]
    fn nothing_can_drive_a_level_negative() {
        let led = StockLedger::fold(&[recv("rice", 10)]).unwrap();
        assert!(led.decide(&StockEvent::Wasted {
            item: "rice".into(), qty: 11, reason: WasteReason::Spoiled
        }).is_err());
        // And waste cannot eat a reservation: that portion is owed to somebody.
        let led = StockLedger::fold(&[recv("rice", 10), res("rice", 8, "ord_1")]).unwrap();
        assert!(led.decide(&StockEvent::Wasted {
            item: "rice".into(), qty: 3, reason: WasteReason::Dropped
        }).is_err(), "only 2 are unspoken for");
        assert!(led.decide(&StockEvent::Wasted {
            item: "rice".into(), qty: 2, reason: WasteReason::Dropped
        }).is_ok());
    }

    /// §4: "Always > 0".
    #[test]
    fn a_quantity_that_is_not_a_quantity_is_refused() {
        let led = StockLedger::default();
        for q in [0, -1, i64::MIN] {
            assert!(matches!(led.decide(&recv("x", q)), Err(StockError::NotPositive { .. })));
        }
        // An observed count of zero is legitimate: a shelf can be empty.
        assert!(led.decide(&StockEvent::Stocktake {
            item: "x".into(), observed: 0, stocktake_id: "s1".into()
        }).is_ok());
        assert!(led.decide(&StockEvent::Stocktake {
            item: "x".into(), observed: -1, stocktake_id: "s1".into()
        }).is_err());
    }

    /// I3: exactly one of Consumed or Released, never both, never neither.
    #[test]
    fn a_reservation_is_matched_exactly_once() {
        // Released, then consumed: refused.
        let led = StockLedger::fold(&[recv("a", 10), res("a", 4, "o1"), rel("a", 4, "o1")]).unwrap();
        assert!(matches!(led.decide(&con("a", 4, "o1")), Err(StockError::Linkage(_))));
        assert_eq!(led.level("a"), StockLevel { on_hand: 10, reserved: 0 });

        // Consumed, then released: also refused.
        let led = StockLedger::fold(&[recv("a", 10), res("a", 4, "o1"), con("a", 4, "o1")]).unwrap();
        assert!(matches!(led.decide(&rel("a", 4, "o1")), Err(StockError::Linkage(_))));

        // Consuming more than was reserved is refused.
        let led = StockLedger::fold(&[recv("a", 10), res("a", 4, "o1")]).unwrap();
        assert!(matches!(led.decide(&con("a", 5, "o1")), Err(StockError::Linkage(_))));

        // And an order nobody reserved for cannot consume at all.
        assert!(matches!(led.decide(&con("a", 1, "ghost")), Err(StockError::Linkage(_))));
    }

    /// A reservation that is never matched is stock the venue thinks it owes to
    /// an order that ended -- the slow leak that makes a kitchen believe it is
    /// out of something it has.
    #[test]
    fn stranded_reservations_are_visible() {
        let led = StockLedger::fold(&[
            recv("a", 10), res("a", 3, "o1"), res("a", 2, "o2"), rel("a", 2, "o2"),
        ])
        .unwrap();
        assert_eq!(led.stranded(), vec![("o1".to_string(), "a".to_string(), 3)]);
        assert_eq!(led.available("a"), 7);
    }

    /// I2: the basis resets at a count, and reservations survive it.
    #[test]
    fn a_stocktake_resets_the_basis_and_keeps_promises() {
        let led = StockLedger::fold(&[
            recv("tuna", 50),
            res("tuna", 10, "o1"),
            // Somebody counted and there are only 30.
            StockEvent::Stocktake { item: "tuna".into(), observed: 30, stocktake_id: "s1".into() },
        ])
        .unwrap();
        assert_eq!(led.level("tuna"), StockLevel { on_hand: 30, reserved: 10 });
        assert_eq!(led.available("tuna"), 20);
        // The open order can still be fulfilled.
        assert!(led.decide(&con("tuna", 10, "o1")).is_ok());
    }

    /// A count below what is already promised cannot be right, and accepting it
    /// would make the ledger claim it owes more than it has.
    #[test]
    fn a_count_below_what_is_reserved_is_refused() {
        let led = StockLedger::fold(&[recv("tuna", 50), res("tuna", 20, "o1")]).unwrap();
        assert!(led.decide(&StockEvent::Stocktake {
            item: "tuna".into(), observed: 5, stocktake_id: "s1".into()
        }).is_err());
        assert!(led.decide(&StockEvent::Stocktake {
            item: "tuna".into(), observed: 20, stocktake_id: "s1".into()
        }).is_ok());
    }

    /// I4: the same sequence folds to the same projection, and the item order
    /// is stable, so two processes agree byte for byte.
    #[test]
    fn the_fold_is_deterministic() {
        let evs = vec![
            recv("z", 5), recv("a", 3), res("z", 2, "o1"), recv("m", 7), con("z", 2, "o1"),
        ];
        let a = StockLedger::fold(&evs).unwrap();
        let b = StockLedger::fold(&evs).unwrap();
        assert_eq!(a.items(), b.items());
        assert_eq!(
            a.items().iter().map(|(i, _)| i.as_str()).collect::<Vec<_>>(),
            vec!["a", "m", "z"],
            "items come out sorted, so two folds serialise identically"
        );
    }

    /// The events survive the round trip that a restart is.
    #[test]
    fn every_event_round_trips() {
        let evs = vec![
            recv("salmon", 100),
            res("salmon", 5, "ord_1"),
            con("salmon", 5, "ord_1"),
            rel("rice", 2, "ord_2"),
            StockEvent::Wasted { item: "rice".into(), qty: 1, reason: WasteReason::Spoiled },
            StockEvent::Stocktake { item: "nori".into(), observed: 42, stocktake_id: "s1".into() },
        ];
        for ev in &evs {
            assert_eq!(decode(&encode(ev)).as_ref(), Some(ev), "{ev:?}");
        }
        assert_eq!(decode("not json"), None);
        assert_eq!(decode(r#"{"k":"nonsense","item":"x"}"#), None);
    }

    /// An item name with a quote cannot forge a second field.
    #[test]
    fn a_hostile_item_name_cannot_forge_a_record() {
        let ev = recv(r#"x","qty":9999"#, 1);
        let back = decode(&encode(&ev)).expect("decode");
        match back {
            StockEvent::Received { qty, .. } => assert_eq!(qty, 1, "the quantity must not move"),
            other => panic!("{other:?}"),
        }
    }

    /// Checked arithmetic, not wrapping. A receipt that would overflow is
    /// refused rather than turning a full shelf into a negative one.
    #[test]
    fn overflow_is_refused_not_wrapped() {
        let led = StockLedger::fold(&[recv("x", i64::MAX)]).unwrap();
        assert!(matches!(led.decide(&recv("x", 1)), Err(StockError::Overflow)));
    }
}

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
}

/// Room for roughly a year of a single venue's stock events.
pub const DEFAULT_STOCK_BYTES: usize = 8 * 1024 * 1024;

impl StockLog {
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
        let mut store = Store::create_bytes(bytes);
        EvLog::init_bytes(&mut store)?;
        Ok(StockLog { store })
    }

    pub fn load(bytes: &[u8]) -> Result<Self, crate::HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(crate::HubError::NotAHub);
        }
        Ok(StockLog { store })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.store.to_bytes()
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
    /// which is I4's other half.
    fn write(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        let payload = encode(ev).into_bytes();
        let id = crate::content_id(&payload);
        let prev = EvLog::tip(&self.store).unwrap_or([0u8; 32]);
        let rec = Record {
            id,
            prev,
            // Stock events are the venue's own, recorded by the hub rather than
            // signed by a person; the actor slot is zero rather than borrowing
            // an identity that did not act.
            actor_pubkey: [0u8; 32],
            actor_seq: self.events().len() as u64,
            payload,
        };
        // THE IMAGE GROWS RATHER THAN REFUSING, like the order log. A shelf
        // that cannot record a delivery because its arena is full is a kitchen
        // that stops being able to sell -- the refusal path reads this ledger.
        // ── APPEND AND TIP FAIL DIFFERENTLY, SO THEY ARE HANDLED DIFFERENTLY ──
        //
        // Measured: on a nearly full arena the RECORD still fits while the tip
        // update does not. And `walk` follows the store's object chain rather
        // than the tip hash, so a record written without its tip is already IN
        // the chain -- re-appending it after growing counts it twice, which is
        // exactly what the first version of this did (3002 deliveries recorded
        // for 3000 made).
        //
        // So: grow-and-retry only the APPEND. Once the record is in, the tip is
        // set; if that is what ran out of room, growing is enough on its own,
        // because `grow` copies the chain and points the tip at its last
        // record -- which is this one.
        let mut placed = EvLog::append_bytes(&mut self.store, &rec).is_ok();
        for _ in 0..6 {
            if placed {
                break;
            }
            self.grow().map_err(|_| StockError::Malformed)?;
            placed = EvLog::append_bytes(&mut self.store, &rec).is_ok();
        }
        if !placed {
            return Err(StockError::Malformed);
        }
        if EvLog::set_tip_bytes(&mut self.store, &id).is_err() {
            self.grow().map_err(|_| StockError::Malformed)?;
            EvLog::set_tip_bytes(&mut self.store, &id).map_err(|_| StockError::Malformed)?;
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
        let mut fresh = Store::create_bytes(bigger);
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
        Ok(())
    }

    pub fn ledger(&self) -> Result<StockLedger, StockError> {
        StockLedger::fold(&self.events())
    }

    /// Append one event, AFTER the ledger has agreed to it.
    ///
    /// `decide` runs against the current fold and nothing is written when it
    /// refuses -- the fail-closed gate §4 specifies. An event that would break
    /// an invariant never reaches the log, so a replay of the log can never
    /// reconstruct an impossible state.
    pub fn append(&mut self, ev: &StockEvent) -> Result<(), StockError> {
        self.ledger()?.decide(ev)?;
        self.write(ev)
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
        let mut trial = self.ledger()?;
        for ev in evs {
            trial.apply(ev)?;
        }
        for ev in evs {
            self.write(ev)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod log_tests {
    use super::*;

    #[test]
    fn the_log_survives_the_byte_image() {
        let mut log = StockLog::create().expect("create");
        log.append(&StockEvent::Received { item: "salmon".into(), qty: 500 }).unwrap();
        log.append(&StockEvent::Reserved {
            item: "salmon".into(), qty: 50, order_id: "o1".into()
        }).unwrap();
        let bytes = log.to_bytes();

        let log = StockLog::load(&bytes).expect("load");
        assert_eq!(log.events().len(), 2);
        let led = log.ledger().unwrap();
        assert_eq!(led.level("salmon"), StockLevel { on_hand: 500, reserved: 50 });
        assert_eq!(led.available("salmon"), 450);
    }

    /// THE ORDER OF A FOLD IS NOT NEGOTIABLE. `EvLog::walk` returns newest
    /// first; applying that order makes a reservation land before the delivery
    /// that made it possible, and the ledger refuses its own history. This is
    /// the regression test for exactly that -- it failed with
    /// `OutOfStock { wanted: 50, available: 0 }` on a log holding 500.
    #[test]
    fn a_reloaded_log_replays_in_the_order_it_happened() {
        let mut log = StockLog::create().expect("create");
        for ev in [
            StockEvent::Received { item: "salmon".into(), qty: 500 },
            StockEvent::Reserved { item: "salmon".into(), qty: 50, order_id: "o1".into() },
            StockEvent::Consumed { item: "salmon".into(), qty: 50, order_id: "o1".into() },
            StockEvent::Received { item: "salmon".into(), qty: 100 },
        ] {
            log.append(&ev).expect("append");
        }
        let replayed = StockLog::load(&log.to_bytes()).expect("load");
        // Oldest first, as it happened.
        assert!(matches!(replayed.events()[0], StockEvent::Received { qty: 500, .. }));
        assert_eq!(replayed.ledger().unwrap().level("salmon"),
                   StockLevel { on_hand: 550, reserved: 0 });
    }

    /// Nothing is written when the gate refuses, so a replay can never
    /// reconstruct an impossible state.
    #[test]
    fn a_refused_event_does_not_reach_the_log() {
        let mut log = StockLog::create().expect("create");
        log.append(&StockEvent::Received { item: "uni".into(), qty: 2 }).unwrap();
        assert!(log.append(&StockEvent::Reserved {
            item: "uni".into(), qty: 3, order_id: "o1".into()
        }).is_err());
        assert_eq!(log.events().len(), 1, "the refusal wrote nothing");
    }

    /// §4's "one commit, not two". A basket whose third line is short must
    /// reserve NOTHING -- otherwise the first two are held by an order that was
    /// never placed, and nothing will ever release them.
    #[test]
    fn a_batch_is_all_or_nothing() {
        let mut log = StockLog::create().expect("create");
        log.append(&StockEvent::Received { item: "rice".into(), qty: 100 }).unwrap();
        log.append(&StockEvent::Received { item: "nori".into(), qty: 100 }).unwrap();
        log.append(&StockEvent::Received { item: "uni".into(), qty: 1 }).unwrap();
        let before = log.events().len();

        let batch = vec![
            StockEvent::Reserved { item: "rice".into(), qty: 10, order_id: "o1".into() },
            StockEvent::Reserved { item: "nori".into(), qty: 2, order_id: "o1".into() },
            StockEvent::Reserved { item: "uni".into(), qty: 5, order_id: "o1".into() },
        ];
        assert!(log.append_all(&batch).is_err(), "the third line is short");
        assert_eq!(log.events().len(), before, "and so NOTHING was reserved");
        assert_eq!(log.ledger().unwrap().level("rice").reserved, 0);
    }

    /// Two lines of ONE order competing for the same ingredient are caught by
    /// the batch, not by the second one failing after the first was written.
    #[test]
    fn two_lines_of_one_order_are_decided_together() {
        let mut log = StockLog::create().expect("create");
        log.append(&StockEvent::Received { item: "uni".into(), qty: 3 }).unwrap();
        let batch = vec![
            StockEvent::Reserved { item: "uni".into(), qty: 2, order_id: "o1".into() },
            StockEvent::Reserved { item: "uni".into(), qty: 2, order_id: "o1".into() },
        ];
        assert!(log.append_all(&batch).is_err(), "4 wanted, 3 on the shelf");
        assert_eq!(log.ledger().unwrap().level("uni").reserved, 0);
    }
}

// ── what a dish is made of ──────────────────────────────────────────────────

/// One line of a dish's bill of materials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomLine {
    pub supply: String,
    /// How much ONE portion uses, in the supply's base unit.
    pub qty: Qty,
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
    // `"bom":[{"supply":"salmon","qty":40}, ...]`
    let Some(start) = product_json.find("\"bom\"") else { return out };
    let rest = &product_json[start..];
    let Some(open) = rest.find('[') else { return out };
    let Some(close) = rest[open..].find(']') else { return out };
    for chunk in rest[open..open + close].split('{').skip(1) {
        let Some(supply) = crate::minijson::str_field(chunk, "supply") else { continue };
        let Some(qty) = crate::minijson::int_field(chunk, "qty") else { continue };
        if qty > 0 && !supply.is_empty() {
            out.push(BomLine { supply, qty });
        }
    }
    out
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
mod bom_tests {
    use super::*;

    const ROLL: &str = r#"{"id":"p1","name":"Sake","bom":[{"supply":"salmon","qty":40},{"supply":"rice","qty":90}]}"#;
    const MAKI: &str = r#"{"id":"p2","name":"Ebi","bom":[{"supply":"rice","qty":60},{"supply":"prawn","qty":30}]}"#;
    const WATER: &str = r#"{"id":"p3","name":"Water","price":100}"#;

    #[test]
    fn a_recipe_reads_back() {
        assert_eq!(
            bom_of(ROLL),
            vec![
                BomLine { supply: "salmon".into(), qty: 40 },
                BomLine { supply: "rice".into(), qty: 90 },
            ]
        );
    }

    /// A dish with no recipe reserves nothing, and that is a normal venue --
    /// a bought-in bottle of water has no bill of materials worth keeping.
    #[test]
    fn a_dish_with_no_recipe_is_not_an_error() {
        assert!(bom_of(WATER).is_empty());
        assert!(reservations_for("o1", &[(WATER.into(), 3)]).is_empty());
    }

    /// Quantities multiply. Getting this wrong is how a kitchen runs out
    /// mid-service while the ledger says it is fine.
    #[test]
    fn quantities_multiply_by_the_portions_ordered() {
        let evs = reservations_for("o1", &[(ROLL.into(), 2)]);
        let salmon = evs.iter().find(|e| e.item() == "salmon").expect("salmon");
        match salmon {
            StockEvent::Reserved { qty, .. } => assert_eq!(*qty, 80),
            other => panic!("{other:?}"),
        }
    }

    /// Two different dishes sharing an ingredient are checked against the
    /// TOTAL they need, not one line at a time.
    #[test]
    fn a_shared_ingredient_is_summed_across_the_basket() {
        let evs = reservations_for("o1", &[(ROLL.into(), 1), (MAKI.into(), 2)]);
        let rice = evs.iter().find(|e| e.item() == "rice").expect("rice");
        match rice {
            // 90 for one roll + 60x2 for two maki
            StockEvent::Reserved { qty, .. } => assert_eq!(*qty, 210),
            other => panic!("{other:?}"),
        }
        assert_eq!(evs.len(), 3, "salmon, rice, prawn — one event each");
        // Sorted, so the same basket always produces the same sequence.
        let names: Vec<&str> = evs.iter().map(|e| e.item()).collect();
        assert_eq!(names, vec!["prawn", "rice", "salmon"]);
    }

    /// The whole reason to route a basket through the ledger.
    #[test]
    fn a_basket_that_exceeds_the_shelf_reserves_nothing() {
        let mut log = StockLog::create().expect("create");
        log.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
        log.append(&StockEvent::Received { item: "rice".into(), qty: 1000 }).unwrap();

        // Three portions need 120g of salmon and there are 100.
        let evs = reservations_for("o1", &[(ROLL.into(), 3)]);
        assert!(log.append_all(&evs).is_err());
        assert_eq!(log.ledger().unwrap().level("rice").reserved, 0, "rice was not held either");

        // Two portions fit.
        let evs = reservations_for("o2", &[(ROLL.into(), 2)]);
        assert!(log.append_all(&evs).is_ok());
        assert_eq!(log.ledger().unwrap().available("salmon"), 20);
    }

    /// Settlement comes from what the LEDGER holds, not from the basket: if the
    /// recipe changed between placing and cooking, releasing a recomputed
    /// quantity would strand the difference forever.
    #[test]
    fn settlement_releases_exactly_what_was_reserved() {
        let mut log = StockLog::create().expect("create");
        log.append(&StockEvent::Received { item: "salmon".into(), qty: 200 }).unwrap();
        log.append(&StockEvent::Received { item: "rice".into(), qty: 500 }).unwrap();
        log.append_all(&reservations_for("o1", &[(ROLL.into(), 1)])).unwrap();

        let led = log.ledger().unwrap();
        let release = settle(&led, "o1", false);
        assert_eq!(release.len(), 2);
        log.append_all(&release).unwrap();

        let led = log.ledger().unwrap();
        assert!(led.stranded().is_empty(), "nothing left held");
        assert_eq!(led.level("salmon"), StockLevel { on_hand: 200, reserved: 0 });

        // And consuming instead takes it off the shelf.
        log.append_all(&reservations_for("o2", &[(ROLL.into(), 1)])).unwrap();
        let led = log.ledger().unwrap();
        log.append_all(&settle(&led, "o2", true)).unwrap();
        assert_eq!(log.ledger().unwrap().level("salmon"), StockLevel { on_hand: 160, reserved: 0 });
    }

    /// Settling one order must not touch another's reservations.
    #[test]
    fn settlement_is_scoped_to_its_own_order() {
        let mut log = StockLog::create().expect("create");
        log.append(&StockEvent::Received { item: "salmon".into(), qty: 500 }).unwrap();
        log.append(&StockEvent::Received { item: "rice".into(), qty: 500 }).unwrap();
        log.append_all(&reservations_for("o1", &[(ROLL.into(), 1)])).unwrap();
        log.append_all(&reservations_for("o2", &[(ROLL.into(), 1)])).unwrap();

        let led = log.ledger().unwrap();
        log.append_all(&settle(&led, "o1", false)).unwrap();
        let led = log.ledger().unwrap();
        assert_eq!(led.stranded().len(), 2, "o2 still holds its two lines");
        assert!(led.stranded().iter().all(|(o, _, _)| o == "o2"));
    }

    #[test]
    fn a_malformed_recipe_is_ignored_rather_than_fatal() {
        for junk in [
            r#"{"id":"p","bom":"not an array"}"#,
            r#"{"id":"p","bom":[]}"#,
            r#"{"id":"p","bom":[{"supply":"","qty":5}]}"#,
            r#"{"id":"p","bom":[{"supply":"x","qty":0}]}"#,
            r#"{"id":"p","bom":[{"supply":"x","qty":-3}]}"#,
            r#"{"id":"p"}"#,
        ] {
            assert!(bom_of(junk).is_empty(), "accepted {junk}");
        }
    }
}
