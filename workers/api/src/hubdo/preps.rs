//! THE SEMI-FINISHED PRODUCTS' READS, ANSWERED HERE (R3, `/fold/preps`).
//!
//! `GET /api/owner/preps`, `.../supplies/:id/uses` and `.../products/:id/takes`
//! each pulled the whole catalogue image across the hop to derive a few KB of
//! JSON. The object already holds that image; it derives the answer here with
//! the same pure function (`services::operations::preps::answer`) and hands
//! over the answer alone. The Worker keeps the venue check.
//!
//!   GET /fold/preps?q=list | uses:<supply id> | takes:<product id>

use super::{HubImages, CATALOG_IMAGE};
use dowiz_hub::catalog::Catalog;
use worker::*;

impl HubImages {
    /// See the module. A venue with no catalogue yet answers from an empty
    /// one, as `hubstore::load_catalog` did for the Worker.
    pub(super) async fn fold_preps(&self, req: &Request) -> Result<Response> {
        let url = req.url()?;
        let q = url.query_pairs().find(|(k, _)| k == "q").map(|(_, v)| v.to_string()).unwrap_or_default();
        let cat = match self.image(CATALOG_IMAGE).await? {
            Some((_, bytes)) => match Catalog::load(&bytes) {
                Ok(c) => c,
                Err(_) => return Response::error("catalogue image is unreadable", 500),
            },
            None => Catalog::create().map_err(|_| Error::RustError("cannot create catalogue".into()))?,
        };
        match crate::services::operations::preps::answer(&cat, &q) {
            Ok(v) => Response::from_json(&v),
            Err((status, why)) => Response::error(why, status),
        }
    }
}
