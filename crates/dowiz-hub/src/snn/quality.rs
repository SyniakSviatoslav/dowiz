//! QUALITY, NOT ONLY AGREEMENT (operator 2026-10-06, W-SNN question 2 approved): when a guest's
//! order page is read, the two top-3s (the current server ranker's and the network's) are HELD
//! next to that guest's server profile; at the guest's next order they are checked against the
//! dishes ordered, counted per venue (`shadow::Tally::settle`), and cleared.
//!
//! THE RULES, each a test:
//!   * HELD ONLY BESIDE A PROFILE THAT EXISTS. Holding never creates a profile, so a guest who
//!     objected (their profile is deleted, and nothing runs for them) stores nothing.
//!   * ONE CHECK PER HOLD. `settle` takes the hold out; a second order checks nothing.
//!   * NOTHING PER GUEST LEAVES THE PROFILE: only the venue's counts are written elsewhere, and the
//!     hold goes with the profile on forget-me or an objection.

use serde::{Deserialize, Serialize};

/// The two top-3s held for one guest until their next order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub current: Vec<String>,
    pub snn: Vec<String>,
    /// The model that ranked `snn` (`Model::id`).
    pub model: u32,
    /// The UTC day number of the page read.
    pub day: i64,
}

impl Held {
    /// The ids of the first `k` of each list.
    pub fn of(current: &[(String, i64)], snn: &[(String, i64)], k: usize, model: u32, day: i64) -> Held {
        let ids = |v: &[(String, i64)]| v.iter().take(k).map(|x| x.0.clone()).collect();
        Held { current: ids(current), snn: ids(snn), model, day }
    }

    /// Did a dish of the order land in each list? (current, network)
    pub fn hits(&self, ordered: &[String]) -> (bool, bool) {
        let hit = |l: &[String]| ordered.iter().any(|o| l.contains(o));
        (hit(&self.current), hit(&self.snn))
    }
}

/// The next order: take the hold out of `slot` (it is cleared whatever happens) and say what it
/// came to -- `None` when nothing was held, or the order named no dish.
pub fn settle(slot: &mut Option<Held>, ordered: &[String]) -> Option<(bool, bool, u32)> {
    let h = slot.take()?;
    if ordered.is_empty() {
        return None;
    }
    let (c, s) = h.hits(ordered);
    Some((c, s, h.model))
}
