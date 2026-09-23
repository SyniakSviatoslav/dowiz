//! THE TILL'S FOLD: periods from the till log, cash from the order log, and
//! the one equation across the two (`expected`). Pure, no clock, no I/O —
//! the object runs it for every till command and for `/api/owner/health`'s
//! `till` block, and conservation law 10 re-derives it from the parts.

use super::{Money, Refused, CLOSED, COUNTED, OPENED, PAY_IN, PAY_OUT};
use dowiz_hub::logimage::Entry;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One drawer period: from `opened` to `closed` (or still open).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Period {
    pub till_id: String,
    pub opened_at: i64,
    pub opened_by: String,
    pub float: Money,
    pub pay_in: Money,
    pub pay_out: Money,
    /// The LAST count of this period. An earlier count is an X report; the
    /// one before the close is the Z report's.
    pub counted: Option<Money>,
    pub counted_at: Option<i64>,
    pub closed_at: Option<i64>,
    pub closed_by: Option<String>,
    /// What the close RECORDED. Law 10 checks it against the parts as they
    /// are now, so a cash payment that appears or vanishes after the close
    /// is a breach rather than a silently different number.
    pub over_short: Option<Money>,
}

/// One cash payment, as the order log holds it, in the currency it was PAID in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashIn {
    pub order_id: String,
    pub at: i64,
    pub currency: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeriodReport {
    #[serde(flatten)]
    pub period: Period,
    pub cash_paid: Money,
    pub expected: Money,
}

/// The `health.till` block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub open: bool,
    pub periods: Vec<PeriodReport>,
    /// Cash taken while no till was open. Refused by `pay` since the till
    /// existed; any here got in before that, and the audit names them.
    pub outside: Vec<CashIn>,
}

fn money(v: &Value, key: &str) -> Result<Money, String> {
    serde_json::from_value(v.get(key).cloned().unwrap_or(Value::Null)).map_err(|e| format!("{key}: {e}"))
}

fn add(m: &mut Money, currency: &str, n: i64) -> Result<(), String> {
    let slot = m.entry(currency.to_string()).or_insert(0);
    *slot = slot.checked_add(n).ok_or_else(|| format!("{currency} overflows"))?;
    Ok(())
}

/// Every period, oldest first. LOUD: an unknown kind, an event for a till
/// that is not the open one, or an unreadable record is an error naming it,
/// never a skipped line — a skipped pay-out is a drawer that reads short.
pub fn periods(entries: &[Entry]) -> Result<Vec<Period>, String> {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by_key(|e| e.seq);
    let mut out: Vec<Period> = Vec::new();
    for e in sorted {
        let v: Value = serde_json::from_str(&e.json).map_err(|x| format!("till record {}: {x}", e.seq))?;
        let at = v.get("at").and_then(Value::as_i64).ok_or_else(|| format!("till record {} has no time", e.seq))?;
        let by = v.get("by").and_then(Value::as_str).unwrap_or("").to_string();
        if e.kind == OPENED {
            if let Some(p) = out.last().filter(|p| p.closed_at.is_none()) {
                return Err(format!("till record {}: {} opened while {} was open", e.seq, e.subject, p.till_id));
            }
            out.push(Period {
                till_id: e.subject.clone(),
                opened_at: at,
                opened_by: by,
                float: money(&v, "float")?,
                pay_in: Money::new(),
                pay_out: Money::new(),
                counted: None,
                counted_at: None,
                closed_at: None,
                closed_by: None,
                over_short: None,
            });
            continue;
        }
        let p = out
            .last_mut()
            .filter(|p| p.closed_at.is_none() && p.till_id == e.subject)
            .ok_or_else(|| format!("till record {}: {} for {} with no such till open", e.seq, e.kind, e.subject))?;
        match e.kind.as_str() {
            k if k == PAY_IN || k == PAY_OUT => {
                let cur = v.get("currency").and_then(Value::as_str).ok_or("pay record without a currency")?;
                let n = v.get("amount").and_then(Value::as_i64).ok_or("pay record without an amount")?;
                add(if k == PAY_IN { &mut p.pay_in } else { &mut p.pay_out }, cur, n)?;
            }
            k if k == COUNTED => {
                p.counted = Some(money(&v, "observed")?);
                p.counted_at = Some(at);
            }
            k if k == CLOSED => {
                p.closed_at = Some(at);
                p.closed_by = Some(by);
                p.over_short = Some(money(&v, "over_short")?);
            }
            other => return Err(format!("till record {}: unknown kind {other}", e.seq)),
        }
    }
    Ok(out)
}

/// Every cash payment on this venue's orders. A payment with no `currency`
/// was taken before payments carried one, in the venue's currency.
pub fn cash_payments(orders: &[Value], location_id: &str, venue_currency: &str) -> Vec<CashIn> {
    let mut out = Vec::new();
    for o in orders {
        if o.get("location_id").and_then(Value::as_str).is_some_and(|l| l != location_id) {
            continue;
        }
        let id = o.get("id").and_then(Value::as_str).unwrap_or("");
        for p in o.get("payments").and_then(Value::as_array).into_iter().flatten() {
            if p.get("method").and_then(Value::as_str) != Some("cash") {
                continue;
            }
            out.push(CashIn {
                order_id: id.to_string(),
                at: p.get("at").and_then(Value::as_i64).unwrap_or(0),
                currency: p.get("currency").and_then(Value::as_str).unwrap_or(venue_currency).to_string(),
                amount: p.get("amount").and_then(Value::as_i64).unwrap_or(0),
            });
        }
    }
    out.sort_by_key(|c| c.at);
    out
}

/// Inside the period, both ends included: one open till at a time makes the
/// window alone unambiguous.
pub fn within(p: &Period, at: i64) -> bool {
    at >= p.opened_at && p.closed_at.is_none_or(|c| at <= c)
}

/// Σ cash paid into this period, per currency paid in.
pub fn cash_paid(p: &Period, cash: &[CashIn]) -> Result<Money, Refused> {
    let mut m = Money::new();
    for c in cash.iter().filter(|c| within(p, c.at)) {
        add(&mut m, &c.currency, c.amount).map_err(Refused::Invalid)?;
    }
    Ok(m)
}

/// expected[c] = float[c] + cash_paid[c] + pay_in[c] − pay_out[c].
pub fn expected(p: &Period, cash: &[CashIn]) -> Result<Money, Refused> {
    let mut m = p.float.clone();
    let paid = cash_paid(p, cash)?;
    for (c, n) in paid.iter().chain(p.pay_in.iter()) {
        add(&mut m, c, *n).map_err(Refused::Invalid)?;
    }
    for (c, n) in &p.pay_out {
        add(&mut m, c, n.checked_neg().ok_or_else(|| Refused::Invalid("pay-out overflows".into()))?)
            .map_err(Refused::Invalid)?;
    }
    Ok(m)
}

/// over_short[c] = counted[c] − expected[c], over every currency either names.
/// A pile expected and not counted is short by all of it.
pub fn over_short(counted: &Money, exp: &Money) -> Result<Money, Refused> {
    let mut m = Money::new();
    for c in counted.keys().chain(exp.keys()) {
        let d = counted.get(c).copied().unwrap_or(0).checked_sub(exp.get(c).copied().unwrap_or(0));
        m.insert(c.clone(), d.ok_or_else(|| Refused::Invalid(format!("{c} over/short overflows")))?);
    }
    Ok(m)
}

/// The health block: every period with its re-derived parts, and the cash
/// that fell outside all of them.
pub fn report(periods: &[Period], cash: &[CashIn]) -> Result<Report, Refused> {
    let mut rows = Vec::with_capacity(periods.len());
    for p in periods {
        rows.push(PeriodReport { period: p.clone(), cash_paid: cash_paid(p, cash)?, expected: expected(p, cash)? });
    }
    let outside = cash.iter().filter(|c| !periods.iter().any(|p| within(p, c.at))).cloned().collect();
    Ok(Report { open: periods.last().is_some_and(|p| p.closed_at.is_none()), periods: rows, outside })
}
