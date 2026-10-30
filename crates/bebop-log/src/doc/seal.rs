//! SEAL and HEADS (R-MAINGOAL-A §1.2, §2a): one signature per DEVICE COMMIT over the tip --
//! not per record (a 3 309-byte signature on a 400-byte record is not a format) -- and the
//! file's frontier, one row per device, rewritten by every commit.
//!
//! ```text
//! SEAL  v1  {VERSION=1, actor_id 4, seq, tip 4, claimed_ms, ref PREV_SEAL, ref TIME, alg,
//!            sig_len, sig packed}          15 + ceil(sig_len / 8) cells (ML-DSA-65: 429)
//! HEADS v1  {m, (actor_id 4, seq, tip 4, ref SEAL) × m}   sorted by actor_id, 1 + 10m cells
//! ```
//!
//! The signed message (`message`):
//!
//! ```text
//! "bebop-seal/1" ‖ actor_id 32 ‖ seq LE8 ‖ tip 32 ‖ claimed_ms LE8 ‖ prev_tip 32
//! ```
//!
//! `prev_tip` is the tip the device's PREVIOUS seal signed (0^32 for its first). The plan
//! wrote `sha256(prev_seal)` and `sha256(TIME)` here; this crate has no hash, and the tip
//! already commits to the whole history before it (every id chains to its `prev`), so the
//! previous tip binds the seals into a chain just as tightly. `ref TIME` is kept at 0 for
//! M6 (Roughtime) and is NOT in the v1 message: when it is, that is SEAL v2.
//!
//! What the seal proves, in `verify`'s words (§2d): the holder of key K signed these bytes
//! at seq s of its chain, and nothing under the tip changed since. Not that the event
//! happened, not that `claimed_ms` is true.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{Store, StoreError, Tx};
use super::{bytes_to_cells, cells_of, cells_to_bytes, cells_to_id, id_to_cells};

pub const SEAL_VERSION: i64 = 1;
pub const LAYOUT_SEAL: &str = "SEAL{i64,i64,i64,i64,i64,i64,i64,i64,i64,i64,ref SEAL,ref TIME,i64,i64,[i64]}";
pub const DIGEST_SEAL: i64 = 1951851249;
pub const LAYOUT_HEADS: &str = "HEADS{i64,[i64,i64,i64,i64,i64,i64,i64,i64,i64,ref SEAL]}";
pub const DIGEST_HEADS: i64 = 4237402659;
pub const SEAL_PREFIX: &[u8] = b"bebop-seal/1";
const SEAL_HEAD: usize = 15;
const HEADS_ROW: usize = 10;
/// The longest signature a SEAL may carry (64 KiB; ML-DSA-87 is 4 627 B).
pub const MAX_SIG_BYTES: usize = 65536;

/// One seal as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seal {
    pub at: usize,
    pub actor_id: [u8; 32],
    pub seq: u64,
    pub tip: [u8; 32],
    pub claimed_ms: i64,
    pub prev: Option<usize>,
    pub time: Option<usize>,
    pub alg: i64,
    pub sig: Vec<u8>,
}

/// One row of HEADS: a device's newest sealed position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head {
    pub actor_id: [u8; 32],
    pub seq: u64,
    pub tip: [u8; 32],
    pub seal: Option<usize>,
}

/// The bytes a seal signs.
pub fn message(actor_id: &[u8; 32], seq: u64, tip: &[u8; 32], claimed_ms: i64, prev_tip: &[u8; 32]) -> Vec<u8> {
    let mut m = Vec::with_capacity(SEAL_PREFIX.len() + 32 + 8 + 32 + 8 + 32);
    m.extend_from_slice(SEAL_PREFIX);
    m.extend_from_slice(actor_id);
    m.extend_from_slice(&seq.to_le_bytes());
    m.extend_from_slice(tip);
    m.extend_from_slice(&claimed_ms.to_le_bytes());
    m.extend_from_slice(prev_tip);
    m
}

/// One SEAL object; `None` for a wrong digest, version or length. The crc is the caller's
/// question (`verify` counts a seal whose crc fails as bad).
pub fn read_seal(st: &Store, at: usize) -> Option<Seal> {
    if st.obj_digest(at) != DIGEST_SEAL || st.obj_cells(at) < SEAL_HEAD || st.get(at, 0) != SEAL_VERSION {
        return None;
    }
    let sig_len = st.get(at, 14);
    if !(0..=MAX_SIG_BYTES as i64).contains(&sig_len) {
        return None;
    }
    let n_cells = (sig_len as usize).div_ceil(8);
    if st.obj_cells(at) != SEAL_HEAD + n_cells {
        return None;
    }
    Some(Seal {
        at,
        actor_id: cells_to_id(&cells_of(st, at, 1, 4)),
        seq: st.get(at, 5) as u64,
        tip: cells_to_id(&cells_of(st, at, 6, 4)),
        claimed_ms: st.get(at, 10),
        prev: st.follow(at, 11),
        time: st.follow(at, 12),
        alg: st.get(at, 13),
        sig: cells_to_bytes(&cells_of(st, at, SEAL_HEAD, n_cells), sig_len as usize),
    })
}

/// The HEADS rows, in table order (sorted by actor id when the writer wrote them).
pub fn read_heads(st: &Store, heads: Option<usize>) -> Vec<Head> {
    let Some(t) = heads else { return Vec::new() };
    if st.obj_digest(t) != DIGEST_HEADS || st.obj_cells(t) < 1 {
        return Vec::new();
    }
    let m = st.get(t, 0).max(0) as usize;
    let m = m.min(st.obj_cells(t).saturating_sub(1) / HEADS_ROW);
    (0..m)
        .map(|i| {
            let b = 1 + i * HEADS_ROW;
            Head {
                actor_id: cells_to_id(&cells_of(st, t, b, 4)),
                seq: st.get(t, b + 4) as u64,
                tip: cells_to_id(&cells_of(st, t, b + 5, 4)),
                seal: st.follow(t, b + 9),
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn stage_seal(
    st: &mut Store,
    tx: &mut Tx,
    actor_id: &[u8; 32],
    seq: u64,
    tip: &[u8; 32],
    claimed_ms: i64,
    prev: Option<usize>,
    alg: i64,
    sig: &[u8],
) -> Result<usize, StoreError> {
    let sc = bytes_to_cells(sig);
    let o = st.alloc(tx, (SEAL_HEAD + sc.len()) as i64, DIGEST_SEAL)?;
    st.put_cell(o, 0, SEAL_VERSION);
    for (i, c) in id_to_cells(actor_id).iter().enumerate() {
        st.put_cell(o, 1 + i, *c);
    }
    st.put_cell(o, 5, seq as i64);
    for (i, c) in id_to_cells(tip).iter().enumerate() {
        st.put_cell(o, 6 + i, *c);
    }
    st.put_cell(o, 10, claimed_ms);
    match prev {
        Some(p) => st.link(o, 11, p),
        None => st.put_cell(o, 11, 0),
    }
    st.put_cell(o, 12, 0);
    st.put_cell(o, 13, alg);
    st.put_cell(o, 14, sig.len() as i64);
    for (j, c) in sc.iter().enumerate() {
        st.put_cell(o, SEAL_HEAD + j, *c);
    }
    st.seal(o);
    Ok(o)
}

pub(crate) fn stage_heads(st: &mut Store, tx: &mut Tx, heads: &mut Vec<Head>) -> Result<usize, StoreError> {
    heads.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    let t = st.alloc(tx, (1 + heads.len() * HEADS_ROW) as i64, DIGEST_HEADS)?;
    st.put_cell(t, 0, heads.len() as i64);
    for (i, h) in heads.iter().enumerate() {
        let b = 1 + i * HEADS_ROW;
        for (j, c) in id_to_cells(&h.actor_id).iter().enumerate() {
            st.put_cell(t, b + j, *c);
        }
        st.put_cell(t, b + 4, h.seq as i64);
        for (j, c) in id_to_cells(&h.tip).iter().enumerate() {
            st.put_cell(t, b + 5 + j, *c);
        }
        match h.seal {
            Some(s) => st.link(t, b + 9, s),
            None => st.put_cell(t, b + 9, 0),
        }
    }
    st.seal(t);
    Ok(t)
}
