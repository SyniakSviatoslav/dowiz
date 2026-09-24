//! ONE FISCAL DOCUMENT PER SITTING: its payment means (BLUEPRINT-OPERATIONAL-
//! BLIND-SPOTS §2.3, P1-3).
//!
//! The law lists the means of payment per invoice ("mënyrën e pagesës") and
//! ebills has `MULTIPLE` for a mixed one. A table that paid cash 2000 + card
//! 1500 + wallet 500 is ONE document at close listing three means -- which is
//! why the tender (method, tip, wallet) is written on `Paid` and not on the
//! till: this fold reads it back from the rounds.
//!
//! WHAT IS HERE AND WHAT IS NOT. The `Document` itself (TAX §3.7) and its
//! push to the fiscal platform are B8's (the fiscal outbox of §2.8) and are not
//! built; this is the part that does not depend on them: WHEN a sitting closes
//! (Σ paid == bill) and WHAT means its document lists, summing to the bill.

use super::super::sitting::{bill, paid, Round};
use super::settles;
use serde::Serialize;
use serde_json::Value;

/// One means of payment on the document, in the order's currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Means {
    pub method: String,
    pub amount: i64,
}

/// The means a closed sitting's document lists, first use first; `None`
/// while anything is still owed (or nothing was billed). Σ amounts == bill.
pub fn document_means(rounds: &[Round<'_>]) -> Option<Vec<Means>> {
    let b = bill(rounds);
    if b <= 0 || paid(rounds) != b {
        return None;
    }
    let mut out: Vec<Means> = Vec::new();
    for r in rounds.iter().filter(|r| r.billed()) {
        for p in r.order.get("payments").and_then(Value::as_array).into_iter().flatten() {
            let method = p.get("method").and_then(Value::as_str).unwrap_or("other").to_string();
            match out.iter_mut().find(|m| m.method == method) {
                Some(m) => m.amount += settles(p),
                None => out.push(Means { method, amount: settles(p) }),
            }
        }
    }
    Some(out)
}
