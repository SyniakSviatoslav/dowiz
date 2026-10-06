//! STORAGES AND TRANSFERS (research 2026-10-03 P12, W-STORE): where on the
//! premises a supply is -- kitchen, bar, freezer, or a storage the owner
//! named -- as ONE MORE FOLD over the same log, never a second ledger.
//!
//! THE SHELF'S TOTAL IS UNTOUCHED. A transfer moves food between rooms of one
//! venue; nothing is bought, eaten or binned, so the ledger, the cost book,
//! the lots and the carry have nothing to fold. That is why a transfer is a
//! NOTE (`notes.rs`, kind [`MOVED`]) and not a `StockEvent`: it can never
//! change a level, a refusal at checkout, a cost or a lot, by construction --
//! and a Worker rolled back past this file reads a log WITH transfers as the
//! same shelf it always was.
//!
//! THE PER-STORAGE LEVEL IS A FOLD OF DELTAS. Each record's change to its
//! item's `on_hand` (after minus before, exactly as the ledger applied it)
//! lands in ONE storage: the record's `"store"` key (`meta.rs`) when it names
//! one, else the storage that LAST RECEIVED the item (Poster's rule -- a
//! sale draws from where the delivery went), else [`DEFAULT`]. A transfer
//! subtracts in `from` and adds in `to`. So the storages ALWAYS sum to the
//! ledger's `on_hand`, item by item -- `tests.rs` holds it after every record.
//!
//! THE OLD-IMAGE RULE. A log with no `"store"` key and no transfer is
//! UNUSED: nothing is kept, and every item's whole level reads as
//! [`DEFAULT`]'s. The state is only written into a checkpoint once used, so
//! an old venue's checkpoints -- and its whole image -- are byte-identical to
//! the ones written before this file (`oldimage_tests.rs`, a golden taken
//! from the code before it). The first record that names a storage first
//! puts every existing level into [`DEFAULT`] and goes on from there: the
//! same answer a fold from genesis gives when every old record maps to the
//! default storage.

use std::collections::BTreeMap;

use super::notes::is_note;
use super::{moved_into, Qty, StockEvent, StockLedger};
use crate::minijson::{esc, int_field, str_field};

/// Where every record written before storages happened.
pub const DEFAULT: &str = "kitchen";
/// The storages every venue starts with, before the owner names any.
pub const DEFAULTS: [&str; 3] = ["kitchen", "bar", "freezer"];
/// Where in-house freezing of raw fish happens (P13).
pub const FREEZER: &str = "freezer";
/// The note kind of a transfer.
pub const MOVED: &str = "moved";
/// The note kind of a storage's card (named, renamed, archived).
pub const STORAGE: &str = "storage";
/// A storage id: 1..=32 of `a-z 0-9 _ -`.
pub const ID_MAX: usize = 32;
pub const NAME_MAX: usize = 40;

/// The per-storage fold.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stores {
    /// Some record named a storage, or moved stock. False: everything is
    /// [`DEFAULT`]'s and nothing below is kept.
    pub(super) used: bool,
    /// `(item, storage)` -> level. Zero levels are not kept.
    pub(super) levels: BTreeMap<(String, String), Qty>,
    /// The storage that last received each item (a delivery, a batch made,
    /// a prep's output, a transfer's `to`).
    pub(super) last: BTreeMap<String, String>,
    /// W-STORE2: station -> the storage it is bound to (`storages/bind.rs`).
    /// Empty for a venue that never bound one: nothing below changes.
    pub(super) bound: BTreeMap<String, String>,
    /// W-STORE2: `(order, item)` -> the storages a bound reservation's later
    /// `consumed` takes from, `(storage, uq)`; "" is the unbound home.
    pub(super) held: BTreeMap<(String, String), Vec<(String, i64)>>,
}

/// A transfer: `qty` of `item` from one storage to another, signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub item: String,
    pub qty: Qty,
    pub from: String,
    pub to: String,
    pub by: String,
}

impl Moved {
    /// The note's body (`StockLog::append_note`).
    pub fn body(&self) -> String {
        format!(
            r#"{{"item":"{}","qty":{},"from":"{}","to":"{}","by":"{}"}}"#,
            esc(&self.item),
            self.qty,
            esc(&self.from),
            esc(&self.to),
            esc(&self.by)
        )
    }

    /// The transfer a raw record is, or `None` for any other record.
    pub fn of(rec: &str) -> Option<Moved> {
        if !is_note(rec, MOVED) {
            return None;
        }
        Some(Moved {
            item: str_field(rec, "item")?,
            qty: int_field(rec, "qty").filter(|q| *q > 0)?,
            from: str_field(rec, "from")?,
            to: str_field(rec, "to")?,
            by: str_field(rec, "by").unwrap_or_default(),
        })
    }
}

/// A storage's card. The newest card of an id is the card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Storage {
    pub id: String,
    pub name: String,
    pub archived: bool,
}

impl Stores {
    /// Whether any record has named a storage yet.
    pub fn is_used(&self) -> bool {
        self.used
    }

    /// Every existing level into [`DEFAULT`], once: the state a fold from
    /// genesis reaches when every record before now maps to the default.
    pub(super) fn materialise(&mut self, led: &StockLedger) {
        if self.used {
            return;
        }
        self.used = true;
        for (item, l) in led.items() {
            if l.on_hand != 0 {
                self.levels.insert((item, DEFAULT.to_string()), l.on_hand);
            }
        }
    }

    pub(super) fn add(&mut self, item: &str, store: &str, d: Qty) {
        if d == 0 {
            return;
        }
        let key = (item.to_string(), store.to_string());
        let v = self.levels.get(&key).copied().unwrap_or(0).saturating_add(d);
        if v == 0 {
            self.levels.remove(&key);
        } else {
            self.levels.insert(key, v);
        }
    }

    /// The storage that last received `item` (Poster's rule), else the default.
    pub fn home(&self, item: &str) -> &str {
        self.last.get(item).map(String::as_str).unwrap_or(DEFAULT)
    }

    /// A first delivery forgives an uncounted item's negative (the ledger
    /// arms it at `max(0, on_hand)`): that `excess` raises the item's
    /// negative storages toward zero. Answers what is left of it.
    fn forgive(&mut self, item: &str, mut excess: Qty) -> Qty {
        let short: Vec<(String, Qty)> =
            self.levels.iter().filter(|((i, _), q)| i == item && **q < 0).map(|((_, s), q)| (s.clone(), *q)).collect();
        for (s, q) in short {
            if excess <= 0 {
                break;
            }
            let up = excess.min(-q);
            self.add(item, &s, up);
            excess -= up;
        }
        excess
    }

    /// Fold one event that the ledger has JUST applied. `before`: the item's
    /// `on_hand` before it; `before_to`: a prep's output's.
    pub(super) fn step(&mut self, ev: &StockEvent, store: Option<&str>, drawn: Option<&str>, before: Qty, before_to: Qty, led: &StockLedger) {
        if !self.used {
            return;
        }
        let item = ev.item().to_string();
        if let StockEvent::Removed { .. } = ev {
            self.levels.retain(|(i, _), _| *i != item);
            self.last.remove(&item);
            self.held.retain(|(_, i), _| *i != item);
            return;
        }
        let d = led.level(&item).on_hand - before;
        // W-STORE2: a draw of a bound station lands in its storage.
        if self.drawn_step(ev, drawn, d) {
            return;
        }
        let here = store.map(str::to_string).unwrap_or_else(|| self.home(&item).to_string());
        match ev {
            StockEvent::Received { qty, .. } | StockEvent::Made { qty, .. } => {
                let rest = self.forgive(&item, d - qty);
                self.add(&item, &here, qty + rest);
                self.last.insert(item, here);
            }
            StockEvent::Produced { into, .. } => {
                self.add(&item, &here, d);
                if let Some(to) = moved_into(&item, into).map(str::to_string) {
                    let dt = led.level(&to).on_hand - before_to;
                    self.add(&to, &here, dt);
                    self.last.insert(to, here);
                }
            }
            _ => self.add(&item, &here, d),
        }
    }

    /// Fold one transfer. Never refused here: the write door decided it.
    pub(super) fn apply_move(&mut self, m: &Moved, led: &StockLedger) {
        self.materialise(led);
        self.add(&m.item, &m.from, -m.qty);
        self.add(&m.item, &m.to, m.qty);
        self.last.insert(m.item.clone(), m.to.clone());
    }

    /// `item`'s level in `store`.
    pub fn level(&self, item: &str, store: &str, led: &StockLedger) -> Qty {
        if self.used {
            self.levels.get(&(item.to_string(), store.to_string())).copied().unwrap_or(0)
        } else if store == DEFAULT {
            led.level(item).on_hand
        } else {
            0
        }
    }

    /// `item`'s non-zero levels, by storage id.
    pub fn of(&self, item: &str, led: &StockLedger) -> Vec<(String, Qty)> {
        if !self.used {
            let q = led.level(item).on_hand;
            return if q == 0 { vec![] } else { vec![(DEFAULT.to_string(), q)] };
        }
        self.levels.iter().filter(|((i, _), _)| i == item).map(|((_, s), q)| (s.clone(), *q)).collect()
    }

    /// Every item with a non-zero level in `store`.
    pub fn in_store(&self, store: &str, led: &StockLedger) -> Vec<(String, Qty)> {
        if !self.used {
            if store != DEFAULT {
                return vec![];
            }
            return led.items().into_iter().filter(|(_, l)| l.on_hand != 0).map(|(i, l)| (i, l.on_hand)).collect();
        }
        self.levels.iter().filter(|((_, s), _)| s == store).map(|((i, _), q)| (i.clone(), *q)).collect()
    }
}

/// A storage id as stored: trimmed, 1..=[`ID_MAX`] of `a-z 0-9 _ -`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= ID_MAX && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// The write doors: a storage's card and a transfer (split out at 300 lines).
#[path = "storages/write.rs"]
mod write;

/// W-STORE2: a station bound to a storage, and the draws that follow it.
#[path = "storages/bind.rs"]
pub mod bind;

#[cfg(test)]
#[path = "storages/tests.rs"]
mod tests;

/// W-STORE2's golden: an unbound venue writes the pre-W-STORE2 bytes.
#[cfg(test)]
#[path = "storages/golden_tests.rs"]
mod golden_tests;
