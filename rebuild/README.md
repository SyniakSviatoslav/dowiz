# DeliveryOS Rust rebuild — Phase A workspace + S1 storefront-read surface

Scaffold for the from-scratch Rust rebuild (`docs/design/rebuild-plan/06-complete-rebuild-stack.md`
+ `REBUILD-MAP.md`, main-tree paths — not yet merged into every branch). This directory is a
**standalone Cargo workspace** at the repo top level; it is not part of the pnpm workspace
(`pnpm-workspace.yaml` only globs `apps/*`, `packages/*`, `tools/*`, `spikes/*` — `rebuild/`
matches none of them) and touches no existing Node/TS code.

**Update (S1 lane, contract-first):** the `crates/api` menu stub below has been replaced by the
full **S1 storefront-read surface** — all 20 operations in
`docs/design/rebuild-plan/openapi-contracts/openapi-s1-storefront-read.yaml`, ported verbatim
from the live Node source with `utoipa` annotations regenerating the contract at `/openapi.json`.
See "S1 storefront-read (this build)" below for what's real vs. flagged.

## What's here

```
rebuild/
├── Cargo.toml              workspace manifest + lint posture (deny(warnings) via clippy::all)
├── rust-toolchain.toml     stable + rustfmt + clippy components
├── crates/
│   ├── domain/             pure invariant core — NO IO, NO sqlx/tokio/axum (+ utoipa, see error.rs)
│   │   └── src/
│   │       ├── money.rs       Lek(i64) — checked-only arithmetic, no float construction (COUNCIL-LOCKED, unmodified)
│   │       ├── order_status.rs OrderStatus (10 values) + can_transition/assert_transition/is_terminal (COUNCIL-LOCKED, unmodified)
│   │       ├── tenant.rs       TenantId newtype (uuid) (unmodified)
│   │       └── error.rs       DomainError + ErrorCode + ErrorEnvelope (ADR-0010 shape) — EXTENDED for
│   │                          S1 (ServiceUnavailable/InvalidKey codes, http_status() table, always-
│   │                          present status/error fields, utoipa::ToSchema derives)
│   └── api/                axum crate — the only crate that does IO
│       └── src/
│           ├── config.rs      fail-fast env validation (PORT, DATABASE_URL_OPERATIONAL/SESSION) — unmodified
│           ├── db.rs           Pools + with_tenant — unmodified code; doc updated (S1 resolves the
│           │                   "dual tenant GUC" question: no S1 route needs with_tenant, verified)
│           ├── error.rs        ErrorCode -> HTTP status -> axum Response (now delegates to domain::ErrorCode::http_status)
│           ├── openapi.rs      utoipa OpenAPI 3.1 document + /openapi.json — all 20 S1 operations + schemas
│           ├── repo.rs         PublicRepo trait (S1 data access) + PgRepo (sqlx) + FakeRepo (#[cfg(test)])
│           ├── service.rs      pure mapping functions shared by handlers (image URLs, open/closed/busy, etc.)
│           ├── storage.rs      Storage trait (read-only) + LocalFsStorage + traversal-guard/content-type
│           ├── dto.rs          S1 wire DTOs (PublicMenu, PublicLocationInfo, PublicTheme, ProductMedia, ...)
│           ├── routes/
│           │   ├── health.rs          /healthz, /livez
│           │   ├── menu.rs            getPublicMenu, getPublicLocationInfo, getProductMedia
│           │   ├── theme.rs           getPublicTheme, getThemeCss
│           │   ├── storefront.rs      getStorefrontPage + 4 SPA-shell ops (SCOPE CUT, see module doc)
│           │   ├── manifest.rs        getWebManifest
│           │   ├── fallback_config.rs getFallbackConfig
│           │   ├── media_proxy.rs     getImage, getMediaObject
│           │   ├── voice_config.rs    getVoiceConfig
│           │   ├── vapid.rs           getVapidPublicKey
│           │   ├── rates.rs           getExchangeRate
│           │   └── seo.rs             getRobotsTxt, getSitemapIndex, getSitemapShard
│           └── main.rs         tower layers, graceful shutdown w/ deadline, router wiring (all 20 routes)
```

## S1 storefront-read (this build)

**Built, tested, real logic:** all 15 pure-JSON/text/binary operations (menu, location info,
lazy product media, theme JSON+CSS, manifest, fallback-config, image/media proxies, voice-config,
VAPID key, exchange rate, robots.txt, sitemap index+shards) — DB access behind `PublicRepo`
(`#[cfg(test)]`-stubbed via `FakeRepo`), pure mapping functions (venue open/closed/busy,
image-URL resolution, shadow-preview adaptation, CSP building, etc.) unit-tested independent of
axum/sqlx.

**Built, but a documented scope cut:** `getStorefrontPage` + the 4 SPA-shell operations
(cart/checkout/order/order-legacy) port the REAL branching logic (bot/human, shadow-tenant
detection + the 🔴 P6-2/P6-3 noindex+generic-OG privacy invariant, per-tenant CSP) but render a
minimal HTML placeholder instead of the pixel-identical preact-hydrated page — see
`crates/api/src/routes/storefront.rs`'s module doc for why (no Vite build artifact exists in a
pure-Rust workspace, and the contract's own description calls this "the Astro handoff seam").

**Resolved, not deferred:** `db.rs`'s "dual tenant GUC" open question, for S1 specifically —
every S1 route is unauthenticated and verified (against live Node) to never call `withTenant`;
`with_tenant` correctly remains uncalled after this build (see `db.rs`/`repo.rs` module docs).

**New follow-ups this build surfaced (not in the original Phase-A open-questions list):**
1. No menu/info in-process TTL cache (Node's connection-burst guard, `menu.ts:76-111`) — an
   axum/tokio caching-layer concern orthogonal to the data-shape port; flagged per-handler.
2. No R2 storage backend (`r2-storage.ts`) — only `LocalFsStorage` is wired; S1's `/images/*`
   and `/media/*` need a real object-store client before a staging deploy.
3. No rate-limiting middleware — the contract's `RateLimited` response component exists in the
   OpenAPI schema but no tower layer enforces it yet (cross-cutting, not per-operation).
4. `getStorefrontOrderPageLegacy`'s x-quirk (port-as-alias-vs-second-handler decision) was left
   as its own handler, matching the contract's explicit "port decision row" framing.

## Build / test / run

```sh
cd rebuild
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test                      # unit tests only — no DB required
cargo test -- --ignored          # + the with_tenant integration test, needs a live Postgres:
                                  #   DATABASE_URL_OPERATIONAL / DATABASE_URL_SESSION
cargo run -p api                 # needs PORT (default 8080) + both DATABASE_URL_* set
```

## What is deliberately NOT here yet

- **No media-worker crate.** Imaging/OCR/PDF (libvips/tesseract/pdfium) is a separate image per
  the Lane A verdict (REBUILD-MAP §2) — lands with Phase B S4.
- **No job queue.** The hand-rolled SKIP LOCKED + PgListener queue (REBUILD-MAP §2 decision
  register) is Phase B S8.
- **No auth.** No JWT/argon2 crate wiring — auth is Phase B S2, council-gated (🔴).
- **No `cargo sqlx prepare` / compile-checked queries.** Every sqlx call in this workspace uses the
  runtime `sqlx::query()`/`query_scalar()` API, not the `query!`/`query_as!` macros, because there
  is no reachable Postgres + `.sqlx/` offline cache in this build environment. This is fine for
  Phase A (the only live query is a fixed-literal `set_config` call with one bound parameter — not
  user-controlled SQL), but it means the `no-raw-sql` guardrail (inventory/13 §2b row 2: a
  clippy.toml `disallowed-methods` ban on runtime `sqlx::query(`) **cannot be turned on as-is**
  without an explicit allow for `with_tenant`'s GUC-scoping call — see Open Questions.
- **No `rebuild/Dockerfile`.** The design is fully specified below, but this repo's
  `protect-paths.sh` hook hard-blocks writing any file literally named `Dockerfile` anywhere in the
  tree (infra red-line, no self-service override — "requires manual approval"). A human must
  create `rebuild/Dockerfile` with the content below (or explicitly approve it) before the CI
  image-build stage can run. This is not a workaround-able gap; it is the gate working as designed.

### Pending `rebuild/Dockerfile` (human must create this file)

```dockerfile
# syntax=docker/dockerfile:1
#
# cargo-chef -> musl static -> scratch, per REBUILD-MAP inventory/13 §Dockerfile /
# docs/design/rebuild-plan/06-complete-rebuild-stack.md ("Fly.io fra + Docker scratch/static
# image (~15-25 MB) + R2"). Two-stage dependency caching (chef prepare/cook) so a source-only
# change doesn't re-download/re-build the dependency graph. rustls (not native-tls/OpenSSL) is
# used throughout (see crates/api/Cargo.toml) specifically so this links statically against musl
# without an OpenSSL cross-compile step.
#
# NOT YET PRESENT (honest about Phase A scope): no media-worker image (libvips/tesseract/pdfium —
# REBUILD-MAP §1 "Imaging/OCR/PDF", lands with Phase B S4); no Astro `dist/` COPY (frontend lands
# separately); no HEALTHCHECK using a shell (scratch has no shell — Fly's own healthcheck config
# hits /healthz over HTTP instead, see fly.toml when that's authored).

FROM rust:1-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN rustup target add x86_64-unknown-linux-musl \
    && apt-get update \
    && apt-get install -y --no-install-recommends musl-tools pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
# Dependency layer — cached across builds as long as Cargo.toml/Cargo.lock don't change.
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin api

FROM scratch AS runtime
# CA certs for the TLS connection to Supavisor/Postgres (rustls needs a trust store; scratch has
# none by default) — see inventory/13's "scratch (or distroless-static if CA certs/tz needed)".
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/api /app/server
EXPOSE 8080
ENTRYPOINT ["/app/server"]
```

## Phase-A go/no-go criteria (REBUILD-MAP §3 Phase A)

- [ ] OpenAPI SSOT extracted from the 236-route census.
- [ ] Rust workspace scaffold (crates: `api`, `domain`, `media-worker`) — **this deliverable ships
      `api` + `domain` only**; `media-worker` is out of scope for this build (no imaging call-site
      exists yet to justify it — YAGNI until Phase B S4).
- [ ] Two-image build (api scratch + media-worker debian-slim) — **blocked on the human-approved
      `Dockerfile` above**; `media-worker` image itself is Phase B S4.
- [ ] Storefront-read surface (menu read + Astro `/s/[slug]` SSR) live behind the existing proxy —
      **not in this build**: this ships the axum stub route only, returning 501. The Astro
      frontend is a separate lane.
- [ ] sqlx GUC pattern + Supavisor answer — **`with_tenant` is built and unit-tested here**
      (`crates/api/src/db.rs`); the Supavisor cache-off-vs-:5432 decision itself is an open
      question below, explicitly deferred to the Phase A spike per REBUILD-MAP §Decision register.
- [ ] Paraglide spike (≤8 kB gz overhead check) — not in this build (frontend lane).
- [ ] Parity: storefront E2E slice green + RSS/p99 measured vs Node — **cannot run**: no deployed
      binary, no staging environment for this scaffold. This is unit/clippy-tested only; there is
      no Playwright/staging proof for this change (per the Mandatory Proof Rule, this must be
      stated explicitly rather than silently omitted — this is a backend-scaffold change with zero
      UI surface and zero wired API surface; the `cargo test`/`cargo clippy` output below is the
      available proof).
- [ ] Go/no-go fallback trigger (Go stack) evaluated — not applicable yet; nothing here contradicts
      Rust velocity.

**Net: this build covers a slice of Phase A** (workspace scaffold + `with_tenant` + config +
health/menu-stub + OpenAPI skeleton), not the full Phase A exit bar. The remaining Phase-A items
(OpenAPI-from-census, Astro SSR slice, Supavisor spike, Paraglide spike, staging parity
measurement) are separate, larger lanes.

## Open questions for the contract lane

1. **Dual tenant GUC.** The live schema's RLS policies use TWO GUCs — `app.current_tenant`
   (~102 sites, courier/service path) and `app.user_id` (~34 sites, owner path) — see
   REBUILD-MAP inventory/12 §7. This build's `with_tenant` (`crates/api/src/db.rs`) implements
   `app.current_tenant` only, per the build brief. A second `with_user`-style helper (or a unified
   `TenantCtx` that sets both) is needed before Phase B surfaces that rely on the owner path can
   port cleanly.
2. **`no-raw-sql` guardrail vs. `with_tenant`'s runtime query.** Inventory/13 §2b's Rust analog for
   "no SQL string interpolation" is a `clippy.toml disallowed-methods` ban on runtime
   `sqlx::query(`, with `sqlx::query!`/`query_as!` (compile-checked) as the sanctioned form. This
   build cannot use the compile-checked macros (no reachable Postgres + `.sqlx/` cache in this
   environment) for the one query that exists (`with_tenant`'s `SET_TENANT_STATEMENT`, a fixed
   literal with one bound parameter, not user-controlled). The contract lane needs to either (a)
   stand up a schema-shadow DB so `cargo sqlx prepare` can run and every query becomes a `query!`
   macro, or (b) carve an explicit, narrow `#[allow(clippy::disallowed_methods, reason = "...")]`
   for this one call site before turning the lint on repo-wide.
3. **Money storage-width boundary (i32 vs i64).** `domain::Lek` is `i64` (Phase A invariant-core
   decision, council-cleared — see `docs/design/rust-money-newtype-phase-a/`); the live Postgres
   money columns are `integer` (i32) with `CHECK >= 0`. The decode/encode boundary
   (`i32::try_from(lek.minor_units())`, never `as i32`) has no code yet — it lands with the first
   real sqlx query in Phase B, and needs its own clippy `as_conversions`-at-money-boundary
   enforcement (deferred, see `docs/design/rust-money-newtype-phase-a/resolution.md` item O-9).
4. **Directional/signed money has no home yet.** `Lek` is deliberately non-negative-only (matches
   the schema's `CHECK >= 0` + the existing `assertNonNegative`-throws-not-clamps semantics). A
   refund/settlement/owed-back quantity is NOT representable in this type by design — Counsel's
   open question (`docs/design/rust-money-newtype-phase-a/counsel-opinion.md`): should a named
   `SignedLek`/ledger-delta type be a scoped Phase-B S5 deliverable, or an accepted latent risk
   until a real refund path is coded in Rust? Needs a human/S5-lead decision, not a default.
5. **`Deserialize` on `Lek` is sign-checked, not authority-checked.** A client-submitted
   `{"total": 1}` decodes into a perfectly valid `Lek(1)` — this type only proves "not negative,"
   never "this is the price the server actually charges." The order-create transaction remains the
   sole amount authority (unchanged red-line); worth restating here so a future reader of
   `crates/api` doesn't assume `Lek`-typed request bodies are already trustworthy.
6. **JSON f64 precision above 2^53.** `Lek` serializes as a bare JSON integer. Any future
   browser-facing boundary that reads a `Lek` via `JSON.parse` will silently lose precision above
   `2^53` (9,007,199,254,740,992) — no such consumer exists yet in this build, but the first one
   that does must either string-encode `Lek` over the wire or prove all values stay under `2^53`
   (recorded as O-7 in the money-newtype resolution doc).

## Design review trail

The money newtype (`crates/domain/src/money.rs`) went through a full Triadic Council before
landing (this repo's `protect-paths`/`serious-gate` hooks treat money as a red-line surface even
for a pure, unwired scaffold type) — see `docs/design/rust-money-newtype-phase-a/{proposal,
breaker-findings, counsel-opinion, resolution}.md` and `docs/adr/ADR-rust-money-newtype.md`.
