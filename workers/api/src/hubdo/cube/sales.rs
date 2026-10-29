//! THE SALES IMAGE'S READS IN THE VENUE'S OBJECT (R-BEBOP-LEVERS §5.4, lane LV-COL-E): the trace's
//! byte-for-byte check of a day's block, and the span's food cost for the analytics answer. The
//! image is written by `cube_catch_up` (`cube.rs`), beside the cube, from the same archives.

use crate::hubdo::HubImages;
use crate::services::analytics::cube::sales as ss;
use dowiz_hub::block::kernels::{image::Index, pack::Pack, sales_block};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use dowiz_hub::tz::Zone;
use serde_json::{json, Value};
use worker::*;

impl HubImages {
    /// THE DAY'S SALES BLOCK, VERIFIED (LV-COL-E): the stored block of day `d` out of the sales image
    /// against the block refolded from the same archived orders, compared BYTE FOR BYTE, and the
    /// day's food cost read off the stored block by the kernels. No stored block is `stored: null`.
    pub(super) async fn trace_block(&self, d: i64, cold: &[(String, Value)], zone: Zone) -> Result<Value> {
        let (_, bytes) = self.sales_image().await?;
        let pack = match Pack::decode(&bytes) {
            Ok(p) => p,
            Err(r) => return Ok(json!({ "error": format!("the sales image is unreadable: {r:?}") })),
        };
        let enc = |b| dowiz_hub::block::encode::encode(&b).map_err(|r| format!("{r:?}"));
        let stored = pack.days.get(&(d as i32)).map(|r| enc(sales_block(r)));
        let lines: Vec<_> = ss::sale_rows(cold, zone, i64::MAX).into_iter().map(|(s, _)| s).filter(|s| i64::from(s.day) == d).collect();
        // The refold is canonical as the stored day is: one row per dish, ascending (`aggregate`).
        let rows = dowiz_hub::block::kernels::aggregate(&lines).map_err(|r| Error::RustError(format!("{r:?}")))?;
        let refolded = enc(sales_block(&rows));
        let crc = |b: &std::result::Result<Vec<u8>, String>| b.as_ref().map(|b| format!("{:08x}/{}", dowiz_hub::block::crc32(b), b.len())).unwrap_or_else(|e| e.clone());
        let cost = pack.days.get(&(d as i32)).map(|r| {
            crate::services::analytics::history::food_cost::food_cost(&[], &[sales_block(r)], &pack.dishes, &|id: &str| json!(id)).unwrap_or_else(|e| json!({ "error": e }))
        });
        Ok(json!({
            "stored": stored.as_ref().map(crc), "refolded": crc(&refolded),
            "equal": stored.as_ref().is_some_and(|s| s.is_ok() && *s == refolded),
            "rows": rows.len(), "lines": lines.len(), "foodCost": cost,
        }))
    }

    /// THE SPAN'S FOOD COST (LV-COL-E): the archived days' blocks out of the sales image plus the hot
    /// log's rows of the same days, folded `full` by the kernels (`history::food_cost`). Disjoint as
    /// the cube is: an archived row left the hot log. For the analytics answer (`reads.rs`).
    pub(in crate::hubdo) async fn food_cost_of(&self, venue: &str, zone: Zone, now: i64, s: crate::services::analytics::history::Span) -> Result<Value> {
        let (generation, bytes) = self.sales_image().await?;
        let (from, to) = s.days();
        let opened = match opened_image(venue, generation, &bytes) {
            Ok(o) => o,
            Err(r) => return Ok(json!({ "error": format!("the sales image is unreadable: {r:?}") })),
        };
        let (idx, dishes) = &*opened;
        let days = match idx.view(&bytes) {
            Ok(img) => img.days(from as i32, to as i32),
            Err(r) => return Ok(json!({ "error": format!("{r:?}") })),
        };
        let hot: Vec<(String, Value)> = self
            .orders_view()
            .await?
            .1
            .into_iter()
            .filter_map(|v| serde_json::from_str::<Value>(&v.order_json).ok().map(|o| (v.order_id, o)))
            .filter(|(_, o)| crate::services::orders::mine::belongs_to(o, venue))
            .collect();
        let hot_rows = ss::sale_rows(&hot, zone, now);
        let mut names = dishes.clone();
        names.extend(hot_rows.iter().map(|(r, id)| (r.dish, id.clone())));
        let in_span: Vec<_> = hot_rows.into_iter().map(|(r, _)| r).filter(|r| (from..=to).contains(&i64::from(r.day))).collect();
        let hot = [sales_block(&in_span)];
        let mut v = crate::services::analytics::history::food_cost::food_cost(&days, &hot, &names, &|id: &str| json!(id)).unwrap_or_else(|e| json!({ "error": e }));
        v["blocks"] = json!(days.len());
        v["hotLines"] = json!(in_span.len());
        Ok(v)
    }
}

/// What `Index::open` + `pack::dishes_of` learned of one image: the day blocks' ranges and the dish ids.
type Opened = Rc<(Index, BTreeMap<i64, String>)>;

thread_local! {
    /// W-BLOCKVIEW: ONE opened sales image per object, keyed by `(venue, generation)`. A miss is the
    /// full check (`Index::open`: every block's crc and layout, the safe path); a hit pays no crc, and
    /// `Index::view` still refuses bytes whose length or any block's crc trailer differ from the ones
    /// opened, so a stale entry is an error, never another image's answer.
    static OPENED: RefCell<Option<(String, i64, Opened)>> = const { RefCell::new(None) };
}

/// The opened image of `venue` at `generation`: cached (and these bytes still `view` through it), or
/// opened now and cached.
pub(in crate::hubdo) fn opened_image(venue: &str, generation: i64, bytes: &[u8]) -> std::result::Result<Opened, dowiz_hub::block::Refusal> {
    if let Some(o) = OPENED.with_borrow(|c| c.as_ref().filter(|(v, g, o)| v == venue && *g == generation && o.0.view(bytes).is_ok()).map(|(_, _, o)| o.clone())) {
        return Ok(o);
    }
    let o: Opened = Rc::new((Index::open(bytes)?, dowiz_hub::block::kernels::pack::dishes_of(bytes)?));
    OPENED.with_borrow_mut(|c| *c = Some((venue.to_string(), generation, o.clone())));
    Ok(o)
}
