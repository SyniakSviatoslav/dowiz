//! A VOID IN THE IMPORT IS A REFUND OF THE SALE IT NAMES (operator,
//! 2026-10-20: "анулювання рахуй як повернення").
//!
//! IT USED TO BE ITS OWN NEGATIVE ORDER. The two netted to zero in the
//! money, but every count read them as two orders: one table's cancelled set
//! was "2 orders, 0 lek" on the analytics pane, and the voided course still
//! counted its dishes. A refund is what the rest of the product already
//! understands: the original ends `COMPENSATED_REFUND` with a `refund` record,
//! so `status::took_money` drops it from the takings, the dishes and the
//! average on every surface, and the console shows it as refunded.
//!
//! NOT A KERNEL EDGE. `PICKED_UP` is a dead end in the order machine, and a
//! till sale never was in that machine: it is born `PICKED_UP` by the till's
//! word (`map::to_order`), and it ends by the till's word. The event is
//! `Advanced` because the status moved; the `refund.by` says who moved it.
//!
//!   the voided sale is here         -> `Noted` refund record, `Advanced` to
//!                                      COMPENSATED_REFUND, the shelf gets back
//!                                      exactly its unreversed `Served`
//!   already voided                  -> nothing (the week's re-read)
//!   the voided sale never imported  -> a named refusal; a lone negative order
//!                                      would subtract money nobody counted

use super::{append, Book, Outcome};
use crate::command::amend::next_seq;
use crate::command::Refused;
use crate::ebills::state::Noted;
use dowiz_hub::stock::{StockEvent, StockLog};
use dowiz_hub::{EventKind, Hub};
use serde_json::{json, Value};

/// The refund reason a till void records; never a word an owner can pick.
const TILL_VOID: &str = "till_void";

/// The order this envelope voids, if it is a void.
pub(super) fn target(env: &Value) -> Option<&str> {
    env.get("void_of").and_then(Value::as_str)
}

pub(super) fn void(hub: &mut Hub, stock: &mut StockLog, out: &mut Outcome, book: &mut Book, sale_id: i64, env: &Value, now: i64) -> Result<(), Refused> {
    let Some(vid) = target(env).map(str::to_string) else { return Ok(()) };
    let Some((before, cseq)) = book.get(&vid).cloned() else {
        out.refused.push(Noted { at_ms: now, sale_id, why: format!("a void of {vid}, which this venue never imported") });
        return Ok(());
    };
    if before.get("ebills_voided_by").is_some() {
        out.unchanged += 1;
        return Ok(());
    }
    let at = env.get("created_at_ms").and_then(Value::as_i64).unwrap_or(now);
    let from = before.get("status").and_then(Value::as_str).unwrap_or("").to_string();
    let mut noted = before.clone();
    noted["ebills_voided_by"] = json!({ "order_id": env.get("id"), "sale_id": sale_id });
    noted["refund"] = json!({
        "reason": TILL_VOID, "by": "ebills", "at": at, "from": from,
        "owed": before.get("total"), "currency": before.get("currency"), "sale_id": sale_id,
    });
    let seq = next_seq(cseq, at);
    append(hub, out, book, EventKind::Noted, &vid, Some(&before), noted.clone(), seq)?;
    let mut ended = noted.clone();
    ended["status"] = json!("COMPENSATED_REFUND");
    crate::live_eta::stamp(&mut ended, "COMPENSATED_REFUND", at);
    append(hub, out, book, EventKind::Advanced, &vid, Some(&noted), ended, next_seq(seq, at))?;
    out.noted += 1;
    // THE SHELF GETS BACK WHAT THE VOIDED SALE TOOK: exactly its unreversed
    // `Served`, which the ledger refuses to reverse twice.
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
    Ok(())
}
