//! Records the product holds and this build cannot read.
//!
//! QUARANTINE IS A NUMBER OR IT IS NOTHING. A record that fails to parse is
//! left out of `events()` and named here instead — and a non-zero count is a
//! FAILING GATE (`e2e/gates/conservation.mjs`), not a warning, because a
//! quarantine nobody notices is a data-loss feature with better manners. It is
//! the same mistake as the storage error that was read as "no image": a
//! failure converted into an absence.
//!
//! NOTHING HERE REPAIRS ANYTHING. The ids are carried so a human can find the
//! record in the image, verbatim; an automatic repair of a record nobody has
//! looked at is how a corrupted order becomes a plausible one.
//!
//! ITS OWN FILE because `extra.rs` is under a size ratchet and the blueprint's
//! phase 2 is dismantling it. A new feature that lands in the largest file in
//! the tree is the habit the ratchet exists to break.

use serde_json::{json, Value};

/// Every unreadable record across the venue's logs, each naming its image.
///
/// EVERY LOG, NOT ONLY THE ORDERS. The audit image holds this venue's failures
/// and its courier audit, so a quarantine there is the INSTRUMENT losing
/// records — the one loss that would otherwise go unreported by construction.
pub fn seen(hub: &dowiz_hub::Hub, audit: Vec<dowiz_hub::Quarantined>) -> Vec<Value> {
    let named = |image: &'static str, qs: Vec<dowiz_hub::Quarantined>| {
        qs.into_iter()
            .map(move |q| json!({ "image": image, "id": q.id, "at": q.at, "reason": q.reason }))
    };
    named("log", hub.quarantined()).chain(named("audit", audit)).collect()
}

#[cfg(test)]
mod tests {
    /// The join is the whole function, and the thing it must not do is lose
    /// which image a record came from — a bare id would send a human looking
    /// through the wrong image for a record that was never in it.
    #[test]
    fn each_record_names_the_image_it_came_from() {
        let audit = vec![dowiz_hub::Quarantined { id: "ab".repeat(32), at: 3, reason: "kind" }];
        let hub = dowiz_hub::Hub::create_sized(1 << 16).unwrap();
        let out = super::seen(&hub, audit);
        assert_eq!(out.len(), 1, "a healthy order log contributes nothing");
        assert_eq!(out[0]["image"], "audit");
        assert_eq!(out[0]["at"], 3);
    }
}
