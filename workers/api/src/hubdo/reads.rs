//! THE OWNER'S AND THE KITCHEN'S FOLDS, ANSWERED HERE (BN1, R3): the
//! analytics, the kitchen's numbers and the Stock screen each pulled the
//! catalogue image (and the stock log) across the hop to derive a few KB of
//! JSON. Every image they read is in this object's memory; each is derived
//! here with the same pure function its handler is built on, and the answer
//! alone crosses. The Worker keeps the owner / staff checks.
//!
//!   GET /fold/analytics?venue=&now=[&days=7|30|90|365 | &from=&to=]
//!       [&op=trace[&trace=] | &op=catch_up[&rebuild=1]]   (W-HIST, `cube.rs`)
//!   GET /fold/kitchen?venue=&now=[&from=&to=&days=]
//!   GET /fold/stock?now=
//!   GET /fold/week_top?venue=&now=   the storefront's "most ordered this week" (W-MR0)
//!   GET /fold/prep?venue=&now=[&day=]   (W-PREP, `forecast.rs`)
//!   GET /fold/haccp?kind=&from=&to=&names=  the HACCP CSV (P13, W-STORE, `stock/haccp.rs`)

use super::HubImages;
use worker::*;

/// The cube's image in this object: catch-up, trace, rows (W-HIST P2b).
#[path = "cube.rs"]
mod cube;
/// The kitchen's forecast and prep list, and P7's surplus (W-PREP).
#[path = "forecast.rs"]
mod forecast;
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
            "prep" => self.fold_prep(req).await,
            "exceptions" => self.fold_exceptions(req).await,
            "week_top" => self.fold_week_top(req).await,
            "haccp" => self.fold_haccp(req).await,
            _ => Response::error("no such fold", 404),
        }
    }

    /// See the module: `services::analytics::handler::answer_with`, over the
    /// hot log and the cube (W-HIST). `op=catch_up` folds the archives the
    /// cube lacks; `op=trace` answers the records behind one day
    /// (`hubdo/cube.rs`). Only the Worker's two analytics handlers send `op`.
    pub(super) async fn fold_analytics(&self, req: &Request) -> Result<Response> {
        let (venue, now) = match venue_and_now(req)? {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
        let cat = self.catalogue().await?;
        let zone = crate::services::analytics::handler::zone_of(&cat);
        match query(req, "op")?.as_deref() {
            Some("catch_up") => return self.cube_catch_up(&venue, zone, query(req, "rebuild")?.is_some()).await,
            Some("trace") => return self.cube_trace(&venue, zone, now, query(req, "trace")?.as_deref()).await,
            _ => {}
        }
        let q = (query(req, "days")?, query(req, "from")?, query(req, "to")?);
        let (_, listed) = self.orders_view().await?;
        let (_, bytes) = self.cube_image().await?;
        let parsed = cube::parsed(&bytes);
        let (folded, pending) = self.cube_progress(&parsed).await?;
        let cold = cube::rows_of(&parsed);
        match crate::services::analytics::handler::answer_with(listed, &cat, &venue, now, (q.0.as_deref(), q.1.as_deref(), q.2.as_deref()), &cold) {
            Ok(mut v) => {
                v["history"]["folded"] = serde_json::json!(folded);
                v["history"]["pending"] = serde_json::json!(pending);
                // v1 unless the caller asked for v2: the old shape stays byte for byte.
                let v2 = query(req, "v")?.as_deref() == Some("2");
                Response::from_json(&if v2 { v } else { crate::services::analytics::handler::v1_of(v) })
            }
            Err((status, why)) => Response::error(why, status),
        }
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
        // THE DAYS THE HOT LOG NO LONGER HOLDS come from the cube (W-HIST P2a).
        let (_, bytes) = self.cube_image().await?;
        let parsed = cube::parsed(&bytes);
        let cold = cube::rows_of(&parsed);
        match crate::services::analytics::kitchen::answer_with(listed, &cat, &stock, &venue, now, (from.as_deref(), to.as_deref(), days.as_deref()), &cold) {
            Ok(mut v) => {
                // `menu` and `history` are v2 (analytics.kitchen.v2); v1 keeps its bytes.
                if query(req, "v")?.as_deref() != Some("2") {
                    if let Some(m) = v.as_object_mut() {
                        m.remove("menu");
                        m.remove("history");
                    }
                }
                Response::from_json(&v)
            }
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

    /// See `services::analytics::week_top::public`: the badged dishes of the
    /// venue's last seven days, over the hot log (it keeps thirty).
    pub(super) async fn fold_week_top(&self, req: &Request) -> Result<Response> {
        let (venue, now) = match venue_and_now(req)? {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };
        // The zone alone, read in place (W-LOOPB): `zone_of` is `hubstore::zone_of` of the record.
        let zone = self.with_catalog(|c| crate::hubstore::zone_of(c.location().and_then(|j| serde_json::from_str(&j).ok()).as_ref())).await?;
        let (_, listed) = self.orders_view().await?;
        let orders = crate::services::orders::mine::of_venue(listed, &venue);
        Response::from_json(&crate::services::analytics::week_top::public(&orders, zone, now))
    }

    /// See `services::operations::stock::haccp::fold`: one replay of the
    /// stock log, the CSV alone crosses.
    pub(super) async fn fold_haccp(&self, req: &Request) -> Result<Response> {
        let (kind, from, to) = (query(req, "kind")?.unwrap_or_default(), query(req, "from")?, query(req, "to")?);
        let names = query(req, "names")?; // W-STORE2: the signers' names, from the Worker

        let cat = self.catalogue().await?;
        let (_, stock) = self.stock_log().await?;
        match crate::services::operations::stock::haccp::fold(&cat, &stock, &kind, from.as_deref(), to.as_deref(), names.as_deref()) {
            Ok(csv) => Response::ok(csv),
            Err((status, why)) => Response::error(why, status),
        }
    }
}
