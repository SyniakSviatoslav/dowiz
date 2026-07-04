# 06 — Decided Rebuild Stack (Rust + Astro/Svelte islands)

- **Date:** 2026-07-04 · **Decider:** operator (explicit, in-session). The earlier no-rewrite verdict
  (REBUILD-PLAN.md §1) was **overridden by the operator**: a complete rebuild is the chosen program.
- **Council note:** the operator waived a council on the *stack choice itself*. Red-line **implementation
  gates are unchanged**: each 🔴 surface port (money/orders, auth/JWT, RLS/tenancy, WS authz,
  migrations) still gets a Triadic Council + red→green proof before its code lands. The Ethics
  Charter and Mandatory Proof Rule apply to the rebuild in full.
- **Selection criteria (operator):** speed, reliability, overall performance, memory/resource usage;
  long-term **momentum**.

## 1. The stack

| Layer | Decision | Notes |
|---|---|---|
| Backend language | **Rust** (stable, 2024 edition) | Chosen over Go (operator pick; max correctness/perf/memory) — comparison in the chat brief + `02-*.md` grounding |
| HTTP | **axum + tokio + tower** | de-facto standard; tower middleware for rate-limit/timeouts |
| Data access | **sqlx** (Postgres, compile-checked, offline mode in CI) | Tenancy via `BEGIN; SET LOCAL app.current_tenant; …; COMMIT` — ports withTenant verbatim |
| API contract | **OpenAPI 3.1 as SSOT** (utoipa on Rust side → openapi-typescript FE client) | Replaces shared-types/Zod as the cross-boundary type authority |
| Auth | RS256 via mature Rust JWT crate + argon2 crate | 🔴 council-gated port; per-request live membership re-check preserved (ADR-0004) |
| Realtime | **axum WebSocket + tokio broadcast + sqlx PgListener** (Pg LISTEN/NOTIFY) | Ports the proven multi-instance pattern; tri-state authz re-implemented 🔴 (ADR-0013) |
| Jobs | Postgres-backed Rust queue — **decision pending Lane A research** (apalis-postgres vs underway vs pgmq vs hand-rolled SKIP LOCKED) | Hard requirements: transactional enqueue, retry/backoff, DLQ, cron, singleton keys (pg-boss parity) |
| Imaging / OCR / PDF | **pending Lane A research** (libvips Rust bindings vs image+ravif; tesseract via leptess vs sidecar; pdfium-render) | Must preserve AVIF/WebP pipeline plan + menu-import |
| Frontend | **Astro 5.x shell + Svelte 5 islands** | One app, route groups: `/s/[slug]` (SSR/prerender), `/admin`, `/courier` (island-heavy) |
| FE state/data | Svelte 5 runes in islands + OpenAPI-generated client | apiClient semantics (single-flight, cross-tab locks) re-evaluated per-island |
| Styling | Tailwind (port tokens/themes/per-tenant branding) | Token SoT preserved |
| i18n | **pending Lane B research** (Paraglide-JS vs direct port of catalog SSOT) | al/en/uk; parity gate preserved as CI check |
| Database | **UNCHANGED: Supabase Postgres + RLS FORCE** — the rebuild is code-only against the live schema; data never migrates | RLS initplan wrapping done during transition (council-gated, per REBUILD-PLAN A6) |
| Migrations | history frozen (157 node-pg-migrate stay applied); new-era tool **pending Lane C decision** (sqlx::migrate vs refinery) | Forward-only + boot schema-guard equivalent preserved |
| Cache | in-process capped LRU + bus invalidation; **no Redis** | Lane 4 finding carried over |
| Infra | **Fly.io fra + Docker scratch/static image (~15–25 MB) + R2** | Same topology; expect ~10–30 MB RSS |
| Guardrails | clippy deny-set + cargo-deny/cargo-vet + sqlx offline checks + eslint-plugin-local intent catalog ported (Lane D) | The ratchet culture ports into the compiler + CI |
| Proof net | **Playwright E2E suite KEPT — it is the language-independent parity oracle** | Per-surface cutover only on green slice |

## 2. Contingency triggers (recorded, not planned)

- **Elixir realtime tier**: reopen ONLY if concurrent WS exceeds ~10–20k or clustered presence becomes
  product-core. Options then: Phoenix Channels tier or pure-Rust tokio gateway; nothing in this stack
  forecloses either.
- **SvelteKit extraction**: if `/admin` outgrows islands ergonomics, extract that route group into
  SvelteKit; the Astro shell + contract boundary make this a contained move.
- **Go fallback**: if Rust velocity proves prohibitive in Phase A spike, Go stack from the comparison
  brief is the pre-researched fallback. Trigger: Phase A storefront-read surface not shippable in the
  spike window with parity green.

## 3. Adoption strategy (from the Rust-vs-Elixir long-term brief)

Phase A contract+spike (storefront-read in axum behind existing proxy, parity-measured) →
Phase B strangler by surface (auth → catalog/admin → 🔴 orders/money → 🔴 realtime), each surface
cut over only on its green E2E slice, councils on 🔴 ports → ratchet ports (clippy/cargo-deny/sqlx)
wired from Phase A. Elixir/SvelteKit remain recorded contingencies (§2).

## 4. What follows this doc

The **full functional inventory + traceability map** (`inventory/10–14`, synthesized in
`REBUILD-MAP.md`): every route, WS message, job, table/policy, page, component, UI element, script,
guardrail, flag, env var, integration and test — each with a rebuild target and a proof artifact, plus
machine-extractable counts so map completeness is verifiable, never narrated.
