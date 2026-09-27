//! THE FOLD'S STATE AS A RECORD IN THE CHAIN (research 2026-09-26 §3.7.2,
//! row R7): fold = the newest checkpoint + the records after it.
//!
//! Before this, every placement folded the stock log from its FIRST record --
//! three times when a group hears `stock.low` -- so a venue's order paid for
//! the venue's whole history. A checkpoint is written once
//! [`CHECKPOINT_EVERY`] records follow the last one, and when the image was
//! rewritten (`grow`); `EvLog::walk_until` then stops at it, so the records
//! behind it are never unpacked.
//!
//! WHAT IT HOLDS is everything the journal's fold carries, in one fixed
//! order: levels, open reservations, served-and-not-reversed, the counted
//! set, the cost book (qty, value and last average per supply), every lot,
//! and the journal's counters (rows seen, newest `at`, undatable rows). A
//! checkpoint that forgot any one of them would silently corrupt every fold
//! after it; `tests.rs` holds `fold(all) == fold(checkpoint + tail)` for each.
//!
//! IT IS A CACHE, NOT A SOURCE. It never decodes as a `StockEvent`, so every
//! reader that walks the whole log (`events`, `raw`, `journal`, `rebuild`)
//! skips it and folds from genesis exactly as before; one that does not parse
//! is walked past as if it were not there. [`StockLog::verify_checkpoints`]
//! re-folds from genesis and compares each stored state byte for byte (law 8).
//!
//! THE FORMAT is not JSON (the minijson reader has no arrays): a JSON header
//! line, then space-separated tokens, strings as `<bytes>:<text>` so no name
//! can forge a token, `-` for an absent value. The header's `k` comes first,
//! so `decode` reads `"k":"checkpoint"` and answers `None` whatever an item
//! name inside the body says.

use super::cost::CostBook;
use super::journal::Journal;
use super::meta::{with_meta, Meta};
use super::{decode, encode, signed, EvLog, StockError, StockEvent, StockLedger, StockLog};

/// Records between checkpoints. A fold's cost is ~linear in the tail, so this
/// bounds a placement's fold. MEASURED 2026-09-27 (`tests.rs`
/// `measure_ledger_fold`, release, native aarch64, generated fixture):
/// `ledger()` over 50k records 65,407 us from genesis, 268 us through
/// checkpoints; over 5k, 6,262 us vs 578 us. The image costs +4.7 % at 50k.
pub const CHECKPOINT_EVERY: usize = 500;

/// The records after a checkpoint, oldest first, and that checkpoint's state
/// (a journal with no entries) -- or every record and `None` without one.
pub(super) struct Tail {
    pub base: Option<Journal>,
    pub recs: Vec<String>,
}

impl StockLog {
    /// Change the cadence (a test, a measurement). `usize::MAX` writes none,
    /// not even on a rewrite: the log as every image before R7 was written.
    pub fn set_checkpoint_every(&mut self, n: usize) {
        self.every = n.max(1);
    }

    /// Walk back to the newest checkpoint `accept` takes, given its state and
    /// the newest instant it covers (`None` when the checkpoint has no clock).
    pub(super) fn tail(&self, accept: impl Fn(&Journal, Option<i64>) -> bool) -> Tail {
        let mut base = None;
        let mut recs: Vec<String> = EvLog::walk_until(&self.store, |r| {
            let Some((j, at)) = parse(&r.payload) else { return false };
            let newest = at.map(|a| a.max(j.max_at.unwrap_or(a)));
            let take = accept(&j, newest);
            if take {
                base = Some(j);
            }
            take
        })
        .into_iter()
        .map(|r| String::from_utf8_lossy(&r.payload).into_owned())
        .collect();
        if base.is_some() {
            recs.pop();
        }
        recs.reverse();
        Tail { base, recs }
    }

    /// The shelf and/or the cost book NOW, from the newest checkpoint, and
    /// how many records follow it.
    pub(super) fn fold_tail(&self, ledger: bool, book: bool) -> Result<(StockLedger, CostBook, usize), StockError> {
        let t = self.tail(|_, _| true);
        let (mut led, mut b) = t.base.map(|j| (j.ledger, j.book)).unwrap_or_default();
        for rec in &t.recs {
            let Some(ev) = decode(rec) else { continue };
            if ledger {
                led.apply(&ev)?;
            }
            if book {
                b.apply_event(&ev, rec);
            }
        }
        Ok((led, b, t.recs.len()))
    }

    /// THE WRITE DOOR: signed, decided against the shelf as a batch, written,
    /// then a checkpoint when one is due. Answers the cost book of the SAME
    /// fold (when asked) and the log length it was folded at -- a placement's
    /// cost stamp (R4) without a second walk.
    pub(super) fn commit(&mut self, evs: &[(StockEvent, Meta)], book: bool) -> Result<(CostBook, usize), StockError> {
        for (ev, _) in evs {
            signed(ev)?;
        }
        let at = self.len();
        let (mut trial, costs, since) = self.fold_tail(true, book)?;
        for (ev, _) in evs {
            trial.apply(ev)?;
        }
        for (ev, meta) in evs {
            let m = self.stamped(meta);
            self.write_payload(with_meta(&encode(ev), &m).into_bytes())?;
        }
        if !evs.is_empty() && self.every != usize::MAX && (since + evs.len() >= self.every || self.grew) {
            self.checkpoint_now()?;
        }
        Ok((costs, at))
    }

    /// [`StockLog::append_all`], answering the cost book the shelf was
    /// decided against and the log length it was folded at (R4).
    pub fn append_all_costed(&mut self, evs: &[StockEvent]) -> Result<(CostBook, usize), StockError> {
        let with: Vec<(StockEvent, Meta)> = evs.iter().map(|e| (e.clone(), Meta::default())).collect();
        self.commit(&with, true)
    }

    /// Fold from the newest checkpoint through the tail and append the state.
    pub fn checkpoint_now(&mut self) -> Result<(), StockError> {
        let t = self.tail(|_, _| true);
        let mut j = t.base.unwrap_or_default();
        for rec in &t.recs {
            j.step(rec)?;
        }
        let head = match self.clock {
            Some(at) => format!("{HEAD},\"at\":{at}}}"),
            None => format!("{HEAD}}}"),
        };
        self.grew = false;
        self.write_payload(format!("{head}\n{}", body(&j)).into_bytes())?;
        self.grew = false;
        Ok(())
    }

    /// LAW 8 for checkpoints: fold from genesis and compare every stored state
    /// with the fold's own at its position. Answers how many were checked.
    pub fn verify_checkpoints(&self) -> Result<usize, String> {
        let mut j = Journal::default();
        let mut n = 0;
        for (i, rec) in self.raw().iter().enumerate() {
            if let Some(stored) = rec.strip_prefix(HEAD).and_then(|r| r.split_once('\n')).map(|x| x.1) {
                if stored != body(&j) {
                    return Err(format!("checkpoint at record {i} disagrees with the fold from genesis"));
                }
                n += 1;
                continue;
            }
            j.step(rec).map_err(|e| format!("record {i}: {e}"))?;
        }
        Ok(n)
    }
}

/// The state's text form, and back.
#[path = "checkpoint/codec.rs"]
mod codec;
use codec::{body, parse};
pub(super) use codec::HEAD;

#[cfg(test)]
#[path = "checkpoint/tests.rs"]
mod tests;
