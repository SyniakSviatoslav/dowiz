//! THE ORDERS PROJECTION, KEPT CURRENT BY THE EVENTS THAT MOVED IT (R1,
//! `docs/research/2026-09-27-dag-architecture.md` §3.2).
//!
//! WHAT THIS REPLACES. The object's memo was one `(generation, Vec<OrderView>)`
//! dropped whole on every write to the log, so the read after a placement
//! refolded the venue's entire history: MEASURED 120,776 µs natively for
//! `orders_state` over 32k events (W-PERF, `command/place/cost/tests.rs`). The
//! write already knows what it added -- one event for an append, the k newest
//! for a command -- so the projection applies exactly those with `fold_one` and
//! the next read is a copy. O(change), not O(history).
//!
//! THE DIRTY SET IS THE WRITE ITSELF: the same events `broadcast` pushes into the
//! object's `recent` ring (`hubdo.rs`), handed here at the moment the write lands.
//!
//! A STEP IS TAKEN ONLY WHEN IT IS PROVABLY A STEP. Three ways a log write can
//! land, and only two of them may be applied:
//!   * `Written::Appended` -- `append` loaded the image at the memo's generation
//!     and added one record. Applied.
//!   * `Written::Log` -- a command's whole hub. Applied ONLY if every event older
//!     than the new ones is the history this memo folded, checked by a digest,
//!     because `forget` rewrites old records IN PLACE through the same door
//!     (`hubdo/forget.rs` -> `write_both`) and a memo that stepped over a
//!     redaction would keep serving the customer it was asked to forget.
//!   * `Written::Whole` -- a rotation, an import, a Worker's `/image/log` put.
//!     Dropped; the next read refolds.
//!
//! `rebuild` (law 8) is the check on all of it: it refolds the bytes with
//! `hubstore::orders_state`, a fold that shares nothing with this file but
//! `fold_one`, and names every order the two disagree on.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};

use dowiz_hub::room::view::OrderView;
use dowiz_hub::Event;
use serde_json::Value;

/// How a write to the log image landed. See the module header.
pub enum Written {
    Whole,
    Appended(Event),
    /// The hub as written, `Hub::events()` order: NEWEST FIRST.
    Log(Vec<Event>),
}

/// One order in the projection: the position of its newest event and its fold.
struct Slot {
    at: u64,
    state: Value,
}

/// The orders view, current at `generation`.
pub struct Orders {
    generation: i64,
    /// Decoded events folded so far, and their chained digest (oldest first).
    count: usize,
    digest: u64,
    /// Position of the next event; orders render newest-event first.
    next_at: u64,
    slots: HashMap<String, Slot>,
    /// The served rows by position; an order whose fold is null has none.
    rows: BTreeMap<u64, OrderView>,
}

/// One event folded into a running digest. Kind, id, payload and seq: a
/// redaction changes the payload, so it changes every digest after it.
fn chain(digest: u64, e: &Event) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    digest.hash(&mut h);
    (e.kind as u8).hash(&mut h);
    e.order_id.hash(&mut h);
    e.order_json.hash(&mut h);
    e.seq.hash(&mut h);
    h.finish()
}

impl Orders {
    /// The whole fold, from `Hub::events()` (NEWEST FIRST). Rows are rendered
    /// once per order at the end, not once per event.
    pub fn of(generation: i64, newest_first: &[Event]) -> Self {
        let mut o = Orders {
            generation,
            count: 0,
            digest: 0,
            next_at: 0,
            slots: HashMap::new(),
            rows: BTreeMap::new(),
        };
        let mut newest: HashMap<&str, &Event> = HashMap::new();
        for e in newest_first.iter().rev() {
            if o.fold_in(e) {
                newest.insert(e.order_id.as_str(), e);
            }
        }
        for (_, e) in newest {
            o.render(e);
        }
        o
    }

    pub fn generation(&self) -> i64 {
        self.generation
    }

    /// The projection as every reader gets it: newest first.
    pub fn view(&self) -> Vec<OrderView> {
        self.rows.values().rev().cloned().collect()
    }

    /// `(order_id, order_json)` pairs, the shape `rebuild::compare` reads.
    pub fn pairs(&self) -> Vec<(String, String)> {
        self.rows.values().rev().map(|o| (o.order_id.clone(), o.order_json.clone())).collect()
    }

    /// Advance the digest and position; fold an ORDER event into its slot.
    /// True when `e` is an order event (its row must be re-rendered).
    fn fold_in(&mut self, e: &Event) -> bool {
        self.count += 1;
        self.digest = chain(self.digest, e);
        let at = self.next_at;
        self.next_at += 1;
        // NON-ORDER EVENTS ARE SKIPPED, as `orders_state` skips them.
        if !e.kind.is_order() {
            return false;
        }
        let slot = self.slots.entry(e.order_id.clone()).or_insert(Slot { at, state: Value::Null });
        self.rows.remove(&slot.at);
        slot.at = at;
        slot.state = super::fold_one(slot.state.take(), &e.order_json);
        true
    }

    /// Serve `e`'s order again: its newest event's kind and seq, its fold.
    fn render(&mut self, e: &Event) {
        let Some(slot) = self.slots.get(&e.order_id) else { return };
        if slot.state.is_null() {
            return;
        }
        let row = OrderView { order_id: e.order_id.clone(), kind: e.kind as u8, seq: e.seq, order_json: slot.state.to_string() };
        self.rows.insert(slot.at, row);
    }

    /// Apply events written between `from` and `to`, OLDEST FIRST. False, and
    /// nothing applied, when this memo is not at `from`.
    pub fn step(&mut self, from: i64, to: i64, oldest_first: &[Event]) -> bool {
        if self.generation != from {
            return false;
        }
        for e in oldest_first {
            if self.fold_in(e) {
                self.render(e);
            }
        }
        self.generation = to;
        true
    }

    /// Is `older` (NEWEST FIRST) exactly the history this memo folded?
    fn folded_exactly(&self, older: &[Event]) -> bool {
        older.len() == self.count && older.iter().rev().fold(0, chain) == self.digest
    }
}

/// What a log write does to the memo: step it, or drop it. See the header.
pub fn after_log_write(memo: &mut Option<Orders>, from: i64, to: i64, written: Written) {
    let stepped = match (memo.as_mut(), written) {
        (Some(m), Written::Appended(e)) => m.step(from, to, std::slice::from_ref(&e)),
        (Some(m), Written::Log(newest_first)) => {
            let k = newest_first.len().saturating_sub(m.count);
            let (new, older) = newest_first.split_at(k);
            let oldest_first: Vec<Event> = new.iter().rev().cloned().collect();
            m.generation == from && m.folded_exactly(older) && m.step(from, to, &oldest_first)
        }
        _ => false,
    };
    if !stepped {
        *memo = None;
    }
}

/// The projection at `generation`: from the memo when it is there, else
/// refolded from `load()` (the hub's events, NEWEST FIRST) and kept.
pub fn read<E>(
    memo: &mut Option<Orders>,
    generation: i64,
    load: impl FnOnce() -> Result<Vec<Event>, E>,
) -> Result<Vec<OrderView>, E> {
    if let Some(m) = memo.as_ref().filter(|m| m.generation == generation) {
        return Ok(m.view());
    }
    let fresh = Orders::of(generation, &load()?);
    let view = fresh.view();
    *memo = Some(fresh);
    Ok(view)
}

#[cfg(test)]
mod tests;
