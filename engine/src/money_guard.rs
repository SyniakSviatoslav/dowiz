//! FE-09 — Field↔state boundary (money-never-tween guard) 🔴 RED-LINE.
//!
//! RED→GREEN GATE (per blueprint): money is a DISCRETE channel, never a field
//! channel. The field continuum interpolates screen coordinates, NOT monetary
//! amounts. A money value must never pass through spring/interpolation; it
//! jumps integer→integer by construction.
//!
//! Enforced TWO ways:
//! 1. **Type-level (compile-time):** `Money` does NOT implement `FieldValue`,
//!    so `Spring<Money>` / `interpolate(money, ..)` is a build error. The type
//!    system makes the illegal path unrepresentable.
//! 2. **Runtime guard:** `TweenGuard::present_money` refuses any fractional/
//!    interpolated money. It takes an `f64` precisely so a fractional value is
//!    representable and rejected (FEYNMAN-07: an `i64` parameter could never be
//!    fractional, so the old guard was dead code). Money is integer minor-unit
//!    by construction; a non-integer here is the RED signature of tweening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money(pub i64);

/// Marker trait for values the field MAY interpolate (screen coordinates,
/// opacity, scale). Deliberately NOT implemented for [`Money`].
pub trait FieldValue: Copy {
    /// Linear interpolate `self`→`other` at `t∈[0,1]`.
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl FieldValue for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t.clamp(0.0, 1.0)
    }
}

impl FieldValue for (f32, f32) {
    fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        (
            self.0 + (other.0 - self.0) * t,
            self.1 + (other.1 - self.1) * t,
        )
    }
}

/// Animate a FIELD value from `a`→`b` over `t∈[0,1]` (lerp). Only `FieldValue`
/// types allowed — `Money` is excluded at compile time.
pub fn interpolate<T: FieldValue>(a: T, b: T, t: f32) -> T {
    a.lerp(b, t)
}

/// Runtime guard for presenting money. Money is emitted as the DECIDED integer
/// — never an interpolated fraction. Takes an `f64` (not `i64`) so that a
/// fractional/"interpolated" value is actually representable and can be
/// rejected (FEYNMAN-07: the old `i64` parameter made the guard dead code
/// because every `i64` is an exact integer as `f64`).
pub struct TweenGuard;

impl TweenGuard {
    /// Present a decided money amount. `amount_minor` MUST be an integer minor
    /// unit; the guard rejects a fractional presentation (the RED signature of
    /// tweening). Returns the integer minor value on success.
    pub fn present_money(amount_minor: f64) -> Result<i64, String> {
        // Live guard: reject any fractional input. (An `f64` can be non-integral;
        // an `i64` parameter could not, which is why the prior guard was dead.)
        if (amount_minor.fract()).abs() > 1e-9 {
            return Err(
                "money must be presented as a decided integer, never interpolated".into(),
            );
        }
        Ok(amount_minor.round() as i64)
    }

    /// The ONLY legal money transition: jump integer→integer, no interpolation.
    pub fn jump(_from: Money, to: Money) -> Money {
        to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // GREEN: field values interpolate freely.
    #[test]
    fn field_value_interpolates() {
        assert!((interpolate(0.0_f32, 10.0_f32, 0.5) - 5.0).abs() < 1e-6);
        assert_eq!(
            interpolate((0.0_f32, 0.0_f32), (2.0_f32, 4.0_f32), 0.5),
            (1.0, 2.0)
        );
    }

    // GREEN: money jumps integer→integer, no intermediate.
    #[test]
    fn money_jumps_integer_to_integer() {
        let m = TweenGuard::jump(Money(100), Money(250));
        assert_eq!(m.0, 250);
        assert!(TweenGuard::present_money(250.0).is_ok());
    }

    // RED→GREEN: the guard is LIVE — a fractional/"interpolated" amount is
    // actually rejected (FEYNMAN-07 fixed the dead `i64` guard).
    #[test]
    fn present_money_rejects_fractional() {
        // The classic tween product: lerp(100,250,0.37) = 155.5 minor units.
        let interpolated: f64 = 100.0 + (250.0 - 100.0) * 0.37; // 155.5
        assert!(
            interpolated.fract().abs() > 1e-9,
            "setup: the interpolated value is genuinely fractional"
        );
        assert!(
            TweenGuard::present_money(interpolated).is_err(),
            "fractional money (the RED tween signature) must be rejected"
        );
        // A clean integer presentation still succeeds.
        assert_eq!(TweenGuard::present_money(155.0).unwrap(), 155);
    }

    // Compile-time proof that Money is NOT a FieldValue:
    // the following would NOT compile (intentionally commented):
    //   interpolate(Money(100), Money(250), 0.5);  // ERROR: Money: !FieldValue
    //   Spring::<Money>::snappy(Money(0));          // ERROR: Spring requires FieldValue
    //
    // Runtime mirror of the compile-time guarantee: present_money rejects a
    // genuinely fractional (interpolated) amount.
    #[test]
    fn compile_time_boundary_mirrored_at_runtime() {
        // The interpolated "in-between" amount a tween would produce.
        let interpolated: f64 = 100.0 + (250.0 - 100.0) * 0.37; // 155.5 — fractional
        assert!(
            TweenGuard::present_money(interpolated).is_err(),
            "fractional (interpolated) money must be rejected by the live guard"
        );
        // A clean integer decides fine.
        assert_eq!(TweenGuard::present_money(155.0).unwrap(), 155);
        // The real guarantee: Money never enters interpolate(). We assert the
        // type does not carry fractional state by construction.
        let decided = Money(155);
        assert_eq!(decided.0, 155); // integer, always
    }
}
