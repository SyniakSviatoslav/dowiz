//! Taking a census, writing it down, and asking last night's what it thinks.
//!
//! SEPARATED FROM THE LAW ITSELF (`super::contradictions`) because this half
//! cannot run in a test on this box: it needs the objects. The split is the
//! point — the deciding is testable, and this is only fetching.

use worker::*;

use super::{contradictions, Census, Seal, IMAGE, KIND};


/// Take tonight's census of a venue, carrying last night's seals forward.
///
/// THE ARCHIVES ARE READ AT MOST ONCE EACH. Whatever `before` already sealed is
/// taken as sealed; only an archive this platform has never seen is loaded, and
/// that happens about once a month per venue, on the night a rotation creates
/// it.
pub async fn take(
    place: &crate::hubstore::Place,
    hub: &dowiz_hub::Hub,
    settings: &dowiz_hub::settings::Settings,
    before: Option<&Census>,
    now_ms: i64,
) -> Census {
    let mut seals: Vec<Seal> = Vec::new();
    for id in crate::hubstore::archives_of(settings) {
        if let Some(old) = before.and_then(|c| c.seals.iter().find(|s| s.id == id)) {
            seals.push(old.clone());
            continue;
        }
        match crate::hubstore::archive_seal(place, &id).await {
            Ok(Some((records, tip))) => seals.push(Seal { id, records, tip }),
            // AN ARCHIVE THAT WILL NOT READ IS NOT SEALED AS EMPTY. A zero here
            // would be a lie that the next night compares against, and every
            // night after would agree with it.
            Ok(None) => {
                crate::loud!(&place.ns, Some(&place.venue), "hub.witness", "archive {id} is not there to seal")
            }
            Err(e) => crate::loud!(&place.ns, Some(&place.venue), "hub.witness", "archive {id} refused: {e}"),
        }
    }
    let mut c = Census {
        at_ms: now_ms,
        venue: place.venue.clone(),
        records: hub.len(),
        tip: hub.tip(),
        generation: hub.usage().generation,
        seals,
        total: 0,
        found: Vec::new(),
    };
    c.recount();
    c
}

/// The newest census this platform holds for a venue.
pub async fn last(ns: &ObjectNamespace, venue: &str) -> Result<Option<Census>> {
    let stub = ns.id_from_name(crate::platform_store::PLATFORM)?.get_stub()?;
    let loaded = crate::platform_store::load_log_at(&stub, IMAGE).await?;
    Ok(loaded
        .log
        .about(KIND, Some(venue), 1)
        .into_iter()
        .next()
        .and_then(|e| serde_json::from_str(&e.json).ok()))
}

/// Write one down. ON THE PLATFORM OBJECT, never the venue's: a second account
/// kept in the same place as the first is not a second account.
pub async fn record(ns: &ObjectNamespace, c: &Census) -> Result<()> {
    let stub = ns.id_from_name(crate::platform_store::PLATFORM)?.get_stub()?;
    let venue = c.venue.clone();
    let json = serde_json::to_string(c).map_err(|e| Error::RustError(e.to_string()))?;
    crate::platform_store::with_log_at(&stub, IMAGE, move |log| {
        log.append(KIND, &venue, &json).map_err(|x| Error::RustError(format!("{x:?}")))
    })
    .await
}

/// Where the tip last witnessed is now — the hot log, an archive, or nowhere.
///
/// THE ARCHIVES ARE ONLY ASKED IF THE LOG SAYS NO, and then only the ones
/// created since. On an ordinary night this costs nothing at all; on the night
/// after a rotation it costs one read.
async fn tip_held_by(
    place: &crate::hubstore::Place,
    hub: &dowiz_hub::Hub,
    tip: &str,
    now: &Census,
    before: Option<&Census>,
) -> Option<String> {
    if hub.holds(tip) {
        return Some("log".into());
    }
    let known = |id: &str| before.is_some_and(|c| c.seals.iter().any(|s| s.id == id));
    for s in &now.seals {
        // A seal made tonight carries the tip it was sealed at, which is the
        // common case: the rotation that moved the record wrote it.
        if s.tip.as_deref() == Some(tip) {
            return Some(s.id.clone());
        }
        // AN ARCHIVE THAT EXISTED LAST NIGHT CANNOT HOLD LAST NIGHT'S TIP:
        // that record was in the hot log when the census was taken. Only an
        // archive created since is worth opening, which is what keeps this at
        // one read on the night after a rotation and none on every other.
        if known(&s.id) {
            continue;
        }
        // The record may be anywhere in the archive, not only at its tip --
        // a rotation moves a run of records and the witnessed one is the
        // newest of them, but a second rotation the same night would leave it
        // in the middle. Ask the image.
        if crate::hubstore::archive_holds(place, &s.id, tip).await.unwrap_or(false) {
            return Some(s.id.clone());
        }
    }
    None
}

/// The nightly: take a census, compare it with the last one, write it down.
///
/// RETURNS WHAT IT FOUND rather than deciding what to do about it, so the
/// caller can be loud in its own words and a route can show the same answer
/// without being the thing that causes it.
pub async fn nightly(
    place: &crate::hubstore::Place,
    hub: &dowiz_hub::Hub,
    settings: &dowiz_hub::settings::Settings,
    now_ms: i64,
) -> Result<(Census, Vec<String>)> {
    let before = last(&place.ns, &place.venue).await.unwrap_or(None);
    let now = take(place, hub, settings, before.as_ref(), now_ms).await;
    let mut now = now;
    if let Some(prev) = &before {
        let held = match prev.tip.as_deref() {
            Some(tip) => tip_held_by(place, hub, tip, &now, before.as_ref()).await,
            None => Some("log".into()),
        };
        now.found = contradictions(prev, &now, held.as_deref());
    }
    let found = now.found.clone();
    // WRITTEN EVEN WHEN IT CONTRADICTS. A census withheld because it looked
    // wrong would leave the next night comparing against a record of a state
    // that no longer exists, and every night after would report the same thing
    // for ever.
    record(&place.ns, &now).await?;
    Ok((now, found))
}
