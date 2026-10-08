//! THE MENU'S EDIT JOURNAL IN THE OBJECT'S OWN WRITE (W-PITR2; the journal is
//! `dowiz_hub::catalog::edits`, its routes `catalog_history.rs`).
//!
//! W-PITR wrote the journal from the Worker AFTER the catalogue landed: `with_log` pulled the
//! whole `catalog.edits` image across the hop and PUT it back whole -- up to ~2-3 MB per price
//! edit at a 600-record journal -- and a write cut between the two left a catalogue without
//! its record. Now every catalogue write the object stores (`put_image_as`, whatever asked for
//! it: an owner's PUT, the bulk import, a compaction) also APPENDS to the journal it holds in
//! memory, and the catalogue's changed chunks + meta and the journal's changed chunks + meta
//! go to storage in ONE `put_together` (`hubdo/atomic.rs`, `host::Batch`): the catalogue call
//! and the journal call(s) are issued in one turn with no await between them, which the
//! platform combines into one atomic write -- the same coalescing W-ATOMIC relies on past 127
//! keys (a single `Batch` carries one meta, so two images are two calls). Both land or neither.
//!
//! WHO: the Worker sends `x-edit-by` / `x-edit-at` with the catalogue PUT (`hubstore::save_image`
//! from `Place.edit`); the bulk import passes its signer. A write with no stamp (a compaction,
//! `W-PITR: content unchanged`) changes no key and adds no record.
//!
//! A JOURNAL THAT CANNOT BE READ OR APPENDED does not stop the catalogue write: it is said out
//! loud and the catalogue goes alone; the next write finds the journal out of step and records
//! the difference as unseen (`edits::journal`).

use super::atomic::plan;
use super::host::{Batch, MAX_KEYS};
use super::{changed_chunks, HubImages, Meta, CATALOG_IMAGE, CHUNK};
use crate::hubstore::Edited;
use dowiz_hub::catalog::{edits, Catalog};
use dowiz_hub::logimage::LogImage;
use worker::Result;

/// Chunk `n` of an image's bytes under its storage key.
fn slice<'a>(id: &str, b: &'a [u8], n: usize) -> (String, &'a [u8]) {
    let at = n * CHUNK;
    (HubImages::chunk_key(id, n), &b[at..(at + CHUNK).min(b.len())])
}

/// The journal's new bytes and how to store them.
struct Appended {
    bytes: Vec<u8>,
    changed: Vec<usize>,
    old_chunks: usize,
    meta: Meta,
}

impl HubImages {
    /// `put_image` carrying the signer of a catalogue write; the stamp never outlives the call.
    pub(super) async fn put_image_stamped(&self, id: &str, expected: i64, bytes: &[u8], stamp: Option<Edited>) -> Result<Option<i64>> {
        *self.edit.borrow_mut() = stamp;
        let out = self.put_image(id, expected, bytes).await;
        *self.edit.borrow_mut() = None;
        out
    }

    /// `put_image_as`'s ONE storage write: the image's chunks + meta and, for the catalogue, the
    /// journal's appended chunks + meta in the same `put_together`. `current`: the catalogue's
    /// generation before this write (the journal records `(current, meta.generation)`). `now`: the
    /// object's own clock read, taken in `hubdo.rs` (the clock gate's allowed place).
    pub(super) async fn write_image(&self, id: &str, bytes: &[u8], changed: &[usize], old_chunks: usize, meta: &Meta, current: i64, now: i64) -> Result<()> {
        if id != CATALOG_IMAGE {
            return self.write_chunks_then_meta(id, bytes, changed, old_chunks, meta).await;
        }
        let stamp = self.edit.borrow_mut().take().unwrap_or_default();
        let at = if stamp.at_ms > 0 { stamp.at_ms } else { now };
        let Some(j) = self.journal_append(bytes, (current, meta.generation), at, &stamp.by).await else {
            return self.write_chunks_then_meta(id, bytes, changed, old_chunks, meta).await;
        };
        let img = crate::catalog_history::IMAGE;
        let cat = plan(changed, old_chunks);
        if !cat.ahead.is_empty() || cat.with_meta.len() >= MAX_KEYS {
            // A catalogue is at most 107 chunks (`CEILING_BYTES`), so this is unreachable; if it
            // ever is not, the catalogue's own atomic write stands alone and the journal is behind.
            log_error!("menu.journal: a catalogue write of {} chunks cannot share a batch", changed.len());
            return self.write_chunks_then_meta(id, bytes, changed, old_chunks, meta).await;
        }
        let (cat_meta, j_meta) = ((Self::meta_key(id), meta), (Self::meta_key(img), &j.meta));
        let mut calls = vec![Batch { chunks: changed.iter().map(|&n| slice(id, bytes, n)).collect(), value: Some(cat_meta) }];
        let mut jcalls: Vec<Batch<'_, Meta>> =
            j.changed.chunks(MAX_KEYS - 1).map(|c| Batch { chunks: c.iter().map(|&n| slice(img, &j.bytes, n)).collect(), value: None }).collect();
        if jcalls.is_empty() {
            jcalls.push(Batch { chunks: Vec::new(), value: None });
        }
        if let Some(last) = jcalls.last_mut() {
            last.value = Some(j_meta);
        }
        calls.extend(jcalls);
        if let Err(e) = self.state.storage().put_together(&calls).await {
            self.mem.borrow_mut().remove(img);
            return Err(e);
        }
        let new_chunks = j.meta.chunks;
        for n in new_chunks..j.old_chunks {
            let _ = self.state.storage().delete(&Self::chunk_key(img, n)).await;
        }
        self.mem.borrow_mut().insert(img.to_string(), (j.meta, j.bytes));
        Ok(())
    }

    /// The journal with this write's records appended, or `None` (nothing to record, or the
    /// journal is unusable -- said out loud).
    async fn journal_append(&self, bytes: &[u8], gens: (i64, i64), at: i64, by: &str) -> Option<Appended> {
        let img = crate::catalog_history::IMAGE;
        // BEFORE: the catalogue this object holds (`put_image_as` read it just now); none = empty.
        let t0 = crate::otel::wall_us(); // AX0 (e): the two decodes, then the journal's load + append
        let (before, held) = match self.mem.borrow().get(CATALOG_IMAGE) {
            None => (Ok(edits::State::new()), 0),
            Some((_, b)) => (Catalog::load(b).map(|c| edits::state_of(&c)), b.len()),
        };
        let after = Catalog::load(bytes).map(|c| edits::state_of(&c));
        let (t1, decoded, jlen) = (crate::otel::wall_us(), held + bytes.len(), std::cell::Cell::new(0));
        let cost = || self.counters.catalogue_write(t1.saturating_sub(t0), decoded, crate::otel::wall_us().saturating_sub(t1), jlen.get());
        let (before, after) = match (before, after) {
            (Ok(b), Ok(a)) => (b, a),
            (b, a) => {
                log_error!("menu.journal: a catalogue does not load ({:?} / {:?}); not journaled", b.err(), a.err());
                cost();
                return None;
            }
        };
        let (jmeta, mut log) = match self.image(img).await {
            Ok(Some((m, b))) => match { jlen.set(b.len()); LogImage::load(&b) } {
                Ok(l) => (Some(m), l),
                Err(e) => {
                    log_error!("menu.journal: unreadable ({e:?}); not journaled");
                    return None;
                }
            },
            Ok(None) => (None, LogImage::create().ok()?),
            Err(e) => {
                log_error!("menu.journal: not read ({e}); not journaled");
                return None;
            }
        };
        let out = match edits::journal(&mut log, &before, &after, at, by, gens) {
            Ok(j) if j.edits + j.unseen + j.baseline == 0 && !j.compacted => None,
            Ok(j) => {
                if j.unseen > 0 {
                    log_error!("menu.journal: {} change(s) written by a path that did not journal; recorded as unseen", j.unseen);
                }
                let new = log.to_bytes();
                let old_chunks = jmeta.as_ref().map_or(0, |m| m.chunks);
                let changed = changed_chunks(self.mem.borrow().get(img).map(|(_, b)| b.as_slice()), &new, CHUNK);
                let meta = Meta { generation: jmeta.map_or(0, |m| m.generation) + 1, chunks: new.len().div_ceil(CHUNK).max(1), len: new.len() };
                Some(Appended { bytes: new, changed, old_chunks, meta })
            }
            Err(e) => {
                log_error!("menu.journal: does not replay ({e:?}); not journaled");
                None
            }
        };
        cost();
        out
    }
}

#[cfg(test)]
mod tests;
