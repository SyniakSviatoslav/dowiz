//! THE DAILY SALES CUBE IN THE VENUE'S OBJECT (W-HIST P2b): its image, the
//! catch-up that folds the archives into it, and the trace that refolds one
//! day and compares (`services::analytics::cube` is the pure half and says
//! why a store exists at all, against `fold.rs`'s "NO ANALYTICS STORE").
//!
//!   GET /fold/analytics?venue=&now=&op=catch_up[&rebuild=1]
//!   GET /fold/analytics?venue=&now=&op=trace[&trace=yyyy-mm-dd]
//!
//! Both ride the analytics read the dispatcher already routes, so this file
//! needs no line in `hubdo.rs`. Only the Worker's analytics handlers send
//! `op`; the catch-up is reached by `POST /api/owner/analytics/history` and
//! by the rotation, never by a GET from a console.
//!
//! AN ARCHIVE IS READ ONCE AND NOT KEPT RESIDENT (the rule `archives.rs` and
//! `forget` follow). A catch-up reads each archive it folds and the image
//! after it; a healthy night with nothing rotated reads none.

use crate::hubdo::HubImages;
use crate::services::analytics::cube::{self as sc, Cube, DayCube};
use dowiz_hub::tz::Zone;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use worker::*;
use crate::wire::Reply as Response;

thread_local! {
    /// THE CUBE, PARSED, beside the exact bytes it was parsed from. The image
    /// moves once a night (a catch-up) and is read on every analytics
    /// request; parsing a year of rows on each read was the cost
    /// (`history/tests.rs` `cpu_of_a_year_read`). The hit is decided by
    /// COMPARING THE BYTES (a memcmp, far cheaper than a parse), not by a
    /// venue or a generation: two objects in one isolate, or a restore that
    /// resets generations, can never be answered from another image's parse.
    static PARSED: std::cell::RefCell<Option<(Vec<u8>, std::rc::Rc<std::result::Result<Cube, String>>)>> = const { std::cell::RefCell::new(None) };
}

/// The cube these bytes hold, parsed once. No image is an empty cube; an
/// unreadable one is its reason.
pub fn parsed(bytes: &[u8]) -> std::rc::Rc<std::result::Result<Cube, String>> {
    if let Some(hit) = PARSED.with(|p| p.borrow().as_ref().filter(|(b, _)| b.as_slice() == bytes).map(|(_, c)| c.clone())) {
        return hit;
    }
    let fresh = std::rc::Rc::new(if bytes.is_empty() { Ok(Cube::default()) } else { Cube::decode(bytes) });
    PARSED.with(|p| *p.borrow_mut() = Some((bytes.to_vec(), fresh.clone())));
    fresh
}

/// The cube's rows from `yyyymmdd` to `yyyymmdd`, out of a parse; an
/// unreadable image is its reason, never an empty history.
pub fn rows_of(cube: &std::result::Result<Cube, String>) -> impl Fn(i64, i64) -> std::result::Result<BTreeMap<i64, DayCube>, String> + '_ {
    move |from, to| match cube {
        Ok(c) => Ok(c.rows.range(from..=to).map(|(d, r)| (*d, r.clone())).collect()),
        Err(why) => Err(why.clone()),
    }
}

/// One order as the trace shows it: when, what, how much, from where. NO
/// PERSON: the contact is not carried.
fn record(id: &str, o: &Value, zone: Zone, from: &str) -> Value {
    use crate::services::orders::status;
    let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
    let st = o.get("status").and_then(Value::as_str).unwrap_or("");
    let items: Vec<Value> = o
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|it| json!({ "id": it.get("product_id"), "quantity": it.get("quantity"), "unitPrice": it.get("unit_price") }))
        .collect();
    json!({
        "id": id, "at": at, "hour": sc::hour_of(zone, at), "status": st, "from": from,
        "took": status::venue_took(o.get("total").and_then(Value::as_i64).unwrap_or(0), o.get("tip").and_then(Value::as_i64).unwrap_or(0), st),
        "channel": crate::services::ordering::channel::of(o).unwrap_or("unknown"),
        "kind": crate::services::ordering::fulfilment::of(o), "items": items,
    })
}

impl HubImages {
    /// The cube image and its generation; none yet is empty bytes at 0.
    pub(super) async fn cube_image(&self) -> Result<(i64, Vec<u8>)> {
        Ok(self.image(sc::IMAGE).await?.map(|(m, b)| (m.generation, b)).unwrap_or((0, Vec::new())))
    }

    /// The archives the settings list, in rotation order.
    async fn listed_archives(&self) -> Result<Vec<String>> {
        let Some((_, b)) = self.image(crate::hubstore::IMAGE_SETTINGS).await? else { return Ok(Vec::new()) };
        let s = dowiz_hub::settings::Settings::load(&b).map_err(|_| Error::RustError("settings image is unreadable".into()))?;
        Ok(crate::hubstore::archives_of(&s).into_iter().filter(|id| crate::hubstore::is_archive_id(id)).collect())
    }

    /// One archive's `(order id, folded order)` pairs, read without keeping the
    /// archive resident. A listed archive that is absent or unreadable is an
    /// error, never an empty fold.
    async fn archive_pairs(&self, id: &str) -> Result<Vec<(String, String)>> {
        let resident = self.mem.borrow().contains_key(id);
        let image = self.image(id).await?;
        if !resident {
            self.mem.borrow_mut().remove(id);
        }
        let Some((_, bytes)) = image else { return Err(Error::RustError(format!("archive {id} is listed and absent"))) };
        let hub = dowiz_hub::Hub::load(&bytes).map_err(|_| Error::RustError(format!("archive {id} is unreadable")))?;
        Ok(crate::hubstore::orders_state(&hub).into_iter().map(|e| (e.order_id, e.order_json)).collect())
    }

    /// The order ids of the image after archive `k` of `listed`: the next
    /// archive, or the hot log when `k` is the newest.
    async fn ids_after(&self, listed: &[String], k: usize) -> Result<HashSet<String>> {
        Ok(match listed.get(k + 1) {
            Some(next) => self.archive_pairs(next).await?.into_iter().map(|(id, _)| id).collect(),
            None => self.orders_view().await?.1.into_iter().map(|o| o.order_id).collect(),
        })
    }

    /// The orders of `venue` that left the hot log at archive `k`'s rotation.
    async fn moved_at(&self, venue: &str, listed: &[String], k: usize) -> Result<Vec<(String, Value)>> {
        let pairs = self.archive_pairs(&listed[k]).await?;
        let next = self.ids_after(listed, k).await?;
        Ok(sc::moved(pairs, &next).into_iter().filter(|(_, o)| crate::services::orders::mine::belongs_to(o, venue)).collect())
    }

    /// How far the cube is: archives folded, and those listed and not yet.
    pub(super) async fn cube_progress(&self, cube: &std::result::Result<Cube, String>) -> Result<(usize, Vec<String>)> {
        let listed = self.listed_archives().await?;
        let folded = cube.as_ref().map(|c| c.folded.clone()).unwrap_or_default();
        Ok((folded.len(), listed.into_iter().filter(|a| !folded.contains(a)).collect()))
    }

    /// THE CATCH-UP: every listed archive the cube has not folded, oldest
    /// first, each day's moved orders added to its row; one write at the end
    /// under the generation guard. `rebuild` starts from an empty cube.
    pub(super) async fn cube_catch_up(&self, venue: &str, zone: Zone, rebuild: bool) -> Result<Response> {
        let listed = self.listed_archives().await?;
        let (gen, bytes) = self.cube_image().await?;
        let mut cube = match (rebuild, bytes.is_empty()) {
            (true, _) | (_, true) => Cube::default(),
            _ => match Cube::decode(&bytes) {
                Ok(c) => c,
                Err(why) => return Response::error(format!("the cube is unreadable ({why}); rebuild it"), 409),
            },
        };
        let mut added = Vec::new();
        for (k, archive) in listed.iter().enumerate() {
            if cube.folded.contains(archive) {
                continue;
            }
            let moved: Vec<Value> = self.moved_at(venue, &listed, k).await?.into_iter().map(|(_, o)| o).collect();
            let days = sc::fold_orders(&moved, zone, i64::MAX);
            added.push(json!({ "archive": archive, "orders": moved.len(), "days": days.len() }));
            cube.absorb(archive, days);
        }
        if !added.is_empty() || rebuild {
            if self.put_image(sc::IMAGE, gen, &cube.encode()).await?.is_none() {
                return Response::error("the cube moved during the catch-up", 409);
            }
        }
        Response::from_json(&json!({ "folded": cube.folded.len(), "added": added, "days": cube.rows.len(), "listed": listed.len() }))
    }

    /// THE TRACE: the records behind one day and the cube's verification of
    /// it. The archived part is REFOLDED from the archives the stored row
    /// names and compared byte for byte (`cube::line`); the hot part is the
    /// hot log's orders of that day. No day: the newest archived one.
    pub(super) async fn cube_trace(&self, venue: &str, zone: Zone, now: i64, day: Option<&str>) -> Result<Response> {
        let (_, bytes) = self.cube_image().await?;
        let cube = if bytes.is_empty() { Cube::default() } else {
            match Cube::decode(&bytes) {
                Ok(c) => c,
                Err(why) => return Response::error(format!("the cube is unreadable: {why}"), 409),
            }
        };
        let d = match day.map(str::trim).filter(|s| !s.is_empty() && *s != "1") {
            Some(s) => match dowiz_hub::stock::meta::parse_day(s) {
                Some(d) => d,
                None => return Response::error(format!("{s:?} is not a date (yyyy-mm-dd)"), 400),
            },
            None => match cube.rows.keys().next_back() {
                Some(d) => *d,
                None => return Response::error("nothing is archived yet: there is no day to verify", 404),
            },
        };
        let stored = cube.rows.get(&d).cloned();
        let listed = self.listed_archives().await?;
        let mut records = Vec::new();
        let mut cold = Vec::new();
        for src in stored.iter().flat_map(|r| r.src.iter()) {
            let Some(k) = listed.iter().position(|a| a == src) else {
                return Response::error(format!("the row names {src}, which is not listed"), 409);
            };
            for (id, o) in self.moved_at(venue, &listed, k).await? {
                if sc::day_of(zone, o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0)) == d {
                    records.push(record(&id, &o, zone, src));
                    cold.push(o);
                }
            }
        }
        let refolded = sc::fold_orders(&cold, zone, i64::MAX).remove(&d).map(|mut r| {
            r.src = stored.as_ref().map(|s| s.src.clone()).unwrap_or_default();
            r
        });
        let hot: Vec<(String, Value)> = self
            .orders_view()
            .await?
            .1
            .into_iter()
            .filter_map(|v| serde_json::from_str::<Value>(&v.order_json).ok().map(|o| (v.order_id, o)))
            .filter(|(_, o)| crate::services::orders::mine::belongs_to(o, venue))
            .filter(|(_, o)| sc::day_of(zone, o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0)) == d)
            .collect();
        records.extend(hot.iter().map(|(id, o)| record(id, o, zone, "hot")));
        let hot_row = sc::fold_orders(&hot.into_iter().map(|(_, o)| o).collect::<Vec<_>>(), zone, now).remove(&d);
        records.sort_by(|a, b| a["at"].as_i64().cmp(&b["at"].as_i64()).then(a["id"].as_str().cmp(&b["id"].as_str())));
        let line = |r: &Option<DayCube>| r.as_ref().map(sc::line);
        Response::from_json(&json!({
            "day": dowiz_hub::stock::meta::show_day(d),
            "archived": { "stored": line(&stored), "refolded": line(&refolded), "equal": line(&stored) == line(&refolded), "src": stored.map(|s| s.src) },
            "hot": line(&hot_row),
            "records": records,
        }))
    }
}

#[cfg(test)]
#[path = "cube/tests.rs"]
mod tests;
