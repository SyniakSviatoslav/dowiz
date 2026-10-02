//! THE FOLDS OVER THE ORDER LOG AND THE CATALOGUE TOGETHER, ANSWERED HERE
//! (BN1, R3): the reveal log, the two assistants' facts, the owner's graph and
//! the waste report each pulled the log image -- and most of them the
//! catalogue and the stock ledger beside it -- across the hop to derive a few
//! KB of JSON. Every image they read is in this object's memory; each is
//! derived here with the same PURE function its handler is built on, and the
//! answer alone crosses. The Worker keeps the owner / staff checks and, for
//! the assistants, the call to the venue's model.
//!
//!   GET /fold/reveals                       `customers::handlers::reveals_view`
//!   GET /fold/assist?venue=&now=&q=         `assist::owner_facts`
//!   GET /fold/graph?q=&limit=               `assist::graph_facts`
//!   GET /fold/kitchen_facts?venue=&now=     `assist::kitchen::kitchen_facts`
//!   GET /fold/waste?venue=                  `operations::waste::report`

use super::HubImages;
use serde_json::json;
use std::collections::HashMap;
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
    /// What the shelf holds, for the graph's ingredient nodes. An unreadable
    /// ledger labels nothing, as `assist::shelf_labels` did across the hop.
    async fn shelf_labels(&self) -> HashMap<String, String> {
        match self.stock_log().await {
            Ok((_, stock)) => crate::services::engagement::assist::shelf_labels_of(&stock),
            Err(_) => HashMap::new(),
        }
    }

    /// See the module.
    pub(super) async fn fold_facts(&self, what: &str, req: &Request) -> Result<Response> {
        match what {
            "reveals" => {
                let (_, hub) = self.log_hub().await?;
                Response::from_json(&json!({ "reveals": crate::services::customers::handlers::reveals_view(&hub) }))
            }
            "assist" => {
                let (venue, now) = match venue_and_now(req)? {
                    Ok(v) => v,
                    Err(r) => return Ok(r),
                };
                let question = query(req, "q")?.unwrap_or_default();
                let (_, hub) = self.log_hub().await?;
                let cat = self.catalogue().await?;
                let labels = self.shelf_labels().await;
                Response::from_json(&crate::services::engagement::assist::owner_facts(&hub, &cat, &labels, &venue, &question, now))
            }
            "graph" => {
                let question = query(req, "q")?.unwrap_or_default();
                let limit = query(req, "limit")?.and_then(|v| v.parse::<usize>().ok()).unwrap_or(12).clamp(1, 50);
                let (_, hub) = self.log_hub().await?;
                let cat = self.catalogue().await?;
                let labels = self.shelf_labels().await;
                Response::from_json(&crate::services::engagement::assist::graph_facts(&hub, &cat, &labels, &question, limit))
            }
            "kitchen_facts" => {
                let (venue, now) = match venue_and_now(req)? {
                    Ok(v) => v,
                    Err(r) => return Ok(r),
                };
                let (_, listed) = self.orders_view().await?;
                let orders: Vec<(String, String)> = listed.into_iter().map(|o| (o.order_id, o.order_json)).collect();
                let cat = self.catalogue().await?;
                let (_, stock) = self.stock_log().await?;
                let shelf: Vec<(String, i64, i64)> = stock
                    .ledger()
                    .map(|l| l.items().into_iter().map(|(i, lv)| (i, lv.on_hand, lv.reserved)).collect())
                    .unwrap_or_default();
                Response::from_json(&crate::services::engagement::assist::kitchen::kitchen_facts(&orders, &cat.products(), &cat.supplies(), &shelf, &venue, now))
            }
            "waste" => {
                let Some(venue) = query(req, "venue")? else {
                    return Response::error("the waste report needs a venue", 400);
                };
                let (_, stock) = self.stock_log().await?;
                let journal = match stock.journal() {
                    Ok(j) => j,
                    Err(e) => return Response::error(e.to_string(), 500),
                };
                let (_, hub) = self.log_hub().await?;
                Response::from_json(&crate::services::operations::waste::report(&journal.entries, &hub.events_oldest_first(), &venue))
            }
            _ => Response::error("no such fold", 404),
        }
    }
}
