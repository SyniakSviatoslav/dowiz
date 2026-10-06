//! ONE ORDER'S HISTORY WITHOUT DECODING THE LOG (W-OCHAIN, R-BEBOPDB D.1 #5).
//!
//! `history()` was `events()` filtered: every record of the log unpacked, crc-checked and
//! decoded into two `String`s, to keep the four that name one order -- 2.1 ms on a
//! 2000-event log (R-BEBOPDB F.3 W4r). The payload framing is `[kind][id_len][id][json]`
//! (`Hub::append`), so "names this order" is a statement about payload bytes `1..2+id_len`,
//! and `EvLog::walk_marked_where` answers it from the header cells alone. Only the matching
//! records are crc-checked, unpacked and decoded.
//!
//! THE ANSWER IS TODAY'S ANSWER, NOT A NEW ONE. `walk_marked_where` is `walk_marked` filtered
//! by the prefix (held record for record in bebop-store), and every record `events()` would
//! keep for this order has exactly that prefix -- `decode` takes `order_id` from those bytes.
//! The decoded `order_id == id && kind.is_order()` test then runs again on the survivors,
//! so the predicate is literally the old one. `tests::scan_equals_the_walk_*` hold it.
//!
//! W-CRC: a record whose crc fails is never returned (`bad.is_none()`), as `events()` never
//! returned it. There is no index and nothing is stored, so there is nothing to go stale
//! after a grow, a rotation or a redaction rebuild.

use bebop_store::evlog::EvLog;

use crate::decode;
use crate::{Event, Hub};

impl Hub {
    /// One order's events, OLDEST FIRST, reading the payload of only that order's records.
    /// `history()` is this; see its doc for the order and the non-order rule.
    pub(crate) fn history_scan(&self, order_id: &str) -> Vec<Event> {
        let idb = order_id.as_bytes();
        // No record can name a longer id: `append` refuses one, and `id_len` is a byte.
        let Ok(len) = u8::try_from(idb.len()) else { return Vec::new() };
        let mut head = Vec::with_capacity(1 + idb.len());
        head.push(len);
        head.extend_from_slice(idb);
        let mut out: Vec<Event> = EvLog::walk_marked_where(&self.store, 1, &head)
            .into_iter()
            .filter_map(|(r, bad)| bad.is_none().then(|| decode(&r)).flatten())
            .filter(|e| e.order_id == order_id && e.kind.is_order())
            .collect();
        out.reverse();
        out
    }
}

#[cfg(test)]
mod tests;
