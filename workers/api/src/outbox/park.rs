//! PARKED: AN ENTRY WHOSE RAIL IS NOT SET UP WAKES NOTHING (W-EVALFIX, 2026-10-08).
//!
//! THE DEFECT, MEASURED. A rail that is not configured has not failed (`rails.rs`: no bot
//! token, no VAPID key, no WhatsApp or SMS settings), so the drain leaves the entry waiting
//! with no try spent -- correct -- but its `next_at_ms` stayed in the past, `timer::outbox_next`
//! kept answering "due", and `after_run` re-armed the alarm one minute on, for ever. qa-durres
//! held eight such entries for eight days: 1,437 + 13 alarm invocations in 24 h, 48 % of all
//! the platform's Durable Object requests (Cloudflare GraphQL, 2026-10-08), on a day with no
//! order anywhere. Asking again a minute later cannot set a rail up.
//!
//! THE RULE. After the verdicts, every entry the drain was allowed to try (`within_budget`)
//! that got no verdict, no write and no pacing deferral is PARKED: `parked_ms` is stamped (one
//! write, the first time only). `timer::outbox_next` ignores parked entries, so an idle venue
//! schedules nothing again; `due` does not, so ANY wake of the venue -- its nightly, the next
//! order, a settings write (`hubdo/timer.rs::timer_after_write`) -- tries them, and an attempt
//! clears the stamp. The health pane still counts them as waiting.

use super::tgrail::Ops;
use super::{due, Entry, Verdict};
use std::collections::BTreeSet;

/// Stamp the untouched due entries, and clear the stamp on every entry this drain wrote for
/// another reason (attempted, re-aimed, rescheduled): those are not waiting on a rail.
pub fn park(ops: &mut Ops, work: &[Entry], verdicts: &[(String, Verdict)], now_ms: i64) {
    let touched: BTreeSet<&str> = verdicts
        .iter()
        .map(|(id, _)| id.as_str())
        .chain(ops.puts.keys().map(String::as_str))
        .chain(ops.removes.iter().map(String::as_str))
        .chain(ops.deferred.iter().map(String::as_str))
        .collect();
    let tried = crate::cron::timer::within_budget(due(work, now_ms), crate::cron::timer::SEND_CAP);
    let newly: Vec<Entry> = tried
        .into_iter()
        .filter(|e| e.kind != crate::print_rail::KIND && !touched.contains(e.id.as_str()) && e.parked_ms.is_none())
        .map(|e| Entry { parked_ms: Some(now_ms), ..e.clone() })
        .collect();
    for e in ops.puts.values_mut() {
        e.parked_ms = None;
    }
    for e in newly {
        ops.put(e);
    }
}
