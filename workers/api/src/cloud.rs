//! The venue's off-site copy, in a bucket the venue owns.
//!
//! Any S3-compatible store: Cloudflare R2, AWS S3, Backblaze B2, Wasabi,
//! MinIO. Keys alone connect one, which is why this and not a Drive OAuth
//! dance is what "cloud storage" means here. The object is the same bundle
//! `GET /api/owner/backup` downloads, so `POST /api/owner/restore` reads it
//! back unchanged.
//!
//! Signature Version 4 is written out here rather than pulled in: the AWS SDK
//! does not compile to `wasm32-unknown-unknown`, and a PUT needs exactly one
//! canonical request. Every step is named after the AWS document it follows.

use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use worker::*;

/// The SigV4 algorithm name, verbatim.
const ALGORITHM: &str = "AWS4-HMAC-SHA256";
/// The service the credential scope names.
const SERVICE: &str = "s3";
/// The headers every request signs, in the sorted order SigV4 wants.
const SIGNED_HEADERS: &str = "host;x-amz-content-sha256;x-amz-date";
/// How much of a store's refusal is shown to the owner.
const ERROR_SHOWN: usize = 300;
/// Cron: one copy a night, at 03:17 UTC, when no kitchen is open (see wrangler.toml).
pub const NIGHTLY_CRON: &str = "17 3 * * *";
/// The settings key that remembers the last successful push. Written by the
/// hub itself, never by the owner, so it is deliberately not in `KNOWN`.
const LAST_KEY: &str = "cloud.last";

type HmacSha256 = Hmac<Sha256>;

pub struct S3 {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub key: String,
    pub secret: String,
    pub prefix: String,
}

/// The venue's store, if all four required values are set.
pub fn cfg(s: &dowiz_hub::settings::Settings) -> Option<S3> {
    let get = |k: &str| s.known(k).trim().to_string();
    let (endpoint, bucket, key, secret) =
        (get("cloud.s3.endpoint"), get("cloud.s3.bucket"), get("cloud.s3.key"), get("cloud.s3.secret"));
    if endpoint.is_empty() || bucket.is_empty() || key.is_empty() || secret.is_empty() {
        return None;
    }
    let region = { let r = get("cloud.s3.region"); if r.is_empty() { "auto".into() } else { r } };
    let prefix = get("cloud.s3.prefix").trim_matches('/').to_string();
    Some(S3 { endpoint: endpoint.trim_end_matches('/').to_string(), region, bucket, key, secret, prefix })
}

fn hmac(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut m = HmacSha256::new_from_slice(key).expect("hmac accepts any key length");
    m.update(msg);
    m.finalize().into_bytes().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Civil date from days since 1970-01-01 (Howard Hinnant's algorithm), so the
/// `x-amz-date` stamp needs no calendar crate.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `YYYYMMDDTHHMMSSZ` and `YYYYMMDD` for a unix millisecond stamp.
pub fn amz_dates(ms: i64) -> (String, String) {
    let secs = ms.div_euclid(1000);
    let (y, m, d) = civil(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    let date = format!("{y:04}{m:02}{d:02}");
    (format!("{date}T{:02}{:02}{:02}Z", s / 3600, (s / 60) % 60, s % 60), date)
}

/// The host part of the endpoint, which SigV4 signs.
fn host_of(endpoint: &str) -> &str {
    endpoint.trim_start_matches("https://").trim_start_matches("http://").split('/').next().unwrap_or("")
}

/// Sign and send one PUT. Path-style addressing (`/bucket/key`) works on every
/// store above; virtual-host style does not work on MinIO or a local R2 token.
pub async fn put(s3: &S3, object_key: &str, body: Vec<u8>, content_type: &str, now_ms: i64) -> std::result::Result<String, String> {
    let (amz_date, date) = amz_dates(now_ms);
    let host = host_of(&s3.endpoint);
    let path = format!("/{}/{}", s3.bucket, object_key);
    let payload_hash = hex(&Sha256::digest(&body));
    let canonical_headers = format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let canonical_request = format!("PUT\n{path}\n\n{canonical_headers}\n{SIGNED_HEADERS}\n{payload_hash}");
    let scope = format!("{date}/{}/{SERVICE}/aws4_request", s3.region);
    let string_to_sign = format!("{ALGORITHM}\n{amz_date}\n{scope}\n{}", hex(&Sha256::digest(canonical_request.as_bytes())));
    let k_date = hmac(format!("AWS4{}", s3.secret).as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, s3.region.as_bytes());
    let k_service = hmac(&k_region, SERVICE.as_bytes());
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex(&hmac(&k_signing, string_to_sign.as_bytes()));
    let authorization = format!("{ALGORITHM} Credential={}/{scope}, SignedHeaders={SIGNED_HEADERS}, Signature={signature}", s3.key);

    let headers = Headers::new();
    headers.set("authorization", &authorization).map_err(|e| e.to_string())?;
    headers.set("x-amz-date", &amz_date).map_err(|e| e.to_string())?;
    headers.set("x-amz-content-sha256", &payload_hash).map_err(|e| e.to_string())?;
    headers.set("content-type", content_type).map_err(|e| e.to_string())?;
    let url = format!("{}{path}", s3.endpoint);
    let r = Request::new_with_init(
        &url,
        RequestInit::new().with_method(Method::Put).with_headers(headers).with_body(Some(body.into())),
    )
    .map_err(|e| e.to_string())?;
    let mut res = Fetch::Request(r).send().await.map_err(|e| e.to_string())?;
    if res.status_code() < 300 {
        return Ok(res.headers().get("etag").ok().flatten().unwrap_or_default());
    }
    let text = res.text().await.unwrap_or_default();
    Err(format!("{} {}", res.status_code(), text.chars().take(ERROR_SHOWN).collect::<String>()))
}

/// Where the bundle lands: `<prefix>/<venue>/<YYYYMMDD>T<HHMMSS>Z.json`.
fn object_key(s3: &S3, venue: &str, now_ms: i64) -> String {
    let (stamp, _) = amz_dates(now_ms);
    if s3.prefix.is_empty() { format!("{venue}/{stamp}.json") } else { format!("{}/{venue}/{stamp}.json", s3.prefix) }
}

/// Export the venue and put it in its bucket. Returns what was written.
pub async fn push_place(place: &crate::hubstore::Place, now_ms: i64) -> std::result::Result<Value, String> {
    let settings = crate::hubstore::load_settings(place).await.map_err(|e| e.to_string())?.settings;
    let Some(s3) = cfg(&settings) else { return Err("no cloud storage is set".into()) };
    let bundle = crate::hubstore::export(place).await.map_err(|e| e.to_string())?;
    let body = serde_json::to_vec(&bundle).map_err(|e| e.to_string())?;
    let key = object_key(&s3, &place.venue, now_ms);
    let bytes = body.len();
    let etag = put(&s3, &key, body, "application/json", now_ms).await?;
    let note = format!("{now_ms} {key} {bytes}");
    let _ = crate::hubstore::with_settings(place, move |s| { s.set(LAST_KEY, &note); Ok(()) }).await;
    Ok(json!({ "key": key, "bucket": s3.bucket, "bytes": bytes, "etag": etag, "atMs": now_ms }))
}

/// `POST /api/owner/backup/cloud` — push now.
pub async fn push(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    if let Err(r) = crate::owner::owner_and_venue(&req, &ctx, &db).await {
        return Ok(r);
    }
    match push_place(&place, crate::owner::now_ms()).await {
        Ok(v) => Response::from_json(&v),
        Err(e) => Response::error(e, 502),
    }
}

/// `GET /api/owner/backup/cloud` — is a store set, and when did it last take a copy.
pub async fn status(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    if let Err(r) = crate::owner::owner_and_venue(&req, &ctx, &db).await {
        return Ok(r);
    }
    let settings = crate::hubstore::load_settings(&place).await?.settings;
    let last = settings.get(LAST_KEY).unwrap_or_default();
    let mut it = last.split(' ');
    let at_ms = it.next().and_then(|s| s.parse::<i64>().ok());
    let key = it.next().map(str::to_string);
    let bytes = it.next().and_then(|s| s.parse::<usize>().ok());
    Response::from_json(&json!({
        "configured": cfg(&settings).is_some(),
        "bucket": settings.known("cloud.s3.bucket"),
        "endpoint": settings.known("cloud.s3.endpoint"),
        "nightly": NIGHTLY_CRON,
        "last": at_ms.map(|at| json!({ "atMs": at, "key": key, "bytes": bytes })),
    }))
}

/// The nightly cron: every venue with a store set gets a copy. A venue whose
/// store refuses is logged and skipped; the next venue is not its problem.
pub async fn nightly(env: &Env) {
    #[derive(serde::Deserialize)]
    struct Row { id: String }
    let Ok(db) = env.d1("DB") else { console_error!("nightly backup: no DB"); return };
    let rows = match db.prepare("SELECT id FROM locations").all().await.and_then(|r| r.results::<Row>()) {
        Ok(r) => r,
        Err(e) => { console_error!("nightly backup: locations unreadable: {e}"); return }
    };
    let legacy = env.var("LEGACY_VENUE").ok().map(|v| v.to_string()).filter(|v| !v.is_empty());
    let now = crate::owner::now_ms();
    for r in rows {
        let (Ok(db), Ok(ns)) = (env.d1("DB"), env.durable_object("HUB")) else { continue };
        let place = crate::hubstore::Place { db, ns, venue: r.id.clone(), legacy_venue: legacy.clone() };
        let configured = match crate::hubstore::load_settings(&place).await {
            Ok(l) => cfg(&l.settings).is_some(),
            Err(_) => false,
        };
        if !configured { continue }
        match push_place(&place, now).await {
            Ok(v) => console_log!("nightly backup {}: {}", r.id, v),
            Err(e) => console_error!("nightly backup {} refused: {e}", r.id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_utc_and_zero_padded() {
        // 2026-09-19T09:05:07Z
        let (stamp, date) = amz_dates(1_789_808_707_000);
        assert_eq!(date, "20260919");
        assert_eq!(stamp, "20260919T090507Z");
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(-1), (1969, 12, 31));
    }

    #[test]
    fn signing_key_matches_the_aws_worked_example() {
        // docs.aws.amazon.com/general/latest/gr/sigv4-calculate-signature.html
        let k_date = hmac(b"AWS4wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", b"20150830");
        let k_region = hmac(&k_date, b"us-east-1");
        let k_service = hmac(&k_region, b"iam");
        let k_signing = hmac(&k_service, b"aws4_request");
        assert_eq!(hex(&k_signing), "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9");
    }
}
