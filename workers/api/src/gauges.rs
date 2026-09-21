//! What an image reads, and what that reading is allowed to mean.
//!
//! THE ARENA IS THE LIMIT NOBODY SEES UNTIL IT BITES. A bebop store never
//! reclaims a generation, so an image is spent by the NUMBER OF WRITES as much
//! as by the data — measured at 313 empty commits before a fresh roster
//! refused. These are the figures that make that a thing the owner sees coming
//! rather than a thing that happens to them.
//!
//! THERE IS NO `dead` FIGURE. The superblock has a `superseded_cells` column
//! and nothing on this write path ever writes it, so a ratio built on it would
//! read 0 forever while looking like a measurement. See `dowiz_hub::Usage`.
//!
//! LIFTED OUT OF `extra.rs`, which is under a size ratchet, and these were the
//! two pieces of that handler that are pure functions of an image's usage —
//! so they are also the two that could never be tested where they were.

use serde_json::{json, Map, Value};

/// One image's reading.
pub fn gauge(u: dowiz_hub::Usage) -> Value {
    json!({
        "generation": u.generation,
        "usedCells": u.used_cells,
        // What the image holds now, and the most it may ever hold. These
        // differ for the compacted images: see `dowiz_hub::Usage`. The
        // ceiling is what `usedPerMille` measures against, because the
        // capacity of a compacted image is re-chosen on every save and a
        // ratio against it falls by half exactly when the image grows.
        "capacityCells": u.capacity_cells,
        "ceilingCells": u.ceiling_cells,
        // Per mille rather than a fraction: the kernel keeps no floats and
        // a percentage with one decimal is what a gauge shows anyway.
        "usedPerMille": u.used_per_mille(),
        // Does this image double itself instead of refusing? See
        // `dowiz_hub::Usage`. A reading of 900 means opposite things.
        "grows": u.grows,
    })
}

/// The worst compacted image, the worst growing one, and the verdict.
///
/// THE VERDICT COMES ONLY FROM THE IMAGES THAT CAN ACTUALLY REFUSE.
///
/// Measured: a stock log grew from 7168 cells to 523264 over four thousand
/// events and never refused once, and the order log doubles the same way. For
/// those, a reading near full predicts a DOUBLING — a few milliseconds of
/// copying — and counting it as the venue's worst problem put `dubin-durres`
/// on "watch" for a stock image in no danger at all, while the advice attached
/// to it, "compact", is something an append log cannot do. The compacted KV
/// images are the ones with a real ceiling, so they are the ones the verdict
/// is about. The growing one is still reported, because an image doubling
/// every week is worth seeing even though it is not an emergency.
pub fn verdict(images: &Map<String, Value>) -> (i64, i64, &'static str) {
    let per_mille = |v: &Value| v.get("usedPerMille").and_then(Value::as_i64);
    let grows = |v: &&Value| v.get("grows").and_then(Value::as_bool) == Some(true);
    let worst = images.values().filter(|v| !grows(v)).filter_map(per_mille).max().unwrap_or(0);
    let worst_growing = images.values().filter(grows).filter_map(per_mille).max().unwrap_or(0);
    let word = match worst {
        0..=699 => "ok",
        700..=899 => "watch",
        _ => "compact",
    };
    (worst, worst_growing, word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(used_per_mille: i64, grows: bool) -> Value {
        json!({ "usedPerMille": used_per_mille, "grows": grows })
    }

    /// THE WRONG ALARM THIS FUNCTION EXISTS TO NOT RAISE, pinned. A growing
    /// image at 967 per mille is a sawtooth about to double, and it once put a
    /// venue on "watch" with the advice `compact` — which an append log cannot
    /// do. It is reported, and it is not the verdict.
    #[test]
    fn a_growing_image_near_full_is_reported_and_is_not_the_verdict() {
        let mut m = Map::new();
        m.insert("stock".into(), img(967, true));
        m.insert("settings".into(), img(120, false));
        assert_eq!(verdict(&m), (120, 967, "ok"));
    }

    /// And the compacted image is the one that can actually refuse, so it is
    /// the one the three words are about.
    #[test]
    fn the_verdict_follows_the_compacted_images_across_both_thresholds() {
        let mut m = Map::new();
        m.insert("log".into(), img(999, true));
        m.insert("settings".into(), img(750, false));
        assert_eq!(verdict(&m), (750, 999, "watch"));
        m.insert("catalog".into(), img(900, false));
        assert_eq!(verdict(&m), (900, 999, "compact"));
    }

    /// A venue whose smaller images failed to load contributes no readings at
    /// all, and "no reading" must not read as "full". It is `ok` by absence —
    /// which is exactly why the conservation gate checks the images it was
    /// given rather than trusting this word.
    #[test]
    fn no_images_is_zero_and_not_a_thousand() {
        assert_eq!(verdict(&Map::new()), (0, 0, "ok"));
    }
}
