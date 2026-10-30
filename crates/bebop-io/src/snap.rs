//! One published generation, as readers hold it (W-ES-P3, design D1).
//!
//! `Store` is `Vec<i64>` and can never lend `&[u8]` without `unsafe` (forbidden here), so a
//! generation is ALSO kept as its trimmed byte image -- exactly `Store::to_bytes_trimmed` -- in an
//! `Arc`. A `ReadTxn` holds the `Arc<Snap>`; a value it returns is a slice of these bytes. A
//! commit builds the next generation's bytes by PATCHING the previous ones with the cells the
//! commit plan writes (`Snap::patched`), in place when no reader still holds them, on a copy
//! when one does: a reader's bytes never change under it (AC-ES-10).

use std::sync::Arc;

use bebop_kv::zc::{Checked, Image, KvIn};

use crate::db::{Generation, Verify};
use crate::plan::Step;
use crate::refusal::Refusal;
use crate::{Cells, Store, Superblock, View, ARENA};

/// A generation's bytes, shared.
#[derive(Clone)]
pub struct SnapImage(pub(crate) Arc<Vec<u8>>);

impl Cells for SnapImage {
    fn cell_at(&self, i: usize) -> i64 {
        match i.checked_mul(8).and_then(|a| Some((a, a.checked_add(8)?))).and_then(|(a, e)| self.0.get(a..e)) {
            Some(w) => i64::from_le_bytes([w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]]),
            None => 0,
        }
    }
    fn n_cells(&self) -> usize {
        self.0.len() / 8
    }
}

impl Image for SnapImage {
    fn crc_cells(&self, off: usize, n: usize) -> u32 {
        View::new(&self.0).crc_cells(off, n)
    }
    fn bytes_at(&self, at: usize, len: usize) -> Option<&[u8]> {
        self.0.get(at..at.checked_add(len)?)
    }
}

/// A published generation.
pub(crate) struct Snap {
    pub(crate) kv: KvIn<SnapImage>,
    pub(crate) img: SnapImage,
    pub(crate) sb: Superblock,
    pub(crate) capacity: i64,
}

impl Snap {
    /// Read `bytes` as a KV generation. `checked` = these bytes extend ones already opened (a
    /// commit of ours): no crc, the key order carried over. Otherwise `depth` says how much is
    /// proved here: `Anchor` trusts what recovery proved (`KvIn::open_lazy`), `Reachable` checks
    /// every object the root reaches, `WholeArena` also walks every object of the arena.
    pub(crate) fn build(bytes: Vec<u8>, checked: Option<Checked>, depth: Verify) -> Result<Snap, Refusal> {
        let img = SnapImage(Arc::new(bytes));
        let sb = crate::format::pick_checked_in(&img)?;
        let root = crate::root_in(&img).ok_or(Refusal::NotKv)?;
        let ver = bebop_kv::delta::version_in(&img, root);
        let v4 = ver == bebop_kv::tree::VERSION_TREE && bebop_kv::tree::v4_here();
        if ver > bebop_kv::delta::VERSION_DELTA && !v4 {
            return Err(Refusal::UnknownVersion(ver));
        }
        let kv = match (checked, depth) {
            (Some(ck), _) => KvIn::reopen(img.clone(), ck),
            (None, Verify::Anchor) => KvIn::open_lazy(img.clone()),
            (None, _) => KvIn::open(img.clone()),
        }
        .map_err(Refusal::from)?;
        if checked.is_none() && depth == Verify::WholeArena {
            crate::sim::walk(&img, ARENA, sb.arena_used.max(0) as usize).map_err(|at| Refusal::ObjectBad { at })?;
        }
        let capacity = img.cell_at(sb.at + 12);
        Ok(Snap { kv, img, sb, capacity })
    }

    pub(crate) fn generation(&self) -> Generation {
        Generation(self.sb.generation)
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.img.0
    }

    /// The bytes of the generation `st` stages, from `old`'s by the commit plan's writes:
    /// resized to `st`'s trimmed length, then every `Step::Write` range copied from `st`. Takes
    /// `old` by value so that, when no reader holds it, the bytes are patched in place.
    pub(crate) fn patched(old: Arc<Snap>, st: &Store, steps: &[Step]) -> Result<Vec<u8>, Refusal> {
        let mut bytes = match Arc::try_unwrap(old) {
            Ok(snap) => {
                let Snap { kv, img, .. } = snap;
                drop(kv);
                Arc::try_unwrap(img.0).unwrap_or_else(|shared| shared.as_ref().clone())
            }
            Err(shared) => shared.img.0.as_ref().clone(),
        };
        let trim = trimmed_cells(st);
        bytes.resize(trim * 8, 0);
        for s in steps {
            if let Step::Write { off, n } = *s {
                let end = off.checked_add(n).ok_or(Refusal::Plan("a step's range overflows"))?;
                let src = st.cells.get(off..end).ok_or(Refusal::Plan("a step names cells the staged image does not hold"))?;
                let dst = bytes
                    .get_mut(off * 8..end * 8)
                    .ok_or(Refusal::Plan("a step writes past the generation's trimmed image"))?;
                for (d, c) in dst.chunks_exact_mut(8).zip(src) {
                    d.copy_from_slice(&c.to_le_bytes());
                }
            }
        }
        Ok(bytes)
    }
}

/// `Store::to_bytes_trimmed`'s length, in cells.
pub(crate) fn trimmed_cells(st: &Store) -> usize {
    let at = match st.pick() {
        Some(sb) => sb.arena_used.max(ARENA as i64) as usize,
        None => st.cells.len(),
    };
    at.min(st.cells.len())
}
