//! PURE. What an owner typed, turned into a promo code, or the refusal in the
//! owner's own words.
//!
//! AN UNKNOWN FIELD IS A REFUSAL. serde's default is to ignore what it does not
//! recognise, and this is a form that gives money away: a client sending
//! `until` instead of `untilMs` would get a code with no expiry, silently, for
//! ever.

use serde::Deserialize;

use dowiz_hub::promo::{normalise, valid_code, valid_value, Kind, Promo};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromoIn {
    pub code: String,
    pub kind: String,
    pub value: i64,
    #[serde(default)]
    pub min_order: Option<i64>,
    #[serde(default)]
    pub from_ms: Option<i64>,
    #[serde(default)]
    pub until_ms: Option<i64>,
    #[serde(default)]
    pub max_uses: Option<i64>,
    #[serde(default)]
    pub active: Option<bool>,
}

/// The form, checked and turned into the stored code.
///
/// THE REFUSAL TEXT IS PART OF THE ANSWER, so it is decided here with the rule
/// that produced it rather than at the call site where the two can drift.
pub fn to_promo(body: &PromoIn) -> Result<Promo, &'static str> {
    let code = normalise(&body.code);
    if !valid_code(&code) {
        return Err("a code is 3 to 16 letters or digits");
    }
    let Some(kind) = Kind::parse(&body.kind) else {
        return Err("a code takes off a percent or a fixed amount");
    };
    if !valid_value(kind, body.value) {
        return Err(match kind {
            Kind::Percent => "a percentage is between 1 and 100",
            Kind::Fixed => "a fixed discount must be more than nothing",
        });
    }
    if let (Some(f), Some(u)) = (body.from_ms, body.until_ms) {
        if u <= f {
            return Err("that window ends before it starts");
        }
    }
    // ZERO USES WAS UNLIMITED USES. `max_uses.filter(|n| *n > 0)` turns a 0
    // into `None`, and `None` is how this format spells "as many as anybody
    // likes" -- so an owner who typed 0 into the box, or a client that sent
    // the field with its default, published an UNCAPPED discount. The two
    // most dangerous values in that field were adjacent.
    let max_uses = match body.max_uses {
        Some(n) if n <= 0 => return Err("a code that can be used no times is not a code"),
        other => other,
    };
    Ok(Promo {
        code,
        kind,
        value: body.value,
        // A NEGATIVE MINIMUM IS NO MINIMUM. Nothing can be spent below zero,
        // so the only reading of -500 is "do not require anything", and that
        // is what 0 says.
        min_order: body.min_order.unwrap_or(0).max(0),
        from_ms: body.from_ms,
        until_ms: body.until_ms,
        max_uses,
        // A NEW CODE IS ON. An owner who has just filled in a discount form
        // meant to make a discount; the switch is there to turn one off later.
        active: body.active.unwrap_or(true),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(kind: &str, value: i64) -> PromoIn {
        PromoIn {
            code: "WELCOME".into(),
            kind: kind.into(),
            value,
            min_order: None,
            from_ms: None,
            until_ms: None,
            max_uses: None,
            active: None,
        }
    }

    /// ZERO USES WAS UNLIMITED USES, and the two most dangerous values in that
    /// field were adjacent: `None` means uncapped, and `Some(0)` was mapped to
    /// `None`. An owner who typed 0 published an uncapped discount.
    #[test]
    fn a_code_that_can_be_used_no_times_is_refused_and_never_uncapped() {
        for n in [0, -1, i64::MIN] {
            let mut f = form("percent", 10);
            f.max_uses = Some(n);
            assert_eq!(to_promo(&f), Err("a code that can be used no times is not a code"), "{n}");
        }
        let mut f = form("percent", 10);
        f.max_uses = Some(1);
        assert_eq!(to_promo(&f).unwrap().max_uses, Some(1));
        assert_eq!(to_promo(&form("percent", 10)).unwrap().max_uses, None, "unset is uncapped");
    }

    /// A PERCENTAGE IS BETWEEN 1 AND 100, and the refusal says which rule was
    /// broken rather than "invalid".
    #[test]
    fn a_value_is_checked_against_its_own_kind() {
        assert_eq!(to_promo(&form("percent", 0)), Err("a percentage is between 1 and 100"));
        assert_eq!(to_promo(&form("percent", 101)), Err("a percentage is between 1 and 100"));
        assert!(to_promo(&form("percent", 100)).is_ok());
        assert_eq!(to_promo(&form("fixed", 0)), Err("a fixed discount must be more than nothing"));
        assert_eq!(to_promo(&form("fixed", -5)), Err("a fixed discount must be more than nothing"));
        assert!(to_promo(&form("fixed", 500)).is_ok());
        assert_eq!(to_promo(&form("free-delivery", 1)), Err("a code takes off a percent or a fixed amount"));
    }

    /// THE CODE IS NORMALISED BEFORE IT IS CHECKED, so `welcome ` and
    /// `WELCOME` are the same word and cannot both exist.
    #[test]
    fn the_code_is_normalised_and_then_checked() {
        let mut f = form("percent", 10);
        f.code = "  welcome  ".into();
        assert_eq!(to_promo(&f).unwrap().code, "WELCOME");
        f.code = "ab".into();
        assert_eq!(to_promo(&f), Err("a code is 3 to 16 letters or digits"));
        f.code = "".into();
        assert_eq!(to_promo(&f), Err("a code is 3 to 16 letters or digits"));
    }

    /// A WINDOW THAT ENDS BEFORE IT STARTS IS A CODE THAT NEVER WORKS, and the
    /// owner would have watched it do nothing.
    #[test]
    fn a_window_must_end_after_it_starts() {
        let mut f = form("percent", 10);
        f.from_ms = Some(2000);
        f.until_ms = Some(1000);
        assert_eq!(to_promo(&f), Err("that window ends before it starts"));
        f.until_ms = Some(2000);
        assert_eq!(to_promo(&f), Err("that window ends before it starts"), "a zero-length window too");
        f.until_ms = Some(2001);
        assert!(to_promo(&f).is_ok());
    }

    /// AN EXPIRY IN THE PAST IS ALLOWED. The status is derived from the clock,
    /// never stored, so an owner may write down a code that has already run --
    /// and refusing it would stop them recording one they ran last month.
    #[test]
    fn a_window_entirely_in_the_past_is_a_code_that_has_expired_not_an_error() {
        let mut f = form("percent", 10);
        f.from_ms = Some(1);
        f.until_ms = Some(2);
        assert!(to_promo(&f).is_ok());
    }

    /// A NEGATIVE MINIMUM IS NO MINIMUM: nothing can be spent below zero.
    #[test]
    fn a_negative_minimum_becomes_no_minimum() {
        let mut f = form("percent", 10);
        f.min_order = Some(-500);
        assert_eq!(to_promo(&f).unwrap().min_order, 0);
        f.min_order = Some(1500);
        assert_eq!(to_promo(&f).unwrap().min_order, 1500);
        assert_eq!(to_promo(&form("percent", 10)).unwrap().min_order, 0);
    }

    /// A NEW CODE IS ON. An owner who has just filled in a discount form meant
    /// to make a discount.
    #[test]
    fn a_new_code_is_active_unless_it_says_otherwise() {
        assert!(to_promo(&form("percent", 10)).unwrap().active);
        let mut f = form("percent", 10);
        f.active = Some(false);
        assert!(!to_promo(&f).unwrap().active);
    }
}
