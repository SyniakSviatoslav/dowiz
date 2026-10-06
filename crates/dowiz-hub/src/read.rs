//! THE LOG READ BACK: every event, one order's history, the newest state of
//! every order, the audit trail, the checkpoints -- and the records this build
//! could not read, counted rather than dropped.
//!
//! The law every reader here keeps: `len() == events().len() + quarantined().len()`.

use bebop_store::evlog::EvLog;

use crate::{decode, decode_or_reason, hex32};
use crate::{Event, EventKind, Hub, HubError, Quarantined};

impl Hub {
    /// Every record this build cannot read, newest first.
    ///
    /// THE COUNT IS THE POINT. `events()` has always skipped a record it could
    /// not parse, and a silent skip is a venue losing an order with nothing to
    /// say so — the same mistake as the storage error that was read as "no
    /// image". The law that replaces the silence is arithmetic and is asserted
    /// in the tests: `len() == events().len() + quarantined().len()`. Anything
    /// else means a record went somewhere neither list admits to.
    ///
    /// A non-zero count is a FAILING GATE, not a warning: `/api/owner/health`
    /// carries it, and a quarantine nobody notices is a data-loss feature.
    pub fn quarantined(&self) -> Vec<Quarantined> {
        EvLog::walk_marked(&self.store)
            .iter()
            .enumerate()
            .filter_map(|(at, (r, bad))| {
                // A failed crc first (W-CRC): its payload is not the one that was written.
                let ok = if bad.is_some() { Err("crc") } else { decode_or_reason(r).map(|_| ()) };
                ok.err().map(|reason| Quarantined { id: hex32(&r.id), at, reason })
            })
            .collect()
    }

    /// Every event, newest first. A record that cannot be read is left out and
    /// appears in `quarantined()` instead — never dropped silently.
    pub fn events(&self) -> Vec<Event> {
        EvLog::walk_marked(&self.store)
            .into_iter()
            .filter_map(|(r, bad)| bad.is_none().then(|| decode(&r)).flatten())
            .collect()
    }

    /// One order's events, OLDEST FIRST — the input to a fold.
    ///
    /// `events()` is newest-first, which is right for "what just happened" and
    /// backwards for replaying a history: applied in that order a delta lands
    /// before the state it changes. This is the order a fold needs, and it is
    /// the only order in which the answer is the same for a log of snapshots
    /// and a log of deltas.
    /// NON-ORDER EVENTS ARE LEFT OUT, the same rule `orders()` applies: a
    /// `Revealed` record names who read a customer's details, and its payload
    /// is an audit fact rather than an order. Folded into an order it would add
    /// fields no consumer expects to a record that is served to customers.
    pub fn history(&self, order_id: &str) -> Vec<Event> {
        let mut out: Vec<Event> = self
            .events()
            .into_iter()
            .filter(|e| e.order_id == order_id && e.kind.is_order())
            .collect();
        out.reverse();
        out
    }

    /// Every event, OLDEST FIRST. One pass for a caller that folds them all.
    pub fn events_oldest_first(&self) -> Vec<Event> {
        let mut out = self.events();
        out.reverse();
        out
    }

    /// The payload of an order's most recent EVENT.
    ///
    /// NOT NECESSARILY THE ORDER'S STATE any more, and the distinction is the
    /// whole of phase 3. An event may now be a DELTA -- what changed, not what
    /// is -- so the state is the fold of `history()`, which the Worker does
    /// with a real JSON parser (`fold.rs`; see `minijson`'s header for why not
    /// here). For a log written before deltas the two are the same thing,
    /// which is what makes every existing image read correctly.
    ///
    /// Kept because `events()`/`orders()` return events and a caller that
    /// genuinely wants the last one written should be able to say so.
    pub fn order(&self, order_id: &str) -> Result<String, HubError> {
        self.events()
            .into_iter()
            .find(|e| e.order_id == order_id)
            .map(|e| e.order_json)
            .ok_or(HubError::UnknownOrder)
    }

    /// The newest state of every order, newest order first. One pass, keeping
    /// the first sighting of each id because `events()` is already newest-first.
    ///
    /// NON-ORDER EVENTS ARE SKIPPED, and that is load-bearing rather than
    /// tidy. `Revealed` records an audit fact under a subject that is not an
    /// order id; without this filter it would take a slot in this list and
    /// every fold built on it -- the analytics, the promo use-count, the
    /// dashboard -- would count an audit entry as a sale.
    pub fn orders(&self) -> Vec<Event> {
        // A set, not a scanned list: with a list this was O(events × orders),
        // and it runs on every poll of every console and every tracking sheet.
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out = Vec::new();
        for e in self.events() {
            if !e.kind.is_order() {
                continue;
            }
            if !seen.insert(e.order_id.clone()) {
                continue;
            }
            out.push(e);
        }
        out
    }

    /// The checkpoints this image carries, newest first — what a reader follows
    /// to find the archives.
    pub fn checkpoints(&self) -> Vec<String> {
        self.events()
            .into_iter()
            .filter(|e| e.kind == EventKind::Checkpoint)
            .map(|e| e.order_json)
            .collect()
    }

    /// Every audit event, newest first.
    pub fn reveals(&self) -> Vec<Event> {
        self.events().into_iter().filter(|e| e.kind == EventKind::Revealed).collect()
    }
}

#[cfg(test)]
mod tests;
