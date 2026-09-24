//! A BILL IN THE IMPORT: `Paid` on the courses of its sitting (placing any
//! lead it covers first), `Noted` on them when ebills amends it later, and a
//! wait -- then a named refusal -- when its courses are not here.

use super::join::join;
use super::{append, order, Book, Leads, Lookups, Outcome};
use crate::command::amend::next_seq;
use crate::command::Refused;
use crate::ebills::state::{Noted, PendingBill, PENDING_FOR_MS};
use dowiz_hub::stock::StockLog;
use dowiz_hub::{EventKind, Hub};
use serde_json::{json, Value};

/// `(total, logCis length)` of a bill payload -- its amendment witness.
fn bill_fp(paid: &Value) -> (i64, i64) {
    (paid.get("total").and_then(Value::as_i64).unwrap_or(-1), paid.pointer("/external/log_cis_len").and_then(Value::as_i64).unwrap_or(-1))
}

/// A bill this log already applied: `Noted` on its courses if ebills changed
/// it since, silence if not. Answers whether the bill was known at all.
pub(super) fn known_bill(hub: &mut Hub, out: &mut Outcome, book: &mut Book, sale_id: i64, paid: &Value, now: i64) -> Result<bool, Refused> {
    let uuid = paid.pointer("/external/uuid").and_then(Value::as_str).unwrap_or("");
    let held: Vec<String> = book.iter().filter(|(_, (o, _))| !uuid.is_empty() && o.pointer("/bill/uuid").and_then(Value::as_str) == Some(uuid)).map(|(k, _)| k.clone()).collect();
    let fp = bill_fp(paid);
    for id in &held {
        let (cur, seq) = book[id].clone();
        let was = cur.get("ebills_bill_revision").or(cur.get("bill")).cloned().unwrap_or(Value::Null);
        if (was.get("total").and_then(Value::as_i64).unwrap_or(-1), was.get("log_cis_len").and_then(Value::as_i64).unwrap_or(-1)) == fp {
            out.unchanged += 1;
            continue;
        }
        let mut after = cur.clone();
        after["ebills_bill_revision"] = json!({ "total": fp.0, "log_cis_len": fp.1, "sale_id": sale_id, "at_ms": now });
        after["ebills_changed"] = json!(true);
        append(hub, out, book, EventKind::Noted, id, Some(&cur), after, next_seq(seq, now))?;
        out.noted += 1;
    }
    Ok(!held.is_empty())
}

/// `Paid` on each course of the sitting, naming the bill (§3.2 (4)). NOT a
/// `payments[]` entry: that is dowiz's drawer (law 10), and this cash went
/// into the till's. A bill whose courses are not (yet) here waits; after
/// `PENDING_FOR_MS` it is given up on by name.
#[allow(clippy::too_many_arguments)]
pub(super) fn bill(hub: &mut Hub, stock: &mut StockLog, look: &Lookups, out: &mut Outcome, book: &mut Book, leads: &mut Leads, b: PendingBill, now: i64) -> Result<Option<PendingBill>, Refused> {
    if known_bill(hub, out, book, b.sale_id, &b.paid, now)? {
        return Ok(None);
    }
    let cands = book.iter().map(|(k, (v, _))| (k, v)).chain(leads.iter().map(|(k, (v, _, _))| (k, v)));
    let Some(ids) = join(cands, &b.paid) else {
        if now - b.since_ms > PENDING_FOR_MS {
            out.refused.push(Noted { at_ms: now, sale_id: b.sale_id, why: "a bill whose courses never arrived".into() });
            return Ok(None);
        }
        return Ok(Some(b));
    };
    let (x, fp) = (b.paid.get("external").cloned().unwrap_or(Value::Null), bill_fp(&b.paid));
    let at = b.paid.get("at_ms").and_then(Value::as_i64).unwrap_or(now);
    let block = json!({
        "uuid": x.get("uuid"), "sale_id": x.get("sale_id"), "inv_ord_num": x.get("inv_ord_num"), "fic": x.get("fic"),
        "iic": x.get("iic"), "log_cis_len": fp.1, "total": fp.0, "payment": b.paid.get("payment"),
        "at_ms": at, "courses": ids.len(),
    });
    for id in ids {
        // A LEAD THIS BILL COVERS IS A SALE AFTER ALL: placed now, then paid.
        if let Some((env, sale_id, _)) = leads.remove(&id) {
            order(hub, stock, look, out, book, sale_id, env, now)?;
        }
        let Some((cur, seq)) = book.get(&id).cloned() else { continue };
        let mut after = cur.clone();
        after["bill"] = block.clone();
        after["payment_status"] = json!("paid");
        append(hub, out, book, EventKind::Paid, &id, Some(&cur), after, next_seq(seq, at))?;
        out.paid += 1;
    }
    Ok(None)
}
