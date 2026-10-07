//! The journal's slower questions (W-PITR): compaction to a baseline, the newest records for
//! the console, and what a key held just before one edit (the restore action).
use super::*;

/// The journal with everything but its newest `keep` records folded into a baseline: one
/// record per key of the state at the cut (old `None`, `by` [`BASELINE`], the cut record's
/// time), then the kept records verbatim. Replays to the same state at every kept point.
pub fn compacted(log: &LogImage, keep: usize) -> Result<LogImage, JournalError> {
    let list = edits(log).map_err(JournalError::Replay)?;
    let cut = list.len().saturating_sub(keep);
    let mut state = State::new();
    apply(&mut state, &list[..cut], Cut::All).map_err(JournalError::Replay)?;
    let at = list[..cut].last().map_or(0, |e| e.at_ms);
    let mut fresh = LogImage::create().map_err(JournalError::Hub)?;
    // The baseline's last record carries the cut's generation only when nothing is kept after it.
    let gen = list[..cut].iter().rev().find_map(|e| e.gen);
    let n = state.len();
    for (i, (k, v)) in state.iter().enumerate() {
        let g = if i + 1 == n && cut == list.len() { gen } else { None };
        fresh.append(KIND, k, &record_json(at, BASELINE, k, &None, &Some(v.clone()), g)).map_err(JournalError::Hub)?;
    }
    for e in &list[cut..] {
        fresh.append(KIND, &e.key, &record_json(e.at_ms, &e.by, &e.key, &e.old, &e.new, e.gen)).map_err(JournalError::Hub)?;
    }
    Ok(fresh)
}

/// The newest `n` records, NEWEST first, for the console. Unreadable = the replay error.
/// Decodes only those `n`, so the console's read does not pay for the whole journal.
pub fn recent(log: &LogImage, n: usize) -> Result<Vec<Edit>, ReplayError> {
    if let Some(q) = log.quarantined().first() {
        return Err(ReplayError::Unreadable { seq: (log.len() - 1 - q.at) as u64, reason: q.reason });
    }
    log.entries().iter().take(n).map(|e| decode(e).map_err(|reason| ReplayError::Unreadable { seq: e.seq, reason })).collect()
}

/// The record at `seq` and what its key held just before it (`None` = did not exist).
pub fn before(log: &LogImage, seq: u64) -> Result<Option<(Edit, Option<String>)>, ReplayError> {
    let list = edits(log)?;
    let Some(e) = list.iter().find(|e| e.seq == seq).cloned() else { return Ok(None) };
    let mut state = State::new();
    apply(&mut state, &list, Cut::Seq(seq))?;
    let was = state.get(&e.key).cloned();
    Ok(Some((e, was)))
}
