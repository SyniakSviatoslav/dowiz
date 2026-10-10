//! PUBLISH ON GENERATION (BN2; cost map A1/C3): the storefront's read path off
//! the Worker. Every write to one of the menu's three inputs ends here, and
//! the object writes what the storefront needs to R2 as IMMUTABLE,
//! CONTENT-ADDRESSED objects under `v/<slug>/`, then one small root:
//!
//!   `<k64>.json`   the FRAGMENT: the own-language menu body without the clock
//!   `<k64>.json`   the WORDS of each other locale: only what differs
//!   `<k64>.dwb`    the `menu_prices` and `names` blocks (DG7)
//!   `<k64>.json`   the list of photos copied below
//!   `m/<name>`     each photo the menu shows, copied from KV once
//!   `manifest.json` (`max-age=30`) naming all of the above
//!
//! WHY ONLY WHAT CHANGED. R2 Class A operations are the cost (1 M/month on
//! Free); a venue editing ten times a day must not rewrite its photographs.
//! The object keeps a RECORD of what it published (`published` in its storage:
//! logical name -> object key) and `plan` writes an object only when its key --
//! its content hash -- is not the one on record. A price edit is therefore the
//! fragment, the prices block and the manifest: three writes, however many
//! languages the venue speaks (`publish/tests.rs`).
//!
//! WHY IT NEVER FAILS THE WRITE. A publish that cannot reach R2 leaves the
//! record as it was, so the next write diffs against the truth and writes what
//! the failed one did not. The storefront's shell falls back to the Worker's
//! menu route whenever the manifest is missing or stale (`store/shell.js`), so
//! a venue whose publish failed still renders -- through the Worker, as today.
//!
//! WITHOUT THE `CDN` BINDING NOTHING IS WRITTEN AND NOTHING FAILS: the binding
//! is commented in `wrangler.toml` until the bucket and its domain exist (an
//! operator step, written there), and `sink()` answers `None` until then.

use super::HubImages;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::{Call as Request, Reply as Response};

mod sink; // the bucket and the photo store, live or recorded (tests), `hubdo/publish/sink.rs`
use sink::{Photos, Sink};
#[cfg(test)]
use sink::mem;

/// The R2 binding (`wrangler.toml`).
pub(super) const BINDING: &str = "CDN";
/// The object's storage key for what it last published.
const RECORD: &str = "published";
/// Content-addressed objects never change: a year, immutable.
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
/// The root is the one mutable object: the storefront's freshness window.
pub(super) const ROOT_CACHE: &str = "public, max-age=30";
const JSON: &str = "application/json";
/// The blocks published, in order. `bom` is the venue's recipes: not public.
const BLOCKS: [&str; 2] = ["menu_prices", "names"];

/// What the object last published.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub(super) struct Published {
    /// The menu's input generations: catalogue, translations, settings.
    pub generation: [i64; 3],
    /// Logical name (`fragment`, `words/en`, `block/names`, `media`) -> object key.
    pub objects: BTreeMap<String, String>,
    /// Photo names (`<sha256>.<ext>`) copied to `m/`, once each.
    pub media: BTreeSet<String>,
}

/// One object of a generation's set.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Object {
    pub name: String,
    pub key: String,
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
}

impl Object {
    fn new(name: impl Into<String>, bytes: Vec<u8>, ext: &str, content_type: &'static str) -> Self {
        let key = format!("{}.{ext}", super::menu::k64(&bytes));
        Object { name: name.into(), key, bytes, content_type }
    }
    pub(super) fn json(name: impl Into<String>, body: String) -> Self {
        Self::new(name, body.into_bytes(), "json", JSON)
    }
    pub(super) fn block(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self::new(name, bytes, "dwb", super::menu::BLOCK_TYPE)
    }
}

/// The objects of `next` whose key is not the one `prev` has on record.
pub(super) fn plan<'a>(prev: &Published, next: &'a [Object]) -> Vec<&'a Object> {
    next.iter().filter(|o| prev.objects.get(&o.name) != Some(&o.key)).collect()
}

/// The photo names a fragment and a venue record show: every `/media/<name>`
/// in `imageUrl`, `imageUrlSmall` and `logoUrl`, under the media route's own
/// charset rule, sorted and once each.
pub(super) fn media_names(fragment: &str, venue: &serde_json::Value) -> Vec<String> {
    let mut out = BTreeSet::new();
    let mut take = |v: Option<&serde_json::Value>| {
        if let Some(name) = v.and_then(serde_json::Value::as_str).and_then(|u| u.strip_prefix("/media/")) {
            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') && !name.is_empty() && name.len() <= 80 {
                out.insert(name.to_string());
            }
        }
    };
    take(venue.get("logo_url"));
    if let Ok(body) = serde_json::from_str::<serde_json::Value>(fragment) {
        take(body.pointer("/location/logoUrl"));
        for cat in body.get("categories").and_then(serde_json::Value::as_array).into_iter().flatten() {
            for p in cat.get("products").and_then(serde_json::Value::as_array).into_iter().flatten() {
                take(p.get("imageUrl"));
                take(p.get("imageUrlSmall"));
            }
        }
    }
    out.into_iter().collect()
}

/// The root: what the shell reads first. Every name in it is an immutable key.
pub(super) fn manifest_of(slug: &str, gens: [i64; 3], default: &str, locales: &[String], set: &[Object]) -> String {
    let key = |name: &str| set.iter().find(|o| o.name == name).map(|o| o.key.clone());
    let mut words = serde_json::Map::new();
    let mut blocks = serde_json::Map::new();
    for o in set {
        if let Some(l) = o.name.strip_prefix("words/") {
            words.insert(l.to_string(), serde_json::json!(o.key));
        }
        if let Some(b) = o.name.strip_prefix("block/") {
            blocks.insert(b.to_string(), serde_json::json!(o.key));
        }
    }
    serde_json::json!({
        "v": 1, "slug": slug,
        "gens": { "catalog": gens[0], "i18n": gens[1], "settings": gens[2] },
        "default": default, "locales": locales,
        "fragment": key("fragment"), "words": words, "blocks": blocks, "media": key("media"),
    })
    .to_string()
}

impl HubImages {
    fn sink(&self) -> Option<Sink> {
        match self.state.env() {
            Some(env) => env.bucket(BINDING).ok().map(Sink::Live),
            #[cfg(test)]
            None => mem::TEST_BUCKET.with(|b| b.borrow().clone()).map(Sink::Mem),
            #[cfg(not(test))]
            None => None,
        }
    }

    fn photos(&self) -> Option<Photos> {
        match self.state.env() {
            Some(env) => env.kv("MEDIA").ok().map(Photos::Live),
            #[cfg(test)]
            None => mem::TEST_BUCKET.with(|b| b.borrow().clone()).map(Photos::Mem),
            #[cfg(not(test))]
            None => None,
        }
    }

    async fn record(&self) -> Result<Published> {
        Ok(self.state.storage().get::<Published>(RECORD).await?.unwrap_or_default())
    }

    /// `GET /fold/publish`: the record and whether publishing is on.
    /// `POST /fold/publish[?all=1]`: publish now; `all` rewrites every object.
    pub(super) async fn publish_route(&self, req: &Request) -> Result<Response> {
        let enabled = self.sink().is_some();
        if req.method() == Method::Post {
            let all = req.url()?.query_pairs().any(|(k, _)| k == "all");
            return match self.publish(all).await? {
                Some(written) => Response::from_json(&serde_json::json!({ "written": written })),
                None => Response::error("publishing is off: no CDN binding or no venue", 503),
            };
        }
        let record = self.record().await?;
        let slug = self.menu_venue().await?.and_then(|v| v.get("slug").and_then(serde_json::Value::as_str).map(str::to_string));
        Response::from_json(&serde_json::json!({
            "enabled": enabled,
            "manifest": slug.map(|s| format!("v/{s}/manifest.json")),
            "published": record,
            "menu": self.menu_out(), // AX3: the output's K64 and generation (`hubdo/edges.rs`)
        }))
    }

    /// Publish the current generation. `Ok(None)`: nothing to publish to or
    /// from (no binding, no venue record yet). `Ok(Some(n))`: objects written,
    /// the manifest included. `everything` ignores the record.
    pub(super) async fn publish(&self, everything: bool) -> Result<Option<usize>> {
        let Some(sink) = self.sink() else { return Ok(None) };
        let Some(venue) = self.menu_venue().await? else { return Ok(None) };
        let slug = venue.get("slug").and_then(serde_json::Value::as_str).unwrap_or("").to_string();
        if slug.is_empty() {
            return Ok(None);
        }
        let default = venue.get("default_locale").and_then(serde_json::Value::as_str).unwrap_or("sq").to_string();
        let mut locales: Vec<String> = venue
            .get("supported_locales")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();
        locales.retain(|l| l != &default);
        locales.insert(0, default.clone());
        // NO CLOCK: the fragment has the clock cut out and the words never had one, so any instant renders
        // the same bytes; `0` says so, and this file decides nothing from the time (tools/gates/clock.sh).
        let Some(own) = self.menu_body(&slug, None, 0).await? else { return Ok(None) };
        let fragment = super::menu::fragment_of(&own).map_err(Error::RustError)?;
        let mut set = vec![Object::json("fragment", fragment.clone())];
        for l in locales.iter().skip(1) {
            if let Some(body) = self.menu_body(&slug, Some(l.as_str()), 0).await? {
                set.push(Object::json(format!("words/{l}"), super::menu::words_of(&fragment, &body).map_err(Error::RustError)?));
            }
        }
        for name in BLOCKS {
            if let Some(bytes) = self.menu_block(name).await? {
                set.push(Object::block(format!("block/{name}"), bytes));
            }
        }
        let prev = if everything { Published::default() } else { self.record().await? };
        // PHOTOS ARE COPIED ONCE. Their names are content hashes already; one
        // that is on record is never put again, and one KV does not have is
        // left to the Worker's `/media/` route (it is not in the list below).
        let wanted = media_names(&fragment, &venue);
        let mut copied = prev.media.clone();
        let mut written = 0usize;
        if let Some(photos) = self.photos() {
            for name in &wanted {
                if copied.contains(name) {
                    continue;
                }
                if let Some((bytes, kind)) = photos.get(name).await? {
                    sink.put(&format!("v/{slug}/m/{name}"), bytes, &kind, IMMUTABLE).await?;
                    copied.insert(name.clone());
                    written += 1;
                }
            }
        }
        let listed: Vec<&String> = wanted.iter().filter(|n| copied.contains(*n)).collect();
        set.push(Object::json("media", serde_json::to_string(&listed).map_err(|e| Error::RustError(e.to_string()))?));

        let g = self.menu_gens_in_memory();
        let generation = [g.catalog, g.i18n, g.settings];
        let todo = plan(&prev, &set);
        for o in &todo {
            sink.put(&format!("v/{slug}/{}", o.key), o.bytes.clone(), o.content_type, IMMUTABLE).await?;
            written += 1;
        }
        // THE ROOT LAST, and only when something under it moved: an object a
        // reader can reach is on the bucket before the name that reaches it.
        if !todo.is_empty() || prev.generation != generation || everything {
            let manifest = manifest_of(&slug, generation, &default, &locales, &set);
            sink.put(&format!("v/{slug}/manifest.json"), manifest.into_bytes(), JSON, ROOT_CACHE).await?;
            written += 1;
        }
        let record = Published {
            generation,
            objects: set.iter().map(|o| (o.name.clone(), o.key.clone())).collect(),
            media: copied,
        };
        self.state.storage().put(RECORD, &record).await?;
        Ok(Some(written))
    }
}

#[cfg(test)]
mod tests;
