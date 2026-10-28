//! THE SALES IMAGE'S READS IN THE VENUE'S OBJECT (R-BEBOP-LEVERS §5.4, lane LV-COL-E): the trace's
//! byte-for-byte check of a day's block, and the span's food cost for the analytics answer. The
//! image is written by `cube_catch_up` (`cube.rs`), beside the cube, from the same archives.

use crate::hubdo::HubImages;
use crate::services::analytics::cube::sales as ss;
use dowiz_hub::block::kernels::{pack::Pack, sales_block};
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
            crate::services::analytics::history::food_cost::food_cost(&[sales_block(r)], &pack.dishes, &|id: &str| json!(id)).unwrap_or_else(|e| json!({ "error": e }))
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
        let (_, bytes) = self.sales_image().await?;
        let (from, to) = s.days();
        let pack = match Pack::decode(&bytes) {
            Ok(p) => p,
            Err(r) => return Ok(json!({ "error": format!("the sales image is unreadable: {r:?}") })),
        };
        let mut blocks = match dowiz_hub::block::kernels::pack::days_in(&bytes, from as i32, to as i32) {
            Ok(b) => b,
            Err(r) => return Ok(json!({ "error": format!("{r:?}") })),
        };
        let cold_days = blocks.len();
        let hot: Vec<(String, Value)> = self
            .orders_view()
            .await?
            .1
            .into_iter()
            .filter_map(|v| serde_json::from_str::<Value>(&v.order_json).ok().map(|o| (v.order_id, o)))
            .filter(|(_, o)| crate::services::orders::mine::belongs_to(o, venue))
            .collect();
        let hot_rows = ss::sale_rows(&hot, zone, now);
        let mut names = pack.dishes.clone();
        names.extend(hot_rows.iter().map(|(r, id)| (r.dish, id.clone())));
        let in_span: Vec<_> = hot_rows.into_iter().map(|(r, _)| r).filter(|r| (from..=to).contains(&i64::from(r.day))).collect();
        blocks.push(sales_block(&in_span));
        let mut v = crate::services::analytics::history::food_cost::food_cost(&blocks, &names, &|id: &str| json!(id)).unwrap_or_else(|e| json!({ "error": e }));
        v["blocks"] = json!(cold_days);
        v["hotLines"] = json!(in_span.len());
        Ok(v)
    }
}
