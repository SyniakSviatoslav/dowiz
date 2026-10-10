//! THE READS THE TABLE ANSWERS FROM THE LOG AND THE BYTES (split out of
//! `hubdo.rs`, W-SPLIT): the generation, what changed since, the queue, one
//! order, where the couriers are, the read-only rebuild, and an image's bytes
//! by id. The routes are named in `hubdo/route.rs`.

use super::{changes_since, HubImages, LOG_IMAGE};
use crate::wire::{Call, Reply as Response};
use worker::*;

impl HubImages {
    /// `/fold/positions`: where the couriers are, as they last said over their
    /// sockets. Empty after a hibernation, which is honest: a position whose
    /// meaning expires in minutes should not survive a sleep.
    pub(super) fn answer_positions(&self) -> Result<Response> {
        Response::from_json(&self.fresh_positions())
    }

    /// `/fold/generation`: THE GENERATION ALONE. A writer that needs nothing but
    /// the guard used to ask for the orders and throw them away, which on a
    /// venue with a thousand of them is a list built for a number.
    pub(super) async fn answer_generation(&self) -> Result<Response> {
        let generation =
            self.image(LOG_IMAGE).await?.map(|(m, _)| m.generation).unwrap_or(0);
        let mut res = Response::from_json(
            &serde_json::json!({ "generation": generation }),
        )?;
        res.headers_mut().set("x-generation", &generation.to_string())?;
        Ok(res)
    }

    /// `/fold/changes`: WHAT CHANGED SINCE, for a client that already has a copy.
    /// `full: true` means the window does not reach that far and the list is
    /// the answer -- which is also what a cold object says, and is not an error.
    pub(super) async fn answer_changes(&self, req: &Call) -> Result<Response> {
        let url = req.url()?;
        let since: i64 = url
            .query_pairs()
            .find(|(k, _)| k == "since")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(-1);
        let generation =
            self.image(LOG_IMAGE).await?.map(|(m, _)| m.generation).unwrap_or(0);
        let body = match self.counters.since(changes_since(&self.recent.borrow(), since)) { // AX0 (a)
            Some(changes) => serde_json::json!({
                "generation": generation, "full": false, "changes": changes
            }),
            None => serde_json::json!({
                "generation": generation, "full": true, "changes": []
            }),
        };
        let mut res = Response::from_json(&body)?;
        res.headers_mut().set("x-generation", &generation.to_string())?;
        Ok(res)
    }

    /// `/fold/orders`: the queue, from the memo (`orders_view`).
    pub(super) async fn answer_orders(&self) -> Result<Response> {
        let (generation, view) = self.orders_view().await?;
        let mut res = Response::from_json(&view)?;
        res.headers_mut().set("x-generation", &generation.to_string())?;
        Ok(res)
    }

    /// `/fold/order?id=`: one order.
    pub(super) async fn answer_order(&self, req: &Call) -> Result<Response> {
        // THE ID TRAVELS AS A QUERY PARAMETER, not as a path
        // segment: an order id is not this object's to constrain,
        // and `query_pairs` decodes it exactly once, where a hand
        // written path split would decode it never.
        let url = req.url()?;
        let Some(id) = url
            .query_pairs()
            .find(|(k, _)| k == "id")
            .map(|(_, v)| v.to_string())
        else {
            return Response::error("no order named", 400);
        };
        let (generation, view) = self.orders_view().await?;
        let found = view.into_iter().find(|o| o.order_id == id);
        let mut res = match found {
            Some(o) => Response::from_json(&o)?,
            None => Response::empty()?.with_status(404),
        };
        res.headers_mut().set("x-generation", &generation.to_string())?;
        Ok(res)
    }

    /// `/img/<id>`: an image's BYTES, for a reader that is not this object.
    pub(super) async fn image_route(&self, path: &str, req: Call) -> Result<Response> {
        let id = path.rsplit('/').next().unwrap_or("").to_string();
        if id.is_empty() {
            return Response::error("no image named", 400);
        }
        match req.method() {
            Method::Get => match self.image(&id).await? {
                Some((meta, bytes)) => {
                    let mut res = Response::from_bytes(bytes)?;
                    res.headers_mut().set("x-generation", &meta.generation.to_string())?;
                    Ok(res)
                }
                // Absent is not an error: the caller creates a fresh image, which
                // is how a venue's very first order is placed.
                None => {
                    let mut res = Response::empty()?.with_status(204);
                    res.headers_mut().set("x-generation", "0")?;
                    Ok(res)
                }
            },
            Method::Put => {
                let expected: i64 = req.headers().get("x-generation").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(-1);
                if expected < 0 {
                    return Response::error("x-generation is required on a write", 400);
                }
                let mut req = req;
                let bytes = req.bytes().await?;
                match self.put_image_stamped(&id, expected, &bytes, crate::catalog_history::stamp_of(req.headers())).await? {
                    Some(next) => {
                        let mut res = Response::empty()?;
                        res.headers_mut().set("x-generation", &next.to_string())?;
                        Ok(res)
                    }
                    // The caller re-reads and replays. Same contract the D1
                    // generation guard had, and `hubstore`'s retry already
                    // speaks it.
                    None => Response::error("generation moved", 409),
                }
            }
            _ => Response::error("method not allowed", 405),
        }
    }

    /// REFOLD FROM THE BYTES AND DIFF AGAINST WHAT IS BEING SERVED.
    ///
    /// IT DELIBERATELY IGNORES THE MEMO IT IS CHECKING. `orders_view` answers
    /// from `self.folded` when the generation has not moved, and that memo is
    /// what every reader in the platform gets — so nothing else in the system
    /// is in a position to disagree with it. This folds the image again, from
    /// the stored bytes, and compares.
    ///
    /// AND IT CROSSES THE TWO IMAGES, which is the check nothing did. The log
    /// can say an order is over while the stock ledger is still holding its
    /// ingredients; `StockLedger::stranded()` has been able to report that
    /// since it was written and nothing in production ever called it.
    ///
    /// READ-ONLY. It writes nothing, takes no generation and cannot repair
    /// anything — a gate that fixes what it finds is a gate whose findings
    /// nobody ever sees.
    pub(super) async fn rebuild(&self) -> Result<crate::rebuild::Report> {
        let Some((meta, bytes)) = self.image(LOG_IMAGE).await? else {
            return self.rebuild_rows(crate::rebuild::Report::default()).await; // no log: the other memo rows still answer
        };
        // FRESH, from the bytes. `Hub::load` parses the arena and
        // `orders_state` replays the chain (`rebuild::of_log`); neither
        // consults the memo.
        let hub = dowiz_hub::Hub::load(&bytes)
            .map_err(|e| crate::image_err::unreadable("hub", e))?;

        let (held, modelled) = match self.image(crate::hubstore::IMAGE_STOCK).await? {
            Some((_, b)) => {
                let log = dowiz_hub::stock::StockLog::load(&b)
                    .map_err(|e| crate::image_err::unreadable("stock", e))?;
                let modelled = log.len() > 0;
                let led = log
                    .ledger()
                    .map_err(|e| Error::RustError(format!("stock ledger: {e}")))?;
                (led.stranded(), modelled)
            }
            None => (Vec::new(), false),
        };
        // The memo is borrowed only here, before the archives' awaits.
        let mut report = crate::rebuild::of_log(&hub, self.folded.borrow().as_ref(), meta.generation, &held, modelled);
        // R5: a stranded order the hot log does not know is looked for in
        // the archives (`hubdo/archives.rs`, `rebuild::cross_archives`).
        if crate::rebuild::needs_archives(&report, &hub) {
            crate::rebuild::cross_archives(&mut report, &self.archive_folds().await?);
        }
        self.rebuild_rows(report).await // law 8 over EVERY memo row of the edge table (`hubdo/edges.rs`)
    }
}
