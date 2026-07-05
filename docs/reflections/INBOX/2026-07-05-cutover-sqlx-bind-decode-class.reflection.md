---
CONTEXT:   Rust strangler cutover on staging — flipping S1–S10 surface by surface, driving one
           order through the full lifecycle L0–L7+L11 live, and porting the strangler tail
           (dwell-settings). Across the session ~10+ distinct 500s/503s appeared, each on the
           first LIVE drive of a surface (never in the 838 green unit tests).
DECISIONS: Fix each cast at its site (numeric::float8, int4::bigint, text→enum ::<enumtype> on
           binds, enum→String ::text on reads) AND, when the same root recurred a 4th time,
           STOP fixing sites and build the class-level ratchet instead (docs/design/ci-rust-live-pg/
           — a CI job running the #[ignore] live-PG suite). Ran that suite manually against the
           staging DB to batch-catch the whole class rather than one-live-probe-at-a-time.
           Held the courier-deliver conflict-bool race + S8 webhook path-secret as council items
           (money/auth business decisions), not reflexive patches. Held all prod flips at the gate.
WHERE:     rebuild/crates/api — orders/pg.rs (request_hash NOT-NULL + type::order_type +
           order_status_history::order_status + payment_outcome-read + tax_rate::float8),
           courier/assignments.rs (payment_method-read ::text + orders.payment_outcome-write
           ::payment_outcome), owner/product_media.rs (kind::product_media_kind), repo.rs
           (numeric/int4 casts + geo column), dto.rs (js-number + toISOString wire). The batch
           live-PG run: 833 pass / 3 fail, all 3 test-infra (advisory-lock contention with the
           live app; a "degrades-while-unapplied" test vs the fn I'd applied on staging; a probe
           with a random location_id that FK-fails before RLS) — ZERO code defects of the class left.
WHY:       ONE root under every symptom: **a hand-written psql literal probe and the sqlx
           bind/decode path have different type rules, so the probe is a false witness.** Postgres
           implicitly coerces an unknown-type SQL literal (`'pickup'`, `42`) to the target column
           type, and psql RENDERS every value as text — so `SELECT ... o.type` and
           `INSERT ... VALUES ('pickup')` both "work" in psql. But sqlx sends a bound parameter
           with an explicit wire OID and decodes into a concrete Rust type with NO implicit
           coercion — so the enum column rejects the bound text, and the String rejects the
           decoded enum. I repeatedly "confirmed" a fix in psql and it still 500'd live, because
           I was testing the wrong path. The META-root is deeper: this entire class is ONLY
           observable by executing the code against a real Postgres, and the rebuild's DB tests
           are all `#[ignore]` with nothing running them — so the class was structurally invisible
           at every gate (unit tests green, clippy green, fmt green) until a human drove the live
           surface. The cost was ~8 redeploy cycles discovering serially what one CI job discovers
           in parallel. The fix that MATTERS is not the Nth cast — it is running the live-PG suite
           in CI. Corollary learned twice: when a psql repro contradicts a live 500, distrust the
           psql repro (it uses literals); reproduce with a BOUND param (`PREPARE t(text) AS ...;
           EXECUTE t('pickup')`) to see the real type error.
---

Advisory only — feeds the Council retro, which decides the deterministic artifact. Candidate
artifacts already in flight: regression-ledger #77 (this class) + the `rust-live-pg` CI job draft
(the guardrail). Ratchet-refinements this run surfaced for that job: (1) do NOT apply 086/087
before the refund "degrades-while-unapplied" arm; (2) the create/erasure probes need a seeded
canonical location, not `Uuid::new_v4()`; (3) the advisory-lock cron tests need a single runner
(fresh CI DB with no app attached — which CI provides, staging does not).
