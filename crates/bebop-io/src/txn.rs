//! `ReadTxn`, `WriteTxn`, `Scan` (W-ES-P3, R-EMBED-STORE §3.2-§3.4).
//!
//! A `ReadTxn` is SNAPSHOT isolation of one generation: it holds that generation's bytes and
//! nothing a later commit does changes them (AC-ES-10). A `WriteTxn` is the only writer
//! (Serializable, trivially): it reads its own writes through its `Batch` (the batch first,
//! then the generation it started from) and touches the image only at commit. So `abort` costs
//! nothing and changes no byte (AC-ES-12): there is nothing to undo.

use std::marker::PhantomData;
use std::ops::Bound;
use std::sync::{Arc, MutexGuard};

use bebop_kv::zc::KvIn;
use bebop_kv::{Batch, Bytes, MergeCursor, Staged};

use crate::db::{Db, Generation, Mode};
use crate::refusal::Refusal;
use crate::snap::{Snap, SnapImage};
use crate::{Device, Durability, Superblock, Writer};

/// The bound just past every key that starts with `p`: `p` with its last byte below 0xFF
/// incremented and the rest cut, or no bound when `p` is all 0xFF (or empty).
fn prefix_end(p: &[u8]) -> Bound<Vec<u8>> {
    let mut end = p.to_vec();
    while let Some(last) = end.pop() {
        if last < 0xFF {
            end.push(last + 1);
            return Bound::Excluded(end);
        }
    }
    Bound::Unbounded
}

fn as_ref(b: &Bound<Vec<u8>>) -> Bound<&[u8]> {
    match b {
        Bound::Included(k) => Bound::Included(k.as_slice()),
        Bound::Excluded(k) => Bound::Excluded(k.as_slice()),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// A scan over a snapshot (and a write transaction's batch), in key order.
pub struct Scan<'t>(MergeCursor<'t, SnapImage>);

impl<'t> Iterator for Scan<'t> {
    type Item = Result<(Bytes<'t>, Bytes<'t>), Refusal>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|r| r.map_err(Refusal::from))
    }
}

fn scan_of<'a>(kv: &'a KvIn<SnapImage>, batch: &'a Batch, lo: Bound<&[u8]>, hi: Bound<&[u8]>) -> Result<Scan<'a>, Refusal> {
    Ok(Scan(Staged::new(kv, batch).cursor(lo, hi)?))
}

fn prefix_of<'a>(kv: &'a KvIn<SnapImage>, batch: &'a Batch, p: &[u8]) -> Result<Scan<'a>, Refusal> {
    let end = prefix_end(p);
    scan_of(kv, batch, Bound::Included(p), as_ref(&end))
}

/// A snapshot of one generation.
pub struct ReadTxn<'db> {
    snap: Arc<Snap>,
    _db: PhantomData<&'db ()>,
}

impl<'db> ReadTxn<'db> {
    pub(crate) fn new(snap: Arc<Snap>) -> ReadTxn<'db> {
        ReadTxn { snap, _db: PhantomData }
    }

    /// The value under `key`, read in place for a v2/v3 image (`Bytes::is_borrowed`).
    pub fn get(&self, key: &[u8]) -> Result<Option<Bytes<'_>>, Refusal> {
        Ok(self.snap.kv.get(key)?.map(Bytes::from))
    }

    pub fn contains(&self, key: &[u8]) -> Result<bool, Refusal> {
        Ok(self.snap.kv.get(key)?.is_some())
    }

    /// The entries between the bounds, in key order.
    pub fn scan(&self, lo: Bound<&[u8]>, hi: Bound<&[u8]>) -> Result<Scan<'_>, Refusal> {
        scan_of(&self.snap.kv, Batch::empty(), lo, hi)
    }

    /// The entries whose key starts with `p`, in key order.
    pub fn prefix(&self, p: &[u8]) -> Result<Scan<'_>, Refusal> {
        prefix_of(&self.snap.kv, Batch::empty(), p)
    }

    pub fn len(&self) -> usize {
        self.snap.kv.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snap.kv.is_empty()
    }

    pub fn generation(&self) -> Generation {
        self.snap.generation()
    }

    /// FNV-1a 64 over the entries: `Kv::snapshot_root_u64`, the root dowiz publishes.
    pub fn fold_root(&self) -> Result<u64, Refusal> {
        Ok(self.snap.kv.snapshot_root_u64()?)
    }

    pub fn superblock(&self) -> Superblock {
        self.snap.sb
    }

    /// The arena's capacity in cells, as the live superblock says (cell 12).
    pub fn capacity_cells(&self) -> i64 {
        self.snap.capacity
    }

    /// This generation's trimmed image: `Store::to_bytes_trimmed`'s bytes, borrowed.
    pub fn image(&self) -> &[u8] {
        self.snap.bytes()
    }

    /// This snapshot with `batch`'s writes on top: read-your-writes for a holder that keeps
    /// its pending writes itself (dowiz-hub's `Catalog`), with no writer's lock taken.
    pub fn staged<'a>(&'a self, batch: &'a Batch) -> Staged<'a, SnapImage> {
        Staged::new(&self.snap.kv, batch)
    }
}

/// The one write transaction.
pub struct WriteTxn<'db, D: Device> {
    db: &'db Db<D>,
    writer: MutexGuard<'db, Writer<D>>,
    base: Arc<Snap>,
    batch: Batch,
}

impl<'db, D: Device> WriteTxn<'db, D> {
    pub(crate) fn new(db: &'db Db<D>, writer: MutexGuard<'db, Writer<D>>, base: Arc<Snap>, batch: Batch) -> WriteTxn<'db, D> {
        WriteTxn { db, writer, base, batch }
    }

    fn staged(&self) -> Staged<'_, SnapImage> {
        Staged::new(&self.base.kv, &self.batch)
    }

    /// The value under `key` as this transaction sees it: its own writes first (AC-ES-10).
    pub fn get(&self, key: &[u8]) -> Result<Option<Bytes<'_>>, Refusal> {
        Ok(self.staged().get(key)?)
    }

    pub fn contains(&self, key: &[u8]) -> Result<bool, Refusal> {
        Ok(self.staged().contains(key)?)
    }

    /// The entries between the bounds as this transaction sees them, in key order.
    pub fn scan(&self, lo: Bound<&[u8]>, hi: Bound<&[u8]>) -> Result<Scan<'_>, Refusal> {
        scan_of(&self.base.kv, &self.batch, lo, hi)
    }

    pub fn prefix(&self, p: &[u8]) -> Result<Scan<'_>, Refusal> {
        prefix_of(&self.base.kv, &self.batch, p)
    }

    /// Entries this transaction sees.
    pub fn len(&self) -> Result<usize, Refusal> {
        Ok(self.staged().len()?)
    }

    pub fn is_empty(&self) -> Result<bool, Refusal> {
        Ok(self.staged().is_empty()?)
    }

    /// Insert or overwrite. A key or value past the limits is refused HERE, and the
    /// transaction is as it was (T-ES-16).
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<(), Refusal> {
        Ok(self.batch.put(key, value)?)
    }

    /// Remove `key`; `true` when this transaction saw it.
    pub fn remove(&mut self, key: &[u8]) -> Result<bool, Refusal> {
        let had = self.staged().contains(key)?;
        self.batch.remove(key)?;
        Ok(had)
    }

    /// Commit as one delta (or a compaction, when the delta writer asks for one), flushed with
    /// `d`. An empty transaction makes no new generation. More durability than the device
    /// gives is refused (`Refusal::Durability`).
    pub fn commit(self, d: Durability) -> Result<Generation, Refusal> {
        self.finish(d, Mode::Delta)
    }

    /// Commit as a whole v2 rewrite, compacted, whatever the delta writer would do.
    pub fn commit_compacted(self, d: Durability) -> Result<Generation, Refusal> {
        self.finish(d, Mode::Compact)
    }

    /// Stage the commit in memory and publish it to readers WITHOUT flushing it: `Db::flush`
    /// writes every staged commit at once (P11's group commit). Visible is not durable:
    /// `Db::durable` says what is.
    pub fn stage(self) -> Result<Generation, Refusal> {
        self.finish(Durability::None, Mode::Stage)
    }

    fn finish(self, d: Durability, mode: Mode) -> Result<Generation, Refusal> {
        let WriteTxn { db, mut writer, base, batch } = self;
        db.commit_txn(&mut writer, base, batch, d, mode)
    }

    /// Drop the writes. Nothing touched the image, so nothing is undone.
    pub fn abort(self) {}

    /// Drop the transaction but keep its writes, to resume later (`Db::begin_write_with`).
    pub fn into_batch(self) -> Batch {
        self.batch
    }
}
