//! A REQUEST AND A RESPONSE THAT ARE PLAIN RUST, so the code behind a route can run under
//! `cargo test` (W-COV C2).
//!
//! WHY. `worker::Request` is a JS object and every `worker::Response` constructor builds a
//! `web_sys::Headers` — a wasm-bindgen import, which PANICS on a native target. So a handler
//! written against those types can only ever run inside the Workers runtime, and 74 files of
//! this crate had never executed in a test (62.8% of the Worker's lines, main a7fcbd4d).
//!
//! THE SEAM. `Call` and `Reply` carry the same methods the handlers already used on the worker
//! types (`url`, `path`, `method`, `headers().get`, `text`, `bytes`; `from_json`, `ok`, `error`,
//! `from_bytes`, `empty`, `with_status`, `headers_mut().set`), so a module switches by importing
//! them under the old names and its bodies do not change. The Workers edge converts at ONE place:
//! `Call::from_worker` in, `Reply::into_response` out, with the same status, headers and bytes
//! the worker constructors would have produced (content types copied from worker 0.8.5).

use serde::Serialize;
use worker::{Error, Method, Result, Url};

/// Header names are case-insensitive; stored lower-case, set replaces.
#[derive(Clone, Debug, Default)]
pub struct Fields(Vec<(String, String)>);

impl Fields {
    pub fn new() -> Self {
        Self::default()
    }
    #[cfg(test)]
    pub fn has(&self, name: &str) -> Result<bool> {
        Ok(self.get(name)?.is_some())
    }
    pub fn append(&mut self, name: &str, value: &str) -> Result<()> {
        self.0.push((name.to_ascii_lowercase(), value.to_string()));
        Ok(())
    }
    #[cfg(test)]
    pub fn delete(&mut self, name: &str) -> Result<()> {
        let n = name.to_ascii_lowercase();
        self.0.retain(|(k, _)| *k != n);
        Ok(())
    }
    pub fn get(&self, name: &str) -> Result<Option<String>> {
        let n = name.to_ascii_lowercase();
        Ok(self.0.iter().find(|(k, _)| *k == n).map(|(_, v)| v.clone()))
    }
    pub fn set(&mut self, name: &str, value: &str) -> Result<()> {
        let n = name.to_ascii_lowercase();
        self.0.retain(|(k, _)| *k != n);
        self.0.push((n, value.to_string()));
        Ok(())
    }
    /// The platform's headers, for a response that leaves as a `worker::Response`.
    pub fn into_worker(&self) -> Result<worker::Headers> {
        let h = worker::Headers::new();
        for (k, v) in &self.0 {
            h.append(k, v)?;
        }
        Ok(h)
    }
    pub fn entries(&self) -> impl Iterator<Item = &(String, String)> {
        self.0.iter()
    }
}

/// An incoming request, owned and native.
#[derive(Clone, Debug)]
pub struct Call {
    method: Method,
    url: Url,
    headers: Fields,
    body: Vec<u8>,
}

impl Call {
    /// Same argument order as `worker::Request::new`.
    pub fn new(url: &str, method: Method) -> Result<Self> {
        let url = Url::parse(url).map_err(|e| Error::RustError(format!("bad url {url}: {e}")))?;
        Ok(Call { method, url, headers: Fields::default(), body: Vec::new() })
    }
    /// Same shape as `worker::Request::new_with_init`.
    pub fn new_with_init(url: &str, init: &RequestInit) -> Result<Self> {
        let mut c = Self::new(url, init.method.clone())?;
        c.headers = init.headers.clone();
        c.body = init.body.clone().unwrap_or_default();
        Ok(c)
    }
    pub fn headers_mut(&mut self) -> Result<&mut Fields> {
        Ok(&mut self.headers)
    }
    /// For an outbound call that leaves this crate (the platform's `fetch`).
    pub fn into_worker(self) -> Result<worker::Request> {
        // Natively there is no platform to hand a request to; say so rather than panic in a
        // wasm-bindgen import (W-COV C2).
        #[cfg(not(target_arch = "wasm32"))]
        if !cfg!(target_arch = "wasm32") {
            return Err(Error::RustError(format!("no outbound fetch outside the platform: {}", self.url)));
        }
        let headers = worker::Headers::new();
        for (k, v) in self.headers.entries() {
            headers.append(k, v)?;
        }
        let mut init = worker::RequestInit::new();
        init.with_method(self.method.clone()).with_headers(headers);
        if !matches!(self.method, Method::Get | Method::Head) {
            init.with_body(Some(worker::js_sys::Uint8Array::from(self.body.as_slice()).into()));
        }
        worker::Request::new_with_init(self.url.as_str(), &init)
    }
    #[cfg(test)]
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        let _ = self.headers.set(name, value);
        self
    }
    #[cfg(test)]
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
    /// What a recorded outbound call carried (tests read `edge::mem::sent()`).
    #[cfg(test)]
    pub fn body_bytes(&self) -> Vec<u8> {
        self.body.clone()
    }
    #[cfg(test)]
    pub fn with_json<T: Serialize>(self, body: &T) -> Self {
        let text = serde_json::to_vec(body).unwrap_or_default();
        self.with_body(text)
    }
    pub fn method(&self) -> Method {
        self.method.clone()
    }
    pub fn path(&self) -> String {
        self.url.path().to_string()
    }
    pub fn url(&self) -> Result<Url> {
        Ok(self.url.clone())
    }
    pub fn headers(&self) -> &Fields {
        &self.headers
    }
    pub async fn bytes(&mut self) -> Result<Vec<u8>> {
        Ok(self.body.clone())
    }
    pub async fn text(&mut self) -> Result<String> {
        String::from_utf8(self.body.clone()).map_err(|e| Error::RustError(e.to_string()))
    }

    /// The Workers edge: read the whole request once. Nothing here is lazy any more, which is
    /// the same as before for every route this is used on — each of them read its body.
    pub async fn from_worker(mut req: worker::Request) -> Result<Self> {
        let method = req.method();
        let url = req.url()?;
        let mut headers = Fields::default();
        for (k, v) in req.headers().entries() {
            let _ = headers.set(&k, &v);
        }
        let body = match method {
            Method::Get | Method::Head => Vec::new(),
            _ => req.bytes().await?,
        };
        Ok(Call { method, url, headers, body })
    }
}

/// `worker::RequestInit`, native: a method, headers and an owned body.
#[derive(Clone, Debug, Default)]
pub struct RequestInit {
    pub method: Method,
    pub headers: Fields,
    pub body: Option<Vec<u8>>,
}

impl RequestInit {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_method(&mut self, method: Method) -> &mut Self {
        self.method = method;
        self
    }
    pub fn with_headers(&mut self, headers: Fields) -> &mut Self {
        self.headers = headers;
        self
    }
    pub fn with_body(&mut self, body: Option<Vec<u8>>) -> &mut Self {
        self.body = body;
        self
    }
}

/// An outgoing response, owned and native.
#[derive(Clone, Debug)]
pub struct Reply {
    status: u16,
    headers: Fields,
    /// `None` is worker's `ResponseBody::Empty`, which is not the same as zero bytes.
    body: Option<Vec<u8>>,
    /// A platform response that cannot be read into bytes and rebuilt: the
    /// object's `101 Switching Protocols`, whose WebSocket lives on the JS
    /// object itself. Rebuilding it from status + headers + body dropped the
    /// socket and the runtime answered 500 to every `/api/live` upgrade
    /// (live on b1f777b3, 2026-10-02, found by the flows gate). It is handed
    /// back untouched by `into_response`.
    upgrade: Upgrade,
}

/// The untouched platform response of an upgrade (see `Reply::upgrade`).
#[derive(Clone, Default)]
pub struct Upgrade(std::rc::Rc<std::cell::RefCell<Option<worker::Response>>>);

impl std::fmt::Debug for Upgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.borrow().is_some() { "Upgrade(socket)" } else { "Upgrade(none)" })
    }
}

impl Reply {
    fn with(content_type: Option<&str>, body: Option<Vec<u8>>) -> Self {
        let mut headers = Fields::default();
        if let Some(t) = content_type {
            let _ = headers.set("content-type", t);
        }
        Reply { status: 200, headers, body, upgrade: Upgrade::default() }
    }
    pub fn from_json<B: Serialize>(value: &B) -> Result<Self> {
        match serde_json::to_string(value) {
            Ok(s) => Ok(Self::with(Some("application/json"), Some(s.into_bytes()))),
            Err(_) => Err(Error::Json(("Failed to encode data to json".into(), 500))),
        }
    }
    pub fn ok(body: impl Into<String>) -> Result<Self> {
        Ok(Self::with(Some("text/plain; charset=utf-8"), Some(body.into().into_bytes())))
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self::with(Some("application/octet-stream"), Some(bytes)))
    }
    pub fn empty() -> Result<Self> {
        Ok(Self::with(None, None))
    }
    /// Same range check as worker's: an error outside 400..=599 is itself an error.
    pub fn error(msg: impl Into<String>, status: u16) -> Result<Self> {
        if !(400..=599).contains(&status) {
            return Err(Error::Internal(
                "error status codes must be in the 400-599 range".into(),
            ));
        }
        let mut r = Self::with(None, Some(msg.into().into_bytes()));
        r.status = status;
        Ok(r)
    }
    pub fn from_html(html: impl AsRef<str>) -> Result<Self> {
        Ok(Self::with(Some("text/html; charset=utf-8"), Some(html.as_ref().as_bytes().to_vec())))
    }
    pub fn with_headers(mut self, headers: Fields) -> Self {
        self.headers = headers;
        self
    }
    pub fn cloned(&mut self) -> Result<Self> {
        Ok(self.clone())
    }
    pub async fn text(&mut self) -> Result<String> {
        String::from_utf8(self.body().to_vec()).map_err(|e| Error::RustError(e.to_string()))
    }
    pub async fn bytes(&mut self) -> Result<Vec<u8>> {
        Ok(self.body().to_vec())
    }
    pub async fn json<T: serde::de::DeserializeOwned>(&mut self) -> Result<T> {
        serde_json::from_slice(self.body()).map_err(|e| Error::RustError(e.to_string()))
    }
    /// A platform response (a Durable Object's, a `fetch`'s), read whole.
    pub async fn from_worker(mut res: worker::Response) -> Result<Self> {
        let mut headers = Fields::default();
        for (k, v) in res.headers().entries() {
            let _ = headers.append(&k, &v);
        }
        let status = res.status_code();
        if status == 101 {
            return Ok(Reply { status, headers, body: None, upgrade: Upgrade(std::rc::Rc::new(std::cell::RefCell::new(Some(res)))) });
        }
        let body = Some(res.bytes().await?);
        Ok(Reply { status, headers, body, upgrade: Upgrade::default() })
    }
    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }
    pub fn headers_mut(&mut self) -> &mut Fields {
        &mut self.headers
    }
    pub fn headers(&self) -> &Fields {
        &self.headers
    }
    pub fn status_code(&self) -> u16 {
        self.status
    }
    pub fn body(&self) -> &[u8] {
        self.body.as_deref().unwrap_or(&[])
    }
    /// Test readers, synchronous: the body as text, and as a JSON value (`Null` if it is not).
    #[cfg(test)]
    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(self.body()).into_owned()
    }
    #[cfg(test)]
    pub fn body_value(&self) -> serde_json::Value {
        serde_json::from_slice(self.body()).unwrap_or(serde_json::Value::Null)
    }

    /// The Workers edge: the response the worker constructors would have built.
    pub fn into_response(self) -> Result<worker::Response> {
        if let Some(res) = self.upgrade.0.borrow_mut().take() {
            return Ok(res);
        }
        let mut b = worker::Response::builder().with_status(self.status);
        for (k, v) in self.headers.entries() {
            b = b.with_header(k, v)?;
        }
        Ok(match self.body {
            Some(bytes) => b.fixed(bytes),
            None => b.empty(),
        })
    }
}
