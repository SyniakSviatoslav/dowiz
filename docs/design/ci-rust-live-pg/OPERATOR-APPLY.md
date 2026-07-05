# OPERATOR-APPLY — wire the Rust live-PG suite into CI (the cutover ratchet)

> **Why this is the top follow-up.** Every SQL defect found during the Rust staging cutover
> (2026-07-05) was one class: **passes as a psql literal / text render, fails as a sqlx bind or
> decode.** ~14 fixes across S1/S4/S5/S7/S9/S10, each caught only by driving the live stack one
> redeploy at a time. The rebuild's DB tests are all `#[ignore]` and **nothing in CI runs them**,
> so the class was invisible at build time. This job makes it a red build instead.

## What to place

`docs/design/ci-rust-live-pg/rust-live-pg.job.yml` → a new job under `jobs:` in
`.github/workflows/ci.yml` (this dir is a red-line protect-path — an agent can't write it, so it
ships as a draft, the same flow as the 085/086/087/088 migration drafts and the deploy drafts).

It reuses `fresh-provision`'s exact Postgres 16 + `dowiz_migrator`/`dowiz_app` roles + migration
chain, then runs `cargo test --features dev-routes -- --include-ignored` in `rebuild/`. The helper
`scripts/rebuild-cutover/apply-migration-draft.mjs` (committed, self-tested) applies the operator-
gated DEFINER drafts so the S9/S10 arms find their functions.

## Before merging — two checks the draft can't make for you

1. **MSRV pin.** The job pins toolchain `1.85` (the `rebuild/Cargo.toml` `[workspace.package]
   rust-version`). If that floor moves, bump both in lockstep.
2. **The skip list is the danger.** The run ends with `--skip channels:: --skip bridge:: --skip
   r2_storage` (external creds: Resend/Telegram/R2). Keep this list SHORT and reviewed in the PR —
   **a silent skip is exactly how the bind/decode class hid the first time.** If an arm needs a
   fixture (a seeded location/order), seed it in the "Apply schema" step; do NOT skip it away.

## What it would have caught (regression-ledger the class)

| Bug | Site | Symptom it 500'd |
|---|---|---|
| numeric→f64 | `repo.rs` location_info, `orders/pg.rs` tax_rate | /info 503, order-create 500 |
| int4→i64 | gdpr count(*)::int | stranded erasure request |
| text→enum bind | `orders.type`, `order_status_history`, `product_media.kind` | create + status + media-confirm 500 |
| enum→String decode | `o.type`, `o.payment_method` | owner-transition + courier-deliver 500 |
| NOT NULL omit | `request_hash` | order-create 500 |

Each is a `#[ignore]` arm that exercises the real SQL against real Postgres. With this job, the
next one fails the PR instead of a staging redeploy.

## Verification after placing

Push a branch that reintroduces ANY one cast removal (e.g. drop `::order_type` from
`CREATE_ORDER_SQL`) → the `rust-live-pg` job must go red. That red→green is the proof the ratchet
is live (regression-ledger it per the self-improvement loop).
