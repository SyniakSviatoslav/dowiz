//! ONE SYNC SESSION (W-MG-P1, W-MG-P2, docs/design/MESH-TRANSPORT-OVER-EINFILE.md §3, §8, §11).
//!
//! NO TRANSPORT IN THIS CRATE, AND NONE ASSUMED. Bytes in, bytes out: no socket, no task, no clock,
//! and no message longer than `Budget::message`, which a tail is cut to fit (`Doc::tail_part`). A
//! carrier may be a LAN stream, a browser data channel with small frames, a relay that never reads
//! what it holds, or a person with a phone: `frame.rs` and `file.rs` sit on top of the same messages,
//! and each message means the same on all of them.
//!
//! WHAT RECOVERS A BREAK. A live file moves only when a tail has been judged whole, and the same tail
//! twice writes nothing. So a break anywhere is answered by `Peer::hello`, and a frame stream can go
//! on from `Assembler::resume_at`. A tail built for a position we never held is answered by our own
//! FRONTIER. No session state outlives a message.
//!
//! WHAT IS NOT WRITTEN HERE: HELLO and PROVE (§3) need a channel to bind to; `Peer::on` must only be
//! fed by a caller that has authenticated its peer. Encryption for a blind relay is the carrier's
//! layer and takes these bytes as they are. Both come after P2.
//!
//! This crate names no algorithm: ids, hashes and signatures come through `bebop-log::doc`'s seams.

#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]
extern crate alloc;

use alloc::string::String;
use bebop_log::doc::DocError;

mod file;
mod frame;
mod grant;
mod memo;
mod session;
mod wire;

#[cfg(test)]
mod toys;

pub use file::{pack, unpack, Carried, FILE_MARK};
pub use frame::{frames, Assembler, FRAME_HEAD, FRAME_MAX};
pub use grant::{grant_body, is_red_line, parse_grant, Grants, Pair, RedLine, GRANT_MARK};
pub use memo::Memo;
pub use session::{Adopted, Device, Peer, Step};
pub use wire::{decode, encode_ack, encode_frontier, encode_need, encode_tail, refusal_name, Msg, Need, MARK};

/// What one message may hold, checked before anything is allocated (§11). The defaults are the
/// design's, every one of them a HYPOTHESIS until measured, but `record_bytes`: its 1 024 refuses an
/// ADMIT, which carries a 1 952-byte key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub message: usize,
    pub places: u64,
    pub records: u64,
    pub record_bytes: u64,
    pub seals: u64,
    pub keys: u64,
    pub anchors: u64,
    pub proof_bytes: u64,
    /// The largest image an adoption may rebuild into.
    pub image: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Budget { message: 8 << 20, places: 4096, records: 4096, record_bytes: 16 << 10, seals: 64, keys: 8, anchors: 16, proof_bytes: 64 << 10, image: 1 << 30 }
    }
}

/// Why a message or a file was refused. A refusal changes nothing.
#[derive(Debug, PartialEq)]
pub enum SyncError {
    /// The bytes ended inside `what`.
    Short(&'static str),
    /// Bytes past the end of a message that is exact.
    Trailing,
    /// `what` holds `got` and the budget says `max`.
    Over { what: &'static str, got: u64, max: u64 },
    /// Not one of ours, or a number with no meaning at `what`.
    Mark(&'static str),
    /// The bytes are not the bytes that were sent.
    BadSum,
    /// A carried tail past the time its sender gave it.
    Expired { expires_ms: i64, now_ms: i64 },
    /// A sound message about a document the library refuses.
    Doc(DocError),
    /// A `Need` read as a tail, a message read at the wrong place. The string names it.
    Unexpected(String),
}

impl From<DocError> for SyncError {
    fn from(e: DocError) -> Self {
        SyncError::Doc(e)
    }
}
