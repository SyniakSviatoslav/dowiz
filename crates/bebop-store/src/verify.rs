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

use crate::evlog::{EvLog, Record};
use crate::kv::Kv;
use crate::{crc32_cells, Store, StoreError};

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

/// What `EvLog::chain_scan` found: the chain's length (`None` = it never ends) and the
/// records it quarantines for a failed crc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainScan {
    pub chained: Option<usize>,
    pub quarantined: Vec<BadCrc>,
}

/// Why a KV image was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvError {
    /// No KV root, or a slice in it does not fit its array (what `load` always said `None` to).
    NotKv,
    /// The root or one of its four arrays fails its crc.
    BadCrc(BadCrc),
}

impl Kv {
    /// Read all entries out of a store, refusing an object whose crc does not match.
    pub fn load_checked(st: &Store) -> Result<Kv, KvError> {
        let root = st.root().ok_or(KvError::NotKv)?;
        st.check_obj(root).map_err(KvError::BadCrc)?;
        for i in 1..=4 {
            if let Some(arr) = st.follow(root, i) {
                st.check_obj(arr).map_err(KvError::BadCrc)?;
            }
        }
        // W-DELTA: every record on the chain is checked too, before any is replayed -- after a
        // version this code does not know is refused, so it never surfaces as a crc failure.
        if Kv::version(st) > crate::kv::delta::VERSION_DELTA {
            return Err(KvError::NotKv);
        }
        let chain = crate::kv::delta::chain_in(st, root).ok_or(KvError::NotKv)?;
        crate::kv::delta::check_chain(st, &chain).map_err(KvError::BadCrc)?;
        Kv::decode(st).ok_or(KvError::NotKv)
    }

    /// `load_checked` for a caller that only asks yes or no. `None` covers a crc
    /// mismatch too: it is never data.
    pub fn load(st: &Store) -> Option<Kv> {
        Self::load_checked(st).ok()
    }
}

impl EvLog {
    /// `chain_len`, CHECKING THE CRC of the root and of every record it counts.
    ///
    /// `Ok(None)` is what `chain_len` says about a chain that never ends -- and here also
    /// about records that OVERLAP: the records of an honest log are disjoint objects, so
    /// together they hold at most the image's cells. Hashing is bounded by that, which is
    /// what keeps a crafted loop of large records from costing `step_cap` x image.
    pub fn chain_crc(st: &Store) -> Result<Option<usize>, BadCrc> {
        let Some(root) = st.root() else { return Ok(Some(0)) };
        st.check_obj(root)?;
        let cap = Self::step_cap(st);
        let mut budget = st.cells.len();
        let mut n = 0usize;
        let mut cur = st.follow(root, 1);
        while let Some(obj) = cur {
            n += 1;
            let cost = st.obj_cells(obj).saturating_add(2);
            if n > cap || cost > budget {
                return Ok(None);
            }
            budget -= cost;
            st.check_obj(obj)?;
            cur = st.follow(obj, 2);
        }
        Ok(Some(n))
    }

    /// The APPEND-LOG policy (operator, 2026-10-05): a record whose crc fails is
    /// QUARANTINED -- named and counted, every other record served -- as long as the walk
    /// THROUGH it can still be trusted. Where the line is, exactly:
    ///
    ///   * the ROOT's crc fails -> refused (`Err`): the root holds the count, the newest
    ///     record's ref and the tip; nothing below it can be found without believing it.
    ///   * a RECORD's crc fails -> quarantined only if (a) the chain still delivers
    ///     exactly the root's count and ends, which the caller checks from `chained`, and
    ///     (b) the record's own `prev` id (payload cells 7..10) equals the id (cells 3..6)
    ///     of the record its `next` ref (cell 2) leads to. (b) is what makes a changed
    ///     REF visible: it would land on a record whose id this one does not name. A
    ///     change that lands in the `prev` cells themselves fails (b) too and is refused
    ///     -- it cannot be told from a changed ref. The oldest record (no `next`) has no
    ///     neighbour to check against; (a) covers it.
    ///   * anything else -- (b) failing -> refused (`Err`, naming the record).
    ///
    /// `quarantined` lists the bad records newest first, as `BadCrc`s; their positions
    /// in a newest-first walk are what `walk_marked` reports.
    pub fn chain_scan(st: &Store) -> Result<ChainScan, BadCrc> {
        let Some(root) = st.root() else { return Ok(ChainScan { chained: Some(0), quarantined: Vec::new() }) };
        st.check_obj(root)?;
        let cap = Self::step_cap(st);
        let mut budget = st.cells.len();
        let mut out = ChainScan { chained: None, quarantined: Vec::new() };
        let mut n = 0usize;
        let mut cur = st.follow(root, 1);
        while let Some(obj) = cur {
            n += 1;
            let cost = st.obj_cells(obj).saturating_add(2);
            if n > cap || cost > budget {
                return Ok(out);
            }
            budget -= cost;
            let next = st.follow(obj, 2);
            if let Err(bad) = st.check_obj(obj) {
                if let Some(older) = next {
                    let linked = (0..4).all(|i| st.get(obj, 7 + i) == st.get(older, 3 + i));
                    if !linked {
                        return Err(bad);
                    }
                }
                out.quarantined.push(bad);
            }
            cur = next;
        }
        out.chained = Some(n);
        Ok(out)
    }

    /// `walk`, each record with `Some(stored crc)` when its crc FAILS (a quarantined
    /// record), `None` when it holds. Same order and cap as `walk`.
    pub fn walk_marked(st: &Store) -> Vec<(Record, Option<u32>)> {
        let mut objs = Vec::new();
        if let Some(root) = st.root() {
            let cap = Self::step_cap(st);
            let mut cur = st.follow(root, 1);
            while let Some(o) = cur {
                if objs.len() >= cap {
                    break;
                }
                objs.push(o);
                cur = st.follow(o, 2);
            }
        }
        Self::walk(st)
            .into_iter()
            .zip(objs.into_iter().map(Some).chain(std::iter::repeat(None)))
            .map(|(r, o)| (r, o.and_then(|o| st.check_obj(o).err().map(|b| b.want))))
            .collect()
    }

    /// Append a record CARRYING a failed crc: `carry = Some(stored crc)` writes that crc
    /// back into the new object instead of the one `seal` computed, so a record that was
    /// quarantined in the old image is quarantined in the copy too. Every replay (a grow,
    /// a rotation, a redaction rebuild) goes through this: a replay that re-sealed would
    /// LAUNDER a changed byte into a record with a valid crc.
    pub fn append_carry_bytes(st: &mut Store, rec: &Record, carry: Option<u32>) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_append_pub(st, rec)?;
        if let (Some(c), Some(ev)) = (carry, st.follow(root, 1)) {
            st.cells[ev + 1] = ((c as i64) << 32) | (st.cells[ev + 1] & 0xFFFF_FFFF);
        }
        Ok(st.commit_bytes(&tx, root))
    }

    /// Copy every record of `from` into `to` (an initialised log), oldest first, failed
    /// crcs carried, and name the newest as the tip. What `grow` does.
    pub fn copy_chain_bytes(from: &Store, to: &mut Store) -> Result<(), StoreError> {
        let mut recs = Self::walk_marked(from);
        recs.reverse();
        let mut last = None;
        for (r, carry) in &recs {
            Self::append_carry_bytes(to, r, *carry)?;
            last = Some(r.id);
        }
        if let Some(id) = last {
            Self::set_tip_bytes(to, &id)?;
        }
        Ok(())
    }

    /// `walk`, after `chain_crc` passed. For a reader with no loader above it.
    pub fn walk_checked(st: &Store) -> Result<Vec<Record>, BadCrc> {
        Self::chain_crc(st)?;
        Ok(Self::walk(st))
    }
}

#[cfg(test)]
mod tests;
