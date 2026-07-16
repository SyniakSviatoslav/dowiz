//! Living-memory persistence — content-addressed store (M4 / W4-1).
//!
//! The kernel's HARD INVARIANT is pure-`std`, NO network deps. The DEFAULT
//! living-memory store is therefore native, std-only, and content-addressed:
//! an [`InMemoryStore`] keyed in a `BTreeMap` so iteration order is
//! deterministic and a [`snapshot_root`](MemoryStore::snapshot_root) over all
//! entries yields a reproducible content hash for tamper-evidence / merge
//! ordering.
//!
//! Postgres (`pgrust`) is allowed ONLY as an OPTIONAL adapter behind a
//! NON-default feature flag. We do NOT add the pgrust dependency now — the
//! trait boundary + a feature-gated stub ship so the SQL path is opt-in later
//! (see [`PgStore`]).

use std::collections::BTreeMap;
use std::sync::Mutex;

/// Deterministic content-addressable living-memory contract.
///
/// Implemented natively by [`InMemoryStore`] (std-only, the default) and,
/// behind the `pgrust` feature, by [`PgStore`] (SQL adapter stub — OFF by
/// default). The kernel never reaches over the network to satisfy this trait;
/// the default path is entirely in-process.
pub trait MemoryStore {
    /// Store `value` under `key`. Overwrites any prior value for the key.
    fn put(&self, key: &str, value: &[u8]) -> Result<(), String>;
    /// Fetch a clone of the bytes stored under `key`, if present.
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    /// All keys currently present, in sorted (deterministic) order.
    fn keys(&self) -> Vec<String>;
    /// A deterministic content hash over ALL entries (a "snapshot root").
    ///
    /// Two stores holding the same key→value mapping yield the same root, and
    /// any change to any entry changes the root. Used for tamper-evidence and
    /// content-addressed merge ordering. Stable across runs/platforms.
    fn snapshot_root(&self) -> String;
}

/// Native std-only default living-memory store (content-addressed).
///
/// Backed by a `BTreeMap` behind a `Mutex` so [`keys`](MemoryStore::keys) is
/// always returned in deterministic sorted order and
/// [`snapshot_root`](MemoryStore::snapshot_root) is reproducible. No network,
/// no SQL, no new deps — satisfies the kernel's pure-`std` red line.
pub struct InMemoryStore {
    map: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl InMemoryStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            map: Mutex::new(BTreeMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore for InMemoryStore {
    fn put(&self, key: &str, value: &[u8]) -> Result<(), String> {
        self.map
            .lock()
            .map_err(|e| e.to_string())?
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.map.lock().ok()?.get(key).cloned()
    }

    fn keys(&self) -> Vec<String> {
        self.map
            .lock()
            .ok()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn snapshot_root(&self) -> String {
        let m = match self.map.lock() {
            Ok(m) => m,
            Err(_) => return "lock_poisoned".to_string(),
        };
        // FNV-1a 64-bit over frame-delimited (key, value) pairs. Each entry is
        // wrapped in `len || bytes` frames so keys/values can never bleed into
        // one another and the fold is canonical regardless of insertion order.
        let mut h: u64 = FNV_OFFSET;
        for (k, v) in m.iter() {
            h = fnv1a(h, &(k.len() as u64).to_le_bytes());
            h = fnv1a(h, k.as_bytes());
            h = fnv1a(h, &(v.len() as u64).to_le_bytes());
            h = fnv1a(h, v);
        }
        format!("{:016x}", h)
    }
}

/// FNV-1a 64-bit offset basis.
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime.
const FNV_PRIME: u64 = 0x100000001b3;

/// Single-pass FNV-1a 64-bit fold. std-only, no deps.
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

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
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BYTEA NOT NULL);",
        )
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(feature = "pgrust")]
impl MemoryStore for PgStore {
    fn put(&self, key: &str, value: &[u8]) -> Result<(), String> {
        self.rt
            .block_on(async {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_put_get_roundtrip() {
        let s = InMemoryStore::new();
        assert!(s.get("a").is_none(), "absent key must return None");
        s.put("a", b"hello").unwrap();
        assert_eq!(s.get("a").unwrap(), b"hello".to_vec());
        // Overwrite must replace, not append.
        s.put("a", b"world").unwrap();
        assert_eq!(s.get("a").unwrap(), b"world".to_vec());
    }

    #[test]
    fn memory_store_snapshot_root_changes_on_put() {
        let s = InMemoryStore::new();
        let empty = s.snapshot_root();
        s.put("k", b"v").unwrap();
        let after_one = s.snapshot_root();
        assert_ne!(
            empty, after_one,
            "snapshot root must change after the first put"
        );
        s.put("k2", b"v2").unwrap();
        assert_ne!(
            after_one,
            s.snapshot_root(),
            "snapshot root must change after a second put"
        );
    }

    #[test]
    fn memory_store_deterministic() {
        let mut a = InMemoryStore::new();
        let mut b = InMemoryStore::new();
        // Different insertion order must NOT affect the content root.
        a.put("x", b"1").unwrap();
        a.put("y", b"2").unwrap();
        b.put("y", b"2").unwrap();
        b.put("x", b"1").unwrap();
        assert_eq!(
            a.snapshot_root(),
            b.snapshot_root(),
            "same content ⇒ same root regardless of insertion order"
        );
        // A differing value must yield a differing root.
        let mut c = InMemoryStore::new();
        c.put("x", b"1").unwrap();
        c.put("y", b"99").unwrap();
        assert_ne!(a.snapshot_root(), c.snapshot_root());
    }

    #[test]
    fn memory_store_keys_sorted() {
        let s = InMemoryStore::new();
        s.put("c", b"3").unwrap();
        s.put("a", b"1").unwrap();
        s.put("b", b"2").unwrap();
        assert_eq!(
            s.keys(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            "keys must be returned in deterministic sorted order"
        );
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
