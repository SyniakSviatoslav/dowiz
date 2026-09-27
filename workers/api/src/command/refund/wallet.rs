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

/// A completed refund's wallet reversal the ledger lacks (W-FIX O3), or why it
/// cannot be written.
#[derive(Debug, Clone, Default)]
pub struct HandBack {
    pub write: Vec<Debit>,
    /// `(order id, why)`: a reversal the journal refuses.
    pub refused: Vec<(String, String)>,
}

impl HandBack {
    /// The reversals as the screen lists them: order, amount, currency (the
    /// credited posting -- the wallet's side).
    pub fn listed(&self) -> Vec<Value> {
        self.write.iter().map(|d| {
            let r: Value = serde_json::from_str(&d.record).unwrap_or_default();
            let credit = r.get("postings").and_then(Value::as_array).into_iter().flatten()
                .find(|p| p.get("minor").and_then(Value::as_i64).unwrap_or(0) > 0).cloned().unwrap_or_default();
            json!({ "tx_id": d.tx_id, "order_id": r.get("memo"), "amount": credit.get("minor"), "currency": credit.get("currency") })
        }).collect()
    }

    /// The refusals, as the screen lists them.
    pub fn refused_listed(&self) -> Vec<Value> {
        self.refused.iter().map(|(o, why)| json!({ "order_id": o, "why": why })).collect()
    }
}

/// THE REPAIR OF A LOST HAND-BACK (W-FIX O3). `refund` appends the reversals
/// after the order's log, and a ledger write that failed was only logged: the
/// order sat at COMPENSATED_REFUND with its wallet never credited, and nothing
/// re-entered it. Every COMPENSATED_REFUND order's reversals are decided here
/// against the ledger AS IT GROWS (oldest first), by the same `reversals` the
/// refund runs -- derived ids, so a reversal already written is skipped and a
/// second repair writes nothing.
pub fn hand_back(orders: &[Value], venue: &str, ledger: &[String], now_ms: i64) -> Result<HandBack, Refused> {
    let mut rows = ledger.to_vec();
    crate::wallet::journal_from(&rows).map_err(|e| Refused::Append(format!("the wallet ledger does not replay: {e}")))?;
    let mut plan = HandBack::default();
    for o in orders.iter().filter(|o| o.get("status").and_then(Value::as_str) == Some("COMPENSATED_REFUND")) {
        match reversals(o, venue, &rows, now_ms) {
            Ok(back) => {
                rows.extend(back.iter().map(|d| d.record.clone()));
                plan.write.extend(back);
            }
            Err(r) => plan.refused.push((o.get("id").and_then(Value::as_str).unwrap_or_default().to_string(), r.message().to_string())),
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests;
