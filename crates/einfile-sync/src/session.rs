//! THE SESSION (§3, one MERGE): `I` says where it stands, `R` answers with what `I` lacks, `I` glues
//! and says what `R` lacks, `R` glues and acknowledges.
//!
//! ```text
//! I  FRONTIER ->
//!               <- TAIL      what I lacks
//! I  glues; both wrote: one MERGE
//! I  TAIL     ->             what R lacks, with that MERGE
//!               R glues: everything of R is under I's rows, so R's tail is empty and R owes no MERGE
//!               <- ACK
//! ```
//!
//! THERE ARE NO STATES. Each message is answered from the file and the message alone, so the same
//! rules run for a first meeting, a repeat, a dropped message and a side that wrote meanwhile. Two
//! sides that both say hello write two MERGEs, and then each holds a tail of nothing but MERGEs, which
//! owes none: that is where the storm ends. `tail_into` writes `fresh`, and `store` moves only when
//! the union was judged whole and its MERGE sealed.

#[allow(unused_imports)]
use alloc::{string::ToString, vec::Vec};
use einfile_image::{Store, StoreError};
use einfile_log::doc::{merge_body, Doc, DocError, Front, Ids, Limits, MergeView, Signer, Tail, Trust, Union, Verifier, STATE_COMMUTES, STATE_OBSTRUCTED};
use einfile_log::Record;
use crate::wire::{decode, encode_ack, encode_frontier, encode_need, encode_tail, refusal_name, Msg, Need};
use crate::{Budget, SyncError};

/// The smallest image an adoption starts from; a smaller guess only costs a retry.
const FLOOR: usize = 1 << 20;

/// What a session needs of the device that may have to sign a MERGE. This crate knows no framing and
/// no conflict: `record` builds the device's next record over `body` on the file as it stands in
/// `union`, and `conflicts` names them (GLUING DETECTS, a person resolves). Each may be asked again,
/// on a larger image, so neither may count calls.
pub trait Device {
    fn signer(&self) -> &dyn Signer;
    fn record(&self, union: &Store, body: &[u8]) -> Record;
    fn conflicts(&self, union: &Store, u: &Union) -> Vec<Vec<u8>>;
    fn claimed_ms(&self) -> i64;
}

/// What one adoption did. `wrote` false: the file already held it all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Adopted {
    pub wrote: bool,
    pub records: usize,
    pub merged: bool,
}

/// What to do after one message.
#[derive(Debug, Default)]
pub struct Step {
    /// The next message, if there is one: it goes to the peer as it stands.
    pub send: Option<Vec<u8>>,
    /// This side has nothing left to send, as of this message.
    pub done: bool,
    pub adopted: Adopted,
    /// We refused: the file did not move and `send` tells the peer why.
    pub refused: Option<DocError>,
    /// The peer refused, or wants the whole file.
    pub need: Option<Need>,
}

/// One copy of the file, in a session. Holds nothing a session needs back: put `store` to disk
/// whenever it moves. Feed `on` only from a peer the caller has authenticated: `Front` shows who wrote.
pub struct Peer<'a> {
    pub store: Store,
    pub ids: &'a dyn Ids,
    pub v: &'a dyn Verifier,
    pub roots: &'a [[u8; 32]],
    pub device: &'a dyn Device,
    pub budget: Budget,
}

impl Peer<'_> {
    pub fn front(&self) -> Result<Front, SyncError> {
        Ok(Doc::front(&self.store, self.ids)?)
    }

    /// The first message, and the answer to any break.
    pub fn hello(&self) -> Result<Vec<u8>, SyncError> {
        Ok(encode_frontier(&self.front()?))
    }

    pub(crate) fn limits(&self, f: &Front) -> Limits {
        let head = (f.places.len() + 1) * 104;
        Limits { records: self.budget.records as usize, seals: self.budget.seals as usize, bytes: self.budget.message.saturating_sub(head) / 2 }
    }

    pub(crate) fn tail_for(&self, f: &Front) -> Result<Tail, SyncError> {
        Ok(Doc::tail_part(&self.store, f, self.ids, &self.limits(f))?)
    }

    /// Answer a bytes-in with a bytes-out. `Err` only for bytes that are no message: a refusal of a
    /// sound message goes back to the peer by name, in `Step`.
    pub fn on(&mut self, bytes: &[u8]) -> Result<Step, SyncError> {
        match decode(bytes, &self.budget, self.ids)? {
            Msg::Frontier(f) => self.answer(&f),
            Msg::Ack(f) if f == self.front()? => Ok(Step { done: true, ..Step::default() }),
            Msg::Ack(f) => self.answer(&f),
            Msg::Tail(t) => self.take(&t),
            Msg::Need(n) => Ok(Step { done: true, need: Some(n), ..Step::default() }),
        }
    }

    fn refuse(&self, e: DocError) -> Step {
        let need = match &e {
            DocError::CutOneSide { root } => Need::Archive { seg_root: *root },
            other => Need::Refused { name: refusal_name(other).to_string() },
        };
        Step { send: Some(encode_need(&need)), done: true, refused: Some(e), ..Step::default() }
    }

    fn answer(&self, f: &Front) -> Result<Step, SyncError> {
        let mine = self.front()?;
        if *f == mine {
            return Ok(Step { send: Some(encode_ack(&mine)), done: true, ..Step::default() });
        }
        if f.doc != mine.doc {
            return Ok(self.refuse(DocError::ForeignRoot));
        }
        if f.seg_hash != mine.seg_hash {
            return Ok(self.refuse(DocError::CutOneSide { root: mine.seg_root }));
        }
        match self.tail_for(f) {
            Ok(t) => Ok(Step { send: Some(encode_tail(&t, &self.budget)?), ..Step::default() }),
            Err(SyncError::Doc(e)) => Ok(self.refuse(e)),
            Err(e) => Err(e),
        }
    }

    fn take(&mut self, t: &Tail) -> Result<Step, SyncError> {
        let adopted = match self.adopt(t) {
            Ok(a) => a,
            // Built for a position we never held: say where we stand, and it is built again.
            Err(SyncError::Doc(DocError::Seq { .. })) => return Ok(Step { send: Some(self.hello()?), ..Step::default() }),
            Err(SyncError::Doc(e)) => return Ok(self.refuse(e)),
            Err(e) => return Err(e),
        };
        let mine = self.front()?;
        if mine == t.front {
            return Ok(Step { send: Some(encode_ack(&mine)), done: true, adopted, ..Step::default() });
        }
        match self.tail_for(&t.front) {
            Ok(next) => Ok(Step { send: Some(encode_tail(&next, &self.budget)?), adopted, ..Step::default() }),
            Err(SyncError::Doc(e)) => Ok(Step { adopted, ..self.refuse(e) }),
            Err(e) => Err(e),
        }
    }

    /// `tail` into this file, with its MERGE if one is owed, or nothing at all.
    pub fn adopt(&mut self, tail: &Tail) -> Result<Adopted, SyncError> {
        let need: usize = tail.recs.iter().map(|r| r.payload.len() * 2 + 512).sum::<usize>() + tail.seals.iter().map(|s| s.sig.len() * 2 + 512).sum::<usize>();
        let mut size = self.store.writer_bytes().map_err(DocError::from)?.saturating_add(need).max(FLOOR);
        loop {
            let mut fresh = Store::create_bytes(size).map_err(DocError::from)?;
            match self.glue(tail, &mut fresh) {
                Err(SyncError::Doc(DocError::Store(StoreError::ArenaFull { .. }))) if size < self.budget.image => size = size.saturating_mul(2),
                Err(e) => return Err(e),
                Ok(a) => {
                    if a.wrote {
                        self.store = fresh;
                    }
                    return Ok(a);
                }
            }
        }
    }

    fn glue(&self, tail: &Tail, fresh: &mut Store) -> Result<Adopted, SyncError> {
        let u = Doc::tail_into(&self.store, tail, fresh, self.ids, self.v, Trust::Only(self.roots))?;
        if u.wrote && u.both_wrote {
            let conflicts = self.device.conflicts(fresh, &u);
            let state = if conflicts.is_empty() { STATE_COMMUTES } else { STATE_OBSTRUCTED };
            let body = merge_body(&MergeView { ours: u.ours.clone(), theirs: u.theirs.clone(), state, seg_root: u.seg_root, conflicts });
            let rec = self.device.record(fresh, &body);
            Doc::commit_bytes(fresh, &rec, self.device.signer(), self.device.claimed_ms(), None)?;
        }
        Ok(Adopted { wrote: u.wrote, records: u.their_tail.len(), merged: u.wrote && u.both_wrote })
    }
}

#[cfg(test)]
mod tests;
