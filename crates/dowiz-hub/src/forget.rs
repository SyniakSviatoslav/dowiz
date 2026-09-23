//! FORGETTING A PERSON IN A LOG THAT CANNOT FORGET (§2.2 B, §3.3 of
//! `BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22`).
//!
//! THE REDACTION KEEPS THE LINK. A record of the person has its payload
//! rewritten with the personal fields emptied, and keeps its `id` and `prev`
//! exactly as they were. So the tip does not move, every later record still
//! verifies, last night's witness still `holds()` -- and the record itself no
//! longer matches its own content id. That is the one place the chain stops
//! vouching for content, and it is DECLARED rather than hidden:
//!
//!   * the kind byte gets its high bit (`REDACTED_BIT`), so every reader still
//!     classifies the event by the low seven bits and folds it as before --
//!     the money, the status and the lines are untouched;
//!   * `chain_check` counts such a record as `redacted` when its LINK holds
//!     (the next record's `prev`, or the tip, names it) and as `broken`
//!     otherwise;
//!   * the caller appends one `EventKind::Forgotten` declaration naming how
//!     many records it redacted, and conservation law 9 requires the two
//!     numbers to agree: an undeclared tombstone, or a declaration naming more
//!     than exist, is a breach.
//!
//! WHY NOT A REBUILD (pattern A): it changes every id after the first removed
//! record, and the witness exists to contradict exactly that. WHY NOT
//! CRYPTO-SHREDDING (C): no cipher here, every reader of `contact` would
//! decrypt, and history already in clear needs this anyway. §2.2 has the table.
//!
//! PURE. What counts as the person, and what their record looks like emptied,
//! is the caller's (`redact` returns the new JSON): this crate has no JSON
//! parser (`minijson`'s header), and the Worker has one.

use crate::{content_id_chained, decode, EventKind, Event, Hub, HubError};
use bebop_store::evlog::{EvLog, Record};
use bebop_store::Store;

/// The high bit of the payload's kind byte: this record was redacted in place.
pub(crate) const REDACTED_BIT: u8 = 0x80;

/// Is the record at `at` (newest-first walk) a tombstone whose LINK holds?
/// A tombstone cannot vouch for its content; it vouches for its place.
pub(crate) fn tombstone_holds(walked: &[Record], at: usize, tip: Option<[u8; 32]>) -> bool {
    let r = &walked[at];
    if r.payload.first().map_or(true, |k| k & REDACTED_BIT == 0) {
        return false;
    }
    // A redacted record that still verifies was never changed, and one whose
    // content id matches is not a tombstone at all -- `chain_check` asked that
    // first.
    if content_id_chained(&r.prev, &r.payload) == r.id {
        return false;
    }
    match at {
        0 => tip == Some(r.id),
        n => walked[n - 1].prev == r.id,
    }
}

impl Hub {
    /// Redact every record `redact` answers for, in place. Returns how many.
    ///
    /// `redact` is asked once per readable record that is not already a
    /// tombstone; `Some(json)` is that record's payload with the person taken
    /// out. `id`, `prev`, the actor and the sequence are kept, so the chain's
    /// links and its tip are unchanged. Nothing is written unless something
    /// was redacted.
    pub fn redact<F>(&mut self, redact: F) -> Result<usize, HubError>
    where
        F: Fn(&Event) -> Option<String>,
    {
        let tip = EvLog::tip(&self.store);
        let mut records = EvLog::walk(&self.store);
        records.reverse();
        let mut n = 0usize;
        for r in records.iter_mut() {
            if r.payload.first().map_or(true, |k| k & REDACTED_BIT != 0) {
                continue;
            }
            let Some(ev) = decode(r) else { continue };
            let Some(json) = redact(&ev) else { continue };
            let id = ev.order_id.as_bytes();
            let mut payload = Vec::with_capacity(2 + id.len() + json.len());
            payload.push(ev.kind as u8 | REDACTED_BIT);
            payload.push(id.len() as u8);
            payload.extend_from_slice(id);
            payload.extend_from_slice(json.as_bytes());
            r.payload = payload;
            n += 1;
        }
        if n == 0 {
            return Ok(0);
        }
        // SAME SIZE FIRST, and double on the rare redaction that grew a record.
        let mut size = self.store.to_bytes().len().max(64 * 1024);
        loop {
            match rebuilt(&records, tip, size) {
                Ok(fresh) => {
                    self.store = fresh;
                    return Ok(n);
                }
                Err(HubError::Store(e)) if crate::e_is_full(&e) => size = size.saturating_mul(2),
                Err(e) => return Err(e),
            }
        }
    }

    /// How many redactions the `Forgotten` declarations in THIS image name,
    /// summed over their `records` field. The other side of law 9.
    pub fn declared(&self) -> usize {
        self.declared_where(|_| true)
    }

    /// The same sum, over the declarations filed under one subject
    /// (`cust:<key>`). What a retried erasure compares its tombstones with, so
    /// it declares only what no earlier run of the same erasure declared.
    pub fn declared_for(&self, subject: &str) -> usize {
        self.declared_where(|s| s == subject)
    }

    fn declared_where(&self, subject: impl Fn(&str) -> bool) -> usize {
        self.events()
            .iter()
            .filter(|e| e.kind == EventKind::Forgotten && subject(&e.order_id))
            .filter_map(|e| crate::minijson::int_field(&e.order_json, "records"))
            .map(|n| n.max(0) as usize)
            .sum()
    }

    /// How many records whose order id `of` accepts are tombstones -- redacted
    /// in place, by this run or an earlier one. Counted by the BIT, the same
    /// mark `chain_check` starts from; a tombstone whose link fails is still
    /// counted here and still `broken` there, so the two cannot agree by
    /// accident.
    pub fn tombstones_where(&self, of: impl Fn(&str) -> bool) -> usize {
        EvLog::walk(&self.store)
            .iter()
            .filter(|r| r.payload.first().is_some_and(|k| k & REDACTED_BIT != 0))
            .filter_map(decode)
            .filter(|e| of(&e.order_id))
            .count()
    }
}

/// The same chain, oldest first, in a fresh store of `size` bytes. The ids and
/// `prev` links are copied, never recomputed: that is what "in place" means.
pub(crate) fn rebuilt(records: &[Record], tip: Option<[u8; 32]>, size: usize) -> Result<Store, HubError> {
    let mut fresh = Store::create_bytes(size);
    EvLog::init_bytes(&mut fresh)?;
    for r in records {
        EvLog::append_bytes(&mut fresh, r)?;
    }
    if let Some(t) = tip {
        EvLog::set_tip_bytes(&mut fresh, &t)?;
    }
    Ok(fresh)
}

#[cfg(test)]
mod tests;
