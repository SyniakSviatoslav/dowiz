//! ONE DRAIN AT A TIME PER VENUE (W-FIX O4, 2026-09-27).
//!
//! THE DEFECT (W-AUDIT O4). `drain` reads the outbox, awaits Telegram and
//! WhatsApp (the object's input gate is open across every one of those
//! fetches), and only then writes what was sent. A drain slower than a minute
//! -- a rail timing out, fifteen paced messages -- let the next minute's
//! `/fold/cron` read the same image, find the same entries due, and send every
//! one of them again: a kitchen told about one order twice.
//!
//! THE LEASE. Before anything is sent the drain takes a lease in the outbox
//! image itself (one guarded write: two drains cannot both take it), and the
//! write that applies its verdicts gives it back. A drain that finds the lease
//! held sends nothing; a drain that died holding it is outlived by `LEASE_MS`.
//!
//! WHY THE DRAIN AND NOT EACH ENTRY. The drain does not only send due entries:
//! it fans routed events out into NEW per-group entries and reconciles the
//! summaries (`tgrail::route_all`), and it paces a chat to fifteen a minute. A
//! lease per due entry leaves a second drain free to fan the same routed event
//! out again, and delays every paced entry by the lease. One lease covers all
//! of it.
//!
//! THE HOLDER IS NAMED BY ITS EXPIRY, `now + LEASE_MS`, which is distinct per
//! cron minute: a drain gives back only its own lease, never one a later drain
//! took after its own ran out.

use dowiz_hub::table::Table;

/// The record kind and id of the lease, beside the entries in the outbox image.
pub const KIND: &str = "drain";
const ID: &str = "lease";

/// How long a drain may hold the outbox. Longer than any drain (fifteen paced
/// sends and a rail's timeout), short enough that a drain cut off mid-send
/// delays the queue by minutes, not by a day.
pub const LEASE_MS: i64 = 5 * 60 * 1000;

fn until(t: &Table) -> Option<i64> {
    t.get(KIND, ID).and_then(|j| j.parse::<i64>().ok())
}

/// Take the lease at `now_ms`. `Ok(false)`, and nothing written, while another
/// drain holds it.
pub fn take(t: &mut Table, now_ms: i64) -> Result<bool, String> {
    if until(t).is_some_and(|u| now_ms < u) {
        return Ok(false);
    }
    t.put(KIND, ID, &(now_ms + LEASE_MS).to_string(), &[], &[]).map_err(|e| format!("outbox lease: {e:?}"))?;
    Ok(true)
}

/// Give back the lease taken at `now_ms`, and no other.
pub fn give_back(t: &mut Table, now_ms: i64) {
    if until(t) == Some(now_ms + LEASE_MS) {
        t.remove(KIND, ID);
    }
}

#[cfg(test)]
mod tests;
