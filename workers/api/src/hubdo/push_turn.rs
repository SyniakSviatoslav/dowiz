//! THE PHONES' MESSAGES, WRITTEN IN THE TURN THAT CAUSED THEM (W-PUSH).
//!
//! The rule is pure (`notify::push::plan`); this file reads the `push` image
//! the object already holds, asks the rule what is owed, and writes the
//! outbox entries in ONE outbox write (`route_owed`, as the groups' messages
//! are) -- which also arms the venue's alarm, so the drain sends them.
//!
//! NOBODY SUBSCRIBED, NOTHING WRITTEN: a venue with no `push` image (most of
//! them today) pays one in-memory lookup per order turn.
//!
//! THE CUSTOMER'S DEVICES GO WHEN THE ORDER ENDS, after the last word is
//! queued (the entry carries its own keys), and any customer record past two
//! days goes with them -- in one `push` image write.
//!
//! A MESSAGE THAT CANNOT BE QUEUED NEVER UNDOES THE EVENT: loud, not fatal.

use super::routed::Owed;
use super::HubImages;
use crate::notify::push::plan::{self, Ev};
use crate::notify::push::subs::{self, Sub, IMAGE_PUSH, K_COURIER, K_CUSTOMER, K_STAFF, PUSH_BYTES};
use dowiz_hub::table::Table;
use worker::*;

impl HubImages {
    /// FROM `tell_orders` (a placement, or an `advance` command): `status`
    /// `None` = the order just placed, the last of `orders`.
    pub(super) async fn tell_push_orders(&self, status: Option<(&str, &str)>, orders: &[(String, String)], now_ms: i64) {
        match status {
            None => {
                if let Some((id, json)) = orders.last() {
                    self.tell_push(id, None, json, now_ms).await;
                }
            }
            Some((id, next)) => {
                let json = orders.iter().find(|(o, _)| o == id).map(|(_, j)| j.as_str()).unwrap_or("{}");
                self.tell_push(id, Some(next), json, now_ms).await;
            }
        }
    }

    /// FROM THE GENERIC APPEND (a courier's pickup and delivery, `hubstore::append_for`):
    /// an `Advanced` event's delta names the new status.
    pub(super) async fn tell_push_appended(&self, kind: u8, order_id: &str, payload: &str, now_ms: i64) {
        if kind != dowiz_hub::EventKind::Advanced as u8 {
            return;
        }
        let status = serde_json::from_str::<serde_json::Value>(payload)
            .ok()
            .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(str::to_string));
        if let Some(st) = status {
            self.tell_push(order_id, Some(&st), payload, now_ms).await;
        }
    }

    /// AFTER AN ORDER TURN: `status` is `None` for a placement, else the new
    /// status; `order_json` the order as the turn left it (its courier).
    pub(super) async fn tell_push(&self, order_id: &str, status: Option<&str>, order_json: &str, now_ms: i64) {
        let courier = serde_json::from_str::<serde_json::Value>(order_json)
            .ok()
            .and_then(|v| v.get("courier_id").and_then(|c| c.as_str()).map(str::to_string));
        let ev = match status {
            None => Ev::Placed { order: order_id },
            Some(s) => Ev::Status { order: order_id, status: s, courier: courier.as_deref() },
        };
        self.push_owed(ev, now_ms).await;
    }

    /// AFTER AN ASSIGNMENT: the courier's phones.
    pub(super) async fn tell_push_assigned(&self, order_id: &str, courier_id: &str, now_ms: i64) {
        self.push_owed(Ev::Assigned { order: order_id, courier: courier_id }, now_ms).await;
    }

    async fn push_owed(&self, ev: Ev<'_>, now_ms: i64) {
        let (generation, table) = match self.image(IMAGE_PUSH).await {
            Ok(Some((meta, b))) => match Table::load(&b, PUSH_BYTES) {
                Ok(t) => (meta.generation, t),
                Err(_) => return log_error!("push: the push image is unreadable; nobody was told"),
            },
            _ => return,
        };
        let mut held: Vec<(&str, String, Sub)> = Vec::new();
        for kind in [K_CUSTOMER, K_COURIER, K_STAFF] {
            for (id, j) in table.all(kind) {
                if let Ok(s) = serde_json::from_str::<Sub>(&j) {
                    held.push((kind, id, s));
                }
            }
        }
        let owed = plan::owed(ev, &held, now_ms);
        let mut remove = owed.remove;
        remove.extend(subs::stale_customers(&table.all(K_CUSTOMER), now_ms).into_iter().map(|id| (K_CUSTOMER, id)));
        if !owed.entries.is_empty() {
            self.route_owed(Owed { entries: owed.entries, ..Owed::default() }, "an order turn (push)").await;
        }
        if !remove.is_empty() {
            if let Err(e) = self.forget_push(generation, table, &remove).await {
                log_error!("push: {} subscription(s) due for removal were NOT removed: {e}", remove.len());
            }
        }
    }

    async fn forget_push(&self, generation: i64, mut table: Table, remove: &[(&'static str, String)]) -> Result<()> {
        for (k, id) in remove {
            table.remove(k, id);
        }
        let bytes = table.to_bytes().map_err(|x| Error::RustError(format!("push will not serialise: {x:?}")))?;
        match self.put_image(IMAGE_PUSH, generation, &bytes).await? {
            Some(_) => Ok(()),
            None => Err(Error::RustError("the push generation moved".into())),
        }
    }
}
