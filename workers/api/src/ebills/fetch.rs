//! THE ONLY NETWORK IN THE TILL LINK. It sends a [`Wire`] -- and a `Wire`
//! can only be an allow-listed read or the login (`client.rs`) -- and reads
//! the answer into an [`Answer`] for `judge.rs` to rule on.
//!
//! No clock is read here and nothing is logged: a list response embeds the
//! business's signing certificate (§1.7), so a body is parsed and dropped,
//! never printed.

use super::client::{Path, Session, Verb, Wire};
use super::judge::{self, Answer, Fail};
use worker::wasm_bindgen::JsValue;
use worker::*;

/// Send one request. Refuses, without sending, anything `Wire::sendable`
/// rejects: the second lock on the allow-list.
async fn send(w: &Wire) -> std::result::Result<Answer, Fail> {
    if !w.sendable() {
        return Err(Fail::NotAllowed(w.url().to_string()));
    }
    let net = |e: Error| Fail::Network(e.to_string());
    let headers = Headers::new();
    for (k, v) in w.headers() {
        headers.set(k, v).map_err(net)?;
    }
    let mut init = RequestInit::new();
    init.with_method(if w.verb() == Verb::Post { Method::Post } else { Method::Get })
        .with_headers(headers)
        // A REDIRECT IS AN ANSWER, not something to follow: a lapsed session
        // is sent to the login page, and following it would read HTML as data.
        .with_redirect(RequestRedirect::Manual);
    if let Some(b) = w.body() {
        init.with_body(Some(JsValue::from_str(b)));
    }
    let req = Request::new_with_init(w.url(), &init).map_err(net)?;
    let mut res = Fetch::Request(req).send().await.map_err(net)?;
    let h = res.headers();
    let one = |k: &str| h.get(k).ok().flatten();
    let mut a = Answer {
        status: res.status_code(),
        content_type: one("content-type").unwrap_or_default(),
        set_cookies: h.get_all("set-cookie").unwrap_or_default(),
        tenant: one("x-tenant-identifier"),
        tenant_needed: one("x-tenant-identifier-needed").is_some(),
        shift_error: one("x-shift-error"),
        body: String::new(),
    };
    a.body = res.text().await.map_err(net)?;
    Ok(a)
}

/// A logged-in reader for one firing: the session it started with (from the
/// venue's image), a fresh login when there is none or it is refused ONCE,
/// and whether the session changed so the caller can write it back.
pub(crate) struct Client {
    user: String,
    password: String,
    pub(crate) session: Session,
    pub(crate) changed: bool,
    relogged: bool,
}

impl Client {
    pub(crate) fn new(user: &str, password: &str, session: Option<Session>) -> Client {
        Client { user: user.into(), password: password.into(), session: session.unwrap_or_default(), changed: false, relogged: false }
    }

    /// `GET /` for the CSRF cookie, then the one `POST` (§1.2).
    async fn login(&mut self) -> std::result::Result<(), Fail> {
        let mut s = Session::default();
        let root = Wire::get(&Path::Root, &s).ok_or_else(|| Fail::NotAllowed("/".into()))?;
        let a = send(&root).await?;
        if !(200..=299).contains(&a.status) {
            return Err(Fail::Refused(a.status, "the front page did not answer".into()));
        }
        s.absorb(&a.set_cookies);
        let w = Wire::login(&self.user, &self.password, &s)
            .ok_or_else(|| Fail::Auth("the front page set no XSRF-TOKEN cookie".into()))?;
        self.session = judge::logged_in(&send(&w).await?, s)?;
        self.changed = true;
        Ok(())
    }

    /// One allow-listed read. A refused session is re-established once per
    /// firing and the read retried; a second refusal is the answer.
    pub(crate) async fn read(&mut self, p: &Path) -> std::result::Result<String, Fail> {
        if !self.session.live() {
            self.relogged = true;
            self.login().await?;
        }
        loop {
            let w = Wire::get(p, &self.session).ok_or_else(|| Fail::NotAllowed(format!("{p:?}")))?;
            let a = send(&w).await?;
            if self.session.absorb(&a.set_cookies) {
                self.changed = true;
            }
            match judge::read(&a) {
                Ok(body) => return Ok(body.to_string()),
                Err(f) if f.is_auth() && !self.relogged => {
                    self.relogged = true;
                    self.login().await?;
                }
                Err(f) => return Err(f),
            }
        }
    }
}
