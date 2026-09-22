//! The waiting list: the one thing a visitor to dowiz.org can do.
//!
//! The landing page carries no price, so its form asks for the venue's email
//! and nothing else. `POST /api/waitlist` writes the row first and rings the
//! bell second: the row in D1 is the record an administrator reads in the hub
//! console, and the mail to the operator (`WAITLIST_TO`) is a convenience that
//! rides on Cloudflare's `send_email` binding when it is configured. A missing
//! binding or a refused send is logged and never shown to the visitor, because
//! the visitor did their part the moment the row was written.
//!
//! No token, no captcha: the endpoint accepts one address per submit, keys the
//! table by that address, and caps the free-text field, so the worst an
//! abuser can do is fill a list an administrator reads by hand.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::now_ms;
use crate::platform::admin_only;

/// The longest venue name kept; anything past it is cut, not refused.
const VENUE_MAX: usize = 120;
/// RFC 5321's limit on a path; longer is not an address anybody can mail.
const EMAIL_MAX: usize = 254;
/// The languages the landing page is served in; anything else is recorded as
/// the default rather than as whatever the client sent.
const LANGS: &[&str] = &["uk", "en", "sq"];
/// How many rows the console reads at once. The list is read by a person.
const LIST_MAX: usize = 500;

/// The record kind inside the platform's waitlist image. One kind, so `all`
/// lists exactly the waitlist and nothing that shares the image later.
const KIND: &str = "wl";

#[derive(Deserialize)]
struct Join {
    email: String,
    #[serde(default)]
    venue: String,
    #[serde(default)]
    lang: String,
}

/// The cheapest check that is still a check: one `@`, something on both
/// sides, a dot in the domain, no whitespace. Deliverability is the mail
/// server's question, not this function's.
pub fn plausible_email(s: &str) -> bool {
    if s.len() > EMAIL_MAX || s.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, domain)) = s.split_once('@') else { return false };
    !local.is_empty()
        && domain.len() >= 3
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains('@')
}

/// The mail the operator reads. Plain text, 7-bit safe headers, the body in
/// UTF-8; a MIME parser would call this the minimum, and that is the point.
pub fn mail_raw(from: &str, to: &str, email: &str, venue: &str, lang: &str, source: &str, at_ms: i64) -> String {
    let subject = if venue.is_empty() {
        format!("dowiz waitlist: {email}")
    } else {
        format!("dowiz waitlist: {venue} <{email}>")
    };
    // Header values are ASCII-only here: the subject may carry a venue name in
    // any script, so it is RFC 2047 encoded when it has to be.
    let subject = if subject.is_ascii() { subject } else { format!("=?UTF-8?B?{}?=", b64(subject.as_bytes())) };
    let venue_line = if venue.is_empty() { "(no venue name)".to_string() } else { venue.to_string() };
    format!(
        "From: dowiz <{from}>\r\nTo: <{to}>\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n\
         A venue left its address on dowiz.org.\r\n\r\n\
         email:  {email}\r\nvenue:  {venue_line}\r\nlang:   {lang}\r\nsource: {source}\r\nat_ms:  {at_ms}\r\n\r\n\
         The row is in the hub console under \"Список очікування\".\r\n"
    )
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (chunk.get(1).copied().unwrap_or(0) as u32) << 8
            | chunk.get(2).copied().unwrap_or(0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// `POST /api/waitlist` — `{email, venue?, lang?}`. 204 once the row is
/// written; the mail is best-effort and reported only in the row.
pub async fn join(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: Join = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("expected {email, venue?, lang?}", 400),
    };
    let email = body.email.trim().to_ascii_lowercase();
    if !plausible_email(&email) {
        return Response::error("that is not an email address", 400);
    }
    let venue: String = body.venue.trim().chars().take(VENUE_MAX).collect();
    let lang = if LANGS.contains(&body.lang.as_str()) { body.lang.as_str() } else { "uk" };
    let source = req
        .headers()
        .get("host")
        .ok()
        .flatten()
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let now = now_ms();

    // ONE RECORD PER ADDRESS, in the platform's bebop image. A second submit
    // refreshes the venue name and the time rather than making a duplicate; the
    // FIRST submit's `at_ms` survives, which is the half of `ON CONFLICT DO
    // UPDATE` that had to be read carefully -- `at_ms` was excluded from the
    // update list and nothing said why.
    //
    // No SQL. The upsert is a read of the existing record inside the object's
    // own single-threaded turn, which is also what makes "keep the first
    // `at_ms`" correct rather than racy.
    {
        let e = email.clone();
        let v = venue.clone();
        let sc = source.clone();
        crate::platform_store::with(&ctx.env, crate::platform_store::WAITLIST, move |t| {
            let first = t
                .get(KIND, &e)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .and_then(|r| r.get("at_ms").and_then(Value::as_i64))
                .unwrap_or(now);
            let notified = t
                .get(KIND, &e)
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .and_then(|r| r.get("notified_ms").and_then(Value::as_i64));
            let rec = json!({
                "email": e, "venue": v, "lang": lang, "source": sc,
                "at_ms": first, "updated_ms": now, "notified_ms": notified,
            });
            t.put(KIND, &e, &rec.to_string(), &[], &[])
                .map_err(|x| Error::RustError(format!("waitlist: {x}")))?;
            Ok(())
        })
        .await?;
    }

    // THE BELL. Configured = the binding exists AND both addresses are set;
    // anything else is "store only", written to the log once per submit so an
    // administrator reading the log knows why the mailbox is quiet.
    let to = ctx.var("WAITLIST_TO").map(|v| v.to_string()).unwrap_or_default();
    let from = ctx.var("WAITLIST_FROM").map(|v| v.to_string()).unwrap_or_default();
    match (ctx.env.send_email("WAITLIST_MAIL"), to.is_empty() || from.is_empty()) {
        (Ok(mailer), false) => {
            let raw = mail_raw(&from, &to, &email, &venue, lang, &source, now);
            let sent = match EmailMessage::new(&from, &to, &raw) {
                Ok(msg) => mailer.send(&msg).await.map(|_| ()).map_err(|e| format!("{e:?}")),
                Err(e) => Err(format!("{e:?}")),
            };
            match sent {
                Ok(()) => {
                    let e = email.clone();
                    crate::platform_store::with(
                        &ctx.env,
                        crate::platform_store::WAITLIST,
                        move |t| {
                            // Read-modify-write inside the object's turn, so
                            // the mark cannot overwrite a refresh that arrived
                            // between the store above and the mail returning.
                            let Some(j) = t.get(KIND, &e) else { return Ok(()) };
                            let mut rec: Value = serde_json::from_str(&j).unwrap_or(json!({}));
                            rec["notified_ms"] = json!(now);
                            t.put(KIND, &e, &rec.to_string(), &[], &[])
                                .map_err(|x| Error::RustError(format!("waitlist: {x}")))?;
                            Ok(())
                        },
                    )
                    .await?;
                }
                Err(e) => console_error!("waitlist: row written, mail to {to} refused: {e}"),
            }
        }
        _ => console_log!("waitlist: row written for {email}; no WAITLIST_MAIL binding, mail not sent"),
    }

    Response::empty().map(|r| r.with_status(204))
}

/// `GET /api/platform/waitlist` — every row, newest first. Administrators only.
pub async fn list(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Err(r) = admin_only(&req, &ctx).await {
        return Ok(r);
    }
    #[derive(Deserialize, serde::Serialize)]
    struct Row {
        email: String,
        venue: Option<String>,
        lang: String,
        source: Option<String>,
        at_ms: i64,
        updated_ms: i64,
        notified_ms: Option<i64>,
    }
    // `ORDER BY updated_ms DESC LIMIT n` with no index, and on purpose: a
    // waitlist is hundreds of records, so the sort is cheaper than the index
    // entry every write would have to maintain. An index is a key written ON
    // PURPOSE, and this one would not have earned its keep. When it does, it is
    // `wl.recent/<complement of updated_ms>/<email>` and a prefix scan.
    let loaded = crate::platform_store::load(&ctx.env, crate::platform_store::WAITLIST).await?;
    let mut rows: Vec<Row> = loaded
        .table
        .all(KIND)
        .into_iter()
        .filter_map(|(_, j)| serde_json::from_str::<Row>(&j).ok())
        .collect();
    rows.sort_by(|a, b| b.updated_ms.cmp(&a.updated_ms));
    rows.truncate(LIST_MAX);
    Response::from_json(&json!({ "rows": rows }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_needs_a_local_part_a_domain_and_a_dot() {
        assert!(plausible_email("kitchen@dubin-sushi.al"));
        assert!(plausible_email("a@b.co"));
        assert!(!plausible_email("kitchen"));
        assert!(!plausible_email("@dubin.al"));
        assert!(!plausible_email("kitchen@localhost"));
        assert!(!plausible_email("kitchen@.al"));
        assert!(!plausible_email("kit chen@dubin.al"));
        assert!(!plausible_email("k@d.al@x.al"));
    }

    #[test]
    fn the_mail_names_the_venue_and_encodes_a_non_ascii_subject() {
        let raw = mail_raw("waitlist@dowiz.org", "op@example.com", "k@d.al", "Dubin & Sushi", "sq", "dowiz.org", 7);
        assert!(raw.starts_with("From: dowiz <waitlist@dowiz.org>\r\nTo: <op@example.com>\r\n"));
        assert!(raw.contains("Subject: dowiz waitlist: Dubin & Sushi <k@d.al>\r\n"));
        assert!(raw.contains("\r\n\r\nA venue left"));
        assert!(raw.contains("venue:  Dubin & Sushi\r\n"));
        let uk = mail_raw("w@dowiz.org", "op@example.com", "k@d.al", "Кав'ярня", "uk", "dowiz.org", 7);
        assert!(uk.contains("Subject: =?UTF-8?B?"));
        assert!(!uk.lines().next().unwrap().contains('К'));
    }

    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"f"), "Zg==");
        assert_eq!(b64(b"fo"), "Zm8=");
        assert_eq!(b64(b"foo"), "Zm9v");
        assert_eq!(b64("Кав".as_bytes()), "0JrQsNCy");
    }
}
