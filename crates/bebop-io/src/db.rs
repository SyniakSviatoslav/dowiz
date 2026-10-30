//! `Db`: the library's KV database over a `Device` (W-ES-P3, R-EMBED-STORE §3.2-§3.5).
//!
//! ```text
//! Db<D>    one writer (Mutex<Writer<D>>, try_lock: a second begin_write is WriterBusy at once)
//!          + the published generation (Mutex<Arc<Snap>>: begin_read clones the Arc, O(1))
//! ReadTxn  a snapshot: one generation's bytes, never changed under it (AC-ES-10)
//! WriteTxn the writer's lock + the generation it started from + a Batch (read-your-writes)
//! commit   Batch -> append_delta_ops (the same bytes as dowiz's appends) -> Writer::flush
//!          (protocol v2, P2) -> the next generation's bytes patched from the previous ones
//!          -> published; or, when the delta writer says so, a compaction into a fresh device
//!          (generation + 1, never back to 1) published by Device::publish
//! ```
//!
//! WHAT IS NOT HERE: multi-process readers (Q4), multi-writer (§6: rejected), an mmap read path
//! (P14: `SnapImage` gains a mapped backing, the API stays).

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use bebop_kv::delta::KeyOp;
use bebop_kv::zc::KvIn;
use bebop_kv::{Batch, Kv};

use crate::refusal::Refusal;
use crate::sim::{recover, Via};
use crate::snap::Snap;
use crate::txn::{ReadTxn, WriteTxn};
use crate::{cells_to_bytes, Device, Durability, Store, Writer, ARENA};

mod adopt;
mod commit;
mod tree;
mod options;
pub use options::{Generation, OnTorn, Options, Report, Verify};

/// How a commit writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// A delta, compacting when the delta writer says so; flushed with the durability asked.
    Delta,
    /// A whole v2 rewrite, always.
    Compact,
    /// A delta staged in memory and published to readers, NOT flushed (P11: `Db::flush`).
    Stage,
}

/// One KV database over one device.
pub struct Db<D: Device> {
    writer: Mutex<Writer<D>>,
    /// The published generation. `None` only for the instant a commit holds the lock and
    /// patches the bytes in place (nobody can observe it: the lock is held throughout).
    current: Mutex<Option<Arc<Snap>>>,
    opts: Options,
    durable: AtomicI64,
    poisoned: AtomicBool,
    /// Compactions since open (W-ES-P8C: es-bench reports it next to every write row).
    compactions: AtomicU64,
    /// `Durability::Normal` commits staged since the last checkpoint (W-ES-P11).
    unsynced: AtomicU64,
}

/// Read `dev` whole and recover it as `opts.on_torn` says.
fn writer_of<D: Device>(dev: D, opts: &Options) -> Result<Writer<D>, Refusal> {
    let n = dev.len_cells()?;
    let cells = dev.read_cells(0, n)?;
    let store = Store::from_bytes(&cells_to_bytes(&cells));
    Ok(Writer::adopt(dev, store, opts.on_torn == OnTorn::Fallback)?)
}

impl<D: Device> Db<D> {
    /// Open the image on `dev`: recovery (P2's `recover`, repairing a hidden superblock on the
    /// device), then `opts.verify`.
    pub fn open(dev: D, opts: Options) -> Result<Db<D>, Refusal> {
        let w = writer_of(dev, &opts)?;
        Db::over(w, opts)
    }

    /// Make an empty KV database on `dev`: an empty device is formatted (`opts.initial_bytes`), a
    /// formatted one with no KV root gets the empty schema; one that already holds a KV image
    /// is `Refusal::Exists`. Written with the most durability the device has.
    pub fn create(mut dev: D, opts: Options) -> Result<Db<D>, Refusal> {
        let best = dev.durability();
        let mut w = if dev.len_cells()? == 0 {
            let fresh = Store::create_bytes(opts.initial_bytes)?;
            dev.write_cells(0, &fresh.cells)?;
            if best > Durability::None {
                dev.barrier()?;
            }
            Writer::adopt(dev, fresh, true)?
        } else {
            writer_of(dev, &opts)?
        };
        if w.store().root().is_some() {
            return Err(Refusal::Exists);
        }
        if opts.create_tree {
            w.stage_tree(&[], opts.tree_fanout)?;
        } else {
            Kv::init_bytes(w.stage()?)?;
        }
        w.flush(best)?;
        Db::over(w, opts)
    }

    fn over(w: Writer<D>, opts: Options) -> Result<Db<D>, Refusal> {
        let snap = Snap::build(w.store().to_bytes_trimmed(), None, opts.verify)?;
        let durable = w.durable().gen;
        Ok(Db {
            writer: Mutex::new(w),
            current: Mutex::new(Some(Arc::new(snap))),
            opts,
            durable: AtomicI64::new(durable),
            poisoned: AtomicBool::new(false),
            compactions: AtomicU64::new(0),
            unsynced: AtomicU64::new(0),
        })
    }

    /// The options this database was opened with.
    pub fn options(&self) -> Options {
        self.opts
    }

    fn current(&self) -> Arc<Snap> {
        let cur = self.current.lock().unwrap_or_else(|e| e.into_inner());
        match &*cur {
            Some(s) => Arc::clone(s),
            // Unreachable: `publish_patch` puts a snapshot back before it lets go of the lock.
            None => unreachable!("the published generation is always present outside a commit"),
        }
    }

    /// A snapshot of the published generation. Never refused: a poisoned writer does not stop
    /// reads of what was published before it.
    pub fn begin_read(&self) -> ReadTxn<'_> {
        ReadTxn::new(self.current())
    }

    /// The one write transaction. `WriterBusy` at once if one is open (no waiting: waiting is
    /// the application's business); `Poisoned` after a failed write/barrier or a panic in a txn.
    pub fn begin_write(&self) -> Result<WriteTxn<'_, D>, Refusal> {
        self.begin_write_with(Batch::new(self.opts.limits))
    }

    /// `begin_write` carrying writes made earlier (`WriteTxn::into_batch`, or a holder's own
    /// pending `Batch`). Every op is checked against this database's limits first.
    pub fn begin_write_with(&self, batch: Batch) -> Result<WriteTxn<'_, D>, Refusal> {
        let guard = self.lock_writer()?;
        for op in batch.ops() {
            match op {
                KeyOp::Put(k, v) => {
                    self.opts.limits.check_key(k.len())?;
                    self.opts.limits.check_value(v.len())?;
                }
                KeyOp::Remove(k) => self.opts.limits.check_key(k.len())?,
            }
        }
        Ok(WriteTxn::new(self, guard, self.current(), batch))
    }

    fn lock_writer(&self) -> Result<MutexGuard<'_, Writer<D>>, Refusal> {
        let guard = match self.writer.try_lock() {
            Ok(g) => g,
            Err(TryLockError::WouldBlock) => return Err(Refusal::WriterBusy),
            Err(TryLockError::Poisoned(_)) => {
                self.poisoned.store(true, Ordering::SeqCst);
                return Err(Refusal::Poisoned);
            }
        };
        if guard.poisoned() || self.poisoned.load(Ordering::SeqCst) {
            return Err(Refusal::Poisoned);
        }
        Ok(guard)
    }

    /// THE CHECKPOINT (W-ES-P11): flush every commit `WriteTxn::stage` or a `Normal` commit
    /// left in memory as ONE group under two barriers (`Writer::flush`'s flip), and re-publish
    /// the snapshot where the flip moved a superblock page in the writer's image.
    pub fn flush(&self, d: Durability) -> Result<Generation, Refusal> {
        let mut w = self.lock_writer()?;
        check_durability(&*w, d)?;
        Ok(Generation(self.checkpoint(&mut w, d)?))
    }

    /// Commits staged (`Durability::Normal`, `WriteTxn::stage`) since the last checkpoint.
    pub fn unsynced(&self) -> u64 {
        self.unsynced.load(Ordering::SeqCst)
    }

    /// The generation the device holds, as far as this database has written it.
    pub fn durable(&self) -> Generation {
        Generation(self.durable.load(Ordering::SeqCst))
    }

    /// COMPACT into `into` (a new, empty device) and publish it over this one
    /// (`Device::publish`: for a file, rename): the live entries written once, v2, generation
    /// + 1. Readers holding the old generation keep reading it (AC-ES-15).
    pub fn compact(&self, into: D) -> Result<Generation, Refusal> {
        let mut w = self.lock_writer()?;
        let base = self.current();
        let d = w.device().durability();
        let tree = base.kv.tree_head().is_some();
        self.compact_into(&mut w, base, Batch::empty(), into, d, tree)
    }

    /// `compact` into the device's own fresh sibling (`Device::fresh`; `FileDevice`:
    /// `<path>.compact`, published by rename over the store file).
    pub fn compact_fresh(&self) -> Result<Generation, Refusal> {
        let mut w = self.lock_writer()?;
        let base = self.current();
        let d = w.device().durability();
        let into = w.device().fresh()?;
        let tree = base.kv.tree_head().is_some();
        self.compact_into(&mut w, base, Batch::empty(), into, d, tree)
    }

    /// The writer's own image, trimmed: what the published snapshot must equal after every
    /// commit (the invariant `db/tests.rs` checks).
    #[cfg(test)]
    pub(crate) fn writer_image(&self) -> Vec<u8> {
        self.writer.lock().unwrap_or_else(|e| e.into_inner()).store().to_bytes_trimmed()
    }

    /// Prove the published generation to `depth`. Readers are not stopped.
    pub fn verify(&self, depth: Verify) -> Result<Report, Refusal> {
        let snap = self.current();
        let rec = recover(&snap.img)?;
        if let (Some(_), Via::Fallback { why, .. }) = (rec.hidden, rec.via) {
            return Err(Refusal::LastCommit(why));
        }
        let mut objects = 0;
        if depth != Verify::Anchor {
            KvIn::open(snap.img.clone())?;
        }
        if depth == Verify::WholeArena {
            objects = crate::sim::walk(&snap.img, ARENA, snap.sb.arena_used.max(0) as usize).map_err(|at| Refusal::ObjectBad { at })?;
        }
        Ok(Report { generation: snap.generation(), depth, via: rec.via, objects })
    }

    /// The published generation's image, trimmed: `Store::to_bytes_trimmed`'s bytes.
    pub fn snapshot_bytes(&self) -> Vec<u8> {
        self.current().bytes().to_vec()
    }

    /// Has a write failed (or a transaction panicked) since this database was opened?
    pub fn poisoned(&self) -> bool {
        self.poisoned.load(Ordering::SeqCst)
    }

    /// Forget the poison and open again: a device is read again; a `Detached` one (it holds
    /// nothing) is re-adopted from the last published generation.
    pub fn reopen(self) -> Result<Db<D>, Refusal> {
        let opts = self.opts;
        let published = self.snapshot_bytes();
        let w = self.writer.into_inner().unwrap_or_else(|e| e.into_inner());
        let dev = w.into_device();
        let w = if dev.len_cells()? == 0 {
            Writer::adopt(dev, Store::from_bytes(&published), opts.on_torn == OnTorn::Fallback)?
        } else {
            writer_of(dev, &opts)?
        };
        Db::over(w, opts)
    }

    /// The device back.
    pub fn into_device(self) -> D {
        self.writer.into_inner().unwrap_or_else(|e| e.into_inner()).into_device()
    }
}

pub(crate) use commit::compaction_durability;

pub(crate) fn check_durability<D: Device>(w: &Writer<D>, d: Durability) -> Result<(), Refusal> {
    let device = w.device().durability();
    if d > device {
        return Err(Refusal::Durability { asked: d, device });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod normal_tests;
#[cfg(all(test, feature = "tree"))]
mod tree_tests;
