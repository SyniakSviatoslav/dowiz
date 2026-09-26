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

/// The venue's dishes, read from its image. A translation table that cannot
/// be read leaves the venue's own names, which are still a menu.
pub async fn load(place: &crate::hubstore::Place, lang: &str) -> worker::Result<Vec<Dish>> {
    let catalog = crate::hubstore::load_catalog(place).await?;
    let products = catalog.catalog.products();
    let i18n: Vec<(String, String)> =
        match crate::hubstore::load_table(place, crate::hubstore::IMAGE_I18N, crate::hubstore::I18N_BYTES).await {
            Ok(l) => l.table.all(crate::hubstore::I18N_KIND).into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            Err(_) => Vec::new(),
        };
    Ok(dishes(&products, &i18n, lang))
}

#[cfg(test)]
mod tests;
