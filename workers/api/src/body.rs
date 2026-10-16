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
use worker::{Error, Request, Result};

/// The text as `T`, or serde_json's reason -- which names the field
/// (`unknown field \`nmae\`, expected one of ...`). The one place the rule lives;
/// the tests in `body/tests.rs` go through this function.
pub(crate) fn from_text<T: DeserializeOwned>(text: &str) -> std::result::Result<T, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// The body as `T`. A drop-in for `req.json().await`: the error is a
/// `worker::Error` whose `Display` is serde_json's message.
/// A request whose body can be read as text: the platform's, or the plain-Rust
/// `wire::Call` a Durable Object route runs on under `cargo test` (W-COV C2).
pub(crate) trait Body {
    async fn body_text(&mut self) -> Result<String>;
    /// `?location_id=` of the request's URL, if it carries one.
    fn query_venue(&self) -> Option<String>;
}
fn venue_of(url: Result<worker::Url>) -> Option<String> {
    url.ok()?.query_pairs().find(|(k, _)| k == "location_id").map(|(_, v)| v.into_owned())
}
impl Body for Request {
    async fn body_text(&mut self) -> Result<String> {
        self.text().await
    }
    fn query_venue(&self) -> Option<String> {
        venue_of(self.url())
    }
}
impl Body for crate::wire::Call {
    async fn body_text(&mut self) -> Result<String> {
        self.text().await
    }
    fn query_venue(&self) -> Option<String> {
        venue_of(self.url())
    }
}

/// THE VENUE MAY RIDE IN THE QUERY (operator 2026-10-16): the GET routes take
/// `?location_id=`, so a write sent the same way was a `400 missing field
/// location_id`. `text` as `T` with `query` the URL's venue: a body naming a
/// DIFFERENT venue is refused (never one silently picked); a body without one
/// that `T` refuses is read again with the query's put in. A `T` with no venue
/// field never gets one, so `deny_unknown_fields` means what it meant. PURE.
pub(crate) fn from_text_at<T: DeserializeOwned>(text: &str, query: Option<&str>) -> std::result::Result<T, String> {
    let Some(q) = query else { return from_text(text) };
    let mut v: serde_json::Value = match serde_json::from_str(text) {
        Ok(serde_json::Value::Object(m)) => serde_json::Value::Object(m),
        _ => return from_text(text),
    };
    match v.get("location_id") {
        Some(b) if b.as_str() != Some(q) => return Err("location_id in the query and the body differ".into()),
        Some(_) => return from_text(text),
        None => {}
    }
    let first = match from_text(text) {
        Ok(t) => return Ok(t),
        Err(e) => e,
    };
    v["location_id"] = serde_json::Value::String(q.to_string());
    // Still refused with the venue put in: the body's OWN reason, unchanged.
    serde_json::from_value(v).map_err(|_| first)
}

pub(crate) async fn parse<T: DeserializeOwned>(req: &mut impl Body) -> Result<T> {
    let query = req.query_venue();
    let text = req.body_text().await?;
    from_text_at(&text, query.as_deref()).map_err(Error::RustError)
}

/// The body as `T`, or the 400 that names what was wrong with it.
pub(crate) async fn strict<T: DeserializeOwned>(req: &mut impl Body) -> std::result::Result<T, crate::wire::Reply> {
    parse(req).await.map_err(|e| crate::wire::Reply::error(format!("bad request body: {e}"), 400).unwrap())
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
