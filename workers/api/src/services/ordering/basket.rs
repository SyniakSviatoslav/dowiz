//! THE BASKET'S CATALOGUE NODES, answered by the venue's object (BN1, R3,
//! `/fold/basket`, `hubdo/basket.rs`).
//!
//! A placement, the promo preview, an aggregator entry and a round's added
//! line each pulled the whole catalogue image across the Worker<->object hop
//! to read a handful of records: the venue's own, the dishes in the basket,
//! what the shelf reserves for each, and one promo code. The object holds that
//! image; it derives those nodes here (`answer`, PURE) and the Worker asks for
//! them (`ask`). The Worker keeps its auth, its venue checks and THE pricer.
//!
//!   GET /fold/basket?ids=<product id>&ids=...[&promo=<normalised code>]

use dowiz_hub::catalog::Catalog;
use serde_json::{json, Value};
use std::collections::HashMap;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

/// What a basket needs of the catalogue, derived where the catalogue is. PURE.
///
/// `venue`: the venue's record, or `null`; `products`: the records of the ids
/// asked for that exist; `ledger`: per existing id, the record the shelf
/// reserves against for one sale of it (`prep::for_ledger`, the semi-finished
/// products expanded); `promo`: the code's record as stored, or `null`.
pub fn answer(cat: &Catalog, ids: &[String], promo: Option<&str>) -> Value {
    let venue: Value = cat.location().and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(Value::Null);
    let mut products = serde_json::Map::new();
    let mut ledger = serde_json::Map::new();
    for id in ids {
        if products.contains_key(id) {
            continue;
        }
        let Some(j) = cat.product(id) else { continue };
        ledger.insert(id.clone(), json!(dowiz_hub::prep::for_ledger(&|s| cat.supply(s), &j).0));
        products.insert(id.clone(), serde_json::from_str(&j).unwrap_or(Value::Null));
    }
    let promo = promo.and_then(|c| cat.promo(c));
    json!({ "venue": venue, "products": products, "ledger": ledger, "promo": promo })
}

/// The object's answer, as the Worker reads it.
#[derive(Default, serde::Deserialize)]
pub struct Basket {
    pub venue: Option<Value>,
    pub products: HashMap<String, Value>,
    pub ledger: HashMap<String, String>,
    pub promo: Option<String>,
}

impl Basket {
    /// The dish's record as the pricer reads it: what `cat.product(id)`
    /// answered before the move, `None` for a dish the catalogue does not have.
    pub fn product(&self, id: &str) -> Option<String> {
        self.products.get(id).filter(|v| !v.is_null()).map(Value::to_string)
    }

    /// The venue's record as a string, for the readers that parse it themselves.
    pub fn venue_json(&self) -> Option<String> {
        self.venue.as_ref().filter(|v| !v.is_null()).map(Value::to_string)
    }
}

/// Ask the venue's object for the basket's nodes. `promo` is the NORMALISED
/// code (`dowiz_hub::promo::normalise`), or none. A refusal by the object is
/// an error here: this is a read of the venue's own catalogue, and the venue
/// was checked by the caller.
pub async fn ask(place: &crate::hubstore::Place, ids: &[String], promo: Option<&str>) -> Result<Basket> {
    let enc = crate::mcp::enc;
    let mut q: Vec<String> = ids.iter().map(|id| format!("ids={}", enc(id))).collect();
    if let Some(code) = promo {
        q.push(format!("promo={}", enc(code)));
    }
    let url = format!("https://hub/fold/basket?{}", q.join("&"));
    let mut res = place.stub()?.fetch_with_request(Request::new(&url, Method::Get)?).await?;
    if res.status_code() != 200 {
        return Err(Error::RustError(format!("hub object refused the basket: {} {}", res.status_code(), res.text().await.unwrap_or_default())));
    }
    res.json().await
}
