//! `autonomic.rs` — pure no_std core: the bounded rate newtype (item 21).
//!
//! [`BoundedRate`] is a newtype over `f64` that **cannot** hold a value outside
//! `[BOUND_MIN, BOUND_MAX]`. The field is private and the only constructors clamp
//! ([`BoundedRate::from_f64`]) or reject ([`BoundedRate::try_from_f64`]), so an
//! out-of-bound rate is *inexpressible* — not merely avoided.
//!
//! This is the pure half of the kernel's `autonomic` module; the std side (the
//! gain-scheduling control law `schedule`, the FDR-tagged `FdrAdjustment`, and the
//! breaker routing) stays in the kernel shim, which re-exports this type.

const BOUND_MIN: f64 = 0.0;
const BOUND_MAX: f64 = 100.0;

/// A newtype over `f64` that cannot hold a value outside `[BOUND_MIN, BOUND_MAX]`.
/// The field is private and the only constructors clamp/reject, so an out-of-bound
/// `BoundedRate` is inexpressible.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BoundedRate {
    value: f64,
}

impl BoundedRate {
    /// The proven-stable interval (named constants, not literals scattered in code).
    pub const MIN: f64 = BOUND_MIN;
    pub const MAX: f64 = BOUND_MAX;

    /// Clamping constructor — the ONLY path the control-law `schedule` uses. An
    /// out-of-bound input is brought back into `[MIN, MAX]`, so the over-grant
    /// invariant can never be broken by an adjustment equation.
    #[inline]
    pub fn from_f64(v: f64) -> Self {
        let v = if v.is_nan() {
            BOUND_MIN
        } else if v < BOUND_MIN {
            BOUND_MIN
        } else if v > BOUND_MAX {
            BOUND_MAX
        } else {
            v
        };
        debug_assert!(v >= BOUND_MIN && v <= BOUND_MAX, "BoundedRate clamp broke");
        BoundedRate { value: v }
    }

    /// Rejecting constructor — returns `None` iff `v` is outside `[MIN, MAX]` or NaN.
    /// The public fallible entry point and the proof surface for the planted-fault
    /// self-test: an unsafe law that would produce an out-of-bound rate is *rejected*.
    #[inline]
    pub fn try_from_f64(v: f64) -> Option<Self> {
        if v < BOUND_MIN || v > BOUND_MAX || v.is_nan() {
            None
        } else {
            Some(BoundedRate { value: v })
        }
    }

    /// The rate as a raw `f64`. Always within `[MIN, MAX]` (the invariant).
    #[inline]
    pub fn get(self) -> f64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_are_inclusive() {
        assert!(BoundedRate::try_from_f64(BoundedRate::MIN).is_some());
        assert!(BoundedRate::try_from_f64(BoundedRate::MAX).is_some());
    }

    #[test]
    fn try_from_rejects_out_of_bound_and_nan() {
        assert!(BoundedRate::try_from_f64(BoundedRate::MAX + 1.0).is_none());
        assert!(BoundedRate::try_from_f64(BoundedRate::MIN - 1.0).is_none());
        assert!(BoundedRate::try_from_f64(f64::NAN).is_none());
        assert!(BoundedRate::try_from_f64(f64::INFINITY).is_none());
    }

    #[test]
    fn from_f64_clamps() {
        assert_eq!(BoundedRate::from_f64(1e9).get(), BoundedRate::MAX);
        assert_eq!(BoundedRate::from_f64(-1e9).get(), BoundedRate::MIN);
        assert_eq!(BoundedRate::from_f64(f64::NAN).get(), BoundedRate::MIN);
        assert_eq!(BoundedRate::from_f64(50.0).get(), 50.0);
    }
}
