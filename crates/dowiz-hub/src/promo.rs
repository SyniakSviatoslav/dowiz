//! Promo codes: what a code is, and what it refuses.
//!
//! THE DISCOUNT IS A REFUSAL ENGINE, not a calculator. Almost every line here
//! is about saying no -- expired, not yet started, used up, basket too small --
//! because a promo code that silently applies when it should not is money the
//! venue did not agree to give away, and one that silently fails is a customer
//! who thinks they were cheated. Both need a NAMED reason.
//!
//! NO CLOCK, NO STORE, NO FLOAT. `now_ms` and the used-count arrive as
//! arguments, so this module replays identically on any node (MANIFESTO C2) and
//! can be tested at any instant in history. The money is integer minor units
//! throughout, and a percentage is `subtotal * pct / 100` -- integer division,
//! which rounds the discount DOWN, in the venue's favour by at most one minor
//! unit. Rounding the other way would let a customer with the right basket size
//! extract a lek that nobody budgeted.
//!
//! THE USED-COUNT IS NOT STORED HERE. It is folded from the order log by the
//! caller, for the same reason the analytics are: a counter kept beside the
//! orders is a second number that can disagree with them, and the one that
//! disagrees is always the counter.

use crate::minijson::{bool_field, esc, int_field, str_field};

/// What the code takes off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A percentage of the food subtotal, 1..=100.
    Percent,
    /// A flat amount in minor units.
    Fixed,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Percent => "percent",
            Kind::Fixed => "fixed",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "percent" => Some(Kind::Percent),
            "fixed" => Some(Kind::Fixed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Promo {
    /// The code AS TYPED BY THE CUSTOMER, normalised. This is also the storage
    /// id, which is what makes two promos with the same code unrepresentable
    /// rather than merely discouraged.
    pub code: String,
    pub kind: Kind,
    pub value: i64,
    /// The basket must reach this before the code applies at all.
    pub min_order: i64,
    pub from_ms: Option<i64>,
    pub until_ms: Option<i64>,
    pub max_uses: Option<i64>,
    /// The owner's switch. Separate from the window: a code can be paused for
    /// an evening without losing the dates it was created with.
    pub active: bool,
}

/// What an owner sees in the list. Derived, never stored -- a stored status is
/// a field that goes stale the moment the clock passes the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Active,
    Inactive,
    Scheduled,
    Expired,
    Exhausted,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Inactive => "inactive",
            Status::Scheduled => "scheduled",
            Status::Expired => "expired",
            Status::Exhausted => "exhausted",
        }
    }
}

/// Why a code did not apply. Every variant carries enough to write a sentence a
/// customer can act on; `BelowMinimum` carries the number they have to reach,
/// because "spend more" without a figure is not an instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Unknown,
    Inactive,
    Scheduled,
    Expired,
    Exhausted,
    BelowMinimum(i64),
}

impl Refusal {
    pub fn as_str(&self) -> &'static str {
        match self {
            Refusal::Unknown => "unknown code",
            Refusal::Inactive => "this code is not active",
            Refusal::Scheduled => "this code has not started yet",
            Refusal::Expired => "this code has expired",
            Refusal::Exhausted => "this code has been fully used",
            Refusal::BelowMinimum(_) => "the order is below this code's minimum",
        }
    }
}

/// Normalise a code the way a customer will get it wrong: lower case, stray
/// spaces, a copied trailing newline. Everything that is not a letter or digit
/// is dropped, so `save-10`, `SAVE 10` and `save10` are one code.
pub fn normalise(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// A code is 3..=16 characters after normalising. The ceiling matches the
/// invite code the couriers get; the floor stops a one-letter code that a
/// customer would hit by accident.
pub fn valid_code(code: &str) -> bool {
    (3..=16).contains(&code.chars().count())
}

impl Promo {
    /// Parse the stored JSON. A promo whose `kind` or `value` is unusable is
    /// `None` rather than a promo that gives away an unbounded amount.
    pub fn parse(json: &str) -> Option<Promo> {
        let code = normalise(&str_field(json, "code")?);
        if !valid_code(&code) {
            return None;
        }
        let kind = Kind::parse(&str_field(json, "kind")?)?;
        let value = int_field(json, "value")?;
        if !valid_value(kind, value) {
            return None;
        }
        Some(Promo {
            code,
            kind,
            value,
            min_order: int_field(json, "minOrder").unwrap_or(0).max(0),
            from_ms: int_field(json, "fromMs"),
            until_ms: int_field(json, "untilMs"),
            max_uses: int_field(json, "maxUses").filter(|n| *n > 0),
            active: bool_field(json, "active").unwrap_or(true),
        })
    }

    pub fn to_json(&self) -> String {
        let opt = |k: &str, v: Option<i64>| match v {
            Some(n) => format!(",\"{k}\":{n}"),
            None => String::new(),
        };
        format!(
            "{{\"code\":\"{}\",\"kind\":\"{}\",\"value\":{},\"minOrder\":{},\"active\":{}{}{}{}}}",
            esc(&self.code),
            self.kind.as_str(),
            self.value,
            self.min_order,
            self.active,
            opt("fromMs", self.from_ms),
            opt("untilMs", self.until_ms),
            opt("maxUses", self.max_uses),
        )
    }

    /// The order the checks run in is the order the owner would read them: a
    /// code they switched off reads as off even if it also expired last week,
    /// because "I turned it off" is the answer they are looking for.
    pub fn status(&self, now_ms: i64, used: i64) -> Status {
        if !self.active {
            return Status::Inactive;
        }
        if self.from_ms.is_some_and(|f| now_ms < f) {
            return Status::Scheduled;
        }
        if self.until_ms.is_some_and(|u| now_ms >= u) {
            return Status::Expired;
        }
        if self.max_uses.is_some_and(|m| used >= m) {
            return Status::Exhausted;
        }
        Status::Active
    }

    /// What this takes off a subtotal, in minor units. Never more than the
    /// subtotal: a discount that exceeds the food would make the venue owe the
    /// customer money, and the delivery fee is deliberately outside it -- the
    /// courier is paid either way.
    pub fn discount(&self, subtotal: i64) -> i64 {
        let raw = match self.kind {
            // i128 for the product: 100% of a basket near i64::MAX would wrap
            // in release, and a wrapped discount is a negative total.
            Kind::Percent => ((subtotal.max(0) as i128 * self.value as i128) / 100) as i64,
            Kind::Fixed => self.value,
        };
        raw.clamp(0, subtotal.max(0))
    }

    /// The whole question, answered once: what comes off, or why nothing does.
    pub fn redeem(&self, subtotal: i64, now_ms: i64, used: i64) -> Result<i64, Refusal> {
        match self.status(now_ms, used) {
            Status::Inactive => return Err(Refusal::Inactive),
            Status::Scheduled => return Err(Refusal::Scheduled),
            Status::Expired => return Err(Refusal::Expired),
            Status::Exhausted => return Err(Refusal::Exhausted),
            Status::Active => {}
        }
        if subtotal < self.min_order {
            return Err(Refusal::BelowMinimum(self.min_order));
        }
        Ok(self.discount(subtotal))
    }
}

/// A percentage outside 1..=100 is not a discount, and a fixed amount of zero
/// or less is a code that does nothing while looking like it works.
pub fn valid_value(kind: Kind, value: i64) -> bool {
    match kind {
        Kind::Percent => (1..=100).contains(&value),
        Kind::Fixed => value > 0,
    }
}

#[cfg(test)]
mod tests;
