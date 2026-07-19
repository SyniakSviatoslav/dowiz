//! Cart domain — RW-07 (consolidate 2 JS cart impls → single kernel authority).
//!
//! Ports the pure cart state machine that lived in `apps/web/CartProvider.tsx`
//! + `packages/ui/use-cart.ts` + `cartReconcile.ts` into one kernel module.
//! Totals go through [`crate::money`] (integer minor units) — money red-line.
//!
//! RED→GREEN GATE: one kernel cart, both wrappers become thin over it;
//! Totals go through [`crate::money`] (integer minor units) — money red-line.

/// A cart line: product id + option key + quantity (0 = remove).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartLine {
    pub product_id: String,
    pub options: String,
    /// quantity in integer units (never fractional).
    pub qty: i64,
}

/// A priced cart line (after applying unit price in minor units).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PricedLine {
    pub line: CartLine,
    /// unit price in integer minor units.
    pub unit_price_minor: i64,
    /// line total = unit_price * qty (overflow-safe).
    pub line_total_minor: i64,
}

/// Cart state machine. Single authority for add/update/clear/dedupe/total.
#[derive(Debug, Clone, Default)]
pub struct Cart {
    lines: Vec<CartLine>,
}

impl Cart {
    pub fn new() -> Self {
        Cart::default()
    }

    /// Add `qty` to the line identified by (product_id, options); dedupe key.
    /// `qty == 0` removes the line. Negative qty is rejected (fail-closed).
    pub fn add(&mut self, product_id: &str, options: &str, qty: i64) -> Result<(), String> {
        if qty < 0 {
            return Err("cart: negative qty rejected".into());
        }
        if qty == 0 {
            self.remove(product_id, options);
            return Ok(());
        }
        match self
            .lines
            .iter_mut()
            .find(|l| l.product_id == product_id && l.options == options)
        {
            Some(existing) => existing.qty = existing.qty.saturating_add(qty),
            None => self.lines.push(CartLine {
                product_id: product_id.to_string(),
                options: options.to_string(),
                qty,
            }),
        }
        Ok(())
    }

    /// Remove a line by (product_id, options).
    pub fn remove(&mut self, product_id: &str, options: &str) {
        self.lines
            .retain(|l| !(l.product_id == product_id && l.options == options));
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Total item count (sum of qtys).
    pub fn item_count(&self) -> i64 {
        self.lines.iter().map(|l| l.qty).sum()
    }

    /// Borrow the current cart lines (read-only). Used by the P66 checkout path to
    /// snapshot a cart into an off-device [`crate::wallet::draft::CartSnapshot`].
    /// No money is moved here — only an immutable view of the current lines.
    pub fn lines(&self) -> &[CartLine] {
        &self.lines
    }

    /// Price every line at the supplied unit price lookup (product_id → minor units).
    /// Returns priced lines + the integer subtotal (overflow-safe via checked mul/add).
    pub fn price<F>(&self, unit_price: F) -> Result<(Vec<PricedLine>, i64), String>
    where
        F: Fn(&str) -> i64,
    {
        let mut total: i64 = 0;
        let mut out = Vec::with_capacity(self.lines.len());
        for l in &self.lines {
            let unit = unit_price(&l.product_id);
            let line_total = l
                .qty
                .checked_mul(unit)
                .ok_or_else(|| "cart: line total overflow".to_string())?;
            total = total
                .checked_add(line_total)
                .ok_or_else(|| "cart: subtotal overflow".to_string())?;
            out.push(PricedLine {
                line: l.clone(),
                unit_price_minor: unit,
                line_total_minor: line_total,
            });
        }
        Ok((out, total))
    }

    /// Reconcile against the authoritative menu: re-price every line at the
    /// current menu price and DROP lines whose product no longer exists
    /// (`unit_price` returns `None`). Fixes drifted carts.
    pub fn reconcile<F>(&mut self, unit_price: F)
    where
        F: Fn(&str) -> Option<i64>,
    {
        let mut kept = Vec::new();
        for l in self.lines.drain(..) {
            if let Some(_price) = unit_price(&l.product_id) {
                kept.push(l); // keep; price() re-prices at current menu rate
            }
            // else: product delisted → drop (drift removed)
        }
        self.lines = kept;
    }
}

/// Format integer minor units as a display string with currency symbol.
/// Belongs with money authority (RW-08). `minor` is e.g. cents; `decimals`=2.
pub fn format_money(minor: i64, decimals: u32, symbol: &str) -> String {
    let sign = if minor < 0 { "-" } else { "" };
    let m = minor.unsigned_abs();
    let base = 10u64.pow(decimals);
    let whole = m as u64 / base;
    let frac = m as u64 % base;
    if decimals == 0 {
        format!("{}{}{}", sign, symbol, whole)
    } else {
        format!(
            "{}{}{}.{:0width$}",
            sign,
            symbol,
            whole,
            frac,
            width = decimals as usize
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // add dedupes by (product, options); qty 0 removes.
    #[test]
    fn add_dedupes_and_removes() {
        let mut c = Cart::new();
        c.add("p1", "small", 2).unwrap();
        c.add("p1", "small", 3).unwrap(); // same key → 5
        c.add("p1", "large", 1).unwrap(); // different options → separate
        assert_eq!(c.len(), 2);
        assert_eq!(c.item_count(), 6);
        c.add("p1", "small", 0).unwrap(); // remove
        assert_eq!(c.len(), 1);
        assert_eq!(c.item_count(), 1);
    }

    // negative qty rejected (fail-closed).
    #[test]
    fn negative_qty_rejected() {
        let mut c = Cart::new();
        assert!(c.add("p1", "", -1).is_err());
    }

    // total via kernel money (integer), overflow-safe.
    #[test]
    fn total_via_integer_money() {
        let mut c = Cart::new();
        c.add("a", "", 2).unwrap(); // 2 × 500 = 1000
        c.add("b", "", 1).unwrap(); // 1 × 250 = 250
        let (lines, total) = c.price(|p| if p == "a" { 500 } else { 250 }).unwrap();
        assert_eq!(total, 1250);
        assert_eq!(lines[0].line_total_minor, 1000);
        assert_eq!(lines[1].line_total_minor, 250);
    }

    // reconcile drops delisted products and re-prices survivors.
    #[test]
    fn reconcile_drops_drifted() {
        let mut c = Cart::new();
        c.add("live", "", 2).unwrap();
        c.add("dead", "", 1).unwrap();
        // menu: "live" now 999, "dead" delisted
        c.reconcile(|p| if p == "live" { Some(999) } else { None });
        assert_eq!(c.len(), 1);
        assert_eq!(c.item_count(), 2);
        let (_lines, total) = c.price(|p| if p == "live" { 999 } else { 0 }).unwrap();
        assert_eq!(total, 1998); // re-priced at current menu rate
    }

    // formatMoney: integer minor → display string (money authority).
    #[test]
    fn format_money_display() {
        assert_eq!(format_money(1250, 2, "€"), "€12.50");
        assert_eq!(format_money(0, 2, "$"), "$0.00");
        assert_eq!(format_money(1000, 0, "kr"), "kr1000");
        assert_eq!(format_money(-750, 2, "€"), "-€7.50");
    }

    // money formatting stays integer (no float on money).
    #[test]
    fn format_money_no_float() {
        // 1 cent must never render as 0.00
        assert_eq!(format_money(1, 2, "€"), "€0.01");
    }
}
