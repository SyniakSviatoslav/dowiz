//! THE OPEN THAT READS ONLY THE SNAPSHOT (W-ES-P10, R-EMBED-STORE §12 P10 / §4.6 B4).
//!
//! Measured 2026-10-30 (docs/measurements/B4-LAZY-OPEN-2026-10-30.md): `Db::open_path` at the
//! default depth took 30 ms for a 11.7 MB image, of which the anchor's proof was 1 us and 20 ms
//! were COPIES -- `read_cells` -> `Vec<i64>`, `Store::from_bytes` (padding the image back to its
//! arena capacity), `to_bytes_trimmed` for the snapshot -- all of them for the WRITER, which an
//! open that only reads never needs. W-ES-P8E measured the same linearity at 330 MB: 1046 ms.
//!
//! Now `Db::open`:
//!   1. reads the two superblock pages (`ARENA` cells), takes the larger `arena_used` of the valid
//!      superblocks, and reads the image to there, AS BYTES (`Device::read_bytes`): one read of
//!      the live image, no decode, no padding, no re-encode;
//!   2. runs `recover` over those bytes (pick + PartTab crc + the anchor: O(last commit)) and
//!      applies `OnTorn`: `Refuse` names the torn commit, `Fallback` repairs the device (the
//!      hidden superblock's magic written 0, barriered) and the bytes alike, trimmed to the
//!      fallback generation;
//!   3. builds the snapshot at `opts.verify` -- `Anchor` adds nothing to step 2.
//! The WRITER is built at the first write (`Lazy::ready`, from the device again, with the same
//! recovery), so a database that is only read never pays for it.
//!
//! What remains linear is the read itself (memcpy speed from the page cache); removing it is P14
//! (a mapped `SnapImage`), which this lane does not do.

use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64};
use std::sync::{Arc, Mutex, MutexGuard};

use super::{writer_of, Db, OnTorn, Options};
use crate::refusal::Refusal;
use crate::sim::{recover, Via};
use crate::snap::Snap;
use crate::{sb_valid_in, Cells, Device, View, Writer, ARENA, SB_A, SB_B};

/// The writer's slot: the device alone until the first write, then the writer over it.
pub(crate) enum Lazy<D: Device> {
    /// Recovered and repaired at open; no writer built yet.
    Pending(D),
    Ready(Writer<D>),
    /// Building the writer failed and took the device with it: every later write is `Poisoned`.
    Taken,
}

impl<D: Device> Lazy<D> {
    /// The writer, built now if it was not yet.
    pub(crate) fn ready(&mut self, opts: &Options) -> Result<&mut Writer<D>, Refusal> {
        if matches!(self, Lazy::Pending(_)) {
            let Lazy::Pending(dev) = std::mem::replace(self, Lazy::Taken) else { return Err(Refusal::Poisoned) };
            *self = Lazy::Ready(writer_of(dev, opts)?);
        }
        match self {
            Lazy::Ready(w) => Ok(w),
            _ => Err(Refusal::Poisoned),
        }
    }

    pub(crate) fn into_device(self) -> Result<D, Refusal> {
        match self {
            Lazy::Pending(d) => Ok(d),
            Lazy::Ready(w) => Ok(w.into_device()),
            Lazy::Taken => Err(Refusal::Poisoned),
        }
    }
}

/// The writer's lock, held by a `WriteTxn` or a checkpoint; always over a `Ready` writer.
pub struct WriterGuard<'a, D: Device>(pub(crate) MutexGuard<'a, Lazy<D>>);

impl<D: Device> Deref for WriterGuard<'_, D> {
    type Target = Writer<D>;
    fn deref(&self) -> &Writer<D> {
        match &*self.0 {
            Lazy::Ready(w) => w,
            // Unreachable: `Db::lock_writer` makes a guard only after `Lazy::ready` succeeded.
            _ => unreachable!("a WriterGuard is only made over a Ready writer"),
        }
    }
}

impl<D: Device> DerefMut for WriterGuard<'_, D> {
    fn deref_mut(&mut self) -> &mut Writer<D> {
        match &mut *self.0 {
            Lazy::Ready(w) => w,
            _ => unreachable!("a WriterGuard is only made over a Ready writer"),
        }
    }
}

/// The live image's bytes: the superblock pages, then the arena to the larger `arena_used` the
/// valid superblocks claim (bounded by the device). A reader past the end reads 0.
pub(crate) fn live_bytes<D: Device>(dev: &D) -> Result<Vec<u8>, Refusal> {
    let n = dev.len_cells()?;
    let head = ARENA.min(n);
    let mut bytes = dev.read_bytes(0, head)?;
    let v = View::new(&bytes);
    let mut used = head as i64;
    for at in [SB_A, SB_B] {
        if sb_valid_in(&v, at) {
            used = used.max(v.cell_at(at + 4));
        }
    }
    let used = (used.max(0) as usize).min(n);
    if used > head {
        bytes.extend(dev.read_bytes(head, used - head)?);
    }
    Ok(bytes)
}

impl<D: Device> Db<D> {
    /// THE SECTION: the writer, waited for (it is held only across a validate + replay or a
    /// flush, never across a transaction), and built now if only reads came before
    /// (W-ES-P10, `db/lazy.rs`). `Poisoned` after a failed write/barrier.
    pub(crate) fn lock_writer(&self) -> Result<WriterGuard<'_, D>, Refusal> {
        let mut guard = match self.writer.lock() {
            Ok(g) => g,
            Err(_) => {
                self.poisoned.store(true, std::sync::atomic::Ordering::SeqCst);
                return Err(Refusal::Poisoned);
            }
        };
        if self.poisoned.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Refusal::Poisoned);
        }
        let w = match guard.ready(&self.opts) {
            Ok(w) => w,
            Err(e) => {
                self.poisoned.store(true, std::sync::atomic::Ordering::SeqCst);
                return Err(e);
            }
        };
        if w.poisoned() {
            return Err(Refusal::Poisoned);
        }
        Ok(WriterGuard(guard))
    }

    /// Open the image on `dev`: recovery over the live bytes (P2's `recover`, repairing a hidden
    /// superblock on the device), then `opts.verify`. The writer is built at the first write.
    pub fn open(mut dev: D, opts: Options) -> Result<Db<D>, Refusal> {
        let mut bytes = live_bytes(&dev)?;
        let rec = recover(&View::new(&bytes))?;
        if let Some(h) = rec.hidden {
            let why = match rec.via {
                Via::Fallback { why, .. } => why,
                _ => return Err(Refusal::Plan("recovery hid a superblock without falling back")),
            };
            if opts.on_torn == OnTorn::Refuse {
                return Err(Refusal::LastCommit(why));
            }
            // The repair `Writer::adopt` makes, made here so a writer built later recovers the
            // same generation this snapshot shows, and every other process sees it too.
            dev.write_cells(h, &[0])?;
            dev.barrier()?;
            match bytes.get_mut(h * 8..h * 8 + 8) {
                Some(cell) => cell.fill(0),
                None => return Err(Refusal::Plan("recovery hid a cell outside the image")),
            }
        }
        // `Store::to_bytes_trimmed`'s length: the recovered generation's `arena_used`.
        let trim = (rec.sb.arena_used.max(ARENA as i64) as usize).min(bytes.len() / 8) * 8;
        bytes.truncate(trim);
        let snap = Snap::build(bytes, None, opts.verify)?;
        Ok(Db {
            writer: Mutex::new(Lazy::Pending(dev)),
            log: Mutex::new(std::collections::VecDeque::new()),
            current: Mutex::new(Some(Arc::new(snap))),
            opts,
            durable: AtomicI64::new(rec.sb.generation),
            poisoned: AtomicBool::new(false),
            compactions: AtomicU64::new(0),
            unsynced: AtomicU64::new(0),
        })
    }
}
