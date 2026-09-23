//! PURE. The venue's tax configuration, read from its settings image.
//!
//! Four declared keys (`dowiz_hub::settings::KNOWN`, BLUEPRINT-TAX-PRICE-
//! CHANNEL §3.3): `tax.default_ppm`, `tax.prices_include`,
//! `tax.delivery_fee_ppm`, `tax.schedule`. Every rate is `RatePpm` — an
//! integer in parts per million — and a float is refused at the settings
//! parser, which is `validate` below.
//!
//! WHERE A LINE'S RATE COMES FROM, in order: the product's own `vat_ppm`
//! (copied onto the line by the pricer), else the venue default IN FORCE at
//! `now_ms`. Nothing else — no rate by fulfilment kind (§4 item 4).
//!
//! A VENUE WITH NO RATE IS A NAMED STATE, not a zero: `resolve` answers
//! `Ok(None)`, the order carries no `tax` block, and the fiscal seam refuses
//! it as `NoTax`. That is also the transition rule for the live venues —
//! nothing changes until an owner sets one key.

use dowiz_core::tax::{schedule, RatePpm};

/// What `command::place::decide` needs to write the `tax` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueTax {
    /// The default in force at the moment of the order.
    pub default: RatePpm,
    /// `tax.prices_include`: the menu price is the gross.
    pub inclusive: bool,
    /// The delivery fee's own rate; the default when unset.
    pub fee: RatePpm,
}

/// Refuse a malformed `tax.*` value WHILE THE OWNER CAN STILL FIX IT, rather
/// than at the first order. Any other key is not this function's business.
/// An empty value clears the setting and is always accepted.
pub fn validate(key: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    match key {
        "tax.default_ppm" | "tax.delivery_fee_ppm" => {
            RatePpm::parse(value).map(|_| ()).map_err(|e| format!("{key}: {e}"))
        }
        "tax.prices_include" => match value {
            "true" | "false" => Ok(()),
            _ => Err(format!("{key}: true or false")),
        },
        "tax.schedule" => schedule::parse(value).map(|_| ()).map_err(str::to_string),
        _ => Ok(()),
    }
}

fn rate(known: &impl Fn(&str) -> String, key: &str) -> Result<Option<RatePpm>, String> {
    let v = known(key);
    if v.is_empty() {
        return Ok(None);
    }
    RatePpm::parse(&v).map(Some).map_err(|e| format!("{key}: stored value {v:?}: {e}"))
}

/// The venue's tax at `now_ms`, from a settings reader (`Settings::known`).
/// `Ok(None)`: not configured. `Err`: a stored value that does not parse —
/// loud, because `validate` should have made it impossible.
pub fn resolve(known: impl Fn(&str) -> String, now_ms: i64) -> Result<Option<VenueTax>, String> {
    let sched = schedule::parse(&known("tax.schedule")).map_err(str::to_string)?;
    let Some(default) = schedule::in_force(&sched, rate(&known, "tax.default_ppm")?, now_ms) else {
        return Ok(None);
    };
    let inclusive = match known("tax.prices_include").as_str() {
        "" | "true" => true,
        "false" => false,
        other => return Err(format!("tax.prices_include: stored value {other:?}: true or false")),
    };
    let fee = rate(&known, "tax.delivery_fee_ppm")?.unwrap_or(default);
    Ok(Some(VenueTax { default, inclusive, fee }))
}

/// A line's rate: its own, else the venue's. `None` only when the venue has
/// no rate at all, and then no line is taxed — never some lines and not others.
pub fn rate_for(line: Option<RatePpm>, venue: Option<&VenueTax>) -> Option<RatePpm> {
    venue.map(|v| line.unwrap_or(v.default))
}
