//! PURE. HOW FAR THE PHONE AND THE VENUE AGREE ON WHAT GUESTS LIKE (W-TASTE2 S7a): for every
//! profile holding both sections -- the venue's folded `sense` and the phone's last `taste_sync`
//! vector -- the consistency radius (`dowiz_hub::sense::radius`), summarised for the whole venue.
//! A count and three figures for the owner's health page and the log; no key, no list, and
//! nothing acts on it (R-S7 E1: the radius measures distance, it does not detect conflicts).

use dowiz_hub::sense::radius::{radius_pm, summarise, Summary};

use super::Profile;

/// The venue's radius over every profile with both sections.
pub fn summary(profiles: &[Profile]) -> Summary {
    summarise(profiles.iter().filter_map(|p| radius_pm(&p.sense, &p.device.as_ref()?.sense)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::customers::taste::SyncIn;

    fn p(own: &[(&str, i64)], phone: Option<&[(&str, i64)]>) -> Profile {
        let m = |xs: &[(&str, i64)]| xs.iter().map(|(k, w)| (k.to_string(), *w)).collect();
        Profile { v: 1, sense: m(own), device: phone.map(|x| SyncIn { v: 1, sense: m(x), ..SyncIn::default() }), ..Profile::default() }
    }

    #[test]
    fn only_profiles_with_both_sections_are_compared() {
        let all = [
            p(&[("a:smoky", 2000)], Some(&[("a:smoky", 900)])),               // agree: 0
            p(&[("a:smoky", 2000)], Some(&[("t:sweet", 900)])),               // disagree: 1000
            p(&[("a:smoky", 2000)], None),                                     // no phone vector
            p(&[], Some(&[("t:sweet", 900)])),                                 // nothing folded yet
        ];
        let s = summary(&all);
        assert_eq!((s.compared, s.max_pm, s.median_pm, s.over_half), (2, 1000, 0, 1), "{s:?}");
        assert_eq!(summary(&[]).compared, 0);
    }
}
