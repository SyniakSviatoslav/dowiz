//! THE BODY, PARSED SO THAT `deny_unknown_fields` MEANS IT (W-CRUD, 2026-09-29).
//!
//! MEASURED LIVE on qa-durres: `POST /api/owner/categories/x/delete` with an
//! extra field answered `404 unknown category`, not `400 bad request body`,
//! although its struct carries `#[serde(deny_unknown_fields)]`. `Request::json`
//! in workers-rs hands the parsed JS value to `serde_wasm_bindgen`, which reads
//! a struct's DECLARED fields off the object and never sees the rest -- so the
//! attribute was decoration on every route that wore it, and a field the route
//! had no name for (`name`, `category_id`, `move` on `update_product`) was
//! accepted with `{"ok":true}` and dropped. Through the text and `serde_json`
//! the refusal is real, and the same struct definitions need no change.

use worker::*;

/// The body as `T`, or the 400 that names what was wrong with it.
pub(crate) async fn strict<T: serde::de::DeserializeOwned>(req: &mut Request) -> std::result::Result<T, Response> {
    let text = req.text().await.map_err(|e| Response::error(format!("bad request body: {e}"), 400).unwrap())?;
    serde_json::from_str(&text).map_err(|e| Response::error(format!("bad request body: {e}"), 400).unwrap())
}
