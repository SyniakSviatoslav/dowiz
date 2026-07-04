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
}
