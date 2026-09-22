//! REBUILD EVERY PROJECTION FROM THE LOG, AND DIFF IT AGAINST WHAT IS SERVED.
//!
//! THE PROPERTY THIS TURNS AN ARCHITECTURE INTO. The storage here is genuinely
//! event-sourced — an append-only chained log, `fold` is state, projections are
//! memoised per generation — and none of that was CHECKED. A memo that had gone
//! stale, a fold that had changed shape between deploys, or a projection
//! derived once and then mutated in place would all serve wrong answers with
//! nothing disagreeing, because every reader asks the same memo.
//!
//! So: refold from the BYTES, ignoring the memo, and compare. The log is the
//! authority by construction; anything that differs from a fresh fold of it is
//! a defect in the serving path, not in the history.
//!
//! AND THE SECOND IMAGE IS THE ONE THAT ACTUALLY DRIFTS. The class `253e1ece`
//! closed for bookings — an aggregate whose status disagreed with its own
//! history — is still open ACROSS images: the order log says an order is over,
//! and the stock ledger can still be holding its ingredients. Nothing reconciled
//! those two until this. `StockLedger::stranded()` has reported open
//! reservations since it was written and nothing ever called it in production.
//!
//! PURE, so it is exercised by `cargo test` rather than by a deployment. Both
//! arguments are already in the object's memory when this runs.

use serde::{Deserialize, Serialize};
use worker::*;

/// What one rebuild found.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    /// Orders in the fresh fold.
    pub orders: usize,
    /// Orders whose memoised record differs from the fresh fold, by id.
    /// EMPTY IS THE ONLY ACCEPTABLE ANSWER; a name here is a served lie.
    pub stale: Vec<String>,
    /// Orders the LEDGER is still holding ingredients for, which the LOG says
    /// have ended. Each is a kitchen believing it is out of something it has.
    pub stranded: Vec<String>,
    /// Orders the LOG says are live and unprepared, for which the ledger holds
    /// nothing. Usually a venue that models no recipes — see `modelled`.
    pub unheld: Vec<String>,
    /// Does this venue model ingredients at all? When false, `unheld` is the
    /// expected state and means nothing; reporting the count without this
    /// would make every venue without recipes look broken.
    pub modelled: bool,
}

impl Report {
    /// The only question the gate asks.
    pub fn intact(&self) -> bool {
        self.stale.is_empty() && self.stranded.is_empty()
    }
}

/// A status that is over: the order will not be prepared or delivered from
/// here, so nothing may still be held for it.
///
/// DERIVED FROM THE KERNEL, NOT LISTED. `is_terminal` is the FSM's own word for
/// it, and a hand-written list here would be the fourteenth copy of a status
/// set — the defect `a18025d4` and `tools/gates/vocab.sh` exist for.
pub fn is_over(status: &str) -> bool {
    dowiz_kernel::order_machine::OrderStatus::from_str(status)
        .map(|s| s.is_terminal())
        .unwrap_or(false)
}

/// Has this order already consumed what it held? `PREPARING` is where the
/// ingredients stop being a reservation and become food, so from there on the
/// ledger holding nothing is correct rather than missing.
pub fn has_consumed(status: &str) -> bool {
    !matches!(status, "PENDING" | "CONFIRMED" | "SCHEDULED")
}

/// Compare a fresh fold of the log against the memo that is being served, and
/// against what the stock ledger is holding.
///
/// `fresh` and `memo` are `(order_id, order_json)` pairs. `held` is
/// `StockLedger::stranded()` — `(order_id, item, qty)` — which is every open
/// reservation, not only the wrong ones: whether one is WRONG is a question
/// about the order's status, and the ledger does not know statuses.
pub fn compare(
    fresh: &[(String, String)],
    memo: &[(String, String)],
    held: &[(String, String, i64)],
    modelled: bool,
) -> Report {
    let mut stale: Vec<String> = Vec::new();
    for (id, json) in fresh {
        match memo.iter().find(|(m, _)| m == id) {
            // A DIFFERENT PAYLOAD FOR THE SAME ORDER is the serious one: the
            // memo is what every reader gets.
            Some((_, m)) if m != json => stale.push(id.clone()),
            // MISSING FROM THE MEMO is equally a lie, and it is the shape a
            // fold that gained a record would take.
            None => stale.push(id.clone()),
            _ => {}
        }
    }
    // AND THE OTHER DIRECTION: a memo holding an order the log does not.
    for (id, _) in memo {
        if !fresh.iter().any(|(f, _)| f == id) {
            stale.push(id.clone());
        }
    }
    stale.sort();
    stale.dedup();

    let status_of = |id: &str| -> String {
        fresh
            .iter()
            .find(|(f, _)| f == id)
            .and_then(|(_, j)| serde_json::from_str::<serde_json::Value>(j).ok())
            .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(str::to_string))
            .unwrap_or_default()
    };

    let mut stranded: Vec<String> = Vec::new();
    for (order_id, _, _) in held {
        if stranded.contains(order_id) {
            continue;
        }
        let st = status_of(order_id);
        // AN ORDER THE LOG HAS NEVER HEARD OF, still holding stock, is stranded
        // too — and is the worse case, because no screen will ever show it.
        if st.is_empty() || is_over(&st) {
            stranded.push(order_id.clone());
        }
    }
    stranded.sort();

    let mut unheld: Vec<String> = Vec::new();
    if modelled {
        for (id, json) in fresh {
            let st = serde_json::from_str::<serde_json::Value>(json)
                .ok()
                .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(str::to_string))
                .unwrap_or_default();
            if !has_consumed(&st) && !held.iter().any(|(o, _, _)| o == id) {
                unheld.push(id.clone());
            }
        }
        unheld.sort();
    }

    Report { orders: fresh.len(), stale, stranded, unheld, modelled }
}

/// Ask the venue's object to refold and diff. READ-ONLY; see `hubdo::rebuild`
/// for what it compares.
///
/// IT LIVES BESIDE THE RULES IT CARRIES rather than in the storage module,
/// which is also how `hubstore.rs` gave back the lines this added to it.
pub async fn ask(place: &crate::hubstore::Place) -> Result<Report> {
    let stub = place.stub()?;
    let req = Request::new("https://hub/fold/rebuild", Method::Get)?;
    let mut res = stub.fetch_with_request(req).await?;
    if res.status_code() != 200 {
        return Err(Error::RustError(format!("hub refused a rebuild: {}", res.status_code())));
    }
    res.json().await
}

#[cfg(test)]
mod tests;
