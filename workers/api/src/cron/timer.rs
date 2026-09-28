//! WHEN A VENUE'S OBJECT NEXT HAS WORK -- the rules its ALARM is set from.
//!
//! PURE. The object (`hubdo/timer.rs`) reads its own images and hands them
//! here; this file says when the next drain, poll or fiscal firing is due, and
//! what to do with the one alarm an object has (`Arm`).
//!
//! WHY AN ALARM AND NOT THE MINUTE CRON (DAG Phase 2, FT2). The minute cron
//! woke every venue's runner 1,440 times a day whether or not anything was
//! queued -- three object requests per venue per minute, measured at 19,130 of
//! the 19,839 object requests of 2026-09-27 -- and ran under the Worker's 10 ms
//! (`crate::cron`'s header has the outage). An alarm is set by the object that
//! HOLDS the work, for the instant the work is due, and an object with nothing
//! due sets none: an idle venue costs nothing.
//!
//! AT-LEAST-ONCE. An alarm is retried after a failure, and may fire twice; a
//! drain run twice sends nothing twice because a sent entry is removed in the
//! write that records it, under the drain's lease (`outbox/lease.rs`). These
//! rules only say WHEN; they never decide whether a message was sent.

use crate::outbox::Entry;

/// The images whose writes can make work due: a write to one of them re-arms.
pub const TIMED: [&str; 3] = [crate::outbox::IMAGE_OUTBOX, crate::ebills::state::IMAGE, crate::fiscal::wire::IMAGE];

/// After a run, the next one waits at least this long. THE MINUTE THE CRON
/// HAD: an entry that stays due (a rail not configured, a chat paced to its
/// fifteen a minute, a drain cut at `SEND_CAP`) is tried again a minute
/// later, as before -- never in a loop of alarms a second apart.
pub const RUN_GAP_MS: i64 = 60_000;

/// External sends one drain may make. The platform allows 50 subrequests per
/// invocation; the drain also reads the venue's object a handful of times, so
/// forty sends leave room, and what is left waits `RUN_GAP_MS`.
pub const SEND_CAP: usize = 40;

/// What to do with the object's alarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arm {
    /// Set it to this instant (it replaces whatever was set).
    Set(i64),
    /// Delete it: nothing is due.
    Clear,
    /// Leave it as it is.
    Keep,
}

/// The earliest instant the drain has something to do in this outbox.
///
/// A PRINT JOB IS NOT THE DRAIN'S: the printer pulls its own ticket
/// (`print_rail.rs`), and the drain skips it. Counted, one ticket waiting for
/// a printer that is switched off would wake the venue every minute for ever.
pub fn outbox_next(entries: &[Entry]) -> Option<i64> {
    entries.iter().filter(|e| e.kind != crate::print_rail::KIND).map(|e| e.next_at_ms).min()
}

/// The earliest instant the till link's poll is due, or `None` when the link
/// is not usable or is halted (nothing unattended fires then; the owner saving
/// the link writes the `ebills` image, which re-arms).
///
/// THE POLL'S OWN CADENCE (`ebills::state::plan`): every minute while open (the
/// floor), and while closed only when the sales list is due -- but never later
/// than the minute the venue opens, found by asking `open_at` minute by minute
/// over the closed wait. A backlog or a re-check pass under way fires a minute
/// on. A backing-off link waits out its backoff.
pub fn ebills_next(
    usable: bool,
    st: &crate::ebills::state::State,
    now_ms: i64,
    open_at: &dyn Fn(i64) -> bool,
) -> Option<i64> {
    use crate::ebills::state::{backoff_ms, MINUTE, SALES_CLOSED_MS};
    if !usable || st.halted {
        return None;
    }
    let busy = st.backlog || (st.recheck_from > 0 && st.recheck_from <= st.recheck_until);
    let base = if busy || open_at(now_ms) {
        now_ms + MINUTE
    } else {
        let sales = (st.last_sales_ms + SALES_CLOSED_MS).clamp(now_ms + MINUTE, now_ms + SALES_CLOSED_MS);
        (1..=SALES_CLOSED_MS / MINUTE)
            .map(|k| now_ms + k * MINUTE)
            .find(|t| *t >= sales || open_at(*t))
            .unwrap_or(sales)
    };
    let backoff_end = match (st.failures, &st.last_error) {
        (0, _) | (_, None) => i64::MIN,
        (n, Some(e)) => e.at_ms + backoff_ms(n),
    };
    Some(base.max(backoff_end))
}

/// Fiscal documents are fired a minute on while any is queued -- and never
/// while sending is switched off (`fiscal::SEND_ENABLED`, operator 2026-09-24).
pub fn fiscal_next(enabled: bool, queued: usize, now_ms: i64) -> Option<i64> {
    (enabled && queued > 0).then_some(now_ms + crate::ebills::state::MINUTE)
}

/// The earliest of the three.
pub fn next_due(parts: &[Option<i64>]) -> Option<i64> {
    parts.iter().flatten().copied().min()
}

/// After a write to a `TIMED` image, outside an alarm run: bring the alarm
/// FORWARD to the work, never push it back, and never clear it (a set alarm
/// that finds nothing due clears itself -- `after_run`).
pub fn on_write(have: Option<i64>, want: Option<i64>, now_ms: i64) -> Arm {
    let Some(w) = want else { return Arm::Keep };
    let at = w.max(now_ms);
    match have {
        Some(h) if h <= at => Arm::Keep,
        _ => Arm::Set(at),
    }
}

/// After an alarm's run: the next due instant, at least `RUN_GAP_MS` on; or
/// nothing at all -- AN IDLE VENUE SCHEDULES NOTHING.
pub fn after_run(want: Option<i64>, now_ms: i64) -> Arm {
    match want {
        Some(w) => Arm::Set(w.max(now_ms + RUN_GAP_MS)),
        None => Arm::Clear,
    }
}

/// THE NIGHTLY SAFETY NET: work is due and no alarm is set -- an alarm lost
/// (its retries spent, a write that failed between the image and the arm) --
/// so set one now. A set alarm is left alone.
pub fn rearm(have: Option<i64>, want: Option<i64>, now_ms: i64) -> Arm {
    match (have, want) {
        (None, Some(w)) => Arm::Set(w.max(now_ms)),
        _ => Arm::Keep,
    }
}

/// The due entries one drain attempts, in the order given: every entry that
/// costs no send (a print job, a routed event the drain fans out), and at most
/// `cap` that do.
pub fn within_budget(due: Vec<&Entry>, cap: usize) -> Vec<&Entry> {
    let mut sends = 0usize;
    due.into_iter()
        .filter(|e| {
            if e.kind == crate::print_rail::KIND || e.kind == crate::notify::route::ROUTE_KIND {
                return true;
            }
            sends += 1;
            sends <= cap
        })
        .collect()
}

/// What the nightly sweep says about one venue's timer (`/fold/timer`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Seen {
    /// An alarm is set.
    Armed,
    /// Work was due and no alarm was set: one was set now. A LOST ALARM.
    Rearmed,
    /// Nothing due, nothing set.
    Idle,
}

impl Seen {
    pub fn of(arm: Arm, have: Option<i64>) -> Seen {
        match (arm, have) {
            (Arm::Set(_), _) => Seen::Rearmed,
            (_, Some(_)) => Seen::Armed,
            _ => Seen::Idle,
        }
    }
}

/// The nightly sweep's count, one line in the log: how many venues were
/// armed, idle, re-armed (each a lost alarm), or could not be asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    pub armed: usize,
    pub idle: usize,
    pub rearmed: Vec<String>,
    pub failed: Vec<String>,
}

impl Tally {
    pub fn add(&mut self, venue: &str, seen: Result<Seen, String>) {
        match seen {
            Ok(Seen::Armed) => self.armed += 1,
            Ok(Seen::Idle) => self.idle += 1,
            Ok(Seen::Rearmed) => self.rearmed.push(venue.to_string()),
            Err(e) => self.failed.push(format!("{venue}: {e}")),
        }
    }

    pub fn line(&self) -> String {
        format!(
            "timers: {} armed, {} idle, {} re-armed (lost alarms), {} unreachable",
            self.armed,
            self.idle,
            self.rearmed.len(),
            self.failed.len()
        )
    }
}

#[cfg(test)]
mod tests;
