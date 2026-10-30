//! The log half of the crc check on load (W-CRC, W-HUBCRC): `chain_crc`, the quarantine
//! policy `chain_scan`, `ChainMark`. The module doc of the whole check is bebop-image's
//! `verify.rs`.

use crate::{EvLog, Record};
use crate::{Store, StoreError};
pub use bebop_image::verify::BadCrc;
#[allow(unused_imports)]
use crate::prelude::*;

mod since;
pub use since::ChainMark;

/// What `EvLog::chain_scan` found: the chain's length (`None` = it never ends) and the
/// records it quarantines for a failed crc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainScan {
    pub chained: Option<usize>,
    pub quarantined: Vec<BadCrc>,
}

impl EvLog {
    /// `chain_len`, CHECKING THE CRC of the root and of every record it counts.
    ///
    /// `Ok(None)` is what `chain_len` says about a chain that never ends -- and here also
    /// about records that OVERLAP: the records of an honest log are disjoint objects, so
    /// together they hold at most the image's cells. Hashing is bounded by that, which is
    /// what keeps a crafted loop of large records from costing `step_cap` x image.
    #[inline]
    pub fn chain_crc(st: &Store) -> Result<Option<usize>, BadCrc> {
        Self::chain_crc_at(st, st.root())
    }

    /// `chain_crc` of the log rooted at `root` (W-MG-M1: a `DOC`'s log).
    #[inline]
    pub fn chain_crc_at(st: &Store, root: Option<usize>) -> Result<Option<usize>, BadCrc> {
        let Some(root) = root else { return Ok(Some(0)) };
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
        // THE SAME WALK `chain_scan_since` takes, with no mark: one body for both (W-HUBCRC).
        Self::chain_scan_since(st, None).map(|(scan, _)| scan)
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
            .zip(objs.into_iter().map(Some).chain(core::iter::repeat(None)))
            .map(|(r, o)| (r, o.and_then(|o| st.check_obj(o).err().map(|b| b.want))))
            .collect()
    }

    /// Append a record CARRYING a failed crc: `carry = Some(stored crc)` writes that crc
    /// back into the new object instead of the one `seal` computed, so a record that was
    /// quarantined in the old image is quarantined in the copy too. Every replay (a grow,
    /// a rotation, a redaction rebuild) goes through this: a replay that re-sealed would
    /// LAUNDER a changed byte into a record with a valid crc.
    #[allow(clippy::indexing_slicing)] // T-ES-06 inventory (W-ES-P1): a pre-split index site; this list may only shrink
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
