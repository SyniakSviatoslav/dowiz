//! PURE. THE CONSISTENCY RADIUS OF TWO SECTIONS OVER ONE COVER (W-TASTE2, S7a; R-S7 research
//! 2026-10-06 §E1 and Part 4, operator: "consistency radius goes to owner health, not a red alarm,
//! and to the log").
//!
//! WHAT IT IS. Two contexts describe the same thing: here the guest's PHONE (the aggregated vector
//! it sends with an order, `taste_sync.sense`) and the VENUE (the profile the hub folds from the
//! same guest's orders). Each is a section over the 27 keys of the taste vocabulary; the radius is
//! Robinson's: the largest disagreement on the overlap, L-infinity over the union of keys, after
//! both sides are scaled to per mille of their own strongest key (`rank::per_mille`), so it reads
//! "0 = the two agree on what this guest likes, 1000 = they agree on nothing".
//!
//! WHAT IT IS NOT, measured by R-S7 E1: a detector. It separated only 645/1000 oversold merges,
//! 2/1000 double amends and 0/1000 count-vs-sale conflicts. So NOTHING ACTS ON IT: it is shown on
//! the owner's health page as a figure and written to the log, never a refusal, never an alarm,
//! never a decision about a person (it is a property of two data sources, aggregated per venue).
//! The stock cover (hub / kitchen / waiter) waits for S0's `glue()`, which does not exist yet.
//!
//! INTEGERS ONLY.

use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::rank::per_mille;

/// The radius of two sections, per mille; `None` when either side is empty (no overlap to
/// measure, which is not the same as agreement).
pub fn radius_pm(a: &BTreeMap<String, i64>, b: &BTreeMap<String, i64>) -> Option<i64> {
    let (a, b) = (per_mille(a), per_mille(b));
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let keys = a.keys().chain(b.keys());
    Some(keys.map(|k| (a.get(k).copied().unwrap_or(0) - b.get(k).copied().unwrap_or(0)).abs()).max().unwrap_or(0))
}

/// A venue's radii, summarised. No list, no key: a count and three figures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Summary {
    /// How many pairs had both sections.
    pub compared: i64,
    pub max_pm: i64,
    /// The lower median.
    pub median_pm: i64,
    /// Pairs whose two sections disagree by more than half the scale.
    pub over_half: i64,
}

impl Summary {
    pub fn json(&self) -> Value {
        json!({ "contract": "sheaf.radius.v1", "compared": self.compared, "maxPm": self.max_pm,
                "medianPm": self.median_pm, "overHalf": self.over_half, "acts": false })
    }
}

/// Every radius of a cover's pairs into one summary.
pub fn summarise(radii: impl IntoIterator<Item = i64>) -> Summary {
    let mut v: Vec<i64> = radii.into_iter().collect();
    if v.is_empty() {
        return Summary::default();
    }
    v.sort_unstable();
    Summary {
        compared: v.len() as i64,
        max_pm: v[v.len() - 1],
        median_pm: v[(v.len() - 1) / 2],
        over_half: v.iter().filter(|r| **r > 500).count() as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(xs: &[(&str, i64)]) -> BTreeMap<String, i64> {
        xs.iter().map(|(k, w)| (k.to_string(), *w)).collect()
    }

    #[test]
    fn the_same_taste_at_any_scale_is_radius_zero_and_disjoint_tastes_are_the_whole_scale() {
        let a = m(&[("a:smoky", 2000), ("x:crispy", 1000)]);
        assert_eq!(radius_pm(&a, &m(&[("a:smoky", 1000), ("x:crispy", 500)])), Some(0), "scale-free");
        assert_eq!(radius_pm(&a, &m(&[("t:sweet", 900)])), Some(1000), "no key in common");
        // smoky 1000 vs 1000, crispy 500 vs 1000: the largest gap is 500.
        assert_eq!(radius_pm(&a, &m(&[("a:smoky", 700), ("x:crispy", 700)])), Some(500));
    }

    #[test]
    fn an_empty_side_is_no_measurement_not_agreement() {
        let a = m(&[("a:smoky", 10)]);
        assert_eq!(radius_pm(&a, &BTreeMap::new()), None);
        assert_eq!(radius_pm(&BTreeMap::new(), &a), None);
        assert_eq!(radius_pm(&a, &m(&[("a:smoky", 0)])), None, "a zero weight is no weight");
        assert_eq!(radius_pm(&a, &a), Some(0), "positive twin");
    }

    #[test]
    fn a_summary_is_a_count_and_three_figures_and_empty_is_all_zero() {
        assert_eq!(summarise(Vec::new()), Summary::default());
        let s = summarise([300, 0, 1000, 600]);
        assert_eq!(s, Summary { compared: 4, max_pm: 1000, median_pm: 300, over_half: 2 });
        let j = s.json();
        assert_eq!(j["acts"], false, "it is shown, never acted on");
        assert_eq!(j.as_object().unwrap().len(), 6, "no list of pairs, no key: {j}");
    }
}
