//! ONE SUBJECT'S RECORDS WITHOUT UNPACKING THE WHOLE LOG (W-OCHAIN, R-BEBOPDB D.1 #5).
//!
//! `walk_marked` unpacks every payload and hashes every record, so asking for one order's
//! history of a 2000-event log decoded 2000 records to keep four: 2.1 ms against SQLite's
//! 3.1 us on an `(order_id, seq)` index (R-BEBOPDB F.3 W4r).
//!
//! `walk_marked_where` walks the SAME chain, under the SAME `step_cap` and the SAME
//! read budget, but per record reads only the header cells and the few payload cells
//! that hold `head`. Only a record whose payload bytes `at..at + head.len()` EQUAL `head`
//! is crc-checked (`check_obj`) and unpacked (`read_at`). Its answer is, by construction,
//! `walk_marked` filtered by that prefix -- `tests::equals_walk_marked_filtered` holds it
//! to exactly that, record for record, over v1, v2, named actors and truncated headers.
//!
//! NO FORMAT CHANGE AND NO INDEX. Nothing is written, so there is nothing that can go
//! stale after a grow, a rotation or a redaction rebuild: every call reads the image as it
//! is. It is still a walk -- O(n) header reads plus O(k) unpacks -- which is the honest
//! limit; a per-subject back-link (a v3 record) would make it O(k) and costs a format.
//!
//! THE BYTES COMPARED ARE THE BYTES `read_at` WOULD RETURN. A byte exists only below
//! `min(payload_len claim, what the object holds, what the walk's budget still allows)`,
//! the three bounds `read_at` applies, so a truncated or lying header cannot make this
//! match a record whose unpacked payload would not.

use super::{EvLog, Record, FLAG_ACTOR, HEAD_V1, HEAD_V2};
use crate::Store;

impl EvLog {
    /// `walk_marked`, keeping only the records whose payload bytes at `at` equal `head`.
    /// Newest first; each with `Some(stored crc)` when its crc FAILS (quarantined).
    pub fn walk_marked_where(st: &Store, at: usize, head: &[u8]) -> Vec<(Record, Option<u32>)> {
        let head = Head::new(at, head);
        let mut out = Vec::new();
        Self::scan(st, |p| {
            if p.has(&head) {
                out.push((p.record(), p.crc_bad()));
            }
            false
        });
        out
    }

    /// THE WALK UNDER EVERY PREFIX READER (W-LOOPB): the chain newest first, under `walk`'s
    /// `step_cap` and read budget, handing `visit` a `Peek` of each record -- its position,
    /// its payload bytes one at a time -- WITHOUT unpacking or hashing it. `Peek::record` and
    /// `Peek::crc_bad` unpack / hash the one record asked about, with exactly the bytes
    /// `read_at` hands `walk` there. `visit` returning true stops the walk after that record.
    pub fn scan(st: &Store, mut visit: impl FnMut(&Peek<'_>) -> bool) {
        let Some(root) = st.root() else { return };
        let version = Self::version(st);
        let cap = Self::step_cap(st);
        // `walk_until`'s budget, spent the same way record by record, so a record far down
        // a hostile chain is cut short here exactly where `walk` would cut it.
        let mut budget = st.cells.len();
        let mut pos = 0usize;
        let mut cur = st.follow(root, 1);
        while let Some(obj) = cur {
            if pos >= cap {
                break;
            }
            let span = span(st, version, obj, budget);
            let p = Peek { st, version, obj, budget, span, pos };
            if visit(&p) {
                break;
            }
            budget -= p.span.cells;
            pos += 1;
            cur = st.follow(obj, 2);
        }
    }
}

/// A prefix to look for: payload bytes `at..at + bytes.len()`, compared a cell at a time.
pub struct Head {
    at: usize,
    bytes: Vec<u8>,
    words: Words,
}

impl Head {
    pub fn new(at: usize, bytes: &[u8]) -> Self {
        Head { at, bytes: bytes.to_vec(), words: Words::of(at, bytes) }
    }
}

/// One record of a `scan`, not yet unpacked.
pub struct Peek<'a> {
    st: &'a Store,
    version: i64,
    obj: usize,
    budget: usize,
    span: Span,
    /// Its place in the newest-first walk: 0 = the newest.
    pub pos: usize,
}

impl Peek<'_> {
    /// How many payload bytes `read_at` would hand back for this record.
    pub fn len(&self) -> usize {
        self.span.bytes
    }

    pub fn is_empty(&self) -> bool {
        self.span.bytes == 0
    }

    /// Payload byte `j`, as `read_at` would return it; `None` past `len()`.
    pub fn byte(&self, j: usize) -> Option<u8> {
        (j < self.span.bytes).then(|| self.span.byte(self.st, self.obj, j))
    }

    /// Do this record's payload bytes at `head.at` equal `head`'s?
    pub fn has(&self, head: &Head) -> bool {
        self.span.matches(self.st, self.obj, head.at, &head.bytes, &head.words)
    }

    /// The record, unpacked: what `walk` returns at this position.
    pub fn record(&self) -> Record {
        let mut b = self.budget;
        EvLog::read_at(self.st, self.version, self.obj, &mut b)
    }

    /// `Some(stored crc)` when this record's crc FAILS -- `walk_marked`'s mark.
    pub fn crc_bad(&self) -> Option<u32> {
        self.st.check_obj(self.obj).err().map(|bad| bad.want)
    }
}

/// Where one record's payload is and how many of its bytes `read_at` would hand back.
struct Span {
    /// Payload cell index of payload byte 0.
    from: usize,
    /// Payload cells charged to the walk's budget (`read_at`'s `have`).
    cells: usize,
    /// Payload bytes that exist: `read_at`'s returned length.
    bytes: usize,
    /// v2 packs eight bytes to a cell; v1 stores one per cell.
    packed: bool,
}

/// `read_at`'s arithmetic, without reading the payload. Kept beside the test that
/// compares the two answers record for record (`tests.rs`).
fn span(st: &Store, version: i64, obj: usize, budget: usize) -> Span {
    let plen = st.get(obj, 1) as usize;
    if version >= 2 {
        let named = st.get(obj, 11) & FLAG_ACTOR != 0 && st.obj_cells(obj) >= (HEAD_V2 + 4) as usize;
        let from = HEAD_V2 as usize + if named { 4 } else { 0 };
        let cells = st.obj_cells(obj).saturating_sub(from).min(budget);
        Span { from, cells, bytes: plen.min(cells * 8), packed: true }
    } else {
        let from = HEAD_V1 as usize;
        let cells = st.obj_cells(obj).saturating_sub(from).min(budget);
        Span { from, cells, bytes: plen.min(cells), packed: false }
    }
}

impl Span {
    fn byte(&self, st: &Store, obj: usize, j: usize) -> u8 {
        if self.packed {
            (st.get(obj, self.from + j / 8) >> ((j % 8) * 8)) as u8
        } else {
            st.get(obj, self.from + j) as u8
        }
    }

    fn matches(&self, st: &Store, obj: usize, at: usize, head: &[u8], words: &Words) -> bool {
        let Some(end) = at.checked_add(head.len()) else { return false };
        if end > self.bytes {
            return false;
        }
        if self.packed {
            words.0.iter().all(|&(k, mask, want)| st.get(obj, self.from + k) as u64 & mask == want)
        } else {
            head.iter().enumerate().all(|(i, b)| self.byte(st, obj, at + i) == *b)
        }
    }
}

/// `head` at byte `at` as v2 cells: `(cell, mask, want)` per cell it touches, so a packed
/// record is compared a cell at a time -- `Span::byte`'s little-endian order, built ONCE per
/// walk rather than per record. Empty when `at + head.len()` overflows (`matches` refuses
/// that before looking here).
struct Words(Vec<(usize, u64, u64)>);

impl Words {
    fn of(at: usize, head: &[u8]) -> Self {
        let mut out: Vec<(usize, u64, u64)> = Vec::new();
        if at.checked_add(head.len()).is_none() {
            return Words(out);
        }
        for (i, &b) in head.iter().enumerate() {
            let j = at + i;
            let sh = (j % 8) * 8;
            match out.last_mut() {
                Some(w) if w.0 == j / 8 => {
                    w.1 |= 0xFF << sh;
                    w.2 |= u64::from(b) << sh;
                }
                _ => out.push((j / 8, 0xFF << sh, u64::from(b) << sh)),
            }
        }
        Words(out)
    }
}

#[cfg(test)]
mod tests;
