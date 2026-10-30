//! The stock event as one line of JSON, and back -- the record format of the log.
//!
//! THE NONCE IN FRONT (M0, `crate::chain`). A record written since M0 is 16 base64url
//! characters and then its JSON object; one written before is the object alone. Every stock
//! record is a JSON object, so the first byte says which: `{` is a legacy record, anything else
//! a nonced one -- and that is how the wire form (`glue::wire`), which carries records as
//! text with no flags cell, tells the hub and a tablet which id scheme each record is under.
//! `str_field`/`int_field` scan for a key and read a nonced record as is; a parser that needs
//! the object to START the text takes `rec_body`.

use super::*;

/// Does this record text carry an M0 nonce in front of its JSON object?
pub fn nonced(rec: &str) -> bool {
    nonced_bytes(rec.as_bytes())
}

/// `nonced`, over the payload bytes.
pub fn nonced_bytes(payload: &[u8]) -> bool {
    payload.len() >= crate::chain::NONCE_LEN && payload.first() != Some(&b'{')
}

/// The record's JSON object: the text without its nonce.
pub fn rec_body(rec: &str) -> &str {
    if nonced(rec) {
        rec.get(crate::chain::NONCE_LEN..).unwrap_or("")
    } else {
        rec
    }
}

/// `rec_body`, over the payload bytes.
pub fn body_bytes(payload: &[u8]) -> &[u8] {
    if nonced_bytes(payload) {
        payload.get(crate::chain::NONCE_LEN..).unwrap_or(&[])
    } else {
        payload
    }
}

// ── serialisation, so the log survives a restart ────────────────────────────

pub fn encode(ev: &StockEvent) -> String {
    match ev {
        StockEvent::Received { item, qty } => {
            format!(r#"{{"k":"received","item":"{}","qty":{qty}}}"#, esc(item))
        }
        StockEvent::Reserved { item, qty, order_id } => format!(
            r#"{{"k":"reserved","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Consumed { item, qty, order_id } => format!(
            r#"{{"k":"consumed","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Released { item, qty, order_id } => format!(
            r#"{{"k":"released","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Wasted { item, qty, reason, by } => format!(
            r#"{{"k":"wasted","item":"{}","qty":{qty},"reason":"{}","by":"{}"}}"#,
            esc(item),
            reason.as_str(),
            esc(by)
        ),
        StockEvent::Stocktake { item, observed, stocktake_id, by } => format!(
            r#"{{"k":"stocktake","item":"{}","observed":{observed},"id":"{}","by":"{}"}}"#,
            esc(item),
            esc(stocktake_id),
            esc(by)
        ),
        StockEvent::Served { item, qty, order_id } => format!(
            r#"{{"k":"served","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Unserved { item, qty, order_id } => format!(
            r#"{{"k":"unserved","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Returned { item, qty, order_id, resell, by, chosen_by } => format!(
            r#"{{"k":"returned","item":"{}","qty":{qty},"order":"{}","resell":{},"by":"{}","chosen_by":"{}"}}"#,
            esc(item),
            esc(order_id),
            i64::from(*resell),
            esc(by),
            esc(chosen_by)
        ),
        StockEvent::Produced { item, qty, out, stage, into, by } => format!(
            r#"{{"k":"produced","item":"{}","qty":{qty},"out":{out},"stage":"{}","into":"{}","by":"{}"}}"#,
            esc(item),
            stage.as_str(),
            esc(into.as_deref().unwrap_or("")),
            esc(by)
        ),
        StockEvent::Removed { item, by } => format!(r#"{{"k":"removed","item":"{}","by":"{}"}}"#, esc(item), esc(by)),
        StockEvent::Cooked { item, qty, into, act, by } => format!(
            r#"{{"k":"cooked","item":"{}","qty":{qty},"into":"{}","act":"{}","by":"{}"}}"#,
            esc(item),
            esc(into),
            esc(act),
            esc(by)
        ),
        StockEvent::Made { item, qty, planned, gross, act, by } => format!(
            r#"{{"k":"made","item":"{}","qty":{qty},"planned":{planned},"gross":{gross},"act":"{}","by":"{}"}}"#,
            esc(item),
            esc(act),
            esc(by)
        ),
    }
}

pub fn decode(rec: &str) -> Option<StockEvent> {
    let item = str_field(rec, "item")?;
    match str_field(rec, "k")?.as_str() {
        "received" => Some(StockEvent::Received { item, qty: int_field(rec, "qty")? }),
        "reserved" => Some(StockEvent::Reserved {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "consumed" => Some(StockEvent::Consumed {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "released" => Some(StockEvent::Released {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "wasted" => Some(StockEvent::Wasted {
            item,
            qty: int_field(rec, "qty")?,
            reason: WasteReason::from_str(&str_field(rec, "reason")?)?,
            // Absent on every record written before 2026-09-23: "" = unsigned.
            by: str_field(rec, "by").unwrap_or_default(),
        }),
        "stocktake" => Some(StockEvent::Stocktake {
            item,
            observed: int_field(rec, "observed")?,
            stocktake_id: str_field(rec, "id")?,
            // Absent on every record written before 2026-09-23: "" = unsigned.
            by: str_field(rec, "by").unwrap_or_default(),
        }),
        "served" => Some(StockEvent::Served {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "unserved" => Some(StockEvent::Unserved {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
        }),
        "returned" => Some(StockEvent::Returned {
            item,
            qty: int_field(rec, "qty")?,
            order_id: str_field(rec, "order")?,
            resell: match int_field(rec, "resell")? { 0 => false, 1 => true, _ => return None },
            by: str_field(rec, "by")?,
            chosen_by: str_field(rec, "chosen_by")?,
        }),
        "produced" => Some(StockEvent::Produced {
            item,
            qty: int_field(rec, "qty")?,
            out: int_field(rec, "out")?,
            stage: PrepStage::from_str(&str_field(rec, "stage")?)?,
            into: str_field(rec, "into").filter(|t| !t.is_empty()),
            by: str_field(rec, "by")?,
        }),
        "removed" => Some(StockEvent::Removed { item, by: str_field(rec, "by")? }),
        "cooked" => Some(StockEvent::Cooked {
            item,
            qty: int_field(rec, "qty")?,
            into: str_field(rec, "into")?,
            act: str_field(rec, "act")?,
            by: str_field(rec, "by")?,
        }),
        "made" => Some(StockEvent::Made {
            item,
            qty: int_field(rec, "qty")?,
            planned: int_field(rec, "planned")?,
            gross: int_field(rec, "gross")?,
            act: str_field(rec, "act")?,
            by: str_field(rec, "by")?,
        }),
        _ => None,
    }
}
