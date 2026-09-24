//! WHAT AN ANSWER FROM ebills.al MEANS -- pure, so every way the platform
//! can say "no" is a unit test and not a surprise in production.
//!
//! THE RULE (BLUEPRINT-EBILLS §6.7, memory "A silent fallback hid a broken
//! call"): a `401`, a `403`, a `problem+json`, a redirect to a login page, an
//! HTML page where JSON was asked for -- each is a FAILURE with its reason,
//! surfaced to the venue's error log and health. None of them is ever read as
//! "no sales". Only a `2xx` carrying JSON is an answer.

use super::client::Session;

/// One HTTP answer, as `fetch.rs` read it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Answer {
    pub(crate) status: u16,
    pub(crate) content_type: String,
    pub(crate) body: String,
    pub(crate) set_cookies: Vec<String>,
    /// `x-tenant-identifier`, which login answers with and every call echoes.
    pub(crate) tenant: Option<String>,
    /// `x-tenant-identifier-needed`: one login, several tenants (§1.2).
    pub(crate) tenant_needed: bool,
    /// `x-shift-error`: a login outside the operator's shift (§1.2).
    pub(crate) shift_error: Option<String>,
}

/// Why a call did not produce an answer. Every variant is LOUD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Fail {
    /// The session is not (or no longer) accepted: re-login once, then stop.
    Auth(String),
    /// MFA is on for the polling user: unattended polling cannot continue.
    Mfa,
    /// The login names several tenants and the venue did not say which.
    TenantNeeded,
    /// A refusal that is not about the session: the status and its key.
    Refused(u16, String),
    /// `404` -- for a sale id, a gap the platform does not have (a deleted
    /// draft); recorded, never retried.
    NotFound,
    /// A `2xx` that is not JSON: a WAF page, the SPA's index, anything else.
    NotJson(String),
    /// The request never completed.
    Network(String),
    /// A body that is JSON and not the shape (`parse_*` said why).
    Shape(String),
    /// The executor refused to send it: not on the allow-list.
    NotAllowed(String),
    /// The venue's own object refused a command: not ebills' fault.
    Hub(u16, String),
}

impl Fail {
    /// One line for the owner's health and the venue's error log.
    pub(crate) fn line(&self) -> String {
        match self {
            Fail::Auth(m) => format!("ebills refused the session: {m}"),
            Fail::Mfa => "ebills asks for a second factor: turn MFA off for the polling user".into(),
            Fail::TenantNeeded => "ebills: this login belongs to several businesses".into(),
            Fail::Refused(s, k) => format!("ebills answered {s}: {k}"),
            Fail::NotFound => "ebills: not found".into(),
            Fail::NotJson(ct) => format!("ebills answered a page, not data ({ct})"),
            Fail::Network(m) => format!("ebills unreachable: {m}"),
            Fail::Shape(m) => format!("ebills changed its answer: {m}"),
            Fail::NotAllowed(u) => format!("refused to send a request off the allow-list: {u}"),
            Fail::Hub(s, m) => format!("the venue's hub refused ({s}): {m}"),
        }
    }

    /// Worth one fresh login and one retry.
    pub(crate) fn is_auth(&self) -> bool {
        matches!(self, Fail::Auth(_))
    }
}

/// The key a JHipster error body names: `errorKey`, else `message`, else
/// `title`, else the first 120 characters. Never the whole body -- a list
/// response embeds the fiscal certificate (§1.7), an error might too.
fn key_of(body: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    for k in ["errorKey", "message", "title", "detail"] {
        if let Some(s) = v.get(k).and_then(|x| x.as_str()) {
            return s.chars().take(120).collect();
        }
    }
    body.chars().take(120).collect()
}

/// A read's answer: the body, or why there is none.
pub(crate) fn read(a: &Answer) -> Result<&str, Fail> {
    let ct = a.content_type.to_ascii_lowercase();
    match a.status {
        200..=299 if ct.contains("problem+json") => Err(Fail::Refused(a.status, key_of(&a.body))),
        200..=299 if ct.contains("json") => Ok(&a.body),
        200..=299 => Err(Fail::NotJson(ct)),
        300..=399 => Err(Fail::Auth(format!("redirected ({})", a.status))),
        401 | 403 => Err(Fail::Auth(format!("{} {}", a.status, key_of(&a.body)))),
        404 => Err(Fail::NotFound),
        s => Err(Fail::Refused(s, key_of(&a.body))),
    }
}

/// The login's answer (§1.2): `200`, a `JSESSIONID` (or the `remember-me`
/// that re-creates one), and the tenant header -- or the named reason.
pub(crate) fn logged_in(a: &Answer, mut s: Session) -> Result<Session, Fail> {
    if a.tenant_needed {
        return Err(Fail::TenantNeeded);
    }
    if a.status == 401 && a.body.contains("mfaRequired") {
        return Err(Fail::Mfa);
    }
    if let Some(e) = &a.shift_error {
        return Err(Fail::Auth(format!("outside the shift: {e}")));
    }
    if !(200..=299).contains(&a.status) {
        return Err(Fail::Auth(format!("login {} {}", a.status, key_of(&a.body))));
    }
    s.absorb(&a.set_cookies);
    if !s.live() {
        return Err(Fail::Auth("login answered 200 and set no session cookie".into()));
    }
    let Some(t) = a.tenant.clone().filter(|t| !t.is_empty()) else {
        return Err(Fail::Auth("login answered with no tenant identifier".into()));
    };
    s.tenant = Some(t);
    Ok(s)
}
