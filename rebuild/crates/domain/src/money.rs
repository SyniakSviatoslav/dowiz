//! `Lek` — the money newtype. Integer minor units (matches the live schema's `integer CHECK >= 0`
//! money columns — see REBUILD-MAP inventory/12 §9). There is deliberately NO `From<f64>` /
//! `From<f32>` impl anywhere on this type: float construction of a money value is not just
//! discouraged, it does not compile. Arithmetic is checked-only (no `Add`/`Sub` operator overloads,
//! which would silently panic-or-wrap on overflow); callers must handle the error case.
//!
//! Design cleared via Triadic Council (`docs/design/rust-money-newtype-phase-a/`, all artifacts:
//! proposal + ADR-rust-money-newtype + breaker-findings + counsel-opinion + resolution). Two
//! points worth carrying forward from that review:
//!
//! - **Deserialized validity is sign-checked, not authority-checked.** A client can submit
//!   `{"total": 1}` and get back a perfectly valid `Lek(1)` — this type only proves "not
//!   negative," never "this is the price the server actually charges." The order-create
//!   transaction remains the sole authority on amounts (🔴 red-line, unchanged by this type).
//! - **KNOWN LIMITATION (accepted for Phase A, must-solve-before-cutover, owner: S5
//!   orders/money lead):** `#[serde(transparent)]` serializes as a bare JSON integer. Any i64
//!   value above `2^53` (9,007,199,254,740,992) round-trips exactly within Rust (serde_json's
//!   i64 parsing is lossless) but would silently lose precision if ever decoded by a JS/browser
//!   `JSON.parse` consumer, which represents JSON numbers as f64. No such consumer exists yet in
//!   Phase A (this crate has zero wired call-sites). Before any browser-facing boundary reads a
//!   `Lek` as JSON, that boundary must either string-encode it or prove all values stay under
//!   `2^53`.

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

/// A monetary amount in minor units (e.g. qindarka for ALL), never negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Lek(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("money amount cannot be negative: {0}")]
    Negative(i64),
    #[error("quantity cannot be negative: {0}")]
    NegativeQuantity(i64),
    #[error("money arithmetic overflowed: {op}({lhs}, {rhs})")]
    Overflow {
        op: &'static str,
        lhs: i64,
        rhs: i64,
    },
}

impl Lek {
    pub const ZERO: Lek = Lek(0);

    /// Construct from an integer minor-unit amount. Rejects negative values — refunds/reversals
    /// are separate signed ledger rows in this domain (not yet modeled by this type; see the
    /// module doc note on directional money), never a negative `Lek`.
    pub fn new(minor_units: i64) -> Result<Self, MoneyError> {
        if minor_units < 0 {
            return Err(MoneyError::Negative(minor_units));
        }
        Ok(Lek(minor_units))
    }

    pub const fn minor_units(self) -> i64 {
        self.0
    }

    pub fn checked_add(self, rhs: Lek) -> Result<Lek, MoneyError> {
        self.0
            .checked_add(rhs.0)
            .map(Lek)
            .ok_or(MoneyError::Overflow {
                op: "add",
                lhs: self.0,
                rhs: rhs.0,
            })
    }

    /// Subtraction that also rejects a negative result — this domain has no representable
    /// negative `Lek`, so "underflow into negative" surfaces as `Negative`, not `Overflow`.
    /// Intentionally NOT saturating/clamping-to-zero: a promo/discount that would take a total
    /// below zero is a caller-level business error to handle explicitly (matches the existing
    /// `assertNonNegative`-throws-not-clamps semantics), not something this type silently floors.
    pub fn checked_sub(self, rhs: Lek) -> Result<Lek, MoneyError> {
        match self.0.checked_sub(rhs.0) {
            Some(result) if result < 0 => Err(MoneyError::Negative(result)),
            Some(result) => Ok(Lek(result)),
            None => Err(MoneyError::Overflow {
                op: "sub",
                lhs: self.0,
                rhs: rhs.0,
            }),
        }
    }

    /// Multiply by an integer quantity (e.g. unit price × line-item qty). Never accepts a float
    /// multiplier — quantities are integers in this domain. The negative-quantity check runs
    /// BEFORE any arithmetic, so `qty == i64::MIN` is rejected here and never reaches a
    /// `.abs()`/`.unsigned_abs()`-style path that could itself overflow (pinned by
    /// `checked_mul_qty_rejects_i64_min_before_arithmetic` below).
    pub fn checked_mul_qty(self, qty: i64) -> Result<Lek, MoneyError> {
        if qty < 0 {
            return Err(MoneyError::NegativeQuantity(qty));
        }
        self.0
            .checked_mul(qty)
            .map(Lek)
            .ok_or(MoneyError::Overflow {
                op: "mul_qty",
                lhs: self.0,
                rhs: qty,
            })
    }
}

impl fmt::Display for Lek {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for Lek {
    type Error = MoneyError;
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Lek::new(value)
    }
}

// Hand-written (not derived): `#[serde(transparent)]` + `#[derive(Deserialize)]` would decode
// straight into the tuple field, bypassing `Lek::new` — a negative wire value (`-100`) would
// silently produce an invalid `Lek(-100)`. Routing through the constructor keeps the
// non-negativity invariant true at the type's one untrusted boundary (breaker finding, resolved).
impl<'de> Deserialize<'de> for Lek {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = i64::deserialize(deserializer)?;
        Lek::new(raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_zero_and_positive() {
        assert_eq!(Lek::new(0).unwrap().minor_units(), 0);
        assert_eq!(Lek::new(500).unwrap().minor_units(), 500);
        assert_eq!(Lek::new(i64::MAX).unwrap().minor_units(), i64::MAX);
    }

    #[test]
    fn new_rejects_negative() {
        assert_eq!(Lek::new(-1), Err(MoneyError::Negative(-1)));
    }

    #[test]
    fn zero_const_matches_constructor() {
        assert_eq!(Lek::ZERO, Lek::new(0).unwrap());
    }

    #[test]
    fn checked_add_edges() {
        let a = Lek::new(100).unwrap();
        let b = Lek::new(250).unwrap();
        assert_eq!(a.checked_add(b).unwrap().minor_units(), 350);

        let max = Lek::new(i64::MAX).unwrap();
        let one = Lek::new(1).unwrap();
        assert_eq!(
            max.checked_add(one),
            Err(MoneyError::Overflow {
                op: "add",
                lhs: i64::MAX,
                rhs: 1
            })
        );
    }

    #[test]
    fn overflow_error_carries_operands_for_diagnosis() {
        let max = Lek::new(i64::MAX).unwrap();
        let err = max.checked_add(Lek::new(1).unwrap()).unwrap_err();
        assert_eq!(
            err.to_string(),
            format!("money arithmetic overflowed: add({}, 1)", i64::MAX)
        );
    }

    #[test]
    fn checked_sub_edges() {
        let a = Lek::new(500).unwrap();
        let b = Lek::new(200).unwrap();
        assert_eq!(a.checked_sub(b).unwrap().minor_units(), 300);

        // going negative is rejected, not wrapped/panicked/clamped-to-zero
        let small = Lek::new(100).unwrap();
        let large = Lek::new(200).unwrap();
        assert!(matches!(
            small.checked_sub(large),
            Err(MoneyError::Negative(_))
        ));

        assert_eq!(a.checked_sub(a).unwrap(), Lek::ZERO);
    }

    #[test]
    fn checked_mul_qty_edges() {
        let unit_price = Lek::new(1_000).unwrap();
        assert_eq!(unit_price.checked_mul_qty(3).unwrap().minor_units(), 3_000);
        assert_eq!(unit_price.checked_mul_qty(0).unwrap(), Lek::ZERO);
        assert_eq!(
            unit_price.checked_mul_qty(-1),
            Err(MoneyError::NegativeQuantity(-1))
        );

        let huge = Lek::new(i64::MAX).unwrap();
        assert_eq!(
            huge.checked_mul_qty(2),
            Err(MoneyError::Overflow {
                op: "mul_qty",
                lhs: i64::MAX,
                rhs: 2
            })
        );
    }

    /// Pins the M-2 breaker finding: `qty == i64::MIN` must be rejected by the early
    /// negative-quantity guard and must NEVER reach an `.abs()`/`.unsigned_abs()`-style path
    /// (where `i64::MIN.abs()` itself overflows i64). This crate contains no such path today;
    /// this test exists so that guarantee cannot silently regress.
    #[test]
    fn checked_mul_qty_rejects_i64_min_before_arithmetic() {
        let price = Lek::new(1).unwrap();
        assert_eq!(
            price.checked_mul_qty(i64::MIN),
            Err(MoneyError::NegativeQuantity(i64::MIN))
        );
    }

    #[test]
    fn serde_round_trip_is_a_bare_integer() {
        let amount = Lek::new(123_456).unwrap();
        let json = serde_json::to_string(&amount).unwrap();
        assert_eq!(
            json, "123456",
            "Lek must serialize as a bare integer, not an object"
        );

        let decoded: Lek = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, amount);
    }

    /// Guards the exact band the H-2 breaker finding is about: values above `2^53`
    /// (9,007,199,254,740,992 — the largest integer a JS/browser f64 represents exactly) still
    /// round-trip exactly within serde_json/Rust. `2^53 + 1` is the smallest integer where an
    /// f64-based consumer would first lose precision; using anything below `2^53` here would
    /// prove nothing about this band (a prior draft of this test used a constant that was, by
    /// mistake, still under `2^53` — re-attack-caught, see breaker-findings.md).
    #[test]
    fn serde_round_trip_exact_above_2_pow_53() {
        const ABOVE_2_POW_53: i64 = 9_007_199_254_740_993; // 2^53 + 1
        let amount = Lek::new(ABOVE_2_POW_53).unwrap();
        let json = serde_json::to_string(&amount).unwrap();
        let decoded: Lek = serde_json::from_str(&json).unwrap();
        assert_eq!(
            decoded, amount,
            "Rust-side i64 round-trip must stay exact above 2^53"
        );
        assert_eq!(decoded.minor_units(), ABOVE_2_POW_53);
    }

    #[test]
    fn rejects_float_and_string_in_json() {
        // No float construction: a JSON float value must not decode into a Lek.
        let float_result: Result<Lek, _> = serde_json::from_str("12.50");
        assert!(float_result.is_err(), "Lek must not accept a JSON float");

        // Nor a numeric string.
        let string_result: Result<Lek, _> = serde_json::from_str("\"500\"");
        assert!(string_result.is_err(), "Lek must not accept a JSON string");
    }

    /// The invariant-hole the council review caught: a naive derived `Deserialize` on a
    /// `#[serde(transparent)]` tuple struct decodes straight into the field, bypassing
    /// `Lek::new`. This must be rejected, not silently accepted as `Lek(-100)`.
    #[test]
    fn rejects_negative_at_deserialization_boundary() {
        let result: Result<Lek, _> = serde_json::from_str("-100");
        assert!(
            result.is_err(),
            "a negative wire value must not decode into a valid Lek"
        );
    }

    #[test]
    fn try_from_i64_matches_new() {
        assert_eq!(Lek::try_from(42i64), Lek::new(42));
        assert_eq!(Lek::try_from(-1i64), Lek::new(-1));
    }
}
