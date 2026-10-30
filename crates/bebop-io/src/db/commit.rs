//! THE COMMIT PATH of `Db` (W-ES-P3): a write transaction's batch becomes one delta (or a
//! compaction), the device gets it through the commit plan, readers get it as the next
//! generation's bytes patched from the previous ones. Split out of `db.rs` (the 300-line rule).

use std::sync::atomic::Ordering;
use std::sync::Arc;

use bebop_kv::delta::{append_delta_ops, Appended, KeyOp};
use bebop_kv::{Batch, Staged};

use super::{check_durability, Db, Generation, Mode, Verify};

/// The durability a compaction publishes with: a staged commit's compaction is not synced
/// (`WriteTxn::stage`), a `Normal` commit's IS (a compaction is a checkpoint: everything
/// staged is in the new image, and nothing unsynced is left behind).
pub(crate) fn compaction_durability(d: Durability, mode: Mode) -> Durability {
    match (d, mode) {
        (Durability::Normal, _) => Durability::Data,
        (_, Mode::Stage) => Durability::None,
        (d, _) => d,
    }
}
use crate::plan::plan;
use crate::refusal::Refusal;
use crate::snap::Snap;
use crate::{Device, Durability, Store, Writer};

impl<D: Device> Db<D> {
    /// The compaction both `compact` and a commit that must compact share.
    /// `tree`: write the fresh image as v4 (a v4 image's compaction, or `migrate_to_tree`), else v2.
    pub(crate) fn compact_into(&self, w: &mut Writer<D>, base: Arc<Snap>, batch: &Batch, into: D, d: Durability, tree: bool) -> Result<Generation, Refusal> {
        let gen = base.sb.generation;
        let bytes = {
            let entries = Staged::new(&base.kv, batch).entries()?;
            let it = entries.iter().map(|(k, v)| (k.as_bytes(), v.as_bytes()));
            if tree {
                bebop_kv::tree::tree_compacted(it, self.opts.max_image_bytes, self.opts.tree_fanout, Some(gen))?
            } else {
                bebop_kv::compacted_from_fit(it, self.opts.max_image_bytes, Some(gen))?
            }
        };
        self.compactions.fetch_add(1, Ordering::SeqCst);
        drop(base);
        let image = Store::from_bytes(&bytes);
        let g = match w.publish(into, image, d) {
            Ok(g) => g,
            Err(e) => {
                if w.poisoned() {
                    self.poisoned.store(true, Ordering::SeqCst);
                }
                return Err(e.into());
            }
        };
        self.durable.store(g, Ordering::SeqCst);
        self.unsynced.store(0, Ordering::SeqCst);
        let snap = Snap::build(bytes, None, Verify::Anchor)?;
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(snap));
        Ok(Generation(g))
    }

    /// THE COMMIT (`WriteTxn::commit*`/`stage`), under the writer's lock. A `Durability::Normal`
    /// commit (W-ES-P11) is staged like `WriteTxn::stage`, and the `Options::checkpoint_every`-th
    /// commit staged since the last checkpoint flushes them all as one group.
    pub(crate) fn commit_txn(&self, w: &mut Writer<D>, base: Arc<Snap>, batch: Batch, d: Durability, mode: Mode) -> Result<Generation, Refusal> {
        if mode != Mode::Stage {
            check_durability(w, d)?;
        }
        let normal = d == Durability::Normal && mode == Mode::Delta;
        let g = self.commit_inner(w, base, batch, d, if normal { Mode::Stage } else { mode })?;
        if w.durable().gen < g.get() {
            let n = self.unsynced.fetch_add(1, Ordering::SeqCst) + 1;
            let every = self.opts.checkpoint_every as u64;
            if normal && every > 0 && n >= every {
                self.checkpoint(w, Durability::Data)?;
            }
        }
        Ok(g)
    }

    fn commit_inner(&self, w: &mut Writer<D>, base: Arc<Snap>, batch: Batch, d: Durability, mode: Mode) -> Result<Generation, Refusal> {
        if batch.is_empty() && mode != Mode::Compact {
            return Ok(base.generation());
        }
        let tree = base.kv.tree_head().is_some();
        if mode == Mode::Compact {
            let into = w.device().fresh()?;
            return self.compact_into(w, base, &batch, into, d, tree);
        }
        let ops: Vec<KeyOp> = batch.ops().collect();
        if tree {
            return self.commit_tree(w, base, &batch, &ops, d, mode);
        }
        let appended = append_delta_ops(w.stage()?, &ops, self.opts.max_deltas, self.opts.dead_div)?;
        if let Appended::Compact(_) = appended {
            let into = w.device().fresh()?;
            return self.compact_into(w, base, &batch, into, compaction_durability(d, mode), false);
        }
        // The steps from the durable generation to the staged one: what the device gets, and
        // (idempotently, every staged commit since the last flush) what the snapshot gets.
        let steps = if mode == Mode::Delta { self.flush_writer(w, d)?.1 } else { plan(&w.durable(), w.store())? };
        let checked = base.kv.checked();
        drop(base);
        self.publish_patch(w, checked, &steps)
    }

    /// THE CHECKPOINT on a held writer (W-ES-P11): the group flush, then the snapshot patched
    /// with every cell the flip moved (nothing to patch when nothing was staged or no flip
    /// happened).
    pub(crate) fn checkpoint(&self, w: &mut Writer<D>, d: Durability) -> Result<i64, Refusal> {
        let (g, steps) = self.flush_writer(w, d)?;
        if !steps.is_empty() {
            let checked = self.current().kv.checked();
            self.publish_patch(w, checked, &steps)?;
        }
        Ok(g)
    }

    pub(super) fn flush_writer(&self, w: &mut Writer<D>, d: Durability) -> Result<(i64, Vec<crate::plan::Step>), Refusal> {
        match w.flush_steps(d) {
            Ok((g, steps)) => {
                self.durable.store(g, Ordering::SeqCst);
                self.unsynced.store(0, Ordering::SeqCst);
                Ok((g, steps))
            }
            Err(e) => {
                if w.poisoned() {
                    self.poisoned.store(true, Ordering::SeqCst);
                }
                Err(e.into())
            }
        }
    }

    /// Publish the writer's staged image to readers, patched from the current snapshot.
    pub(super) fn publish_patch(&self, w: &Writer<D>, checked: bebop_kv::zc::Checked, steps: &[crate::plan::Step]) -> Result<Generation, Refusal> {
        let mut cur = self.current.lock().unwrap_or_else(|e| e.into_inner());
        // Take the published Arc OUT, so that when no reader holds it the bytes are unique and
        // are patched in place (O(commit)); a reader that holds it gets a copy made instead.
        // A plan step outside the image is a bug, not data: then rebuild from the writer's cells
        // (`snapshot == to_bytes_trimmed`, db/tests.rs, pins both paths).
        let bytes = match cur.take() {
            Some(old) => Snap::patched(old, w.store(), steps).unwrap_or_else(|_| w.store().to_bytes_trimmed()),
            None => w.store().to_bytes_trimmed(),
        };
        let snap = match Snap::build(bytes, Some(checked), Verify::Anchor) {
            Ok(s) => s,
            Err(e) => {
                // Never leave the slot empty.
                let s = Snap::build(w.store().to_bytes_trimmed(), None, Verify::Anchor)?;
                *cur = Some(Arc::new(s));
                return Err(e);
            }
        };
        let g = snap.generation();
        *cur = Some(Arc::new(snap));
        Ok(g)
    }
}
