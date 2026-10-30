//! `Db` over a v4 image (W-ES-P8C): the tree commit, and the explicit migration v1-v3 -> v4.
//!
//! A commit on a v4 image is ONE path copy (`Writer::stage_tree`), flushed through the commit
//! plan like a delta and published to readers by the same patch. Compaction is the copy
//! compaction (`kv::tree::tree_compacted`), and it runs only when
//!   * the image's superseded cells pass its live cells / `Options::tree_dead_div` (checked
//!     BEFORE staging, so a commit that triggers it is written by the compaction itself), or
//!   * the arena cannot hold the path copy (`ArenaFull`), or
//!   * the holder asks (`Db::compact*`, `WriteTxn::commit_compacted`).
//! Never on a commit count: v3's 32-delta rule does not apply to a tree.
//!
//! MIGRATION is never automatic: `open` reads whatever version the image says and writes it in
//! that version. `migrate_to_tree` is a compaction whose output is v4.

use std::sync::Arc;

use bebop_kv::delta::KeyOp;
use bebop_kv::tree::{dead_over, WriteError};
use bebop_kv::Batch;

use super::{Db, Generation, Mode};
use crate::plan::plan;
use crate::refusal::Refusal;
use crate::snap::Snap;
use crate::{Device, Durability, IoError, StoreError, Writer};

impl<D: Device> Db<D> {
    /// THE v4 COMMIT (`commit_txn` on a v4 base), under the writer's lock.
    pub(super) fn commit_tree(&self, w: &mut Writer<D>, base: Arc<Snap>, batch: &Batch, ops: &[KeyOp], d: Durability, mode: Mode) -> Result<Generation, Refusal> {
        let cd = super::compaction_durability(d, mode);
        if dead_over(w.store(), self.opts.tree_dead_div) {
            let into = w.device().fresh()?;
            return self.compact_into(w, base, batch, into, cd, true);
        }
        match w.stage_tree(ops, self.opts.tree_fanout) {
            Ok(_) => {}
            Err(IoError::Tree(WriteError::Store(StoreError::ArenaFull { .. }))) => {
                // Nothing was committed: the superblock still names the base generation.
                let into = w.device().fresh()?;
                return self.compact_into(w, base, batch, into, cd, true);
            }
            Err(e) => return Err(e.into()),
        }
        let steps = if mode == Mode::Delta { self.flush_writer(w, d)?.1 } else { plan(&w.durable(), w.store())? };
        let checked = base.kv.checked();
        drop(base);
        self.publish_patch(w, checked, &steps)
    }

    /// MIGRATE to v4 (explicit, never on open): the live entries bulk-loaded as a v4 tree into
    /// `into`, generation + 1, published over this device as a compaction is. A v4 image is
    /// compacted (the same call). Readers holding the old generation keep reading it.
    pub fn migrate_to_tree(&self, into: D) -> Result<Generation, Refusal> {
        // Refused BEFORE anything is published: a build that cannot read v4 must never write it.
        if !bebop_kv::tree::v4_here() {
            return Err(Refusal::UnknownVersion(bebop_kv::tree::VERSION_TREE));
        }
        let mut w = self.lock_writer()?;
        let base = self.current();
        let d = w.device().durability();
        self.compact_into(&mut w, base, Batch::empty(), into, d, true)
    }

    /// `migrate_to_tree` into the device's own fresh sibling (`FileDevice`: `<path>.compact`).
    pub fn migrate_to_tree_fresh(&self) -> Result<Generation, Refusal> {
        let into = self.lock_writer()?.device().fresh()?;
        self.migrate_to_tree(into)
    }

    /// Compactions this handle made since open (es-bench prints it next to every write row).
    pub fn compactions(&self) -> u64 {
        self.compactions.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Is the published generation a v4 tree?
    pub fn is_tree(&self) -> bool {
        self.current().kv.tree_head().is_some()
    }
}
