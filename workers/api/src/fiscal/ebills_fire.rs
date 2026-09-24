//! ONE FIRING OF THE eBills SENDER, over a `Transport` -- the Worker's
//! allow-listed `ebills::fetch::Client` in production, a mock till in every
//! test (card L70 HARD LIMIT: no test sends anything to ebills.al).
//!
//! THE ORDER OF THINGS, and why each is where it is:
//!   1. The sale list (yesterday..today) is read FIRST, always: it is the
//!      link's health, the reconcile's evidence, and where the default client
//!      is found. A failure here sends nothing (`NotSent`).
//!   2. `Recheck` items: an unfiscalised sale read by id -- a read, never a POST.
//!   3. The till's context (menu rows, the chosen sale unit, a till sale's
//!      default-client/POS/currency blocks), read once, passed through, kept
//!      nowhere (EBILLS-WRITE-PATH §6).
//!   4. Each document: RECONCILE first if an earlier send went unanswered,
//!      then build (refusals by name), then ONE `POST /api/sales`.
//!   5. An unanswered POST stops the batch: the platform is not answering,
//!      and every further POST would be another unknown invoice.

use serde_json::Value;

use super::document::Document;
use super::ebills_body::{body, marker, Till};
use super::ebills_sender::{Action, Item, Outcome};
use super::sender::Codes;
use crate::ebills::client::Path;
use crate::ebills::judge::{created, sale_state, Answer, Created, Fail};
use crate::ebills::parse_item_rows;

/// How many recent till sales are read for a default-client template.
const TEMPLATE_TRIES: usize = 3;

/// The one door a firing has: allow-listed reads, and the one create.
#[allow(async_fn_in_trait)]
pub(crate) trait Transport {
    async fn read(&mut self, p: &Path) -> Result<String, Fail>;
    async fn create(&mut self, body: &str) -> Result<Answer, Fail>;
}

/// What the object hands a firing (`hubdo/fiscal_send.rs`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Batch {
    pub pos_id: i64,
    pub today: String,
    pub yesterday: String,
    pub sale_unit: String,
    #[serde(default)]
    pub fee_item: Option<String>,
    pub crosswalk: Vec<(String, String)>,
    pub items: Vec<Item>,
}

/// An allow-listed read's reason as the outcome of every item it stopped.
fn stop(items: &[&Item], why: &str) -> Vec<(String, Outcome)> {
    items.iter().map(|i| (i.uuid.clone(), Outcome::NotSent(why.to_string()))).collect()
}

/// A sale the platform holds, ruled: the codes, or why not yet.
fn outcome_of(c: Created) -> Outcome {
    match c {
        Created::Fiscalised { sale_id, iic, fic, inv_ord_num } => Outcome::Sent { sale_id, codes: Codes { iic, fic, inv_ord_num } },
        Created::Unfiscalised { sale_id, fault } => Outcome::Unfiscalised { sale_id, fault },
        Created::Refused(s, k) => Outcome::Refused(format!("{s} {k}")),
        Created::Auth(m) => Outcome::Refused(format!("the session was refused after a fresh login: {m}")),
        Created::Unknown(m) => Outcome::Unknown(m),
    }
}

/// `/api/sales/{id}` read and ruled like a create's answer.
async fn read_sale<T: Transport>(t: &mut T, id: i64, pos: i64) -> Outcome {
    match t.read(&Path::Sale { id, pos }).await {
        Ok(b) => match serde_json::from_str::<Value>(&b) {
            Ok(v) => match outcome_of(sale_state(&v["sale"])) {
                Outcome::Unknown(m) => Outcome::Inconclusive(format!("sale {id} read back without its state: {m}")),
                o => o,
            },
            Err(e) => Outcome::NotSent(format!("sale {id}: {e}")),
        },
        Err(Fail::NotFound) => Outcome::Inconclusive(format!("sale {id} is gone from ebills")),
        Err(f) => Outcome::NotSent(f.line()),
    }
}

/// What the sale list says about one marker (§3 (b)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Sale(i64),
    NotFound,
    /// The list carries no `notes` at all: it cannot settle anything.
    NoNotes,
    /// The marker is there with another total: the owner must look.
    Mismatch(i64),
}

/// Find an unanswered send in the list by its `notes` marker, total as the tiebreak.
pub fn reconcile(rows: &[Value], mark: &str, total: i64) -> Found {
    if !rows.is_empty() && !rows.iter().any(|r| r.get("notes").is_some()) {
        return Found::NoNotes;
    }
    let mut hits = rows.iter().filter(|r| r.get("notes").and_then(Value::as_str) == Some(mark));
    let Some(hit) = hits.next() else { return Found::NotFound };
    let id = hit.get("id").and_then(Value::as_i64).unwrap_or(0);
    let lek = hit.get("totalValue").and_then(Value::as_f64).and_then(|v| crate::ebills::whole(v, "totalValue").ok());
    match lek == Some(total) {
        true if id > 0 => Found::Sale(id),
        _ => Found::Mismatch(id),
    }
}

/// A till sale's default client, POS and currency, as a create must echo them.
pub fn template_of(sale: &Value, pos: i64) -> Option<(Value, Value, Value)> {
    let client = sale.get("client").filter(|c| c.get("defaultClient") == Some(&Value::Bool(true)))?;
    let point = sale.get("pointOfSale").filter(|p| p.get("id").and_then(Value::as_i64) == Some(pos))?;
    let cur = sale.get("currency").filter(|c| c.get("currencyCode").and_then(Value::as_str) == Some("ALL"))?;
    Some((client.clone(), point.clone(), cur.clone()))
}

/// The sale unit the owner chose, as the floor serves it, cut to the six
/// keys a sale's `saleUnit` carries (§1.6) -- never the waiter's name.
pub fn sale_unit_of(tables: &str, identifier: &str) -> Result<Value, String> {
    let rows: Vec<Value> = serde_json::from_str(tables).map_err(|e| format!("ebills tables: {e}"))?;
    let row = rows.iter().find(|r| r.get("identifier").and_then(Value::as_str) == Some(identifier));
    let row = row.ok_or_else(|| format!("sale unit {identifier:?} is not on this point of sale's floor"))?;
    let keep = ["id", "identifier", "type", "status", "posX", "posY"];
    Ok(Value::Object(keep.iter().filter_map(|k| row.get(*k).map(|v| (k.to_string(), v.clone()))).collect()))
}

/// Step 3: the till's context, or why it could not be read.
async fn context<T: Transport>(t: &mut T, b: &Batch, rows: &[Value]) -> Result<Till, String> {
    let items = parse_item_rows(&t.read(&Path::Items { size: 500 }).await.map_err(|f| f.line())?)?;
    let tables = t.read(&Path::Tables { pos: b.pos_id }).await.map_err(|f| f.line())?;
    let unit = sale_unit_of(&tables, &b.sale_unit)?;
    let mut tried = 0;
    for id in rows.iter().filter_map(|r| r.get("id").and_then(Value::as_i64)) {
        if tried == TEMPLATE_TRIES {
            break;
        }
        tried += 1;
        let detail: Value = serde_json::from_str(&t.read(&Path::Sale { id, pos: b.pos_id }).await.map_err(|f| f.line())?).map_err(|e| format!("sale {id}: {e}"))?;
        if let Some((client, pos, currency)) = template_of(&detail["sale"], b.pos_id) {
            let fee_code = b.fee_item.clone().filter(|f| !f.is_empty());
            return Ok(Till { items, crosswalk: b.crosswalk.clone(), client, pos, currency, sale_unit: Some(unit), fee_code });
        }
    }
    Err(format!("no recent till sale on POS {} with the default client to copy it from", b.pos_id))
}

/// THE FIRING. Every item of the batch gets exactly one outcome.
pub(crate) async fn fire<T: Transport>(t: &mut T, b: &Batch) -> Vec<(String, Outcome)> {
    let all: Vec<&Item> = b.items.iter().collect();
    if all.is_empty() {
        return Vec::new();
    }
    let list = Path::Sales { begin: b.yesterday.clone(), end: b.today.clone(), pos: b.pos_id, size: 500 };
    let rows: Vec<Value> = match t.read(&list).await.map(|s| serde_json::from_str::<Value>(&s)) {
        Ok(Ok(v)) => match v.get("sales").and_then(Value::as_array) {
            Some(r) => r.clone(),
            None => return stop(&all, "ebills sale list: no `sales`"),
        },
        Ok(Err(e)) => return stop(&all, &format!("ebills sale list: {e}")),
        Err(f) => return stop(&all, &f.line()),
    };
    let mut out = Vec::with_capacity(all.len());
    for i in all.iter().filter(|i| matches!(i.action, Action::Recheck(_))) {
        let Action::Recheck(id) = i.action else { continue };
        out.push((i.uuid.clone(), read_sale(t, id, b.pos_id).await));
    }
    let sends: Vec<&Item> = all.into_iter().filter(|i| !matches!(i.action, Action::Recheck(_))).collect();
    if sends.is_empty() {
        return out;
    }
    let till = match context(t, b, &rows).await {
        Ok(c) => c,
        Err(why) => return out.into_iter().chain(stop(&sends, &why)).collect(),
    };
    let mut halted: Option<String> = None;
    for i in sends {
        if let Some(why) = &halted {
            out.push((i.uuid.clone(), Outcome::NotSent(why.clone())));
            continue;
        }
        let o = send_one(t, b, &till, &rows, i).await;
        if matches!(o, Outcome::Unknown(_)) {
            halted = Some("an earlier send in this firing went unanswered".into());
        }
        out.push((i.uuid.clone(), o));
    }
    out
}

/// Step 4 for one document.
async fn send_one<T: Transport>(t: &mut T, b: &Batch, till: &Till, rows: &[Value], i: &Item) -> Outcome {
    let Ok(doc) = serde_json::from_str::<Document>(&i.doc) else {
        return Outcome::Blocked("the queued document does not parse".into());
    };
    let mark = marker(&doc.order_id);
    if i.action == Action::ReconcileThenSend {
        match reconcile(rows, &mark, doc.total) {
            Found::Sale(id) => return read_sale(t, id, b.pos_id).await,
            Found::NotFound => {}
            Found::NoNotes => return Outcome::Inconclusive("an earlier send went unanswered and the sale list carries no notes to find it by: check ebills by hand".into()),
            Found::Mismatch(id) => return Outcome::Inconclusive(format!("sale {id} carries {mark} with another total: check ebills by hand")),
        }
    }
    let wire = match body(&doc, till, Some(&mark)) {
        Ok(w) => w.to_string(),
        Err(r) => return Outcome::Blocked(r.line()),
    };
    match t.create(&wire).await {
        Ok(a) => outcome_of(created(&a)),
        Err(Fail::Network(m)) => Outcome::Unknown(format!("no answer: {m}")),
        Err(Fail::NotAllowed(u)) => Outcome::Refused(format!("the allow-list refused {u}")),
        // A LOGIN THAT FAILED BEFORE THE POST: nothing was sent.
        Err(f) => Outcome::NotSent(f.line()),
    }
}

#[cfg(test)]
mod tests;
