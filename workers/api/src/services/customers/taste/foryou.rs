//! PURE. "FOR YOU" ON THE ORDER PAGE, FROM THE VENUE'S OWN PROFILE OF THE GUEST (W-TASTE2 row 2;
//! operator 2026-10-06, main's recommendation accepted: "server For you -- add one, on the
//! order-tracking page, via taste.dwb top-k, guests with a server profile only, objection respected").
//!
//! THE RULES, each one a test in `foryou_tests.rs`:
//!   * OBJECTION FIRST. A guest who objected (Art. 21, one tap) gets nothing: no read of the
//!     profile, no ranking, the answer says only that it is off.
//!   * A SERVER PROFILE ONLY. No profile (no phone, expired, never ordered with one) or a profile
//!     with no taste axes yet: nothing is shown. The phone's own strip (store/taste.js) is a
//!     different thing and stays as it is.
//!   * THE TASTE BLOCK ONLY. Dishes come from `dowiz_hub::block::taste::top_k` over the venue's
//!     `taste.dwb`: on sale, the card's allergens excluded BY MASK before any arithmetic (an
//!     undeclared dish is excluded as soon as one allergen is on the card), integer per-mille
//!     cosine, ties by id. The dishes of THIS order are left out: the guest has just had them.
//!   * IT SHOWS DISHES, NEVER A NUMBER. No similarity figure, no segment, no amount: the answer is
//!     ids and the two taste words that explain them, the same words the guest's own view shows.

use std::collections::BTreeMap;

use dowiz_hub::block::{taste, view::View};
use serde_json::{json, Value};

use super::{senses, Profile};

pub const CONTRACT: &str = "order.for-you.v1";
/// How many dishes the order page shows.
pub const K: usize = 3;

/// Why nothing is shown, or that something is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The guest objected: nothing read, nothing ranked.
    Off,
    /// No server profile for this guest (no phone, expired, or none yet).
    NoProfile,
    /// A profile, but no taste axes in it yet (dishes without a declared taste were ordered).
    NoTaste,
    /// Ranked; `items` may still be empty when no dish on sale shares a taste.
    Shown,
}

impl State {
    pub fn word(self) -> &'static str {
        match self {
            State::Off => "off",
            State::NoProfile => "no-profile",
            State::NoTaste => "no-taste",
            State::Shown => "shown",
        }
    }
}

/// The vector the dishes are ranked against: the venue's own decayed sense weights, or -- for a
/// profile that holds none yet -- the device's last aggregated vector, which the guest sent.
pub fn want_of(p: &Profile) -> BTreeMap<String, i64> {
    let own: BTreeMap<String, i64> = p.sense.iter().filter(|(_, w)| **w > 0).map(|(k, w)| (k.clone(), *w)).collect();
    if !own.is_empty() {
        return own;
    }
    p.device.as_ref().map(|d| d.sense.iter().filter(|(_, w)| **w > 0).map(|(k, w)| (k.clone(), *w)).collect()).unwrap_or_default()
}

/// The answer for one order page. `block` is the venue's `taste.dwb` (`None`: the catalogue has
/// none, so nothing is ranked); `card` the allergen codes on the guest's card; `this_order` the
/// dish ids of the order being tracked.
/// `Err` names why the block could not be read (the route logs it and shows nothing).
pub fn answer(objected: bool, p: Option<&Profile>, block: Option<&[u8]>, card: &[String], this_order: &[String]) -> Result<Value, String> {
    let out = |st: State, items: Vec<Value>, because: Vec<String>| json!({ "contract": CONTRACT, "state": st.word(), "items": items, "because": because });
    if objected {
        return Ok(out(State::Off, Vec::new(), Vec::new()));
    }
    let Some(p) = p else { return Ok(out(State::NoProfile, Vec::new(), Vec::new())) };
    let want = want_of(p);
    if want.is_empty() {
        return Ok(out(State::NoTaste, Vec::new(), Vec::new()));
    }
    let Some(bytes) = block else { return Ok(out(State::Shown, Vec::new(), Vec::new())) };
    let view = View::new(bytes).map_err(|e| format!("taste.dwb: {e:?}"))?;
    let codes: Vec<&str> = card.iter().map(String::as_str).collect();
    let got = taste::top_k(&view, &want, taste::avoid_mask(&codes), K + this_order.len()).map_err(|e| format!("taste.dwb: {e:?}"))?;
    let items: Vec<Value> = got.into_iter().filter(|(id, _)| !this_order.contains(id)).take(K).map(|(id, _)| json!({ "id": id })).collect();
    let because = if p.sense.is_empty() { Vec::new() } else { senses::because(p) };
    Ok(out(State::Shown, items, because))
}

#[cfg(test)]
#[path = "foryou_tests.rs"]
mod tests;
