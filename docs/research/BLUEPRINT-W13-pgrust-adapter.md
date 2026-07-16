# BLUEPRINT W13 — real pgrust SQL adapter (dowiz-kernel)

Status: BUILDABLE offline (code + compile + default-green). Live-DB test gated.
Decart: sqlx 0.8.6 (cached) — see NEXT-PHASES-research-decisions.md.

## Interface (reuse existing `MemoryStore` trait)
File: `dowiz/kernel/src/retrieval/memory_store.rs` — replace the `PgStore`
stub (currently `Err("pgrust adapter not built")`) with a real impl under
`#[cfg(feature = "pgrust")]`.

- `PgStore { pool: sqlx::PgPool }`
- `MemoryStore::put` → `sqlx::query("INSERT INTO kv(key,value) VALUES($1,$2)
  ON CONFLICT(key) DO UPDATE SET value=$2").bind(key).bind(value).execute(pool)`
- `get` → `SELECT value FROM kv WHERE key=$1` → `Option<Vec<u8>>`
- `keys` → `SELECT key FROM kv ORDER BY key` → `Vec<String>` (sorted = deterministic)
- `snapshot_root` → open a tx, `SELECT key,value FROM kv ORDER BY key`, fold the
  SAME FNV-1a over `len||bytes` frames as `InMemoryStore::snapshot_root` so the
  two stores produce COMPARABLE roots (used for merge-evidence).

## SQL schema (idempotent, run once on `PgStore::new`)
`CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BYTEA NOT NULL);`
Migration is a RED-LINE (money/auth/RLS/migrations = per-change-confirm) → the
adapter MUST NOT auto-run DDL against a production DB. Provide `PgStore::migrate(&self)`
as an EXPLICIT, separately-called fn (never called by default path). Default
build never touches it.

## Cargo.toml (kernel)
```
[features]
pgrust = ["dep:sqlx"]          # replaces the empty `pgrust = []`
[dependencies]
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls"], optional = true }
```

## Tests (RED→GREEN, offline-safe)
- `PgStore` struct + `new`/`migrate` compile under `--features pgrust`
  (`cargo build -p dowiz-kernel --features pgrust` exit 0).
- `#[cfg(feature="pgrust")] mod pg_tests {` with `#[test] #[ignore = "needs DATABASE_URL"]`
  `fn pg_roundtrip()` that early-returns unless `std::env::var("DATABASE_URL").is_ok()`.
  Guarantees: offline `cargo test` stays GREEN (test ignored, not failed); with a
  live DB + env set, `cargo test --features pgrust -- --ignored` exercises the real
  SQL roundtrip + snapshot_root parity vs `InMemoryStore`.
- DEFAULT `cargo test -p dowiz-kernel` UNCHANGED (325/0) — pgrust still OFF.

## Verify (parent)
1. `cargo test -p dowiz-kernel` → 325/0 (default, pgrust off).
2. `cargo build -p dowiz-kernel --features pgrust` → exit 0 (sqlx cached, offline).
3. `cargo test -p dowiz-kernel --features pgrust` → compiles, pg_roundtrip IGNORED.

## Honest ceiling
Real SQL execution requires a live Postgres (not available offline). Shipped:
compiling adapter + DB-gated test, default kernel untouched. NOT fake-greened.
