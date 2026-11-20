//! ONE SITE, SEVERAL ADDRESSES: a venue's slug that opens another venue.
//!
//! `SLUG_ALIASES` in `wrangler.toml` is a comma-separated list of `from=to`
//! slugs: the host `<from>.dowiz.org` and the public paths naming `from` reach
//! the venue `to` names, so the two addresses are one site with one database.
//! The venue `from` used to name is RETIRED: a staff, owner or courier token
//! that still carries its id is refused (401), so every console signs in again
//! and lands on the venue the address now opens. A guest's order token is not
//! refused: that order lives where it was placed and its tracking keeps working.
//!
//! `from` must be a venue whose id is its slug (how `platform::create_hub`
//! makes them): the retired id is read from the slug.

/// The pairs of a spec, trimmed and lower-cased; malformed items are skipped. PURE.
pub fn pairs(spec: &str) -> Vec<(String, String)> {
    spec.split(',')
        .filter_map(|item| item.split_once('='))
        .map(|(from, to)| (from.trim().to_ascii_lowercase(), to.trim().to_ascii_lowercase()))
        .filter(|(from, to)| !from.is_empty() && !to.is_empty() && from != to)
        .collect()
}

/// The slug `slug` opens: its alias's target, or itself. One hop. PURE.
pub fn target(spec: &str, slug: &str) -> String {
    let s = slug.trim().to_ascii_lowercase();
    pairs(spec).into_iter().find(|(from, _)| *from == s).map(|(_, to)| to).unwrap_or_else(|| slug.to_string())
}

/// Is `venue` an id an alias retired? PURE.
pub fn retired(spec: &str, venue: &str) -> bool {
    let v = venue.trim().to_ascii_lowercase();
    pairs(spec).iter().any(|(from, _)| *from == v)
}

/// The refusal a request naming a retired venue gets: sign in again.
pub fn refusal() -> crate::wire::Reply {
    crate::wire::Reply::error("this venue now opens at another address: sign in again", 401).unwrap()
}

/// A path's parameters with its `slug` replaced by the slug its address opens:
/// every handler that reads the path's slug, not only `Place::of_slug`, sees
/// one site. A menu handler passed the alias on to the venue's object, which
/// answered 404, while only `of_slug` knew it.
pub fn params(mut p: Vec<(String, String)>, env: &crate::edge::Env) -> Vec<(String, String)> {
    let spec = spec(env);
    for (_, v) in p.iter_mut().filter(|(k, _)| k == "slug") {
        *v = target(&spec, v);
    }
    p
}

/// A retired venue's till link, forgotten with its password (the owner's
/// "disconnect", `ebills::glue::apply_config`): the till's sales stop landing in
/// a venue no address opens. Its owner can no longer reach it to do this.
pub async fn forget_till(place: &crate::hubstore::Place) {
    let off = serde_json::json!({ "enabled": false, "pos_id": 0, "user": "" });
    match crate::command::send::<_, serde_json::Value>(place, "ebills/config", &off).await {
        Ok(_) => log_line!("ebills {}: retired by an alias, its till link is forgotten", place.venue),
        Err((s, m)) => log_error!("ebills {}: the retired venue's till link could not be forgotten: {s} {m}", place.venue),
    }
}

/// The deployment's spec (`SLUG_ALIASES`), or none.
pub fn spec(env: &crate::edge::Env) -> String {
    env.var("SLUG_ALIASES").map(|v| v.to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests;
