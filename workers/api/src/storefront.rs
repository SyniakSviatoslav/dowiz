//! The public storefront: read the menu, place an order.
//!
//! THE MONEY RULE, and it is the reason this module exists rather than the
//! client posting a total: every line's price is RE-DERIVED from `products` here
//! before the kernel ever sees the order. Whatever `unit_price` the browser sent
//! is discarded. A client that inflates a discount, or deflates a price, changes
//! nothing -- which is the same guarantee `place_order_priced` gives natively,
//! reached through the JSON boundary.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use worker::*;

use crate::auth;
use dowiz_kernel::json_api;

#[derive(Deserialize)]
pub struct LineIn {
    pub product_id: String,
    #[serde(default)]
    pub modifier_ids: Vec<String>,
    pub quantity: i64,
}

#[derive(Deserialize)]
pub struct ContactIn {
    #[serde(default)]
    pub name: Option<String>,
    pub phone: String,
}

#[derive(Deserialize)]
pub struct AddressIn {
    pub line: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub struct FulfilmentIn {
    pub kind: String,
    #[serde(default)]
    pub address: Option<AddressIn>,
}

#[derive(Deserialize)]
pub struct PlaceIn {
    pub items: Vec<LineIn>,
    pub contact: ContactIn,
    pub fulfilment: FulfilmentIn,
    #[serde(default)]
    pub payment: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
}

#[derive(Serialize)]
struct LocationOut {
    id: String,
    name: String,
    slug: String,
    phone: String,
    address: Option<String>,
    status: String,
    #[serde(rename = "closesAt")]
    closes_at: Option<String>,
    #[serde(rename = "deliveryEta")]
    delivery_eta: String,
    #[serde(rename = "deliveryFee")]
    delivery_fee: i64,
    #[serde(rename = "freeDeliveryThreshold")]
    free_delivery_threshold: Option<i64>,
    #[serde(rename = "minOrder")]
    min_order: i64,
    #[serde(rename = "currencyCode")]
    currency_code: String,
    #[serde(rename = "menuVersion")]
    menu_version: i64,
    #[serde(rename = "supportedLocales")]
    supported_locales: Value,
    #[serde(rename = "defaultLocale")]
    default_locale: String,
}

#[derive(Deserialize)]
struct LocRow {
    id: String,
    name: String,
    slug: String,
    phone: String,
    address: Option<String>,
    status: String,
    closes_at: Option<String>,
    delivery_eta: String,
    delivery_fee: i64,
    free_delivery_threshold: Option<i64>,
    min_order: i64,
    currency_code: String,
    menu_version: i64,
    supported_locales: String,
    default_locale: String,
    delivery_paused: i64,
}

#[derive(Deserialize)]
struct ProdRow {
    id: String,
    category_id: Option<String>,
    category_name: Option<String>,
    category_sort: Option<i64>,
    name: String,
    description: Option<String>,
    price: i64,
    available: i64,
    unavailable_note: Option<String>,
    image_url: Option<String>,
    sort_order: i64,
}

/// `GET /api/public/locations/:slug/menu`
pub async fn menu(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let db = ctx.d1("DB")?;

    let loc: Option<LocRow> = db
        .prepare(
            "SELECT id,name,slug,phone,address,status,closes_at,delivery_eta,delivery_fee, \
             free_delivery_threshold,min_order,currency_code,menu_version,supported_locales, \
             default_locale,delivery_paused FROM locations WHERE slug = ?1",
        )
        .bind(&[slug.into()])?
        .first(None)
        .await?;
    let Some(loc) = loc else {
        return Response::error("not found", 404);
    };

    // A paused venue is CLOSED to the storefront even if its status says open --
    // the old platform had `delivery_paused` as a separate kill switch so an
    // owner could stop the queue without changing opening hours.
    let status = if loc.delivery_paused == 1 { "closed".to_string() } else { loc.status.clone() };

    let rows = db
        .prepare(
            "SELECT p.id AS id, p.category_id AS category_id, c.name AS category_name, \
             c.sort_order AS category_sort, p.name AS name, p.description AS description, \
             p.price AS price, p.available AS available, p.unavailable_note AS unavailable_note, \
             p.image_url AS image_url, p.sort_order AS sort_order \
             FROM products p LEFT JOIN categories c ON c.id = p.category_id \
             WHERE p.location_id = ?1 ORDER BY c.sort_order, p.sort_order, p.name",
        )
        .bind(&[loc.id.clone().into()])?
        .all()
        .await?
        .results::<ProdRow>()?;

    // Group in one pass, preserving the SQL order rather than re-sorting.
    let mut cats: Vec<Value> = Vec::new();
    let mut last: Option<String> = None;
    for p in rows {
        let cid = p.category_id.clone().unwrap_or_else(|| "uncategorised".into());
        if last.as_deref() != Some(cid.as_str()) {
            cats.push(json!({
                "id": cid, "name": p.category_name.clone().unwrap_or_else(|| "—".into()),
                "sortOrder": p.category_sort.unwrap_or(0), "products": []
            }));
            last = Some(cid);
        }
        let entry = json!({
            "id": p.id, "name": p.name, "description": p.description,
            "price": p.price, "available": p.available == 1,
            "unavailableNote": p.unavailable_note, "imageUrl": p.image_url,
            "sortOrder": p.sort_order
        });
        if let Some(Value::Array(a)) = cats.last_mut().and_then(|c| c.get_mut("products")) {
            a.push(entry);
        }
    }

    let out = json!({
        "location": LocationOut {
            id: loc.id, name: loc.name, slug: loc.slug, phone: loc.phone, address: loc.address,
            status, closes_at: loc.closes_at, delivery_eta: loc.delivery_eta,
            delivery_fee: loc.delivery_fee, free_delivery_threshold: loc.free_delivery_threshold,
            min_order: loc.min_order, currency_code: loc.currency_code,
            menu_version: loc.menu_version,
            supported_locales: serde_json::from_str(&loc.supported_locales)
                .unwrap_or_else(|_| json!(["sq"])),
            default_locale: loc.default_locale,
        },
        "categories": cats
    });
    let mut res = Response::from_json(&out)?;
    // The menu is public and changes rarely; the version field is what a client
    // uses to notice it moved. Short edge cache, revalidated.
    res.headers_mut().set("cache-control", "public, max-age=30, stale-while-revalidate=300")?;
    Ok(res)
}

/// `POST /api/public/locations/:slug/orders`
pub async fn place(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(slug) = ctx.param("slug").cloned() else {
        return Response::error("missing slug", 400);
    };
    let body: PlaceIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if body.items.is_empty() {
        return Response::error("empty order", 400);
    }
    if body.contact.phone.chars().filter(|c| c.is_ascii_digit()).count() < 8 {
        return Response::error("invalid phone", 400);
    }
    if body.fulfilment.kind == "delivery"
        && body.fulfilment.address.as_ref().map_or(true, |a| a.line.trim().is_empty())
    {
        return Response::error("delivery address required", 400);
    }

    let db = ctx.d1("DB")?;
    let loc: Option<LocRow> = db
        .prepare(
            "SELECT id,name,slug,phone,address,status,closes_at,delivery_eta,delivery_fee, \
             free_delivery_threshold,min_order,currency_code,menu_version,supported_locales, \
             default_locale,delivery_paused FROM locations WHERE slug = ?1",
        )
        .bind(&[slug.into()])?
        .first(None)
        .await?;
    let Some(loc) = loc else { return Response::error("not found", 404) };
    if loc.delivery_paused == 1 || loc.status == "closed" {
        return Response::error("venue is closed", 409);
    }

    // ── re-derive every price from the catalogue ──
    #[derive(Deserialize)]
    struct PriceRow { id: String, price: i64, available: i64 }
    let mut lines = Vec::with_capacity(body.items.len());
    let mut subtotal: i64 = 0;
    for it in &body.items {
        if it.quantity < 1 || it.quantity > 99 {
            return Response::error("invalid quantity", 400);
        }
        let row: Option<PriceRow> = db
            .prepare("SELECT id, price, available FROM products WHERE id = ?1 AND location_id = ?2")
            .bind(&[it.product_id.clone().into(), loc.id.clone().into()])?
            .first(None)
            .await?;
        let Some(row) = row else {
            // Fail CLOSED on an unknown product rather than pricing it at zero.
            return Response::error(format!("unknown product: {}", it.product_id), 400);
        };
        if row.available != 1 {
            return Response::error(format!("unavailable: {}", it.product_id), 409);
        }
        subtotal += row.price * it.quantity;
        lines.push(json!({
            "product_id": row.id, "modifier_ids": it.modifier_ids,
            "quantity": it.quantity, "unit_price": row.price   // trusted, from D1
        }));
    }
    if subtotal < loc.min_order {
        return Response::error("below minimum order", 409);
    }

    let fee = match loc.free_delivery_threshold {
        Some(th) if subtotal >= th => 0,
        _ if body.fulfilment.kind == "pickup" => 0,
        _ => loc.delivery_fee,
    };

    let Some(id) = crate::edge_id() else {
        return Response::error("no platform CSPRNG for order id", 500);
    };
    let created_at_ms = Date::now().as_millis() as i64;

    let order_json = match json_api::place_order_at(
        id.clone(),
        None,
        &serde_json::to_string(&lines).unwrap_or_else(|_| "[]".into()),
        created_at_ms,
        Some("storefront".into()),
    ) {
        Ok(j) => j,
        Err(e) => return Response::error(e, 400),
    };

    // The kernel owns items and subtotal. Delivery, contact and fulfilment are
    // carried alongside until the aggregate's new fields reach this boundary.
    let mut envelope: Value = serde_json::from_str(&order_json)
        .map_err(|e| Error::RustError(format!("kernel order json unreadable: {e}")))?;
    let total = subtotal + fee;
    envelope["delivery_fee"] = json!(fee);
    envelope["total"] = json!(total);
    envelope["location_id"] = json!(loc.id);
    envelope["contact"] = json!({ "name": body.contact.name, "phone": body.contact.phone });
    envelope["fulfilment"] = json!({
        "kind": body.fulfilment.kind,
        "address": body.fulfilment.address.as_ref().map(|a| json!({ "line": a.line, "note": a.note })),
        "fee": fee
    });
    envelope["payment"] = json!(body.payment.unwrap_or_else(|| "cash".into()));

    let phone_hash = auth::sha256_hex(&body.contact.phone);
    let stored = serde_json::to_string(&envelope).unwrap_or(order_json);

    // The order goes into the HUB's event log, not into a table. An order's state
    // is a fold over what happened to it, so there is one place it can be read
    // from and no row that could disagree with the log.
    let seq = created_at_ms as u64;
    let ev_id = id.clone();
    let ev_json = stored.clone();
    crate::hubstore::with_hub(&db, move |hub| {
        hub.append(dowiz_hub::EventKind::Placed, &ev_id, &ev_json, seq, [0u8; 32])
            .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))
    })
    .await?;

    // The customer row is keyed by a HASH of the phone, never the phone itself,
    // so the table can be joined without holding the number in the clear.
    let cust_id = crate::edge_id().unwrap_or_else(|| format!("cust_{created_at_ms}"));
    let _ = db
        .prepare(
            "INSERT INTO customers (id,location_id,phone_hash,name,created_at_ms) VALUES (?1,?2,?3,?4,?5) \
             ON CONFLICT(location_id,phone_hash) DO UPDATE SET name = COALESCE(excluded.name, customers.name)",
        )
        .bind(&[cust_id.into(), loc.id.into(), phone_hash.into(),
                body.contact.name.clone().unwrap_or_default().into(),
                worker::wasm_bindgen::JsValue::from_f64(created_at_ms as f64)])?
        .run()
        .await;

    let mut res = Response::ok(stored)?;
    res.headers_mut().set("content-type", "application/json; charset=utf-8")?;
    Ok(res)
}
