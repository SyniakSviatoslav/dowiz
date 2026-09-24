//! D12 (G5): A REFUND HANDS A WALLET'S MONEY BACK TO THE WALLET.
//!
//! THE DEFECT. `refund::decide` records what is owed and walks the FSM to
//! COMPENSATED_REFUND; for a round paid from a wallet nothing ever touched the
//! ledger, so the SPEND stayed and the guest's balance was simply gone.
//!
//! THE RULE. When the refund is completed (`complete: true`, the moment the
//! venue says the money went back), every wallet payment on the round gets
//! its exact compensation: `ledger_account::refund_of` its SPEND, one REFUND
//! record in the shape `wallet::top_up` and `pay::wallet::debit` write. The id
//! is DERIVED (venue, order, the payment's instant), so a completion replayed
//! finds its reversal already in the journal and writes nothing twice; a
//! wallet leg that was never written (law 12's missing leg) has nothing to
//! reverse and is skipped -- the money never left that wallet.

use crate::command::pay::wallet::{tx_id_of, Debit};
use crate::command::Refused;
use dowiz_kernel::ledger_account;
use serde_json::{json, Value};

/// The REFUND record's id for the wallet payment taken at `at_ms`.
pub fn refund_tx_id(venue: &str, order_id: &str, at_ms: i64) -> String {
    format!("tx_{:016x}", crate::wallet::id64(&format!("{venue}:refund:{order_id}:{at_ms}")))
}

/// The ledger records that hand every wallet payment on `order` back, oldest
/// first. `ledger` is the ledger's records, oldest first. Nothing is written
/// here; the object appends what this returns after the order's log.
pub fn reversals(order: &Value, venue: &str, ledger: &[String], now_ms: i64) -> Result<Vec<Debit>, Refused> {
    let order_id = order.get("id").and_then(Value::as_str).unwrap_or("");
    let mut journal = crate::wallet::journal_from(ledger)
        .map_err(|e| Refused::Append(format!("the wallet ledger does not replay: {e}")))?;
    let mut out = Vec::new();
    for p in order.get("payments").and_then(Value::as_array).into_iter().flatten() {
        if p.get("method").and_then(Value::as_str) != Some("wallet") {
            continue;
        }
        let at = p.get("at").and_then(Value::as_i64).unwrap_or(0);
        let (spend, back) = (tx_id_of(venue, order_id, at), refund_tx_id(venue, order_id, at));
        let (spend_id, back_id) = (crate::wallet::id64(&spend), crate::wallet::id64(&back));
        if journal.iter().any(|t| t.id == back_id) || !journal.iter().any(|t| t.id == spend_id) {
            continue;
        }
        let tx = ledger_account::refund_of(&journal, spend_id, back_id).map_err(Refused::Conflict)?;
        journal = ledger_account::post(journal, tx.clone()).map_err(Refused::Conflict)?;
        let record = json!({
            "id": back,
            "kind": crate::wallet::kind_str(tx.kind),
            "reverses": spend,
            "memo": order_id,
            "at_ms": now_ms,
            "postings": tx.postings.iter().map(|p| json!({
                "account": p.account.as_str(),
                "minor": p.amount.minor,
                "currency": p.amount.currency.code(),
            })).collect::<Vec<_>>(),
        })
        .to_string();
        out.push(Debit { tx_id: back, record });
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
