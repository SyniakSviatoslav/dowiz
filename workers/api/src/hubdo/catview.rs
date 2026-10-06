//! THE CATALOGUE READ IN PLACE, IN THE OBJECT (W-ZC, R-BEBOPDB D.1 #1).
//!
//! `catalogue()` (hubdo/basket.rs) answers a read with `Catalog::load`: `image()` clones
//! the bytes out of `mem`, `from_bytes` copies them into cells, the crc runs over all
//! five objects and every entry is decoded -- for one dish. `with_catalog` borrows the
//! bytes `mem` already holds and reads them through `dowiz_hub::catalog::CatalogView`.
//!
//! THE CRC, ONCE PER GENERATION (W-CRC's policy, at the point bytes enter the object).
//! The first read of a generation runs the full check (`CatalogView::open`); its
//! `Checked` is kept in `cat_checked` with the generation, and the next reads of the
//! same bytes `reopen` without a crc. Bytes ENTER `mem` in exactly two places, the cold
//! read in `image()` and the write in `put_image`, and both call `catview_forget` first,
//! so a `Checked` never outlives the bytes it was taken on -- a re-read after an
//! eviction is checked again even at the same generation.

use super::{HubImages, CATALOG_IMAGE};
use dowiz_hub::catalog::{Catalog, CatalogRead, CatalogView};
use worker::*;

impl HubImages {
    /// The memo no longer describes the bytes about to enter `mem` under `id`.
    pub(super) fn catview_forget(&self, id: &str) {
        if id == CATALOG_IMAGE {
            self.cat_checked.set(None);
        }
    }

    /// Run `f` over the catalogue as this object holds it, read in place. No image yet
    /// reads as an empty catalogue (as `catalogue()` does); a corrupt one is an error.
    pub(super) async fn with_catalog<T>(&self, f: impl FnOnce(&dyn CatalogRead) -> T) -> Result<T> {
        let resident = self.mem.borrow().contains_key(CATALOG_IMAGE);
        if !resident && self.image(CATALOG_IMAGE).await?.is_none() {
            let empty = Catalog::create().map_err(|_| Error::RustError("cannot create catalogue".into()))?;
            return Ok(f(&empty));
        }
        // NO AWAIT FROM HERE ON: the borrow of `mem` is held while `f` runs.
        let mem = self.mem.borrow();
        let Some((meta, bytes)) = mem.get(CATALOG_IMAGE) else {
            return Err(Error::RustError("catalogue image left memory during a read".into()));
        };
        let view = match self.cat_checked.get() {
            Some((gen, ck)) if gen == meta.generation => CatalogView::reopen(bytes, ck),
            _ => CatalogView::open(bytes).inspect(|v| self.cat_checked.set(Some((meta.generation, v.checked())))),
        }
        .map_err(|_| Error::RustError("catalogue image is unreadable".into()))?;
        Ok(f(&view))
    }
}

#[cfg(test)]
mod tests;
