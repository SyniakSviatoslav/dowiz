//! THE OWNER'S AND THE KITCHEN'S FOLDS, ANSWERED HERE (BN1, R3): the
//! analytics, the kitchen's numbers and the Stock screen each pulled the
//! catalogue image (and the stock log) across the hop to derive a few KB of
//! JSON. Every image they read is in this object's memory; each is derived
//! here with the same pure function its handler is built on, and the answer
//! alone crosses. The Worker keeps the owner / staff checks.
//!
//!   GET /fold/analytics?venue=&now=&days=7|30
//!   GET /fold/kitchen?venue=&now=[&from=&to=&days=]
//!   GET /fold/stock?now=

use super::HubImages;
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::{Call as Request, Reply as Response};

/// `?k=` of a request, decoded once.
fn query(req: &Request, k: &str) -> Result<Option<String>> {
    Ok(req.url()?.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.to_string()))
}

/// The venue named and the clock sent, or the 400 that says which is missing.
fn venue_and_now(req: &Request) -> Result<std::result::Result<(String, i64), Response>> {
    let venue = query(req, "venue")?;
    let now = query(req, "now")?.and_then(|n| n.parse::<i64>().ok());
    Ok(match (venue, now) {
        (Some(v), Some(n)) => Ok((v, n)),
        _ => Err(Response::error("a fold needs a venue and a clock", 400)?),
    })
}

impl HubImages {
    /// `GET /fold/<what>` for the reads of BN1: the ones below, the basket
    /// (`hubdo/basket.rs`) and the exception report (`hubdo/exceptions.rs`).
    pub(super) async fn fold_read(&self, what: &str, req: &Request) -> Result<Response> {
        match what {
            "basket" => self.fold_basket(req).await,
            "analytics" => self.fold_analytics(req).await,
            "kitchen" => self.fold_kitchen(req).await,
            "stock" => self.fold_stock(req).await,
            "exceptions" => self.fold_exceptions(req).await,
            _ => Response::error("no such fold", 404),
        }
    }

    /// See the module: `services::analytics::handler::answer`.
    pub(super) async fn fold_analytics(&self, req: &Request) -> Result<Response> {
        let (venue, now) = match venue_and_now(req)? {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
        let days = crate::services::analytics::fold::window(query(req, "days")?.as_deref());
        let (_, listed) = self.orders_view().await?;
        let cat = self.catalogue().await?;
        Response::from_json(&crate::services::analytics::handler::answer(listed, &cat, &venue, now, days))
    }

    /// See the module: `services::analytics::kitchen::answer`.
    pub(super) async fn fold_kitchen(&self, req: &Request) -> Result<Response> {
        let (venue, now) = match venue_and_now(req)? {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
        let (from, to, days) = (query(req, "from")?, query(req, "to")?, query(req, "days")?);
        let (_, listed) = self.orders_view().await?;
        let cat = self.catalogue().await?;
        let (_, stock) = self.stock_log().await?;
        match crate::services::analytics::kitchen::answer(listed, &cat, &stock, &venue, now, (from.as_deref(), to.as_deref(), days.as_deref())) {
            Ok(v) => Response::from_json(&v),
            Err((status, why)) => Response::error(why, status),
        }
    }

    /// See the module: `services::operations::stock::shelf`.
    pub(super) async fn fold_stock(&self, req: &Request) -> Result<Response> {
        let Some(now) = query(req, "now")?.and_then(|n| n.parse::<i64>().ok()) else {
            return Response::error("the shelf needs a clock", 400);
        };
        let cat = self.catalogue().await?;
        let (_, stock) = self.stock_log().await?;
        match crate::services::operations::stock::shelf(&cat, &stock, now) {
            Ok(v) => Response::from_json(&v),
            Err(why) => Response::error(why, 500),
        }
    }
}
