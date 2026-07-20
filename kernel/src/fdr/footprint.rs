//! `fdr/footprint.rs` — Item 69: water / carbon as derived, constant-multiplied views of `joules_uj`.
//!
//! The kernel needs NO new *measured* footprint field beyond `joules_uj` (item 27's RAPL/PMU
//! work already is the silicon power-draw mechanism). This module is a **consumer-side**
//! derivation layer keyed on operator-supplied `(region, deployment-class)` constants — it
//! *derives* from `joules_uj`, it does NOT *store* a footprint (roadmap line 1194; fabricating a
//! facility-cooling number software cannot observe is procedure step 4).
//!
//! Honesty invariants (item 69 §69.3/§69.4):
//!   * `co2e`  = `joules × grid-carbon-intensity` (gCO₂e/kWh), degrading to a named absence when
//!     `joules_uj` is absent OR the regional constant is unsupplied.
//!   * `offsite_water` = `joules × WUE-source` (L/kWh), same degradation.
//!   * `onsite_water` = PERMANENT named absence (`NotSoftwareObservable`) by construction — a local
//!     device cannot observe facility cooling; there must be NO code path that produces an
//!     on-site-water value.
//!   * The module ships with the regional constants ABSENT (named-absence) and only lights up when
//!     the operator supplies them. No new `HwStamp` field is added.

use super::schema::{Absence, Reading};

/// µJ per kWh (1 kWh = 3.6e9 J = 3.6e15 µJ). Used to convert the joules reading into a
/// normalized energy unit before multiplying by the operator's regional constant.
const MICROJ_PER_KWH: f64 = 3.6e15;

/// Operator-supplied regional grid constants. Absent by default (named-absence discipline) —
/// the module never fabricates a default constant.
#[derive(Clone, Copy, Debug, Default)]
pub struct RegionConstants {
    /// Grid carbon intensity, gCO₂e per kWh. `None` ⇒ derives to `Unavailable(NoRegionalConstant)`.
    pub carbon_g_per_kwh: Option<f64>,
    /// Water-usage-effectiveness of off-site (source) water, L per kWh. `None` ⇒ derives to
    /// `Unavailable(NoRegionalConstant)`.
    pub wue_l_per_kwh: Option<f64>,
}

impl RegionConstants {
    /// The default: BOTH constants absent. The kernel ships in this state; derived views are
    /// named absences until the operator supplies real values.
    pub fn missing() -> Self {
        RegionConstants {
            carbon_g_per_kwh: None,
            wue_l_per_kwh: None,
        }
    }

    /// Construct with explicit (operator-provided) constants.
    pub fn new(carbon_g_per_kwh: Option<f64>, wue_l_per_kwh: Option<f64>) -> Self {
        RegionConstants {
            carbon_g_per_kwh,
            wue_l_per_kwh,
        }
    }
}

/// Item 69 (a): `co2e = joules × carbon-intensity`, in microgrammes.
///
/// Degrades to a named absence when `joules_uj` is absent (the same reason) OR when the regional
/// carbon constant is unsupplied (`NoRegionalConstant`). The product is rounded to the nearest µ
/// since the kernel carries integer `Reading<u64>` everywhere.
pub fn co2e_ug(joules: Reading<u64>, carbon_g_per_kwh: Option<f64>) -> Reading<u64> {
    match joules {
        Reading::Unavailable(a) => Reading::Unavailable(a),
        Reading::Value(j) => match carbon_g_per_kwh {
            None => Reading::Unavailable(Absence::NoRegionalConstant),
            Some(c) => {
                let kwh = j as f64 / MICROJ_PER_KWH;
                let ug = kwh * c * 1e6;
                if !ug.is_finite() || ug < 0.0 {
                    Reading::Unavailable(Absence::ReadError)
                } else {
                    Reading::Value(ug.round() as u64)
                }
            }
        },
    }
}

/// Item 69 (a): `off-site water = joules × WUE-source`, in microlitres. Same degradation as
/// [`co2e_ug`] (absent joules ⇒ same reason; absent WUE constant ⇒ `NoRegionalConstant`).
pub fn offsite_water_ul(joules: Reading<u64>, wue_l_per_kwh: Option<f64>) -> Reading<u64> {
    match joules {
        Reading::Unavailable(a) => Reading::Unavailable(a),
        Reading::Value(j) => match wue_l_per_kwh {
            None => Reading::Unavailable(Absence::NoRegionalConstant),
            Some(w) => {
                let kwh = j as f64 / MICROJ_PER_KWH;
                let ul = kwh * w * 1e6;
                if !ul.is_finite() || ul < 0.0 {
                    Reading::Unavailable(Absence::ReadError)
                } else {
                    Reading::Value(ul.round() as u64)
                }
            }
        },
    }
}

/// Item 69 (b): on-site water is a PERMANENT named absence (`NotSoftwareObservable`) by
/// construction. A local device cannot observe facility cooling; this function NEVER produces a
/// `Reading::Value`. The argument is ignored (no input makes this a value).
pub fn onsite_water_ul(_joules: Reading<u64>) -> Reading<u64> {
    Reading::Unavailable(Absence::NotSoftwareObservable)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Item 69 §69.4 (a): golden derivation tests against hand-computed values.
    // 3.6e15 µJ == exactly 1 kWh. carbon = 400 gCO₂e/kWh ⇒ 400e6 µg.
    // wue = 1.5 L/kWh ⇒ 1.5e6 µL.
    #[test]
    fn co2e_and_offsite_water_golden_derivation() {
        let one_kwh = Reading::Value(3_600_000_000_000_000u64); // 3.6e15 µJ == 1 kWh
        assert_eq!(
            co2e_ug(one_kwh, Some(400.0)),
            Reading::Value(400_000_000),
            "1 kWh @ 400 gCO2e/kWh == 400e6 µg"
        );
        assert_eq!(
            offsite_water_ul(one_kwh, Some(1.5)),
            Reading::Value(1_500_000),
            "1 kWh @ 1.5 L/kWh == 1.5e6 µL"
        );
    }

    #[test]
    fn co2e_golden_second_region() {
        // 7.2e15 µJ == 2 kWh. carbon = 50 gCO2e/kWh ⇒ 100e6 µg.
        let two_kwh = Reading::Value(7_200_000_000_000_000u64);
        assert_eq!(
            co2e_ug(two_kwh, Some(50.0)),
            Reading::Value(100_000_000)
        );
    }

    // Item 69 §69.4 (b): on a RAPL-less / constant-less host every derived view is a greppable
    // named absence.
    #[test]
    fn derived_views_are_named_absences_when_inputs_absent() {
        // joules absent ⇒ the SAME absence reason is propagated (not a missing key, not a 0).
        let no_rapl = Reading::Unavailable(Absence::NoRaplInterface);
        let co2 = co2e_ug(no_rapl, Some(400.0));
        assert_eq!(co2, Reading::Unavailable(Absence::NoRaplInterface));
        let w = co2.write_field(crate::fdr::json::JsonWriter::obj(), "co2e_ug").finish();
        assert!(
            w.contains("\"co2e_ug\":{\"unavailable\":\"no_rapl_interface\"}"),
            "absent joules must serialize as a named absence: {w}"
        );

        // joules present but NO regional constant ⇒ NoRegionalConstant.
        let j = Reading::Value(3_600_000_000_000_000u64);
        let co2_nc = co2e_ug(j, None);
        assert_eq!(co2_nc, Reading::Unavailable(Absence::NoRegionalConstant));
        let w2 = co2_nc
            .write_field(crate::fdr::json::JsonWriter::obj(), "co2e_ug")
            .finish();
        assert!(
            w2.contains("\"co2e_ug\":{\"unavailable\":\"no_regional_constant\"}"),
            "absent constant must serialize as a named absence: {w2}"
        );
    }

    // Item 69 §69.4 (c): on-site water is a `Value` under NO input (grep proves no producing
    // path) — the strongest honesty clause in the item.
    #[test]
    fn onsite_water_has_no_value_producing_path() {
        // Structural grep: no source line mentioning `onsite` may produce a `Value`.
        let src = include_str!("footprint.rs");
        let prod = src.split("#[cfg(test)]").next().unwrap_or(src);
        for line in prod.lines() {
            if line.contains("onsite") {
                assert!(
                    !line.contains("Reading::Value"),
                    "no on-site-water line may produce a Value: {line}"
                );
            }
        }
        // Behavioral: ANY joules input (value OR absence) ⇒ permanent named absence.
        let jv = Reading::Value(123u64);
        let ju = Reading::Unavailable(Absence::NoRaplInterface);
        assert_eq!(
            onsite_water_ul(jv),
            Reading::Unavailable(Absence::NotSoftwareObservable)
        );
        assert_eq!(
            onsite_water_ul(ju),
            Reading::Unavailable(Absence::NotSoftwareObservable)
        );
    }

    #[test]
    fn onsite_absence_serializes_greppable() {
        let w = onsite_water_ul(Reading::Value(1))
            .write_field(crate::fdr::json::JsonWriter::obj(), "onsite_water_ul")
            .finish();
        assert!(
            w.contains("\"onsite_water_ul\":{\"unavailable\":\"not_software_observable\"}"),
            "on-site absence must be greppable: {w}"
        );
    }
}
