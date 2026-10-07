//! ONE IMAGE WRITE IS ONE ATOMIC STORAGE WRITE (W-ATOMIC).
//!
//! THE DEFECT THIS REPLACES. `put_image_as` sent an image's changed chunks one `put` at a time,
//! each awaited, then the meta. Chunk 0 (the superblock) and the tail are rewritten IN PLACE, under
//! keys the OLD meta still names, so a write cut after chunk 0 and before the tail or the meta left
//! an image the next cold read refused (`is N bytes, its meta says M`) or, at the same length,
//! assembled from two generations without a word. Every log append and every catalogue save took
//! that window.
//!
//! NOW the chunks and the meta go in ONE `put(entries)`, which Cloudflare stores whole or not at
//! all (`host::Batch` quotes the page: https://developers.cloudflare.com/durable-objects/api/storage-api/).
//! A cut write leaves the previous generation, byte for byte.
//!
//! MORE THAN `MAX_KEYS` (128) KEYS. A compacted image stops at `CEILING_BYTES` (10 MiB):
//! `ceil(10 MiB / 96 KiB) = 107` chunks + the meta = 108 keys, one call. The append logs have NO
//! ceiling (`dowiz_hub::CEILING_BYTES`'s own comment), so a log past 127 chunks can change more
//! than 127 of them in one write, and then [`plan`] splits:
//!   * AHEAD of the meta, awaited one call after another, go ONLY chunks at or past the OLD meta's
//!     `chunks`. Nothing the old meta names is touched, so a cut there leaves the previous
//!     generation readable (stray chunks past its end are never read, and the next write
//!     overwrites them).
//!   * WITH the meta goes every chunk the old meta DOES name. Up to 127 of them that is one call.
//!     Past 127 (an old image over 11.9 MiB rewritten across more than 127 of its chunks) it is
//!     several calls ISSUED TOGETHER with no await between them, which the same page says are
//!     "combined and submitted atomically" -- the one case that rests on coalescing rather than on
//!     a single call.

use super::host::{Batch, MAX_KEYS};
use super::{HubImages, Meta, CHUNK};
use worker::Result;

/// Which changed chunks go AHEAD of the meta and which go WITH it.
#[derive(Debug, PartialEq)]
pub(super) struct Plan {
    /// Separate calls, each at most `MAX_KEYS` chunks, every index `>= old_chunks`.
    pub ahead: Vec<Vec<usize>>,
    /// The chunks that land with the meta: every changed index `< old_chunks`, then fresh ones
    /// while one call has room.
    pub with_meta: Vec<usize>,
}

/// Split `changed` (ascending) for an image whose stored meta names `old_chunks` chunks.
pub(super) fn plan(changed: &[usize], old_chunks: usize) -> Plan {
    if changed.len() < MAX_KEYS {
        return Plan { ahead: Vec::new(), with_meta: changed.to_vec() };
    }
    let (mut with_meta, fresh): (Vec<usize>, Vec<usize>) = changed.iter().partition(|&&n| n < old_chunks);
    let room = (MAX_KEYS - 1).saturating_sub(with_meta.len()).min(fresh.len());
    with_meta.extend_from_slice(&fresh[..room]);
    Plan { ahead: fresh[room..].chunks(MAX_KEYS).map(<[usize]>::to_vec).collect(), with_meta }
}

impl HubImages {
    /// Store `changed` chunks of `bytes` and then `meta`, so that no cut leaves a meta naming a chunk
    /// of another generation (module header). `old_chunks`: what the STORED meta names (0: none).
    pub(super) async fn write_chunks_then_meta(
        &self,
        id: &str,
        bytes: &[u8],
        changed: &[usize],
        old_chunks: usize,
        meta: &Meta,
    ) -> Result<()> {
        let store = self.state.storage();
        let slice = |&n: &usize| {
            let at = n * CHUNK;
            (Self::chunk_key(id, n), &bytes[at..(at + CHUNK).min(bytes.len())])
        };
        let p = plan(changed, old_chunks);
        for ahead in &p.ahead {
            store.put_batch(&Batch::<Meta> { chunks: ahead.iter().map(slice).collect(), value: None }).await?;
        }
        let mut calls: Vec<Batch<'_, Meta>> =
            p.with_meta.chunks(MAX_KEYS).map(|c| Batch { chunks: c.iter().map(slice).collect(), value: None }).collect();
        if calls.last().is_none_or(|c| c.keys() == MAX_KEYS) {
            calls.push(Batch { chunks: Vec::new(), value: None });
        }
        if let Some(last) = calls.last_mut() {
            last.value = Some((Self::meta_key(id), meta));
        }
        store.put_together(&calls).await
    }
}

#[cfg(test)]
mod tests;
