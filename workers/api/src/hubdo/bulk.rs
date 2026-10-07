//! A SPREADSHEET OF SUPPLIES OR RECIPES, AS ONE OBJECT TURN (BN1 by the BN4
//! shape, `POST /fold/bulk`).
//!
//! The dry run read the whole catalogue across the hop to judge the file
//! against it, and Apply read it again inside `with_catalog` to write. Both
//! halves are one PURE function now (`import::bulk::turn`): the object reads
//! the catalogue it holds, judges the draft, and -- when asked to apply and
//! nothing refuses -- writes the same catalogue back under the generation
//! guard, in one turn. A refusal writes nothing; the Worker keeps the venue
//! check and the size bound, and hands the object's words through.

use super::{HubImages, CATALOG_IMAGE};
use crate::services::catalogue::import::bulk::{self, BulkIn, Turn};
use dowiz_hub::catalog::Catalog;
use worker::*;
// The plain-Rust request/response (W-COV C2): these bodies run under `cargo test`.
use crate::wire::Reply as Response;

impl HubImages {
    /// The catalogue and the generation it was read at, for a write-back.
    async fn catalogue_at(&self) -> Result<(i64, Catalog)> {
        Ok(match self.image(CATALOG_IMAGE).await? {
            Some((meta, bytes)) => (
                meta.generation,
                Catalog::load(&bytes).map_err(|_| Error::RustError("catalogue image is unreadable".into()))?,
            ),
            None => (0, Catalog::create().map_err(|_| Error::RustError("cannot create catalogue".into()))?),
        })
    }

    /// See the module.
    pub(super) async fn bulk_import(&self, input: BulkIn) -> Result<Response> {
        let (gen, mut cat) = self.catalogue_at().await?;
        match bulk::turn(&mut cat, &input) {
            Turn::Shown(summary) => Response::from_json(&summary),
            Turn::Refused(status, why) => Response::error(why, status),
            Turn::Written(summary) => {
                let bytes = cat.to_bytes().map_err(|e| Error::RustError(format!("catalogue serialise failed: {e:?}")))?;
                // Journaled by the object in the same write, as this signer's (W-PITR2, `hubdo/journal.rs`).
                let by = Some(crate::hubstore::Edited { by: input.by.clone(), at_ms: input.now_ms });
                if self.put_image_stamped(CATALOG_IMAGE, gen, &bytes, by).await?.is_none() {
                    return Response::error("the catalogue generation moved during the import", 409);
                }
                Response::from_json(&summary)
            }
        }
    }
}
