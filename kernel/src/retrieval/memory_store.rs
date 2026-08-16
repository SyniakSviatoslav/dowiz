//! `retrieval/memory_store.rs` — std shim over the pure no_std core (M4 / W4-1).
//!
//! The pure half — the [`MemoryStore`] trait, the content-addressed
//! [`InMemoryStore`] (`BTreeMap` behind a spinlock), and the FNV-1a snapshot-root
//! fold ([`fnv1a`]/[`FNV_OFFSET`]) — lives in `dowiz_core::retrieval::memory_store`
//! and is re-exported here so the kernel's `crate::retrieval::memory_store::…`
//! spellings keep resolving unchanged.
//!
//! This shim adds ONLY the std-only [`PgStore`] — the opt-in `pgrust` SQL adapter
//! (`sqlx` + `tokio`) behind a NON-default feature flag. The default path remains
//! the pure in-process [`InMemoryStore`].

pub use dowiz_core::retrieval::memory_store::*;

use alloc::string::String;
use alloc::vec::Vec;

/// Optional Postgres (`pgrust`) adapter — a REAL sqlx-backed living-memory store.
///
/// Compiled ONLY under the `pgrust` feature. The kernel's hard invariant is
/// pure-`std` with NO network deps by DEFAULT; the native [`InMemoryStore`]
/// remains the default. `pgrust` is the opt-in SQL adapter: it pulls `sqlx` +
/// `tokio` and implements the [`MemoryStore`] contract against a Postgres `kv`
/// table. Because the trait boundary is synchronous but sqlx is async,
/// `PgStore` captures a Tokio runtime [`Handle`] at construction and drives
/// each call with `block_on` — so its public API is byte-identical to
/// [`InMemoryStore`]. DDL is NEVER auto-run; call [`PgStore::migrate`] once,
/// explicitly, against a known database.
#[cfg(feature = "pgrust")]
pub struct PgStore {
    pool: sqlx::PgPool,
    rt: tokio::runtime::Handle,
}

#[cfg(feature = "pgrust")]
impl PgStore {
    /// Connect to Postgres at `database_url` and build the connection pool.
    ///
    /// Does NOT create the schema — call [`migrate`](Self::migrate) explicitly.
    /// Requires a Tokio runtime to be active on the constructing thread (it
    /// captures that runtime's [`Handle`] so later sync calls can `block_on`).
    pub async fn new(database_url: &str) -> Result<Self, String> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect(database_url)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self {
            pool,
            rt: tokio::runtime::Handle::current(),
        })
    }

    /// Idempotent schema creation — EXPLICIT, NEVER auto-called.
    ///
    /// Migration is a red-line op; the default/adapter path never runs this.
    /// Callers must invoke it deliberately against a known database.
    pub async fn migrate(&self) -> Result<(), String> {
        sqlx::query("CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BYTEA NOT NULL);")
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(feature = "pgrust")]
impl MemoryStore for PgStore {
    fn put(&self, key: &str, value: &[u8]) -> Result<(), String> {
        self.rt.block_on(async {
            sqlx::query(
                "INSERT INTO kv(key,value) VALUES($1,$2) \
                     ON CONFLICT(key) DO UPDATE SET value=$2",
            )
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())
        })?;
        Ok(())
    }

    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.rt
            .block_on(async {
                sqlx::query_scalar::<_, Vec<u8>>("SELECT value FROM kv WHERE key=$1")
                    .bind(key)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| e.to_string())
            })
            .ok()
            .flatten()
    }

    fn keys(&self) -> Vec<String> {
        self.rt
            .block_on(async {
                sqlx::query_scalar::<_, String>("SELECT key FROM kv ORDER BY key")
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| e.to_string())
            })
            .unwrap_or_default()
    }

    fn snapshot_root(&self) -> String {
        let rows: Vec<(String, Vec<u8>)> = match self.rt.block_on(async {
            sqlx::query_as::<_, (String, Vec<u8>)>("SELECT key,value FROM kv ORDER BY key")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())
        }) {
            Ok(r) => r,
            Err(_) => return "pg_error".to_string(),
        };
        // Fold the SAME FNV-1a over `len || bytes` frames as
        // `InMemoryStore::snapshot_root` so roots are comparable across stores.
        let mut h: u64 = FNV_OFFSET;
        for (k, v) in &rows {
            h = fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = fnv1a(h, k.as_bytes());
            h = fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = fnv1a(h, v);
        }
        format!("{:016x}", h)
    }
}

/// DB-gated integration test for the `pgrust` adapter.
///
/// Compiled only under `--features pgrust` AND `cfg(test)`. It is `#[ignore]`d
/// so an OFFLINE `cargo test` stays GREEN (the test is skipped, never failed).
/// With a live Postgres reachable via `DATABASE_URL`, run it explicitly:
/// `DATABASE_URL=... cargo test -p dowiz-kernel --features pgrust -- --ignored`.
#[cfg(all(test, feature = "pgrust"))]
mod pg_tests {
    use super::*;

    #[test]
    #[ignore = "needs DATABASE_URL"]
    fn pg_roundtrip() {
        // Offline-safe: early-return unless a live Postgres URL is provided.
        let url = match std::env::var("DATABASE_URL") {
            Ok(u) => u,
            Err(_) => return,
        };
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let pg = rt.block_on(PgStore::new(&url)).expect("connect");
        rt.block_on(pg.migrate()).expect("migrate");
        // put / get roundtrip + overwrite semantics.
        pg.put("roundtrip-key", b"hello-pg").expect("put");
        assert_eq!(
            pg.get("roundtrip-key"),
            Some(b"hello-pg".to_vec()),
            "get must roundtrip"
        );
        pg.put("roundtrip-key", b"world-pg").expect("put2");
        assert_eq!(
            pg.get("roundtrip-key"),
            Some(b"world-pg".to_vec()),
            "overwrite must replace, not append"
        );
        // snapshot_root parity vs an equivalent InMemoryStore (merges evidence).
        let mut mem = InMemoryStore::new();
        mem.put("roundtrip-key", b"world-pg").expect("mem put");
        assert_eq!(
            pg.snapshot_root(),
            mem.snapshot_root(),
            "pg snapshot_root must match InMemoryStore for identical content"
        );
    }
}
