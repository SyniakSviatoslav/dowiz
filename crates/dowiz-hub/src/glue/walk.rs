//! A ZERO-COPY WALK of a v2 append log's chain, newest first, over the image BYTES (`View`):
//! no `Store::from_bytes` copy, and a record's payload is unpacked only when the caller asks
//! for it. The ancestor search reads four id cells per record and nothing else.
//!
//! The layout is `bebop_store::evlog`'s v2 record (its module header): `0 actor_seq`,
//! `1 payload_len`, `2 ref prev`, `3..6 id`, `7..10 prev id`, `11 flags`, `12..` (+4 when an
//! actor is named) the payload packed eight bytes to a cell. A v1 root is not walked here; the
//! caller refuses it by name (`Why::OldFormat`) -- every log that has grown once is v2.
//!
//! EVERY CLAIM IS BOUNDED as `EvLog::read_at` bounds it: the object's length is the fact, the
//! header's is a claim, and a chain that does not end stops at the image's record capacity.

use bebop_store::{follow_in, get_in, obj_cells_in, obj_crc_ok_in, root_in, View};

/// The v2 record head, in cells (`evlog::HEAD_V2`).
const HEAD_V2: usize = 12;
/// Root cell 7: the log's version.
const ROOT_VERSION: usize = 7;

/// One log, opened for walking.
pub(crate) struct Chain<'a> {
    pub(crate) view: View<'a>,
    /// Records the root claims.
    pub(crate) n: usize,
    /// The root's tip (all zeros when unset).
    pub(crate) tip: [u8; 32],
    /// The newest record, or `None` for an empty log.
    first: Option<usize>,
    /// The most records this image can hold: past it the chain is a cycle.
    cap: usize,
}

/// Why a log could not be opened for walking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenErr {
    NoRoot,
    RootCrc,
    OldFormat,
}

impl<'a> Chain<'a> {
    pub(crate) fn open(bytes: &'a [u8]) -> Result<Chain<'a>, OpenErr> {
        let view = View::new(bytes);
        let root = root_in(&view).ok_or(OpenErr::NoRoot)?;
        if !obj_crc_ok_in(&view, root) {
            return Err(OpenErr::RootCrc);
        }
        if get_in(&view, root, ROOT_VERSION) < 2 {
            return Err(OpenErr::OldFormat);
        }
        let n = get_in(&view, root, 0).max(0) as usize;
        let tip = if get_in(&view, root, 2) == 0 { [0u8; 32] } else { cells_b32(&view, root, 3) };
        let first = follow_in(&view, root, 1);
        let cap = bytes.len() / 8 / (2 + HEAD_V2) + 1;
        Ok(Chain { view, n, tip, first, cap })
    }

    /// The records newest first, as object offsets.
    pub(crate) fn objs(&self) -> Objs<'_, 'a> {
        Objs { chain: self, cur: self.first, steps: 0 }
    }

    /// A record's content id.
    pub(crate) fn id(&self, obj: usize) -> [u8; 32] {
        cells_b32(&self.view, obj, 3)
    }

    /// A record's `prev` id.
    pub(crate) fn prev(&self, obj: usize) -> [u8; 32] {
        cells_b32(&self.view, obj, 7)
    }

    /// Does the object's crc match its cells?
    pub(crate) fn crc_ok(&self, obj: usize) -> bool {
        obj_crc_ok_in(&self.view, obj)
    }

    /// The object's cells including its two header cells: what it costs on the wire.
    pub(crate) fn wire_cells(&self, obj: usize) -> usize {
        2 + obj_cells_in(&self.view, obj)
    }

    /// The payload bytes, bounded by the object (a short record hands back what is there).
    pub(crate) fn payload(&self, obj: usize) -> Vec<u8> {
        let v = &self.view;
        let plen = get_in(v, obj, 1).max(0) as usize;
        let have = obj_cells_in(v, obj);
        let named = get_in(v, obj, 11) & 1 != 0 && have >= HEAD_V2 + 4;
        let at = HEAD_V2 + if named { 4 } else { 0 };
        let cells = have.saturating_sub(at);
        let mut out = Vec::with_capacity(cells * 8);
        for j in 0..cells {
            out.extend_from_slice(&get_in(v, obj, at + j).to_le_bytes());
        }
        out.truncate(plen.min(cells * 8));
        out
    }
}

/// Newest-first iterator over a chain's record objects, stopping at the record capacity.
pub(crate) struct Objs<'c, 'a> {
    chain: &'c Chain<'a>,
    cur: Option<usize>,
    steps: usize,
}

impl Iterator for Objs<'_, '_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        let obj = self.cur?;
        if self.steps >= self.chain.cap {
            self.cur = None;
            return None;
        }
        self.steps += 1;
        self.cur = follow_in(&self.chain.view, obj, 2);
        Some(obj)
    }
}

/// Four little-endian cells from payload cell `at` as 32 bytes (`evlog::cells_to_b32`).
fn cells_b32(v: &View<'_>, obj: usize, at: usize) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..i * 8 + 8].copy_from_slice(&get_in(v, obj, at + i).to_le_bytes());
    }
    out
}

/// 32 bytes as the cover manifest's four digest cells (`cover::Entry::digest`, `Kind::Log`).
pub fn to_cells(b: &[u8; 32]) -> [i64; 4] {
    let mut out = [0i64; 4];
    for (i, c) in out.iter_mut().enumerate() {
        let mut w = [0u8; 8];
        w.copy_from_slice(&b[i * 8..i * 8 + 8]);
        *c = i64::from_le_bytes(w);
    }
    out
}

/// The cover manifest's four digest cells back to 32 bytes.
pub fn from_cells(c: &[i64; 4]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..i * 8 + 8].copy_from_slice(&c[i].to_le_bytes());
    }
    out
}
