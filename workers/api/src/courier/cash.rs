//! THE CASH A COURIER HANDS OVER, at `deliver` (W-FIX H1, 2026-09-27).
//!
//! THE DEFECT (W-AUDIT O5). `deliver` read its body with
//! `unwrap_or(In { cash_collected: None })` and then `unwrap_or(cash_due)`: a
//! body the handler could not read -- `"cash_collected":"0"` as a string,
//! `1200.0`, a truncated upload -- became "the courier collected everything",
//! and the shortfall the courier was reporting was erased without a word.
//! Nothing bounded the number from above either, so `99999999` on a 1200
//! order was stored as cash in the courier's hand, and the shift's running
//! total was a plain `+` that a large enough claim wraps.
//!
//! THE RULE, three pure functions the handler is made of:
//!   * `said`     -- an ABSENT body is "no number given" (the default is the
//!     full amount, as before); an UNREADABLE one is a 400, never a default.
//!   * `handover` -- collected is inside `0..=cash_due`; the short is the rest.
//!   * `add`      -- the shift's totals move by `checked_add`, and an
//!     overflow is a refusal, not a wrap.

use serde::Deserialize;

/// What the courier said they collected: `Ok(None)` when they said nothing.
pub(crate) fn said(raw: &str) -> Result<Option<i64>, String> {
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        cash_collected: Option<i64>,
    }
    if raw.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str::<In>(raw)
        .map(|b| b.cash_collected)
        .map_err(|e| format!("bad request body: {e}"))
}

/// The cash in the courier's hand and what they came back without.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Handover {
    pub collected: i64,
    pub short: i64,
}

/// `said` against what the order owed. Nothing given means the full amount.
pub(crate) fn handover(cash_due: i64, said: Option<i64>) -> Result<Handover, &'static str> {
    let collected = said.unwrap_or(cash_due);
    if collected < 0 {
        return Err("cash cannot be negative");
    }
    if collected > cash_due {
        return Err("that is more cash than this delivery owes");
    }
    Ok(Handover { collected, short: cash_due - collected })
}

/// A shift total moved by one delivery, or `None` where it would overflow.
pub(crate) fn add(total: i64, by: i64) -> Option<i64> {
    total.checked_add(by)
}

#[cfg(test)]
mod tests;
