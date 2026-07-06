---
TRIGGER: rebuild/crates/api/**/*.rs
CAUSE: >
  A hand-written psql *literal* probe and the sqlx *bind/decode* path have DIFFERENT Postgres
  type rules, so a psql probe is a FALSE witness. Postgres implicitly coerces an unknown-type SQL
  literal (`'pickup'`, `42`) to the target column type, and psql renders every value as text — so
  `SELECT o.type` and `INSERT ... VALUES ('pickup')` both "work" in psql. But sqlx sends a bound
  parameter with an explicit wire OID and decodes into a concrete Rust type with NO implicit
  coercion: an enum column rejects bound text ("expression is of type text"), a String rejects a
  decoded enum, `numeric` rejects f64, `int4` rejects i64. 10+ live 500s/503s across S1/S4/S5/S7/
  S9/S10 during the 2026-07-05 staging cutover were ALL this one root — structurally invisible
  because the rebuild's DB tests are all `#[ignore]` and nothing ran them (unit/clippy/fmt green).
ACTION: >
  When editing an sqlx query in the Rust rebuild → cause: the bound/decoded path has no implicit
  coercion, so a psql literal probe that "passes" proves nothing → do: on every enum column add
  `::<enumtype>` on a bound WRITE and `::text` on a String READ; on every `numeric` read into f64
  add `::float8`; on every `int4` read into i64 add `::bigint`; and satisfy NOT-NULL columns (e.g.
  `request_hash`) the literal-probe silently accepted. NEVER "confirm" a fix with a psql literal —
  reproduce with a BOUND param (`PREPARE t(text) AS ...; EXECUTE t('pickup')`) to surface the real
  type error, or run the ignored live-PG suite against a real Postgres
  (`cargo test --features dev-routes -- --include-ignored` in `rebuild/`).
LINK: docs/regressions/REGRESSION-LEDGER.md #77 ; docs/design/ci-rust-live-pg/ ;
  rebuild/crates/api/src/routes/orders/pg.rs ;
  docs/reflections/ARCHIVE/2026-07-05-cutover-sqlx-bind-decode-class.reflection.md
SCOPE: rebuild/crates/api/**/*.rs sqlx queries ONLY — the DB-touching Rust crate. Does NOT apply
  to `rebuild/crates/domain/**` (pure logic, no sqlx) nor to Node/TS code. The DURABLE fix is the
  `rust-live-pg` CI job (ledger #77, drafted at docs/design/ci-rust-live-pg/, operator-gated
  because `.github/**` is protect-path); until that job is wired, this lesson + the bound-param
  repro rule are the advisory backstop.
STATUS: active
---

# sqlx bind/decode has no implicit coercion — a psql literal probe is a false witness

Source: reflection `docs/reflections/ARCHIVE/2026-07-05-cutover-sqlx-bind-decode-class.reflection.md`
(cutover class #77); recurred 10+ times serially across the S1–S10 staging flip.

Postgres coerces a bare SQL literal to the column type and psql renders everything as text, so a
hand-written psql probe of a query "passes" while the identical query 500s under sqlx — because
sqlx binds a typed parameter and decodes into a concrete Rust type with **no** implicit coercion.
Every enum column therefore needs `::<enumtype>` on a bound write and `::text` on a String read;
`numeric`→f64 needs `::float8`; `int4`→i64 needs `::bigint`. The META-root is deeper: this whole
class is only observable by executing the code against a **real Postgres**, and the rebuild's
DB-touching tests are all `#[ignore]` with nothing running them — so the class is invisible at
every static gate (unit/clippy/fmt all green) until a human drives the live surface. The Nth cast
is not the fix; running the live-PG suite in CI is (ledger #77 draft). Until that job lands, when a
psql repro contradicts a live 500, distrust the psql repro (it uses literals) and reproduce with a
BOUND param.
