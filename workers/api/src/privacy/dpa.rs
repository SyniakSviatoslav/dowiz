//! THE dowiz <-> VENUE DATA PROCESSING AGREEMENT (P9; Law 124/2024 Art. 26,
//! GDPR Art. 28).
//!
//! The text is `docs/privacy/DPA-v1-2026-09-24.{sq,en,uk}.md`, compiled in, so
//! the version served is the version in the tree. Acceptance is a fact on the
//! venue's `loc` record in the platform registry: `dpa_version`,
//! `dpa_accepted_at_ms`, `dpa_accepted_by`.
//!
//! * `POST /api/platform/hubs` refuses (400) a hub whose body does not carry
//!   `"dpa": VERSION` — the operator's click at creation (`platform.rs`).
//! * `GET /api/owner/dpa` and `POST /api/owner/dpa/accept` let an existing
//!   venue's owner read and accept it from the console.
//! * `GET /dpa?lang=` serves the text as a page.

#[cfg(test)]
mod tests;

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

/// The agreement's version. A new text is a new version, and every venue
/// shows as not having accepted it until its owner does.
pub const VERSION: &str = "dpa v1 2026-09-24";

const SQ: &str = include_str!("../../../../docs/privacy/DPA-v1-2026-09-24.sq.md");
const EN: &str = include_str!("../../../../docs/privacy/DPA-v1-2026-09-24.en.md");
const UK: &str = include_str!("../../../../docs/privacy/DPA-v1-2026-09-24.uk.md");

/// The text in a language; Albanian otherwise.
pub fn text(lang: &str) -> &'static str {
    match lang {
        "en" => EN,
        "uk" => UK,
        _ => SQ,
    }
}

/// The acceptance a hub creation (or an owner) sent: exactly this version.
pub fn check(accepted: Option<&str>) -> std::result::Result<(), String> {
    match accepted.map(str::trim) {
        Some(v) if v == VERSION => Ok(()),
        Some(v) => Err(format!("the data processing agreement accepted ({v}) is not the current one ({VERSION}); read /dpa and accept it")),
        None => Err(format!("the data processing agreement must be accepted: send \"dpa\": \"{VERSION}\" after reading /dpa")),
    }
}

/// Write the acceptance onto a `loc` record.
pub fn stamp(rec: &mut Value, at_ms: i64, by: &str) {
    rec["dpa_version"] = json!(VERSION);
    rec["dpa_accepted_at_ms"] = json!(at_ms);
    rec["dpa_accepted_by"] = json!(by);
}

/// What the console shows: the version the venue accepted, and whether it is
/// the current one.
pub fn state(rec: Option<&Value>) -> Value {
    let got = rec.and_then(|r| r.get("dpa_version")).and_then(Value::as_str);
    json!({
        "version": VERSION,
        "accepted": got.map(|v| json!({
            "version": v,
            "atMs": rec.and_then(|r| r.get("dpa_accepted_at_ms")).cloned().unwrap_or(Value::Null),
            "by": rec.and_then(|r| r.get("dpa_accepted_by")).cloned().unwrap_or(Value::Null),
        })),
        "current": got == Some(VERSION),
        "url": "/dpa",
    })
}

/// The markdown subset the texts use: `#` headings, `-` items, paragraphs.
/// Escaped first, so nothing in a text can become markup.
pub fn to_html(md: &str) -> String {
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut out = String::new();
    let mut in_list = false;
    for block in md.split("\n\n") {
        for line in block.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()) {
            let item = line.strip_prefix("- ");
            if item.is_none() && in_list {
                out.push_str("</ul>");
                in_list = false;
            }
            if let Some(i) = item {
                if !in_list {
                    out.push_str("<ul>");
                    in_list = true;
                }
                out.push_str(&format!("<li>{}</li>", esc(i)));
            } else if let Some(h) = line.strip_prefix("## ") {
                out.push_str(&format!("<h2>{}</h2>", esc(h)));
            } else if let Some(h) = line.strip_prefix("# ") {
                out.push_str(&format!("<h1>{}</h1>", esc(h)));
            } else {
                out.push_str(&format!("<p>{}</p>", esc(line)));
            }
        }
    }
    if in_list {
        out.push_str("</ul>");
    }
    out
}

fn lang_of(req: &Request) -> String {
    req.url().ok().and_then(|u| u.query_pairs().find(|(k, _)| k == "lang").map(|(_, v)| v.to_string())).unwrap_or_else(|| "sq".into())
}

/// `GET /dpa?lang=sq|en|uk`
pub async fn page(req: Request, _ctx: RouteContext<crate::Req>) -> Result<Response> {
    let lang = lang_of(&req);
    let l = if ["sq", "en", "uk"].contains(&lang.as_str()) { lang } else { "sq".into() };
    let body = format!(
        "<!doctype html><html lang=\"{l}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>DPA</title><link rel=\"stylesheet\" href=\"/store/legal.css\"></head><body><main class=\"legal\">\
         <nav class=\"langs\"><a href=\"?lang=sq\">SQ</a> · <a href=\"?lang=en\">EN</a> · <a href=\"?lang=uk\">UK</a></nav>{}</main></body></html>",
        to_html(text(&l))
    );
    let mut res = Response::from_html(body)?;
    res.headers_mut().set("content-security-policy", "default-src 'none'; style-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'")?;
    res.headers_mut().set("cache-control", "public, max-age=300")?;
    Ok(res)
}

/// `GET /api/owner/dpa` — the venue's acceptance, read from its `loc` record.
pub async fn read(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    let t = crate::identity_store::registry(&ctx.env).await?;
    Response::from_json(&state(crate::identity_store::rec(&t, crate::identity_store::K_LOC, &loc).as_ref()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptIn {
    #[allow(dead_code)]
    location_id: Option<String>,
    version: Option<String>,
}

/// `POST /api/owner/dpa/accept {version}` — the owner accepts the current text.
pub async fn accept(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let body: AcceptIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if let Err(why) = check(body.version.as_deref()) {
        return Response::error(why, 400);
    }
    let (user, loc) = match crate::owner::owner_and_venue(&req, &ctx).await {
        Ok(x) => x,
        Err(r) => return Ok(r),
    };
    let now = ctx.data.now_ms;
    let by = format!("owner:{user}");
    let l2 = loc.clone();
    let out = crate::identity_store::with_registry(&ctx.env, move |t| {
        let Some(mut r) = crate::identity_store::rec(t, crate::identity_store::K_LOC, &l2) else {
            return Ok(None);
        };
        stamp(&mut r, now, &by);
        let slug = crate::identity_store::s_of(&r, "slug");
        let index = vec![(crate::identity_store::loc_by_slug(&slug), l2.clone())];
        t.put(crate::identity_store::K_LOC, &l2, &r.to_string(), &index, &[])
            .map_err(|e| Error::RustError(format!("registry: {e}")))?;
        Ok(Some(state(Some(&r))))
    })
    .await?;
    match out {
        Some(v) => Response::from_json(&v),
        None => Response::error("this venue has no registry record", 404),
    }
}
