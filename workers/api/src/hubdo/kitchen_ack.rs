//! THE KITCHEN'S "SEEN", in the venue's object, in one turn: the ack decided in
//! memory (`command::kitchen_ack::decide`), the log written, then broadcast so
//! every console draws "seen" at once. A repeated ack writes and broadcasts
//! nothing, and answers the order as it stands.

use super::{HubImages, OrderView};
use crate::command::kitchen_ack::{KitchenAckIn, KitchenAckOut};
use crate::command::Refused;
use worker::*;

impl HubImages {
    pub(super) async fn kitchen_ack(&self, input: KitchenAckIn) -> Result<std::result::Result<KitchenAckOut, Refused>> {
        let (log_gen, listed) = self.orders_view().await?;
        let current: Option<OrderView> = listed.into_iter().find(|o| o.order_id == input.order_id);
        let seq_before = current.as_ref().map_or(0, |c| c.seq);
        let (_, mut hub) = self.log_hub().await?;
        let (merged, write) = match crate::command::kitchen_ack::decide(&mut hub, current.as_ref(), &input) {
            Ok(v) => v,
            // NOTHING HAS BEEN WRITTEN.
            Err(r) => return Ok(Err(r)),
        };
        let Some((body, seq)) = write else {
            return Ok(Ok(KitchenAckOut { merged: merged.to_string(), seq: seq_before, generation: log_gen, fresh: false }));
        };
        let next = match self.write_both("a kitchen ack", log_gen, &hub, None).await? {
            Ok(n) => n,
            Err(r) => return Ok(Err(r)),
        };
        self.broadcast(dowiz_hub::EventKind::Noted as u8, &input.order_id, &body, next);
        Ok(Ok(KitchenAckOut { merged: merged.to_string(), seq, generation: next, fresh: true }))
    }
}
