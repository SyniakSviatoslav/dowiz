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

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{crc32_cells, Store};

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
