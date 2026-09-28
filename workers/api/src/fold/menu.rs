//! THE CATALOGUE AS A PROJECTION (R2, `docs/research/2026-09-27-dag-architecture.md`
//! §4 rule 2, §7 R2).
//!
//! WHAT THIS REPLACES. `/api/public/locations/:slug/menu` -- the most requested
//! route in the product -- pulled the whole catalogue image (a SOURCE node,
//! ~0.5 MB on a 165-dish venue) across the hop into the Worker, parsed it,
//! translated it and serialised it, on every request the edge cache missed. The
//! Free plan kills a Worker at 10 ms of CPU and the kills cluster exactly there.
//! The object already holds the bytes; this file is what it answers instead: a
//! DERIVED node, rendered once per catalogue generation and served as bytes.
//!
//! THE KEY IS THE THREE INPUT GENERATIONS (`Gens`): the catalogue, its
//! translations (`i18n`) and the settings (the storefront's feature switches).
//! Every write to any of the three bumps its generation in `put_image_as`, and a
//! memo whose key differs is refolded -- Salsa's revision check reduced to a
//! tuple compare (§3.3). The object also DROPS the memo on those writes, before
//! the write, so a failed write cannot leave one standing over bytes it no
//! longer describes.
//!
//! THE CLOCK IS NOT IN THE MEMO. Whether the venue is open moves with the time,
//! so `status`, `nextOpen` and `closedReason` are decided per request from the
//! `now` the Worker passes down; everything else is rendered once.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::storefront::LocRow;

/// A locale is rendered once and kept; past this many the answer is rendered
/// and not kept, so a stranger asking for a thousand made-up locales cannot
/// grow the object's memory.
pub const LOCALES_KEPT: usize = 8;

/// The input generations a memo was folded from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Gens {
    pub catalog: i64,
    pub i18n: i64,
    pub settings: i64,
}

/// What the deployment adds (secrets): fixed for an object's life.
#[derive(Clone, Default)]
pub struct Rails {
    pub stripe_key: Option<String>,
    pub telegram_bot: Option<String>,
}

/// The venue record, parsed once.
struct Venue {
    raw: Value,
    row: LocRow,
    /// The storefront's `location` block minus the three clock fields.
    base: Value,
}

/// The catalogue projection as §B.4 blocks (row DG7; SPEC-DATALOG-AND-CODEC
/// Part B), folded in the same pass as the JSON, so the two cannot describe
/// different generations. `/fold/menu?block=` and `/fold/products?block=`
/// answer them; the JSON body stays for the browser.
pub struct Blocks {
    pub menu_prices: Vec<u8>,
    pub bom: Vec<u8>,
    pub names: Vec<u8>,
    /// Products the projection could not read (the readers fall back to JSON).
    pub skipped: Vec<String>,
}

fn blocks_of(listed: &[(String, String)]) -> Result<Blocks, String> {
    use dowiz_hub::block::encode::{encode, project};
    let p = project(listed).map_err(|e| e.to_string())?;
    let enc = |b: &dowiz_hub::block::Block| encode(b).map_err(|e| e.to_string());
    Ok(Blocks { menu_prices: enc(&p.menu_prices)?, bom: enc(&p.bom)?, names: enc(&p.names)?, skipped: p.skipped })
}

/// What `/fold/menu` answers.
#[derive(Debug, PartialEq)]
pub enum Answer {
    Body(String),
    NotFound,
    Broken(String),
}

/// The catalogue projection, current at `gens`.
pub struct Memo {
    gens: Gens,
    /// The venue's record AS STORED, whether or not it reads as a `LocRow`:
    /// `/fold/products` and the live estimate read it untyped, as they always did.
    record: Option<String>,
    venue: Result<Option<Venue>, String>,
    cats: Vec<(String, String, i64)>,
    products: Vec<(String, Value)>,
    stored: HashMap<String, String>,
    i18n: Result<Vec<(String, String)>, String>,
    blocks: Result<Blocks, String>,
    stripe_key: Option<String>,
    /// Per locale: the `categories` array and the `warnings` array, as JSON.
    rendered: HashMap<String, (String, String)>,
}

impl Memo {
    /// The memo from the three images' BYTES as the object holds them (`None`:
    /// the image was never written). The same readings the Worker's loaders
    /// made: an absent catalogue or settings image is an empty one, a corrupt
    /// one is an error and NOT an empty one; a corrupt translation table is a
    /// warning on the menus that needed it, as it always was.
    pub fn from_images(
        gens: Gens,
        catalog: Option<&[u8]>,
        i18n: Option<&[u8]>,
        settings: Option<&[u8]>,
        rails: Rails,
    ) -> Result<Memo, String> {
        use dowiz_hub::{catalog::Catalog, settings::Settings, table::Table};
        let catalog = match catalog {
            Some(b) => Catalog::load(b).map_err(|_| "catalogue image is unreadable".to_string())?,
            None => Catalog::create().map_err(|_| "cannot create catalogue".to_string())?,
        };
        let settings = match settings {
            Some(b) => Settings::load(b).map_err(|_| "settings image is unreadable".to_string())?,
            None => Settings::create().map_err(|_| "cannot create settings".to_string())?,
        };
        let i18n = match i18n {
            Some(b) => Table::load(b, crate::hubstore::I18N_BYTES)
                .map(|t| t.all(crate::hubstore::I18N_KIND))
                .map_err(|_| format!("image {} is unreadable", crate::hubstore::IMAGE_I18N)),
            None => Ok(Vec::new()),
        };
        Ok(Memo::build(gens, &catalog, i18n, &settings, rails))
    }

    /// Fold the three images once. `i18n` is the translation table's records
    /// (`hubstore::I18N_KIND`), or why it could not be read.
    pub fn build(
        gens: Gens,
        catalog: &dowiz_hub::catalog::Catalog,
        i18n: Result<Vec<(String, String)>, String>,
        settings: &dowiz_hub::settings::Settings,
        rails: Rails,
    ) -> Memo {
        let record = catalog.location();
        let venue = match &record {
            None => Ok(None),
            Some(stored) => match serde_json::from_str::<LocRow>(stored) {
                Err(e) => Err(format!("catalogue location unreadable: {e}")),
                Ok(row) => {
                    let raw: Value = serde_json::from_str(stored).unwrap_or(json!({}));
                    let base = super::menu_venue::base(&row, &raw, settings, &rails);
                    Ok(Some(Venue { raw, row, base }))
                }
            },
        };
        let mut cats: Vec<(String, String, i64)> = catalog
            .categories()
            .into_iter()
            .filter_map(|(id, j)| {
                let v: Value = serde_json::from_str(&j).ok()?;
                let name = v.get("name").and_then(Value::as_str).unwrap_or("—").to_string();
                Some((id, name, v.get("sortOrder").and_then(Value::as_i64).unwrap_or(0)))
            })
            .collect();
        cats.sort_by_key(|(_, _, sort)| *sort);
        // The KV layout's order, as the Worker used to read it.
        let listed = catalog.products();
        let blocks = blocks_of(&listed);
        let products: Vec<(String, Value)> = listed
            .iter()
            .filter_map(|(id, j)| serde_json::from_str::<Value>(j).ok().map(|v| (id.clone(), v)))
            .collect();
        let stored: HashMap<String, String> = listed.into_iter().collect();
        Memo { gens, record, venue, cats, products, stored, i18n, blocks, stripe_key: rails.stripe_key, rendered: HashMap::new() }
    }

    #[cfg(test)]
    pub fn gens(&self) -> Gens {
        self.gens
    }

    /// The venue's own record as stored, or `null`: what `/fold/venue` answers.
    pub fn venue(&self) -> &str {
        self.record.as_deref().unwrap_or("null")
    }

    /// The storefront's menu for `slug` in `locale` (None: the venue's own) at `now_ms`.
    pub fn menu(&mut self, slug: &str, locale: Option<&str>, now_ms: i64) -> Answer {
        self.menu_as(slug, locale, false, now_ms)
    }

    /// `menu`, and `fresh` is the console's read-back: a customer missing a
    /// name in `locale` reads the English one (`ru -> en -> venue`,
    /// `dowiz_hub::lang::content_fallback`), the console does NOT -- it fills
    /// the dish sheet's translation fields from this answer, and an English
    /// name shown there would be saved back as the Russian one.
    pub fn menu_as(&mut self, slug: &str, locale: Option<&str>, fresh: bool, now_ms: i64) -> Answer {
        let v = match &self.venue {
            Err(e) => return Answer::Broken(e.clone()),
            Ok(None) => return Answer::NotFound,
            Ok(Some(v)) => v,
        };
        if v.row.slug != slug {
            return Answer::NotFound;
        }
        let want = locale.map(str::to_string).unwrap_or_else(|| v.row.default_locale.clone());
        let location = super::menu_venue::at(&v.base, &v.row, &v.raw, now_ms).to_string();
        let second = if fresh { None } else { dowiz_hub::lang::content_fallback(&want, &v.row.default_locale) };
        // One rendering per (language, fallback): the console's and the customer's differ.
        let kept = match second {
            Some(s) => format!("{want}>{s}"),
            None => want.clone(),
        };
        let mut unkept = None;
        if !self.rendered.contains_key(&kept) {
            let own = want == v.row.default_locale || want.is_empty();
            let done = super::menu_venue::render(&self.cats, &self.products, &self.i18n, &want, second, own);
            if self.rendered.len() < LOCALES_KEPT {
                self.rendered.insert(kept.clone(), done);
            } else {
                unkept = Some(done);
            }
        }
        let (cats, warnings) = match &unkept {
            Some(done) => done,
            None => &self.rendered[&kept],
        };
        let key = self.stripe_key.as_ref().map_or(Value::Null, |k| json!(k));
        Answer::Body(format!(
            "{{\"categories\":{cats},\"location\":{location},\"stripePublishableKey\":{key},\"warnings\":{warnings}}}"
        ))
    }

    /// One block of the projection by its §B.4 name (`Ok(None)`: no such
    /// block), or why the projection could not be folded.
    pub fn block(&self, name: &str) -> Result<Option<(&[u8], usize)>, String> {
        let b = self.blocks.as_ref().map_err(Clone::clone)?;
        let bytes = match name {
            "menu_prices" => &b.menu_prices,
            "bom" => &b.bom,
            "names" => &b.names,
            _ => return Ok(None),
        };
        Ok(Some((bytes.as_slice(), b.skipped.len())))
    }

    /// `{"venue": <record|null>, "products": {id: <product as stored>}}` for
    /// the ids asked; an id the catalogue does not have is left out.
    pub fn products(&self, ids: &[String]) -> String {
        let mut out = format!("{{\"venue\":{},\"products\":{{", self.venue());
        let mut first = true;
        for id in ids {
            let Some(p) = self.stored.get(id) else { continue };
            if !first {
                out.push(',');
            }
            first = false;
            out.push_str(&json!(id).to_string());
            out.push(':');
            out.push_str(p);
        }
        out.push_str("}}");
        out
    }
}

/// Is `memo` the fold of the images at `current`? `current` is read off the
/// images the object holds, so asking costs three map lookups and no storage
/// call; a memo that is not current is rebuilt by the caller and replaced.
pub fn is_current(memo: &Option<Memo>, current: Gens) -> bool {
    memo.as_ref().is_some_and(|m| m.gens == current)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod measure;
