//! THE IMAGES THIS OBJECT HOLDS: read from memory or storage, written under the
//! generation guard, and the log's fold memo kept in step (split out of
//! `hubdo.rs`, W-SPLIT). `put_image_as` itself stays in `hubdo.rs` because the
//! write reads the object's clock and `clock.sh` allows that read there only.

use super::{changed_chunks, menu, HubImages, Meta, OrderView, CHUNK, LOG_IMAGE, TAG_CONSOLE, TAG_COURIER};
use worker::*;

impl HubImages {
    /// The image, from memory if this object has it, from storage if not.
    pub(super) async fn image(&self, id: &str) -> Result<Option<(Meta, Vec<u8>)>> {
        if let Some(hit) = self.mem.borrow().get(id) {
            return Ok(Some(hit.clone()));
        }
        let store = self.state.storage();
        // AN ERROR IS NOT AN ABSENCE, and `.ok().flatten()` said it was.
        //
        // `Storage::get` answers Ok(None) for a key that is not there and Err
        // for a store that could not answer -- a timeout, an internal error, a
        // `Meta` that stopped deserialising. Folding the second into the first
        // tells every caller above that this venue has NO IMAGE, and every
        // guard downstream is built to be helpful about that: `load` hands the
        // handler a fresh 64 KiB hub, `put_image` reads the generation as 0,
        // agrees with the caller's 0, and writes chunk zero plus a meta saying
        // `chunks: 1`. The venue's entire order log is then unreachable -- the
        // old chunks are still on disk with nothing naming them. The same
        // conflation defeats `import`'s never-overwrite check and
        // `seed_fresh_hub`'s emptiness test, and makes `/fold/orders` answer an
        // empty list at generation 0 instead of failing.
        //
        // The chunk read two lines down has always propagated its errors with
        // `?`. Only the meta read swallowed them.
        let Some(meta) = store.get::<Meta>(&Self::meta_key(id)).await? else {
            return Ok(None);
        };
        // ALL CHUNKS IN ONE CALL. Asking for them one at a time would reproduce
        // the round-trip-per-chunk shape that made the D1 version slow, on a
        // store where the calls are cheaper but not free.
        let keys: Vec<String> = (0..meta.chunks).map(|n| Self::chunk_key(id, n)).collect();
        let got = store.get_chunks(&keys).await?;
        let mut bytes = Vec::with_capacity(meta.len);
        for (key, part) in keys.iter().zip(got) {
            let Some(part) = part else {
                // A meta record whose chunks are not all there is not an image.
                // Refusing names it; assembling what IS there would hand the
                // kernel a truncated arena.
                return Err(Error::RustError(format!("image {id} is missing chunk {key}")));
            };
            bytes.extend_from_slice(&part);
        }
        if bytes.len() != meta.len {
            return Err(Error::RustError(format!(
                "image {id} is {} bytes, its meta says {}",
                bytes.len(),
                meta.len
            )));
        }
        let entry = (meta, bytes);
        self.catview_forget(id); self.logmark_forget(id);
        self.mem.borrow_mut().insert(id.to_string(), entry.clone());
        Ok(Some(entry))
    }

    /// The log image's folded orders, newest first — from the memo when the
    /// generation has not moved, from the bytes when it has.
    ///
    /// THE FOLD HAPPENS HERE AND NOT IN THE WORKER, which is the whole of
    /// phase 2. The image lives in this object; a Worker that wants the queue
    /// used to be handed every order the venue had ever taken so it could keep
    /// the last day. Now it is handed the last day.
    pub(super) async fn orders_view(&self) -> Result<(i64, Vec<OrderView>)> {
        let (t0, cold) = (crate::otel::wall_us(), std::cell::Cell::new(false)); // AX0 (c): the wall of a fold from bytes
        let Some((meta, bytes)) = self.image(LOG_IMAGE).await? else {
            return Ok((0, Vec::new()));
        };
        let view = crate::fold::projection::read(&mut self.folded.borrow_mut(), meta.generation, || {
            cold.set(true); self.load_log(meta.generation, &bytes)
                .map(|hub| crate::image_err::hub_loaded(hub).events())
                .map_err(|e| crate::image_err::unreadable("hub", e))
        })?;
        if cold.get() { self.counters.cold_fold(crate::otel::wall_us().saturating_sub(t0)); }
        Ok((meta.generation, view))
    }

    /// Write, with the SAME generation guard the D1 version used.
    ///
    /// A Durable Object serialises its own requests, so two writers cannot
    /// interleave here and the guard should never fire. It is kept because it
    /// costs one comparison, because it is what lets a caller tell "someone
    /// else moved it" from "it failed", and because the day this moves again is
    /// the day the guard is load-bearing.
    pub(super) async fn put_image(&self, id: &str, expected: i64, bytes: &[u8]) -> Result<Option<i64>> {
        self.put_image_as(id, expected, bytes, crate::fold::projection::Written::Whole).await
    }

    /// A COMMAND'S LOG WRITE: `put_image` of the hub, and the memo steps over
    /// the events the command added (R1) -- when the memo is at `expected`;
    /// a cold one is not walked for, the next read folds it.
    pub(super) async fn put_log(&self, expected: i64, hub: &dowiz_hub::Hub) -> Result<Option<i64>> {
        let warm = self.folded.borrow().as_ref().is_some_and(|m| m.generation() == expected);
        let written = if warm {
            hub.appended().map_or_else(|| crate::fold::projection::Written::Log(hub.events()), crate::fold::projection::Written::Tail) // O(k) when only appended (W-LOOPB)
        } else {
            crate::fold::projection::Written::Whole
        };
        self.put_image_as(LOG_IMAGE, expected, &hub.to_bytes_trimmed(), written).await
    }

    /// `put_image_as` up to and including the menu's republish: the guard, the
    /// chunks and the meta in one atomic write, the stale chunks dropped, the
    /// copy in memory replaced. `None` is the guard refusing. `now` is the
    /// object's clock, read where `hubdo.rs` reads it, at the moment of the write.
    pub(super) async fn put_chunks(
        &self,
        id: &str,
        expected: i64,
        bytes: &[u8],
        how: &crate::fold::projection::Written,
        now: impl Fn() -> i64,
    ) -> Result<Option<i64>> {
        let current = self.image(id).await?.map(|(m, _)| m.generation).unwrap_or(0);
        if expected != current {
            return Ok(None);
        }
        let next = current + 1;
        let pinned = self.v2_when_pinned(id, bytes).await?; let bytes = pinned.as_deref().unwrap_or(bytes); // a KV image while pinned for a rollback (`hubdo/compact.rs`)
        let store = self.state.storage();
        // The menu memo is TAKEN DOWN BEFORE a write to what it was folded from, so a failed write cannot leave one standing over bytes it no longer describes (R2); it is kept aside only to be compared with its successor (AX3, `hubdo/menu.rs`).
        let prev_menu = if menu::MENU_INPUTS.contains(&id) { self.menu.borrow_mut().take() } else { None };

        let chunks = bytes.len().div_ceil(CHUNK).max(1);
        // ONLY THE CHUNKS THAT MOVED. The log is append-only: an append touches
        // the superblock at the front and the tail at the back, and every
        // chunk between them is byte-for-byte what storage already holds.
        // Writing all of them made one order cost `ceil(image / 96 KiB)` row
        // writes -- 45 at a 4 MB log, and growing with every order the venue
        // ever took. The comparison is against the copy in memory, which
        // `image()` above has just made current.
        let changed = {
            let mem = self.mem.borrow();
            changed_chunks(mem.get(id).map(|(_, b)| b.as_slice()), bytes, CHUNK)
        };
        let meta = Meta { generation: next, chunks, len: bytes.len() };
        // THE CHUNKS AND THE META IN ONE ATOMIC STORAGE WRITE (W-ATOMIC, `hubdo/atomic.rs`): a cut write leaves the previous generation, never a mix.
        let old_chunks = self.mem.borrow().get(id).map(|(m, _)| m.chunks).unwrap_or(0);
        if let Err(e) = self.write_image(id, bytes, &changed, old_chunks, &meta, current, now()).await { // + the catalogue's journal, same batch (W-PITR2, `hubdo/journal.rs`)
            // STORAGE IS THE ONLY TRUTH AFTER A FAILED WRITE. Some chunks may
            // have landed and some not; the copy in memory no longer says
            // what is on disk, and the next write's diff would trust it and
            // skip a chunk that is wrong. Forget the copy: the next read
            // rebuilds it from storage and the next diff is against the truth.
            self.mem.borrow_mut().remove(id);
            return Err(e);
        }
        self.count_write(changed.len()); // AX0 (b) + (d) + W-AE every NTH point (`hubdo/counters.rs`)
        // Chunks past the end of the new image are unreachable now that the
        // meta describes a shorter one, and only now.
        //
        // THE BORROW IS RELEASED BEFORE THE AWAITS. Holding a `Ref` across a
        // storage call leaves a window in which any re-entry that borrows
        // mutably -- `image()` on a cold key -- panics the whole object. The
        // number is copied out first; it is a `usize`.
        for n in chunks..old_chunks {
            let _ = store.delete(&Self::chunk_key(id, n)).await;
        }
        self.catview_forget(id); self.logmark_written(id, how, expected, next);
        self.mem.borrow_mut().insert(id.to_string(), (meta, bytes.to_vec()));
        // THE STOREFRONT'S READ PATH IS PUBLISHED by the write that moved it (BN2, `hubdo/publish.rs`); it never fails the write.
        if menu::MENU_INPUTS.contains(&id) { self.menu_after_write(prev_menu).await; } // AX3: not when the menu's out bytes did not move (`hubdo/edges.rs`)
        Ok(Some(next))
    }

    /// The tail of `put_image_as`: the log's projection moves with the write.
    pub(super) fn log_moved(&self, id: &str, expected: i64, next: i64, how: crate::fold::projection::Written) {
        // THE PROJECTION IS DERIVED FROM THIS IMAGE, so it moves with the
        // write: stepped over the events an append or a command added, dropped
        // by any other write (`fold::projection::after_log_write`).
        if id == LOG_IMAGE {
            // AN APPEND IS NOT A GAP (W-COV BUG-1): it adds exactly the one
            // event `append` broadcasts next, into the window. Only a write
            // that adds events nobody broadcast one by one is a gap.
            let gap = !matches!(how, crate::fold::projection::Written::Appended(_));
            crate::fold::projection::after_log_write(&mut self.folded.borrow_mut(), expected, next, how);
            // ── A WHOLE-IMAGE WRITE IS A GAP, AND IT HAS TO BE ONE ──
            //
            // Not every write to this log comes through `append`. A placement
            // still does its own read-modify-write (a promotion's last use has
            // to be counted and spent in one breath), and so does a rotation.
            // Those move the generation WITHOUT putting a `Change` in the
            // window -- and a window that stayed quiet would answer the next
            // `?since=` with "nothing changed" about a log that gained an
            // order. A console would believe it, advance its copy's
            // generation, and never see that order again.
            //
            // So the window is CLEARED, which makes `changes_since` answer
            // `None` -- "ask for the list" -- and the sockets are told the log
            // moved so they ask now rather than in ninety seconds.
            if gap {
                self.recent.borrow_mut().clear();
                let msg = serde_json::json!({ "t": "moved", "generation": next }).to_string();
                for tag in [TAG_CONSOLE, TAG_COURIER, crate::services::orders::kitchen_ack::board::TAG_KITCHEN] {
                    for ws in self.state.get_websockets_with_tag(tag) {
                        let _ = ws.send_with_str(&msg);
                    }
                }
            }
        }
    }
}
