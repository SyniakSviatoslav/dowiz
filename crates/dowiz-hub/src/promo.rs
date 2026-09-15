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
mod tests {
    use super::*;

    const HOUR: i64 = 3_600_000;
    const NOW: i64 = 1_789_000_000_000;

    fn pct(v: i64) -> Promo {
        Promo {
            code: "SAVE10".into(),
            kind: Kind::Percent,
            value: v,
            min_order: 0,
            from_ms: None,
            until_ms: None,
            max_uses: None,
            active: true,
        }
    }

    #[test]
    fn a_code_is_the_same_code_however_the_customer_types_it() {
        for raw in ["save10", "SAVE10", " Save 10 ", "save-10", "save10\n"] {
            assert_eq!(normalise(raw), "SAVE10", "{raw:?}");
        }
    }

    #[test]
    fn a_percentage_rounds_down_never_up() {
        // 33% of 1000 is 330.0; 33% of 1001 is 330.33, and the customer gets
        // 330. The venue is never short by the rounding.
        assert_eq!(pct(33).discount(1000), 330);
        assert_eq!(pct(33).discount(1001), 330);
        assert_eq!(pct(33).discount(1004), 331);
    }

    #[test]
    fn a_discount_never_exceeds_the_food() {
        let flat = Promo { kind: Kind::Fixed, value: 5000, ..pct(10) };
        assert_eq!(flat.discount(2000), 2000, "a 5000 code on a 2000 basket takes 2000");
        assert_eq!(flat.discount(0), 0);
        assert_eq!(pct(100).discount(2650), 2650);
    }

    /// Release builds wrap on overflow. A hundred per cent of a basket near the
    /// i64 ceiling must not come back negative -- a negative discount is money
    /// ADDED to the bill.
    #[test]
    fn a_huge_basket_does_not_wrap_the_discount() {
        let d = pct(100).discount(i64::MAX);
        assert!(d >= 0 && d <= i64::MAX, "discount out of range: {d}");
        assert_eq!(d, i64::MAX);
    }

    #[test]
    fn each_refusal_names_itself() {
        let base = Promo {
            from_ms: Some(NOW),
            until_ms: Some(NOW + HOUR),
            max_uses: Some(2),
            min_order: 1000,
            ..pct(10)
        };
        assert_eq!(base.redeem(2000, NOW - 1, 0), Err(Refusal::Scheduled));
        assert_eq!(base.redeem(2000, NOW + HOUR, 0), Err(Refusal::Expired));
        assert_eq!(base.redeem(2000, NOW + 1, 2), Err(Refusal::Exhausted));
        assert_eq!(base.redeem(999, NOW + 1, 0), Err(Refusal::BelowMinimum(1000)));
        assert_eq!(Promo { active: false, ..base.clone() }.redeem(2000, NOW + 1, 0),
                   Err(Refusal::Inactive));
        assert_eq!(base.redeem(2000, NOW + 1, 1), Ok(200));
    }

    /// The window is half-open: the first millisecond counts, the last does
    /// not. An "until midnight" code that still works at 00:00:00.000 the next
    /// day is the classic off-by-one that shows up as a complaint.
    #[test]
    fn the_window_is_half_open() {
        let p = Promo { from_ms: Some(NOW), until_ms: Some(NOW + HOUR), ..pct(10) };
        assert_eq!(p.status(NOW - 1, 0), Status::Scheduled);
        assert_eq!(p.status(NOW, 0), Status::Active, "the first instant counts");
        assert_eq!(p.status(NOW + HOUR - 1, 0), Status::Active);
        assert_eq!(p.status(NOW + HOUR, 0), Status::Expired, "the last does not");
    }

    /// The owner's switch is read first: "I turned it off" is the answer they
    /// are looking for, even when the code also expired.
    #[test]
    fn off_reads_as_off_even_when_it_also_expired() {
        let p = Promo { active: false, until_ms: Some(NOW - HOUR), ..pct(10) };
        assert_eq!(p.status(NOW, 0), Status::Inactive);
    }

    #[test]
    fn a_promo_round_trips_through_json() {
        let p = Promo {
            code: "WELCOME".into(),
            kind: Kind::Fixed,
            value: 300,
            min_order: 1500,
            from_ms: Some(NOW),
            until_ms: Some(NOW + HOUR),
            max_uses: Some(50),
            active: false,
        };
        assert_eq!(Promo::parse(&p.to_json()), Some(p));
    }

    #[test]
    fn an_unusable_promo_is_refused_at_parse_rather_than_applied() {
        for bad in [
            r#"{"code":"X","kind":"percent","value":10}"#,          // code too short
            r#"{"code":"SAVE","kind":"percent","value":0}"#,        // 0%
            r#"{"code":"SAVE","kind":"percent","value":101}"#,      // over 100%
            r#"{"code":"SAVE","kind":"percent","value":-10}"#,      // negative
            r#"{"code":"SAVE","kind":"fixed","value":0}"#,          // a code that does nothing
            r#"{"code":"SAVE","kind":"freebie","value":1}"#,        // unknown kind
            r#"{"code":"SAVE","value":10}"#,                        // no kind
        ] {
            assert_eq!(Promo::parse(bad), None, "accepted: {bad}");
        }
    }

    /// `maxUses: 0` would mean a code that can never be used, which nobody
    /// creates on purpose -- it is what an empty form field serialises to. It
    /// reads as no limit.
    #[test]
    fn a_zero_use_cap_reads_as_no_cap() {
        let p = Promo::parse(r#"{"code":"SAVE","kind":"percent","value":10,"maxUses":0}"#).unwrap();
        assert_eq!(p.max_uses, None);
        assert_eq!(p.status(NOW, 9_999), Status::Active);
    }

    #[test]
    fn active_defaults_to_on_but_false_survives() {
        let on = Promo::parse(r#"{"code":"SAVE","kind":"percent","value":10}"#).unwrap();
        assert!(on.active);
        let off =
            Promo::parse(r#"{"code":"SAVE","kind":"percent","value":10,"active":false}"#).unwrap();
        assert!(!off.active);
    }
}
