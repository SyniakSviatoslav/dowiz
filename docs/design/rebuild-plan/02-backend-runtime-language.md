# Rebuild Plan — Lane 2: Backend Runtime & Language Strategy

Date: 2026-07-04 · Model: Fable 5 · Method: codebase verification (file:line) + 2026 web research (3 parallel research agents, URLs inline).
Philosophy applied: YAGNI/ponytail — a switch must have quantified ROI net of migration cost. "Rewrite it in X" for its own sake = FAIL.

## TL;DR decision table

| # | Area | From | To | Effort | Risk | Expected gain | Verdict |
|---|------|------|----|--------|------|---------------|---------|
| 1a | Runtime | Node 22 (Maintenance LTS) | **Node 24 LTS** | S (2 Dockerfile lines + CI) | Low | V8 13.6, stable permission model, N-API v10 ABI stability for sharp/argon2 | **ADOPT** |
| 1b | Runtime | Node | Bun / Deno | L | High (sharp/argon2/tesseract N-API gaps) | ~3–15% real-world throughput, ~15% RSS — not worth it | **REJECT** |
| 2 | Hot-path polyglot | TS | Go/Rust/Zig service or addon | L | High (polyglot ops) | None at current scale (1×512MB VM) | **REJECT (all 3 candidates)** |
| 3 | HTTP framework | Fastify 5 | Hono / Elysia | L | Med | ≈0% under real DB load | **KEEP Fastify** |
| 4a | Data access | raw `pg` + hand SQL | + **SafeQL ESLint plugin** (lints existing SQL vs live schema) | S–M | Low (lint-only, zero runtime change) | Type-checks the SQL we already wrote; preserves `withTenant`/`SET LOCAL` exactly | **ADOPT** |
| 4b | Data access | raw `pg` | Kysely for *new* modules (shares same `pg.Pool`) | M | Low–Med | Typed builder where useful; `sql` tag keeps GUC control | **PILOT (optional, non-blocking)** |
| 4c | Data access | raw `pg` | Drizzle ORM | L | Med–High | RLS policy API can't generate our migrations anyway | **REJECT** |
| 5 | Job queue | pg-boss 12 | graphile-worker / River / BullMQ | M–L | Med–High | None — we run ~0.1–1 job/sec; ceilings are 2–4 orders of magnitude away | **KEEP pg-boss** |
| 6a | Validation | Zod 3.23 | **Zod 4** (+ fastify-type-provider-zod v6/v7) | M (57 files) | Med | 6.5–14.7× parse, ~100× fewer tsc instantiations, −57% bundle | **ADOPT (staged)** |
| 6b | JWT | jose 6 | fast-jwt / @node-rs | S | Low | ~1µs/verify — noise | **KEEP jose** |
| 6c | Hashing | argon2 (C++) | @node-rs/argon2 | S | Low | µs on a deliberately-50–100ms op — noise | **KEEP argon2** |
| 6d | Logging | pino 9 | anything else | — | — | pino still fastest mainstream on Node | **KEEP pino** |
| 6e | WebSocket lib | ws 8 | uWebSockets.js | M | Med (no npm publish, custom binary, Docker/Fly friction) | Real only >10k concurrent conns; we're at hundreds | **KEEP ws** |
| — | Hygiene | `pdfjs-dist` dep | remove | S | Low | Dead dependency (zero references in src/build) | **ADOPT (verify then drop)** |

---

## Verified current state (codebase facts)

- **Node ≥22, ESM, TS 5.6**: `package.json:5-7` (`"node": ">=22"`), `Dockerfile:2,41` (`FROM node:22-slim`). esbuild bundles to CJS single files (`scripts/build-apps.ts:8-10`, `fly.toml:15-19` runs `dist/api/server.cjs` + `dist/worker/index.cjs`).
- **Size**: apps/api = **35,549 LOC** TS. Hot files: `apps/api/src/routes/orders.ts` (980), `apps/api/src/server.ts` (890), `apps/api/src/websocket.ts` (531), `apps/api/src/lib/order-pricing.ts` (184).
- **Deploy envelope (critical constraint)**: Fly.io `fra`, **web VM = 512MB, worker VM = 256MB** (`fly.toml:35-41`), `auto_stop_machines = false` (always warm — cold-start deltas are irrelevant). Two processes from one image; migrations via `release_command`.
- **RLS/GUC crown jewel**: `packages/platform/src/auth/tenant.ts:3-21` — `withTenant()` = `BEGIN` → `SELECT set_config('app.user_id', $1, true)` → fn(client) → `COMMIT` on a raw `pg` `PoolClient`. Also `SET LOCAL statement_timeout` (`orders.ts:119`), `set_config('app.current_tenant', …)` (`spa-proxy.ts:458,771`, `notifications/workers/index.ts:122`). Any data-access layer MUST preserve arbitrary statements inside its transactions.
- **Integer money**: pricing in minor units throughout `apps/api/src/lib/order-pricing.ts` (server-mirror + parity, per fee-council).
- **pg-boss 12.18**: `server.ts:231-245` (`PgBossQueueProvider` on the operational session-mode URL for LISTEN/NOTIFY), ~15 workers under `apps/api/src/workers/` + `apps/worker/src/handlers.ts`; retry/DLQ in use (`workers/backup/index.ts:41-49` `deadLetter: true`, `lib/signals/velocity-increment.ts:54` `retryLimit: 3`). Boot-resilience incident note at `server.ts:323`.
- **WS fan-out**: raw `ws` `WebSocketServer` sharing the Fastify HTTP server; per-channel rooms `Map<string, Set<RoomMember>>` (`websocket.ts:194`), Redis message-bus bridge (`packages/platform/src/message-bus.ts`), 30s heartbeat/zombie reaper (`websocket.ts:286-296`), re-validation on the broadcast path (authz hardening #4/#5, `websocket.ts:224,347`).
- **Image/OCR is NOT a hot path**: `sharp` is used only on owner theme upload (`routes/owner/themes.ts:5,127`) and brand extraction (`lib/brand-extractor.ts`). `tesseract.js` only in `lib/ai-ocr-parser.ts` (owner menu import — rare, operator-triggered), which already has an **external-process escape hatch** (`execFileSync` paddle-ocr seam, `tests/paddle-ocr-seam.test.ts`). `pdfjs-dist` (declared `apps/api/package.json:42`) has **zero references** in src or build scripts — PDF inputs flow to the AI parser port as bytes (`routes/owner/menu-import.ts:87,185`, `ports.ts:7`).
- **Auth**: jose RS256 sign/verify with alg pinning (`packages/platform/src/auth/jwt.ts:55-109`); argon2 hash/verify (`routes/courier/auth.ts:68-75`, `routes/auth/local.ts`). Zod at boundary in **57 files** via `fastify-type-provider-zod`.

---

## 1. Runtime: Node 22 → Node 24 LTS; Bun/Deno rejected

**From → To:** `node:22-slim` → `node:24-slim` (Docker base + CI matrix). Keep `engines: >=22` or bump to `>=22 <25`.

**WHY:** As of mid-2026, **Node 24 is Active LTS** (since 2025-10-28; Node 22 is Maintenance until 2027-04) — https://nodejs.org/en/about/previous-releases, https://endoflife.date/nodejs. Node 24 brings V8 13.6 (vs 12.4), a **stable permission model** (`--allow-fs-read`, `--allow-net`, `--allow-child-process` — free defense-in-depth for an app that shells out via `execFileSync` in the OCR seam) and stable type stripping — https://nodejs.org/en/blog/release/v24.0.0. N-API ABI stability (v10, stable since 22) means sharp/argon2 prebuilds keep working across the bump.

**Bun (REJECT for this stack):** Bun ~1.3.x runs Fastify and `pg`, but the **native-addon story is the disqualifier**: recurring sharp install/runtime failures (missing `.node`, musl `libvips-cpp.so`) — https://github.com/oven-sh/bun/issues/4549, https://github.com/oven-sh/bun/issues/4819, https://github.com/lovell/sharp/issues/4317; tesseract.js worker crashes — https://github.com/oven-sh/bun/issues/7984, https://github.com/oven-sh/bun/issues/11350; OTel needs Bun-specific bootstrapping — https://docs.sentry.io/platforms/javascript/guides/bun/opentelemetry/custom-setup/. Tellingly, Bun's 2026 answer is to *route around* N-API with its own `Bun.Image` and `Bun.password` replacements rather than fix compat — a migration would mean swapping sharp AND argon2 APIs, not just the runtime. Real-world gain once Postgres dominates the request: **~3–15% throughput, ~15% RSS under load** (the marketed 2.2× is a no-DB hello-world: 20,683 vs 9,340 req/s on Sharkbench — https://sharkbench.dev/web/javascript-fastify/bun-vs-nodejs). Cold-start wins are irrelevant with `auto_stop_machines = false`.

**Deno 2 (REJECT):** npm/N-API compat requires `node_modules` mode and **postinstall scripts don't run by default** — exactly how sharp/argon2 fetch prebuilds — https://docs.deno.com/runtime/fundamentals/node/, https://socket.dev/blog/deno-2. Least proven of the three for this dependency set.

**Node tuning captures the closable gap for free:** `--max-semi-space-size=64` for young-gen GC p95/p99 (https://github.com/nodejs/node/issues/42511); undici keep-alive/pool tuning showed ~50% p50/p95 latency cuts in a Fastify test (https://blog.platformatic.dev/http-fundamentals-understanding-undici-and-its-working-mechanism); a `worker_threads` pool (Piscina) if OCR/sharp ever moves off the request path.

**Effort S · Risk Low · Gain:** stay on supported LTS + permission model; avoids the Bun/Deno rewrite tax (~L effort, high risk) for single-digit % gains. **Verdict: ADOPT Node 24; REJECT Bun/Deno. Revisit only for a new stateless addon-free service.**

## 2. Faster languages for hot paths: all three candidates rejected

The honest test: would Go/Rust/Zig materially beat TS *at this app's actual scale* (one 512MB web VM, one 256MB worker VM, a delivery-orders workload)?

| Candidate | Reality check (verified) | Verdict |
|---|---|---|
| **WS fan-out gateway** (Go/Rust/uWS) | `websocket.ts` fans out per-room via `Map<string, Set>` — rooms are per-order/per-location-dashboard, so fan-out degree is *couriers + owner per location*, i.e. tens, not thousands. `ws` is fine below ~10k concurrent connections (https://www.pkgpulse.com/guides/socketio-vs-ws-vs-uwebsockets-websocket-servers-nodejs-2026). A separate gateway would also re-open the authz surface hardened in ADR-0013 (re-validation on the broadcast path, `websocket.ts:224`) — regression risk on a red-line for zero measured need. | **REJECT** |
| **Image/OCR pipeline** (Rust/Go service) | Not a hot path: sharp runs on owner theme upload only (`themes.ts:127`); tesseract on rare operator menu-imports, and the seam already supports an external binary via `execFileSync` (paddle-ocr). sharp is *already* C (libvips) under a thin JS binding — a Rust rewrite buys nothing. **Incremental step if it ever blocks the event loop: move parse jobs to the existing `apps/worker` process via pg-boss (queue + process already exist), or a Piscina worker pool. Same language, zero new ops.** | **REJECT (route to worker process instead)** |
| **Pricing hot loop** | `order-pricing.ts` is 184 LOC of integer arithmetic in minor units — this executes in microseconds in V8; the surrounding transaction (`withTenant` + inserts, `orders.ts:119` 4.5s statement timeout) is 3–4 orders of magnitude more expensive. Moving it to another language would *split the money invariant across languages* — a red-line regression risk with negative expected value. | **REJECT (hard)** |

**Polyglot tax (why the bar is high):** second toolchain in CI, second deploy artifact on Fly, cross-service authz/tracing, and the loss of the shared Zod/TS contract types (`@deliveryos/shared-types`). None of the three candidates clears it. **Expected-gain quantification: 0 measurable user-facing ms at current load; cost ≥ L per service. Verdict: KEEP TS everywhere; the only sanctioned "faster-language" pattern remains what's already used — native libs behind npm bindings (sharp/argon2) and external binaries behind the existing OCR port seam.**

## 3. HTTP framework: KEEP Fastify 5

Fastify is healthy (v5.9.0 June 2026, 5 lead + 14 core maintainers — https://github.com/fastify/fastify). 2026 benchmarks: hello-world Fastify ~130k req/s vs Hono-on-Node ~120k; **with a real Postgres query in the path the gap collapses to ~13k vs ~12k req/s, and with JWT+ORM both converge to 4–6k** — https://www.pkgpulse.com/guides/hono-vs-express-vs-fastify-2026, https://hono.dev/docs/concepts/benchmarks. Hono's Node adapter lacks the depth of `@fastify/multipart`/`@fastify/rate-limit` (both in use, `apps/api/package.json:27-28`); Elysia remains Bun-first (https://bun.com/docs/guides/ecosystem/elysia). Migrating 35.5k LOC + the fastify-plugin decorator graph (`fastify.verifyAuth`, `requireRole`, per-route type providers) for ≈0% real gain is a textbook YAGNI FAIL. **Effort to switch: L · Gain: ~0 · Verdict: KEEP.**

## 4. Data access: keep raw SQL, make it *checked* — SafeQL now, Kysely optional pilot

The tension: `withTenant`'s GUC transaction (`tenant.ts:10-13`), `SET LOCAL` timeouts, RLS-FORCE, integer money, hand-tuned SQL. Anything that hides SQL or owns the transaction lifecycle risks all four.

- **ADOPT — SafeQL (`@ts-safeql/eslint-plugin`)**: lints the SQL strings we *already wrote* against a live/introspected schema at lint time; zero runtime change, zero new abstraction, works with raw `pg`; actively maintained (v5 roadmap, June 2026 — https://safeql.dev/blog/safeql-v5-roadmap.html, https://github.com/ts-safeql/safeql). Fits the existing gate topology (we already run a local ESLint plugin, `tools/eslint-plugin-local`). Cost: wiring a shadow DB/schema URL into lint; queries may need the `sql` tag or config for template recognition. **Effort S–M · Risk Low (advisory→gate, no runtime path touched) · Gain: column/type drift in ~57 route files caught at lint instead of prod.**
- **PILOT (optional) — Kysely 0.29.x for new modules only**: `PostgresDialect` wraps the *existing* `pg.Pool` (no second pool); `Transaction<DB>` accepts arbitrary `sql` statements, so a `withTenantKysely` can run the same `set_config` — https://kysely.dev/docs/recipes/raw-sql, https://kysely.dev/docs/examples/transactions/controlled-transaction; types generated from the live DB via kysely-codegen — https://github.com/RobinBlomberg/kysely-codegen. Adopted in production by Cal.com/Maersk/Deno. Strictly coexist-first; never rewrite existing hand-tuned queries. **Only if new-module velocity justifies it — otherwise skip.**
- **REJECT — Drizzle**: its RLS policy API cannot generate/diff policies in migrations (open RFC — https://github.com/drizzle-team/drizzle-orm/discussions/2450), so we'd hand-write RLS migrations anyway; drizzle-kit auto-migrations are the opposite of our forward-only node-pg-migrate discipline (prod-safety critiques: https://fixdevs.com/blog/drizzle-orm-not-working/). Net add ≈ a schema DSL we don't need.
- **REJECT — pgTyped** (single maintainer, ~1yr stale — https://www.npmjs.com/package/@pgtyped/cli); **REJECT — sqlc-gen-typescript** (preview WASM plugin + Go toolchain — https://github.com/sqlc-dev/sqlc-gen-typescript).

🔴 **RED-LINE:** any change under `withTenant`, money SQL, or `packages/db/migrations/` is council-gated. SafeQL qualifies as a *gate addition* (allowed, ratchet-positive); it must be introduced warn-first → error, never weakening existing gates.

## 5. Job queue: KEEP pg-boss 12

Verified usage: transactional-adjacent enqueue with order data, retry/DLQ/cron in ~15 workers, session-mode LISTEN/NOTIFY (`server.ts:240-245`). Load is hundreds-to-low-thousands of jobs/**hour** (≈0.1–1/sec). Ceilings: pg-boss is built exactly for this (SKIP LOCKED, backoff, DLQ, cron — https://timgit.github.io/pg-boss/, active releases through 12.24.x — https://www.npmjs.com/package/pg-boss); graphile-worker's headline ~196k jobs/s is a tuned burst figure with realistic sustained figures far lower (https://worker.graphile.org/docs/performance, https://news.ycombinator.com/item?id=46614277); River has only an **insert-only** Node client — a Go binary would have to drain the queue (https://github.com/riverqueue/river/discussions/369) → non-viable; BullMQ moves enqueue *outside* the Postgres transaction, re-creating the dual-write problem pg-boss solves for free. **We are 2–4 orders of magnitude below any ceiling. Effort to switch: M–L · Gain: negative (loses transactional enqueue) · Verdict: KEEP. Minor action: track pg-boss 12.24.x patch upgrades (S).**

## 6. Validation / JWT / hashing / logging / WS lib

- **Zod 3 → 4 — ADOPT, staged (the one real library win).** Measured: 14.7× faster string parse, 7.4× arrays, 6.5× objects, −57% bundle, ~100× fewer tsc instantiations (a 4000ms→400ms compile case) — https://zod.dev/v4. We Zod-parse on every request boundary in 57 files, and tsc time is a daily dev cost. Migration is real work: error-API consolidation, top-level string formats, ZodEffects removal; plus `fastify-type-provider-zod` v6 (Zod 4) then v7 (requires Zod 4.2, switches response serialization to `z.output<T>`) — https://github.com/turkerdev/fastify-type-provider-zod. Stage it: bump provider to v6 + Zod 4 with codemod, defer v7. **Effort M · Risk Med (57 boundary files — full E2E suite is the gate) · Gain: quantified parse + compile-time wins.** Valibot: bundle-size play, irrelevant server-side — REJECT (https://valibot.dev/guides/comparison/).
- **jose — KEEP.** RS256 verify is ~56µs across jose/fast-jwt/jsonwebtoken (https://github.com/nearform/fast-jwt/blob/master/benchmarks/README.md) — never a bottleneck at our req/s; jose is zero-dep, WebCrypto, actively maintained (https://github.com/panva/jose). Swapping a security-critical lib for ~1µs is negative-EV. 🔴 auth = council-gated anyway.
- **argon2 — KEEP.** @node-rs/argon2 is ~1.47× faster *binding overhead* on an op that is deliberately 50–100ms (https://npm-compare.com/@node-rs/argon2) — noise. Revisit only on measured libuv threadpool contention. 🔴 auth red-line.
- **pino — KEEP.** Still the fastest mainstream Node logger (~222k ops/s vs winston 36k; v10 line active — https://github.com/pinojs/pino/releases).
- **ws — KEEP.** uWebSockets.js gains (5–10× throughput, <200MB vs ~1.5GB heap at scale — https://www.pkgpulse.com/guides/socketio-vs-ws-vs-uwebsockets-websocket-servers-nodejs-2026) only materialize at tens of thousands of connections; it isn't on npm (GitHub-tag install, per-platform binaries — https://github.com/uNetworking/uWebSockets.js/) — bad fit for the Fly Docker pipeline at our scale.
- **Hygiene — `pdfjs-dist` is a dead dependency** (declared `apps/api/package.json:42`; zero imports in src/build; PDF bytes go to the AI parser port). Drop it after a `pnpm build` + menu-import E2E proof. Shrinks install/image surface for free.

## Invariant-regression check (what must NOT regress)

| Invariant | Threatened by | Protection in this plan |
|---|---|---|
| RLS-FORCE + GUC (`withTenant`) | ORMs owning the tx lifecycle | Only SafeQL (lint-only) adopted; Kysely pilot must reuse `set_config` pattern; Drizzle rejected |
| Integer money | Cross-language pricing split; ORM numeric coercion | Pricing stays TS; no data-layer runtime change |
| Zod-at-boundary | Zod 4 breaking changes | Staged migration; E2E + typecheck gate; provider v6 before v7 |
| WS authz (ADR-0013 broadcast re-validation) | External WS gateway | No gateway; `ws` kept in-process |
| Transactional enqueue (order + job atomically) | BullMQ/Redis queue | pg-boss kept |

## Sequenced adoption plan (all coexist-first)

1. **S — now:** drop `pdfjs-dist`; pg-boss patch bump; Node flag `--max-semi-space-size` experiment on staging (measure p95 before/after — perf skill applies, no blind flags).
2. **S — next image bump:** `node:22-slim` → `node:24-slim` on staging → prod (Node 22 is Maintenance until 2027-04, so this is scheduled hygiene, not urgent).
3. **S–M — next quality sprint:** SafeQL warn-mode on `apps/api/src/routes/`, then ratchet to error per-directory.
4. **M — dedicated branch:** Zod 3→4 + fastify-type-provider-zod v6 (codemod + full E2E + typecheck proof), defer v7.
5. **Optional/PILOT:** Kysely for the next genuinely new module only. No rewrites of existing SQL.
6. **Explicit no-ops recorded:** Bun, Deno, Hono, Elysia, Drizzle, graphile-worker, River, BullMQ, uWS, fast-jwt, @node-rs/argon2, valibot, Go/Rust/Zig services.
