//! The owner's health row for the taste sheaf (W-TASTE2 S7a): one read of the taste image.

use serde_json::json;
use super::{taste, IMAGE_TASTE, KIND, TASTE_BYTES};
use crate::hubstore::Place;

/// W-TASTE2 S7a: the consistency radius between the phones' vectors and the venue's profiles, for
/// the owner's health page (a figure, never an alarm) and the log. An unreadable image says so.
pub async fn radius_health(place: &Place) -> serde_json::Value {
    match crate::hubstore::load_table(place, IMAGE_TASTE, TASTE_BYTES).await {
        Ok(held) => {
            let all: Vec<taste::Profile> = held.table.all(KIND).iter().filter_map(|(_, j)| taste::parse(j)).collect();
            let s = taste::agreement::summary(&all);
            log_line!("sheaf.radius venue={} cover=phone|venue compared={} max_pm={} median_pm={} over_half={}", place.venue, s.compared, s.max_pm, s.median_pm, s.over_half);
            json!({ "taste": s.json(), "stock": "needs S0 glue (not built)" })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}
