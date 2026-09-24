//! AN AGGREGATOR ORDER in the venue's object (§2.9): the platform's id is the
//! idempotency key, checked in the same turn as the placement, so two entries
//! of one platform order cannot both append `Placed`; a second entry with
//! different content is refused as a conflict (`same_entry`).

use super::HubImages;
use crate::command::aggregator::{existing, same_entry, AggregatorOut};
use crate::command::place::PlaceIn;
use crate::command::Refused;
use worker::*;

impl HubImages {
    pub(super) async fn aggregator(&self, input: PlaceIn) -> Result<std::result::Result<AggregatorOut, Refused>> {
        let (_, listed) = self.orders_view().await?;
        if let Some(o) = existing(&listed, &input.order_id) {
            // Same content: idempotent. Different content: a named conflict.
            return Ok(same_entry(&o.order_json, &input.envelope)
                .map(|()| AggregatorOut { stored: o.order_json.clone(), existing: true }));
        }
        Ok(self.place(input).await?.map(|out| AggregatorOut { stored: out.stored, existing: false }))
    }
}
