//! The archives' folds, for `rebuild`'s R5 crossing (`rebuild::cross_archives`).
//!
//! READ ONLY WHEN SOMETHING IS STRANDED that the hot log does not know, so a
//! healthy night reads no archive. An archive read here is not kept resident
//! (the same rule `forget` follows): it is read once a night at most.

use super::HubImages;
use worker::*;

impl HubImages {
    /// Every archive the settings list, as `(id, orders_state pairs)`. An
    /// archive that is listed and unreadable is an error, not an empty fold.
    pub(super) async fn archive_folds(&self) -> Result<Vec<(String, Vec<(String, String)>)>> {
        let ids = match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, b)) => crate::hubstore::archives_of(
                &dowiz_hub::settings::Settings::load(&b)
                    .map_err(|_| Error::RustError("settings image is unreadable".into()))?,
            ),
            None => Vec::new(),
        };
        let mut out = Vec::new();
        for id in ids.into_iter().filter(|id| crate::hubstore::is_archive_id(id)) {
            let resident = self.mem.borrow().contains_key(&id);
            let image = self.image(&id).await?;
            if !resident {
                self.mem.borrow_mut().remove(&id);
            }
            let Some((_, bytes)) = image else { continue };
            let hub = dowiz_hub::Hub::load(&bytes)
                .map_err(|_| Error::RustError(format!("archive {id} is unreadable")))?;
            let pairs = crate::hubstore::orders_state(&hub).into_iter().map(|e| (e.order_id, e.order_json)).collect();
            out.push((id, pairs));
        }
        Ok(out)
    }
}
