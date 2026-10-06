//! THE BASKET'S CATALOGUE NODES, ANSWERED HERE (BN1, R3, `/fold/basket`), and
//! the recipes a room command moves the shelf with.
//!
//! A placement, the promo preview, an aggregator entry and a round's added
//! line each pulled the whole catalogue image across the hop to read a few
//! records. The object holds that image: it answers the derived nodes with
//! the same pure function the Worker reads them through
//! (`services::ordering::basket::answer`), and a room command's recipes
//! (`handlers::recipes`) are read here rather than sent across twice.
//!
//!   GET /fold/basket?ids=<product id>&ids=...[&promo=<normalised code>]

use super::{HubImages, CATALOG_IMAGE};
use dowiz_hub::catalog::Catalog;
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::{Call as Request, Reply as Response};

impl HubImages {
    /// The catalogue as this object holds it. A venue with no catalogue yet
    /// reads an empty one, as `hubstore::load_catalog` did for the Worker; a
    /// corrupt one is an error and NOT an empty one.
    pub(super) async fn catalogue(&self) -> Result<Catalog> {
        match self.image(CATALOG_IMAGE).await? {
            Some((_, bytes)) => Catalog::load(&bytes).map_err(|_| Error::RustError("catalogue image is unreadable".into())),
            None => Catalog::create().map_err(|_| Error::RustError("cannot create catalogue".into())),
        }
    }

    /// See the module.
    pub(super) async fn fold_basket(&self, req: &Request) -> Result<Response> {
        let url = req.url()?;
        let ids: Vec<String> = url.query_pairs().filter(|(k, _)| k == "ids").map(|(_, v)| v.to_string()).collect();
        let promo = url.query_pairs().find(|(k, _)| k == "promo").map(|(_, v)| v.to_string());
        // In place (W-ZC, `hubdo/catview.rs`): every placement asks this.
        Response::from_json(&self.with_catalog(|cat| crate::services::ordering::basket::answer(cat, &ids, promo.as_deref())).await?)
    }

    /// Every product with a recipe, for a room command's shelf movements
    /// (`command::amend`, `command::transfer`): read from the catalogue HERE,
    /// where both the catalogue and the shelf are, instead of being pulled by
    /// the Worker and sent back across the hop.
    async fn recipes(&self) -> Result<Vec<(String, String)>> {
        Ok(crate::services::orders::room::handlers::recipes(&self.catalogue().await?))
    }

    /// An amendment with THE recipes (`room.rs`): the shelf moves with the
    /// catalogue this object holds, whatever `boms` the Worker sent -- and it
    /// now sends none.
    pub(super) async fn amend_with_recipes(&self, input: crate::command::amend::AmendIn) -> Result<crate::command::amend::AmendIn> {
        Ok(crate::command::amend::AmendIn { boms: self.recipes().await?, ..input })
    }

    /// A transfer with THE recipes, for the same reason.
    pub(super) async fn transfer_with_recipes(&self, input: crate::command::transfer::TransferIn) -> Result<crate::command::transfer::TransferIn> {
        Ok(crate::command::transfer::TransferIn { boms: self.recipes().await?, ..input })
    }
}
