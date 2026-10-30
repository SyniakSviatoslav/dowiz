//! SHEAF S2: RECORDS ADOPTED VERBATIM. The two doors a glued section needs that the write door
//! (`commit`) is not: the hub adopting a kitchen tablet's tail AS WRITTEN, and a tablet replaying
//! the hub's tail AS WRITTEN. Both keep the bytes, so a record's chained id -- `sha256(prev ‖
//! payload)` since M0, the legacy FNV for a record whose text carries no nonce (`codec::nonced`,
//! `chain::id_for`) -- is the same on the hub and on the tablet once the prefix below it is the
//! same: that equality is the whole gluing condition on the wire (`glue::wire`), one 32-byte
//! compare. ONE EXCEPTION, stated: a tablet record that arrives WITHOUT a nonce (written by a
//! tablet from before M0) is adopted with a nonce put in front, because the hub writes no
//! legacy id after its first sha256 one (`chain::walk_check`'s rule); the reply then carries it
//! back as its new bytes rather than as an echo, and the tablet replays those.
//!
//! `commit` re-encodes an event with the meta it is given and stamps the clock; that is right for
//! a new fact and wrong for an old one, whose bytes ARE the fact. So the hub's door here decides
//! each record against the running ledger exactly as `commit` would (`signed`, `decide`,
//! `apply`), writes the bytes it received, and hands back the ones the ledger refused by
//! position and reason -- the caller keeps those for a person (`glue::merge`), it never drops
//! them.

use super::*;

/// What `adopt_from` did with a tail: the positions written and the ones refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Adopted {
    pub merged: usize,
    /// `(position in the tail, oldest = 0, the ledger's reason)`.
    pub held: Vec<(usize, String)>,
}

impl StockLog {
    /// The newest record's id (all zeros on an empty log).
    pub fn tip_id(&self) -> [u8; 32] {
        EvLog::tip(&self.store).unwrap_or([0u8; 32])
    }

    /// Every record ABOVE `base`, OLDEST FIRST, as written; `None` when `base` is not in this
    /// chain. An all-zeros base is the empty log's: every record is above it.
    pub fn since(&self, base: &[u8; 32]) -> Option<Vec<String>> {
        let zero = *base == [0u8; 32];
        let mut found = zero;
        let mut recs: Vec<String> = EvLog::walk_until(&self.store, |r| {
            let hit = !zero && r.id == *base;
            found |= hit;
            hit
        })
        .into_iter()
        .map(|r| String::from_utf8_lossy(&r.payload).into_owned())
        .collect();
        if !found {
            return None;
        }
        if !zero {
            recs.pop();
        }
        recs.reverse();
        Some(recs)
    }

    /// THE TABLET'S DOOR: append `recs` verbatim, then refuse unless the tip is `want`. Nothing
    /// is decided -- these are the hub's records, already decided there -- which is why the tip
    /// must match: a replay that does not land on the hub's own id is not the hub's section,
    /// and the caller drops this copy. Checkpoints are written only when the hub wrote them.
    pub fn replay_raw(&mut self, recs: &[&str], want: &[u8; 32]) -> Result<(), StockError> {
        self.every = usize::MAX;
        for r in recs {
            self.write_payload(r.as_bytes().to_vec())?;
        }
        if self.tip_id() != *want {
            return Err(StockError::Linkage("the replayed tail does not end at the hub's tip".into()));
        }
        Ok(())
    }

    /// Append records verbatim with no decision at all: a tablet's own records carried across
    /// a rebase (decided on the tablet; the hub decides them again at the next glue).
    pub fn carry_raw(&mut self, recs: &[&str]) -> Result<(), StockError> {
        self.every = usize::MAX;
        for r in recs {
            self.write_payload(r.as_bytes().to_vec())?;
        }
        Ok(())
    }

    /// THE NIGHT'S CHECKPOINT (S2): write the checkpoint when one is due -- `CHECKPOINT_EVERY`
    /// records or more since the newest -- and answer whether one was written. The venue's
    /// nightly alarm calls it (`hubdo/timer/night.rs`), so the checkpoint a big glue deferred is
    /// paid in a turn no guest or cook is waiting on; `adopt_from`'s 2x bound stays the net.
    pub fn checkpoint_if_due(&mut self) -> Result<bool, StockError> {
        if self.every == usize::MAX || self.tail(|_, _| true).recs.len() < self.every {
            return Ok(false);
        }
        self.checkpoint_now()?;
        Ok(true)
    }

    /// The shelf now and how many records follow the newest checkpoint: one walk, for a caller
    /// that hands the ledger on (`glue::merge` folds once per turn, S2 profile).
    pub fn ledger_since(&self) -> Result<(StockLedger, usize), StockError> {
        self.fold_tail(true, false).map(|(l, _, _, n)| (l, n))
    }

    /// THE HUB'S DOOR for a glued tail, from the ledger the caller folded (`ledger_since`): each
    /// record decoded, signed-checked and decided against the ledger as it stands after the ones
    /// before it, then written AS RECEIVED. A refusal writes nothing for that record and is
    /// answered by position; the rest go on. Answers the ledger after the adoption too.
    pub fn adopt_from(&mut self, recs: &[String], start: (StockLedger, usize)) -> Result<(Adopted, StockLedger), StockError> {
        let (mut led, since) = start;
        let mut out = Adopted::default();
        for (pos, rec) in recs.iter().enumerate() {
            let Some(ev) = decode(rec) else {
                out.held.push((pos, "not a stock movement".into()));
                continue;
            };
            // `apply` runs `decide` before it moves anything, so a refusal leaves `led` as it
            // was; only an i64 overflow could stop it half-way, and that refuses the batch.
            if let Err(e) = signed(&ev).and_then(|_| led.decide(&ev)) {
                out.held.push((pos, e.to_string()));
                continue;
            }
            led.apply(&ev)?;
            if nonced(rec) {
                self.write_payload(rec.as_bytes().to_vec())?;
            } else {
                self.write_new(rec.clone())?;
            }
            out.merged += 1;
        }
        // THE CHECKPOINT IS DEFERRED, BOUNDED. A glued tail is up to a day of records at once, and
        // the checkpoint's journal fold is ~9 us a record (4.3 ms at 500, S2 profile): writing it
        // in the glue turn is what put the turn over the 10 ms CPU line. So the glue writes one
        // only past TWICE the cadence; below that the next write door writes it when it is due
        // (`commit`: `since + n >= every`), as every 500th write already does. The fold `ledger()`
        // walks meanwhile stays under 2 x CHECKPOINT_EVERY records.
        if out.merged > 0 && self.every != usize::MAX && (since + out.merged >= 2 * self.every || self.grew) {
            self.checkpoint_now()?;
        }
        Ok((out, led))
    }
}

#[cfg(test)]
#[path = "adopt/tests.rs"]
mod tests;
