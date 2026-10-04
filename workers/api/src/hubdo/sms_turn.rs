//! THE CUSTOMER'S TEXT, WRITTEN IN THE TURN THAT CAUSED IT (W-SMS).
//!
//! The rule is pure (`notify::sms::plan::owed`); this file finds the order's
//! STAMP (kept in the outbox image by `notify::sms::checkout::keep` when the
//! box was ticked) and writes the one entry in ONE outbox write
//! (`route_owed`), which arms the venue's alarm, so the drain sends it. The
//! same write removes the stamp when the order ends, and any stamp past its
//! life (`STAMP_TTL_MS`): the number does not outlive the order.
//!
//! CHEAP WHEN UNUSED: a venue with SMS off pays one settings read per status
//! change only if the outbox holds a stamp for this order; most orders have
//! none, decided by one in-memory lookup of an image the object already holds.
//!
//! A TEXT THAT CANNOT BE QUEUED NEVER UNDOES THE EVENT: loud, not fatal.

use super::routed::Owed;
use super::HubImages;
use crate::notify::sms::checkout::{stale, STAMP_KIND};
use crate::notify::sms::{config, plan};
use crate::outbox::{IMAGE_OUTBOX, OUTBOX_BYTES};
use dowiz_hub::table::Table;
use serde_json::Value;

impl HubImages {
    /// FROM `tell_orders` (an `advance` command): `status` is `(order, next)`;
    /// `orders` the orders as this turn left them. A placement (`None`) is
    /// PENDING, which no customer is texted about.
    pub(super) async fn tell_sms_orders(&self, status: Option<(&str, &str)>, orders: &[(String, String)], now_ms: i64) {
        let Some((id, next)) = status else { return };
        let Some(order) = orders.iter().find(|(o, _)| o == id).and_then(|(_, j)| serde_json::from_str::<Value>(j).ok()) else { return };
        self.sms_owed(id, order, next, now_ms).await;
    }

    /// FROM THE GENERIC APPEND (a courier's pickup: IN_DELIVERY; delivered):
    /// the delta names the status. The fulfilment is the stamp's business only
    /// for READY, which a courier never sends.
    pub(super) async fn tell_sms_appended(&self, kind: u8, order_id: &str, payload: &str, now_ms: i64) {
        if kind != dowiz_hub::EventKind::Advanced as u8 {
            return;
        }
        let Some(status) = serde_json::from_str::<Value>(payload).ok().and_then(|v| v.get("status").and_then(Value::as_str).map(str::to_string)) else { return };
        let order = serde_json::json!({ "fulfilment": { "kind": "delivery" } });
        self.sms_owed(order_id, order, &status, now_ms).await;
    }

    async fn sms_owed(&self, order_id: &str, mut order: Value, status: &str, now_ms: i64) {
        let table = match self.image(IMAGE_OUTBOX).await {
            Ok(Some((_, b))) => match Table::load(&b, OUTBOX_BYTES) {
                Ok(t) => t,
                Err(_) => return,
            },
            _ => return,
        };
        let Some(stamp) = table.get(STAMP_KIND, order_id) else { return };
        let mut owed = Owed::default();
        // THE NUMBER GOES WITH THE ORDER: an ended order's stamp, and any past its life.
        if crate::services::orders::status::is_terminal(status) {
            owed.unmark.push((STAMP_KIND, order_id.to_string()));
        }
        owed.unmark.extend(table.all(STAMP_KIND).into_iter().filter(|(id, s)| id != order_id && stale(s, now_ms)).map(|(id, _)| (STAMP_KIND, id)));
        let on = match self.image(crate::hubstore::IMAGE_SETTINGS).await {
            Ok(Some((_, b))) => dowiz_hub::settings::Settings::load(&b).map(|s| config::is_on(&s)).unwrap_or(false),
            _ => false,
        };
        if on && !stale(&stamp, now_ms) {
            order["sms"] = serde_json::from_str(&stamp).unwrap_or(Value::Null);
            owed.entries.extend(plan::owed(order_id, &order, status, now_ms));
        }
        self.route_owed(owed, "an order turn (sms)").await;
    }
}
