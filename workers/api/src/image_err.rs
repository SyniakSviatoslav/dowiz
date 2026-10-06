//! THE ONE PLACE AN IMAGE THAT WILL NOT LOAD IS SAID OUT LOUD (W-CRC, 2026-10-05).
//!
//! Every loader in `hubstore.rs` / `hubdo.rs` answered a refused image with
//! `map_err(|_| "<kind> image is unreadable")`: the request fails with a 500, which is
//! right (a corrupt image must never be replaced by an empty one -- that would show a
//! venue with no orders as a healthy venue), but the REASON was thrown away, so nobody
//! could tell a truncation from a changed byte from "not a hub image".
//!
//! Same answer to the caller, same message, and one named console line with the
//! loader's typed reason -- `HubError::BadCrc(BadCrc { obj, want, got })` names the
//! object whose crc failed. The line is greppable as `image-refused:` in `wrangler tail`
//! / Workers Logs. The venue does not go dark SILENTLY; it goes dark with this line.

/// The error every loader returns for a refused image, after logging why.
pub(crate) fn unreadable<E: core::fmt::Debug>(what: &str, e: E) -> worker::Error {
    log_error!("image-refused: the {what} image is unreadable: {e:?}");
    worker::Error::RustError(format!("{what} image is unreadable"))
}

/// A hub log that LOADED with records quarantined for a failed crc (operator policy
/// 2026-10-05: append logs serve around a bad record). Same hub back; one named line,
/// `image-quarantined:`, naming each record's cell. The owner sees the same records in
/// `/api/owner/health`'s quarantine list (reason `crc`).
pub(crate) fn hub_loaded(hub: dowiz_hub::Hub) -> dowiz_hub::Hub {
    let bad = hub.crc_quarantined();
    if !bad.is_empty() {
        log_error!("image-quarantined: the hub log serves around {} record(s) whose crc failed: {bad:?}", bad.len());
    }
    hub
}

#[cfg(test)]
mod tests {
    /// Same message the callers always returned: no route's text changes.
    #[test]
    fn the_message_is_unchanged_and_the_reason_is_not_swallowed() {
        let e = super::unreadable("catalogue", "BadCrc(obj 1234)");
        assert!(matches!(e, worker::Error::RustError(ref m) if m == "catalogue image is unreadable"), "{e:?}");
    }
}
