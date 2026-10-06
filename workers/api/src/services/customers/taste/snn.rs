//! W-SNN row 3: THE SHEAF NETWORK BESIDE THE VENUE'S CURRENT RANKER, IN SHADOW.
//!
//! WHERE IT RUNS. The server's own "For you" (W-TASTE: `block::taste` top-k over a guest's server
//! profile, on the order page) is the ranker this shadows -- `compare` asks it first, exactly as
//! `dowiz_hub::block::taste::top_k_json` answers, then asks the network the same question about the
//! same dishes. It is called from the guest's own order-page read (`GET /api/order/:id/taste`,
//! `taste_routes::guest_view`) AFTER that route has checked the objection and found a profile, so a
//! guest who objected, or who has no profile, costs nothing and is counted nowhere.
//!
//! WHAT IS KEPT: one aggregate per venue in its own image (`IMAGE_SNN`, not personal: counts only,
//! `dowiz_hub::snn::shadow::Tally`) -- comparisons, first-dish agreement, top-3 overlap, how often
//! the network could not run. No key, no dish list, no day per guest.
//!
//! WHAT THE GUEST SEES IS NOT CHANGED: the route's answer is the same bytes with the shadow on or off
//! (`snn_tests.rs`). `on` is honoured by `compare` (its first value), for the day the server's
//! "For you" shows a list; until then nothing shows either ranker.
//!
//! THE SWITCH is the settings key `snn` = shadow | on | off (default shadow, `Mode::parse`).
//! `off` loads nothing and runs nothing. Never money: no price, no code, no fee is read here.

use std::collections::BTreeMap;

use dowiz_hub::snn::{self, shadow::{decide, Mode, Outcome, Tally}};

use super::{Profile, IMAGE_TASTE, KIND as TASTE_KIND, TASTE_BYTES};
use dowiz_hub::snn::quality::Held;
use crate::hubstore::Place;

/// The venue's shadow counter: its own small image (privacy registry row "snn", not personal).
pub const IMAGE_SNN: &str = "snn";
pub const SNN_BYTES: usize = dowiz_hub::CEILING_BYTES;
pub const KIND: &str = "tally";
pub const ID: &str = "venue";
/// The settings key of the switch.
pub const SETTING: &str = "snn";
/// How many dishes the two lists are compared over (the strip's "taste" row shows about this many).
pub const K: usize = 3;

/// PURE. The list to show and what the comparison came to, for one guest's server profile over
/// the venue's `(id, product JSON)`. The current list is the server cosine over the profile's
/// sense vector; the network sees the sense vector, category and tag weights -- nothing else.
pub fn compare(mode: Mode, products: &[(String, String)], p: &Profile, k: usize) -> (Vec<(String, i64)>, Outcome) {
    let current = dowiz_hub::block::taste::top_k_json(products, &p.sense, 0, k);
    decide(mode, current, k, || {
        let model = snn::shipped()?;
        let menu = snn::Menu::from_products(products, 0);
        let g = snn::Guest::from_maps(&menu, &p.sense, &p.cats, &p.tags, &BTreeMap::new());
        Ok(snn::rank(&model, &menu, &g, k, &[]))
    })
}

/// PURE. The tally as stored, read; anything unreadable starts again from zero.
pub fn tally_of(t: &dowiz_hub::table::Table) -> Tally {
    t.get(KIND, ID).and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default()
}

/// The venue's switch, from its settings (absent or unreadable: the default, shadow).
pub fn mode_of(s: &dowiz_hub::settings::Settings) -> Mode {
    Mode::parse(s.get(SETTING).as_deref())
}

/// Run the shadow for one guest, count it, and HOLD both top-3s beside their profile for the
/// quality check at their next order. NEVER FAILS THE CALLER and returns nothing the caller shows:
/// a storage hiccup loses one count, never the guest's page. The caller has checked the objection.
pub async fn observe(place: &Place, key: &str, p: &Profile, day: i64) {
    let Ok(settings) = crate::hubstore::load_settings(place).await else { return };
    let mode = mode_of(&settings.settings);
    if mode == Mode::Off {
        return;
    }
    let Ok(products) = venue_products(place).await else { return };
    let current = dowiz_hub::block::taste::top_k_json(&products, &p.sense, 0, K);
    let (snn_top, outcome) = network_top(&products, p);
    count(place, &outcome_of(mode, current.clone(), snn_top.clone(), outcome), day).await;
    if let Some(net) = snn_top {
        hold(place, key, Held::of(&current, &net, K, model_id(), day)).await;
    }
}

/// PURE. The network's top-k, or why it could not run (as `compare` would say it).
fn network_top(products: &[(String, String)], p: &Profile) -> (Option<Vec<(String, i64)>>, Option<Outcome>) {
    let mut got = None;
    let (_, o) = decide(Mode::Shadow, Vec::new(), K, || {
        let model = snn::shipped()?;
        let menu = snn::Menu::from_products(products, 0);
        let g = snn::Guest::from_maps(&menu, &p.sense, &p.cats, &p.tags, &BTreeMap::new());
        let r = snn::rank(&model, &menu, &g, K, &[]);
        got = Some(r.clone());
        Ok(r)
    });
    (got, matches!(o, Outcome::Unusable(_)).then_some(o))
}

/// PURE. The comparison of the two lists (or the reason the network could not run).
fn outcome_of(mode: Mode, current: Vec<(String, i64)>, snn_top: Option<Vec<(String, i64)>>, unusable: Option<Outcome>) -> Outcome {
    match (unusable, snn_top) {
        (Some(o), _) => o,
        (None, net) => decide(mode, current, K, || Ok(net.unwrap_or_default())).1,
    }
}

fn model_id() -> u32 {
    snn::shipped().map(|m| m.id).unwrap_or(0)
}

/// PURE. Put the hold on the profile; answers whether that changed it (no write otherwise).
pub fn hold_in(p: &mut Profile, h: Held) -> bool {
    if p.snn_held.as_ref() == Some(&h) {
        return false;
    }
    p.snn_held = Some(h);
    true
}

/// PURE. The record to write for a hold: `None` when no profile is stored (holding never creates
/// one -- a guest who objected has none) or when the same hold is already there.
pub fn held_record(stored: Option<Profile>, h: Held) -> Option<Profile> {
    let mut p = stored?;
    hold_in(&mut p, h).then_some(p)
}

/// Write the hold beside the guest's profile -- ONLY if a profile is stored: holding never creates
/// one, so a guest who objected (profile deleted) stores nothing.
pub async fn hold(place: &Place, key: &str, h: Held) {
    let k = key.to_string();
    let _ = crate::hubstore::with_table(place, IMAGE_TASTE, TASTE_BYTES, move |t| {
        let Some(p) = held_record(t.get(TASTE_KIND, &k).as_deref().and_then(super::parse), h.clone()) else { return Ok(()) };
        let json = serde_json::to_string(&p).map_err(|e| worker::Error::RustError(format!("snn hold: {e}")))?;
        t.put(TASTE_KIND, &k, &json, &[], &[]).map_err(|e| worker::Error::RustError(format!("snn hold: {e:?}")))
    })
    .await;
}

/// PURE. At the guest's next order (inside the profile write, `taste_routes::after`): take the
/// hold out of the profile and say what it came to -- (current hit, network hit, model).
pub fn settle_profile(p: &mut Profile, lines: &[super::Line]) -> Option<(bool, bool, u32)> {
    let ordered: Vec<String> = lines.iter().filter(|l| l.qty > 0 && !l.id.is_empty()).map(|l| l.id.clone()).collect();
    dowiz_hub::snn::quality::settle(&mut p.snn_held, &ordered)
}

/// PURE. The hold as the guest's own export lists it (Art. 15, main's decision 2026-10-06): the
/// two top-3s by id and name, labelled as a temporary check deleted at the next order.
pub fn held_view(h: &Held, names: &std::collections::HashMap<String, serde_json::Value>) -> serde_json::Value {
    let name = |id: &str| names.get(id).and_then(|p| p.get("name")).map(|n| n.as_str().map(str::to_string).unwrap_or_else(|| n.as_object().and_then(|o| o.values().find_map(|v| v.as_str().map(str::to_string))).unwrap_or_default())).unwrap_or_default();
    let rows = |l: &[String]| l.iter().map(|id| serde_json::json!({ "id": id, "name": name(id) })).collect::<Vec<_>>();
    serde_json::json!({ "current": rows(&h.current), "network": rows(&h.snn), "day": h.day, "temporary": true, "deletedAt": "next_order" })
}

/// The hold for the guest's export, with the dishes' names (one `/fold/products` ask for at most
/// six ids; without names on a failed ask, never without the hold).
pub async fn held_export(place: &Place, h: &Held) -> serde_json::Value {
    let ids: Vec<String> = h.current.iter().chain(&h.snn).cloned().collect();
    let names = crate::fold::menu_edge::products(place, &ids).await.map(|(_, m)| m).unwrap_or_default();
    held_view(h, &names)
}

/// Count one settled hold in the venue's tally.
pub async fn count_settled(place: &Place, s: (bool, bool, u32), day: i64) {
    let _ = crate::hubstore::with_table(place, IMAGE_SNN, SNN_BYTES, move |t| {
        let mut tally = tally_of(t);
        tally.settle(s.0, s.1, day, s.2);
        let rec = serde_json::to_string(&tally).map_err(|e| worker::Error::RustError(format!("snn: {e}")))?;
        t.put(KIND, ID, &rec, &[], &[]).map_err(|e| worker::Error::RustError(format!("snn: {e}")))
    })
    .await;
}

/// The venue's dishes as stored, from the object's DERIVED nodes (R2, `hubdo/menu.rs`): the ids
/// from the `taste` block, then `/fold/products` for those ids. The catalogue image never crosses
/// the hop (tools/gates/dataflow.sh). Sorted by id, so the answer does not depend on a map's order.
pub async fn venue_products(place: &Place) -> worker::Result<Vec<(String, String)>> {
    let req = crate::wire::Call::new("https://hub/fold/products?block=taste", worker::Method::Get)?;
    let mut res = place.stub()?.fetch_with_request(req).await?;
    if res.status_code() != 200 {
        return Err(worker::Error::RustError(format!("snn: the taste block answered {}", res.status_code())));
    }
    let bytes = res.bytes().await?;
    let view = dowiz_hub::block::view::View::new(&bytes).map_err(|e| worker::Error::RustError(format!("snn: {e:?}")))?;
    let ids: Vec<String> = (0..view.n()).filter_map(|i| view.str_at(i).map(str::to_string)).collect();
    let (_, stored) = crate::fold::menu_edge::products(place, &ids).await?;
    let mut out: Vec<(String, String)> = stored.into_iter().map(|(id, v)| (id, v.to_string())).collect();
    out.sort();
    Ok(out)
}

/// Add one outcome to the venue's tally.
pub async fn count(place: &Place, o: &Outcome, day: i64) {
    let model = snn::shipped().map(|m| m.id).unwrap_or(0);
    let o = o.clone();
    let _ = crate::hubstore::with_table(place, IMAGE_SNN, SNN_BYTES, move |t| {
        let mut tally = tally_of(t);
        tally.add(&o, day, model);
        let rec = serde_json::to_string(&tally).map_err(|e| worker::Error::RustError(format!("snn: {e}")))?;
        t.put(KIND, ID, &rec, &[], &[]).map_err(|e| worker::Error::RustError(format!("snn: {e}")))
    })
    .await;
}

/// What the owner's health shows: the switch, the counts, the two agreements per mille, the model.
pub fn health_json(mode: Mode, t: &Tally) -> serde_json::Value {
    serde_json::json!({
        "mode": mode.as_str(), "model": t.model, "compared": t.compared, "unusable": t.unusable,
        "top1Pm": t.top1_pm(), "overlapPm": t.overlap_pm(), "k": K, "sinceDay": t.since_day, "lastDay": t.last_day,
        "settled": t.settled, "currentHitPm": t.hit_pm().0, "snnHitPm": t.hit_pm().1,
    })
}

#[cfg(test)]
#[path = "snn_tests.rs"]
mod tests;
