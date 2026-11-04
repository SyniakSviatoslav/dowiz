//! WHERE A VENUE'S FIRST RUN STARTS: the till's last `HISTORY_DAYS` days,
//! not yesterday.
//!
//! THE DEFECT (operator 2026-11-03: the till has a month of history and the
//! site shows none of it). The walk (`walk.rs`) starts a first run `LEAD_IDS`
//! before the oldest id the yesterday..today list shows, and the watermark
//! only ever moves up from there; the daily re-read (`poll::reread`) hands
//! the week's rows to the import as fingerprints that are never placed. So a
//! link switched on today imported today, and everything before it was
//! unreachable by construction -- measured 2026-11-03 against the real till:
//! 855 listed sales in 30 days (ids 9329..11542), 48 of them in the window a
//! first run read.
//!
//! THE FIX IS ONE REQUEST, ONCE. On the first run (nothing handled, no lead
//! boundary) the oldest sale of the last `HISTORY_DAYS` days is asked for
//! (`Path::Oldest`: the list sorted ascending, one row, measured to answer
//! id 9329 for 2026-10-04..11-03) and handed to the walker as a listed id.
//! Everything the walker already does then does the backfill: it starts
//! `LEAD_IDS` below that id, reads `walk::BUDGET` sales a firing, leaves a
//! `backlog` that re-arms the alarm a minute on (`cron::timer::ebills_next`),
//! and the import keeps a sale ONCE by its uuid (`import::decide`) -- so a
//! second pass over the same month places nothing. Bounded: the window is a
//! constant, the per-firing budget is unchanged, and the request is made
//! only while `first_run`.

use super::client::Path;
use super::fetch::Client;
use super::judge::Fail;
use super::state::Plan;
use super::wire::SaleList;

/// The venue's first firing: no id handled yet, no lead boundary kept.
pub(crate) fn first_run(plan: &Plan) -> bool {
    plan.watermark == 0 && plan.lead_below == 0
}

/// The id the history adds to the window's, or `None`: an empty month, or an
/// oldest id the window already reaches (then the walk starts there anyway).
pub(crate) fn floor_of(listed: &[i64], history: &SaleList) -> Option<i64> {
    let oldest = history.sales.iter().map(|s| s.id).min()?;
    (listed.iter().all(|id| oldest < *id)).then_some(oldest)
}

/// The ids the walker is given: the window's rows and, on a first run, the
/// month's oldest sale -- one request, never repeated once the walk has begun.
pub(crate) async fn ids(c: &mut Client, plan: &Plan, list: &SaleList) -> std::result::Result<Vec<i64>, Fail> {
    let mut ids: Vec<i64> = list.sales.iter().map(|s| s.id).collect();
    if first_run(plan) {
        let path = Path::Oldest { begin: plan.history_from.clone(), end: plan.today.clone(), pos: plan.pos_id };
        let history = super::parse_list(&c.read(&path).await?).map_err(Fail::Shape)?;
        ids.extend(floor_of(&ids, &history));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests;
