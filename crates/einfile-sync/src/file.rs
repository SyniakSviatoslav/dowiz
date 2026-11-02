//! `.eintail` (§8): a session with no second party present. The sender packs what the addressee lacks
//! as of the last `Front` it saw of it, keeps that table itself (`peer_seen`, outside the file), and
//! anyone carries the bytes. Not one carrier is a participant: a tail authenticates itself.
//!
//! ```text
//! EINTAIL  "ein-tail/1" ‖ expires_ms LE8 ‖ one TAIL message (`wire.rs`) ‖ sum 32
//!          sum = hash("ein-tail-sum/1" ‖ every byte before it)
//! ```
//!
//! THE SUM IS AGAINST DAMAGE, NOT AGAINST A PERSON. Anchors are unsigned, so without it a flipped
//! byte in a proof would land as a new anchor; a forger recomputes it, and then records and seals meet
//! `tail_into`. `expires_ms` is the sender's word, as `claimed_ms` is, and covers only the sum: a
//! carrier can extend it, and all that buys is a late copy of a true tail. The clock is the caller's.
//!
//! WHAT A FILE CANNOT DO. A stale `peer_seen` still sends plaintext of a record the addressee has
//! since erased: it is refused there (`ForgetOneSide`), after the bytes moved. So is a tail with no
//! encryption: orders do not travel this way until the addressee's key does (§8), and a store tail
//! has no personal data by rule. A tail for a place the addressee never held comes back as `Seq`: send
//! a fresh `Front`.

#[allow(unused_imports)]
use alloc::{vec, vec::Vec};
use bebop_log::doc::{Front, Ids, Tail};
use crate::session::{Adopted, Peer};
use crate::wire::{decode, encode_tail, Msg};
use crate::{Budget, SyncError};

pub const FILE_MARK: &[u8] = b"ein-tail/1";
const SUM_TAG: &[u8] = b"ein-tail-sum/1";
const SUM_BYTES: usize = 32;

/// What reading a file did, and what to carry back.
#[derive(Debug)]
pub struct Carried {
    pub adopted: Adopted,
    /// The sender's position: this is what `peer_seen` keeps.
    pub seen: Front,
    /// The tail the sender lacks, packed, if it lacks one.
    pub reply: Option<Vec<u8>>,
}

fn sum(body: &[u8], ids: &dyn Ids) -> Result<[u8; 32], SyncError> {
    let mut m = Vec::with_capacity(SUM_TAG.len() + body.len());
    m.extend_from_slice(SUM_TAG);
    m.extend_from_slice(body);
    ids.hash(&m).ok_or(SyncError::BadSum)
}

/// The bytes of a `.eintail`.
pub fn pack(t: &Tail, expires_ms: i64, budget: &Budget, ids: &dyn Ids) -> Result<Vec<u8>, SyncError> {
    let msg = encode_tail(t, budget)?;
    let mut b = Vec::with_capacity(FILE_MARK.len() + 8 + msg.len() + SUM_BYTES);
    b.extend_from_slice(FILE_MARK);
    b.extend_from_slice(&expires_ms.to_le_bytes());
    b.extend_from_slice(&msg);
    let s = sum(&b, ids)?;
    b.extend_from_slice(&s);
    Ok(b)
}

/// A tail out of a `.eintail`: the length, the mark, the sum, the time, and only then the parse.
pub fn unpack(bytes: &[u8], now_ms: i64, budget: &Budget, ids: &dyn Ids) -> Result<Tail, SyncError> {
    let room = budget.message.saturating_add(FILE_MARK.len() + 8 + SUM_BYTES);
    if bytes.len() > room {
        return Err(SyncError::Over { what: "file", got: bytes.len() as u64, max: room as u64 });
    }
    let rest = bytes.strip_prefix(FILE_MARK).ok_or(SyncError::Mark("file"))?;
    if rest.len() < 8 + SUM_BYTES {
        return Err(SyncError::Short("file"));
    }
    let (body, tail_sum) = bytes.split_at(bytes.len() - SUM_BYTES);
    if sum(body, ids)? != tail_sum {
        return Err(SyncError::BadSum);
    }
    let (when, msg) = rest.split_at(8);
    let expires_ms = i64::from_le_bytes(when.try_into().map_err(|_| SyncError::Short("file"))?);
    if now_ms > expires_ms {
        return Err(SyncError::Expired { expires_ms, now_ms });
    }
    let msg = msg.get(..msg.len() - SUM_BYTES).ok_or(SyncError::Short("file"))?;
    match decode(msg, budget, ids)? {
        Msg::Tail(t) => Ok(t),
        _ => Err(SyncError::Mark("file")),
    }
}

impl Peer<'_> {
    /// What a file standing at `base` lacks, packed. `base` is the caller's `peer_seen`.
    pub fn write_tail(&self, base: &Front, expires_ms: i64) -> Result<Vec<u8>, SyncError> {
        pack(&self.tail_for(base)?, expires_ms, &self.budget, self.ids)
    }

    /// Adopt a `.eintail`. A reply lives as long as the file it answers was given; a refusal changes
    /// nothing, and `Doc(Seq)` means the sender needs a fresh `Front`.
    pub fn read_tail(&mut self, bytes: &[u8], now_ms: i64, ttl_ms: i64) -> Result<Carried, SyncError> {
        let t = unpack(bytes, now_ms, &self.budget, self.ids)?;
        let adopted = self.adopt(&t)?;
        let reply = if self.front()? == t.front { None } else { Some(self.write_tail(&t.front, now_ms.saturating_add(ttl_ms))?) };
        Ok(Carried { adopted, seen: t.front, reply })
    }
}

#[cfg(test)]
mod tests;
