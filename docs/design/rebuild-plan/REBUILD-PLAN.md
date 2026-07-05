# dowiz / DeliveryOS — Rebuild Plan (Synthesis)

- **Date:** 2026-07-04 · **Type:** architecture/quality master plan (docs-only; no code changed by this doc)
- **Method:** 5 parallel research lanes (Fable 5) — each grounded in codebase `file:line` + 2025–26 web sources:
  - `01-current-state-audit.md` — module map, hotspots, debt, must-preserve invariants
  - `02-backend-runtime-language.md` — runtime/language/framework/data-access/queue/libs
  - `03-frontend-build-bundle.md` — FE runtimes, build tooling, bundle/CWV, i18n, PWA
  - `04-data-infra-reliability.md` — pooling/RLS-GUC, queries, caching, queue runtime, WS, Docker/Fly
  - `05-dx-maintainability-quality.md` — task runner, tests, typecheck, pre-commit/CI, boundaries, harness
- **Authority note:** every 🔴 red-line item below (money / RLS / auth / WS authz / state-machine / migrations)
  is **council-gated** — Triadic Council to APPROVED before any code. Migrations additionally operator-gated.

---

## 1. Verdict — one paragraph

**Do not rewrite. Nothing in the stack is wrong.** All five lanes converged independently: the architecture
shape is sound (clean package edges, no dependency cycles, correct load-bearing invariants), and Node 22/
Fastify 5/raw-pg/pg-boss/React+Vite/Supabase/Fly are each within noise of any alternative *at this scale*
(one 512MB VM, ~0.1–1 job/s, tens of WS conns/room). Every "faster language / faster runtime / new
framework" option was evaluated and **rejected on measured cost-benefit** (see §6). The real quality gap is
concentrated and cheap to fix: **(a)** ~5 god-files and a triple-rendered storefront, **(b)** an asset diet
nobody ran (820 kB icon font, monolithic i18n chunk, no AVIF/srcset), **(c)** config-and-discipline debt in
the data layer (GUC-on-txn-pool leak class, order-create N+1, unbounded caches, worker on the wrong pool
port), **(d)** a dev loop that burns 5–6 min/commit on a Docker build while the 1,198-test unit suite runs
nowhere automatically, and **(e)** unfinished rollout of patterns that are already correct (`withTenant` in
24 files vs 174 raw `db.query` sites; 193 `request: any` handlers under a Zod type provider). The plan is
**targeted decomposition + completion + diet**, phased below.

**Expected outcomes (quantified by lane evidence):**
- Commit loop 5–6 min → **<30 s** (P0.1); no-op workspace ops 46 s → ~2 s (Turborepo); typecheck 18.6 s → ~2–3 s (tsgo at GA); prod FE build 8.6 s → ~3 s (Vite 8/Rolldown).
- Storefront payload: **−820 kB** icon font, −40–55 kB gz i18n, −70–85% card-image bytes, −15–40 kB gz motion → materially better LCP/TTI on the mobile-first surface that sells the product.
- Reliability: kills the cross-tenant GUC-leak class, the pool-starvation driver (~3 s order-create txn hold → ~10–30× shorter), the residual cache-OOM class, the SW staleness defect, and makes at-least-once job delivery safe.
- Maintainability: hotspot health 4.1 → ~7 (staged god-file splits + max-lines ratchet), one datastore fewer (drop Redis), 70→35 root scripts, boundaries machine-enforced, ~2,370 dead LOC removed.
- Zod 4: 6.5–14.7× parse on 57 boundary files, ~100× fewer tsc instantiations, −57% validator bundle.

---

## 2. Consolidated stack decision table

Merged across lanes; conflicts checked (none found — Lane 3's Vite 8/Rolldown ADOPT vs Lane 5's Rspack/
Turbopack REJECT are the same verdict: stay on the Vite line, take its Rust bundler).

### KEEP (evaluated, verdict: already right)
| Layer | Decision | Why (lane) |
|---|---|---|
| Runtime | **Node** (bump 22→24 LTS) | Bun/Deno ~3–15% real gain vs N-API breakage of sharp/argon2/tesseract (L2) |
| HTTP | **Fastify 5** | Hono/Elysia ~0% once Postgres is in the path (L2) |
| Data access | **raw `pg` + withTenant/SET LOCAL** (+SafeQL lint; optional Kysely pilot for new modules) | Preserves RLS/GUC control + hand-tuned SQL; type-safety via lint not runtime (L2) |
| Queue | **pg-boss** (fix runtime config, §P0.3) | 2–4 orders of magnitude below any ceiling; transactional enqueue is load-bearing (L2/L4) |
| Realtime | **raw ws + Pg LISTEN/NOTIFY** (+`maxPayload`/backpressure floor) | Multi-instance-correct, ADR-0013-hardened; Supabase Realtime = high authz-regression risk + 200-conn cap (L4) |
| FE framework | **React SPA + Preact bot-SSR, formalized** (shared DTO + SSR↔SPA contract test, delete dead vanilla bundles) | preact/compat broken w/ react-router v7 + framer-motion; rewrite gains ~40–50 kB for 35k-LOC cost (L3) |
| Test runner | **node:test** (consolidated, §P2.4) | Vitest migration = M–L effort for none-at-this-scale gain (L5) |
| Infra | **Fly.io + Docker + Supabase + R2** | Cost floor already reached (~$5/mo); Postgres-as-center is the right bet — strengthen it (L4) |
| JWT/hash/log | **jose / argon2 / pino** | µs-level noise between alternatives; argon2 deliberately slow (L2) |

### ADOPT (the actual changes — see phases for order)
| # | From → To | Effort | Gain | 🔴 |
|---|---|---|---|---|
| A1 | Docker build in pre-commit → **CI job w/ layer cache**; pre-commit = staged-lint + affected-typecheck | S | 5–6 min/commit → <30 s; retires disk-fill→PG-crash exposure | |
| A2 | Unit suite unenforced → **`test:unit` in CI** (staged patch already proven green) | S | 1,198 tests become a gate | |
| A3 | pg-boss worker on :6543 (broken LISTEN/NOTIFY) → **`DATABASE_URL_SESSION`** (1 line) + queue defaults + DLQ monitoring + durable outbox dedup | S→M | at-least-once made safe | |
| A4 | `set_config(...,false)` on txn-pooled clients → **`withTenant` txn-scoped GUC everywhere** + ESLint guardrail (completes the 24-vs-174 rollout) | S | kills cross-tenant GUC-leak + flaky-RLS class | 🔴 |
| A5 | Order-create ~60-insert N+1 → **2 multi-row `unnest` inserts** + pool-waiting alert | S–M | ~10–30× shorter txn hold on hottest write (the starvation driver) | 🔴 |
| A6 | 48 bare `current_setting()` RLS policies → **initplan `(SELECT …)` form**, hottest tables first | M | up to ~100× policy eval (Supabase-documented) | 🔴 op-gated |
| A7 | Icon webfont (820 kB, 165/5,900 used) → **tree-shaken SVG** | M | −820 kB + −35 kB gz CSS — top CWV win | |
| A8 | 1024px-WebP-only images → **srcset + AVIF** (sharp already wired) | S/M | −70–85% card payload, LCP | |
| A9 | Monolithic 3-locale i18n chunk → **build-time per-locale split** (catalog stays SSOT) | M | −40–55 kB gz always-loaded | |
| A10 | SW cache-first-forever shell → **network-first navigations** | S | fixes live staleness defect (sw.js biomarker) | |
| A11 | framer-motion in vendor → **LazyMotion/CSS** | M | −15–40 kB gz (checkout-path E2E proof required) | |
| A12 | Hand-rolled fetch/loading/error ×~10 → **TanStack Query v5, admin/courier only** (apiClient stays; 0 storefront bytes) | M | kills a duplication class | |
| A13 | `pnpm -r` → **Turborepo 2.7** (cache + `--affected`) | S | no-op 46 s → ~2 s; enables A1's affected-typecheck | |
| A14 | Zod 3 → **Zod 4** staged (+type-provider v6; defer v7) | M | 6.5–14.7× parse, ~100× fewer tsc instantiations | |
| A15 | Hand SQL unchecked → **SafeQL ESLint** vs live schema | S–M | compile-time SQL safety, zero runtime change | |
| A16 | No boundary enforcement → **dependency-cruiser (4 rules, CI)** | S | layering can't rot | |
| A17 | God-files → **staged seam-splits** (server.ts bootstrap/, orders.ts route-extraction, MenuManagerPage/MenuPage, spa-proxy) + max-lines ratchet | M×4 | hotspot health 4.1 → ~7 | 🔴 orders |
| A18 | Stage-numbered test sprawl → **fold 26 scripts into named node:test files** + one `test:integration` | M | 70→35 scripts, one runner | |
| A19 | Unbounded caches + TTL-only coherence + 4 tiny Redis uses → **cap Maps, bus-pushed invalidation, migrate Redis→Pg, drop ioredis** | S–M | closes OOM class; one datastore fewer | |
| A20 | No timeouts/shutdown/heap ceiling → **`requestTimeout` + 25 s shutdown race + `--max-old-space-size`** | S | graceful deploys, OOM→GC | |
| A21 | Advisory harness 19:1 noise → **hard-block red-lines only; cap lesson/nudge injection; archive 0-run loops** (deterministic arm untouched) | S–M | ~10× less context noise, zero red-line safety loss | op-gated (hooks) |
| A22 | Vite 6 → **Vite 8/Rolldown** (re-verify manualChunks hack) | S | FE build 8.6 s → ~3 s | |
| A23 | tsc → **tsgo `--noEmit` lane** at TS 7.0 GA (~Jul 2026) | S | typecheck 18.6 s → ~2–3 s | |
| A24 | Hygiene: **dead-code sweep** (2,370 LOC, dead authz exports, `three`, `pdfjs-dist`, 9 zombie dirs, 1.4 GB worktrees) + `--max-warnings` freeze + lhci → mobile preset + size-limit over ALL storefront chunks | S | ratchet with teeth; closes budget blind spot | |
| A25 | No slow-query surfacing → **`pg_stat_statements` + index-advisor ritual** (rejects OTel SDK — YAGNI) | S | free visibility | |
| A26 | 193 `request: any` handlers → **re-arm Zod type provider** incrementally (with A14/A17 route splits) | M | compiler re-armed at the boundary | |

### REJECT (evaluated and closed — do not re-litigate without new evidence)
Bun · Deno · Go/Rust/Zig hot-path rewrites (would split the 🔴 integer-money invariant across languages) ·
Hono · Elysia · Drizzle · graphile-worker/River/BullMQ · Preact-everywhere / React-SSR-everywhere ·
Solid/Svelte/Qwik/Astro rewrite (Astro: revisit only for future marketing pages) · Rspack/Turbopack ·
Vitest · Nx (supply-chain history) / moonrepo · TS project refs/isolatedDeclarations · Supabase Realtime ·
OTel SDK in API · Fly autoscale/CDN/distroless (cost floor reached) · offline order-POST queueing (🔴 money).

---

## 3. Phased plan

### Phase 0 — this week (all S, no council needed except noted)
| Step | Items | Proof |
|---|---|---|
| P0.1 | A1 Docker→CI + fast pre-commit (needs A13 Turborepo for `--affected`) | commit wall-clock <30 s measured; CI green w/ layer cache |
| P0.2 | A2 unit tests into CI (staged patch) | CI run shows 1,198 tests executed |
| P0.3 | A3 pg-boss `DATABASE_URL_SESSION` 1-liner (+defaults/DLQ follow-up in P1) | LISTEN/NOTIFY verified live on staging worker |
| P0.4 | A10 SW network-first + A24 budget gates (lhci mobile, all-chunk size-limit) | staleness repro red→green; gates fail on regression |
| P0.5 | A24 dead-code/dep sweep (safe deletes only) | typecheck+build+suite green post-sweep |
- ⚠️ P0.1/P0.2 touch `.claude/hooks` + CI config = **operator-gated paths** — prepare diffs, operator applies.

### Phase 1 — weeks 1–3 (reliability + asset diet)
| Step | Items | Gate |
|---|---|---|
| P1.1 | 🔴 A4 GUC-discipline rollout + ESLint guardrail | **council** (RLS red-line), red→green leak repro |
| P1.2 | 🔴 A5 order-create unnest batching | **council** (money path), fee-parity + order-composition suites + staging E2E |
| P1.3 | A7 SVG icons → A8 srcset/AVIF → A9 i18n split (in that order — largest wins first) | size-limit deltas + `/s/demo`→checkout E2E each |
| P1.4 | A19 cache caps/invalidation/drop-Redis; A20 timeouts/shutdown/heap | soak on staging; OOM repro retired |
| P1.5 | A16 dependency-cruiser; A15 SafeQL; A25 pg_stat ritual | CI rules live |

### Phase 2 — weeks 3–8 (structure + speed)
| Step | Items | Gate |
|---|---|---|
| P2.1 | 🔴 A17 god-file splits: server.ts → bootstrap/ (finish the half-done extraction) → spa-proxy (de-dup membership SQL ×4) → MenuManagerPage/MenuPage → **orders.ts last, council-gated**, behavior-frozen via refactor-converge loop | full-flow E2E stays green each split |
| P2.2 | A14 Zod 4 staged + A26 re-arm type provider on split routes | typecheck + boundary tests |
| P2.3 | A18 test consolidation (stage-scripts → named files) | one `pnpm test:unit` + one `test:integration` |
| P2.4 | A12 TanStack Query (admin/courier) + A11 LazyMotion + A22 Vite 8 | E2E + bundle gates |
| P2.5 | 🔴 A6 RLS initplan migrations, hottest tables first | **council + operator-gated migrations**, staging-first |

### Phase 3 — opportunistic / at-GA
A23 tsgo lane (TS 7.0 GA) · Node 24 base-image bump (can land any time; S) · Kysely pilot on one new
module · A21 harness tiering (operator applies hook diffs — this is the standing P2/P3 "fresh air" item) ·
Astro-for-marketing-pages re-evaluation only if a marketing site materializes.

---

## 4. Never-regress invariants (from Lane 1 — the rebuild's constitution)
🔴 Integer minor-unit money + server-authoritative pricing · 🔴 RLS FORCE + withTenant `set_config` · 🔴
RS256-only JWT + per-request live membership re-check (ADR-0004) · 🔴 157 forward-only migrations + boot
schema-guard · 🔴 WS tri-state authz at subscribe+fan-out (ADR-0013) · Zod parse-at-boundary + shared-types
contracts · error envelope (ADR-0010) · domain `assertTransition` (NOTE: worker `handlers.ts:15-40` raw-SQL
cancel *bypasses* it today — fix inside P2.1, council) · order idempotency · platform ports (Queue/Bus/
Storage) · default-off feature flags · guardrail/ledger ratchet + eslint-plugin-local · 174-spec staging E2E
net · fail-closed PII primitives. **Any plan step that would weaken one of these is void.**

## 5. Cross-lane convergences (why confidence is high)
- **Docker→CI**: independently P0 in L4 and L5, with the same measured 4–5 min cost and the same fix.
- **Triple-renderer storefront**: independently found by L1 (structure) and L3 (bundle) with matching dead-code lists.
- **GUC/pooling**: L1's "withTenant 24 vs raw 174" rollout gap = L4's `set_config(false)`-on-txn-pool P0 — same root, two lenses.
- **No-rewrite**: L1 (shape sound), L2 (stack right), L3 (no framework swap), L5 (no tooling swap) — four independent Fable lanes, one verdict.

## 6. Standing rejections — conditions to reopen
Each REJECT above may be reopened only on: >10× scale change (conns/jobs/req-rate), an N-API-free
dependency set (Bun/Deno), preact/compat gaining react-router-v7+framer-motion support (unification), or a
measured bottleneck that survives Phase 1–2 fixes. Record the trigger in this doc before re-researching.
