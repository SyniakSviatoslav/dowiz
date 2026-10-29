//! The journal's slower questions (W-PITR): compaction to a baseline, the newest records for
//! the console, and what a key held just before one edit (the restore action).
use super::*;

/// The journal with everything but its newest `keep` records folded into a baseline: one
/// record per key of the state at the cut (old `None`, `by` [`BASELINE`], the cut record's
/// time), then the kept records verbatim. Replays to the same state at every kept point.
pub fn compacted(log: &LogImage, keep: usize) -> Result<LogImage, JournalError> {
    compacted_to(log, keep, usize::MAX)
}

/// What one record costs in the image beyond its payload: id, prev, actor key and seq (104
/// bytes), the frame and the crc, rounded up to whole cells. An upper bound, not a measure.
pub const RECORD_OVERHEAD: usize = 128;

/// An upper bound on one baseline record of `(k, v)`: the record's JSON carries the key twice
/// and the value escaped (at most doubled), plus [`RECORD_OVERHEAD`] and the fixed fields.
fn baseline_cost(k: &str, v: &str) -> usize {
    RECORD_OVERHEAD + 128 + 2 * k.len() + 2 * v.len()
}

/// The byte budgets of a journal of the menu `after`: `(keep, bound)`. A compaction keeps the
/// newest records up to `keep` = [`KEEP_BYTES`] or the menu's own baseline size, whichever is
/// more (a menu of large dishes keeps as much history as it has menu); it runs past `bound` =
/// [`MAX_BYTES`] or baseline + `keep` + the largest record (the one always kept), whichever is
/// more -- a bound a fresh compaction is always under, so it never runs twice in a row.
/// O(keys), no hashing.
pub fn byte_budget(after: &State) -> (usize, usize) {
    let (mut menu, mut largest) = (0usize, 0usize);
    for (k, v) in after {
        let c = baseline_cost(k, v);
        menu += c;
        largest = largest.max(c);
    }
    let keep = KEEP_BYTES.max(menu);
    (keep, MAX_BYTES.max(menu + keep + largest))
}

/// Is the journal past its byte bound ([`byte_budget`])? The used cells are the store's count.
pub fn over_bytes(log: &LogImage, after: &State) -> bool {
    log.usage().used_cells.max(0) as usize * 8 > byte_budget(after).1
}

/// [`compacted`] keeping at most `keep` records AND at most `keep_bytes` of their payloads,
/// newest first, each counted with its [`RECORD_OVERHEAD`], but never fewer than one (the
/// newest write stays restorable).
pub fn compacted_to(log: &LogImage, keep: usize, keep_bytes: usize) -> Result<LogImage, JournalError> {
    let list = edits(log).map_err(JournalError::Replay)?;
    let mut cut = list.len().saturating_sub(keep);
    let mut held = 0usize;
    for (i, e) in list.iter().enumerate().skip(cut).rev() {
        held = held.saturating_add(RECORD_OVERHEAD + KIND.len() + e.key.len() + record_json(e.at_ms, &e.by, &e.key, &e.old, &e.new, e.gen).len());
        if held > keep_bytes && i + 1 < list.len() {
            cut = i + 1;
            break;
        }
    }
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
/// Decodes only those `n`, so the console's read does not pay for the whole journal: ONE walk
/// that hashes nothing (the load's crc verdicts, `LogImage::recent_checked`) -- a record
/// quarantined anywhere, older than the `n` included, still refuses, as it did.
pub fn recent(log: &LogImage, n: usize) -> Result<Vec<Edit>, ReplayError> {
    let newest = log
        .recent_checked(n)
        .map_err(|q| ReplayError::Unreadable { seq: (log.len() - 1 - q.at) as u64, reason: q.reason })?;
    newest.iter().map(|e| decode(e).map_err(|reason| ReplayError::Unreadable { seq: e.seq, reason })).collect()
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
