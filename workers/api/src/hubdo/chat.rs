//! THE CHAT NUDGE: `POST /fold/chat?order=<id>&courier=<id>`.
//!
//! A courier chat message is appended to the `threads` image by the Worker
//! (`services/orders/chat.rs`); this object holds the sockets, so this is
//! where the two parties learn the thread moved. The frame says WHICH order
//! and nothing else -- the client asks the route for the text, which is the
//! same shape every other socket frame keeps (`broadcast`). It goes to the
//! customer's tag and the courier's tag ONLY: never the console, which would
//! put the conversation on every screen in the venue, and never the kitchen.

use super::HubImages;
use crate::wire::{Call, Reply};
use worker::*;

impl HubImages {
    pub(super) fn chat_nudge(&self, req: &Call) -> Result<Reply> {
        let url = req.url()?;
        let get = |k: &str| url.query_pairs().find(|(q, _)| q == k).map(|(_, v)| v.to_string()).unwrap_or_default();
        let (order, courier) = (get("order"), get("courier"));
        if order.is_empty() {
            return Reply::error("a chat nudge names its order", 400);
        }
        let msg = serde_json::json!({ "t": "chat", "orderId": order }).to_string();
        let mut tags = vec![super::tag_order(&order)];
        if !courier.is_empty() {
            tags.push(super::tag_courier(&courier));
        }
        for tag in tags {
            for ws in self.state.get_websockets_with_tag(&tag) {
                // A send that fails is a socket that went away; the poll is the fallback.
                let _ = ws.send_with_str(&msg);
            }
        }
        Reply::ok("nudged")
    }
}
