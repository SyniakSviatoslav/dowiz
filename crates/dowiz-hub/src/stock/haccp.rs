//! RAW-FISH FREEZING AND TRACEABILITY (research 2026-10-03 P13, W-STORE).
//!
//! EU Regulation 853/2004 Annex III Section VIII Chapter III D: fish to be
//! eaten raw is frozen to kill parasites -- at -20 C or below for at least
//! 24 hours, or at -35 C or below for at least 15 hours -- unless the
//! supplier already did it and says so on the paper. The operator gives no
//! data for this (Q8): the OWNER records it, either way, in the console.
//!
//!   * IN-HOUSE: a [`Frozen`] note on the log -- which lot, when, how long,
//!     how cold. [`rule`] says which of the two rules the record meets, or
//!     none. It NEVER blocks a sale: the record is evidence, not a gate.
//!   * SUPPLIER-TREATED: the receipt's `"treated"` key (`meta.rs`), the
//!     paper's number. Old receipts have none and read as "nothing claimed".
//!
//! THE TRACE answers "which orders ate this lot" and "which lots did this
//! order eat" by replaying the log once from genesis with the same `Journal`
//! the screen folds, and reading each draw's effect on the lots (FEFO, or
//! the lot the record named). Like every report it is a fold: nothing is
//! stored that could drift from the log.

use super::journal::Journal;
use super::meta::meta_of;
use super::notes::is_note;
use super::{decode, Qty, StockError, StockEvent, StockLog};
use crate::minijson::{esc, int_field, str_field};

/// The note kind of an in-house freezing record.
pub const FROZEN: &str = "frozen";
/// A freezing record's bounds: hours 1..=2000, temperature -80..=0 C.
pub const HOURS_MAX: i64 = 2000;
pub const TEMP_MIN: i64 = -80;

/// The two rules of 853/2004 Annex III VIII, as words a CSV and a screen show.
pub const RULE_20: &str = "-20C/24h";
pub const RULE_35: &str = "-35C/15h";

/// Which rule `temp_c` for `hours` meets, or `None`. Integers: -19 C for a
/// week meets nothing, -20 C for 23 h meets nothing, -20 C for 24 h meets
/// [`RULE_20`].
pub fn rule(temp_c: i64, hours: i64) -> Option<&'static str> {
    if temp_c <= -20 && hours >= 24 {
        Some(RULE_20)
    } else if temp_c <= -35 && hours >= 15 {
        Some(RULE_35)
    } else {
        None
    }
}

/// An in-house freezing of one lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frozen {
    pub item: String,
    /// The lot code as the Stock screen shows it (a label's, or `#<seq>`).
    pub lot: String,
    /// ms, when it was recorded -- the console records a freezing once it
    /// is done, so this is its end. Stored as `from` (a note's own `at` is the
    /// write clock).
    pub at: i64,
    pub hours: i64,
    pub temp_c: i64,
    /// The storage it was frozen in; the freezer unless the owner said.
    pub store: String,
    pub by: String,
}

impl Frozen {
    fn body(&self) -> String {
        format!(
            r#"{{"item":"{}","lot":"{}","from":{},"hours":{},"temp_c":{},"store":"{}","by":"{}"}}"#,
            esc(&self.item),
            esc(&self.lot),
            self.at,
            self.hours,
            self.temp_c,
            esc(&self.store),
            esc(&self.by)
        )
    }

    /// The record a raw note is, or `None` for any other record.
    pub fn of(rec: &str) -> Option<Frozen> {
        if !is_note(rec, FROZEN) {
            return None;
        }
        Some(Frozen {
            item: str_field(rec, "item")?,
            lot: str_field(rec, "lot")?,
            at: int_field(rec, "from")?,
            hours: int_field(rec, "hours")?,
            temp_c: int_field(rec, "temp_c")?,
            store: str_field(rec, "store").unwrap_or_default(),
            by: str_field(rec, "by").unwrap_or_default(),
        })
    }
}

/// One row of the freezing log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreezeRow {
    /// ms: the freezing record's, or the receipt's time.
    pub at: Option<i64>,
    pub item: String,
    pub lot: String,
    /// `in_house` or `supplier`.
    pub how: &'static str,
    pub hours: Option<i64>,
    pub temp_c: Option<i64>,
    /// The rule an in-house record meets; `None` meets none (or supplier).
    pub rule: Option<&'static str>,
    /// The supplier's paper.
    pub doc: Option<String>,
    pub store: String,
    pub by: String,
}

/// One draw of one order from one lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draw {
    pub order: String,
    pub item: String,
    pub lot: String,
    pub qty: Qty,
    /// ms of the record that drew it; `None` before records were dated.
    pub at: Option<i64>,
}

/// The whole trace: every order's draw per lot, and the freezing log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trace {
    pub draws: Vec<Draw>,
    pub freezing: Vec<FreezeRow>,
}

impl StockLog {
    /// Record an in-house freezing. Refused, nothing written: unsigned, no
    /// item or lot, hours outside 1..=[`HOURS_MAX`], a temperature outside
    /// [`TEMP_MIN`]..=0. NOT refused for missing a rule: the record is what
    /// happened, and [`rule`] says what it meets.
    pub fn record_frozen(&mut self, f: &Frozen) -> Result<(), StockError> {
        if f.by.trim().is_empty() {
            return Err(StockError::Unsigned);
        }
        if f.item.trim().is_empty() || f.lot.trim().is_empty() {
            return Err(StockError::Linkage("a freezing record names the supply and its lot".into()));
        }
        if !(1..=HOURS_MAX).contains(&f.hours) || !(TEMP_MIN..=0).contains(&f.temp_c) {
            return Err(StockError::Linkage(format!("hours are 1 to {HOURS_MAX}, the temperature {TEMP_MIN} to 0 C")));
        }
        self.append_note(FROZEN, &f.body())
    }

    /// The trace, one replay from genesis.
    pub fn trace(&self) -> Result<Trace, StockError> {
        let mut j = Journal::default();
        let mut t = Trace::default();
        for rec in self.raw() {
            if let Some(f) = Frozen::of(&rec) {
                t.freezing.push(FreezeRow {
                    at: Some(f.at),
                    rule: rule(f.temp_c, f.hours),
                    item: f.item,
                    lot: f.lot,
                    how: "in_house",
                    hours: Some(f.hours),
                    temp_c: Some(f.temp_c),
                    doc: None,
                    store: f.store,
                    by: f.by,
                });
                continue;
            }
            let ev = decode(&rec);
            let order = match &ev {
                Some(StockEvent::Consumed { order_id, .. } | StockEvent::Served { order_id, .. }) => Some(order_id.clone()),
                _ => None,
            };
            let item = ev.as_ref().map(|e| e.item().to_string()).unwrap_or_default();
            let left = |j: &Journal| -> Vec<(String, Qty)> { j.lots.lots.iter().filter(|l| l.item == item).map(|l| (l.code.clone(), l.left)).collect() };
            let was = left(&j);
            let seq = j.seen;
            j.step(&rec)?;
            let meta = meta_of(&rec);
            if let (Some(StockEvent::Received { .. }), Some(doc)) = (&ev, meta.treated.clone()) {
                t.freezing.push(FreezeRow {
                    at: meta.at,
                    item: item.clone(),
                    lot: meta.lot.clone().unwrap_or_else(|| format!("#{seq}")),
                    how: "supplier",
                    hours: None,
                    temp_c: None,
                    rule: None,
                    doc: Some(doc),
                    store: meta.store.clone().unwrap_or_default(),
                    by: meta.by.clone().unwrap_or_default(),
                });
            }
            let Some(order) = order else { continue };
            let now = left(&j);
            for (code, before) in was {
                let after = now.iter().find(|(c, _)| *c == code).map_or(0, |x| x.1);
                if before > after {
                    t.draws.push(Draw { order: order.clone(), item: item.clone(), lot: code, qty: before - after, at: meta.at });
                }
            }
        }
        Ok(t)
    }
}

#[cfg(test)]
#[path = "haccp/tests.rs"]
mod tests;
