# Rebuild Cutover Harness — Design Proposal

- **Date:** 2026-07-04 · **Author:** System Architect (DeliveryOS rebuild program)
- **Scope:** the reversible per-surface switch/rollback mechanism for the Rust+Astro rebuild (S1..S10).
- **Grounding:** `REBUILD-MAP.md` §3 (Phase B strangler / cutover DoD; Phase D decommission), the S2/S3/S5
  council resolutions (cutover posture), the S1 OpenAPI SSOT, `06-complete-rebuild-stack.md` (parity oracle),
  memory `deploy-topology`. Reconciles the three already-decided per-surface postures into ONE mechanism.
- **Packet-status: 🟡 DRAFT — this is a 🔴 red-line design (it routes production traffic; a defect is a
  live outage or a duplicate paid order). No code. No flag flips. Operator sign-off gates every 🔴 below.**

---

## 1. Problem + non-goals

### Problem
Ten Rust surfaces (S1..S10) are being built **dark** in `rebuild/crates/api` (S1..S4 mounted, gated on
auth-env presence; see `main.rs:96-131`). **None has cut over.** The switch mechanism itself — how one
surface's production traffic routes to Node **or** Rust, reversibly, in isolation, without a redeploy — is
**unbuilt and untested**. This is the through-line that makes the rebuild's second half real: without a
proven, instantly-reversible per-surface switch, the operator's binding directive ("complete the rebuild
only after isolated switch/testing from S1 to S10, so each phase is reversible and safely switched") cannot
be honored, and the crown-jewel money cutover (S5) has no safe path.

### Goals (what this harness must nail)
1. A **runtime** (not build-time) per-surface switch: flip a surface to Rust and back in seconds, no redeploy.
2. **Reversibility with zero DB divergence** — both stacks share one Supabase DB; the DB is the reconciliation
   point. Routing-rollback is instant; the design proves no surface commits an effect a rollback can't paper over.
3. **Per-surface isolation** — S1..S10 flip independently; one on Rust while the rest stay Node.
4. A **parameterized cutover DoD** — one checklist, per-surface parameters, gating every flip.
5. A **deploy topology** where Rust is deployed dark (zero external traffic) until a flag flips.
6. The **S1-first proof** — the mechanism proves itself on the read-only surface before any risky one.
7. An **ordering** safe→risky, reconciled with the S2/S3/S5 council postures.

### Non-goals
- Not the surface *ports* themselves (each surface has/gets its own council packet; this is the switch only).
- Not the DB migration strategy (`sqlx::migrate` coexistence with frozen `pgmigrations` — REBUILD-MAP §2 — is
  assumed; this doc only constrains **when** a migration may run relative to a flip: §5, §9).
- Not the Astro frontend build (treated as an upstream target; the render sub-target is an open question, §5).
- Not Phase-D decommission mechanics beyond stating the harness is the *strangler vine* designed to be removed.

---

## 2. Back-of-envelope

**Purpose:** prove the switch mechanism is sized to the real load, and that the **connection budget** — not
request throughput — is the binding constraint during the two-stack overlap.

### Traffic (estimates — replaced by Phase-A measurements; labelled as such)
- **Locations:** ~1–50 today (MVP / early sales), model a 1-year 10× to ~500.
- **Orders:** peak ~10% of locations active concurrently × ~2 orders/min ≈ **~10 orders/min ≈ 0.17 orders/s**
  today; ~1–2 orders/s at 10× growth.
- **Reads (dominant):** storefront is read-heavy — each session ~5–15 menu/info/theme GETs; ~100 concurrent
  peak sessions ⇒ **~5–10 req/s** today; ~50–100 req/s at 10×. Media/images are R2/CDN-served (off the origin).
- **Verdict:** firmly **monolith / single-front-door** territory (ADR-0001 monolith-first; the Prime Video
  re-consolidation case). This load does **not** justify an edge-gateway's extra hop/component (§3 option b).
  A Node front-door reverse-proxy is comfortably over-provisioned for it.

### Connection budget (the actual ceiling)
Both stacks draw pooled Postgres connections from **one** Supabase/Supavisor during the overlap. This — not
CPU or req/s — is what a two-stack window can blow. Budget it as a sum that must never exceed the pooler ceiling:

```
overlap_conns = node_api + node_worker + node_analytics + node_migrations(transient)
              + rust_api(dark→flipped) + rust_migrations(transient, release only)
```

| Pool | Node (today) | Rust (dark) | Rust (post-flip) | Note |
|---|---|---|---|---|
| api / operational | ~10–15 | ~5–8 (small) | absorbs the shed Node draw, per surface | the flip *sheds* Node's draw for that surface |
| worker | ~5 | 0 (S8 not flipped) | ~5 at S8 | jobs stay Node until S8 |
| analytics | ~2–3 transient | 0 | — | owner analytics reads |
| migrations | transient, release-only | transient, release-only | — | **never concurrent — schema frozen during cutover, §5/§9** |

**Governing input:** the **Supavisor decision (cache-off vs all-:5432)** is the Phase-A spike exit question
(REBUILD-MAP §2). The harness rule: **Rust dark-phase client pools ≤ the Node draw they will later shed**, so
the *sum* never grows past the ceiling; the atomic flip (§4) is the contract that hands one surface's draw
from Node to Rust rather than adding it. The safe→risky ordering (§7) sheds Node's draw incrementally, so the
peak two-stack budget is at the *start* (S1 flip, both stacks full) — S1 is read-only, so its draw is trivial
and cache-absorbable. **The overlap must be time-boxed** (§9) so the doubled steady-state pool footprint is bounded.

---

## 3. Options (≥2) + trade-offs

### Concept vocabulary
Strangler Fig (Fowler) · runtime feature flag · reverse proxy / single ingress · API gateway · circuit
breaker (health-gated degrade) · the shared DB as the reconciliation point (CAP: one datastore, so no
cross-stack consistency problem to solve — only cross-stack *write-byte-identity*).

### Option A — Node front-door reverse-proxy + Postgres-backed runtime flag  ✅ CHOSEN
Node keeps the **single public ingress** (`dowiz.fly.dev`). A thin front-door hook checks a per-surface
**runtime flag** (a `cutover_flags` row); when a surface is `rust`, the matched `(method, path)` is streamed
to an **internal-only** Rust Fly app over Fly private networking; when `node`, the existing Node handler runs
unchanged. The flag lives in Postgres, cached in-process with a short TTL + `LISTEN/NOTIFY` invalidation.
- **Concept:** Strangler Fig with the incumbent as the front-door; runtime flag in the shared datastore.
- **Pros:** one ingress (no DNS/cert/CORS change, no client change); reuses the component that already owns
  rate-limit, correlation-id, real-client-IP (`clientIp`), and auth pre-checks; flag is runtime → instant
  flip/rollback via one `UPDATE`+`NOTIFY`, no redeploy; Rust deploys dark with **no public route** (zero
  external traffic until a flip); **adds no new SPOF** (Node is *already* the sole ingress); the flag is
  durable + auditable (who flipped, when). At Phase D the front-door role migrates to Rust and the shim is
  removed — it is the vine, built to be cut.
- **Cons:** Node stays in the request path for flipped surfaces until Phase D (one extra intra-region hop,
  ~sub-ms on Fly 6PN; a Node dependency while "Rust is live"). The flag store must be fast + shared (solved by
  TTL+NOTIFY). WS (S6) needs an HTTP-upgrade proxy path, distinct from the request/response path (§5, threat T-WS).

### Option B — Edge/gateway router in front of both (new component: Caddy/HAProxy/Fly-native)
A third component routes per-path to Node or Rust; neither incumbent sits in the other's path.
- **Concept:** API gateway.
- **Pros:** clean equal-backends separation; the gateway can be the permanent front-door after Node dies.
- **Cons:** a **new** component to build/operate/secure — its own SPOF, deploy, health, flag store, and attack
  surface, **before S1 even flips**. Fly-native path routing is coarse (per-app, not per-`(method,path)`
  runtime flag); a config-reload flip is slower than a DB flag unless the gateway *also* reads a live flag
  store (reinventing option A's flag inside a heavier box). Adds a hop for **all** traffic from day one. The
  back-of-envelope load does not warrant it. **Rejected: over-engineering against a modest load; violates
  boring-&-proven / minimum-that-holds.**

### Option C — DNS / per-surface hostname split
- **Rejected:** DNS TTL makes rollback non-instant (the opposite of the directive); not per-`(method,path)`;
  not a runtime flag.

### Option D — build-time `VITE_*` / client flag
- **Rejected by the brief and by first principles:** not runtime; needs a redeploy to flip/rollback; a client
  flag can't route server surfaces (S2/S5 are server-side).

**Decision: Option A.**

---

## 4. Decision + rationale (ADR-format — also `docs/adr/ADR-rebuild-cutover-harness.md`)

**ADR-0022 (provisional) — Reversible per-surface cutover via a Node front-door reverse-proxy gated on a
Postgres runtime flag.**

**Decision.** The Node/Fastify server remains the single public ingress. A front-door routing hook
(`onRequest`) resolves each request's owning surface from a **provably-disjoint `(method, path)` ownership
map** (§ path map below), reads that surface's target from an in-process cache of the `cutover_flags` table,
and:
- `target = node` (default) → falls through to the existing Node handler (today's behavior, unchanged);
- `target = rust` → streams the request to the internal Rust upstream (`dowiz-rust.flycast`) via `undici`
  (the runtime's existing HTTP client — no new dependency class), preserving `Authorization`, the
  server-authoritative correlation-id, and the real client IP (`X-Forwarded-For`), and pipes the response back.

The flag table:
```
cutover_flags(
  surface       text primary key,            -- 's1'..'s10'
  target        text not null default 'node' check (target in ('node','rust')),
  readiness_ok  boolean not null default false, -- set true only when the surface's DoD (§4 gates) is recorded green
  updated_at    timestamptz not null default now(),
  updated_by    text not null                  -- audit: which operator flipped it
);
```
A flip is one statement (`UPDATE cutover_flags SET target='rust', updated_by=$1 WHERE surface='s3'`) followed
by `NOTIFY cutover_flags_changed`. Every Node instance `LISTEN`s and refreshes its cache on notify; a short
TTL (1–5s) is the backstop if a notify is missed. **Rollback is the inverse statement** — instant, no redeploy.

**Flip precondition (scaling-gate).** The flip command refuses unless: (i) `readiness_ok = true` for that
surface (its DoD is recorded green), **and** (ii) the Rust upstream `/healthz` is green, **and** (iii) an
operator sign-off token is present. An unready or unhealthy surface cannot be flipped.

**Break-glass.** A front-door env `CUTOVER_FORCE_ALL_NODE=1` overrides the table and forces every surface to
Node regardless of the flag rows — the defense for "the flag store read is itself impaired" (threat T6).

**Rationale.** Boring & proven: it reuses the incumbent ingress, the shared datastore for state, and the
runtime's own HTTP client. "Schema rich, runtime minimal": the ownership map + flag table are the seams; the
proxy runtime engages only when a flag is `rust`. The DB is the single reconciliation point, so there is no
distributed-consistency problem — only a write-byte-identity problem (which the DoD gates enforce, §4/§6).
Failure-first: a Rust outage degrades a surface to Node automatically (circuit breaker below), never a cascade.

**Consequences.** Node cannot be decommissioned until every surface has flipped and the front-door role
migrates to Rust (Phase D). The two-stack overlap doubles the steady-state pool footprint (bounded by §2 +
the time-box). The front-door forward path is a mild SSRF-adjacent surface, mitigated by a hardcoded internal
upstream (never client-derived) and by applying auth/rate-limit at the front-door **before** forwarding.

### Path-ownership map (the partition — a total function `(method, path) → surface`)
Ownership is by **`(method, prefix)`**, longest-prefix wins; an unmapped route defaults to Node (fail-safe to
the incumbent). Selected rows (full map lives in `traceability.csv`, extended with a `surface_owner` column):

| Surface | Owns `(method, prefix)` — representative | Notes |
|---|---|---|
| **S1** storefront-read | `GET /public/locations/*`, `GET /api/public/theme/*`, `GET /s/{slug}` + shell subpaths, `GET /images/*`, `GET /media/*`, `GET /api/public/locations/*/fallback-config`, `GET /api/public/voice-config`, `GET /api/push/vapid-public-key`, `GET /v1/rates`, `GET /robots.txt`, `GET /sitemap*` | all read; `GET /s/:slug/checkout` (shell) is S1, but `POST /orders` is S5 — method-disambiguated |
| **S2** auth | `POST /api/auth/*`, `POST /api/customer/otp/*`, `POST /api/customer/track/*`, `/dev/mock-auth` (dark) | token mint/refresh; **verification** parity is a prerequisite for S3/S4/S5 flips (§7) |
| **S3** catalog CRUD | `GET|POST|PUT|PATCH|DELETE /api/owner/menu/*`, `/api/owner/brand`, `/api/owner/settings`, `/api/owner/categories/*` | owner writes; **menu-import stays Node** (S3 REV-7 two-writer) |
| **S4** media | `POST /api/public/entry-photo`, `POST /api/owner/menu/products/*/image`, media upload-token PUT | object writes, mostly idempotent by content-hash |
| **S5** orders/money 🔴 | `POST /orders`, `GET|PATCH /api/owner/orders/*`, `GET|POST /api/customer/orders/*`, `/deliver` | crown jewel; whole family flips atomically (S5 Q6) |
| **S6** WS 🔴 | `GET /ws` (HTTP upgrade) | distinct upgrade-proxy path; a flip drops+reconnects |
| **S7** courier/dispatch 🔴 | `/api/courier/*`, dispatch routes | ADR-0013 binding-scope authz |
| **S8** jobs/notifications | worker process + `/api/*/notifications` | not request-path; flips the worker process |
| **S9** GDPR 🔴 | `/api/owner/gdpr/*`, export/erase | DEFINER fns stay in PG (REBUILD-MAP §8) |
| **S10** platform-admin | `/api/admin/*`, provisioning, backup triggers | last |

**Phase-0 gate:** a CI extractor proves the map is a **partition** — every one of the 236 census routes maps
to exactly one surface; no `(method, path)` matches two surface rules (prevents split-brain by mis-routing,
threat T1). This is the map-coverage gate (REBUILD-MAP §5) extended to the router.

---

## 5. Data / migrations (forward-only, atomic, RLS FORCE, integer)

- **No schema change is introduced by the harness itself** beyond the `cutover_flags` table above — a
  platform-ops table, **not tenant-scoped** (it is global operator state), so it takes `RLS ENABLE + FORCE`
  with a policy admitting only the platform-admin role (belt: it holds no tenant data; suspenders: FORCE by
  the standing rule on every table). Forward-only, atomic, single migration.
- **The DB is the reconciliation point.** Both stacks read/write the same 86 tables. Money stays integer
  minor units, half-up, server-authoritative (ADR-0005) on **both** stacks — the S5 byte-parity gate (§6)
  makes that provable, not asserted.
- **Migration/flip ordering rule (hard):** **no schema migration runs inside a surface's cutover window.**
  The DB is frozen during the rebuild (REBUILD-MAP rule); the only migrations in the whole program are the
  operator-gated money drafts (085/086/087), and each lands **before** its surface's flip on the incumbent
  Node migrator (086 before the S5 flip — S5 Q6/Q7). Rust's `sqlx::migrate` (`_sqlx_migrations`) authors
  **nothing** during cutover; it only coexists (REBUILD-MAP §2). This prevents the two migrators racing one DB
  (threat T9): they are never both introducing a change in the same window.
- **Astro render sub-target (open):** S1's `GET /s/{slug}` (bot → SSR HTML; human → SPA shell) becomes an
  Astro SSR route in the rebuild. The harness treats "the new-stack upstream" as **one logical target per
  surface**; whether the Astro renderer is the same Fly app as `dowiz-rust` or a second internal app is a
  deploy detail (open-questions Q6). The path map assigns each path to exactly one upstream regardless.

---

## 6. Consistency + idempotency

Because both stacks share one DB, there is **no cross-stack consistency problem to solve** (no second
datastore, no replication lag between stacks). The only cross-stack hazard is **two stacks writing different
bytes for the same logical write**. The harness's consistency guarantee therefore reduces to enforced
**write-byte-identity + shared-constraint guards**, proven per surface before its flip:

- **S1 (read-only):** trivially consistent — GETs are idempotent; a mid-flip caller may get one response from
  Node and the next from Rust, byte-identical by the parity oracle. The only per-stack state is the 30s
  in-process menu cache; a flip just re-warms the other stack's cache. No DB write, no divergence.
- **S3 (owner writes, single-writer):** same rows, same DB. Hazards are the **two-writer window** (menu-import
  stays Node while Rust owns catalog writes — REV-7) and **S1 read-after-write cache staleness**. Both are
  cutover-DoD items (invalidation story + replace-mode guard).
- **S5 (orders/money 🔴):** the sharpest. Idempotency lives in **Postgres** (`idempotency_keys (key,
  location_id)` + a `request_hash` compare) — never Redis. The **only** guard against a cross-stack retry
  producing a duplicate paid order is the shared unique constraint, effective **iff** the `request_hash` is
  **byte-identical** across stacks. So the S5 flip gate includes a **request-hash golden-vector, both
  directions** (Node-hash verifies on Rust and vice-versa), money byte-parity vectors (`i128` intermediate,
  half-up, `chargedTax` not `taxTotal`), and a cross-stack idempotency probe (create on X, retry on Y → one
  order). Crypto stays dark through the overlap (no live charge at create).
- **S2 (auth):** cross-stack concurrent-refresh is safe **iff** both stacks run identical refresh SQL incl.
  the `interval '5 seconds'` window (the SQL is authority over the stale "10s" comment) against the shared
  `auth_refresh_tokens` row — an atomic UPDATE picks one winner, 409s the other. Token verification is
  stateless (RS256 public key, shared) with the body-`kid` round-trip proven both directions.

### Per-surface cutover DoD (one parameterized checklist)
**Base gates (every surface, before `readiness_ok = true`):**
- **G1 E2E parity slice green** — the surface's Playwright specs run against the Rust-served paths (flag ON in
  staging), all green (the language-independent parity oracle).
- **G2 openapi-diff empty** — the surface's utoipa OpenAPI == the frozen SSOT contract (additive-only).
- **G3 invariant-cluster red→green** — the surface's Rust unit cluster passes, each proven red first.
- **G4 map-coverage zero-diff** for the surface's namespaces (no UNMAPPED/UNBUILT/ORPHAN).
- **G5 a11y + size budgets** (FE surfaces).
- **G6 rollback drill proven** — scripted flip ON→OFF in staging returns traffic to Node with zero errors,
  within the flip-latency SLO (not a claim; a run).
- **G7 upstream health-gate green** — Rust `/healthz` green (the flip precondition is machine-checked).
- **G8 shadow-diff (read surfaces)** — a sample of live GETs mirrored to both stacks, 0 byte-diffs over the window.

**Per-surface addendum:**
| Surface | Extra gates |
|---|---|
| S1 | G8 is the strongest gate; no write gates |
| S2 | body-`kid` round-trip both directions · hash-format parity · courier `has_location` revocation E2E · customer-scope 403 E2E (REV-3) · cross-stack concurrent-refresh proof · **family-revoke-rate baseline captured + post-flip auto-rollback trip-wire** (§7 R-1) |
| S3 | read-after-write cache-invalidation gate · two-writer (menu-import Node) regression pins · NOBYPASSRLS GUC-seating probe (REV-5) |
| S4 | content-hash idempotency of image writes · token-proxy PUT auth · media-worker health |
| S5 🔴 | request-hash byte-identity golden-vector (both directions) · money byte-parity vectors · **086 landed before flip** · cross-stack idempotency probe · **crypto stays dark** · NOBYPASSRLS order-create probe · 085 watermark timing check |
| S6 🔴 | WS fan-out authz parity · reconnect-continuity across a flip · upgrade-proxy path proven |
| S7/S9 🔴 | B3 post-flip RLS probe · binding-scope authz (S7) · DEFINER fns called-not-reimplemented (S9) |

---

## 7. Failures + degradation (every external call: timeout + fallback, zero cascade)

Failure-first: the degradation path is designed before the happy path.

- **Rust upstream down (health-gate, circuit breaker).** The front-door polls Rust `/healthz`. If a surface is
  `rust` but the upstream is failing health, the front-door **degrades that surface to Node** automatically and
  alerts — a Rust outage cannot take a flipped surface down. The forward call has a **per-request timeout**
  (matched to Rust's own 30s tower timeout, tighter at the front-door, e.g. 10s) + a bounded retry-once on a
  connection error, then fall back to Node for that request if the surface is read-only, or fail with the
  surface's typed envelope if it is a non-idempotent write (never silently retry a money write cross-stack).
- **Rust upstream up but erroring (divergence).** The observability layer (below) compares per-stack error
  rates. A configured **trip-wire** (S2 family-revoke-rate; S5 duplicate-order rate) auto-flips the surface
  back to Node when the metric exceeds the Node baseline + ε within the observation window.
- **Flag-store impaired.** Fail-safe to the last-known-good cached map; if there is no cache and the DB is
  unreachable, default every surface to Node (the incumbent); the `CUTOVER_FORCE_ALL_NODE` break-glass is the
  operator override.
- **Zero cascade:** the front-door applies rate-limit + auth **before** forwarding, so a Rust surface cannot be
  used to bypass the incumbent's throttles; a slow Rust upstream is bounded by the front-door timeout and does
  not exhaust Node's event loop (streaming, not buffering; bounded undici pool).
- **WS (S6):** a flip drops live WS connections; clients auto-reconnect and the target stack re-authenticates
  the same token — **continuity via reconnect, not connection-migration**. The front-door upgrade-proxy honors
  the flag at upgrade time only (an already-upgraded socket stays where it upgraded until it reconnects).

---

## 8. Security + tenant isolation

- **JWT RS256 verify is stateless + shared** — a token minted by either stack verifies on the other via the
  shared public key (the S2 body-`kid` round-trip gate makes this provable both directions). **Zero cookies**
  is preserved end-to-end; the forward carries `Authorization`, never a Set-Cookie.
- **Tenant isolation survives the hop** — RLS `ENABLE + FORCE` on every tenant table is unchanged; both stacks
  seat the tenancy GUC per surface family (owner `with_user(app.user_id)` — S3 REV-10; order-write
  `with_tenant(app.current_tenant=locationId)` — S5 Q3). The B3 NOBYPASSRLS flip is **orthogonal** to and
  independently reversible from the Node→Rust flip (S5 threat-model §4) — the harness never couples them.
- **The forward is not an SSRF vector** — the upstream host is hardcoded (`dowiz-rust.flycast`), never derived
  from the request; only mapped surface paths forward; traversal/`..` guards on `/images|/media/*` are carried
  verbatim in the Rust port.
- **Real client IP preserved** (`X-Forwarded-For` = `clientIp`, the #9 fix) so S5's velocity throttles key on
  the customer, not the Fly edge socket. A dropped XFF is a named test vector (threat T8).
- **Zero PII to AI, zero secrets in git** — unaffected; the harness moves no data to any AI path and adds no secret.
- **`cutover_flags` is operator-only** — RLS FORCE + platform-admin policy; the flip is audited (`updated_by`).

---

## 9. Operability

- **Health: degraded vs down.** Front-door distinguishes *upstream-down* (→ degrade surface to Node + alert)
  from *upstream-up-but-erroring* (→ trip-wire / alert). Rust exposes `/livez` (cheap liveness, Fly internal
  check) + `/healthz` (readiness — DB pools). Node's `/livez` stays the Fly public liveness (unchanged).
- **Observability < 1 min.** The front-door emits, **per surface**: routed target, req/s, error-rate, p99,
  and the surface-specific trip metric (S2 family-revoke-rate; S5 duplicate-order-rate). A dashboard shows
  per-surface {target, error-rate divergence Node-vs-Rust}; an alert fires < 1 min on divergence. The
  server-authoritative correlation-id propagates through the hop so one request is traceable across stacks.
- **Rollback.** One `UPDATE cutover_flags ... target='node'` + `NOTIFY` — sub-second via notify, ≤ TTL worst
  case; plus the `CUTOVER_FORCE_ALL_NODE` break-glass if the flag path itself is impaired. No redeploy, ever.
- **Flag / scaling-gate.** A flip to `rust` is refused unless `readiness_ok=true` + upstream health green +
  operator token (§4). This is the machine gate that stops flipping an unready surface.
- **Overlap time-box.** Each surface's two-stack overlap is time-boxed (operator-set, e.g. days not weeks) so
  the doubled pool footprint (§2) is bounded and the surface either commits (Node draw shed) or rolls back.

### Deploy topology change
- **Rust runs as a second Fly app** — `dowiz-rust` (prod) / `dowiz-rust-staging` (staging), region `fra`, with
  **no public `[http_service]`** route (internal-only service on Fly 6PN; reachable only at `.flycast`/
  `.internal`). It receives **zero external traffic**; only the Node front-door forwards to it, only for `rust`
  surfaces. **Why a second app, not a co-process:** independent deploy/rollback/scale/health, clean blast
  radius; the scratch Rust image is ~15–25 MB, so a small `shared-cpu-1x` 256–512 MB machine in `fra` is cheap
  (~ the existing worker VM). **Cost:** one extra tiny Fly app; the real cost is the overlap connection budget
  (§2), controlled by small dark-phase pools + the time-box.
- **Health-check:** Fly internal check on Rust `/livez`; the front-door additionally gates flips on `/healthz`.
- **Rollback (deploy-level):** redeploy/scale Rust independently of Node; a bad Rust deploy is caught by its own
  health check (Fly aborts the rollout, old Rust keeps serving) and, if a surface is live, the front-door's
  health-gate degrades it to Node in the meantime.
- **`release_command`:** Rust's `sqlx::migrate` runs as its own release step but authors nothing during cutover
  (§5) — the schema is frozen; the 157 Node migrations stay applied and frozen.

---

## 10. Open / accepted risks (owner + justification)

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R-1 | **S2 posture revision** — the council said "canary gated on family-revoke-rate, not a hard switch"; this harness offers **atomic-flip + a family-revoke-rate auto-rollback trip-wire** instead of a per-request split. Argued *safer* for auth (a per-request split would send concurrent refreshes of the SAME family to different stacks — the exact cross-stack concurrent-refresh hazard). But it **revises a council decision**, so it is not adopted silently. | **OPEN — operator 🔴 sign-off** (open-questions Q3) | architect + operator |
| R-2 | **Cross-stack token-verification parity is a prerequisite** for flipping S3/S4/S5 ahead of S2-mint: an owner logs in on Node (Node-minted token) then hits a Rust owner surface which must verify it. The body-`kid` round-trip + hash-format parity gates therefore gate S3, not just S2. | **OPEN — operator 🔴** (Q4); accept only with the parity gate green | architect + S2 lead + operator |
| R-3 | **Cross-stack duplicate paid order (S5)** — the money-irreversible failure the atomic-flip exists to prevent; bounded by the shared `idempotency_keys` unique **iff** request-hash is byte-identical. | **ACCEPTED-RISK, gated** — flip forbidden until the request-hash golden-vector + cross-stack idempotency probe are green and 086 has landed; crypto dark throughout | operator + S5 lead + breaker |
| R-4 | **`discountTotal=0` carry (S5)** — re-shipping a known money gap through the rewrite. | **ACCEPTED-RISK** (unbuilt feature, not a defect) — explicit, owned, not silent | operator + S5 lead |
| R-5 | **Front-door is the sole ingress** — but Node *already* is; the shim adds no new SPOF, only new forward-path code. | **ACCEPTED** — mitigated by health-gate degrade + break-glass; SSRF-guarded | architect |
| R-6 | **Overlap doubles the pool footprint** — bounded by §2 budget + the time-box; peak is at S1 (read-only, trivial draw). | **ACCEPTED**, gated on the Phase-A Supavisor decision | operator |
| R-7 | **085 settlement watermark (2026-07-10)** — a timing landmine independent of S5 order flow; an apply slipping past the literal DOUBLE-PAYS old rows. | **ACCEPTED-RISK, operator-owned** — surfaced so the cutover schedule can't silently trip it | operator |
| R-8 | **Astro render sub-target** — whether SSR is the same app as `dowiz-rust`. | **OPEN — deploy detail** (Q6); does not block the mechanism design | architect |

---

council seats: breaker, counsel

**packet-status: 🟡 DRAFT** (this is 🔴 — it routes production traffic; no code, no flip, until the
open-questions 🔴 items are operator-signed and the breaker + counsel round is folded).
