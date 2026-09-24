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

/// WHAT A CREATE ANSWERED (EBILLS-WRITE-PATH §2.1). A `200` is a sale that
/// EXISTS, fiscalised or not; only a refusal before the controller (401, a
/// 4xx) is "no sale". Anything that does not show which is `Unknown`, and an
/// unknown is reconciled by a read before any retry (§3 (c)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Created {
    /// `logCis[0].status` SUCCESS: the tax authority's codes are in hand.
    Fiscalised { sale_id: i64, iic: String, fic: String, inv_ord_num: String },
    /// The sale exists and was NOT fiscalised (`ERROR`, `WEBSERVICEERROR`,
    /// `PENDING`): numbered, listed, and never to be sent again (§7.1 9).
    Unfiscalised { sale_id: i64, fault: String },
    /// Refused before a sale existed: the status and its key.
    Refused(u16, String),
    /// The session or the CSRF pair: nothing was created (§1.2).
    Auth(String),
    /// Whether a sale exists is not known: a 5xx, a redirect, a body that
    /// is not the sale.
    Unknown(String),
}

/// A create's answer, ruled.
pub(crate) fn created(a: &Answer) -> Created {
    let ct = a.content_type.to_ascii_lowercase();
    match a.status {
        200..=299 if ct.contains("json") && !ct.contains("problem+json") => match serde_json::from_str(&a.body) {
            Ok(v) => sale_state(&v),
            Err(_) => Created::Unknown(format!("{} answered a body that is not JSON", a.status)),
        },
        200..=299 => Created::Unknown(format!("{} answered {ct}, not the sale", a.status)),
        300..=399 => Created::Unknown(format!("redirected ({})", a.status)),
        401 | 403 => Created::Auth(format!("{} {}", a.status, key_of(&a.body))),
        400..=499 => Created::Refused(a.status, key_of(&a.body)),
        s => Created::Unknown(format!("{s} {}", key_of(&a.body))),
    }
}

/// A sale entity (the create's answer, or `/api/sales/{id}`'s `sale`), ruled
/// by its fiscalisation log. The fault is cut to 200 characters: the log
/// embeds the sale, and nothing of it is kept beyond the tax authority's words.
pub(crate) fn sale_state(v: &serde_json::Value) -> Created {
    let Some(sale_id) = v.get("id").and_then(serde_json::Value::as_i64) else {
        return Created::Unknown("the answer carries no sale id".into());
    };
    let text = |x: Option<&serde_json::Value>| x.and_then(|s| s.as_str()).map(str::to_string);
    let log = v.get("logCis").and_then(|l| l.get(0));
    let status = text(log.and_then(|l| l.get("status")));
    let fic = text(log.and_then(|l| l.get("fic"))).or_else(|| text(v.get("fic"))).filter(|f| !f.is_empty());
    let fiscal = text(v.get("fiscalSatus")).unwrap_or_default();
    let ok = status.as_deref() == Some("SUCCESS") || (log.is_none() && fiscal == "FINISHED");
    match (ok, fic) {
        (true, Some(fic)) => Created::Fiscalised {
            sale_id,
            iic: text(log.and_then(|l| l.get("iic"))).unwrap_or_default(),
            fic,
            inv_ord_num: v.get("invOrdNum").map(|n| n.to_string().trim_matches('"').to_string()).unwrap_or_default(),
        },
        _ => {
            let fault = text(log.and_then(|l| l.get("faultStringMsg")))
                .or_else(|| text(log.and_then(|l| l.get("faultString"))))
                .unwrap_or_else(|| format!("{} {fiscal}", status.unwrap_or_else(|| "no fiscalisation log".into())));
            Created::Unfiscalised { sale_id, fault: fault.chars().take(200).collect() }
        }
    }
}

#[cfg(test)]
mod tests;
