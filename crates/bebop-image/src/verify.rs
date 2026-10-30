//! CRC CHECKED ON LOAD (W-CRC, R-BEBOPDB D.1 #2, 2026-10-05).
//!
//! Every object carries `crc32(payload) << 32` in its second header cell (`Store::seal`),
//! and until this module no hot reader looked at it: `Kv::load` and `EvLog::walk`
//! believed every payload cell they were handed, so one changed byte of a value came
//! back as a different value, with nothing anywhere saying so. Only `proj::lookup` and
//! the `inspect` binary checked.
//!
//! WHERE THE CHECK RUNS, and why there and not on every cell read. The bytes become a
//! `Store` once per load (`Store::from_bytes`); after that the cells are this process's
//! own memory, and every write to them goes through `alloc` + `seal`. So the check sits
//! at the two entry points every loader already passes through:
//!
//!   * `Kv::load` / `Kv::load_checked` -- the root and the four arrays it reads, before
//!     a single entry is decoded. `Kv::load` keeps its `Option` signature for the callers
//!     that only need yes/no; `load_checked` says WHICH object failed.
//!   * `EvLog::chain_crc` -- the log's root and every record on the chain, in the same
//!     walk that counts them. `dowiz_hub::chain_is_whole` calls it on every log load
//!     (`Hub`, `LogImage`, `StockLog`), and `EvLog::walk_checked` is the same check for
//!     a reader that has no loader above it (the wasm reader).
//!
//! A mismatch is a typed `BadCrc` naming the object (its cell index) and both numbers --
//! never a panic, never a short or silently different answer.
//!
//! OLD IMAGES. The crc has been written by every writer since the format existed (Rust
//! `seal`, bebop `st_seal`), over exactly the cells checked here, so a valid old image
//! passes; the bebop-wasm fixtures `kv.store` (v1, written by kv.bp + kvdemo) and
//! `kv2.store` are pinned by `tests::the_frozen_fixtures_pass_the_check`.

//!
//! THE LAST COMMIT, ONE RULE FOR EVERY READER (W-ES-P10, FORMAT §7 phase P2, 2026-10-30).
//! Before it, one image got three answers when an object of the LAST commit was corrupted:
//! bebop.bin's `st_open` fell back to generation k-1, the C ABI refused `LAST_COMMIT` (41),
//! and native/wasm32/python, which had no anchor logic, said 2 or 10 (measured by W-ES-P8D on
//! kv4badf/kv4badd before it re-anchored them). `last_commit_in` is the rule now, and every
//! Rust reader of bytes calls it; oracle.py and kv.bp re-implement it line for line:
//!
//!   * `Anchor::Holds`  -- the range `[mark, arena_used)` hashes to what the commit recorded:
//!                         the last commit is proved, and an open at `Verify::Anchor` is done
//!                         in O(that range), the older commits trusted (`KvIn::open_lazy`).
//!   * `Anchor::Absent` -- `anc` = 0 (a pre-W-ES-P0 image, or a mark past `MAX_ANCHOR_MARK`):
//!                         the anchor proves nothing; the reader checks what it reaches.
//!   * `Anchor::Torn`   -- a reader of BYTES HANDED WHOLE (every conformance reader, the wasm
//!                         module in a Worker, `Db::from_bytes` under `OnTorn::Refuse`) REFUSES:
//!                         it walks the objects of `[mark, arena_used)` and names the first whose
//!                         payload fails its crc (`LastCommit::Object`, status BAD_CRC 10); when
//!                         every object checks (a re-sealed object, a changed header or length)
//!                         the refusal is `LastCommit::Anchor` (status LAST_COMMIT 41). NEVER a
//!                         fallback: bytes handed whole were not torn by a power cut, they are
//!                         corrupt, and generation k-1 is not what the holder stored.
//!                         Only a reader with RECOVERY AUTHORITY over a device -- `Db::open` on a
//!                         file under `OnTorn::Fallback`, bebop `st_open` on its MAP_SHARED
//!                         mapping -- hides that superblock and opens generation k-1 after
//!                         walking its whole arena (`bebop_io::sim::recover`).
//!
//! The documented consequence (R-EMBED-STORE §12 P10's T-ES): a corrupted object of an OLDER
//! commit is NOT caught by an open at `Verify::Anchor` -- the anchor covers the last commit
//! only -- and IS caught by `verify(Reachable)` when the KV reaches it, by `verify(WholeArena)`
//! always. `bebop_io::db::lazy_tests` pins both halves.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::format::{anchor_in, Anchor};
use crate::{crc32_cells, crc32_in, obj_len_in, Cells, Store, Superblock};

/// Why a reader of bytes refused the last commit (FORMAT §7): see the module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastCommit {
    /// The anchor is torn and this object of `[mark, arena_used)` fails its crc (status 10).
    Object(BadCrc),
    /// The anchor is torn and every object of the range checks (status 41).
    Anchor(Anchor),
}

/// `Store::check_obj` over `Cells`.
pub fn check_obj_in<C: Cells + ?Sized>(c: &C, obj: usize) -> Result<(), BadCrc> {
    let want = ((c.cell_at(obj.saturating_add(1)) >> 32) & 0xFFFF_FFFF) as u32;
    let len = obj_len_in(c, obj).max(0) as usize;
    if obj.saturating_add(2).saturating_add(len) > c.n_cells() {
        return Err(BadCrc { obj, want, got: None });
    }
    let got = crc32_in(c, obj + 2, len);
    if got == want {
        Ok(())
    } else {
        Err(BadCrc { obj, want, got: Some(got) })
    }
}

/// THE RULE (module doc): the anchor of the superblock `sb` names, and -- when it is torn --
/// the refusal a reader of bytes gives, the object named where one fails. `Ok` carries the
/// anchor so a caller can tell `Holds` (the last commit proved) from `Absent` (nothing proved).
pub fn last_commit_in<C: Cells + ?Sized>(c: &C, sb: &Superblock) -> Result<Anchor, LastCommit> {
    let a = anchor_in(c, sb);
    let Anchor::Torn { mark, used, got, .. } = a else { return Ok(a) };
    // `got: None`: the mark is not a range inside the arena, so there is nothing to walk (oracle.py
    // and kv.bp say 41 there too; walking from a mark below ARENA would "name" a superblock cell).
    if got.is_none() {
        return Err(LastCommit::Anchor(a));
    }
    let end = (used.max(0) as usize).min(c.n_cells());
    let mut i = mark.max(0) as usize;
    while i < end {
        check_obj_in(c, i).map_err(LastCommit::Object)?;
        i = i.saturating_add(2).saturating_add(obj_len_in(c, i).max(0) as usize);
    }
    Err(LastCommit::Anchor(a))
}

/// An object whose payload does not hash to the crc its header carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadCrc {
    /// The object's first header cell -- the same index `follow`/`root` return.
    pub obj: usize,
    /// What the header says.
    pub want: u32,
    /// What the payload hashes to; `None` when the header's length does not even fit
    /// the image, so there is no payload to hash.
    pub got: Option<u32>,
}

impl Store {
    /// Check one object's payload against its header crc.
    pub fn check_obj(&self, obj: usize) -> Result<(), BadCrc> {
        let want = ((self.cell(obj.saturating_add(1)) >> 32) & 0xFFFF_FFFF) as u32;
        let len = self.obj_len(obj) as usize;
        if obj.saturating_add(2).saturating_add(len) > self.cells.len() {
            return Err(BadCrc { obj, want, got: None });
        }
        let got = crc32_cells(&self.cells, obj + 2, len);
        if got == want {
            Ok(())
        } else {
            Err(BadCrc { obj, want, got: Some(got) })
        }
    }
}

#[cfg(test)]
mod tests;
