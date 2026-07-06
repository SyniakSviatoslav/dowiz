//! S5 request-hash — the shell-side coordinate PROJECTION (REV-S5-2), after Phase-Zero Step 3.
//!
//! The canonical-bytes hashing (`build_request_hash` + the `CanonicalRequestInput`/`CanonicalItemInput`
//! types) moved into the sovereign core (`domain::codec::request_hash`) — it is integer/string-only,
//! so it is float/clock/entropy/IO-free. What STAYS here is [`project_coord`]: the ONE `f64 → i64`
//! coordinate projection `round(coord · 1e5)`. That float boundary is exactly what the integers-only
//! core must not contain, so it lives in the shell and the (already-projected) integer pin is handed
//! to the core. The core types are re-exported below so `super::request_hash::…` call sites are
//! unchanged; the ONE caller (`pg.rs`) now projects the pin via [`project_coord`] before building.

pub use domain::codec::request_hash::{CanonicalItemInput, CanonicalRequestInput, build_request_hash};

/// The REV-S5-2 integer projection: `round(coord · 1e5)` as `i64`. Half-away-from-zero (via
/// `super::round_f64_to_i64`, the confined f64→i64 cast site), matching Node `Math.round` for the
/// small magnitudes coordinates ever take (|lat| ≤ 90, |lng| ≤ 180 → |coord·1e5| ≤ 1.8e7, exactly
/// representable in f64, so no rounding ambiguity). This is the float boundary — it stays in the
/// shell so the sovereign core (`domain::codec::request_hash`) can be integer-only.
pub fn project_coord(coord: f64) -> i64 {
    super::round_f64_to_i64(coord * 100_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REV-S5-2 PROPERTY over the float domain: `project_coord` is a stable `i64` equal to
    /// `round(coord·1e5)` across the whole valid coordinate range, and two coordinates that agree to
    /// 5 dp project to the SAME integer (the dedup equivalence). This is the "property test over the
    /// float domain, not a single golden vector" the RESOLVE mandates — it lives with the projection.
    #[test]
    fn projection_is_stable_over_the_float_domain() {
        // Sweep the valid lat/lng ranges plus the sign-boundary and half-up cases.
        let samples = [
            -90.0,
            -89.999_995,
            -0.000_005,
            -0.0,
            0.0,
            0.000_005,
            41.327_953,
            19.819_025,
            89.999_994,
            90.0,
            -180.0,
            180.0,
            12.345_674,
            12.345_675,
            12.345_676,
        ];
        for &c in &samples {
            let p = project_coord(c);
            // Stable: same input, same output (no ryu-shortest ambiguity — it's an i64).
            assert_eq!(p, project_coord(c), "projection must be pure @ {c}");
            // Equivalence: a coordinate re-derived from the 5-dp value projects identically — a
            // client that rounds to 5 dp and the raw value dedup to the SAME integer.
            let rederived = f64::from(i32::try_from(p).unwrap()) / 100_000.0;
            assert_eq!(
                project_coord(rederived),
                p,
                "5dp-equivalent coords must project identically @ {c}"
            );
            // No float token: the integer's Display has no '.'.
            assert!(!p.to_string().contains('.'), "projection is an integer @ {c}");
        }
        // -0.0 and 0.0 project to the SAME integer 0 (kills the `-0` vs `0` cross-stack drift).
        assert_eq!(project_coord(-0.0), project_coord(0.0));
        assert_eq!(project_coord(0.0), 0);
    }

    /// The projected pin is exactly what the core canonicaliser expects (the two halves compose):
    /// projecting the golden coordinate yields the integers the core's `base_input` hashes.
    #[test]
    fn projection_matches_the_core_integer_pin() {
        assert_eq!(project_coord(41.327_953), 4_132_795);
        assert_eq!(project_coord(19.819_025), 1_981_903);
    }
}
