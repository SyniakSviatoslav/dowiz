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
    /// The durable superblock page as the device holds it (16 + 23 cells: superblock and
    /// PartTab), kept so a group flush can put it back in memory after the flip (`flip`).
    page: Vec<i64>,
}

/// The cells of one superblock page the writer keeps: superblock + PartTab object.
const PAGE_CELLS: usize = crate::format::SB_CELLS + crate::format::PARTTAB_OBJ_CELLS;

fn page_of(st: &Store, at: usize) -> Vec<i64> {
    (at..at + PAGE_CELLS).map(|i| st.cells.get(i).copied().unwrap_or(0)).collect()
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
        let page = page_of(&store, durable.at);
        Ok(Writer { dev, store, durable, recovered, poisoned: false, page })
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
    /// At `None` the device publishes UNSYNCED (W-ES-HARD: `FileDevice` defers the fdatasync and
    /// the directory fsync to the next barrier; `sim::crash` names what a power loss can leave).
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
        self.page = page_of(&self.store, self.durable.at);
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
    /// `Durability::None` writes the same cells in the same order with no barrier; `Normal`
    /// is `Data` here (the batching is `Db`'s).
    pub fn flush(&mut self, d: Durability) -> Result<i64, IoError> {
        self.flush_steps(d).map(|(g, _)| g)
    }

    /// `flush`, and every cell of the in-memory image it changed, as steps: the plan's writes
    /// and, before them, the flip's (W-ES-P11). `Db::flush` patches the published snapshot
    /// with them so it stays the writer's image byte for byte.
    pub fn flush_steps(&mut self, d: Durability) -> Result<(i64, Vec<Step>), IoError> {
        if self.poisoned {
            return Err(IoError::Poisoned);
        }
        // The flip's steps are in-memory only; the plan's reach the device, in order.
        let mut steps = self.flip()?;
        let planned = plan(&self.durable, &self.store)?;
        for s in &planned {
            let r = match *s {
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
        steps.extend(planned);
        self.durable = Durable::of(&self.store)?;
        self.page = page_of(&self.store, self.durable.at);
        Ok((self.durable.gen, steps))
    }

    /// THE FLIP (W-ES-P11, the group commit under exactly two barriers). Staged commits
    /// alternate superblock pages, so after an EVEN number the live staged generation sits
    /// in memory ON the durable page; P2's plan then published the inner generation first
    /// (four barriers). Instead the live superblock and its PartTab are moved, in memory, to
    /// the other page (cell `SB_PARTTAB` re-pointed, crc15 recomputed; the anchor covers the
    /// arena only, FORMAT §7) and the durable page is put back as the device holds it (`page`).
    /// The image is then "k on the durable page, k+N opposite", exactly an odd group, and the
    /// generations between never reach the device. Returns the two pages as steps (in-memory
    /// changes, for the snapshot patch); empty when no flip was needed.
    fn flip(&mut self) -> Result<Vec<Step>, IoError> {
        use crate::format::{SB_CRC, SB_PARTTAB, PT_PAGE_OFFSET};
        let live = self.store.pick().ok_or(IoError::Plan("the staged image has no live superblock"))?;
        if live.at != self.durable.at || live.generation <= self.durable.gen {
            return Ok(Vec::new());
        }
        let (at, other) = (live.at, crate::SB_B - live.at);
        let moved = page_of(&self.store, at);
        let cells = &mut self.store.cells;
        if cells.len() < other + PAGE_CELLS || self.page.len() != PAGE_CELLS {
            return Err(IoError::Plan("the staged image has no room for the flipped superblock page"));
        }
        cells[other..other + PAGE_CELLS].copy_from_slice(&moved);
        cells[other + SB_PARTTAB] = (other + PT_PAGE_OFFSET) as i64;
        cells[other + SB_CRC] = crate::crc32_cells(cells, other, SB_CRC) as i64;
        cells[at..at + PAGE_CELLS].copy_from_slice(&self.page);
        if !self.store.sb_valid(other) || self.store.pick().map(|s| (s.at, s.generation)) != Some((other, live.generation)) {
            return Err(IoError::Plan("the flipped superblock is not the live one"));
        }
        Ok(vec![Step::Write { off: other, n: PAGE_CELLS }, Step::Write { off: at, n: PAGE_CELLS }])
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

    /// STAGE ONE v4 TREE COMMIT (W-ES-P8C): `ops` (strictly ascending, one per key: `Batch::ops`)
    /// path-copied into the image's tree (`kv::tree::stage_tree`, the bytes are its) and committed
    /// in memory; nothing is written. Returns the staged generation. A v1-v3 image is refused
    /// (`WriteError::NotTree`): it moves to v4 only through `Db::migrate_to_tree`.
    pub fn stage_tree(&mut self, ops: &[crate::kv::delta::KeyOp], fanout: usize) -> Result<i64, IoError> {
        if !crate::kv::tree::v4_here() {
            return Err(IoError::Plan("KV v4 needs bebop-io's `tree` feature"));
        }
        let st = self.stage()?;
        let (tx, root) = crate::kv::tree::stage_tree(st, ops, fanout).map_err(IoError::Tree)?;
        Ok(st.commit_bytes(&tx, root))
    }

    /// THE DURABLE TREE COMMIT: `stage_tree`, then `flush(d)`.
    pub fn commit_tree(&mut self, ops: &[crate::kv::delta::KeyOp], fanout: usize, d: Durability) -> Result<i64, IoError> {
        self.stage_tree(ops, fanout)?;
        self.flush(d)
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
