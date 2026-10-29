//! `KvImage`: one KV image of a hub, held through the bebop library API (W-ES-P3,
//! R-EMBED-STORE §5 row P3).
//!
//! What `Catalog`, `Settings`, `Roster`, `Posts` and `Table` each did by hand -- a `Store`, a
//! decoded `Kv` copied out of it, and (the catalogue) a `dirty` set of changed keys -- is one
//! thing now: a `Db<Detached>` (the image; `Detached` because the Durable Object adapter
//! persists the bytes, W-ATOMIC) and a `Batch` of pending writes, read through together
//! (`ReadTxn::staged`: the batch first, then the image). The batch IS the dirty set and the
//! in-memory map at once; `to_bytes` hands it to one write transaction.
//!
//! THE API IS `Kv`'S (`get`/`put`/`remove`/`keys`/`snapshot_root`), so the five holders kept
//! their method bodies; only `.entries.iter().filter(starts_with(P))` became `.prefixed(P)`.
//!
//! TWO WAYS TO SAVE, per holder, exactly as before:
//!   * `Save::Delta` (the catalogue, W-DELTA): one record per changed key and a new root,
//!     compacting when the delta writer says so;
//!   * `Save::Compact` (settings, roster, posts, tables): the entries rewritten whole, v2.
//! A holder CREATED fresh compacts on its first save either way (the catalogue's old
//! `dirty: None`), and a save with nothing pending returns the image as it is.
//!
//! WHAT CHANGED IN THE BYTES: a compaction through the library keeps the generation climbing
//! (generation + 1, design D3b) where `Kv::compacted_bytes` restarted at 1 -- only the
//! generation-bearing cells differ, and no golden here pins those (main, 2026-10-29). A
//! loaded image whose newest commit fails its anchor is REFUSED as `BadCrc` (design D3a):
//! a Durable Object's writes are atomic, so a torn image there is a changed byte.

use std::ops::Bound;

use bebop_store::{Batch, Db, Detached, Durability, Options, Refusal, Verify};

use crate::HubError;

/// How a holder writes its image back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Save {
    Delta,
    Compact,
}

pub(crate) struct KvImage {
    db: Db<Detached>,
    pending: Batch,
    /// Created here and never saved: the first save compacts.
    fresh: bool,
    /// The doubling loop's limit, and the denominator of `usage`.
    ceiling: usize,
    save: Save,
    /// A write the limits refused (`put` cannot fail, as `Kv::put` could not): reported by
    /// every later `to_bytes`, as a commit with that entry was refused before.
    refused: Option<Refusal>,
}

fn options(ceiling: usize) -> Options {
    Options::detached().max_image_bytes(ceiling)
}

impl KvImage {
    /// An empty image, starting small (16 KiB, doubled at save time up to `ceiling`).
    pub(crate) fn create(ceiling: usize, save: Save) -> Result<KvImage, HubError> {
        let db = Db::create_detached(options(ceiling).initial_bytes(16 * 1024))?;
        let pending = Batch::new(db.options().limits);
        Ok(KvImage { db, pending, fresh: true, ceiling, save, refused: None })
    }

    /// The image in `bytes`, every object the KV root reaches checked (`Kv::load_checked`'s
    /// proof: a changed byte is `HubError::BadCrc`, not "not a hub").
    pub(crate) fn load(bytes: &[u8], ceiling: usize, save: Save) -> Result<KvImage, HubError> {
        let db = Db::from_bytes(bytes, options(ceiling).verify(Verify::Reachable))?;
        let pending = Batch::new(db.options().limits);
        Ok(KvImage { db, pending, fresh: false, ceiling, save, refused: None })
    }

    /// The value under `key`, the pending writes first. `None` on a read the image refuses --
    /// which `load` already proved it does not (every reachable object's crc, the key order).
    pub(crate) fn get(&self, key: &str) -> Option<Vec<u8>> {
        let r = self.db.begin_read();
        let v = r.staged(&self.pending).get(key.as_bytes()).ok()??;
        Some(v.into_owned())
    }

    pub(crate) fn contains(&self, key: &str) -> bool {
        let r = self.db.begin_read();
        r.staged(&self.pending).contains(key.as_bytes()).unwrap_or(false)
    }

    /// Insert or overwrite (pending until `to_bytes`).
    pub(crate) fn put(&mut self, key: &str, value: &[u8]) {
        if let Err(e) = self.pending.put(key.as_bytes(), value) {
            self.refuse(Refusal::from(e));
        }
    }

    /// Keep the first refusal: every later save reports it.
    fn refuse(&mut self, e: Refusal) {
        if self.refused.is_none() {
            self.refused = Some(e);
        }
    }

    /// Remove `key`, reporting whether it was there.
    pub(crate) fn remove(&mut self, key: &str) -> bool {
        if !self.contains(key) {
            return false;
        }
        if let Err(e) = self.pending.remove(key.as_bytes()) {
            self.refuse(Refusal::from(e));
            return false;
        }
        true
    }

    /// Every (full key, value) whose key starts with `prefix`, in key order: one range scan.
    pub(crate) fn prefixed(&self, prefix: &str) -> Vec<(String, Vec<u8>)> {
        self.range(prefix.as_bytes())
    }

    /// Every key, in key order.
    pub(crate) fn keys(&self) -> Vec<String> {
        self.range(b"").into_iter().map(|(k, _)| k).collect()
    }

    fn range(&self, prefix: &[u8]) -> Vec<(String, Vec<u8>)> {
        let r = self.db.begin_read();
        let s = r.staged(&self.pending);
        let end = prefix_end(prefix);
        let hi = match &end {
            Some(e) => Bound::Excluded(e.as_slice()),
            None => Bound::Unbounded,
        };
        let Ok(cursor) = s.cursor(Bound::Included(prefix), hi) else { return Vec::new() };
        cursor
            .filter_map(Result::ok)
            .map(|(k, v)| (String::from_utf8_lossy(&k).into_owned(), v.into_owned()))
            .collect()
    }

    /// FNV-1a over the entries with the pending writes applied: `Kv::snapshot_root`, formatted
    /// the way dowiz-core formats it.
    pub(crate) fn snapshot_root(&self) -> String {
        let r = self.db.begin_read();
        format!("{:016x}", r.staged(&self.pending).fold_root().unwrap_or(0))
    }

    /// What the published image has spent, against `ceiling`.
    pub(crate) fn usage(&self) -> crate::Usage {
        let r = self.db.begin_read();
        crate::usage::usage_of_sb(Some(r.superblock()), r.capacity_cells(), crate::ceiling_cells(self.ceiling), false)
    }

    /// The image the pending writes would compact into at `capacity` bytes, written nowhere
    /// (`Catalog::projected`'s dry run).
    pub(crate) fn compacted_bytes(&self, capacity: usize) -> Result<Vec<u8>, HubError> {
        let r = self.db.begin_read();
        let entries = r.staged(&self.pending).entries().map_err(Refusal::from)?;
        Ok(bebop_store::kv::compacted_from(entries.iter().map(|(k, v)| (k.as_bytes(), v.as_bytes())), capacity, None)?)
    }

    /// Write the pending writes into the image and hand back its bytes. The caller persists
    /// them; nothing here touches storage. A failed save keeps the pending writes.
    pub(crate) fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        let compact = self.fresh || self.save == Save::Compact;
        self.save_as(compact)
    }

    /// Rewrite the image whole (v2, no chain), the pending writes included -- in the CANONICAL
    /// form: exactly `Kv::compacted_bytes_fit` of the entries, generation 1, the bytes every
    /// compaction of the same entries has always written. This is the explicit operator path
    /// (`Catalog::compact`: a rollback to a build before v3) and W-PITR's canonical form (replay to
    /// T is compared byte for byte in it, catalog/edits/tests.rs), so it does NOT take the
    /// library's climbing generation (design D3b), which a save's own compaction does.
    pub(crate) fn compact(&mut self) -> Result<Vec<u8>, HubError> {
        if let Some(e) = &self.refused {
            return Err(e.clone().into());
        }
        let bytes = {
            let r = self.db.begin_read();
            let entries = r.staged(&self.pending).entries().map_err(Refusal::from)?;
            bebop_store::kv::compacted_from_fit(entries.iter().map(|(k, v)| (k.as_bytes(), v.as_bytes())), self.ceiling, None)?
        };
        self.db = Db::from_bytes(&bytes, options(self.ceiling))?;
        self.pending = Batch::new(self.db.options().limits);
        self.fresh = false;
        Ok(self.db.snapshot_bytes())
    }

    fn save_as(&mut self, compact: bool) -> Result<Vec<u8>, HubError> {
        if let Some(e) = &self.refused {
            return Err(e.clone().into());
        }
        if self.pending.is_empty() && !self.fresh && !compact {
            return Ok(self.db.snapshot_bytes());
        }
        if self.pending.is_empty() && !self.fresh && self.save == Save::Compact {
            // Nothing changed: the image IS the compaction of its entries already.
            return Ok(self.db.snapshot_bytes());
        }
        let kept = self.pending.clone();
        let batch = std::mem::replace(&mut self.pending, Batch::new(self.db.options().limits));
        let r = self.db.begin_write_with(batch).and_then(|w| if compact { w.commit_compacted(Durability::None) } else { w.commit(Durability::None) });
        if let Err(e) = r {
            self.pending = kept;
            return Err(e.into());
        }
        self.fresh = false;
        Ok(self.db.snapshot_bytes())
    }
}

/// The smallest key above every key that starts with `p`, or none (an empty or all-0xFF `p`).
fn prefix_end(p: &[u8]) -> Option<Vec<u8>> {
    let mut end = p.to_vec();
    while let Some(last) = end.pop() {
        if last < 0xFF {
            end.push(last + 1);
            return Some(end);
        }
    }
    None
}

#[cfg(test)]
mod tests;
