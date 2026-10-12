//! PURE. THE ADJUSTMENTS OF A SALE AS LINES (D34, R-GAPS C12): a comp, the
//! promo cut and the tip, each written the way the till itself writes it, so
//! the records sum to the order's total EXACTLY.
//!
//! WHAT THE WIRE OFFERS (EBILLS-WRITE-PATH §1.4.1, the SPA's own builder): a
//! record's `discount` is a PERCENT and its `price` is the unit price AFTER
//! it, VAT-inclusive; `totalValue = amount x price`. There is no negative line
//! and no lek-amount discount on a sale (`fees`, `vouchers` exist and their
//! shapes were never observed: UNVERIFIED, not used).
//!
//! A COMP is the till's free dish, measured live (BLUEPRINT-EBILLS §8.2 (3)):
//! `price 0, discount 100, totalValue 0`, its quantity kept.
//!
//! THE PROMO CUT is what is left of `discount` after the comps. It is split
//! over the VAT rates by THE LAW, `dowiz_core::tax::summarise` (largest
//! remainder, ties to the lower rate) -- called, not copied -- so each rate's
//! records sum to the base the stamped `tax` block taxed. Inside a rate it is
//! split over the lines by the same rule, ties to the earlier line. A line's
//! net `n` over its quantity `q` is then written as at most TWO records of
//! whole-lek unit prices: `n mod q` units at `n div q + 1`, the rest at
//! `n div q`. Nothing is rounded, so nothing can drift.
//!
//! THE TIP is one record of the till item the owner chose
//! (`fiscal.ebills.tip_item`) at VAT_0: a gratuity is outside the taxable base
//! (`dowiz_core::tax::TaxInput::tip`). An item the till taxes is refused by
//! name rather than sent at a rate the till would not use. Whether Albanian law
//! wants a tip on the invoice at all is the operator's (OD-4): UNVERIFIED here.

use super::{row_for, vat_of, BodyRefusal, Till};
use crate::fiscal::document::Document;
use dowiz_core::tax::{summarise, RatePpm, TaxInput, TaxLine};
use serde_json::Value;

/// One record to be: `qty` units at `price`, listed at `list` (the discount
/// percent is `(list - price) / list`).
#[derive(Debug)]
pub(super) struct Part<'a> {
    pub row: &'a Value,
    pub name: &'a str,
    pub qty: i64,
    pub price: i64,
    pub list: i64,
    pub ppm: i64,
}

/// `cut` over `weights`, largest remainder, ties to the earlier index.
/// Σ out == cut exactly; `cut <= Σ weights` is the caller's.
fn spread(cut: i64, weights: &[i64]) -> Vec<i64> {
    let total: i128 = weights.iter().map(|w| i128::from(*w)).sum();
    if cut == 0 || total == 0 {
        return vec![0; weights.len()];
    }
    let c = i128::from(cut);
    let mut out: Vec<i64> = weights.iter().map(|w| (c * i128::from(*w) / total) as i64).collect();
    let mut rems: Vec<(i128, usize)> = weights.iter().enumerate().map(|(i, w)| (c * i128::from(*w) % total, i)).collect();
    rems.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let left = cut - out.iter().sum::<i64>();
    for r in rems.iter().take(left as usize) {
        out[r.1] += 1;
    }
    out
}

/// The records the lines become after the comps and the promo cut.
pub(super) fn parts<'a>(doc: &'a Document, till: &'a Till) -> Result<Vec<Part<'a>>, BodyRefusal> {
    let mut rows = Vec::with_capacity(doc.lines.len());
    for l in &doc.lines {
        rows.push(row_for(till, &l.name, l.product_id.as_deref())?);
        vat_of(l.rate_ppm).ok_or_else(|| BodyRefusal::Rate { line: l.name.clone(), ppm: l.rate_ppm })?;
    }
    let comps: i64 = doc.lines.iter().filter(|l| l.comped).map(|l| l.gross).sum();
    let paid: i64 = doc.lines.iter().filter(|l| !l.comped).map(|l| l.gross).sum();
    if doc.discount < 0 || comps > doc.discount {
        return Err(BodyRefusal::Discount(format!("the comped lines come to {comps}, more than the order's discount of {}", doc.discount)));
    }
    let cut = doc.discount - comps;
    if cut > paid {
        return Err(BodyRefusal::Discount(format!("a cut of {cut} is more than the paid lines' {paid}")));
    }
    // Per rate, by the law; then per line inside the rate.
    let mut share = vec![0i64; doc.lines.len()];
    if cut > 0 {
        let taxed: Vec<TaxLine> = doc.lines.iter().filter(|l| !l.comped).map(|l| TaxLine { amount: l.gross, rate: RatePpm(l.rate_ppm as u32) }).collect();
        let law = summarise(&TaxInput { lines: &taxed, discount: cut, inclusive: true, fee: None, tip: 0 }).map_err(BodyRefusal::Discount)?;
        for (g, got) in law.groups.iter().zip(&law.discount_allocated) {
            let at: Vec<usize> = (0..doc.lines.len()).filter(|&i| !doc.lines[i].comped && doc.lines[i].rate_ppm == i64::from(g.rate_ppm)).collect();
            let weights: Vec<i64> = at.iter().map(|&i| doc.lines[i].gross).collect();
            for (k, d) in spread(*got, &weights).into_iter().enumerate() {
                share[at[k]] = d;
            }
        }
    }
    let mut out = Vec::with_capacity(doc.lines.len() + 1);
    for ((l, row), d) in doc.lines.iter().zip(rows).zip(share) {
        let part = |qty, price| Part { row, name: &l.name, qty, price, list: l.unit_as_priced, ppm: l.rate_ppm };
        if l.comped {
            out.push(part(l.qty, 0));
            continue;
        }
        let net = l.gross - d;
        let (unit, odd) = (net / l.qty, net % l.qty);
        if odd > 0 {
            out.push(part(odd, unit + 1));
        }
        if odd < l.qty {
            out.push(part(l.qty - odd, unit));
        }
    }
    Ok(out)
}

/// The tip's record: its till item, at VAT_0, or why not.
pub(super) fn tip<'a>(doc: &Document, till: &'a Till) -> Result<Option<Part<'a>>, BodyRefusal> {
    if doc.tip == 0 {
        return Ok(None);
    }
    if doc.tip < 0 {
        return Err(BodyRefusal::Tip(format!("a negative tip of {}", doc.tip)));
    }
    let code = till.tip_code.as_deref().ok_or_else(|| BodyRefusal::Tip("the tip has no eBills item: choose one in the fiscal pane".into()))?;
    let row = till.items.iter().find(|(c, _, _)| c == code).map(|(_, _, r)| r);
    let row = row.ok_or_else(|| BodyRefusal::Tip(format!("the tip's eBills item {code} is not on the till's menu")))?;
    match row.get("vat").and_then(Value::as_str) {
        Some("VAT_0") => Ok(Some(Part { row, name: "tip", qty: 1, price: doc.tip, list: doc.tip, ppm: 0 })),
        other => Err(BodyRefusal::Tip(format!(
            "the tip's eBills item {code} is {} on the till; a tip is outside VAT: choose a VAT_0 item",
            other.unwrap_or("of no stated VAT")
        ))),
    }
}

#[cfg(test)]
mod tests;
