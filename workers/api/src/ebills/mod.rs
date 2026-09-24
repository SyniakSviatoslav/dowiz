//! ebills.al, THE PURE HALF: the Albanian fiscal platform's sale, mapped to
//! the hub's order envelope. Bytes in, JSON out. No route, no fetch, no clock.
//! The shapes are in `wire.rs`; this file is the rules.
//!
//! WHY IT EXISTS. Dubin & Sushi rings up every dine-in course on ebills.al
//! (`docs/design/BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md`), and the owner
//! wants those orders beside the delivery orders in ONE hub with ONE stock
//! ledger. The platform has a JSON API (§1.3 there), so this is a parser and
//! a mapper, not a scraper.
//!
//! THE THREE RULES, each one a fact the blueprint measured:
//!
//! * EVERY AMOUNT ON THE WIRE IS A DOUBLE (`"totalValue": 500.0`, VAT as
//!   `83.3333333333`). Lek has no minor unit and this hub stores lek whole
//!   (`notify.rs::lek_has_no_minor_unit_and_euro_has_two`), so a value maps
//!   only if it IS whole; anything else refuses the sale and names the field.
//!   Rounding at a fiscal border is how a drawer stops reconciling.
//! * A SALE IS TWO-LEVEL. Each course at a table is a fiscalised sale of its
//!   own (`summaryInvoice:false` + `saleUnitOrder`); the bill that closes the
//!   table is a SECOND fiscalised sale whose lines repeat the courses' (§1.6:
//!   260 + 4600 = 4860, 2 + 2 = 4 lines, measured). A bill is therefore never
//!   an order here -- it is the `Paid` fact for the courses of that sitting.
//! * REFUSE, DO NOT DEFAULT. Only `CLOSED / FINISHED / draft 0 / logCis
//!   SUCCESS` was ever observed (452 of 452). A word outside that set is not
//!   "probably fine"; it is a sale the poller must show the owner unmapped.
//!
//! THE MODULE, AND WHERE EACH HALF LIVES (2026-09-23, the wiring):
//!
//! * `wire.rs`, `time.rs`, this file, `map.rs` -- the measured shapes and the
//!   rules: bytes in, the hub's envelope out. No I/O, no clock.
//! * `client.rs` + `judge.rs` -- the allow-list AS A TYPE and the ruling on
//!   every answer; `fetch.rs` -- the only network, which sends nothing the
//!   allow-list refuses.
//! * `walk.rs` -- which sale ids a firing reads; `poll.rs` -- the cron's loop.
//! * `import.rs` -- the check-and-append the venue's object executes in one
//!   turn (`hubdo/ebills.rs`, its image decisions in `glue.rs`);
//!   `state.rs` / `status.rs` -- the venue's
//!   `ebills` image and what the owner is shown; `routes.rs` -- the owner's.

pub(crate) mod cmd;
pub(crate) mod client;
pub(crate) mod fetch;
pub(crate) mod glue;
pub(crate) mod import;
pub(crate) mod judge;
#[cfg(test)]
mod live;
pub(crate) mod map;
pub(crate) mod poll;
pub(crate) mod routes;
pub(crate) mod state;
pub(crate) mod status;
mod time;
mod walk;
mod wire;

use wire::*;

/// What a sale IS, decided from two flags whose four combinations were all
/// accounted for on the wire except one (§1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// One course rung up at a table: an order.
    Course,
    /// The summary invoice that closed a table: the `Paid` fact, NOT an order.
    Bill,
    /// A sale with no table: an order, collected at the counter.
    CounterSale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MapError {
    /// A double that is not a whole number, or too large to be one exactly.
    NotWhole {
        field: &'static str,
        value: String,
    },
    Negative {
        field: &'static str,
    },
    /// Not `CLOSED`/`FINISHED`/`draft 0`: the observed set, and only it.
    NotFinished {
        status: String,
        fiscal: String,
        draft: i64,
    },
    /// No `fic`, no `logCis`, or a `logCis` row that is not `SUCCESS`.
    NotFiscalised,
    /// A `summaryInvoice` that also carries a `saleUnitOrder`: never seen,
    /// and the one combination whose meaning is unknown.
    BillWithOrder,
    /// `to_order` on a bill, or `to_paid` on anything else (§3.1).
    BillIsNotAnOrder,
    Payment(String),
    Currency(String),
    NoLines,
    /// `price × amount − discount ≠ totalValue` on one line, in whole lek.
    LineMismatch {
        line: usize,
        expected: i64,
        got: i64,
    },
    /// Σ line totals ≠ the sale's `totalValue`.
    TotalMismatch {
        expected: i64,
        got: i64,
    },
    Timestamp(String),
}

pub(crate) fn parse_list(body: &str) -> Result<SaleList, String> {
    serde_json::from_str(body).map_err(|e| format!("ebills sale list: {e}"))
}

pub(crate) fn parse_detail(body: &str) -> Result<Sale, String> {
    serde_json::from_str::<Detail>(body)
        .map(|d| d.sale)
        .map_err(|e| format!("ebills sale detail: {e}"))
}

pub(crate) fn parse_tables(body: &str) -> Result<Vec<TableState>, String> {
    serde_json::from_str(body).map_err(|e| format!("ebills tables: {e}"))
}

/// The till's menu as `(code, name, whole-lek price)`. A row with no code or
/// a price that is not whole lek is left out: it cannot be keyed or shown.
pub(crate) fn parse_items(body: &str) -> Result<Vec<(String, String, i64)>, String> {
    let rows: Vec<ItemInSale> = serde_json::from_str(body).map_err(|e| format!("ebills items: {e}"))?;
    Ok(rows
        .into_iter()
        .filter_map(|r| Some((r.item_code.filter(|c| !c.is_empty())?, r.item, lek(r.price, "price").ok()?)))
        .collect())
}

/// THE MARKER dowiz's fiscal sender writes into a created sale's `notes`.
pub(crate) const OURS: &str = "dowiz:";

/// A sale dowiz itself created (`fiscal::ebills_fire`): its order is already
/// in the log, so importing it back would be a second order and a second
/// draw on the shelf. The poller passes over it.
pub(crate) fn ours(s: &Sale) -> bool {
    s.notes.as_deref().is_some_and(|n| n.starts_with(OURS))
}

/// The till's menu rows WITH their id and the whole row as served, for the
/// fiscal create (EBILLS-WRITE-PATH §1.4.1: the SPA echoes the row). A row
/// with no code or no id cannot be named in a create and is left out.
pub(crate) fn parse_item_rows(body: &str) -> Result<Vec<(String, i64, serde_json::Value)>, String> {
    let raw: Vec<serde_json::Value> = serde_json::from_str(body).map_err(|e| format!("ebills items: {e}"))?;
    let mut out = Vec::with_capacity(raw.len());
    for v in raw {
        let row: ItemInSale = serde_json::from_value(v.clone()).map_err(|e| format!("ebills item row: {e}"))?;
        if let (Some(code), Some(id)) = (row.item_code.filter(|c| !c.is_empty()), row.id) {
            out.push((code, id, v));
        }
    }
    Ok(out)
}

pub(crate) fn classify(s: &Sale) -> Result<Kind, MapError> {
    match (s.summary_invoice, s.sale_unit_order.is_some()) {
        (true, false) => Ok(Kind::Bill),
        (true, true) => Err(MapError::BillWithOrder),
        (false, true) => Ok(Kind::Course),
        (false, false) => Ok(Kind::CounterSale),
    }
}

/// Largest double that is still an exact integer: 2^53.
const EXACT_MAX: f64 = 9_007_199_254_740_992.0;

/// A wire double as a whole number, or a refusal naming the field. `-0.0`
/// and `0.0` are both zero; `NaN` and infinities are not whole.
pub(crate) fn whole(v: f64, field: &'static str) -> Result<i64, MapError> {
    if !v.is_finite() || v.fract() != 0.0 || v.abs() >= EXACT_MAX {
        return Err(MapError::NotWhole {
            field,
            value: format!("{v}"),
        });
    }
    Ok(v as i64)
}

/// A whole amount that may not be negative -- prices and totals.
pub(crate) fn lek(v: f64, field: &'static str) -> Result<i64, MapError> {
    match whole(v, field)? {
        n if n < 0 => Err(MapError::Negative { field }),
        n => Ok(n),
    }
}

/// The platform's fifteen payment words against `storefront::PAYMENT_KINDS`.
/// Three map; the rest (bank transfer, vouchers, waivers, "MULTIPLE"...) are
/// facts this hub has no word for and must not be spelled as cash.
pub(crate) fn payment(method: &str) -> Result<&'static str, MapError> {
    match method {
        "CASH" => Ok("cash"),
        "CARD" | "CARD_ON_POS" | "POK_CARD" => Ok("card"),
        other => Err(MapError::Payment(other.to_string())),
    }
}

/// Fiscalised: `FINISHED`, not a draft, every fiscalisation row `SUCCESS`
/// with a `fic` in hand. The sale's own `status` is `CLOSED` -- or `OPENED`
/// for a COURSE whose table is still open (measured 2026-09-23: the list
/// shows those, and they turn `CLOSED` when the bill is issued). A
/// `changedStatus` other than `CANCELLED` (a void, `void_of`) is unknown.
pub(crate) fn finished(s: &Sale) -> Result<(), MapError> {
    let open_course = s.status == "OPENED" && s.sale_unit_order.is_some() && !s.summary_invoice;
    let changed_ok = matches!(s.changed_status.as_deref(), None | Some("CANCELLED"));
    if (s.status != "CLOSED" && !open_course) || s.fiscal_status != "FINISHED" || s.draft != 0 || !changed_ok {
        return Err(MapError::NotFinished {
            status: s.status.clone(),
            fiscal: s.fiscal_status.clone(),
            draft: s.draft,
        });
    }
    let log = s.log_cis.as_deref().unwrap_or(&[]);
    let sent = log.iter().all(|l| l.status.as_deref() == Some("SUCCESS"));
    if s.fic.is_some() && !log.is_empty() && sent {
        Ok(())
    } else {
        Err(MapError::NotFiscalised)
    }
}

/// `VAT_20` → `20`; an unknown spelling is `None`, never a guessed rate.
pub(crate) fn vat_pct(vat: Option<&str>) -> Option<i64> {
    vat?.strip_prefix("VAT_")?.parse().ok()
}

#[cfg(test)]
mod tests;
