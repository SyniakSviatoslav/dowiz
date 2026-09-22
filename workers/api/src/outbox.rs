//! THE MESSAGE THAT MUST NOT BE LOST BECAUSE THE ORDER LANDED.
//!
//! THE DEFECT. `storefront::place` awaited `notify::order_placed` INLINE, after
//! the order was already in the log. Telegram refuses, the isolate is cut off
//! at the end of the response, the venue's bot token is briefly wrong — and the
//! kitchen is never told about an order that exists, is paid for, and is
//! sitting in the queue. The order is safe; the only thing that knew about it
//! is gone. Nothing retried, and nothing recorded that anything was missed.
//!
//! An inline await is the wrong shape for this whether it succeeds or not: it
//! puts a third party on the customer's critical path AND loses the message
//! when that third party is down, which is the worst of both.
//!
//! WHAT REPLACES IT. The effect is WRITTEN, in the same turn as the event it
//! belongs to, into the venue's own `outbox` image. A write that landed is a
//! message that will be delivered; a write that did not is an order that was
//! not placed. There is no third state, which is the property the inline await
//! could not have.
//!
//! THE DRAIN IS SEPARATE AND IT RETRIES. An entry carries its attempt count and
//! the instant it may next be tried; a failure reschedules it rather than
//! dropping it, and a persistent failure is REPORTED rather than retried for
//! ever — see `Verdict::Abandon`, and `/api/owner/health`, where the depth and
//! the oldest entry are visible. A queue nobody can see the depth of is a queue
//! that fills up silently.
//!
//! PURE. Every rule below is a function of its arguments; the sending lives in
//! `notify.rs` and the storage in the venue's object.

use serde::{Deserialize, Serialize};

/// The image, and the record kind inside it.
pub const IMAGE_OUTBOX: &str = "outbox";
pub const OUTBOX_BYTES: usize = 1024 * 1024;
pub const KIND: &str = "o";

/// How many times an effect is attempted before it is abandoned.
///
/// SIX, AND THE NUMBER IS THE BACKOFF'S NOT A GUESS: with the schedule below,
/// six attempts span about half an hour, which is longer than any outage this
/// has been observed to have and shorter than the age at which telling a
/// kitchen about an order stops being useful. A seventh attempt at midnight
/// about a lunch order is not a delivery, it is noise.
pub const MAX_TRIES: u32 = 6;

/// Wait this long before attempt number `tries` (1-based).
///
/// EXPONENTIAL, AND IT STARTS SHORT. The common failure is a moment's
/// unavailability, so the first retry is 10 seconds and the customer is still
/// looking at their confirmation. It reaches ~10 minutes by the sixth, which is
/// where a human should be looking instead.
pub fn backoff_ms(tries: u32) -> i64 {
    match tries {
        0 | 1 => 10_000,
        2 => 30_000,
        3 => 120_000,
        4 => 300_000,
        _ => 600_000,
    }
}

/// One outbound effect, waiting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    /// Unique, and it is the ORDER's id plus the kind — so the same event
    /// enqueued twice by a retried command is ONE entry, not two messages to a
    /// kitchen. Idempotency at the queue, not only at the route.
    pub id: String,
    /// What to send. A closed set; an unknown kind is abandoned rather than
    /// guessed at, because guessing means sending a stranger something.
    pub kind: String,
    /// The already-rendered message. RENDERED AT ENQUEUE TIME, deliberately:
    /// the drain runs minutes later and must not re-read a catalogue that has
    /// changed, or a kitchen is told about a dish at the wrong price.
    pub text: String,
    /// Where it goes — a Telegram chat id, today.
    pub to: String,
    pub queued_at_ms: i64,
    pub tries: u32,
    pub next_at_ms: i64,
}

impl Entry {
    pub fn new(id: String, kind: &str, to: String, text: String, now_ms: i64) -> Self {
        Entry {
            id,
            kind: kind.to_string(),
            text,
            to,
            queued_at_ms: now_ms,
            tries: 0,
            next_at_ms: now_ms,
        }
    }
}

/// What to do with an entry after an attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// It went. Remove it.
    Sent,
    /// It did not. Put it back with this many tries and this next instant.
    Retry { tries: u32, next_at_ms: i64 },
    /// It has failed enough. Remove it and SAY SO — an abandoned message that
    /// is silently dropped is the defect this module exists to close, arriving
    /// a second time with a queue in front of it.
    Abandon { after: u32 },
}

/// The rule, after one attempt.
pub fn after_attempt(entry: &Entry, ok: bool, now_ms: i64) -> Verdict {
    if ok {
        return Verdict::Sent;
    }
    let tries = entry.tries + 1;
    if tries >= MAX_TRIES {
        return Verdict::Abandon { after: tries };
    }
    Verdict::Retry { tries, next_at_ms: now_ms + backoff_ms(tries) }
}

/// Which entries may be attempted now, oldest first.
///
/// OLDEST FIRST because a kitchen told about the second order before the first
/// has a queue in the wrong order on its screen, and because an entry that
/// keeps failing must not starve the ones behind it — it is rescheduled into
/// the future, so the order below it becomes the oldest due.
pub fn due(entries: &[Entry], now_ms: i64) -> Vec<&Entry> {
    let mut out: Vec<&Entry> = entries.iter().filter(|e| e.next_at_ms <= now_ms).collect();
    out.sort_by_key(|e| (e.queued_at_ms, e.id.clone()));
    out
}

/// What the health pane says about the queue.
///
/// DEPTH AND AGE, NOT JUST DEPTH. A queue of two that has been two for an hour
/// is a broken integration; a queue of two that is two seconds old is a busy
/// lunchtime, and the same number means both.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Depth {
    pub waiting: usize,
    /// Milliseconds since the oldest waiting entry was queued. 0 when empty.
    pub oldest_ms: i64,
    /// Entries that have failed at least once. A non-zero number here with a
    /// small `oldest_ms` is a rail that is flapping.
    pub failing: usize,
}

pub fn depth(entries: &[Entry], now_ms: i64) -> Depth {
    Depth {
        waiting: entries.len(),
        oldest_ms: entries.iter().map(|e| now_ms - e.queued_at_ms).max().unwrap_or(0).max(0),
        failing: entries.iter().filter(|e| e.tries > 0).count(),
    }
}

/// The storage and the drain: `waiting`, `drain`, and the minute cron's
/// `sweep`. Split out when this file crossed the 300-line ratchet — the rules
/// above are pure and the ones next door touch the images and the rails, which
/// is the same seam `services/mod.rs` describes.
mod rails;
pub use rails::{sweep, waiting};

#[cfg(test)]
mod tests;
