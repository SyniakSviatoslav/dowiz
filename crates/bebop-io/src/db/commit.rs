//! THE COMMIT PATH of `Db` (W-ES-P3): a write transaction's batch becomes one delta (or a
//! compaction), the device gets it through the commit plan, readers get it as the next
//! generation's bytes patched from the previous ones. Split out of `db.rs` (the 300-line rule).

use std::sync::atomic::Ordering;
use std::sync::Arc;

use bebop_kv::delta::{append_delta_ops, Appended, KeyOp};
use bebop_kv::{Batch, Staged};

use super::{check_durability, Db, Generation, Mode, Verify};
use crate::plan::plan;
use crate::refusal::Refusal;
use crate::snap::Snap;
use crate::{Device, Durability, Store, Writer};

impl<D: Device> Db<D> {
    /// The compaction both `compact` and a commit that must compact share.
    pub(crate) fn compact_into(&self, w: &mut Writer<D>, base: Arc<Snap>, batch: &Batch, into: D, d: Durability) -> Result<Generation, Refusal> {
        let gen = base.sb.generation;
        let bytes = {
            let entries = Staged::new(&base.kv, batch).entries()?;
            bebop_kv::compacted_from_fit(entries.iter().map(|(k, v)| (k.as_bytes(), v.as_bytes())), self.opts.max_image_bytes, Some(gen))?
        };
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
        let snap = Snap::build(bytes, None, Verify::Anchor)?;
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(snap));
        Ok(Generation(g))
    }

    /// THE COMMIT (`WriteTxn::commit*`/`stage`), under the writer's lock.
    pub(crate) fn commit_txn(&self, w: &mut Writer<D>, base: Arc<Snap>, batch: Batch, d: Durability, mode: Mode) -> Result<Generation, Refusal> {
        if mode != Mode::Stage {
            check_durability(w, d)?;
        }
        if batch.is_empty() && mode != Mode::Compact {
            return Ok(base.generation());
        }
        if mode == Mode::Compact {
            let into = w.device().fresh()?;
            return self.compact_into(w, base, &batch, into, if mode == Mode::Stage { Durability::None } else { d });
        }
        let ops: Vec<KeyOp> = batch.ops().collect();
        let appended = append_delta_ops(w.stage()?, &ops, self.opts.max_deltas, self.opts.dead_div)?;
        if let Appended::Compact(_) = appended {
            let into = w.device().fresh()?;
            let d = if mode == Mode::Stage { Durability::None } else { d };
            return self.compact_into(w, base, &batch, into, d);
        }
        // The steps from the durable generation to the staged one: what the device gets, and
        // (idempotently, every staged commit since the last flush) what the snapshot gets.
        let steps = plan(&w.durable(), w.store())?;
        if mode == Mode::Delta {
            self.flush_writer(w, d)?;
        }
        let checked = base.kv.checked();
        drop(base);
        self.publish_patch(w, checked, &steps)
    }

    /// Publish the writer's staged image to readers, patched from the current snapshot.
    fn publish_patch(&self, w: &Writer<D>, checked: bebop_kv::zc::Checked, steps: &[crate::plan::Step]) -> Result<Generation, Refusal> {
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
