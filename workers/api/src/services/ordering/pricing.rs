//! PURE. What a basket costs, and every reason it cannot be priced at all.
//!
//! THERE WERE TWO PRICERS. `storefront::place` re-derived every price from the
//! catalogue and refused six different ways; `extra::promo_check` walked the
//! same basket with `unwrap_or(0)` and refused exactly one. So the two answers
//! a customer sees about one basket — "your code takes 300 off" and "your order
//! is placed" — were computed by different code, and the preview quoted
//! baskets the checkout would not accept:
//!
//! * a dish the kitchen had turned off was priced and discounted;
//! * a dish with no price in the catalogue counted as FREE, which lowers the
//!   subtotal a promo's minimum is measured against;
//! * an option selection the checkout REFUSES (an id from another dish, a
//!   required group left empty, a sold-out extra) was priced at the dish's
//!   base price, so the quote silently dropped a paid option;
//! * a quantity of 0 or 500 was clamped into range and quoted, where the
//!   checkout refuses it.
//!
//! One pricer now, and the preview refuses precisely what the checkout refuses.
//!
//! NO I/O. The catalogue arrives as a lookup closure, which is what lets every
//! rule below be tested without a Durable Object.

use serde_json::Value;

/// One priced line, with everything the order, the ticket and the bell need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub product_id: String,
    /// THE DISH'S NAME TRAVELS WITH THE LINE. The catalogue is not versioned
    /// per order, so resolving it later renames a dish on orders placed before
    /// the rename and loses it on one that has been deleted.
    pub name: String,
    pub modifier_ids: Vec<String>,
    pub quantity: i64,
    /// Base price plus the chosen options' delta, floored at zero: a negative
    /// option delta must never make a line pay the customer.
    pub unit_price: i64,
    /// The dish's OWN tax rate, when its record carries `vat_ppm`. `None` is
    /// "use the venue's default", resolved where the settings are
    /// (`tax_cfg::rate_for`, inside the object) — never a zero.
    pub vat_ppm: Option<dowiz_core::tax::RatePpm>,
    /// Where the line is made (`bell_route`): the product's `station`, the
    /// kitchen when it names none.
    pub station: crate::bell_route::Station,
}

/// A whole basket, priced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Basket {
    pub lines: Vec<Line>,
    pub subtotal: i64,
}

/// Why a basket could not be priced, in the words the customer is given.
///
/// THE STATUS IS PART OF THE ANSWER. A 400 says the browser sent something
/// wrong; a 409 says the browser was right when it asked and the kitchen has
/// changed its mind since. The cart shows those two differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Quantity,
    Unknown(String),
    Unreadable { product_id: String, why: String },
    Unavailable(String),
    NoPrice(String),
    Options { product_id: String, why: String },
}

impl Refusal {
    pub fn text(&self) -> String {
        match self {
            Refusal::Quantity => "invalid quantity".into(),
            Refusal::Unknown(id) => format!("unknown product: {id}"),
            Refusal::Unreadable { why, .. } => format!("catalogue product unreadable: {why}"),
            Refusal::Unavailable(id) => format!("unavailable: {id}"),
            Refusal::NoPrice(id) => format!("product has no price: {id}"),
            Refusal::Options { product_id, why } => format!("{product_id}: {why}"),
        }
    }

    /// 500 for `Unreadable`: a catalogue this venue wrote and cannot read back
    /// is the platform's fault, and telling the customer to fix their basket
    /// would be a lie.
    pub fn status(&self) -> u16 {
        match self {
            Refusal::Quantity | Refusal::Unknown(_) | Refusal::Options { .. } => 400,
            Refusal::Unavailable(_) | Refusal::NoPrice(_) => 409,
            Refusal::Unreadable { .. } => 500,
        }
    }
}

/// What one line of the request says. The browser's own `unit_price`, if it
/// sent one, is not in this struct AT ALL: it is discarded before it can be
/// believed.
pub struct Want<'a> {
    pub product_id: &'a str,
    pub modifier_ids: &'a [String],
    pub quantity: i64,
}

/// Price a basket against the catalogue, or refuse it.
///
/// `lookup` is the catalogue: product id → its stored JSON record. FAILING
/// CLOSED is the rule throughout — an unknown dish, an unreadable record, a
/// missing price and a disallowed option are all refusals, never a zero.
pub fn price_basket<'a>(
    lookup: impl Fn(&str) -> Option<String>,
    want: impl IntoIterator<Item = Want<'a>>,
) -> Result<Basket, Refusal> {
    let mut lines = Vec::new();
    let mut subtotal = 0i64;
    for w in want {
        // FIRST, because a quantity out of range makes every number after it
        // meaningless and there is nothing to look up on the customer's behalf.
        if w.quantity < 1 || w.quantity > 99 {
            return Err(Refusal::Quantity);
        }
        let Some(pj) = lookup(w.product_id) else {
            return Err(Refusal::Unknown(w.product_id.to_string()));
        };
        let p: Value = serde_json::from_str(&pj).map_err(|e| Refusal::Unreadable {
            product_id: w.product_id.to_string(),
            why: e.to_string(),
        })?;
        // A DISH WITH NO `available` FIELD IS NOT AVAILABLE. The catalogue
        // writes it on every product; a record without it is one nobody has
        // finished, and serving it is worse than refusing it.
        if !p.get("available").and_then(Value::as_bool).unwrap_or(false) {
            return Err(Refusal::Unavailable(w.product_id.to_string()));
        }
        let Some(price) = p.get("price").and_then(Value::as_i64).filter(|x| *x >= 0) else {
            return Err(Refusal::NoPrice(w.product_id.to_string()));
        };
        // THE OPTIONS ARE PART OF THE PRICE, and `price` is also the only
        // thing that VALIDATES a choice: an id belonging to no group on this
        // dish, or a required group left empty, was once accepted silently.
        let groups = dowiz_hub::modifiers::groups_of(&pj);
        let chosen = dowiz_hub::modifiers::price(&groups, w.modifier_ids).map_err(|e| {
            Refusal::Options { product_id: w.product_id.to_string(), why: e.to_string() }
        })?;
        let vat_ppm = own_rate(&p).map_err(|why| Refusal::Unreadable {
            product_id: w.product_id.to_string(),
            why,
        })?;
        let station = crate::bell_route::Station::of_line(&p);
        let unit_price = (price + chosen.delta).max(0);
        subtotal += unit_price * w.quantity;
        lines.push(Line {
            name: p
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(w.product_id)
                .to_string(),
            product_id: w.product_id.to_string(),
            modifier_ids: w.modifier_ids.to_vec(),
            quantity: w.quantity,
            unit_price,
            vat_ppm,
            station,
        });
    }
    Ok(Basket { lines, subtotal })
}

/// The record's own `vat_ppm`: absent is `None`; anything but an integer in
/// `0..=1000000` is REFUSED, because a rate the catalogue wrote and this cannot
/// read must not quietly become "use the default".
pub(super) fn own_rate(p: &Value) -> Result<Option<dowiz_core::tax::RatePpm>, String> {
    match p.get("vat_ppm") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_u64() {
            Some(n) => dowiz_core::tax::RatePpm::parse(&n.to_string())
                .map(Some)
                .map_err(|e| format!("vat_ppm: {e}")),
            None => Err(format!("vat_ppm: {v} is not an integer in parts per million")),
        },
    }
}
