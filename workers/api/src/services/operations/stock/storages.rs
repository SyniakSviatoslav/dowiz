//! STORAGES, TRANSFERS AND FREEZING RECORDS through the stock door (P12, P13,
//! W-STORE), pure. Three kinds on `POST /api/owner/stock/:kind`, no new route:
//!
//!   storage  {card: {id?, name, archived?}} -- name, rename or archive one;
//!            the id is minted from the name when empty. Refused: archiving
//!            the default storage, or one that still holds stock.
//!   moved    {item, qty, from, to}          -- a transfer: never makes or
//!            destroys stock; refused past what `from` holds.
//!   frozen   {item, lot, hours, tempC, store?} -- an in-house freezing of a
//!            lot (853/2004 Annex III VIII): written whatever rule it meets,
//!            and the answer says which one (`rule`), never blocking a sale.
//!
//! All three are NOTES on the stock log (`dowiz_hub::stock::{storages,
//! haccp}`): the shelf, the cost, the lots and every checkpoint of an old
//! venue are as they were.

use dowiz_hub::stock::haccp::{rule, Frozen};
use dowiz_hub::stock::journal::Journal;
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::storages::{Moved, Storage, Stores, DEFAULT, DEFAULTS, FREEZER};
use dowiz_hub::stock::{Qty, StockError, StockEvent, StockLedger, StockLog};
use serde::Deserialize;
use serde_json::{json, Value};

use super::turn::{StockTurnIn, Told};

pub const STORAGE: &str = "storage";
pub const MOVED: &str = "moved";
pub const FROZEN: &str = "frozen";

/// Is `kind` one of this file's?
pub fn is_mine(kind: &str) -> bool {
    kind == STORAGE || kind == MOVED || kind == FROZEN
}

/// A storage card as the console sends it.
#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct CardIn {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub archived: bool,
}

type Bad = (u16, String);

fn refused(e: StockError) -> Bad {
    (409, e.to_string())
}

/// An id from a name: lowercase ASCII letters and digits, `-` between words;
/// a name with none (Кухня 2) gets `s<n>`. PURE.
pub fn mint_id(name: &str, taken: &[Storage]) -> String {
    let mut id = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else if !id.ends_with('-') && !id.is_empty() {
            id.push('-');
        }
    }
    let mut id: String = id.trim_end_matches('-').chars().take(24).collect();
    if id.is_empty() || taken.iter().any(|s| s.id == id) {
        let base = if id.is_empty() { "s".to_string() } else { id };
        let mut n = taken.len() + 1;
        while taken.iter().any(|s| s.id == format!("{base}{n}")) {
            n += 1;
        }
        id = format!("{base}{n}");
    }
    id
}

/// One of this file's kinds, applied. A refusal writes nothing.
pub fn run(log: &mut StockLog, input: &StockTurnIn) -> Result<(Value, Told), Bad> {
    log.set_clock(input.now_ms);
    let b = &input.body;
    let s = |k: &str| b.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string();
    let item = s("item");
    if input.kind != STORAGE && !input.supplies.contains_key(&item) {
        return Err((404, format!("not found: {item}")));
    }
    let shown = match input.kind.as_str() {
        STORAGE => {
            let card: CardIn = serde_json::from_value(b.get("card").cloned().unwrap_or(Value::Null)).map_err(|e| (400, format!("bad storage card: {e}")))?;
            let known = log.storages();
            let id = match card.id.trim() {
                "" => mint_id(&card.name, &known),
                id => id.to_string(),
            };
            log.put_storage(&Storage { id: id.clone(), name: card.name.trim().to_string(), archived: card.archived }).map_err(refused)?;
            json!({ "ok": true, "kind": STORAGE, "id": id, "archived": card.archived })
        }
        MOVED => {
            let qty = b.get("qty").and_then(Value::as_i64).ok_or((400, "how much?".to_string()))?;
            let m = Moved { item: item.clone(), qty, from: s("from"), to: s("to"), by: input.by.clone() };
            log.move_stock(&m).map_err(refused)?;
            json!({ "ok": true, "kind": MOVED, "item": item, "qty": qty, "from": m.from, "to": m.to })
        }
        _ => {
            let int = |k: &str| b.get(k).and_then(Value::as_i64).ok_or((400, format!("{k}?")));
            let (hours, temp_c) = (int("hours")?, int("tempC")?);
            let store = Some(s("store")).filter(|x| !x.is_empty()).unwrap_or_else(|| FREEZER.to_string());
            let f = Frozen { item: item.clone(), lot: s("lot"), at: input.now_ms, hours, temp_c, store, by: input.by.clone() };
            log.record_frozen(&f).map_err(refused)?;
            json!({ "ok": true, "kind": FROZEN, "item": item, "lot": f.lot, "hours": hours, "tempC": temp_c, "rule": rule(temp_c, hours) })
        }
    };
    Ok((shown, Vec::new()))
}

/// The storages the movement plan names, checked against the venue's, and
/// the fold a count in one of them needs.
pub struct Rooms {
    stores: Stores,
    led: StockLedger,
}

impl Rooms {
    /// Refused when a line names a storage the venue does not have, or one
    /// archived.
    pub fn of(log: &StockLog, lines: &[(StockEvent, Meta)]) -> Result<Rooms, StockError> {
        let known = log.storages();
        for (_, m) in lines {
            if let Some(st) = m.store.as_deref() {
                if !known.iter().any(|s| s.id == st && !s.archived) {
                    return Err(StockError::Linkage(format!("{st}: no such storage here, or it is archived")));
                }
            }
        }
        let j = if lines.iter().any(|(e, m)| m.store.is_some() && matches!(e, StockEvent::Stocktake { .. })) {
            log.journal_now()?
        } else {
            Journal::default()
        };
        Ok(Rooms { stores: j.stores, led: j.ledger })
    }

    /// The shelf's total for `item` when `store` is counted at `observed`.
    pub fn total_for(&self, item: &str, store: &str, observed: Qty) -> Qty {
        let others: Qty = self.stores.of(item, &self.led).iter().filter(|(s, _)| s != store).map(|x| x.1).sum();
        observed.saturating_add(others)
    }
}

/// The venue's storages for the Stock screen. PURE.
pub fn list(log: &StockLog) -> Value {
    let all: Vec<Value> = log
        .storages()
        .into_iter()
        .map(|s| json!({ "id": s.id, "name": s.name, "archived": s.archived, "builtIn": DEFAULTS.contains(&s.id.as_str()), "default": s.id == DEFAULT }))
        .collect();
    Value::Array(all)
}

/// A supply's levels per storage, `{storage: qty}`, for its row. PURE.
pub fn levels(j: &Journal, id: &str) -> Value {
    let mut m = serde_json::Map::new();
    for (s, q) in j.stores.of(id, &j.ledger) {
        m.insert(s, json!(q));
    }
    json!({ "stores": m, "home": j.stores.home(id) })
}

#[cfg(test)]
#[path = "storages/tests.rs"]
mod tests;
