//! THE TILL: a drawer opened with a float, counted blind, closed with its
//! over/short recorded and never adjusted away (BLUEPRINT-POS-THE-ROOM §2.5,
//! §4.6, §7 item 4; BLUEPRINT-OPERATIONAL-BLIND-SPOTS P3-2).
//!
//! THE TILL LOG HOLDS ONLY WHAT THE ORDER LOG CANNOT KNOW. Five events in a
//! `LogImage` named [`IMAGE_TILL`], subject = the till id:
//!
//!   `till.opened  { float: {cur: n}, by, at }`
//!   `till.pay_in  { currency, amount, reason, source?, by, at }`
//!   `till.pay_out { currency, amount, reason, by, at }`
//!   `till.counted { observed: {cur: n}, by, at }`
//!   `till.closed  { counted, expected, over_short: {cur: n}, by, at }`
//!
//! CASH SALES ARE NEVER WRITTEN HERE. They are folded from the order log's
//! `payments[]` (`fold::cash_payments`), because a till that kept its own
//! counter of sales would be the courier shift's "second place the same
//! numbers lived" again (`courier.rs`, earnings folded from the orders).
//!
//! EVERY AMOUNT IS PER CURRENCY. Guests in Durrës pay lek and euro mixed; a
//! drawer holding both is two piles, not one number, and adding a euro cent to
//! a lek is the bug. So a float, a count and an over/short are maps
//! `currency code -> minor units`, and a cash payment lands in the pile of the
//! currency it was PAID in, not the order's.
//!
//!   expected[c] = float[c] + Σ cash paid[c] in [opened, closed] + Σ pay_in[c] − Σ pay_out[c]
//!   over_short[c] = counted[c] − expected[c]
//!
//! One open till per venue at a time — a second `opened` while one is open is
//! refused, the courier shift's "not a state this layout can represent".

use super::Refused;
use dowiz_core::money::Currency;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub mod fold;
pub use fold::{cash_payments, expected, periods, report, CashIn, Period, Report};

/// The till's image. Here rather than in `hubstore.rs`: the till is this
/// module's, and `hubstore.rs` is at its file-size ratchet.
pub const IMAGE_TILL: &str = "till";

/// Minor units per currency code. A BTreeMap so every serialisation of the
/// same drawer is the same bytes.
pub type Money = BTreeMap<String, i64>;

pub const OPENED: &str = "till.opened";
pub const PAY_IN: &str = "till.pay_in";
pub const PAY_OUT: &str = "till.pay_out";
pub const COUNTED: &str = "till.counted";
pub const CLOSED: &str = "till.closed";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenIn {
    pub location_id: String,
    pub till_id: String,
    #[serde(default)]
    pub float: Money,
    pub by: String,
    pub now_ms: i64,
}

/// A BLIND count: what the counter found, per currency. The answer to it
/// carries no expected figure (`services::orders::room::till::answer`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountIn {
    pub location_id: String,
    pub till_id: String,
    pub observed: Money,
    pub by: String,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseIn {
    pub location_id: String,
    pub till_id: String,
    pub by: String,
    pub now_ms: i64,
}

/// Cash put into or taken out of the drawer that is not a sale: a courier's
/// hand-in (`source: "shift:<courier_id>"`), change fetched from the bank, a
/// supplier paid from the drawer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveIn {
    pub location_id: String,
    pub till_id: String,
    pub currency: String,
    pub amount: i64,
    pub reason: String,
    #[serde(default)]
    pub source: Option<String>,
    pub by: String,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Cmd {
    Open(OpenIn),
    PayIn(MoveIn),
    PayOut(MoveIn),
    Count(CountIn),
    Close(CloseIn),
}

impl Cmd {
    pub fn till_id(&self) -> &str {
        match self {
            Cmd::Open(i) => &i.till_id,
            Cmd::PayIn(i) | Cmd::PayOut(i) => &i.till_id,
            Cmd::Count(i) => &i.till_id,
            Cmd::Close(i) => &i.till_id,
        }
    }
}

/// What the object answers after a till command: the period as it now is.
/// The Worker decides how much of it a person may see (the blind count).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TillOut {
    pub kind: String,
    pub till_id: String,
    pub period: fold::PeriodReport,
    pub generation: i64,
}

fn signed(by: &str) -> Result<(), Refused> {
    if by.trim().is_empty() {
        return Err(Refused::Invalid("a till event names who did it".into()));
    }
    Ok(())
}

fn till_id_ok(id: &str) -> Result<(), Refused> {
    let ok = !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if !ok {
        return Err(Refused::Invalid("a till id is 1-64 letters, digits, '-' or '_'".into()));
    }
    Ok(())
}

/// A known currency code, or Invalid. `Currency::from_code` is the kernel's
/// list; a drawer pile in a currency the product cannot render is refused.
pub fn currency_ok(code: &str) -> Result<Currency, Refused> {
    Currency::from_code(code).ok_or_else(|| Refused::Invalid(format!("{code}: not a currency this venue can take")))
}

/// Every pile is a known currency and not negative (a drawer cannot hold −5).
fn money_ok(m: &Money, what: &str) -> Result<(), Refused> {
    for (c, n) in m {
        currency_ok(c)?;
        if *n < 0 {
            return Err(Refused::Invalid(format!("{what} {c} cannot be negative")));
        }
    }
    Ok(())
}

/// The open period this command must act on, or the refusal naming why not.
fn the_open<'a>(open: Option<&'a Period>, till_id: &str) -> Result<&'a Period, Refused> {
    match open {
        None => Err(Refused::Conflict("no till is open: open the till first".into())),
        Some(p) if p.till_id != till_id => Err(Refused::Conflict(format!("till {} is open, not {till_id}", p.till_id))),
        Some(p) => Ok(p),
    }
}

/// THE WHOLE DECISION, pure: the periods folded so far, the cash payments the
/// order log holds, one command. Returns the record to append as
/// `(kind, subject, json)`. Nothing is written by this function.
pub fn decide(periods: &[Period], cash: &[CashIn], cmd: &Cmd) -> Result<(&'static str, String, Value), Refused> {
    till_id_ok(cmd.till_id())?;
    let open = periods.last().filter(|p| p.closed_at.is_none());
    match cmd {
        Cmd::Open(i) => {
            signed(&i.by)?;
            money_ok(&i.float, "the float")?;
            if let Some(p) = open {
                return Err(Refused::Conflict(format!("till {} is already open: close it first", p.till_id)));
            }
            Ok((OPENED, i.till_id.clone(), json!({ "float": i.float, "by": i.by, "at": i.now_ms })))
        }
        Cmd::PayIn(i) | Cmd::PayOut(i) => {
            signed(&i.by)?;
            currency_ok(&i.currency)?;
            if i.amount < 1 {
                return Err(Refused::Invalid("a pay-in or pay-out is at least 1 minor unit".into()));
            }
            if i.reason.trim().is_empty() {
                return Err(Refused::Invalid("cash moved without a sale says why".into()));
            }
            the_open(open, &i.till_id)?;
            let kind = if matches!(cmd, Cmd::PayIn(_)) { PAY_IN } else { PAY_OUT };
            let mut rec = json!({ "currency": i.currency, "amount": i.amount, "reason": i.reason, "by": i.by, "at": i.now_ms });
            if let Some(s) = &i.source {
                rec["source"] = json!(s);
            }
            Ok((kind, i.till_id.clone(), rec))
        }
        Cmd::Count(i) => {
            signed(&i.by)?;
            money_ok(&i.observed, "a count")?;
            the_open(open, &i.till_id)?;
            Ok((COUNTED, i.till_id.clone(), json!({ "observed": i.observed, "by": i.by, "at": i.now_ms })))
        }
        Cmd::Close(i) => {
            signed(&i.by)?;
            let p = the_open(open, &i.till_id)?;
            // THE Z REPORT IS A COUNT AND A CLOSE. Closing an uncounted drawer
            // would record an over/short against nothing.
            let Some(counted) = &p.counted else {
                return Err(Refused::Conflict("count the drawer before closing it".into()));
            };
            let mut shut = p.clone();
            shut.closed_at = Some(i.now_ms);
            let exp = expected(&shut, cash)?;
            let over_short = fold::over_short(counted, &exp)?;
            Ok((
                CLOSED,
                i.till_id.clone(),
                json!({ "counted": counted, "expected": exp, "over_short": over_short, "by": i.by, "at": i.now_ms }),
            ))
        }
    }
}

#[cfg(test)]
mod tests;
