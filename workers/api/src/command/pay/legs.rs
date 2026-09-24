//! EVERY WALLET `Paid` HAS ITS LEDGER LEG, AND EVERY LEG ITS `Paid`
//! (conservation law 12; the OPEN left by `pay::wallet`).
//!
//! A wallet payment is two writes in one object turn: the `Paid` on the order
//! log, then the debit on the `ledger` image (log first, `write_both`'s
//! order). A lost second write leaves a guest who ate on a wallet that was
//! never charged. Because the leg's id is DERIVED from the `Paid` alone
//! (`wallet::tx_id_of(venue, order, payments[].at)`), the fold below can say
//! exactly which leg is missing, and the repair can rewrite it with the id it
//! would have had: rewriting it twice is the journal's own "id already used".
//!
//! THE REPAIR NEVER TAKES A WALLET NEGATIVE. The rule `pay::wallet::debit`
//! enforces at payment time is enforced again here: a leg the balance no
//! longer covers is REPORTED, not written -- the guest spent the money
//! elsewhere in between, and that is a conversation, not a posting.
//!
//! PURE: orders and ledger records in, a report and the records to write out.

use super::wallet::{debit, tx_id_of, Debit};
use super::PayIn;
use crate::command::Refused;
use serde::Serialize;
use serde_json::Value;

/// One wallet payment's expected leg.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Leg {
    pub order_id: String,
    pub tx_id: String,
    pub wallet: String,
    pub amount: i64,
    pub currency: String,
    pub at: i64,
}

/// What the ledger and the order log say about each other.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Audit {
    /// A wallet `Paid` whose leg is not in the ledger.
    pub missing: Vec<Leg>,
    /// A leg whose wallet, amount or currency is not the `Paid`'s.
    pub mismatched: Vec<String>,
    /// A spend in the ledger naming no wallet `Paid`.
    pub orphans: Vec<String>,
}

impl Audit {
    pub fn holds(&self) -> bool {
        self.missing.is_empty() && self.mismatched.is_empty() && self.orphans.is_empty()
    }
}

/// Every wallet payment in the orders, as the leg it must have.
pub fn expected(orders: &[Value], venue: &str, venue_currency: &str) -> Vec<Leg> {
    let mut out = Vec::new();
    for o in orders {
        let id = o.get("id").and_then(Value::as_str).unwrap_or_default();
        let cur = o.get("currency").and_then(Value::as_str).unwrap_or(venue_currency);
        for p in o.get("payments").and_then(Value::as_array).into_iter().flatten() {
            if p.get("method").and_then(Value::as_str) != Some("wallet") {
                continue;
            }
            let at = p.get("at").and_then(Value::as_i64).unwrap_or(0);
            out.push(Leg {
                order_id: id.to_string(),
                tx_id: tx_id_of(venue, id, at),
                wallet: p.get("wallet").and_then(Value::as_str).unwrap_or_default().to_string(),
                amount: p.get("amount").and_then(Value::as_i64).unwrap_or(0),
                currency: p.get("currency").and_then(Value::as_str).unwrap_or(cur).to_string(),
                at,
            });
        }
    }
    out
}

/// Does this ledger record debit the leg's wallet by the leg's amount?
fn same_leg(rec: &Value, leg: &Leg) -> bool {
    let account = dowiz_kernel::ledger_account::Account::Wallet(crate::wallet::id64(&leg.wallet)).as_str();
    rec.get("postings").and_then(Value::as_array).into_iter().flatten().any(|p| {
        p.get("account").and_then(Value::as_str) == Some(account.as_str())
            && p.get("minor").and_then(Value::as_i64) == Some(-leg.amount)
            && p.get("currency").and_then(Value::as_str) == Some(leg.currency.as_str())
    })
}

/// A ledger spend, in the spelling `wallet::kind_str` writes.
pub fn is_spend(rec: &Value) -> bool {
    rec.get("kind").and_then(Value::as_str) == Some(crate::wallet::kind_str(dowiz_kernel::ledger_account::TxKind::Spend))
}

/// Law 12, over one venue. `ledger` is the ledger's records (any order).
pub fn audit(orders: &[Value], ledger: &[String], venue: &str, venue_currency: &str) -> Audit {
    let recs: Vec<Value> = ledger.iter().filter_map(|j| serde_json::from_str(j).ok()).collect();
    let spends: Vec<&Value> = recs.iter().filter(|r| is_spend(r)).collect();
    let legs = expected(orders, venue, venue_currency);
    let mut a = Audit::default();
    for leg in &legs {
        let found: Vec<&&Value> = spends.iter().filter(|r| r.get("id").and_then(Value::as_str) == Some(leg.tx_id.as_str())).collect();
        match found.as_slice() {
            [] => a.missing.push(leg.clone()),
            [one] if same_leg(one, leg) => {}
            _ => a.mismatched.push(format!("{}: order {} paid {} {} from wallet {:?}", leg.tx_id, leg.order_id, leg.amount, leg.currency, leg.wallet)),
        }
    }
    for r in &spends {
        let id = r.get("id").and_then(Value::as_str).unwrap_or_default();
        if !legs.iter().any(|l| l.tx_id == id) {
            a.orphans.push(format!("{id}: memo {:?}", r.get("memo").and_then(Value::as_str).unwrap_or_default()));
        }
    }
    a
}

/// What a repair would write, and what it refuses to.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub write: Vec<Debit>,
    /// A missing leg the wallet no longer covers, with why.
    pub refused: Vec<(Leg, String)>,
}

/// The missing legs, rebuilt with their derived ids against the ledger AS IT
/// GROWS, so two missing legs on one wallet are checked against each other.
/// `ledger` is the ledger's records OLDEST FIRST.
pub fn repair(orders: &[Value], ledger: &[String], venue: &str, venue_currency: &str) -> Result<Plan, Refused> {
    let mut rows = ledger.to_vec();
    let mut plan = Plan::default();
    for leg in audit(orders, ledger, venue, venue_currency).missing {
        let journal = crate::wallet::journal_from(&rows)
            .map_err(|e| Refused::Append(format!("the wallet ledger does not replay: {e}")))?;
        let input = PayIn {
            order_id: leg.order_id.clone(), location_id: venue.to_string(), amount: leg.amount,
            method: "wallet".into(), by: "repair".into(), till_id: None, covers: None, currency: None,
            rate_ppm: None, tip: None, wallet: Some(leg.wallet.clone()), now_ms: leg.at,
        };
        match debit(&journal, &input, &leg.currency) {
            Ok(d) => {
                rows.push(d.record.clone());
                plan.write.push(d);
            }
            Err(r) => plan.refused.push((leg, r.message().to_string())),
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests;
