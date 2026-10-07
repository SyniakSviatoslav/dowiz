//! ROLLBACK SAFETY FOR v3 KV IMAGES (W-ATOMIC row 3).
//!
//! W-DELTA's catalogue writes append a delta chain (bebop-store image v3) instead of compacting.
//! A Worker built before W-DELTA REFUSES a v3 image, so rolling the Worker back after the first
//! delta write would leave every venue's catalogue unreadable until something compacted it.
//!
//! `POST /fold/compact` (this object only; reached through the platform administrators' fan-out in
//! `compact/fan.rs`, never from a venue's console) does two things, in this order:
//!   1. PINS the object to v2: from now on every write of a KV image is compacted HERE, before it
//!      is stored, whichever Worker build sent it. Without the pin the new Worker would write a
//!      fresh delta in the seconds between this call and the version switch.
//!   2. COMPACTS every KV image that is not v2 already, as an ordinary generation-guarded write.
//!
//! `POST /fold/compact?pin=0` lifts the pin and compacts nothing (a forward deploy that wants the
//! deltas back). The pin is one storage key, read on KV-image writes only.
//!
//! THE COMPACTION ITSELF IS [`to_v2`], and before W-DELTA merges it has nothing to do: no writer
//! in this tree makes a v3 image. Main wires W-DELTA's `Catalog::compact()` there (see its body).

use super::HubImages;
use crate::wire::{Call, Reply};
use serde_json::json;
use worker::{Error, Result};

#[cfg(test)]
mod tests;
pub(crate) mod fan;

/// The images held in the bebop KV layout, the ones W-DELTA can leave at v3.
pub(super) const KV_IMAGES: &[&str] = &[super::CATALOG_IMAGE];
/// `true` while KV images are stored v2 only.
const PIN_KEY: &str = "pin:kv-v2";

/// A compactor: `Ok(None)` when `bytes` is v2 already (nothing to write), `Ok(Some(v2))` otherwise.
pub(super) type Compactor = fn(&str, &[u8]) -> std::result::Result<Option<Vec<u8>>, String>;

/// A catalogue image loaded through W-DELTA's reader and rewritten whole (`Catalog::compact`):
/// a v3 image (base + delta chain) comes back as one v2 image; a v2 image comes back unchanged.
pub(super) fn to_v2(id: &str, bytes: &[u8]) -> std::result::Result<Option<Vec<u8>>, String> {
    let mut cat = dowiz_hub::catalog::Catalog::load(bytes).map_err(|e| format!("{id}: {e:?}"))?;
    let v2 = cat.compact().map_err(|e| format!("{id}: {e:?}"))?;
    Ok((v2.as_slice() != bytes).then_some(v2))
}

#[cfg(test)]
thread_local! {
    /// A test's stand-in for `to_v2` (this tree has no v3 writer to compact).
    pub(super) static FAKE: std::cell::Cell<Option<Compactor>> = const { std::cell::Cell::new(None) };
}

fn compactor() -> Compactor {
    #[cfg(test)]
    if let Some(f) = FAKE.with(std::cell::Cell::get) {
        return f;
    }
    to_v2
}

impl HubImages {
    /// The write path's hook (`put_image_as`): while pinned, a KV image is stored compacted.
    pub(super) async fn v2_when_pinned(&self, id: &str, bytes: &[u8]) -> Result<Option<Vec<u8>>> {
        if !KV_IMAGES.contains(&id) || !self.state.storage().get::<bool>(PIN_KEY).await?.unwrap_or(false) {
            return Ok(None);
        }
        compactor()(id, bytes).map_err(|e| Error::RustError(format!("compact {id}: {e}")))
    }

    /// `POST /fold/compact[?pin=0]`.
    pub(super) async fn compact_route(&self, req: &Call) -> Result<Reply> {
        let pin = !req.url()?.query_pairs().any(|(k, v)| k == "pin" && v == "0");
        self.state.storage().put(PIN_KEY, pin).await?;
        let mut images = Vec::new();
        if pin {
            for id in KV_IMAGES {
                images.push(self.compact_one(id).await?);
            }
        }
        Reply::from_json(&json!({ "pinned": pin, "images": images }))
    }

    async fn compact_one(&self, id: &str) -> Result<serde_json::Value> {
        let Some((meta, bytes)) = self.image(id).await? else {
            return Ok(json!({ "id": id, "absent": true }));
        };
        let Some(v2) = compactor()(id, &bytes).map_err(|e| Error::RustError(format!("compact {id}: {e}")))? else {
            return Ok(json!({ "id": id, "generation": meta.generation, "compacted": false }));
        };
        match self.put_image(id, meta.generation, &v2).await? {
            Some(generation) => Ok(json!({ "id": id, "generation": generation, "compacted": true, "bytes": v2.len() })),
            None => Err(Error::RustError(format!("compact {id}: the generation moved"))),
        }
    }
}
