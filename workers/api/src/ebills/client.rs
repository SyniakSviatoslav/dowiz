//! THE ebills.al CLIENT, PURE HALF: what may be asked, how it is asked, and
//! how an answer is judged. The network is `fetch.rs`'s and nothing else's.
//!
//! THE INVARIANT THIS FILE EXISTS FOR (BLUEPRINT-EBILLS §2, §4, §6.2). On
//! ebills.al THE VERB DOES NOT SAY WHETHER A REQUEST WRITES: `GET
//! /api/close-shift` closes the shift, and it was called once by a reader
//! who trusted the verb. So the allow-list is BY PATH, and it is a TYPE:
//!
//! * [`Path`] is the whole list. There is no variant that names any other
//!   path, and no constructor takes a string, so a caller cannot spell
//!   `close-shift` or `sale-pays` or a `/api/sales/*` sub-path at all.
//! * [`Wire`] has two constructors and private fields: `Wire::get(&Path)`,
//!   which never carries a body, and `Wire::login`, the ONE `POST` -- to
//!   `/api/authentication`, with the credentials as its body. There is no
//!   method anywhere that sends a body to anything else.
//! * [`allowed`] re-checks the final URL against the same list, and the
//!   executor refuses to send a `Wire` it rejects: two locks, one per layer.

use serde::{Deserialize, Serialize};

pub(crate) const ORIGIN: &str = "https://www.ebills.al";

/// A browser's User-Agent. Azure Front Door answers curl's default with a
/// `403` block page before the application sees the request (§1.1, measured).
pub(crate) const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0 Safari/537.36";

/// THE ALLOW-LIST (§4). Every path a poller may request, and nothing else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Path {
    /// `GET /` -- only to receive the `XSRF-TOKEN` cookie the login needs.
    Root,
    /// `GET /api/account` -- liveness of the session.
    Account,
    /// `GET /api/sales?…` -- the fiscal list (bills and counter sales), newest
    /// first. Dates are the venue's `YYYY-MM-DD`, inclusive.
    Sales { begin: String, end: String, pos: i64, size: u32 },
    /// `GET /api/sales/{id}?currentPosId=` -- one sale with its lines.
    Sale { id: i64, pos: i64 },
    /// `GET /api/sale-units-tables?pointOfSaleId=` -- the live floor.
    Tables { pos: i64 },
    /// `GET /api/item-in-sales?…` -- the till's menu, for the crosswalk.
    Items { size: u32 },
}

/// A date the list filter can carry: exactly `YYYY-MM-DD`, digits only.
fn is_day(d: &str) -> bool {
    let b = d.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

impl Path {
    /// The URL, or `None` when a field would put something other than a
    /// number or a date on the wire. Nothing here is typed by a person; the
    /// refusal is for the day a caller passes one through.
    pub(crate) fn url(&self) -> Option<String> {
        let path = match self {
            Path::Root => "/".to_string(),
            Path::Account => "/api/account".to_string(),
            Path::Sales { begin, end, pos, size } => {
                if !is_day(begin) || !is_day(end) || *pos < 1 {
                    return None;
                }
                format!(
                    "/api/sales?page=0&size={size}&sort=id,desc&beginDate={begin}&endDate={end}\
                     &client=-1&pointOfSale=-1&extraUser=-1&sale=-1&invOrdNum=-1\
                     &isSelfIssue=false&isReverseCharge=false&isExport=false&currentPosId={pos}"
                )
            }
            Path::Sale { id, pos } if *id > 0 && *pos > 0 => format!("/api/sales/{id}?currentPosId={pos}"),
            Path::Sale { .. } => return None,
            Path::Tables { pos } if *pos > 0 => format!("/api/sale-units-tables?pointOfSaleId={pos}"),
            Path::Tables { .. } => return None,
            Path::Items { size } => format!("/api/item-in-sales?page=0&size={size}&sort=id,asc"),
        };
        Some(format!("{ORIGIN}{path}"))
    }
}

/// THE SECOND LOCK: is this URL one of the allow-listed paths? Exact paths
/// only, the one numeric segment excepted; no `..`, no `%`, no other host.
pub(crate) fn allowed(url: &str, post: bool) -> bool {
    let Some(rest) = url.strip_prefix(ORIGIN) else { return false };
    let path = rest.split('?').next().unwrap_or("");
    if path.contains("..") || path.contains('%') || path.contains("//") {
        return false;
    }
    if post {
        return path == "/api/authentication";
    }
    match path {
        "/" | "/api/account" | "/api/sales" | "/api/sale-units-tables" | "/api/item-in-sales" => true,
        p => p
            .strip_prefix("/api/sales/")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())),
    }
}

/// The session: the cookies the platform set, and the tenant header it
/// named at login. Kept in the venue's own `ebills` image, never in a
/// Worker global (an isolate is not a venue).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Session {
    pub(crate) jar: Vec<(String, String)>,
    pub(crate) tenant: Option<String>,
}

impl Session {
    pub(crate) fn cookie(&self, name: &str) -> Option<&str> {
        self.jar.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    fn cookie_header(&self) -> String {
        self.jar.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("; ")
    }

    /// Take every `Set-Cookie` of an answer: a rotated `JSESSIONID` or
    /// `XSRF-TOKEN` replaces the old one; an emptied or `Max-Age=0` cookie is
    /// dropped. Answers whether anything changed, so an unchanged session is
    /// not written back.
    pub(crate) fn absorb(&mut self, set_cookies: &[String]) -> bool {
        let before = self.jar.clone();
        for line in set_cookies {
            let mut parts = line.split(';');
            let Some((name, value)) = parts.next().and_then(|nv| nv.split_once('=')) else { continue };
            let (name, value) = (name.trim().to_string(), value.trim().to_string());
            let expired = parts.any(|a| a.trim().eq_ignore_ascii_case("max-age=0"));
            self.jar.retain(|(k, _)| *k != name);
            if !value.is_empty() && !expired && !name.is_empty() {
                self.jar.push((name, value));
            }
        }
        self.jar.sort();
        self.jar != before
    }

    /// Logged in: the platform's session cookie, or its 31-day `remember-me`.
    pub(crate) fn live(&self) -> bool {
        self.cookie("JSESSIONID").is_some() || self.cookie("remember-me").is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Get,
    Post,
}

/// ONE REQUEST, DESCRIBED. Private fields and two constructors: this is the
/// type-level half of "no body except the login".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Wire {
    verb: Verb,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl Wire {
    /// A read of an allow-listed path. Never a body.
    pub(crate) fn get(path: &Path, s: &Session) -> Option<Wire> {
        let url = path.url()?;
        let mut headers = vec![
            ("user-agent".to_string(), USER_AGENT.to_string()),
            ("accept".to_string(), "application/json, text/plain, */*".to_string()),
        ];
        if !s.jar.is_empty() {
            headers.push(("cookie".to_string(), s.cookie_header()));
        }
        if let Some(t) = &s.tenant {
            headers.push(("x-tenant-identifier".to_string(), t.clone()));
        }
        Some(Wire { verb: Verb::Get, url, headers, body: None })
    }

    /// THE ONE POST: the form login, with the CSRF pair (§1.2). `s` must
    /// already hold the `XSRF-TOKEN` cookie `GET /` set.
    pub(crate) fn login(user: &str, password: &str, s: &Session) -> Option<Wire> {
        let xsrf = s.cookie("XSRF-TOKEN")?.to_string();
        let body = format!(
            "username={}&password={}&remember-me=true&submit=Login",
            form(user),
            form(password)
        );
        let headers = vec![
            ("user-agent".to_string(), USER_AGENT.to_string()),
            ("accept".to_string(), "application/json, text/plain, */*".to_string()),
            ("content-type".to_string(), "application/x-www-form-urlencoded".to_string()),
            ("x-xsrf-token".to_string(), xsrf),
            ("cookie".to_string(), s.cookie_header()),
        ];
        let url = format!("{ORIGIN}/api/authentication");
        Some(Wire { verb: Verb::Post, url, headers, body: Some(body) })
    }

    pub(crate) fn verb(&self) -> Verb {
        self.verb
    }
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
    pub(crate) fn headers(&self) -> &[(String, String)] {
        &self.headers
    }
    pub(crate) fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }
    /// Both locks at once: the executor sends nothing this refuses.
    pub(crate) fn sendable(&self) -> bool {
        let post = self.verb == Verb::Post;
        allowed(&self.url, post) && (post == self.body.is_some())
    }
}

/// `application/x-www-form-urlencoded`, byte by byte: unreserved kept, space
/// as `+`, everything else `%XX`. A password with `&` must not end the field.
fn form(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for b in v.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'*' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests;
