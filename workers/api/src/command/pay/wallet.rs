//! A WALLET AS A PAYMENT METHOD (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.3).
//!
//! THE GUARD DID NOT EXIST, measured before this was written (§5 asks for
//! that): `ledger_account::post` validates balance and id uniqueness but not
//! coverage, `ledger_account::spend` builds a debit of any size, and
//! `can_cover` -- the rule -- was called by nothing. "A wallet never goes
//! negative" is therefore enforced HERE, at the single writer: the venue's
//! object replays the ledger image (`wallet::journal_from`, the same reading
//! the wallet routes use) and refuses a debit the balance does not cover,
//! BEFORE the `Paid` is decided. A refusal writes nothing to either image.
//!
//! THE DEBIT IS ONE LEDGER RECORD in the shape `wallet::top_up` writes, so the
//! wallet's statement shows the spend beside the top-ups with no second
//! format. Its id is the venue, the order and the instant, so the same tap
//! replayed by the idempotency layer is the same transaction.

use super::{decide, PayIn, Room};
use crate::command::Refused;
use crate::hubdo::OrderView;
use dowiz_kernel::ledger_account::{self, Journal};
use dowiz_kernel::money::{Currency, Money};
use serde_json::{json, Value};

/// A wallet payment's ledger id: the venue, the order and the payment's
/// instant (`payments[].at`). DERIVED, so a leg can be looked for -- and a lost
/// one rewritten -- from the `Paid` alone (`pay::legs`), and rewriting it twice
/// is refused by the journal as a repeated id.
pub fn tx_id_of(venue: &str, order_id: &str, at_ms: i64) -> String {
    format!("tx_{:016x}", crate::wallet::id64(&format!("{venue}:pay:{order_id}:{at_ms}")))
}

/// The ledger record to append, and its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Debit {
    pub tx_id: String,
    pub record: String,
}

/// The wallet's debit for this payment, or the refusal. `currency` is the
/// one handed over; a wallet held in another currency cannot cover it.
pub fn debit(journal: &Journal, input: &PayIn, currency: &str) -> Result<Debit, Refused> {
    let user = input
        .wallet
        .as_deref()
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .ok_or_else(|| Refused::Invalid("a wallet payment names its wallet".into()))?;
    let cur = Currency::from_code(currency).ok_or_else(|| Refused::Invalid(format!("unknown currency {currency}")))?;
    let amount = Money::new(input.amount, cur);
    let wallet = crate::wallet::id64(user);
    if !ledger_account::can_cover(journal, wallet, amount).map_err(Refused::Append)? {
        return Err(Refused::Conflict(format!(
            "the wallet does not hold {} {currency}: a wallet never goes negative",
            input.amount
        )));
    }
    let tx_id = tx_id_of(&input.location_id, &input.order_id, input.now_ms);
    let tx = ledger_account::spend(
        crate::wallet::id64(&tx_id),
        wallet,
        crate::wallet::id64(&input.location_id),
        amount,
        None,
        &input.order_id,
    )
    .map_err(Refused::Invalid)?;
    // The journal's own rules (balanced, id not already used) before a byte.
    ledger_account::post(journal.clone(), tx.clone()).map_err(Refused::Conflict)?;
    let record = json!({
        "id": tx_id,
        "kind": crate::wallet::kind_str(tx.kind),
        "reverses": Value::Null,
        "memo": input.order_id,
        "at_ms": input.now_ms,
        "postings": tx.postings.iter().map(|p| json!({
            "account": p.account.as_str(),
            "minor": p.amount.minor,
            "currency": p.amount.currency.code(),
        })).collect::<Vec<_>>(),
    })
    .to_string();
    Ok(Debit { tx_id, record })
}

/// THE PAYMENT, wallet-aware: for a wallet the debit is decided FIRST, so a
/// short wallet is refused with no `Paid` appended even in memory; then the
/// payment itself (`pay::decide`). `ledger` is the ledger's records, oldest
/// first; it is read only for a wallet.
pub fn pay(
    hub: &mut dowiz_hub::Hub,
    current: Option<&OrderView>,
    input: &PayIn,
    room: &Room,
    ledger: &[String],
) -> Result<(Value, String, u64, Option<Debit>), Refused> {
    let debit = match current.filter(|_| input.method == "wallet") {
        Some(c) => {
            let order: Value = serde_json::from_str(&c.order_json).unwrap_or(Value::Null);
            let order_cur = order.get("currency").and_then(Value::as_str).unwrap_or(room.venue_currency);
            let cur = input.currency.as_deref().unwrap_or(order_cur).to_string();
            let journal = crate::wallet::journal_from(ledger)
                .map_err(|e| Refused::Append(format!("the wallet ledger does not replay: {e}")))?;
            Some(debit(&journal, input, &cur)?)
        }
        None => None,
    };
    let (order, body, seq) = decide(hub, current, input, room)?;
    Ok((order, body, seq, debit))
}

#[cfg(test)]
mod tests;
