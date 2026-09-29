//! THE REQUEST BODY, PARSED SO THAT `deny_unknown_fields` MEANS IT.
//!
//! MEASURED LIVE on qa-durres (W-CRUD, 2026-09-29): `POST /api/owner/categories/x/delete`
//! with an extra field answered `404 unknown category`, not `400 bad request body`,
//! although its struct carries `#[serde(deny_unknown_fields)]`. `Request::json` in
//! workers-rs is `JSON.parse` on the JS side followed by `serde_wasm_bindgen`, which
//! reads a struct's DECLARED fields off the object and never sees the rest -- so the
//! attribute was decoration on every route that wore it, and a field the route had no
//! name for (`name`, `category_id`, `move` on `update_product`) was accepted with
//! `{"ok":true}` and dropped. Native unit tests parse with `serde_json`, which DOES
//! refuse, so every test was green while production silently changed nothing.
//!
//! THE RULE (W-STRICT, gate `tools/gates/strict-body.sh`): a handler never calls
//! `req.json()`. It calls [`parse`] (or [`strict`]), which reads the text and hands it
//! to `serde_json` -- the same parser the tests use, so what a test proves is what the
//! Worker does. `parse` returns the same `worker::Error` a `req.json()` did, so a call
//! site keeps its own `match`/`?` and its own error text; only the parser changes.

use serde::de::DeserializeOwned;
use worker::{Error, Request, Response, Result};

/// The text as `T`, or serde_json's reason -- which names the field
/// (`unknown field \`nmae\`, expected one of ...`). The one place the rule lives;
/// the tests in `body/tests.rs` go through this function.
pub(crate) fn from_text<T: DeserializeOwned>(text: &str) -> std::result::Result<T, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// The body as `T`. A drop-in for `req.json().await`: the error is a
/// `worker::Error` whose `Display` is serde_json's message.
pub(crate) async fn parse<T: DeserializeOwned>(req: &mut Request) -> Result<T> {
    let text = req.text().await?;
    from_text(&text).map_err(Error::RustError)
}

/// The body as `T`, or the 400 that names what was wrong with it.
pub(crate) async fn strict<T: DeserializeOwned>(req: &mut Request) -> std::result::Result<T, Response> {
    parse(req).await.map_err(|e| Response::error(format!("bad request body: {e}"), 400).unwrap())
}

/// Test aid: the reason `text` is refused as `T` (panics if it was accepted).
/// Body structs do not derive `Debug`, so `unwrap_err` is not available on them.
#[cfg(test)]
pub(crate) fn refusal<T: DeserializeOwned>(text: &str) -> String {
    match from_text::<T>(text) {
        Ok(_) => panic!("accepted a body that should be refused: {text}"),
        Err(e) => e,
    }
}

#[cfg(test)]
mod tests;
