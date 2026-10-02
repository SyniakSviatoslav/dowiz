//! THE CATALOGUE'S DERIVED NODES, ANSWERED HERE (BN1, R3, `/fold/catalogue?q=`).
//!
//! Eleven Worker handlers each pulled the whole catalogue image across the hop
//! to read one node of it: a fingerprint, the category list, the set of ids, a
//! dish's photo, the supplies as the voice matcher sees them, the dishes with
//! every name they have, what a social draft is derived from, what the
//! activation check counts, one dish's record, the dishes' names for an import,
//! the dishes as the owner reads them. The object holds the image; each node
//! is derived here with the PURE function its handler is built on, and the
//! node alone crosses. The Worker keeps its auth and venue checks.
//!
//!   GET /fold/catalogue?q=root
//!   GET /fold/catalogue?q=categories            `catalog_edit::categories_view`
//!   GET /fold/catalogue?q=ids                   `owner::catalogue_ids`
//!   GET /fold/catalogue?q=photo&subject=        `verdict::photo_of`
//!   GET /fold/catalogue?q=supplies              `voice::kitchen::supplies`
//!   GET /fold/catalogue?q=dishes&lang=          `voice::menu::dishes` (+ the i18n table)
//!   GET /fold/catalogue?q=posts                 `posts::menu_state`
//!   GET /fold/catalogue?q=activation            `activation::catalogue_facts`
//!   GET /fold/catalogue?q=product&id=
//!   GET /fold/catalogue?q=owned                 `import::owned_dishes`
//!   GET /fold/catalogue?q=owner_products[&id=]  `import::bulk::owner_view`
//!   GET /fold/venue                             the venue's own record (moved here from `hubdo.rs`)

use super::{HubImages, CATALOG_IMAGE};
use serde_json::{json, Value};
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::{Call as Request, Reply as Response};

fn json_body(body: String) -> Result<Response> {
    let mut res = Response::ok(body)?;
    res.headers_mut().set("content-type", "application/json")?;
    Ok(res)
}

impl HubImages {
    /// The translation table's rows, for the dish matcher. A table that cannot
    /// be read leaves the venue's own names, which are still a menu -- exactly
    /// what `voice::menu::load` did when it read the table across the hop.
    async fn i18n_rows(&self) -> Vec<(String, String)> {
        match self.image(crate::hubstore::IMAGE_I18N).await {
            Ok(Some((_, b))) => dowiz_hub::table::Table::load(&b, crate::hubstore::I18N_BYTES)
                .map(|t| t.all(crate::hubstore::I18N_KIND).into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// See the module.
    pub(super) async fn fold_catalogue(&self, req: &Request) -> Result<Response> {
        let url = req.url()?;
        let q = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.to_string());
        let what = q("q").unwrap_or_default();
        let cat = self.catalogue().await?;
        let body: Value = match what.as_str() {
            "root" => json!({ "root": cat.root() }),
            "categories" => crate::catalog_edit::categories_view(&cat),
            "ids" => crate::owner::catalogue_ids(&cat),
            "photo" => json!({ "photo": crate::services::engagement::verdict::photo_of(&cat, &q("subject").unwrap_or_default()) }),
            "supplies" => json!({ "supplies": crate::services::engagement::voice::kitchen::supplies(&cat.supplies()) }),
            "dishes" => {
                let i18n = self.i18n_rows().await;
                json!({ "dishes": crate::services::engagement::voice::menu::dishes(&cat.products(), &i18n, &q("lang").unwrap_or_default()) })
            }
            "posts" => crate::services::engagement::posts::menu_state(&cat),
            "activation" => crate::services::venue::activation::catalogue_facts(&cat),
            "product" => json!({ "product": cat.product(&q("id").unwrap_or_default()) }),
            "owned" => json!({ "owned": crate::services::catalogue::import::owned_dishes(&cat) }),
            "owner_products" => json!({ "products": crate::services::catalogue::import::bulk::owner_view(&cat, q("id").as_deref()) }),
            _ => return Response::error("no such catalogue fold", 404),
        };
        Response::from_json(&body)
    }

    /// THE VENUE'S OWN RECORD, and nothing else in the catalogue (`/fold/venue`).
    ///
    /// The owner's dashboard needs one field from it -- the time zone, so
    /// "today" starts at the venue's midnight rather than UTC's. Reaching that
    /// through `load_catalog` would pull the whole catalogue image, which on a
    /// venue with a real menu is half a megabyte, on every poll. The object
    /// already holds those bytes; parsing them HERE and answering with the
    /// ~1 KB that was asked for is the same move phase 2 made for the log.
    ///
    /// A venue with no catalogue yet answers `null`, which is not an error:
    /// the caller falls back to the default zone and says so.
    pub(super) async fn fold_venue(&self) -> Result<Response> {
        let rec = match self.image(CATALOG_IMAGE).await? {
            Some((_, bytes)) => dowiz_hub::catalog::Catalog::load(&bytes).ok().and_then(|c| c.location()),
            None => None,
        };
        json_body(rec.unwrap_or_else(|| "null".to_string()))
    }
}
