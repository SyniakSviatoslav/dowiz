//! FRAMES: any message over any carrier that delivers bytes in order, in pieces of any size. One
//! decoder serves both shapes: a data channel hands it one frame at a time, a stream hands it
//! whatever arrived, and a frame cut across two reads waits for the rest.
//!
//! ```text
//! FRAME  len LE4 ‖ total LE8 ‖ offset LE8 ‖ sum 32 ‖ payload[len]     sum = hash("ein-frame/1" ‖ message)
//! ```
//!
//! RESUMING. `sum` names the message and `offset` says where a frame belongs, so a break costs what
//! was not delivered: read `Assembler::resume_at`, and send `frames(.., from)` from there. A frame
//! already held is dropped, a frame past a hole is dropped too, and `resume_at` still names the hole.
//! A message with another sum that does not begin at offset 0 is dropped the same way, and one that
//! does replaces the partial message: the session has moved on and the old one means nothing.
//!
//! BOUNDED. `total` is checked against the budget before a byte is kept, and storage grows only as
//! bytes arrive. The sum is checked on the last byte, so a damaged message never reaches `decode`.
//! A relay that carries sealed bytes needs none of this to mean anything to it.

#[allow(unused_imports)]
use alloc::{vec, vec::Vec};
use bebop_log::doc::Ids;
use crate::SyncError;

const TAG: &[u8] = b"ein-frame/1";
/// The bytes in front of each payload.
pub const FRAME_HEAD: usize = 4 + 8 + 8 + 32;
/// The longest payload a frame may claim: a damaged `len` must not become a buffer.
pub const FRAME_MAX: usize = 1 << 20;

fn sum_of(msg: &[u8], ids: &dyn Ids) -> Result<[u8; 32], SyncError> {
    let mut m = Vec::with_capacity(TAG.len() + msg.len());
    m.extend_from_slice(TAG);
    m.extend_from_slice(msg);
    ids.hash(&m).ok_or(SyncError::BadSum)
}

/// `msg` from byte `from` on, each frame at most `max_frame` bytes on the carrier.
pub fn frames(msg: &[u8], max_frame: usize, from: u64, ids: &dyn Ids) -> Result<Vec<Vec<u8>>, SyncError> {
    let room = max_frame.saturating_sub(FRAME_HEAD).min(FRAME_MAX);
    if room == 0 {
        return Err(SyncError::Over { what: "frame head", got: FRAME_HEAD as u64, max: max_frame as u64 });
    }
    if msg.is_empty() {
        return Err(SyncError::Short("frame"));
    }
    let sum = sum_of(msg, ids)?;
    let start = usize::try_from(from).map_err(|_| SyncError::Short("frame"))?;
    let rest = msg.get(start..).ok_or(SyncError::Short("frame"))?;
    let mut out = Vec::with_capacity(rest.len() / room + 1);
    let mut offset = from;
    for piece in rest.chunks(room) {
        let mut f = Vec::with_capacity(FRAME_HEAD + piece.len());
        f.extend_from_slice(&(piece.len() as u32).to_le_bytes());
        f.extend_from_slice(&(msg.len() as u64).to_le_bytes());
        f.extend_from_slice(&offset.to_le_bytes());
        f.extend_from_slice(&sum);
        f.extend_from_slice(piece);
        out.push(f);
        offset += piece.len() as u64;
    }
    Ok(out)
}

/// One frame's head as read.
struct Head {
    len: usize,
    total: u64,
    offset: u64,
    sum: [u8; 32],
}

fn head_of(b: &[u8]) -> Option<Head> {
    let len = u32::from_le_bytes(b.get(0..4)?.try_into().ok()?) as usize;
    let total = u64::from_le_bytes(b.get(4..12)?.try_into().ok()?);
    let offset = u64::from_le_bytes(b.get(12..20)?.try_into().ok()?);
    Some(Head { len, total, offset, sum: b.get(20..52)?.try_into().ok()? })
}

/// The receiving half. Holds at most one message, and one frame's worth of unread bytes.
pub struct Assembler {
    max: usize,
    pending: Vec<u8>,
    got: Vec<u8>,
    total: u64,
    sum: [u8; 32],
}

impl Assembler {
    /// `max`: the budget's `message`.
    pub fn new(max: usize) -> Self {
        Assembler { max, pending: Vec::new(), got: Vec::new(), total: 0, sum: [0u8; 32] }
    }

    /// Where the sender should go on from: bytes of the message named by `sum` held in order.
    pub fn resume_at(&self) -> (u64, [u8; 32]) {
        (self.got.len() as u64, self.sum)
    }

    fn clear(&mut self) {
        self.got.clear();
        self.total = 0;
        self.sum = [0u8; 32];
    }

    /// One frame: the whole message, if this was its last byte.
    fn take(&mut self, h: &Head, piece: &[u8], ids: &dyn Ids) -> Result<Option<Vec<u8>>, SyncError> {
        if h.total == 0 || h.offset.saturating_add(h.len as u64) > h.total {
            return Err(SyncError::Mark("frame"));
        }
        if h.sum != self.sum || h.total != self.total {
            if h.offset != 0 {
                return Ok(None);
            }
            self.clear();
            (self.sum, self.total) = (h.sum, h.total);
        }
        let have = self.got.len() as u64;
        if h.offset > have {
            return Ok(None);
        }
        let skip = usize::try_from(have - h.offset).unwrap_or(usize::MAX);
        self.got.extend_from_slice(piece.get(skip..).unwrap_or(&[]));
        if self.got.len() as u64 != self.total {
            return Ok(None);
        }
        let msg = core::mem::take(&mut self.got);
        let ok = sum_of(&msg, ids)? == self.sum;
        self.clear();
        if ok {
            Ok(Some(msg))
        } else {
            Err(SyncError::BadSum)
        }
    }

    /// Bytes as they came: every message they finish, oldest first. An `Err` is a frame no sender
    /// of ours wrote: the partial message and every unread byte go with it, and the caller says hello.
    pub fn feed(&mut self, bytes: &[u8], ids: &dyn Ids) -> Result<Vec<Vec<u8>>, SyncError> {
        self.pending.extend_from_slice(bytes);
        let out = self.drain(ids);
        if out.is_err() {
            self.pending.clear();
            self.clear();
        }
        out
    }

    fn drain(&mut self, ids: &dyn Ids) -> Result<Vec<Vec<u8>>, SyncError> {
        let mut out = Vec::new();
        let mut at = 0usize;
        while let Some(h) = self.pending.get(at..).and_then(head_of) {
            if h.len > FRAME_MAX {
                return Err(SyncError::Over { what: "frame", got: h.len as u64, max: FRAME_MAX as u64 });
            }
            if h.total > self.max as u64 {
                return Err(SyncError::Over { what: "message", got: h.total, max: self.max as u64 });
            }
            let end = at + FRAME_HEAD + h.len;
            let Some(piece) = self.pending.get(at + FRAME_HEAD..end).map(|p| p.to_vec()) else { break };
            out.extend(self.take(&h, &piece, ids)?);
            at = end;
        }
        self.pending.drain(..at.min(self.pending.len()));
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
