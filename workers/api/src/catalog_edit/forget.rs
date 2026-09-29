//! TRANSLATIONS DO NOT OUTLIVE WHAT THEY NAME (W-CRUD, 2026-09-29).
//!
//! THE DEFECT, measured on qa-durres: a category was deleted with its dishes,
//! made again under the same name -- so the same slug id -- and its Ukrainian
//! heading came back although nobody had typed one. A dish behaves the same
//! way: `Catalog::remove_product` takes the record and leaves every
//! `<locale>/product/<id>/<field>` row in the venue's `i18n` image, where the
//! next dish to be born under that id inherits them. Nothing read them in
//! between, so the image also grew by every deletion for as long as the venue
//! lived.
//!
//! TWO IMAGES, IN THIS ORDER, ON PURPOSE. The catalogue write is the delete;
//! this sweep follows it and is best-effort: a sweep that fails leaves rows
//! nothing reads (the state every delete left until today), and is LOGGED,
//! never swallowed. The same shape `owner::update_product` has had since the
//! translations moved into their own image (catalogue first, `apply_i18n`
//! after) and for the same reason: a locale table must not take a menu write
//! down with it.

use worker::*;

/// The translation keys of `entity` records whose ids are in `ids`. PURE:
/// the key is `<locale>/<entity>/<id>/<field>` (`hubstore::i18n_key`).
pub fn stale_keys(all: &[(String, String)], entity: &str, ids: &[String]) -> Vec<String> {
    all.iter()
        .map(|(key, _)| key)
        .filter(|key| {
            let mut parts = key.splitn(4, '/');
            let (Some(_locale), Some(e), Some(id), Some(_field)) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
                return false;
            };
            e == entity && ids.iter().any(|i| i == id)
        })
        .cloned()
        .collect()
}

/// Remove every translation of these `entity` ids from the venue's `i18n`
/// image. Answers how many rows went. A venue that never wrote a translation
/// has no image and pays one read.
pub async fn forget_translations(place: &crate::hubstore::Place, entity: &str, ids: &[String]) -> Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    let (entity, ids) = (entity.to_string(), ids.to_vec());
    crate::hubstore::with_table(place, crate::hubstore::IMAGE_I18N, crate::hubstore::I18N_BYTES, move |t| {
        let stale = stale_keys(&t.all(crate::hubstore::I18N_KIND), &entity, &ids);
        for key in &stale {
            t.remove(crate::hubstore::I18N_KIND, key);
        }
        Ok(stale.len())
    })
    .await
}

/// `forget_translations`, logged when it fails; the delete it follows stands.
pub async fn forget_or_log(place: &crate::hubstore::Place, entity: &str, ids: &[String]) -> usize {
    match forget_translations(place, entity, ids).await {
        Ok(n) => n,
        Err(e) => {
            console_error!("translations of {} deleted {entity}(s) were not swept: {e}", ids.len());
            0
        }
    }
}

#[cfg(test)]
mod tests;
