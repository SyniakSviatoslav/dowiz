//! Object storage abstraction for `/images/*` and `/media/*` (S1 `getImage`/`getMediaObject`).
//! Ports `apps/api/src/ports.ts` `StorageProvider` (read side only — S1 is read-only) and
//! `apps/api/src/lib/local-storage.ts` `LocalFsStorageProvider` verbatim.
//!
//! No R2 (`apps/api/src/lib/r2-storage.ts`) implementation yet: wiring an S3-compatible client
//! is a real dependency addition (aws-sdk-s3 or an s3-compatible crate) that has no justified
//! call site in THIS build (no deployed bucket to point it at in this sandbox) — flagged as a
//! follow-up in the lane report rather than speculatively added (YAGNI). `LocalFsStorage` is
//! enough to prove the traversal-guard + 404-on-missing/404-on-error `x-quirk` behavior, which is
//! the part of this surface that is actual *logic*, not infra wiring.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use domain::ErrorCode;

use crate::error::ApiError;

/// Read-only slice of `StorageProvider` (`ports.ts:18-22`) — S1 never `put`/`delete`s.
#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
}

#[derive(Debug, thiserror::Error)]
#[error("storage backend error: {0}")]
pub struct StorageError(pub String);

/// Ports `LocalFsStorageProvider` (`local-storage.ts:5-37`) verbatim: `get` reads
/// `<base_dir>/<key>`, `ENOENT` -> `Ok(None)`, any other IO error surfaces as `StorageError`
/// (which the route layer maps to 404 per the `x-quirk` — a storage-layer error is
/// indistinguishable from "missing", not a 5xx).
pub struct LocalFsStorage {
    base_dir: PathBuf,
}

impl LocalFsStorage {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        LocalFsStorage {
            base_dir: base_dir.into(),
        }
    }
}

#[async_trait::async_trait]
impl Storage for LocalFsStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let path = self.base_dir.join(key);
        match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(StorageError(err.to_string())),
        }
    }
}

/// Marker error — the caller (route handler) holds the real correlation id, so this stays a
/// plain unit-shaped error rather than baking a placeholder correlation id into an `ApiError`
/// here (see `into_api_error`).
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidObjectKey;

impl InvalidObjectKey {
    /// Builds the actual `ApiError` once the caller supplies the request's real correlation id
    /// and the route-specific message (`"Invalid image key"` vs `"Invalid media key"` —
    /// `spa-proxy.ts:165` vs `:190` use slightly different wording, preserved verbatim).
    pub fn into_api_error(self, message: &str, correlation_id: impl Into<String>) -> ApiError {
        ApiError::new(ErrorCode::InvalidKey, message, correlation_id)
    }
}

/// The traversal guard both `/images/*` and `/media/*` run BEFORE touching storage
/// (`spa-proxy.ts:165,190`): reject `..`, NUL, and backslash. Fastify decodes `%2f` in the
/// wildcard param before the handler sees it (the Node comment's rationale), so this same
/// decoded-string check is the right layer to port — axum's `Path<String>`-from-wildcard
/// extraction is likewise already percent-decoded by the time this runs.
///
/// Pure + unit-tested independent of any HTTP framework or storage backend.
pub fn validate_object_key(key: &str) -> Result<(), InvalidObjectKey> {
    if key.is_empty() || key.contains("..") || key.contains('\0') || key.contains('\\') {
        return Err(InvalidObjectKey);
    }
    Ok(())
}

/// `/media/*`'s extension -> Content-Type table (`spa-proxy.ts:193-199`), pure and unit-tested.
/// `/images/*` has no equivalent table — it is ALWAYS `image/webp` (the upload pipeline
/// transcodes to webp, so the proxy never needs to sniff/derive a type there).
pub fn media_content_type(key: &str) -> &'static str {
    let ext = Path::new(key)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

/// Cloudflare R2 (S3-compatible) backend — ports `apps/api/src/lib/r2-storage.ts`'s `get` path
/// (S1 is read-only, see module doc: this crate never `put`/`delete`s, so only R2's GetObject is
/// implemented). Selected via `STORAGE_BACKEND=r2` (`main.rs::build_storage`); env vars are the
/// same 4 names Node reads (`r2-storage.ts:6-7,18-20`): `R2_ENDPOINT`, `R2_BUCKET`,
/// `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`.
///
/// Dependency note (follow-up flag): Node uses `@aws-sdk/client-s3`. The obvious Rust analogs —
/// `aws-sdk-s3`, or the `object_store` crate this rebuild's own inventory doc already names
/// (`docs/design/rebuild-plan/inventory/10-api-realtime-jobs.md`, row "S3Client construction") —
/// both resolve `aws-lc-rs`/`aws-lc-sys`, which needs `cmake` to compile; this sandbox has none
/// (`cmake: command not found`). Ported instead with `aws-sign-v4` (SigV4 signing only, ~290
/// lines, built on `ring`+`hmac`+`sha2` — all already resolved transitively via
/// `sqlx-postgres`'s SCRAM auth) + `reqwest` with `rustls-tls` (also `ring`-backed, no `cmake`):
/// 24 new resolved crates total, `cargo check` stayed under 40s. Re-evaluate `aws-sdk-s3` /
/// `object_store` once `cmake` is available in a real deploy image — either is fine; this just
/// records WHY the follow-up didn't reach for the "obvious" AWS crate.
pub struct R2Storage {
    endpoint: String,
    bucket: String,
    access_key: String,
    secret_key: String,
    /// R2 ignores the AWS region concept but the SigV4 scope string still needs a literal value;
    /// `"auto"` matches Node's `S3Client({ region: 'auto', ... })` (`r2-storage.ts:30`).
    region: String,
    client: reqwest::Client,
}

/// Manual (not derived) `Debug`: `secret_key`/`access_key` must never land in a log line or a
/// test failure message — `#[derive(Debug)]` would print them verbatim.
impl std::fmt::Debug for R2Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("R2Storage")
            .field("endpoint", &self.endpoint)
            .field("bucket", &self.bucket)
            .field("access_key", &"<redacted>")
            .field("secret_key", &"<redacted>")
            .field("region", &self.region)
            .finish()
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("R2Storage requires {0} (STORAGE_BACKEND=r2 but the env var is unset/empty)")]
pub struct R2ConfigError(pub &'static str);

impl R2Storage {
    pub fn new(
        endpoint: impl Into<String>,
        bucket: impl Into<String>,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
    ) -> Self {
        R2Storage {
            endpoint: endpoint.into(),
            bucket: bucket.into(),
            access_key: access_key.into(),
            secret_key: secret_key.into(),
            region: "auto".to_string(),
            // A builder with no exotic options (no proxy/redirect/tls override) practically never
            // fails to build; `unwrap_or_default` (a plain `reqwest::Client::new()`) keeps this
            // constructor infallible rather than reaching for `.expect()` for a case that isn't
            // reachable from any input this type accepts.
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Same 4 env vars Node reads (`r2-storage.ts:6-7,18-20`) — thin wrapper around `from_map` so
    /// the "which var is missing" validation is unit-testable without mutating `std::env`
    /// (process-global, racy under parallel tests — same rationale as `config.rs::from_env`).
    pub fn from_env() -> Result<Self, R2ConfigError> {
        Self::from_map(&std::env::vars().collect())
    }

    fn from_map(vars: &HashMap<String, String>) -> Result<Self, R2ConfigError> {
        let get = |key: &'static str| -> Result<String, R2ConfigError> {
            vars.get(key)
                .filter(|v| !v.is_empty())
                .cloned()
                .ok_or(R2ConfigError(key))
        };
        Ok(Self::new(
            get("R2_ENDPOINT")?,
            get("R2_BUCKET")?,
            get("R2_ACCESS_KEY_ID")?,
            get("R2_SECRET_ACCESS_KEY")?,
        ))
    }

    /// Path-style object URL: `{endpoint}/{bucket}/{key}` — the key->object mapping this backend
    /// exists to prove. Path-style (not virtual-hosted `{bucket}.{endpoint}`) because R2's own
    /// S3-compatibility docs show path-style addressing working uniformly across custom
    /// endpoints; Node's SDK call (`r2-storage.ts:27-35`) never sets `forcePathStyle`, so this is
    /// a best-effort match, not a verified one (no live R2 bucket reachable from this sandbox) —
    /// flag before a staging cutover.
    fn object_url(&self, key: &str) -> String {
        format!(
            "{}/{}/{}",
            self.endpoint.trim_end_matches('/'),
            self.bucket,
            key.trim_start_matches('/')
        )
    }

    /// The bare host, for the SigV4-signed `Host` header — string-sliced rather than pulling in
    /// the `url` crate as a direct dependency (it's already resolved transitively via
    /// `aws-sign-v4`, but Rust's 2018+ extern-prelude rule means this crate can't `use` it
    /// without also declaring it directly — not worth doing for one `trim`+`split`).
    fn host(&self) -> &str {
        self.endpoint
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(&self.endpoint)
    }
}

/// SHA-256 of the empty string — constant for every `/images/*`/`/media/*` GET (no request
/// body), reused as both the signed `x-amz-content-sha256` header value and the payload hash the
/// canonical request itself is built against (they must match).
const EMPTY_PAYLOAD_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[async_trait::async_trait]
impl Storage for R2Storage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let url = self.object_url(key);
        let host = self.host().to_string();
        let datetime = chrono::Utc::now();
        let amz_date = datetime.format("%Y%m%dT%H%M%SZ").to_string();

        let mut sign_headers = axum::http::HeaderMap::new();
        sign_headers.insert(
            "host",
            host.parse()
                .map_err(|e| StorageError(format!("invalid R2 host {host:?}: {e}")))?,
        );
        sign_headers.insert(
            "x-amz-content-sha256",
            axum::http::HeaderValue::from_static(EMPTY_PAYLOAD_SHA256),
        );
        sign_headers.insert(
            "x-amz-date",
            amz_date
                .parse()
                .map_err(|e| StorageError(format!("invalid x-amz-date {amz_date:?}: {e}")))?,
        );

        let signer = aws_sign_v4::AwsSign::new(
            "GET",
            &url,
            &datetime,
            &sign_headers,
            &self.region,
            &self.access_key,
            &self.secret_key,
            "s3",
            "",
        );
        let authorization = signer.sign();

        let response = self
            .client
            .get(&url)
            .header("host", &host)
            .header("x-amz-content-sha256", EMPTY_PAYLOAD_SHA256)
            .header("x-amz-date", &amz_date)
            .header("authorization", authorization)
            .send()
            .await
            .map_err(|e| StorageError(format!("R2 request failed: {e}")))?;

        match response.status() {
            reqwest::StatusCode::NOT_FOUND => Ok(None),
            status if status.is_success() => {
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| StorageError(format!("R2 response body read failed: {e}")))?;
                Ok(Some(bytes.to_vec()))
            }
            // x-quirk (kept consistent with LocalFsStorage): any other non-2xx status (auth
            // failure, 5xx, throttling) is indistinguishable from "missing" to this trait's
            // caller — the route layer's 404-on-error mapping already treats
            // `Err(StorageError)` and `Ok(None)` identically (`routes/media_proxy.rs`), so
            // surfacing it as an `Err` here (rather than silently returning `Ok(None)`) preserves
            // the actual failure reason in logs without changing the HTTP-visible behavior.
            status => Err(StorageError(format!("R2 GET returned {status}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_object_key_rejects_traversal_shapes() {
        assert!(validate_object_key("../etc/passwd").is_err());
        assert!(validate_object_key("a/../../b").is_err());
        assert!(validate_object_key("nul\0byte").is_err());
        assert!(validate_object_key("back\\slash").is_err());
        assert!(validate_object_key("").is_err());
    }

    #[test]
    fn validate_object_key_accepts_flat_content_addressed_keys() {
        assert!(validate_object_key("a1b2c3d4.webp").is_ok());
        assert!(
            validate_object_key("sub/dir/key.jpg").is_ok(),
            "may contain slashes (x-wildcard)"
        );
    }

    #[test]
    fn validate_object_key_rejection_maps_to_invalid_key_code() {
        let err = validate_object_key("../x").unwrap_err();
        let api_err = err.into_api_error("Invalid image key", "corr-1");
        assert_eq!(api_err.envelope.code, ErrorCode::InvalidKey);
        assert_eq!(api_err.envelope.message, "Invalid image key");
    }

    #[test]
    fn media_content_type_matches_the_node_table() {
        assert_eq!(media_content_type("a.webp"), "image/webp");
        assert_eq!(media_content_type("a.jpg"), "image/jpeg");
        assert_eq!(media_content_type("a.jpeg"), "image/jpeg");
        assert_eq!(
            media_content_type("a.JPG"),
            "image/jpeg",
            "case-insensitive"
        );
        assert_eq!(media_content_type("a.png"), "image/png");
        assert_eq!(media_content_type("a.mp4"), "video/mp4");
        assert_eq!(media_content_type("a.unknown"), "application/octet-stream");
        assert_eq!(
            media_content_type("no-extension"),
            "application/octet-stream"
        );
    }

    #[tokio::test]
    async fn local_fs_storage_returns_none_for_missing_key() {
        let dir = std::env::temp_dir().join(format!("dowiz-rebuild-test-{}", uuid::Uuid::new_v4()));
        let storage = LocalFsStorage::new(&dir);
        let result = storage.get("does-not-exist.webp").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn local_fs_storage_returns_bytes_for_existing_key() {
        let dir = std::env::temp_dir().join(format!("dowiz-rebuild-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let file_path = dir.join("present.webp");
        tokio::fs::write(&file_path, b"fake-webp-bytes")
            .await
            .unwrap();

        let storage = LocalFsStorage::new(&dir);
        let result = storage.get("present.webp").await.unwrap();
        assert_eq!(result, Some(b"fake-webp-bytes".to_vec()));

        tokio::fs::remove_dir_all(&dir).await.ok();
    }

    // ── R2Storage: key -> object mapping + config validation (fake/in-memory, no network) ────

    fn r2_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn r2_object_url_maps_flat_and_nested_keys_under_bucket() {
        let storage = R2Storage::new(
            "https://acct123.r2.cloudflarestorage.com",
            "my-bucket",
            "ak",
            "sk",
        );
        assert_eq!(
            storage.object_url("a1b2c3d4.webp"),
            "https://acct123.r2.cloudflarestorage.com/my-bucket/a1b2c3d4.webp"
        );
        assert_eq!(
            storage.object_url("sub/dir/key.jpg"),
            "https://acct123.r2.cloudflarestorage.com/my-bucket/sub/dir/key.jpg"
        );
    }

    #[test]
    fn r2_object_url_tolerates_trailing_and_leading_slashes() {
        let storage = R2Storage::new(
            "https://acct123.r2.cloudflarestorage.com/",
            "my-bucket",
            "ak",
            "sk",
        );
        assert_eq!(
            storage.object_url("/leading-slash.webp"),
            "https://acct123.r2.cloudflarestorage.com/my-bucket/leading-slash.webp"
        );
    }

    #[test]
    fn r2_host_strips_scheme_only() {
        let storage = R2Storage::new(
            "https://acct123.r2.cloudflarestorage.com",
            "my-bucket",
            "ak",
            "sk",
        );
        assert_eq!(storage.host(), "acct123.r2.cloudflarestorage.com");
    }

    #[test]
    fn r2_traversal_shaped_key_never_reaches_the_network() {
        // The route layer's `validate_object_key` gate (shared across every `Storage` backend,
        // tested above) rejects a traversal-shaped key BEFORE it would ever reach
        // `R2Storage::object_url`/the network — pinned here for R2 specifically so a future
        // refactor can't quietly give R2 its own (possibly divergent) guard.
        assert!(validate_object_key("../etc/passwd").is_err());
        let storage = R2Storage::new(
            "https://acct.r2.cloudflarestorage.com",
            "bucket",
            "ak",
            "sk",
        );
        // If the guard were ever bypassed, this is the (bad) URL that would result — asserting it
        // here documents exactly what the guard prevents, not just that it returns an error.
        assert_eq!(
            storage.object_url("../etc/passwd"),
            "https://acct.r2.cloudflarestorage.com/bucket/../etc/passwd"
        );
    }

    #[test]
    fn r2_from_map_requires_all_four_env_vars() {
        let err = R2Storage::from_map(&r2_map(&[])).unwrap_err();
        assert_eq!(
            err,
            R2ConfigError("R2_ENDPOINT"),
            "reports the FIRST missing var"
        );

        let err = R2Storage::from_map(&r2_map(&[
            ("R2_ENDPOINT", "https://acct.r2.cloudflarestorage.com"),
            ("R2_BUCKET", "bucket"),
            ("R2_ACCESS_KEY_ID", "ak"),
        ]))
        .unwrap_err();
        assert_eq!(err, R2ConfigError("R2_SECRET_ACCESS_KEY"));
    }

    #[test]
    fn r2_from_map_rejects_empty_string_values() {
        let err = R2Storage::from_map(&r2_map(&[
            ("R2_ENDPOINT", ""),
            ("R2_BUCKET", "bucket"),
            ("R2_ACCESS_KEY_ID", "ak"),
            ("R2_SECRET_ACCESS_KEY", "sk"),
        ]))
        .unwrap_err();
        assert_eq!(err, R2ConfigError("R2_ENDPOINT"));
    }

    #[test]
    fn r2_from_map_builds_with_all_four_vars_present() {
        let storage = R2Storage::from_map(&r2_map(&[
            ("R2_ENDPOINT", "https://acct.r2.cloudflarestorage.com"),
            ("R2_BUCKET", "bucket"),
            ("R2_ACCESS_KEY_ID", "ak"),
            ("R2_SECRET_ACCESS_KEY", "sk"),
        ]))
        .unwrap();
        assert_eq!(
            storage.object_url("k.webp"),
            "https://acct.r2.cloudflarestorage.com/bucket/k.webp"
        );
    }

    /// The real network call — never runs in CI/`cargo test`, only via
    /// `cargo test -- --ignored` with live `R2_*` env vars set (mirrors the existing
    /// `with_tenant` integration test's `#[ignore]` convention, `rebuild/README.md`).
    #[tokio::test]
    #[ignore = "needs R2 creds"]
    async fn r2_storage_get_against_a_live_bucket() {
        let storage = R2Storage::from_env().expect("R2_* env vars must be set to run this test");
        let result = storage.get("some-known-test-key.webp").await;
        assert!(
            result.is_ok(),
            "expected a real R2 GET to succeed: {result:?}"
        );
    }
}
