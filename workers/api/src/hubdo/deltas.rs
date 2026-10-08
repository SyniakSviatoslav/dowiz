//! THE CATCH-UP WINDOW (moved out of `hubdo.rs` by AX0 to pay for its counter hooks; AX2 owns it
//! next): the recent changes a client that was away is told, or `None` -- "ask for the list".

/// The last few events, so a client that was away can be told what it missed
/// instead of being handed the whole venue again.
///
/// A RING, AND A SMALL ONE. This is not a second copy of the log -- the log is
/// the log -- it is the window in which "what changed since generation N" can
/// be answered cheaply. Past the window the honest answer is "ask for
/// everything", which is also the answer after a hibernation, and a client
/// that hears it re-reads the list. Snapshot-and-delta from twenty years of
/// game netcode: the delta is an optimisation, the snapshot is the truth.
pub(super) const RECENT_KEEP: usize = 256;

/// One event as a catch-up carries it.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Change {
    pub generation: i64,
    pub kind: u8,
    pub order_id: String,
    pub payload: String,
}

/// The changes after `since`, or `None` when this object cannot say.
///
/// `None` is not an error and must not be treated as one: it means the window
/// does not reach back that far -- a cold object, a long absence, a busy hour
/// -- and the caller should read the list instead. Returning an empty slice
/// there would be a lie shaped exactly like "nothing has changed".
pub fn changes_since(recent: &[Change], since: i64) -> Option<Vec<Change>> {
    let oldest = recent.first().map(|c| c.generation)?;
    let newest = recent.last().map(|c| c.generation)?;
    // The client's generation must be one this window covers. `since` equal to
    // the oldest - 1 is the edge that still works: everything after it is here.
    if since + 1 < oldest {
        return None;
    }
    // AND A CLIENT AHEAD OF THIS WINDOW IS NOT UP TO DATE, it is somewhere
    // else. A venue restored from a backup starts its object at generation 1
    // while a console's copy still says 500; answering "nothing changed"
    // would pin that copy forever, because `apply` ignores a generation it is
    // already past. Say the window cannot help and let it read the list.
    if since > newest {
        return None;
    }
    Some(recent.iter().filter(|c| c.generation > since).cloned().collect())
}
