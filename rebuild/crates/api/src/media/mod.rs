//! S4 media surface — non-route business logic. Ports the ADR-0002 product-media seam
//! (`apps/api/src/routes/owner/product-media.ts`), the product-image/theme-logo/entry-photo
//! sharp transcode pipelines (`spa-proxy.ts`, `themes.ts`), and the R2 `put`/`delete` verbs
//! (`r2-storage.ts`) — see `docs/design/rebuild-media-s4-council/resolution.md` for the frozen
//! REV-S4-1..9 revision set every module here implements.
//!
//! Route handlers live in `routes/owner/product_media.rs`, `routes/owner/product_image.rs`,
//! `routes/owner/themes.rs` (logo op), and `routes/media_public.rs` (the token-proxy-PUT
//! endpoint + entry-photo) — this module holds the framework/DB-free logic those handlers call
//! into, mirroring the `service.rs`/`storage.rs` split S1 already established.

pub mod processor;
pub mod upload_token;
pub mod validation;

/// REV-S4-3 golden-fixture parity suite (fixtures generated once via
/// `tests/fixtures/media/generate-goldens.mjs`, embedded via `include_bytes!`) + REV-S4-4's
/// all-8-EXIF-orientation-values DoD. Test-only — never compiled into the release binary.
#[cfg(test)]
mod golden_parity_tests;
