//! The menu as the dish matcher sees it: every dish with every name it has.
//!
//! `dishes` is PURE (the catalogue's `(id, json)` rows and the translation
//! table's `(key, value)` rows in, dishes out); `load` is the one read, and
//! reads only when an utterance names a dish -- "table 5 paid cash" never pays
//! for a menu read.

use super::dish::Dish;
use serde_json::Value;

/// The name the speaker hears FIRST: in their language when the venue wrote
/// one, else the venue's own. The rest follow, so a dish said in any of the
/// three is still found.
pub fn dishes(products: &[(String, String)], i18n: &[(String, String)], lang: &str) -> Vec<Dish> {
    let lang2: String = lang.chars().take(2).collect();
    products
        .iter()
        .filter_map(|(id, json)| {
            let p: Value = serde_json::from_str(json).ok()?;
            let own = p.get("name").and_then(Value::as_str).unwrap_or("").trim().to_string();
            // `<locale>/<entity_type>/<id>/name`, `hubstore::i18n_key`.
            let mut mine: Option<String> = None;
            let mut others: Vec<String> = Vec::new();
            for (k, v) in i18n {
                let mut parts = k.splitn(4, '/');
                let (Some(loc), Some(_), Some(eid), Some("name")) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
                    continue;
                };
                if eid != id || v.trim().is_empty() {
                    continue;
                }
                if loc == lang2 && mine.is_none() {
                    mine = Some(v.trim().to_string());
                } else {
                    others.push(v.trim().to_string());
                }
            }
            let mut names: Vec<String> = mine.into_iter().collect();
            for n in std::iter::once(own).chain(others) {
                if !n.is_empty() && !names.contains(&n) {
                    names.push(n);
                }
            }
            (!names.is_empty()).then(|| Dish {
                id: id.clone(),
                names,
                available: p.get("available").and_then(Value::as_bool).unwrap_or(true),
            })
        })
        .collect()
}

/// The venue's dishes, from its object (BN1, `/fold/catalogue?q=dishes`,
/// which runs `dishes` over the catalogue and the translation table it holds).
/// A translation table that cannot be read leaves the venue's own names,
/// which are still a menu.
pub async fn load(place: &crate::hubstore::Place, lang: &str) -> worker::Result<Vec<Dish>> {
    let mut v = crate::fold::ask::catalogue(place, &format!("q=dishes&lang={}", crate::mcp::enc(lang))).await?;
    serde_json::from_value(v["dishes"].take()).map_err(|e| worker::Error::RustError(format!("dishes: {e}")))
}

#[cfg(test)]
mod tests;
