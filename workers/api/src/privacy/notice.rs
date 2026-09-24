//! THE PRIVACY NOTICE (P8): `GET /privacy?lang=sq|en|uk` on a venue's host.
//!
//! RENDERED, NOT WRITTEN. Every row of "what is kept" is a row of
//! `registry::stores()` about a customer; every recipient is a row of
//! `registry::PROCESSORS` whose switch is ON for this venue (Telegram only
//! when a bot and chat are set, and so on); the backup window is computed from
//! `cloud::KEEP_WEEKLY_MS`. A store added without a row fails the
//! `personal-data` gate, and a row added appears here without touching this
//! file. The words live in `notice/{sq,en,uk}.rs`.
//!
//! On the platform's own host the same route answers dowiz's notice as a
//! controller (owner/staff accounts, the waiting list).

mod en;
mod sq;
mod uk;
pub mod words;
#[cfg(test)]
mod tests;

use serde_json::Value;
use worker::*;

use super::registry::{self, Data, Eraser, Home, Processor, Retention, Store, Subject, Switch, BROWSER, PROCESSORS};
use words::Words;

/// The text's version. Change it with the words.
pub const VERSION: &str = "privacy-notice v1 2026-09-24";
/// Where questions about dowiz itself go.
pub const DOWIZ_PRIVACY_EMAIL: &str = "privacy@dowiz.org";
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// The controller, as its own record names it.
pub struct Venue {
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
}

/// Which of a venue's services are switched on. The handler reads them from
/// the venue's settings; the renderer is pure.
#[derive(Clone, Copy, Default)]
pub struct On {
    pub telegram: bool,
    pub meta: bool,
    pub cloud: bool,
    pub stripe: bool,
    pub ai: bool,
}

impl On {
    pub fn has(&self, s: Switch) -> bool {
        match s {
            Switch::Always | Switch::Map => true,
            Switch::Telegram => self.telegram,
            Switch::Meta => self.meta,
            Switch::Cloud => self.cloud,
            Switch::Stripe => self.stripe,
            Switch::Ai => self.ai,
            Switch::FiscalSend => crate::fiscal::SEND_ENABLED,
            // Import-only: the venue's own login goes there, no customer data.
            // `recipients` drops it by what it receives; this says the same.
            Switch::EbillsLink => false,
        }
    }
}

/// The parties a customer of this venue must be told about: switched on
/// here, and receiving something that is about a customer.
pub fn recipients(on: &On) -> Vec<&'static Processor> {
    PROCESSORS
        .iter()
        .filter(|p| on.has(p.switch))
        .filter(|p| p.receives.iter().any(|d| !matches!(d, Data::StaffId | Data::Password)))
        .collect()
}

/// How long the last nightly copy holding an erased person survives: the
/// weekly window, plus the night it was taken.
pub fn backup_days() -> i64 {
    crate::cloud::KEEP_WEEKLY_MS / DAY_MS + 1
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

fn list(w: &Words, d: &[Data]) -> String {
    d.iter().map(|x| (w.data)(*x)).collect::<Vec<_>>().join(", ")
}

fn how_long(w: &Words, r: Retention) -> String {
    match r {
        Retention::Ms(ms, _) if ms <= DAY_MS => w.one_day.to_string(),
        Retention::Ms(ms, _) => w.days.replace("{n}", &(ms / DAY_MS).to_string()),
        Retention::Latest => w.latest.to_string(),
        Retention::UntilDone(_) => w.until_done.to_string(),
        Retention::NoLimitYet(_) => w.no_limit.to_string(),
    }
}

fn on_erasure(w: &Words, e: Eraser) -> &'static str {
    match e {
        Eraser::Redact(_) | Eraser::Remove(_) => w.erased,
        Eraser::Retain(_) => w.retained,
        Eraser::Expires(_) => w.expires,
        Eraser::Missing(_) => w.not_reached,
        Eraser::NotPersonal(_) => "",
    }
}

fn rows(w: &Words, keep: &[&Store]) -> String {
    let mut out = format!(
        "<table><thead><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr></thead><tbody>",
        esc(w.col_what), esc(w.col_why), esc(w.col_basis), esc(w.col_long), esc(w.col_erase)
    );
    for s in keep {
        out.push_str(&format!(
            "<tr data-store=\"{}\"><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(s.image), esc(&list(w, s.holds)), esc((w.purpose)(s.purpose)), esc((w.basis)(s.basis)),
            esc(&how_long(w, s.retention)), esc(on_erasure(w, s.erase))
        ));
    }
    out.push_str("</tbody></table>");
    out
}

fn section(h: &str, body: &str) -> String {
    format!("<section><h2>{}</h2>{body}</section>", esc(h))
}

fn p(s: &str) -> String {
    format!("<p>{}</p>", esc(s))
}

fn page(w: &Words, title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"{}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{}</title><link rel=\"stylesheet\" href=\"/store/legal.css\"></head><body><main class=\"legal\">\
         <h1>{}</h1><p class=\"ver\">{} {}</p><nav class=\"langs\">{}</nav>{body}</main></body></html>",
        w.lang, esc(title), esc(title), esc(w.version), esc(VERSION),
        words::LANGS.iter().map(|l| format!("<a href=\"?lang={l}\" hreflang=\"{l}\">{}</a>", l.to_uppercase())).collect::<Vec<_>>().join(" · "),
    )
}

/// The recipients block: name, what it receives, where, on what terms.
fn recipients_html(w: &Words, on: &On) -> String {
    let mut out = p(w.recipients_intro);
    out.push_str("<ul class=\"recipients\">");
    for r in recipients(on) {
        let (loc, safe) = (w.processor)(r.id).unwrap_or((r.location, r.safeguard));
        out.push_str(&format!(
            "<li data-processor=\"{}\"><strong>{}</strong><br>{}: {}<br>{}: {}<br>{}: {}{}</li>",
            esc(r.id), esc(r.name), esc(w.receives), esc(&list(w, r.receives)), esc(w.where_), esc(loc), esc(w.safeguard), esc(safe),
            if r.terms.starts_with("https://") { format!(" <a href=\"{0}\" rel=\"noopener\">{0}</a>", esc(r.terms)) } else { String::new() }
        ));
    }
    out.push_str("</ul>");
    out
}

/// A venue's notice, whole.
pub fn render_venue(v: &Venue, on: &On, lang: &str) -> String {
    let w = words::words(lang);
    let mut who = p(&w.who.replace("{venue}", &v.name));
    if let Some(a) = &v.address {
        who.push_str(&p(&w.who_address.replace("{address}", a)));
    }
    if let Some(ph) = &v.phone {
        who.push_str(&p(&w.who_phone.replace("{phone}", ph)));
    }
    who.push_str(&p(w.dowiz_role));
    let keep: Vec<&Store> = registry::stores().filter(|s| s.personal() && s.about(Subject::Customer)).collect();
    let mut device: Vec<Data> = Vec::new();
    for k in BROWSER.iter().filter(|k| k.surface == "store" || k.surface == "kit") {
        for d in k.holds {
            if !device.contains(d) {
                device.push(*d);
            }
        }
    }
    let mut backups = String::new();
    if on.cloud {
        backups.push_str(&p(&w.backups.replace("{days}", &backup_days().to_string())));
    }
    backups.push_str(&p(w.recovery));
    let contact = match (&v.phone, &v.address) {
        (Some(ph), _) => ph.clone(),
        (None, Some(a)) => a.clone(),
        (None, None) => v.name.clone(),
    };
    let rights = format!(
        "<ul>{}</ul>{}{}",
        w.rights.iter().map(|r| format!("<li>{}</li>", esc(r))).collect::<String>(),
        p(&w.how_to_ask.replace("{venue}", &v.name).replace("{contact}", &contact)),
        p(w.deadline)
    );
    let body = [
        section(w.who_h, &who),
        section(w.keep_h, &format!("{}{}{}", p(w.keep_intro), rows(w, &keep), p(w.must_give))),
        section(w.device_h, &p(&w.device.replace("{data}", &list(w, &device)))),
        section(w.recipients_h, &recipients_html(w, on)),
        section(w.backups_h, &backups),
        section(w.rights_h, &rights),
        section(w.marketing_h, &p(w.marketing)),
        section(w.automated_h, &p(w.automated)),
        section(w.complaint_h, &format!("{}{}", p(w.complaint), p(&w.dowiz_contact.replace("{email}", DOWIZ_PRIVACY_EMAIL)))),
    ]
    .concat();
    page(w, &format!("{} — {}", w.title, v.name), &body)
}

/// dowiz's own notice, as a controller: the platform's stores about owners,
/// staff and prospects.
pub fn render_platform(lang: &str) -> String {
    let w = words::words(lang);
    let keep: Vec<&Store> = registry::stores()
        .filter(|s| s.home == Home::Platform && s.personal() && (s.about(Subject::Owner) || s.about(Subject::Prospect)))
        .collect();
    let rights = format!(
        "<ul>{}</ul>{}{}",
        w.rights.iter().map(|r| format!("<li>{}</li>", esc(r))).collect::<String>(),
        p(&w.dowiz_contact.replace("{email}", DOWIZ_PRIVACY_EMAIL)),
        p(w.deadline)
    );
    let body = [
        section(w.who_h, &format!("{}{}", p(w.platform_who), p(w.platform_venues))),
        section(w.keep_h, &rows(w, &keep)),
        section(w.recipients_h, &recipients_html(w, &On::default())),
        section(w.rights_h, &rights),
        section(w.complaint_h, &p(w.complaint)),
    ]
    .concat();
    page(w, w.platform_title, &body)
}

fn lang_of(req: &Request, fallback: &str) -> String {
    let asked = req.url().ok().and_then(|u| u.query_pairs().find(|(k, _)| k == "lang").map(|(_, v)| v.to_string()));
    let l = asked.unwrap_or_else(|| fallback.to_string());
    if words::LANGS.contains(&l.as_str()) { l } else { "sq".to_string() }
}

fn html(body: String) -> Result<Response> {
    let mut res = Response::from_html(body)?;
    let h = res.headers_mut();
    h.set("content-security-policy", "default-src 'none'; style-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'")?;
    h.set("cache-control", "public, max-age=300")?;
    Ok(res)
}

/// A setting that holds something (the same test as the integrations screen).
fn set(s: &dowiz_hub::settings::Settings, key: &str) -> bool {
    !s.known(key).trim().is_empty() || s.get(key).is_some_and(|v| !v.trim().is_empty())
}

/// `GET /privacy` — the venue is the HOST's, never a parameter.
pub async fn serve(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let Some(slug) = crate::hubstore::Place::slug_of_host(&req, &ctx) else {
        return html(render_platform(&lang_of(&req, "en")));
    };
    let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
    let loc: Value = match crate::hubstore::load_catalog(&place).await?.catalog.location() {
        Some(j) => serde_json::from_str(&j).unwrap_or(Value::Null),
        None => return Response::error("not found", 404),
    };
    let s = crate::hubstore::load_settings(&place).await?.settings;
    let text = |k: &str| loc.get(k).and_then(Value::as_str).map(str::trim).filter(|x| !x.is_empty()).map(str::to_string);
    let venue = Venue { name: text("name").unwrap_or_else(|| slug.clone()), address: text("address"), phone: text("phone") };
    let on = On {
        telegram: crate::notify::bot_token(&ctx.env, &s).is_some() && set(&s, "notify.telegram.chat"),
        meta: crate::channels::whatsapp_cfg(&s).is_some() || crate::channels::instagram_cfg(&s).is_some(),
        cloud: crate::cloud::cfg(&s).is_some(),
        stripe: ctx.env.secret("STRIPE_PUBLISHABLE_KEY").is_ok(),
        ai: s.flag("ai.enabled") && s.known("ai.endpoint").starts_with("https://"),
    };
    let lang = lang_of(&req, loc.get("default_locale").and_then(Value::as_str).unwrap_or("sq"));
    html(render_venue(&venue, &on, &lang))
}
