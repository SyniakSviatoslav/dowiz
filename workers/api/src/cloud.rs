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
    // Scheme and host only: SigV4 signs the host, and a path typed into the
    // endpoint would be in the URL but not in the canonical request.
    let endpoint = match endpoint.split_once("://") {
        Some((scheme, rest)) => format!("{scheme}://{}", rest.split('/').next().unwrap_or("")),
        None => format!("https://{}", endpoint.split('/').next().unwrap_or("")),
    };
    Some(S3 { endpoint, region, bucket, key, secret, prefix })
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
    // Each segment percent-encoded ONCE, identically in the canonical request
    // and in the URL sent, or a space in a prefix is a SignatureDoesNotMatch.
    let path = format!("/{}/{}", crate::mcp::enc(&s3.bucket), object_key.split('/').map(crate::mcp::enc).collect::<Vec<_>>().join("/"));
    let payload_hash = hex(&Sha256::digest(&body));
    let (headers, _) = sign(s3, "PUT", &path, "", &payload_hash, now_ms);
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

/// Sign one request and hand back the headers it needs.
///
/// PUT, GET and DELETE differ only in the method, the query string and the
/// payload hash, and SigV4 is unforgiving about all three: the canonical query
/// must be sorted and encoded exactly as the URL sends it, and an unsigned
/// header is a `SignatureDoesNotMatch` with no explanation. One function, so
/// the three callers cannot drift apart.
fn sign(
    s3: &S3,
    method: &str,
    path: &str,
    canonical_query: &str,
    payload_hash: &str,
    now_ms: i64,
) -> (Headers, String) {
    let (amz_date, date) = amz_dates(now_ms);
    let host = host_of(&s3.endpoint);
    let canonical_headers =
        format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let canonical_request = format!(
        "{method}\n{path}\n{canonical_query}\n{canonical_headers}\n{SIGNED_HEADERS}\n{payload_hash}"
    );
    let scope = format!("{date}/{}/{SERVICE}/aws4_request", s3.region);
    let string_to_sign = format!(
        "{ALGORITHM}\n{amz_date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical_request.as_bytes()))
    );
    let k_date = hmac(format!("AWS4{}", s3.secret).as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, s3.region.as_bytes());
    let k_service = hmac(&k_region, SERVICE.as_bytes());
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex(&hmac(&k_signing, string_to_sign.as_bytes()));
    let authorization = format!(
        "{ALGORITHM} Credential={}/{scope}, SignedHeaders={SIGNED_HEADERS}, Signature={signature}",
        s3.key
    );
    let headers = Headers::new();
    let _ = headers.set("authorization", &authorization);
    let _ = headers.set("x-amz-date", &amz_date);
    let _ = headers.set("x-amz-content-sha256", payload_hash);
    (headers, amz_date)
}

/// The SHA-256 of an empty body, which SigV4 requires by name for GET and
/// DELETE.
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Every key under `prefix`, one page of up to a thousand.
///
/// The XML is read by scanning for `<Key>` rather than parsed: the response
/// shape is fixed by the S3 API, an XML crate is 100 KB of wasm this Worker
/// would carry forever for one element name, and a malformed body yields no
/// keys, which makes the rotation do nothing rather than something wrong.
async fn list(s3: &S3, prefix: &str, now_ms: i64) -> std::result::Result<Vec<String>, String> {
    let path = format!("/{}", crate::mcp::enc(&s3.bucket));
    let query = format!("list-type=2&max-keys=1000&prefix={}", crate::mcp::enc(prefix));
    let (headers, _) = sign(s3, "GET", &path, &query, EMPTY_SHA256, now_ms);
    let url = format!("{}{path}?{query}", s3.endpoint);
    let req = Request::new_with_init(&url, RequestInit::new().with_method(Method::Get).with_headers(headers))
        .map_err(|e| e.to_string())?;
    let mut res = Fetch::Request(req).send().await.map_err(|e| e.to_string())?;
    if res.status_code() >= 300 {
        let text = res.text().await.unwrap_or_default();
        return Err(format!("{} {}", res.status_code(), text.chars().take(ERROR_SHOWN).collect::<String>()));
    }
    let body = res.text().await.map_err(|e| e.to_string())?;
    Ok(keys_in_listing(&body))
}

/// The `<Key>` elements of an S3 listing, in the order they appear.
pub fn keys_in_listing(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<Key>") {
        rest = &rest[at + 5..];
        let Some(end) = rest.find("</Key>") else { break };
        out.push(rest[..end].to_string());
        rest = &rest[end + 6..];
    }
    out
}

async fn delete(s3: &S3, object_key: &str, now_ms: i64) -> std::result::Result<(), String> {
    let path = format!(
        "/{}/{}",
        crate::mcp::enc(&s3.bucket),
        object_key.split('/').map(crate::mcp::enc).collect::<Vec<_>>().join("/")
    );
    let (headers, _) = sign(s3, "DELETE", &path, "", EMPTY_SHA256, now_ms);
    let url = format!("{}{path}", s3.endpoint);
    let req =
        Request::new_with_init(&url, RequestInit::new().with_method(Method::Delete).with_headers(headers))
            .map_err(|e| e.to_string())?;
    let mut res = Fetch::Request(req).send().await.map_err(|e| e.to_string())?;
    if res.status_code() < 300 {
        return Ok(());
    }
    let text = res.text().await.unwrap_or_default();
    Err(format!("{} {}", res.status_code(), text.chars().take(ERROR_SHOWN).collect::<String>()))
}

/// Days since 1970-01-01 for a civil date — the inverse of `civil`, and
/// Hinnant's algorithm again so the two agree by construction.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// When the object named by this key was written, from the stamp in its name.
/// `None` for a key this module did not write — and such a key is NEVER
/// deleted, because the venue's bucket is the venue's, and a prefix it shares
/// with something else must survive the rotation untouched.
pub fn stamp_of_key(key: &str) -> Option<i64> {
    let name = key.rsplit('/').next()?;
    let stem = name.strip_suffix(".json.gz").or_else(|| name.strip_suffix(".json"))?;
    // YYYYMMDDTHHMMSSZ
    if stem.len() != 16 || stem.as_bytes()[8] != b'T' || !stem.ends_with('Z') {
        return None;
    }
    let num = |a: usize, b: usize| stem.get(a..b)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0, 4)?, num(4, 6)?, num(6, 8)?);
    let (h, mi, s) = (num(9, 11)?, num(11, 13)?, num(13, 15)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    Some((days_from_civil(y, mo as u32, d as u32) * 86_400 + h * 3600 + mi * 60 + s) * 1000)
}

/// How long every copy is kept, and how far back one copy a week is kept.
pub const KEEP_ALL_MS: i64 = 7 * 24 * 60 * 60 * 1000;
pub const KEEP_WEEKLY_MS: i64 = 35 * 24 * 60 * 60 * 1000;

/// Which of these keys the rotation drops: everything but the last seven days
/// and the NEWEST copy in each of the four weeks before that. Older than five
/// weeks goes, and a key whose name this module did not write stays.
///
/// PURE, so the policy is testable without a bucket. The nightly cron is the
/// only caller and it runs once a venue a night, so "newest per week" is
/// computed by bucketing on whole weeks back from `now_ms` rather than by any
/// calendar rule about Mondays.
pub fn keys_to_drop(keys: &[String], now_ms: i64) -> Vec<String> {
    let mut dated: Vec<(i64, &String)> =
        keys.iter().filter_map(|k| stamp_of_key(k).map(|at| (at, k))).collect();
    // Newest first, so the first key seen in a week bucket is the one kept.
    dated.sort_by(|a, b| b.0.cmp(&a.0));
    let mut kept_weeks: Vec<i64> = Vec::new();
    let mut drop = Vec::new();
    for (at, key) in dated {
        let age = now_ms - at;
        if age < KEEP_ALL_MS {
            continue; // every copy of the last seven days
        }
        if age >= KEEP_WEEKLY_MS {
            drop.push(key.clone()); // older than five weeks
            continue;
        }
        let week = age / KEEP_ALL_MS;
        if kept_weeks.contains(&week) {
            drop.push(key.clone());
        } else {
            kept_weeks.push(week);
        }
    }
    drop
}

/// Apply the policy against the bucket. Returns the keys that were removed.
///
/// A FAILURE HERE IS NOT A FAILED BACKUP. The copy is already written when
/// this runs; if the store refuses a listing or a delete, the venue simply
/// keeps more copies than the policy wanted, which is the safe direction.
pub async fn rotate(s3: &S3, venue: &str, now_ms: i64) -> std::result::Result<Vec<String>, String> {
    let prefix =
        if s3.prefix.is_empty() { format!("{venue}/") } else { format!("{}/{venue}/", s3.prefix) };
    let keys = list(s3, &prefix, now_ms).await?;
    let mut removed = Vec::new();
    for key in keys_to_drop(&keys, now_ms) {
        match delete(s3, &key, now_ms).await {
            Ok(()) => removed.push(key),
            // Named, not swallowed: a bucket that refuses deletes will keep
            // growing and the owner should be able to see why.
            Err(e) => return Err(format!("{} kept: {e}", removed.len())),
        }
    }
    Ok(removed)
}

/// gzip, using the runtime's own `CompressionStream`.
///
/// NO NEW CRATE. A deflate implementation in Rust is 40-60 KB of wasm in a
/// Worker built for size, and the runtime already has one behind a web
/// standard. Absent (a test, an older runtime), the bundle goes up
/// uncompressed rather than not at all -- the backup matters more than its
/// size.
async fn gzip(body: &[u8]) -> Option<Vec<u8>> {
    use worker::js_sys::{Array, Function, Reflect, Uint8Array};
    use worker::wasm_bindgen::JsCast;
    let global = worker::js_sys::global();
    let cs_ctor: Function = Reflect::get(&global, &"CompressionStream".into()).ok()?.dyn_into().ok()?;
    let response_ctor: Function = Reflect::get(&global, &"Response".into()).ok()?.dyn_into().ok()?;
    let cs = Reflect::construct(&cs_ctor, &Array::of1(&"gzip".into())).ok()?;

    let bytes = Uint8Array::from(body);
    let src = Reflect::construct(&response_ctor, &Array::of1(&bytes)).ok()?;
    let src_body = Reflect::get(&src, &"body".into()).ok()?;
    let pipe_through: Function =
        Reflect::get(&src_body, &"pipeThrough".into()).ok()?.dyn_into().ok()?;
    let piped = pipe_through.call1(&src_body, &cs).ok()?;
    let out = Reflect::construct(&response_ctor, &Array::of1(&piped)).ok()?;
    let array_buffer: Function =
        Reflect::get(&out, &"arrayBuffer".into()).ok()?.dyn_into().ok()?;
    let promise: worker::js_sys::Promise = array_buffer.call0(&out).ok()?.dyn_into().ok()?;
    let buf = worker::wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;
    Some(Uint8Array::new(&buf).to_vec())
}

/// Where the bundle lands: `<prefix>/<venue>/<YYYYMMDD>T<HHMMSS>Z.json[.gz]`.
/// The extension says what is actually in the object, because a `.json` that
/// is gzip is a file nobody can open by double-clicking it.
fn object_key(s3: &S3, venue: &str, now_ms: i64, gzipped: bool) -> String {
    let (stamp, _) = amz_dates(now_ms);
    let ext = if gzipped { "json.gz" } else { "json" };
    if s3.prefix.is_empty() {
        format!("{venue}/{stamp}.{ext}")
    } else {
        format!("{}/{venue}/{stamp}.{ext}", s3.prefix)
    }
}

/// Export the venue and put it in its bucket. Returns what was written.
pub async fn push_place(place: &crate::hubstore::Place, now_ms: i64) -> std::result::Result<Value, String> {
    let settings = crate::hubstore::load_settings(place).await.map_err(|e| e.to_string())?.settings;
    let Some(s3) = cfg(&settings) else { return Err("no cloud storage is set".into()) };
    let bundle = crate::hubstore::export(place).await.map_err(|e| e.to_string())?;
    let raw = serde_json::to_vec(&bundle).map_err(|e| e.to_string())?;
    let plain = raw.len();
    // Base64 of an image compresses well even now that the images are trimmed:
    // the bundle is text with a small alphabet. If the runtime has no
    // `CompressionStream`, the plain bundle goes up.
    let (body, gzipped) = match gzip(&raw).await {
        Some(z) if z.len() < raw.len() => (z, true),
        _ => (raw, false),
    };
    let key = object_key(&s3, &place.venue, now_ms, gzipped);
    let bytes = body.len();
    let etag = put(&s3, &key, body, if gzipped { "application/gzip" } else { "application/json" }, now_ms)
        .await?;
    let note = format!("{now_ms} {key} {bytes}");
    let _ = crate::hubstore::with_settings(place, move |s| { s.set(LAST_KEY, &note); Ok(()) }).await;
    // ROTATION AFTER THE COPY IS SAFE, never before: the newest object exists
    // before anything old is removed, so an interrupted night leaves too many
    // copies rather than too few. A rotation that fails is reported beside the
    // copy that succeeded.
    let (removed, rotate_error) = match rotate(&s3, &place.venue, now_ms).await {
        Ok(gone) => (gone.len(), None),
        Err(e) => (0, Some(e)),
    };
    Ok(json!({
        "key": key,
        "bucket": s3.bucket,
        "bytes": bytes,
        "plainBytes": plain,
        "gzip": gzipped,
        "etag": etag,
        "atMs": now_ms,
        "rotated": removed,
        "rotateError": rotate_error,
    }))
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

/// GPS fixes older than this are nobody's. The map reads the newest fix per
/// courier within `live_eta::POSITION_FRESH_MS` (twenty minutes) and nothing
/// else reads the table, yet every fix ever sent stayed in it: the largest
/// D1 write source in the system, kept forever, read never. Twenty-four
/// hours is the retention `compliance/data-map.md` promises for courier GPS.
const POSITIONS_KEEP_MS: i64 = 24 * 60 * 60 * 1000;

async fn prune_positions(db: &D1Database, now_ms: i64) {
    let before = now_ms - POSITIONS_KEEP_MS;
    let stmt = db
        .prepare("DELETE FROM courier_positions WHERE recorded_at_ms < ?1")
        .bind(&[worker::wasm_bindgen::JsValue::from_f64(before as f64)]);
    match stmt {
        Ok(s) => match s.run().await {
            Ok(r) => console_log!("nightly prune: positions older than 24 h removed ({:?})", r.meta().ok().flatten().and_then(|m| m.changes)),
            Err(e) => console_error!("nightly prune refused: {e}"),
        },
        Err(e) => console_error!("nightly prune: bad statement: {e}"),
    }
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
    prune_positions(&db, now).await;
    crate::errlog::prune(&db, now).await;
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
            Err(e) => {
                crate::loud!(&place.db, Some(&r.id), "cloud.nightly", "backup refused: {e}")
            }
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

    /// `stamp_of_key` is the inverse of the name `object_key` writes, and the
    /// two are checked against each other rather than against a constant.
    #[test]
    fn a_key_written_here_is_read_back_to_the_same_moment() {
        let s3 = S3 {
            endpoint: "https://x".into(),
            region: "auto".into(),
            bucket: "b".into(),
            key: "k".into(),
            secret: "s".into(),
            prefix: "backups".into(),
        };
        for at in [0i64, 1_789_808_707_000, 1_600_000_000_000, 2_000_000_000_000] {
            // The stamp has second resolution, so the round trip lands on the
            // second, not on the millisecond it was given.
            let whole_second = at - at.rem_euclid(1000);
            for gz in [false, true] {
                let key = object_key(&s3, "sushi-durres", at, gz);
                assert_eq!(stamp_of_key(&key), Some(whole_second), "{key}");
            }
        }
    }

    /// A KEY THIS MODULE DID NOT WRITE IS NEVER TOUCHED. The bucket belongs to
    /// the venue and the prefix may hold their own files; a rotation that
    /// deleted an unrecognised object would be deleting someone's data.
    #[test]
    fn a_foreign_key_has_no_stamp_and_is_never_dropped() {
        for key in [
            "sushi/notes.txt",
            "sushi/20260919.json",             // no time part
            "sushi/20260919T0905Z.json",       // too short
            "sushi/20261319T090507Z.json",     // month 13
            "sushi/20260919T090507Z.json.enc", // not ours
            "sushi/photos/20260919T090507Z.png",
        ] {
            assert_eq!(stamp_of_key(key), None, "{key}");
        }
        let foreign: Vec<String> = ["sushi/notes.txt".to_string(), "sushi/menu.csv".to_string()].into();
        assert!(keys_to_drop(&foreign, 1_789_808_707_000).is_empty());
    }

    /// The policy: every copy of the last seven days, then the newest copy of
    /// each of the four weeks before that, then nothing.
    #[test]
    fn rotation_keeps_seven_days_then_one_a_week_for_four_weeks() {
        const DAY: i64 = 24 * 60 * 60 * 1000;
        let now = 1_800_000_000_000;
        let s3 = S3 {
            endpoint: "https://x".into(),
            region: "auto".into(),
            bucket: "b".into(),
            key: "k".into(),
            secret: "s".into(),
            prefix: String::new(),
        };
        // A year of nightly copies, newest at `now`.
        let keys: Vec<String> =
            (0..365).map(|d| object_key(&s3, "v", now - d * DAY, true)).collect();
        let dropped = keys_to_drop(&keys, now);
        let kept: Vec<&String> = keys.iter().filter(|k| !dropped.contains(k)).collect();

        // Seven daily (days 0..6) + one for each of weeks 1, 2, 3 and 4.
        assert_eq!(kept.len(), 11, "kept {:?}", kept);
        for d in 0..7 {
            assert!(kept.contains(&&object_key(&s3, "v", now - d * DAY, true)), "day {d} must stay");
        }
        // The newest copy of the week is the one kept: day 7, not day 13.
        assert!(kept.contains(&&object_key(&s3, "v", now - 7 * DAY, true)));
        assert!(dropped.contains(&object_key(&s3, "v", now - 13 * DAY, true)));
        // Nothing older than five weeks survives.
        assert!(dropped.contains(&object_key(&s3, "v", now - 36 * DAY, true)));
        assert!(dropped.contains(&object_key(&s3, "v", now - 364 * DAY, true)));
    }

    /// A bucket with one copy in it loses nothing -- the case that runs on the
    /// second night of a venue's life.
    #[test]
    fn a_young_venue_loses_nothing() {
        let s3 = S3 {
            endpoint: "https://x".into(),
            region: "auto".into(),
            bucket: "b".into(),
            key: "k".into(),
            secret: "s".into(),
            prefix: String::new(),
        };
        let now = 1_800_000_000_000;
        let keys = vec![object_key(&s3, "v", now, true)];
        assert!(keys_to_drop(&keys, now).is_empty());
    }

    /// The listing is read by scanning for `<Key>`, so the scan is what is
    /// tested: a real S3 body, and a malformed one that must yield nothing
    /// rather than half a key.
    #[test]
    fn the_listing_scan_reads_every_key_and_refuses_a_broken_body() {
        let xml = "<?xml version=\"1.0\"?><ListBucketResult><Contents><Key>v/20260919T090507Z.json.gz</Key>\
                   <Size>10</Size></Contents><Contents><Key>v/20260920T090507Z.json.gz</Key></Contents></ListBucketResult>";
        assert_eq!(
            keys_in_listing(xml),
            vec!["v/20260919T090507Z.json.gz", "v/20260920T090507Z.json.gz"]
        );
        assert!(keys_in_listing("<Contents><Key>unterminated").is_empty());
        assert!(keys_in_listing("not xml at all").is_empty());
    }

}
