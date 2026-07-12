//! HTTP DTOs + conversions between wire types and the kernel domain.
//!
//! The kernel's `Order` / `OrderItem` / `OrderStatus` types intentionally do NOT
//! derive `serde` (they're a pure domain library, also compiled to WASM). So the
//! server defines its own Serializable wire shapes and converts at the boundary.
//!
//! RED LINE: no courier scoring / rating fields here. The server surface carries
//! only order + channel + push-subscription data.

use dowiz_kernel::{Order, OrderItem, OrderStatus};
use serde::{Deserialize, Serialize};

/// Request body for `POST /api/orders`.
///
/// Accepts both snake_case and camelCase aliases for each field (the legacy API is
/// inconsistent: `locationId` is camelCase while item `product_id`/`unit_price` are
/// snake_case). Aliases keep the server tolerant of either wire convention.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOrderRequest {
    #[serde(alias = "locationId")]
    pub location_id: String,
    pub items: Vec<CreateItemRequest>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default, alias = "cashPayWith")]
    pub cash_pay_with: Option<String>,
}

/// One line item in a `CreateOrderRequest`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateItemRequest {
    #[serde(alias = "productId")]
    pub product_id: String,
    #[serde(default, alias = "modifierIds")]
    pub modifier_ids: Vec<String>,
    pub quantity: i64,
    #[serde(alias = "unitPrice")]
    pub unit_price: i64,
}

/// Request body for `POST /api/orders/:id/event`.
#[derive(Debug, Clone, Deserialize)]
pub struct EventRequest {
    /// Target status string (e.g. `"CONFIRMED"`).
    pub next_status: String,
}

/// Request body for `POST /api/courier/push/subscribe`.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscribeRequest {
    pub courier_id: String,
    pub endpoint: String,
    pub auth: String,
    pub p256dh: String,
}

/// Wire shape of an order item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemResponse {
    pub product_id: String,
    pub modifier_ids: Vec<String>,
    pub quantity: i64,
    pub unit_price: i64,
}

/// Wire shape of an order (kernel `Order` projected to JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub id: String,
    pub customer_id: Option<String>,
    pub status: String,
    pub items: Vec<ItemResponse>,
    pub subtotal: i64,
    pub total: i64,
    pub created_at_ms: i64,
    pub channel: Option<String>,
    pub cash_pay_with: Option<String>,
}

/// Wire shape of a push subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushSubResponse {
    pub id: String,
    pub courier_id: String,
    pub endpoint: String,
    pub created_at_ms: i64,
}

/// Request body for `POST /api/venues/:id/claim`.
#[derive(Debug, Clone, Deserialize)]
pub struct ClaimVenueRequest {
    pub owner_id: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Response for `GET /api/venues/:id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VenueResponse {
    pub id: String,
    pub name: String,
    pub claimed: bool,
    pub owner_id: Option<String>,
}

/// Channel count entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCount {
    pub channel: String,
    pub count: u64,
}

/// Response for `GET /api/orders/channel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelResponse {
    pub orders_by_channel: Vec<ChannelCount>,
}

/// A serializable projection of the kernel `Order` used for persistence in the
/// SQLite `payload` column. Items + scalars are flattened so the row can be
/// rehydrated into a kernel `Order` on read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredOrder {
    pub id: String,
    pub customer_id: Option<String>,
    pub status: String,
    pub items: Vec<ItemResponse>,
    pub subtotal: i64,
    pub total: i64,
    pub created_at_ms: i64,
    pub channel: Option<String>,
    pub cash_pay_with: Option<String>,
}

// ── conversions ──────────────────────────────────────────────────────────

impl From<&Order> for StoredOrder {
    fn from(o: &Order) -> Self {
        StoredOrder {
            id: o.id.clone(),
            customer_id: o.customer_id.clone(),
            status: o.status.as_str().to_string(),
            items: o
                .items
                .iter()
                .map(|i| ItemResponse {
                    product_id: i.product_id.clone(),
                    modifier_ids: i.modifier_ids.clone(),
                    quantity: i.quantity,
                    unit_price: i.unit_price,
                })
                .collect(),
            subtotal: o.subtotal,
            total: o.total,
            created_at_ms: o.created_at_ms,
            channel: o.channel.clone(),
            cash_pay_with: o.cash_pay_with.clone(),
        }
    }
}

impl From<&StoredOrder> for Order {
    fn from(s: &StoredOrder) -> Self {
        let status = OrderStatus::from_str(&s.status).unwrap_or(OrderStatus::Pending);
        Order {
            id: s.id.clone(),
            customer_id: s.customer_id.clone(),
            status,
            items: s
                .items
                .iter()
                .map(|i| OrderItem {
                    product_id: i.product_id.clone(),
                    modifier_ids: i.modifier_ids.clone(),
                    quantity: i.quantity,
                    unit_price: i.unit_price,
                })
                .collect(),
            subtotal: s.subtotal,
            total: s.total,
            created_at_ms: s.created_at_ms,
            channel: s.channel.clone(),
            cash_pay_with: s.cash_pay_with.clone(),
        }
    }
}

impl From<&Order> for OrderResponse {
    fn from(o: &Order) -> Self {
        OrderResponse {
            id: o.id.clone(),
            customer_id: o.customer_id.clone(),
            status: o.status.as_str().to_string(),
            items: o
                .items
                .iter()
                .map(|i| ItemResponse {
                    product_id: i.product_id.clone(),
                    modifier_ids: i.modifier_ids.clone(),
                    quantity: i.quantity,
                    unit_price: i.unit_price,
                })
                .collect(),
            subtotal: o.subtotal,
            total: o.total,
            created_at_ms: o.created_at_ms,
            channel: o.channel.clone(),
            cash_pay_with: o.cash_pay_with.clone(),
        }
    }
}

/// Build kernel `OrderItem`s from a create request.
pub fn kernel_items(req_items: &[CreateItemRequest]) -> Vec<OrderItem> {
    req_items
        .iter()
        .map(|i| OrderItem {
            product_id: i.product_id.clone(),
            modifier_ids: i.modifier_ids.clone(),
            quantity: i.quantity,
            unit_price: i.unit_price,
        })
        .collect()
}
