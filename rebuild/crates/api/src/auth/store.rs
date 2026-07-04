//! Ephemeral-KV + external-provider seams for the OAuth/Telegram owner flows. The Node code uses
//! Redis for the OAuth state/nonce/PKCE cache and the one-time handoff code (`auth.ts:46,161`); the
//! rebuild inventory (AUTH-02 note, decision A19) says "Redis state cache → Pg/in-proc per A19".
//! These traits are that seam: the flow LOGIC (single-use state, nonce check, fragment handoff,
//! one-shot GET+DEL exchange) lives in the route handlers and is unit-testable against the
//! in-memory / fake impls here; prod wires Redis (or Pg) + a real Google HTTP client behind the
//! same traits. Keeping them behind a trait is why the callback/exchange handlers are "BUILT"
//! (present, contract-correct, tested) without a live Redis or outbound network in this sandbox.

use std::collections::HashMap;
use std::sync::Mutex;

/// A short-TTL key/value store for OAuth state (`{codeVerifier, nonce}`, 600s) and the one-time
/// handoff code (`{access_token, refresh_token}`, 60s). `getdel` is the one-shot GET+DEL the
/// exchange relies on so a code can never be replayed.
#[async_trait::async_trait]
pub trait EphemeralStore: Send + Sync {
    /// `setex(key, ttl_secs, value)` — store a JSON value with a TTL.
    async fn setex(&self, key: &str, ttl_secs: u64, value: serde_json::Value);
    /// `GET` — read without consuming (OAuth state is validated then explicitly deleted).
    async fn get(&self, key: &str) -> Option<serde_json::Value>;
    /// `GET + DEL` atomically — the one-shot exchange (a code is consumed on first read).
    async fn getdel(&self, key: &str) -> Option<serde_json::Value>;
    /// `DEL` — explicit single-use delete (OAuth state after validation).
    async fn del(&self, key: &str);
}

/// The identity a Google OAuth code-exchange yields. Note (Q-GOOGLE-IDTOK): the id_token signature
/// is NOT verified — it's trusted because it was fetched directly from Google over TLS
/// (`auth.ts:105-106`); the council flagged JWKS-verify as a fast-follow, carried as-is here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleIdentity {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub nonce: Option<String>,
}

/// The Google token-exchange seam: trade an authorization code + PKCE verifier for the decoded
/// id_token identity. Prod = a real HTTPS POST to `oauth2.googleapis.com/token`; tests inject a fake.
#[async_trait::async_trait]
pub trait GoogleOAuthClient: Send + Sync {
    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<GoogleIdentity, GoogleOAuthError>;
}

#[derive(Debug, thiserror::Error)]
pub enum GoogleOAuthError {
    #[error("token exchange failed")]
    ExchangeFailed,
    #[error("missing required profile info")]
    MissingProfile,
}

/// In-memory `EphemeralStore` — single-instance only (no TTL expiry thread; entries just live).
/// For the dark port + tests. Prod swaps in a Redis/Pg-backed impl behind the same trait.
#[derive(Default)]
pub struct InMemoryStore {
    map: Mutex<HashMap<String, serde_json::Value>>,
}

impl InMemoryStore {
    /// Lock the map, recovering the guard from a poisoned mutex (a prior thread panicked while
    /// holding it) rather than `.unwrap()`-panicking — an ephemeral cache should degrade, not
    /// crash the process, on poison. Avoids `clippy::unwrap_used` (workspace `deny`) on the
    /// prod-reachable (non-test) `InMemoryStore` path.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, serde_json::Value>> {
        self.map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[async_trait::async_trait]
impl EphemeralStore for InMemoryStore {
    async fn setex(&self, key: &str, _ttl_secs: u64, value: serde_json::Value) {
        self.lock().insert(key.to_string(), value);
    }
    async fn get(&self, key: &str) -> Option<serde_json::Value> {
        self.lock().get(key).cloned()
    }
    async fn getdel(&self, key: &str) -> Option<serde_json::Value> {
        self.lock().remove(key)
    }
    async fn del(&self, key: &str) {
        self.lock().remove(key);
    }
}

/// A Google client that always fails — the default when OAuth is not wired (flag off / dark port).
/// The `google_oauth_start` route 404s when the flag is off, so this is never reached in practice;
/// it exists so `AuthState` has a non-null default and the callback compiles.
pub struct NullGoogleClient;

#[async_trait::async_trait]
impl GoogleOAuthClient for NullGoogleClient {
    async fn exchange_code(
        &self,
        _code: &str,
        _code_verifier: &str,
        _redirect_uri: &str,
    ) -> Result<GoogleIdentity, GoogleOAuthError> {
        Err(GoogleOAuthError::ExchangeFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn getdel_is_one_shot() {
        let store = InMemoryStore::default();
        store.setex("k", 60, serde_json::json!({"v": 1})).await;
        assert!(store.getdel("k").await.is_some());
        assert!(
            store.getdel("k").await.is_none(),
            "a code can be consumed only once"
        );
    }

    #[tokio::test]
    async fn get_then_del_supports_state_validation() {
        let store = InMemoryStore::default();
        store
            .setex("state:abc", 600, serde_json::json!({"nonce": "n"}))
            .await;
        assert!(store.get("state:abc").await.is_some());
        store.del("state:abc").await;
        assert!(store.get("state:abc").await.is_none());
    }
}
