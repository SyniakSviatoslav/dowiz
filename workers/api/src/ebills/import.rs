//! THE IMPORT, AS ONE DECISION THE VENUE'S OBJECT MAKES (BLUEPRINT-EBILLS
//! §6.1). Pure: the log, the shelf and the crosswalk in memory, the mapped
//! sales in, the events appended IN PLACE; the caller writes only on `Ok`.
//!
//! CHECK AND APPEND ARE ONE TURN. The hub does not refuse a second `Placed`
//! for an id it holds (`Hub::holds` is about content ids), so "is this sale
//! already here?" must be asked of the same copy the append goes to, inside
//! the object -- never a Worker read, a hop, and a write that hoped.
//!
//!   unknown uuid                        -> `Placed` (born PICKED_UP), and the
//!                                          shelf draws what it served (§6.4)
//!   known, total or logCis length moved -> `Noted`, flagged for the owner
//!   known and unchanged                 -> nothing at all: the bytes stand
//!   a bill                              -> `Paid` on its sitting's courses,
//!                                          never an order (§1.6)

use super::state::{Noted, PendingBill, PendingLead, PENDING_FOR_MS};
use crate::command::amend::next_seq;
use crate::command::Refused;
use crate::hubdo::OrderView;
use dowiz_hub::stock::{reservations_for, StockEvent, StockLog};
use dowiz_hub::{EventKind, Hub};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// One sale, already mapped by the Worker (`map.rs`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "snake_case")]
pub(crate) enum Mapped {
    /// A course or a counter sale: `to_order`'s envelope.
    Order { sale_id: i64, envelope: Value },
    /// A bill: `to_paid`'s payload.
    Bill { sale_id: i64, paid: Value },
    /// A listed row re-read for the week (§6.6): only its fingerprint.
    Seen { sale_id: i64, order_id: String, total: i64, log_cis_len: i64 },
    /// A bill re-read for the week: checked against the one applied, and
    /// NEVER queued -- a bill older than the import has no courses here.
    Recheck { sale_id: i64, paid: Value },
    /// A course from before the first window: placed only if a bill joins it.
    Lead { sale_id: i64, envelope: Value },
}

/// What earlier firings left waiting: bills with no courses yet, and leads.
#[derive(Debug, Clone, Default)]
pub(crate) struct Waiting {
    pub(crate) bills: Vec<PendingBill>,
    pub(crate) leads: Vec<PendingLead>,
}

#[derive(Debug, Default)]
pub(crate) struct Outcome {
    /// `(kind, order_id, body)` in append order, for the broadcast.
    pub(crate) written: Vec<(EventKind, String, String)>,
    pub(crate) placed: u64,
    pub(crate) noted: u64,
    pub(crate) paid: u64,
    pub(crate) unchanged: u64,
    pub(crate) pending: Vec<PendingBill>,
    /// Leads no bill has claimed yet (dropped after `PENDING_FOR_MS`).
    pub(crate) leads: Vec<PendingLead>,
    /// Sales not taken, and bills given up on: `(sale_id, why)`.
    pub(crate) refused: Vec<Noted>,
    /// Every till line seen: `(code, name, unit price)`.
    pub(crate) seen: Vec<(String, String, i64)>,
}

type Book = BTreeMap<String, (Value, u64)>;
/// Leads by order id: `(envelope, sale_id, since_ms)`.
type Leads = BTreeMap<String, (Value, i64, i64)>;

/// Which courses a bill pays (`import/join.rs`).
mod join;
/// What a bill does to the log: `Paid` on its courses, or `Noted` if amended.
mod bill;
use bill::{bill, known_bill};

/// What the object knows that the batch needs: the crosswalk and the
/// catalogue, as lookups, so nothing is copied that is not asked for.
pub(crate) struct Lookups<'a> {
    /// ebills `itemCode` -> dowiz `product_id`, as the OWNER set it.
    pub(crate) map: &'a dyn Fn(&str) -> Option<String>,
    /// dowiz `product_id` -> the catalogue record (for its `bom`).
    pub(crate) product: &'a dyn Fn(&str) -> Option<String>,
}

/// `(total, logCis length)` -- what an amendment in ebills moves (§6.1).
fn fingerprint(o: &Value) -> (i64, i64) {
    let rev = o.get("ebills_revision");
    let pick = |a: &str, b: Option<&Value>| rev.and_then(|r| r.get(a)).or(b).and_then(Value::as_i64).unwrap_or(-1);
    (pick("total", o.get("total")), pick("log_cis_len", o.pointer("/external/log_cis_len")))
}

fn append(hub: &mut Hub, out: &mut Outcome, book: &mut Book, kind: EventKind, id: &str, before: Option<&Value>, after: Value, seq: u64) -> Result<(), Refused> {
    let body = match before {
        Some(b) => crate::fold::delta(b, &after).to_string(),
        None => after.to_string(),
    };
    hub.append(kind, id, &body, seq, [0u8; 32]).map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    out.written.push((kind, id.to_string(), body));
    book.insert(id.to_string(), (after, seq));
    Ok(())
}

/// A sale ebills changed after it was imported: a `Noted`, never a second
/// `Placed` and never silence. The money on the order does not move -- the
/// new figures ride beside it for the owner, flagged.
fn note(hub: &mut Hub, out: &mut Outcome, book: &mut Book, id: &str, (total, len): (i64, i64), sale_id: i64, now: i64) -> Result<(), Refused> {
    let Some((cur, seq)) = book.get(id).cloned() else { return Ok(()) };
    if fingerprint(&cur) == (total, len) {
        out.unchanged += 1;
        return Ok(());
    }
    let mut after = cur.clone();
    after["ebills_revision"] = json!({ "total": total, "log_cis_len": len, "sale_id": sale_id, "at_ms": now });
    after["ebills_changed"] = json!(true);
    append(hub, out, book, EventKind::Noted, id, Some(&cur), after, next_seq(seq, now))?;
    out.noted += 1;
    Ok(())
}

/// A course or counter sale: `Placed` once, with the crosswalk applied and the
/// shelf drawn for what was served; a repeat is `note`'s question.
fn order(hub: &mut Hub, stock: &mut StockLog, look: &Lookups, out: &mut Outcome, book: &mut Book, sale_id: i64, mut env: Value, now: i64) -> Result<(), Refused> {
    let Some(id) = env.get("id").and_then(Value::as_str).map(str::to_string) else {
        out.refused.push(Noted { at_ms: now, sale_id, why: "no id on the mapped sale".into() });
        return Ok(());
    };
    if book.contains_key(&id) {
        let fp = (env.get("total").and_then(Value::as_i64).unwrap_or(-1), env.pointer("/external/log_cis_len").and_then(Value::as_i64).unwrap_or(-1));
        return note(hub, out, book, &id, fp, sale_id, now);
    }
    let mut bom: Vec<(String, i64)> = Vec::new();
    for item in env.get_mut("items").and_then(Value::as_array_mut).into_iter().flatten() {
        let Some(code) = item.get("product_id").and_then(Value::as_str).and_then(|p| p.strip_prefix("ebills:")).map(str::to_string) else { continue };
        let name = item.get("name").and_then(Value::as_str).unwrap_or("").to_string();
        let qty = item.get("quantity").and_then(Value::as_i64).unwrap_or(0);
        out.seen.push((code.clone(), name, item.get("unit_price").and_then(Value::as_i64).unwrap_or(0)));
        // UNMATCHED STAYS `ebills:<code>` AND DRAWS NOTHING: a guess would
        // take the wrong salmon off the shelf. Matched lines draw their BOM.
        if let Some(pid) = (look.map)(&code) {
            item["ebills_code"] = json!(code);
            item["product_id"] = json!(pid);
            if let Some(p) = (look.product)(&pid) {
                bom.push((p, qty));
            }
        }
    }
    let served: Vec<StockEvent> = reservations_for(&id, &bom)
        .into_iter()
        .filter_map(|e| match e {
            StockEvent::Reserved { item, qty, order_id } => Some(StockEvent::Served { item, qty, order_id }),
            _ => None,
        })
        .collect();
    if let Err(u) = crate::services::ordering::channel::of(&env) {
        out.refused.push(Noted { at_ms: now, sale_id, why: u.to_string() });
        return Ok(());
    }
    if !served.is_empty() {
        // §6.4: `Served` is never refused for the shelf; only arithmetic can.
        if let Err(e) = stock.append_all(&served) {
            out.refused.push(Noted { at_ms: now, sale_id, why: format!("stock: {e}") });
            return Ok(());
        }
    }
    let seq = env.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now).max(0) as u64;
    let void_of = env.get("void_of").and_then(Value::as_str).map(str::to_string);
    append(hub, out, book, EventKind::Placed, &id, None, env, seq)?;
    out.placed += 1;
    // A VOID NAMES WHAT IT VOIDS on the voided order too, so the owner sees
    // it where the money was: a `Noted`, the original's total untouched --
    // the void is its own negative order, and the two net to zero.
    let voided = void_of.and_then(|v| book.get(&v).cloned().map(|held| (v, held)));
    if let Some((vid, (before, cseq))) = voided {
        if before.get("ebills_voided_by").is_none() {
            let mut after = before.clone();
            after["ebills_voided_by"] = json!({ "order_id": id, "sale_id": sale_id });
            after["ebills_changed"] = json!(true);
            append(hub, out, book, EventKind::Noted, &vid, Some(&before), after, next_seq(cseq, now))?;
            out.noted += 1;
            // AND THE SHELF GETS BACK WHAT THE VOIDED SALE TOOK: exactly its
            // unreversed `Served`, which the ledger refuses to reverse twice.
            let back: Vec<StockEvent> = match stock.ledger() {
                Ok(l) => l.served_of(&vid).into_iter().map(|(item, qty)| StockEvent::Unserved { item, qty, order_id: vid.clone() }).collect(),
                Err(e) => {
                    out.refused.push(Noted { at_ms: now, sale_id, why: format!("stock unreadable, void not put back: {e}") });
                    Vec::new()
                }
            };
            if !back.is_empty() {
                if let Err(e) = stock.append_all(&back) {
                    out.refused.push(Noted { at_ms: now, sale_id, why: format!("stock: {e}") });
                }
            }
        }
    }
    Ok(())
}

/// THE WHOLE IMPORT over images already in memory. Orders first (ascending
/// sale id, so a bill's courses are in the book before it), then the bills
/// still waiting from earlier firings, then this batch's. A storage error is
/// the only thing that refuses the batch; a sale that cannot be taken is a
/// named refusal in the outcome and the rest go on.
pub(crate) fn decide(hub: &mut Hub, stock: &mut StockLog, listed: &[OrderView], look: &Lookups, waiting: Waiting, sales: &[Mapped], now: i64) -> Result<Outcome, Refused> {
    let mut book: Book = listed
        .iter()
        .filter(|v| v.order_id.starts_with("ebills:"))
        .filter_map(|v| serde_json::from_str(&v.order_json).ok().map(|o| (v.order_id.clone(), (o, v.seq))))
        .collect();
    let mut out = Outcome::default();
    let mut sorted: Vec<&Mapped> = sales.iter().collect();
    sorted.sort_by_key(|m| match m {
        Mapped::Order { sale_id, .. } | Mapped::Bill { sale_id, .. } | Mapped::Seen { sale_id, .. } | Mapped::Recheck { sale_id, .. } | Mapped::Lead { sale_id, .. } => *sale_id,
    });
    let mut bills = waiting.bills;
    let mut leads: Leads = waiting.leads.into_iter().map(|l| (l.envelope["id"].as_str().unwrap_or("").to_string(), (l.envelope, l.sale_id, l.since_ms))).collect();
    for m in sorted {
        match m {
            Mapped::Order { sale_id, envelope } => order(hub, stock, look, &mut out, &mut book, *sale_id, envelope.clone(), now)?,
            Mapped::Seen { sale_id, order_id, total, log_cis_len } => note(hub, &mut out, &mut book, order_id, (*total, *log_cis_len), *sale_id, now)?,
            Mapped::Bill { sale_id, paid } if !bills.iter().any(|b| b.sale_id == *sale_id) => {
                bills.push(PendingBill { sale_id: *sale_id, paid: paid.clone(), since_ms: now })
            }
            Mapped::Bill { .. } => {}
            Mapped::Recheck { sale_id, paid } => {
                known_bill(hub, &mut out, &mut book, *sale_id, paid, now)?;
            }
            Mapped::Lead { sale_id, envelope } => {
                let id = envelope.get("id").and_then(Value::as_str).unwrap_or("").to_string();
                if !id.is_empty() && !book.contains_key(&id) {
                    leads.entry(id).or_insert((envelope.clone(), *sale_id, now));
                }
            }
        }
    }
    for b in bills {
        if let Some(still) = bill(hub, stock, look, &mut out, &mut book, &mut leads, b, now)? {
            out.pending.push(still);
        }
    }
    // A LEAD NO BILL CLAIMED is a sale from before the link's first window:
    // kept while a bill might still arrive for it, then let go -- never placed.
    out.leads = leads
        .into_iter()
        .filter(|(_, (_, _, since))| now - since <= PENDING_FOR_MS)
        .map(|(_, (envelope, sale_id, since_ms))| PendingLead { sale_id, envelope, since_ms })
        .collect();
    Ok(out)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod more_tests;
