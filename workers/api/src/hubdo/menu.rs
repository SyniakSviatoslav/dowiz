//! THE CATALOGUE PROJECTION'S TWO ROUTES (R2, `fold::menu`): `/fold/menu` and
//! `/fold/products`, answered from the memo the object keeps per generation of
//! the three images it is folded from.
//!
//! A HIT MAKES NO STORAGE CALL. The key is compared against the generations of
//! the images IN MEMORY: `image()` put every image that exists there when the
//! memo was built, every write replaces the copy (a new generation, so the key
//! no longer matches) or removes it (a failed write: generation 0, same), and
//! `put_image_as` drops the memo before any write to the three. An image that
//! did not exist is 0 in both, which is why a venue with no translations does
//! not pay a storage miss per menu.

use super::{HubImages, CATALOG_IMAGE};
use crate::fold::menu::{Answer, Gens, Memo, Rails};
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::{Call as Request, Reply as Response};

/// The images the menu is folded from. A write to any of them drops the memo.
pub(super) const MENU_INPUTS: [&str; 3] = [CATALOG_IMAGE, crate::hubstore::IMAGE_I18N, crate::hubstore::IMAGE_SETTINGS];

fn json_body(body: String) -> Result<Response> {
    let mut res = Response::ok(body)?;
    res.headers_mut().set("content-type", "application/json")?;
    Ok(res)
}

/// §B.4 blocks travel as their own media type; the JSON stays for the browser.
pub(super) const BLOCK_TYPE: &str = "application/vnd.dowiz.block";

fn bytes_of(image: &Option<(super::Meta, Vec<u8>)>) -> Option<&[u8]> {
    image.as_ref().map(|(_, b)| b.as_slice())
}

impl HubImages {
    /// The input generations as this object holds them, without a storage call.
    pub(super) fn menu_gens_in_memory(&self) -> Gens {
        let mem = self.mem.borrow();
        let g = |id: &str| mem.get(id).map_or(0, |(m, _)| m.generation);
        Gens { catalog: g(MENU_INPUTS[0]), i18n: g(MENU_INPUTS[1]), settings: g(MENU_INPUTS[2]) }
    }

    /// Make sure the memo is current: a hit is three map lookups, a miss reads
    /// the three images (from memory when warm) and folds them once.
    async fn menu_memo(&self) -> Result<()> {
        let now = self.menu_gens_in_memory();
        if crate::fold::menu::is_current(&self.menu.borrow(), now) {
            return Ok(());
        }
        let catalog = self.image(MENU_INPUTS[0]).await?;
        let i18n = self.image(MENU_INPUTS[1]).await?;
        let settings = self.image(MENU_INPUTS[2]).await?;
        let gen = |i: &Option<(super::Meta, Vec<u8>)>| i.as_ref().map_or(0, |(m, _)| m.generation);
        let gens = Gens { catalog: gen(&catalog), i18n: gen(&i18n), settings: gen(&settings) };
        let rails = Rails {
            stripe_key: self.state.secret("STRIPE_PUBLISHABLE_KEY"),
            telegram_bot: self.state.secret("TELEGRAM_BOT_USERNAME"),
        };
        let memo = Memo::from_images(gens, bytes_of(&catalog), bytes_of(&i18n), bytes_of(&settings), rails)
            .map_err(Error::RustError)?;
        *self.menu.borrow_mut() = Some(memo);
        Ok(())
    }

    /// AFTER A WRITE TO ONE OF `MENU_INPUTS` (AX3 early cutoff, `fold/menu/out.rs`;
    /// row `menu` of `hubdo/edges.rs`). `prev` is the memo the write took down.
    /// Warm, the menu is refolded now and succeeds it: the same out bytes keep
    /// its generation and the dependents (the publish: R2 objects, the root, the
    /// record) do not run. Cold, there is nothing to compare: publish, as before.
    /// Logged, never an error -- the write that caused it has landed.
    pub(super) async fn menu_after_write(&self, prev: Option<Memo>) {
        if let Some(prev) = prev {
            if let Err(e) = self.menu_memo().await {
                log_line!("menu: the refold after a write failed: {e}");
            }
            if self.menu.borrow_mut().as_mut().is_some_and(|m| m.succeed(&prev)) {
                return;
            }
        }
        let ok = match self.publish(false).await {
            Ok(_) => true,
            Err(e) => {
                log_line!("publish: {e}");
                false
            }
        };
        if let Some(m) = self.menu.borrow_mut().as_mut() {
            m.mark_published(ok);
        }
    }

    /// The menu output's K64 (16 hex), generation and whether it reached its sink, when the memo is in hand.
    pub(super) fn menu_out(&self) -> serde_json::Value {
        match self.menu.borrow().as_ref() {
            Some(m) => serde_json::json!({ "k64": format!("{:016x}", m.out_key()), "generation": m.out_generation(), "published": m.published() }),
            None => serde_json::Value::Null,
        }
    }

    /// `?block=menu_prices|bom|names|taste` on either route: that block of the
    /// catalogue projection (row DG7), from the same memo as the JSON.
    async fn fold_block(&self, name: &str) -> Result<Response> {
        self.menu_memo().await?;
        let found = match self.menu.borrow().as_ref() {
            Some(m) => m.block(name).map(|b| b.map(|(bytes, skipped)| (bytes.to_vec(), skipped))),
            None => Err("the menu memo is missing".into()),
        };
        match found {
            Ok(Some((bytes, skipped))) => {
                let mut res = Response::from_bytes(bytes)?;
                res.headers_mut().set("content-type", BLOCK_TYPE)?;
                res.headers_mut().set("x-dwb-skipped", &skipped.to_string())?;
                Ok(res)
            }
            Ok(None) => Response::error("no such block: menu_prices, bom, names or taste", 404),
            Err(e) => Response::error(e, 500),
        }
    }

    /// `GET /fold/menu?slug=&locale=&now=[&fresh=1]`: the storefront's menu as bytes.
    pub(super) async fn fold_menu(&self, req: &Request) -> Result<Response> {
        let url = req.url()?;
        let q = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.to_string());
        if let Some(name) = q("block") {
            return self.fold_block(&name).await;
        }
        let (Some(slug), Some(now_ms)) = (q("slug"), q("now").and_then(|n| n.parse::<i64>().ok())) else {
            return Response::error("a menu needs a slug and a clock", 400);
        };
        self.menu_memo().await?;
        let answer = match self.menu.borrow_mut().as_mut() {
            Some(m) => m.menu_as(&slug, q("locale").as_deref(), q("fresh").is_some(), now_ms),
            None => Answer::Broken("the menu memo is missing".into()),
        };
        match answer {
            Answer::Body(b) => json_body(b),
            Answer::NotFound => Response::error("not found", 404),
            Answer::Broken(e) => Response::error(e, 500),
        }
    }

    /// The menu body for `locale` (None: the venue's own) at `now_ms`, or why
    /// not -- what `publish.rs` renders the published set from.
    pub(super) async fn menu_body(&self, slug: &str, locale: Option<&str>, now_ms: i64) -> Result<Option<String>> {
        self.menu_memo().await?;
        let answer = match self.menu.borrow_mut().as_mut() {
            Some(m) => m.menu_as(slug, locale, false, now_ms),
            None => Answer::Broken("the menu memo is missing".into()),
        };
        match answer {
            Answer::Body(b) => Ok(Some(b)),
            Answer::NotFound => Ok(None),
            Answer::Broken(e) => Err(Error::RustError(e)),
        }
    }

    /// The venue's record as the memo holds it (`/fold/venue`'s answer), parsed.
    pub(super) async fn menu_venue(&self) -> Result<Option<serde_json::Value>> {
        self.menu_memo().await?;
        let raw = self.menu.borrow().as_ref().map(|m| m.venue().to_string());
        Ok(raw.and_then(|r| serde_json::from_str(&r).ok()).filter(|v: &serde_json::Value| v.is_object()))
    }

    /// One published block's bytes (`menu_prices` or `names`), from the memo.
    pub(super) async fn menu_block(&self, name: &str) -> Result<Option<Vec<u8>>> {
        self.menu_memo().await?;
        let found = match self.menu.borrow().as_ref() {
            Some(m) => m.block(name).map(|b| b.map(|(bytes, _)| bytes.to_vec())),
            None => Err("the menu memo is missing".into()),
        };
        found.map_err(Error::RustError)
    }

    /// `GET /fold/products?ids=a&ids=b`: the venue record and the products asked
    /// for, as stored -- what the live estimate needs, not the catalogue.
    pub(super) async fn fold_products(&self, req: &Request) -> Result<Response> {
        let url = req.url()?;
        if let Some((_, name)) = url.query_pairs().find(|(k, _)| k == "block") {
            return self.fold_block(&name).await;
        }
        let ids: Vec<String> = url.query_pairs().filter(|(k, _)| k == "ids").map(|(_, v)| v.to_string()).collect();
        self.menu_memo().await?;
        let body = self.menu.borrow().as_ref().map(|m| m.products(&ids));
        match body {
            Some(b) => json_body(b),
            None => Response::error("the menu memo is missing", 500),
        }
    }
}

// ── THE PUBLISHED SHAPE (BN2, `hubdo/publish.rs`) ──────────────────────────
//
// What the object writes to R2 is derived from the SAME body `/fold/menu`
// answers, so the storefront reads the same object from the CDN as it did from
// the Worker. Two cuts are made, and both are about what a cache may keep:
//
//   * the FRAGMENT is the venue's own-language body WITHOUT THE CLOCK. `status`,
//     `nextOpen` and `closedReason` move with the time of day; an immutable
//     object cannot carry them, so the shell derives them from `hours`, `tz`,
//     `ownerStatus` and `deliveryPaused`, which the body already has
//     (`fold::menu_venue::at` is the law; `store/shell.js` mirrors it).
//   * the WORDS of another locale are ONLY the fields that differ from the
//     fragment -- name, description, ingredients per product or category -- so
//     a price edit rewrites the fragment and not one object per language.

/// The `location` fields the clock decides. Stripped from the fragment.
pub(super) const CLOCK_FIELDS: [&str; 3] = ["status", "nextOpen", "closedReason"];
/// The fields a translation can change (`fold::menu_venue::render`'s `said`).
const WORD_FIELDS: [&str; 3] = ["name", "description", "ingredients"];

/// The content address of published bytes: the first 16 hex digits of
/// sha256. Sixty-four bits, like the block schema's `K64`, but a HASH of the
/// content rather than crc32+len: a collision here would pin a stale price
/// into an immutable URL, and 2^-32 per edit is not a rate to publish on.
pub(super) fn k64(bytes: &[u8]) -> String {
    use sha2::Digest;
    let d = sha2::Sha256::digest(bytes);
    d[..8].iter().map(|b| format!("{b:02x}")).collect()
}

/// The fragment: `body` (a `/fold/menu` answer) with the clock taken out of
/// `location`, re-serialised canonically so equal menus are equal bytes.
pub(super) fn fragment_of(body: &str) -> std::result::Result<String, String> {
    let mut v: serde_json::Value = serde_json::from_str(body).map_err(|e| format!("menu body is not JSON: {e}"))?;
    let Some(loc) = v.get_mut("location").and_then(serde_json::Value::as_object_mut) else {
        return Err("menu body has no location".into());
    };
    for f in CLOCK_FIELDS {
        loc.remove(f);
    }
    serde_json::to_string(&v).map_err(|e| e.to_string())
}

/// The words of one locale: for every category and product in `other`, the
/// WORD_FIELDS whose value differs from the same id in `fragment`, plus that
/// locale's `warnings`. `{"words":{id:{field:value}},"warnings":[..]}`.
pub(super) fn words_of(fragment: &str, other: &str) -> std::result::Result<String, String> {
    let base: serde_json::Value = serde_json::from_str(fragment).map_err(|e| format!("fragment is not JSON: {e}"))?;
    let them: serde_json::Value = serde_json::from_str(other).map_err(|e| format!("menu body is not JSON: {e}"))?;
    let mut said: std::collections::BTreeMap<String, serde_json::Value> = Default::default();
    for (id, fields) in named(&base) {
        said.insert(id, fields);
    }
    let mut words = serde_json::Map::new();
    for (id, fields) in named(&them) {
        let mine = said.get(&id);
        let mut diff = serde_json::Map::new();
        for (k, v) in fields.as_object().into_iter().flatten() {
            if mine.and_then(|m| m.get(k)) != Some(v) {
                diff.insert(k.clone(), v.clone());
            }
        }
        if !diff.is_empty() {
            words.insert(id, serde_json::Value::Object(diff));
        }
    }
    let warnings = them.get("warnings").cloned().unwrap_or_else(|| serde_json::json!([]));
    serde_json::to_string(&serde_json::json!({ "words": words, "warnings": warnings })).map_err(|e| e.to_string())
}

/// Every category and product of a body, by id, reduced to its WORD_FIELDS.
fn named(body: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    let mut out = Vec::new();
    let pick = |v: &serde_json::Value| {
        let mut m = serde_json::Map::new();
        for f in WORD_FIELDS {
            if let Some(x) = v.get(f) {
                m.insert(f.to_string(), x.clone());
            }
        }
        serde_json::Value::Object(m)
    };
    for cat in body.get("categories").and_then(serde_json::Value::as_array).into_iter().flatten() {
        if let Some(id) = cat.get("id").and_then(serde_json::Value::as_str) {
            out.push((id.to_string(), pick(cat)));
        }
        for p in cat.get("products").and_then(serde_json::Value::as_array).into_iter().flatten() {
            if let Some(id) = p.get("id").and_then(serde_json::Value::as_str) {
                out.push((id.to_string(), pick(p)));
            }
        }
    }
    out
}

#[cfg(test)]
pub(super) mod tests; // the fixtures are shared with `hubdo/publish/tests.rs`
