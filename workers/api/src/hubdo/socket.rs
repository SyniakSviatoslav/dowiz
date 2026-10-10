//! WHO HEARS WHAT (split out of `hubdo.rs`, W-SPLIT): the sockets' tags, the
//! broadcast of an event that landed, and the two frames a client may send.
//! The socket upgrade itself needs the platform and is in `hubdo/platform.rs`.

use super::{host, Change, HubImages, RECENT_KEEP};
use worker::{WebSocket, WebSocketIncomingMessage};

/// What a socket is subscribed to. Tags are how a hibernated object finds its
/// sockets again -- it has forgotten everything else about them.
///
/// THREE AUDIENCES, THREE TAGS, and the separation is not tidiness: a customer
/// watching one order must not be sent another customer's address, and the
/// broadcast is the only thing standing between the two.
pub const TAG_CONSOLE: &str = "console";
pub const TAG_COURIER: &str = "courier";
/// `order:<id>` — one customer, one order.
pub fn tag_order(order_id: &str) -> String {
    format!("order:{order_id}")
}
/// `courier:<id>` — WHICH courier, said by the Worker at connect time.
///
/// A socket carries this BESIDE `courier`, so the queue broadcast still finds
/// every courier with one tag while a GPS frame is attributed from the tag
/// rather than from the frame. A message cannot name a principal; a tag the
/// Worker attached can.
pub fn tag_courier(courier_id: &str) -> String {
    format!("{TAG_COURIER}:{courier_id}")
}

/// The courier a socket belongs to, from its tags.
fn courier_of(tags: &[String]) -> Option<String> {
    tags.iter().find_map(|t| t.strip_prefix("courier:").map(str::to_string))
}

impl HubImages {
    /// Tell everyone who is watching that an event landed.
    ///
    /// INTEREST MANAGEMENT, the oldest trick in multiplayer networking: the
    /// console hears about every order, a courier hears about the queue, and a
    /// customer hears about THEIR order and nothing else. The alternative --
    /// one channel carrying everything -- would put one customer's address on
    /// another customer's socket, and no amount of client-side filtering makes
    /// that acceptable.
    ///
    /// A BROADCAST IS NOT A REQUEST. Outgoing messages on an accepted socket
    /// are not billed as requests, which is the whole economy of this phase: a
    /// venue's consoles, couriers and customers stop asking every twelve
    /// seconds whether anything happened, and the object tells them when
    /// something does.
    pub(super) fn broadcast(&self, kind: u8, order_id: &str, payload: &str, generation: i64) {
        // ONLY ORDER EVENTS TRAVEL. A `Revealed` record is an audit fact under
        // a subject that is not an order id ("cust:<key>"), and its payload is
        // not a delta -- a client folding it would invent a row with a
        // reason and a reader and no items, and keep it. The Rust folds have
        // skipped non-order kinds since they were written; this is the same
        // rule on the wire, applied before the event leaves.
        //
        // The generation still moved, so the window gets a gap instead: the
        // next `?since=` is answered with "ask for the list".
        if dowiz_hub::EventKind::from_u8(kind).is_none_or(|k| !k.is_order()) {
            self.recent.borrow_mut().clear();
            let msg = serde_json::json!({ "t": "moved", "generation": generation }).to_string();
            for tag in [TAG_CONSOLE, crate::services::orders::kitchen_ack::board::TAG_KITCHEN] {
                for ws in self.state.get_websockets_with_tag(tag) {
                    let _ = ws.send_with_str(&msg);
                }
            }
            return;
        }
        {
            let mut recent = self.recent.borrow_mut();
            recent.push(Change {
                generation,
                kind,
                order_id: order_id.to_string(),
                payload: payload.to_string(),
            });
            if recent.len() > RECENT_KEEP {
                let cut = recent.len() - RECENT_KEEP;
                recent.drain(0..cut);
            }
        }
        let msg = serde_json::json!({
            "t": "event",
            "kind": kind,
            "orderId": order_id,
            "payload": payload,
            "generation": generation,
        })
        .to_string();
        let mut seen: Vec<String> = Vec::new();
        for tag in [TAG_CONSOLE.to_string(), TAG_COURIER.to_string(), tag_order(order_id)] {
            if seen.contains(&tag) {
                continue;
            }
            for ws in self.state.get_websockets_with_tag(&tag) {
                // A SEND THAT FAILS IS A SOCKET THAT WENT AWAY, which is the
                // ordinary end of every socket. It is not this append's
                // problem and must not become the caller's error.
                let _ = ws.send_with_str(&msg);
            }
            seen.push(tag);
        }
        // THE KITCHEN HEARS THE TICKET, NEVER THE CUSTOMER (W-KITCHEN K3): the
        // same event through the board's whitelist (`kitchen_ack::board`), so a
        // pass tablet is never sent a phone, an address or a price.
        let kitchen = crate::services::orders::kitchen_ack::board::frame(kind, order_id, payload, generation);
        for ws in self.state.get_websockets_with_tag(crate::services::orders::kitchen_ack::board::TAG_KITCHEN) {
            let _ = ws.send_with_str(&kitchen);
        }
    }

    /// One text frame from a socket (`websocket_message`): the answer to send
    /// back, if any. `tags` is the socket's, asked for only by a GPS frame.
    pub(super) fn on_message(&self, text: &str, tags: impl FnOnce() -> Vec<String>) -> Option<&'static str> {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else { return None };
        match v.get("t").and_then(serde_json::Value::as_str) {
            Some("ping") => return Some(r#"{"t":"pong"}"#),
            Some("gps") => {
                // WHOSE POSITION THIS IS COMES FROM THE TAG. It used to come
                // from the frame, so one courier's socket could move another
                // courier's pin on the venue's map -- and through it every
                // customer's estimate. The tag was attached by the Worker from
                // a verified claim; the frame is whatever was typed into it.
                let Some(courier) = courier_of(&tags()) else {
                    return None;
                };
                let (Some(lat), Some(lng)) = (
                    v.get("lat_e6").and_then(serde_json::Value::as_i64),
                    v.get("lng_e6").and_then(serde_json::Value::as_i64),
                ) else {
                    return None;
                };
                // MICRO-DEGREES, as integers, exactly as an order carries them:
                // a float crossing into this system is what MANIFESTO C2
                // forbids, and a position is not an exception.
                if !(-90_000_000..=90_000_000).contains(&lat)
                    || !(-180_000_000..=180_000_000).contains(&lng)
                {
                    return None;
                }
                self.positions.borrow_mut().insert(
                    courier,
                    self.fix_now(lat, lng), // the object's clock, read in `hubdo.rs`
                );
            }
            _ => {}
        }
        None
    }

    /// What a client may say over its socket (`websocket_message`).
    ///
    /// DELIBERATELY SMALL. A socket is not a second API: everything that
    /// changes an order still goes through the Worker, where the kernel
    /// decides and the caller is authenticated per request. Two messages are
    /// accepted here and both are about the connection itself.
    ///
    ///   `{"t":"ping"}`  — keep-alive; answered with a pong.
    ///   `{"t":"gps", "courier":"…", "lat_e6":…, "lng_e6":…}` — a courier's
    ///   position, which lands in memory and in no table.
    ///
    /// A MESSAGE FROM A SOCKET IS NOT A PRINCIPAL. The tag the Worker attached
    /// at accept time is what this object knows about the client, so a GPS fix
    /// is taken only from a socket tagged `courier`; a console or a customer
    /// saying the same thing is ignored rather than believed.
    pub(super) fn socket_frame(&self, ws: &WebSocket, message: WebSocketIncomingMessage) {
        let WebSocketIncomingMessage::String(text) = message else {
            // Binary frames mean nothing here. Dropping them is the whole
            // handling: answering would teach a client to send more.
            return;
        };
        // The tags are asked for only when a frame needs them (`on_message`).
        let tags = || match &self.state {
            host::Host::Live { state, .. } => state.get_tags(ws),
            #[cfg(test)]
            host::Host::Mem(_) => Vec::new(),
        };
        if let Some(answer) = self.on_message(&text, tags) {
            let _ = ws.send_with_str(answer);
        }
    }
}
