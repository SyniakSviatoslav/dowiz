//! THE OWNER'S CATALOGUE EDITS, each run WHOLE by the venue's object in ONE request
//! (W-D9 = R-GRAPH D9, the write twin of W-ZC; the pattern is W-BN4B's, `hubdo/command.rs`).
//!
//! WHAT IT REPLACED, MEASURED (`catalog_edit/costs/tests.rs`, the test edge's bytes per
//! request): every edit ran `hubstore::with_catalog` from the Worker -- the WHOLE catalogue
//! image read across the hop, changed, and written back across it (`/img/catalog` GET+PUT,
//! twice the image per edit, more on a contended generation), plus the settings image for
//! the allergen gate and the translation table for a delete. Now the Worker picks the
//! object (`Place::of_any`: the signed token's claim, else the Host) and the object runs
//! the same handler body (`*_at`, unchanged) in its own turn, where each of those reads
//! and writes is a function call. The body still authorises the venue it names, journals
//! the signer (W-PITR2: the write is the object's own `/img/catalog` PUT, as before), and
//! the menu projection is re-folded by the catalogue's generation exactly as before.

use crate::edge::Ctx;
use crate::hubdo::command::{forward, Kind};
use crate::wire::{Call, Reply};
use std::{future::Future, pin::Pin};
use worker::Result;

/// A handler's turn, boxed.
pub(crate) type Turn = Pin<Box<dyn Future<Output = Result<Reply>>>>;

/// THE OBJECT'S HALF (`hubdo/command.rs`): the handler body for a catalogue `Kind`, built
/// here and NOT in `command_route`'s own frame; any other kind is handed back untouched.
#[inline(never)]
pub(crate) fn in_object(kind: Kind, call: Call, ctx: Ctx<crate::Req>) -> std::result::Result<Turn, (Call, Ctx<crate::Req>)> {
    Ok(match kind {
        Kind::ProductNew => Box::pin(crate::catalog_edit::create_product_at(call, ctx)),
        Kind::ProductEdit => Box::pin(crate::owner::update_product_at(call, ctx)),
        Kind::ProductDelete => Box::pin(crate::catalog_edit::delete_product_at(call, ctx)),
        Kind::ProductsDelete => Box::pin(crate::catalog_edit::delete_products_at(call, ctx)),
        Kind::CategorySet => Box::pin(crate::catalog_edit::set_category_at(call, ctx)),
        Kind::CategoryDelete => Box::pin(crate::catalog_edit::delete_category_at(call, ctx)),
        Kind::Translations => Box::pin(crate::owner::write_translations_at(call, ctx)),
        Kind::Location => Box::pin(crate::owner::update_location_at(call, ctx)),
        Kind::ProductImage => Box::pin(crate::services::catalogue::media::photo_in_object(call, ctx)),
        Kind::ProductImageClear => Box::pin(crate::services::catalogue::media::clear_product_image_at(call, ctx)),
        Kind::MenuRestore => Box::pin(crate::catalog_history::restore_at(call, ctx)),
        Kind::MenuImport => Box::pin(crate::services::catalogue::import::import_menu_at(call, ctx)),
        _ => return Err((call, ctx)),
    })
}

/// `POST /api/owner/products` (`catalog_edit::create_product_at`).
pub async fn create_product(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::ProductNew, req, ctx).await
}

/// `POST /api/owner/products/:id` (`owner::update_product_at`): edit, price, availability.
pub async fn update_product(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::ProductEdit, req, ctx).await
}

/// `POST /api/owner/products/:id/delete` (`catalog_edit::delete_product_at`).
pub async fn delete_product(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::ProductDelete, req, ctx).await
}

/// `POST /api/owner/products/delete` (`catalog_edit::delete_products_at`).
pub async fn delete_products(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::ProductsDelete, req, ctx).await
}

/// `POST /api/owner/categories` (`catalog_edit::set_category_at`).
pub async fn set_category(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::CategorySet, req, ctx).await
}

/// `POST /api/owner/categories/:id/delete` (`catalog_edit::delete_category_at`).
pub async fn delete_category(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::CategoryDelete, req, ctx).await
}

/// `POST /api/owner/i18n` (`owner::write_translations_at`).
pub async fn write_translations(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::Translations, req, ctx).await
}

/// `PATCH|POST /api/owner/location` (`owner::update_location_at`): open, close, busy, pause.
pub async fn update_location(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::Location, req, ctx).await
}

/// `POST /api/owner/products/:id/image/clear` (`media::clear_product_image_at`).
pub async fn clear_product_image(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::ProductImageClear, req, ctx).await
}

/// `POST /api/owner/menu/history/restore` (`catalog_history::restore_at`).
pub async fn restore(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::MenuRestore, req, ctx).await
}
