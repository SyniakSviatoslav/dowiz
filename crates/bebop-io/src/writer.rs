//! THE WRITER over a `Device` (R-EMBED-STORE §3.2-§3.6, §3.9): open = read + `recover`,
//! commits are STAGED in memory by the existing code (`Kv`, `append_delta`, `commit_bytes`;
//! the bytes are theirs), `flush` writes them with protocol v2 (`plan`), and a failed write or
//! barrier POISONS the writer: nothing more is staged or written until `reopen`, which reads
//! the device again and recovers (AC-ES-14; the lesson of ATC'20 "Can applications recover
//! from fsync failures?": after a failed fdatasync the page cache no longer says what is on disk).

use super::{cells_to_bytes, plan, Device, Durability, Durable, IoError, Step};
use crate::sim::{recover, Recovered};
use crate::Store;

/// One writer over one device. Single-threaded by construction (`&mut self`); across
/// processes the device's lock (`FileDevice`) keeps it single.
pub struct Writer<D: Device> {
    dev: D,
    store: Store,
    durable: Durable,
    recovered: Recovered,
    poisoned: bool,
}

impl<D: Device> Writer<D> {
    /// Read the device, recover (§3.6), and REPAIR before anything is written: when recovery
    /// fell back past a crc-valid superblock whose last commit is torn, that superblock's
    /// magic is zeroed ON THE DEVICE and a barrier follows. Without it the next commit would
    /// write its PartTab into the page of a superblock that is still valid on disk and newer
    /// than the one recovered, and a crash in that window would be read through it.
    pub fn open(dev: D) -> Result<Self, IoError> {
        let n = dev.len_cells()?;
        let cells = dev.read_cells(0, n)?;
        let store = Store::from_bytes(&cells_to_bytes(&cells));
        Writer::adopt(dev, store, true)
    }

    /// `open` over an image the caller already holds (W-ES-P3: `Db::from_bytes`, the Durable
    /// Object's bytes) -- the same recovery and repair, without reading `dev`. With
    /// `fallback == false` a newest generation whose last commit fails its proof is REFUSED
    /// (`IoError::Torn`) instead of being hidden: for a backend whose writes are atomic, a torn
    /// last commit is corruption, not a power cut, and falling back would drop it silently.
    pub fn adopt(mut dev: D, mut store: Store, fallback: bool) -> Result<Self, IoError> {
        let recovered = recover(&store).map_err(IoError::Recover)?;
        if let Some(h) = recovered.hidden {
            if !fallback {
                return Err(match recovered.via {
                    crate::sim::Via::Fallback { why, .. } => IoError::Torn(why),
                    _ => IoError::Plan("recovery hid a superblock without falling back"),
                });
            }
            dev.write_cells(h, &[0])?;
            dev.barrier()?;
            match store.cells.get_mut(h) {
                Some(c) => *c = 0,
                None => return Err(IoError::Plan("recovery hid a cell outside the image")),
            }
        }
        let durable = Durable::of(&store)?;
        Ok(Writer { dev, store, durable, recovered, poisoned: false })
    }

    /// The device, to ask what it can promise.
    pub fn device(&self) -> &D {
        &self.dev
    }

    /// PUBLISH A COMPACTION (W-ES-P3, `Db::compact`): write `image` whole into `staged` (a
    /// `Device::fresh` sibling), barrier it unless `d` is `None`, then `Device::publish` it over
    /// this writer's device; from then on `image` is the writer's image and its generation the
    /// durable one. A failed write before the publish leaves the old device untouched (the
    /// staged one is dropped); a failed publish poisons the writer, as a failed barrier does.
    pub fn publish(&mut self, mut staged: D, image: Store, d: Durability) -> Result<i64, IoError> {
        if self.poisoned {
            return Err(IoError::Poisoned);
        }
        let used = image.pick().map(|sb| sb.arena_used.max(crate::ARENA as i64) as usize).unwrap_or(image.cells.len());
        let cells = image.cells.get(..used.min(image.cells.len())).unwrap_or(&[]);
        staged.write_cells(0, cells)?;
        if d != Durability::None {
            staged.barrier()?;
        }
        if let Err(e) = self.dev.publish(staged) {
            self.poisoned = true;
            return Err(IoError::Io(e));
        }
        self.store = image;
        self.durable = Durable::of(&self.store)?;
        Ok(self.durable.gen)
    }

    /// The image as the writer sees it: durable generation plus anything staged since.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// The image to stage commits into. Refused once poisoned.
    pub fn stage(&mut self) -> Result<&mut Store, IoError> {
        if self.poisoned {
            return Err(IoError::Poisoned);
        }
        Ok(&mut self.store)
    }

    /// What recovery found when this writer opened.
    pub fn recovered(&self) -> &Recovered {
        &self.recovered
    }

    /// The generation the device holds (as far as this writer has written it).
    pub fn durable(&self) -> Durable {
        self.durable
    }

    /// Is the writer poisoned?
    pub fn poisoned(&self) -> bool {
        self.poisoned
    }

    /// Write everything staged since the last flush; return the generation now live.
    /// `Durability::None` writes the same cells in the same order with no barrier.
    pub fn flush(&mut self, d: Durability) -> Result<i64, IoError> {
        if self.poisoned {
            return Err(IoError::Poisoned);
        }
        let steps = plan(&self.durable, &self.store)?;
        for s in steps {
            let r = match s {
                Step::Write { off, n } => match self.store.cells.get(off..off + n) {
                    Some(cells) => self.dev.write_cells(off, cells),
                    None => return Err(IoError::Plan("a step names cells the staged image does not hold")),
                },
                Step::Barrier if d == Durability::None => Ok(()),
                Step::Barrier => self.dev.barrier(),
            };
            if let Err(e) = r {
                self.poisoned = true;
                return Err(IoError::Io(e));
            }
        }
        self.durable = Durable::of(&self.store)?;
        Ok(self.durable.gen)
    }

    /// THE DURABLE DELTA COMMIT (asked for by W-ES-P7 es-bench and P3's `Db`): stage `ops` as
    /// one KV delta (`append_delta`, the same bytes) and flush it with `d`. `Appended::Compact`
    /// is handed back with NOTHING written: the caller compacts, as with `append_delta`.
    pub fn commit_delta(&mut self, ops: &[crate::kv::delta::Op], d: Durability) -> Result<crate::kv::delta::Appended, IoError> {
        let a = crate::kv::delta::append_delta(self.stage()?, ops).map_err(IoError::Store)?;
        if let crate::kv::delta::Appended::Delta(_) = a {
            self.flush(d)?;
        }
        Ok(a)
    }

    /// One barrier over whatever `Durability::None` flushes left in the device's cache.
    pub fn sync(&mut self) -> Result<(), IoError> {
        if self.poisoned {
            return Err(IoError::Poisoned);
        }
        self.dev.barrier().map_err(|e| {
            self.poisoned = true;
            IoError::Io(e)
        })
    }

    /// Throw away everything staged since the last flush (`abort`, AC-ES-12): the device is
    /// read again, so the image is exactly what it holds.
    pub fn abort(self) -> Result<Self, IoError> {
        self.reopen()
    }

    /// Forget the in-memory image (and the poison) and open the device again.
    pub fn reopen(self) -> Result<Self, IoError> {
        Writer::open(self.dev)
    }

    /// The device back, e.g. to inspect what was written.
    pub fn into_device(self) -> D {
        self.dev
    }
}

#[cfg(test)]
mod tests;
