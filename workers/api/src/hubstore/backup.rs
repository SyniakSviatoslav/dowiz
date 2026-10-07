//! THE VENUE'S OWN COPY: export, the archives a nightly copy carries once, and the restore
//! into an empty venue. Moved out of `hubstore.rs` (W-PITR, 2026-10-07) so that file shrinks
//! while `Place` gains the edit stamp; `hubstore` re-exports everything, so no caller moved.

use super::*;
/// Every image this venue has, for the venue to keep.
///
/// P68'S SOVEREIGN BACKUP, STARTED. `settings.rs` says the honest thing about
/// secrets at rest: what protects them is file mode on hardware the venue owns,
/// and real protection needs a key that lives somewhere else. The same is true
/// of the data itself. Today the only thing standing between Dubin & Sushi and
/// losing their entire history is Cloudflare's own thirty-day time travel —
/// which is a fine safety net and is not THEIRS. This is the copy they hold.
///
/// SELF-DESCRIBING AND SELF-CHECKING, because a backup nobody can verify is a
/// backup nobody can trust. The manifest names each image, its length and its
/// SHA-256, so a restore can refuse a corrupted file instead of feeding a
/// truncated arena to the kernel.
///
/// AND THE MENU'S EDIT JOURNAL (W-PITR): the history a restore of the catalogue to any moment
/// replays, copied off-site with the catalogue it describes.
pub const IMAGES: &[&str] =
    &[IMAGE_LOG, IMAGE_CATALOG, IMAGE_SETTINGS, IMAGE_POSTS, IMAGE_STOCK, crate::catalog_history::IMAGE];

/// Archives already copied off-site, so a nightly bundle carries each one ONCE.
const ARCHIVES_BACKED_KEY: &str = "log.archives.backed";

/// Which archives this venue has that have never been in a backup.
pub async fn archives_pending(place: &Place) -> Result<Vec<String>> {
    let settings = load_settings(place).await?.settings;
    let done: Vec<String> = settings
        .get(ARCHIVES_BACKED_KEY)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    Ok(archives_of(&settings).into_iter().filter(|a| !done.contains(a)).collect())
}

/// Mark archives as copied. Called after a bundle lands, never before.
pub async fn archives_marked(place: &Place, ids: &[String]) -> Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let ids = ids.to_vec();
    with_settings(place, move |s| {
        let mut done: Vec<String> = s
            .get(ARCHIVES_BACKED_KEY)
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(str::to_string)
            .collect();
        for id in &ids {
            if !done.contains(id) {
                done.push(id.clone());
            }
        }
        s.set(ARCHIVES_BACKED_KEY, &done.join(","));
        Ok(())
    })
    .await
}

pub async fn export(place: &Place, now_ms: i64) -> Result<serde_json::Value> {
    use sha2::{Digest, Sha256};
    let got = load_images(place, IMAGES).await?;
    let mut images = serde_json::Map::new();
    for id in IMAGES {
        let Some((bytes, generation)) = got.get(*id) else { continue };
        let digest: String =
            Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
        images.insert(
            (*id).to_string(),
            serde_json::json!({
                "generation": generation,
                "bytes": bytes.len(),
                "sha256": digest,
                // Base64 rather than hex: a backup is downloaded whole, so the
                // wire cost is paid once and a third smaller matters, unlike on
                // the read path where the crossing was the cost.
                "image": base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD, bytes),
            }),
        );
    }
    // THE ARCHIVES THAT HAVE NEVER BEEN COPIED, and only those.
    //
    // Rotation makes history into its own image, and an image nobody copies
    // off-site is history kept in exactly one place. Carrying EVERY archive in
    // EVERY nightly bundle would put it back where it started -- a file that
    // grows forever -- so each archive travels once and is marked by the
    // caller after the bundle lands.
    let pending = archives_pending(place).await.unwrap_or_default();
    let mut archives = serde_json::Map::new();
    if !pending.is_empty() {
        let ids: Vec<&str> = pending.iter().map(String::as_str).collect();
        let got = load_images(place, &ids).await?;
        for id in &pending {
            let Some((bytes, generation)) = got.get(id) else { continue };
            let digest: String =
                Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
            archives.insert(
                id.clone(),
                serde_json::json!({
                    "generation": generation,
                    "bytes": bytes.len(),
                    "sha256": digest,
                    "image": base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD, bytes),
                }),
            );
        }
    }

    Ok(serde_json::json!({
        "format": "dowiz-hub-backup/1",
        "venue": place.venue,
        "taken_at_ms": now_ms,
        "images": images,
        "archives": archives,
    }))
}

/// Put a backup back — ONLY INTO A VENUE THAT HAS NONE.
///
/// THE REFUSAL IS THE FEATURE. A restore that overwrites a live hub is a
/// one-click way to erase a venue's entire history, and it would be reachable
/// by anything that could reach an owner's token. So this writes only where
/// generation is zero: disaster recovery into a fresh object, never a rollback
/// over something that exists. An operator who genuinely wants to roll back
/// deletes the object first, deliberately, which is a different act.
///
/// EVERY IMAGE IS CHECKED BEFORE ANY IMAGE IS WRITTEN. A bundle whose third
/// image is corrupt must not leave the first two in place and the rest missing.
pub async fn import(place: &Place, bundle: &serde_json::Value) -> Result<Vec<String>> {
    use sha2::{Digest, Sha256};
    if bundle.get("format").and_then(|v| v.as_str()) != Some("dowiz-hub-backup/1") {
        return Err(Error::RustError("not a dowiz hub backup".into()));
    }
    let Some(images) = bundle.get("images").and_then(|v| v.as_object()) else {
        return Err(Error::RustError("backup has no images".into()));
    };

    // A BUNDLE MAY CARRY ARCHIVES, and a restore that dropped them would put a
    // venue back with its live orders and no history -- which is the shape of
    // loss rotation is supposed to prevent. They are checked and written
    // exactly as the five fixed images are; only the name test differs,
    // because an archive's name carries the generation it was cut at.
    let archives = bundle.get("archives").and_then(|v| v.as_object());
    let named: Vec<(&String, &serde_json::Value)> =
        images.iter().chain(archives.into_iter().flatten()).collect();

    let mut staged: Vec<(String, Vec<u8>)> = Vec::new();
    for (id, entry) in named {
        if !IMAGES.contains(&id.as_str()) && !is_archive_id(id) {
            return Err(Error::RustError(format!("backup names an unknown image: {id}")));
        }
        let Some(b64) = entry.get("image").and_then(|v| v.as_str()) else {
            return Err(Error::RustError(format!("image {id} has no bytes")));
        };
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| Error::RustError(format!("image {id} is not base64: {e}")))?;
        let want = entry.get("sha256").and_then(|v| v.as_str()).unwrap_or("");
        let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        if got != want {
            return Err(Error::RustError(format!(
                "image {id} does not match its digest; the backup is damaged"
            )));
        }
        staged.push((id.clone(), bytes));
    }

    // Now, and only now, that every image has been read and checked.
    // NOTHING MAY EXIST, checked for every image before any is written.
    for (id, _) in &staged {
        let existing = load_images(place, &[id.as_str()]).await?;
        if existing.get(id).map(|(_, g)| *g).unwrap_or(0) != 0 {
            return Err(Error::RustError(format!(
                "{id} already exists in this venue; a restore never overwrites"
            )));
        }
    }
    // THE MENU HISTORY GOES LAST, OVER WHAT THE CATALOGUE'S OWN WRITE JUST JOURNALED: the object
    // journals every catalogue write (W-PITR2, `hubdo/journal.rs`), so restoring the catalogue
    // starts a journal this restore then replaces with the venue's own.
    let journal = crate::catalog_history::IMAGE;
    staged.sort_by_key(|(id, _)| id == journal);
    let mut written = Vec::new();
    for (id, bytes) in staged {
        let at = if id == journal { load_bytes(place, journal).await?.map_or(0, |(_, g)| g) } else { 0 };
        if !save_image(place, &id, bytes, at).await? {
            return Err(Error::RustError(format!("{id} was written by someone else mid-restore")));
        }
        written.push(id);
    }
    Ok(written)
}
