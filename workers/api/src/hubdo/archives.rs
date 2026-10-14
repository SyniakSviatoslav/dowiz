//! The archives' folds, for `rebuild`'s R5 crossing (`rebuild::cross_archives`),
//! and one archived order for the owner's look back (`archived_order`).
//!
//! READ ONLY WHEN SOMETHING IS STRANDED that the hot log does not know, so a
//! healthy night reads no archive. An archive read here is not kept resident
//! (the same rule `forget` follows): it is read once a night at most.
//!
//! AN ARCHIVED ORDER IS FINISHED AND READ-ONLY (2026-10-14). The rotation moves
//! an order out after 30 quiet days in a final status; the hot log then
//! answered 404 for it, so a receipt reprint said "not found" and a refund said
//! the order did not exist. A receipt is read from the archive; a refund is
//! refused by name, because new events for an order the hot log no longer
//! holds would fold to half a history.

use super::HubImages;
use worker::*;

/// What a refund of an archived order answers, instead of "no such order".
pub(super) const ARCHIVED_REFUND: &str =
    "this order finished more than 30 days ago and is archived; a refund can no longer be recorded on it -- give the money back by hand (cash or your card provider)";

impl HubImages {
    /// The archives the settings list, oldest first.
    async fn archive_ids(&self) -> Result<Vec<String>> {
        Ok(match self.image(crate::hubstore::IMAGE_SETTINGS).await? {
            Some((_, b)) => crate::hubstore::archives_of(
                &dowiz_hub::settings::Settings::load(&b)
                    .map_err(|_| Error::RustError("settings image is unreadable".into()))?,
            ),
            None => Vec::new(),
        }
        .into_iter()
        .filter(|id| crate::hubstore::is_archive_id(id))
        .collect())
    }

    /// One archive's folded orders, `(order_id, order json)`, read without
    /// staying resident. `None` when the listed image is absent.
    async fn archive_fold(&self, id: &str) -> Result<Option<Vec<(String, String)>>> {
        let resident = self.mem.borrow().contains_key(id);
        let image = self.image(id).await?;
        if !resident {
            self.mem.borrow_mut().remove(id);
        }
        let Some((_, bytes)) = image else { return Ok(None) };
        let hub = dowiz_hub::Hub::load(&bytes)
            .map_err(|_| Error::RustError(format!("archive {id} is unreadable")))?;
        Ok(Some(crate::hubstore::orders_state(&hub).into_iter().map(|e| (e.order_id, e.order_json)).collect()))
    }

    /// Every archive the settings list, as `(id, orders_state pairs)`. An
    /// archive that is listed and unreadable is an error, not an empty fold.
    pub(super) async fn archive_folds(&self) -> Result<Vec<(String, Vec<(String, String)>)>> {
        let mut out = Vec::new();
        for id in self.archive_ids().await? {
            if let Some(pairs) = self.archive_fold(&id).await? {
                out.push((id, pairs));
            }
        }
        Ok(out)
    }

    /// One order's folded state from the archives, newest first, stopping at
    /// the first that holds it; `None` when no archive does. Asked only after
    /// the hot log said it has never heard of the order.
    pub(super) async fn archived_order(&self, order_id: &str) -> Result<Option<String>> {
        for id in self.archive_ids().await?.iter().rev() {
            if let Some(pairs) = self.archive_fold(id).await? {
                if let Some((_, json)) = pairs.into_iter().find(|(o, _)| o == order_id) {
                    return Ok(Some(json));
                }
            }
        }
        Ok(None)
    }

    /// The refusal for a refund-side command on an order the hot log does not
    /// hold: the archive's, by name, when this venue's archive has it; else
    /// `None` and the command's own "no such order" stands.
    pub(super) async fn refused_as_archived(&self, order_id: &str, location_id: &str) -> Result<Option<crate::command::Refused>> {
        let Some(json) = self.archived_order(order_id).await? else { return Ok(None) };
        Ok(archived_here(&json, location_id).then(|| crate::command::Refused::Conflict(ARCHIVED_REFUND.into())))
    }
}

/// Is this archived order this venue's? A refusal that named an order of
/// another venue would tell a caller it exists (`Refused::NotFound`'s rule).
pub(super) fn archived_here(order_json: &str, location_id: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(order_json)
        .ok()
        .and_then(|v| v.get("location_id").and_then(serde_json::Value::as_str).map(|l| l == location_id))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::archived_here;

    #[test]
    fn an_archived_order_is_named_only_to_its_own_venue() {
        assert!(archived_here(r#"{"location_id":"qa-durres","status":"DELIVERED"}"#, "qa-durres"));
        assert!(!archived_here(r#"{"location_id":"other","status":"DELIVERED"}"#, "qa-durres"));
        assert!(!archived_here(r#"{"status":"DELIVERED"}"#, "qa-durres"));
        assert!(!archived_here("not json", "qa-durres"));
    }
}
