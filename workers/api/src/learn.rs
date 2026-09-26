//! Lesson videos, behind sign-in (Wave L, L7 gated; operator 2026-09-24).
//!
//!   GET /api/learn/manifest        the published cuts this principal may watch
//!   GET /api/learn/media/*key      one file of a cut: `<lesson>/<lang>/<file>`, Range-aware
//!
//! The media lives in the R2 bucket `dowiz-learn` (binding `LEARN`) under `learn/`, written by
//! `tools/learn/publish.sh`; nothing of it is a static asset, because a static asset cannot be
//! gated without `run_worker_first`. The wiki shell (`public/wiki/`) is static and empty; it
//! sends the app's own Bearer token.
//!
//! WHO MAY WATCH WHAT. The lessons are the product's, not a venue's, so no venue data is read:
//! the question is only whether the caller is a live principal of SOME venue and which track
//! is theirs. An owner sees every track; staff the waiter track; a courier the courier track;
//! everyone signed in the guest track. A customer token (minted per order) and a platform
//! token (it names no venue) are refused. A lesson outside the caller's track is a 404, the
//! same answer as a file that does not exist, so the listing cannot be probed.
//!
//! RANGE. A phone's `<video>` asks for `bytes=0-` and then seeks with explicit ranges; Safari
//! refuses to play at all without 206. One range per request (a multi-range is answered whole,
//! which RFC 9110 allows); an unsatisfiable one is 416 with `Content-Range: bytes */size`.
use crate::auth::{self, Principal};
use worker::*;

/// Every key the Worker serves is under this prefix in the bucket.
pub const PREFIX: &str = "learn/";
pub const MANIFEST: &str = "learn/manifest.json";
pub const BINDING: &str = "LEARN";
const LANGS: [&str; 3] = ["sq", "en", "uk"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    Owner,
    Staff,
    Courier,
}

/// Which track this principal watches, or the refusal.
pub fn audience(p: &Principal) -> std::result::Result<Audience, (u16, &'static str)> {
    match p {
        Principal::Owner { active_location_id: Some(_), .. } => Ok(Audience::Owner),
        Principal::Owner { active_location_id: None, .. } => Err((403, "a platform token opens no venue's lessons")),
        Principal::Staff { .. } => Ok(Audience::Staff),
        Principal::Courier { .. } => Ok(Audience::Courier),
        Principal::Customer { .. } => Err((403, "lessons are for the venue's people")),
    }
}

/// May this audience watch this lesson? By the id's track letter (O, W, C, G).
pub fn may_see(a: Audience, lesson: &str) -> bool {
    match lesson.as_bytes().first() {
        Some(b'O') => a == Audience::Owner,
        Some(b'W') => matches!(a, Audience::Owner | Audience::Staff),
        Some(b'C') => matches!(a, Audience::Owner | Audience::Courier),
        Some(b'G') => true,
        _ => false,
    }
}

fn lesson_id_ok(id: &str) -> bool {
    let b = id.as_bytes();
    let digits = b.iter().skip(1).take_while(|c| c.is_ascii_digit()).count();
    let rest = &b[(1 + digits).min(b.len())..];
    b.first().is_some_and(|c| c.is_ascii_uppercase())
        && digits > 0
        && (rest.is_empty() || (rest.len() == 1 && rest[0].is_ascii_lowercase()))
}

/// `W3/sq/video.mp4` -> (lesson, R2 key). Anything else -- another shape, a dot-dot, an
/// unknown language or file type -- is None, and the route answers 404 without a bucket read.
pub fn media_key(raw: &str) -> Option<(String, String)> {
    let raw = raw.strip_prefix('/').unwrap_or(raw);
    let mut it = raw.split('/');
    let (id, lang, file) = (it.next()?, it.next()?, it.next()?);
    if it.next().is_some() || !lesson_id_ok(id) || !LANGS.contains(&lang) {
        return None;
    }
    let (stem, ext) = file.rsplit_once('.')?;
    let stem_ok = !stem.is_empty()
        && stem.len() <= 40
        && stem.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_');
    if !stem_ok || content_type(ext).is_none() {
        return None;
    }
    Some((id.to_string(), format!("{PREFIX}{raw}")))
}

/// The types a cut is made of; nothing else is served.
pub fn content_type(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "mp4" => "video/mp4",
        "vtt" => "text/vtt; charset=utf-8",
        "jpg" => "image/jpeg",
        "webp" => "image/webp",
        "json" => "application/json",
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteRange {
    From(u64),
    Span(u64, u64),
    Suffix(u64),
}

/// `Range: bytes=a-b | a- | -n`. Absent, malformed or multi-range -> None (serve it whole).
pub fn parse_range(h: Option<&str>) -> Option<ByteRange> {
    let spec = h?.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let num = |s: &str| s.trim().parse::<u64>().ok();
    match (a.trim().is_empty(), b.trim().is_empty()) {
        (true, false) => num(b).filter(|n| *n > 0).map(ByteRange::Suffix),
        (false, true) => num(a).map(ByteRange::From),
        (false, false) => match (num(a), num(b)) {
            (Some(x), Some(y)) if x <= y => Some(ByteRange::Span(x, y)),
            _ => None,
        },
        (true, true) => None,
    }
}

/// The inclusive (first, last) byte the range selects in an object of `size`, or None
/// when it cannot be satisfied (416).
pub fn resolve(r: ByteRange, size: u64) -> Option<(u64, u64)> {
    if size == 0 {
        return None;
    }
    match r {
        ByteRange::From(a) if a < size => Some((a, size - 1)),
        ByteRange::Span(a, b) if a < size => Some((a, b.min(size - 1))),
        ByteRange::Suffix(n) => Some((size.saturating_sub(n), size - 1)),
        _ => None,
    }
}

/// The manifest narrowed to the caller's track: `lessons` keeps only what `may_see` allows.
pub fn filter_manifest(mut m: serde_json::Value, a: Audience) -> serde_json::Value {
    if let Some(l) = m.get_mut("lessons").and_then(|v| v.as_object_mut()) {
        l.retain(|id, _| may_see(a, id));
    }
    m
}

async fn who(req: &Request, ctx: &RouteContext<crate::Req>) -> std::result::Result<Audience, Response> {
    let p = auth::authenticate(req, &ctx.env, ctx.data.now_ms)
        .await
        .map_err(|e| e.into_response().unwrap_or_else(|_| Response::error("unauthorised", 401).unwrap()))?;
    audience(&p).map_err(|(code, why)| Response::error(why, code).unwrap())
}

fn bucket(ctx: &RouteContext<crate::Req>) -> std::result::Result<Bucket, Response> {
    ctx.env.bucket(BINDING).map_err(|_| Response::error("lesson media is not bound on this deployment", 503).unwrap())
}

/// GET /api/learn/manifest
pub async fn manifest(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let a = match who(&req, &ctx).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let b = match bucket(&ctx) {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let Some(obj) = b.get(MANIFEST).execute().await? else {
        return Response::from_json(&serde_json::json!({ "version": 1, "lessons": {} }));
    };
    let bytes = match obj.body() {
        Some(body) => body.bytes().await?,
        None => Vec::new(),
    };
    let m: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| Error::RustError(format!("manifest: {e}")))?;
    let mut r = Response::from_json(&filter_manifest(m, a))?;
    r.headers_mut().set("cache-control", "private, max-age=60")?;
    Ok(r)
}

/// GET /api/learn/media/*key
pub async fn media(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let a = match who(&req, &ctx).await {
        Ok(a) => a,
        Err(r) => return Ok(r),
    };
    let raw = ctx.param("key").cloned().unwrap_or_default();
    let Some((lesson, key)) = media_key(&raw).filter(|(id, _)| may_see(a, id)) else {
        return Response::error("not found", 404);
    };
    let _ = lesson;
    let b = match bucket(&ctx) {
        Ok(b) => b,
        Err(r) => return Ok(r),
    };
    let Some(head) = b.head(&key).await? else {
        return Response::error("not found", 404);
    };
    let size = head.size();
    let ext = key.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    let headers = Headers::new();
    headers.set("content-type", content_type(ext).unwrap_or("application/octet-stream"))?;
    headers.set("accept-ranges", "bytes")?;
    headers.set("cache-control", "private, max-age=3600")?;
    headers.set("etag", &head.http_etag())?;
    let range = parse_range(req.headers().get("range")?.as_deref());
    let (status, get) = match range {
        None => (200, b.get(&key)),
        Some(r) => match resolve(r, size) {
            None => {
                headers.set("content-range", &format!("bytes */{size}"))?;
                return Ok(Response::empty()?.with_status(416).with_headers(headers));
            }
            Some((first, last)) => {
                headers.set("content-range", &format!("bytes {first}-{last}/{size}"))?;
                (206, b.get(&key).range(Range::OffsetWithLength { offset: first, length: last - first + 1 }))
            }
        },
    };
    let Some(obj) = get.execute().await? else {
        return Response::error("not found", 404);
    };
    let Some(body) = obj.body() else {
        return Response::error("not found", 404);
    };
    let stream = body.stream()?;
    Ok(Response::from_stream(stream)?.with_status(status).with_headers(headers))
}

#[cfg(test)]
mod tests;
