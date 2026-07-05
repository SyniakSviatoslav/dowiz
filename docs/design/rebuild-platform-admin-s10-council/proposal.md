# S10-PLATFORM-ADMIN / PROVISIONING Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). **No S10 code is ported to Rust until this
> packet is council-APPROVED, every quirk-register row (§11) is dispositioned one by one, and the
> operator signs the 🔴 open questions (`open-questions.md`).** This is the **HIGHEST-PRIVILEGE PLANE**
> (platform-admin sits *above* owner) **and the LAST strangler surface** — its flip is the trigger for
> the Phase-D decommission (cutover-harness REV-C10). Docs only; no product code.

- **Lane:** R3 (complete-rebuild) · **Surface:** S10 platform-admin (REBUILD-MAP §3 Phase B, **10th and
  final** strangler — `… → S8 jobs → S9 GDPR → S10 platform-admin`, safe→risky ordering, cutover-harness
  §7 / open-questions Q3). S10 is deliberately last: it is cross-tenant by design, it holds the DR
  triggers, and flipping it means every other surface has already flipped.
- **Date:** 2026-07-05 · **Source commit:** `fix/audit-remediation` (working tree).
- **Census SSOT:** `route-surface-map.generated.md` rows **60–64, 102–104, 180, 193–195, 214–228** — the
  **27 S10 routes** (enumerated in §2). Backing source: `routes/admin/*` (platform-ops),
  `modules/acquisition/*` + `routes/public/claim.ts` (acquisition/provisioning/claim),
  `routes/owner/onboarding.ts` + `activation.ts` (tenant-lifecycle), `workers/backup/*` (the DR-drill the
  admin routes trigger), and the authority in `lib/platform-admin.ts`.
- **Governing ADRs / prior councils (this surface inherits hard-won invariants — do not re-litigate):**
  - **ADR-admin-platform-authz (B4)** (`docs/adr/ADR-admin-platform-authz.md`, ACCEPTED 2026-06-29) —
    the platform-admin principal is a **server-side allowlist fact, NOT a JWT role**; the `/api/admin/*`
    plane gate; DR-drill hardening; write-ahead audit; **Alternative A (4th JWT role) was REJECTED**.
    **S10 REUSES this verdict verbatim; it is not re-opened.**
  - **P6-1/P6-2/P6-3 provisioning council** (`p6-2-provisioning-council-verdict.md`) + **P6 claim
    council** (`p6-claim-council-verdict.md`) — the shadow-spine `provision_shadow` RLS policy (mig 069),
    the single-use provisioning/claim tokens, the `claim_transfer` DEFINER ownership transfer (mig 071),
    born-with-erasure `hard-delete`, the Art-14 hostile-recipient notice. **P6 flagged ownership transfer
    "needs work"** (memory `p6-provisioning-vertical`) — dispositioned in §4/Q2.
  - **Backup/DR + safe-reversal** (`workers/backup/*`, LC7 restore-verify fixes) — the restore **DRILL**
    (to a sandbox), the R2 manifest as the integrity SoT, fail-loud-on-unknown-keyId. **The real
    restore-to-prod is a manual DR runbook, not code** — the single largest rebuild "gap" (§5, Q3).
  - **secrets-exposure-incident** (memory, 2026-07-03) — prod creds once lived in git; the **backup
    encryption key must never be logged or committed**; every drill line is PII/secret-redacted.
  - **S2-auth RESOLVE** (`rebuild-auth-s2-council/`) — the Rust `Claims` discriminated union is **3-role
    (owner/courier/customer)**; `unknown_role_is_rejected` already forbids a 4th. The admin gate keys on
    `OwnerClaims.user_id`; there is no `PlatformAdminClaims` and there must never be one (Q1).
  - **S3-catalog REV-7** (per-surface **atomic** cutover; two-writer routes that stay Node) — inherited
    for the flat `/api/owner/onboarding` two-writer (§2, row 180) and the drill carve-out (§5).
  - **cutover-harness REV-C10** (`rebuild-cutover-harness/resolution.md` §REV-C10) — the un-cut-vine:
    the front-door shim needs a **dated Phase-D cut-trigger + named owner**. S10's flip is that trigger;
    naming the owner is this packet's closing item (§9, Q5).
- **Parity oracle:** the B4 admin-authz test net (owner→403 ×6 / platform-admin→200 ×6 / courier·customer
  →401·403 / drill 429+409 / write-ahead audit row / **structural sibling-closure**: a throwaway ungated
  `/api/admin` sibling → 403) **plus** the P6 provisioning/claim E2E (acquisition→live, provision_shadow
  RLS, claim_transfer, decline-and-erase) **plus** the restore-verify drill proof (sandbox target,
  checksum-of-plaintext, fail-loud keyId). No behavior change is real without a red→green test (Mandatory
  Proof Rule). Cutover DoD in §12.

---

## 1. Port objective and the load-bearing seam

S1 was read-only; S2 wrote auth; S3–S9 wrote tenant data under a single owner/courier/customer principal.
**S10 is the one surface whose principal sits ABOVE the tenant plane** — a platform-admin who reads and
acts *across every tenant*, who can trigger a fleet DR drill, and who provisions net-new tenants from
scratch. A port defect here is not a wrong charge or a stuck order; it is a **privilege-escalation**
(an owner becoming an admin), a **cross-tenant data breach** (an admin read leaking one tenant into
another), or a **destructive-operation misfire** (a restore to the wrong target, a tenant provisioned
into another's data).

**The load-bearing seam is that S10 is NOT one privilege plane — it is FOUR, sharing one surface label**
(the S10 analogue of S5's "money is not one atomic surface"). Enumerated in §2, the 27 routes split into
four planes with **four different authentication mechanisms and four different cutover DoD gates**:

1. **Platform-ops plane** (`/api/admin/*`, 6 routes) — authority = the **`platform_admins` allowlist**
   re-read per request, keyed on `OwnerClaims.user_id`; cross-tenant by design; holds the DR triggers.
2. **Acquisition-ops plane** (`/internal/acquisition/*`, 9 routes) — authority = the **`PROVISION_OPS_SECRET`
   header** (timing-safe, fail-closed-404), a *different* gate, deliberately decoupled from both the
   platform-admin allowlist AND the dev-login family (B4); cross-tenant provisioning.
3. **Owner-tenant-lifecycle plane** (`/api/owner/onboarding/*` + `/api/owner/activation/*`, 8 routes +
   1 two-writer) — authority = the **owner JWT** (the ordinary tenant plane, `OwnerClaims`), single-tenant
   draft→live gating. **Not privileged above owner at all** — it rides the S3-style owner surface.
4. **Public-claim plane** (`/api/claim/*`, 3 routes) — authority = a **single-use opaque claim token**
   (no session), the ownership-transfer front door.

A single "flip S10 atomically" mental model (REV-7) is **strictly harder here than for any prior
surface**, because atomic means all four auth mechanisms and their DoD gates must be green together, and
because **one S10 route provably never flips** (row 180, the flat `/api/owner/onboarding` two-writer on
`products`/`location_themes`, REV-7). The packet's central recommendation (§9) is that S10 flips as **the
gate + the thin triggers**, while **two heavy sub-capabilities stay permanently on Node** by design (the
subprocess DR-drill orchestration, §5; the row-180 two-writer) — "schema-rich, runtime-minimal": the
Rust surface owns the *authority*, not the 400-line `pg_restore` stream pipeline.

## 2. Scope — the 27 S10 routes, split by plane

**Enumerated from `route-surface-map.generated.md` (the machine-derived partition, 236/236 proven
disjoint). Every row cited by its map `#`.**

**Plane A — platform-ops (`/api/admin/*`), 6 routes, `platform_admins` gate:**
- `#214 GET /api/admin/backups` (`admin/backups.ts:13`) — list recent backups + restore-test results.
- `#215 POST /api/admin/backups/verify` (`admin/backups.ts:73`) — trigger a manual restore **DRILL**
  (weaponizable; rate-limit + single-flight + uuid-validated `backupId` + write-ahead audit).
- `#216 GET /api/admin/backups/dr-report` (`admin/backups.ts:100`) — fleet DR-drill report (`fullHash`).
- `#217 GET /api/admin/fallback/health` (`admin/fallback.ts:13`) — **cross-tenant** SELECT over ALL
  `locations` + notification-target health (incl. public phones).
- `#218 POST /api/admin/fallback/r2-check` (`admin/fallback.ts:47`) — cross-tenant fallback-coverage %.
- `#219 GET /api/admin/notification-audit` (`admin/notification-audit.ts:17`) — PII-free event/status/
  channel/count rollup (no targets, no addresses).

**Plane B — acquisition-ops (`/internal/acquisition/*`), 9 routes, `PROVISION_OPS_SECRET` gate:**
- `#220 POST /internal/acquisition` (`acquisition/route.ts:62`) — idempotent create-source from a place_id.
- `#221 POST /internal/acquisition/extract` (`:77`) — SSRF-guarded menu locate → AI-parse (PII-redacted).
- `#222 POST /internal/acquisition/provision/mint` (`:90`) — mint a single-use provisioning token.
- `#223 POST /internal/acquisition/provision/spine` (`:107`) — write the shadow spine THROUGH the
  `provision_shadow` RLS policy (one tx, consume-LAST).
- `#224 POST /internal/acquisition/provision/hard-delete` (`:130`) — born-with-erasure day-one delete.
- `#225 POST /internal/acquisition/claim/verify` (`:142`) — PROVISIONED→VERIFIED (preview renders).
- `#226 POST /internal/acquisition/claim/mint` (`:159`) — mint a single-use claim invite + Art-14 notice.
- `#227 POST /internal/acquisition/complaint` (`:186`) — record a C&D (structured-log health signal).
- `#228 POST /internal/acquisition/retention/sweep` (`:199`) — GDPR Art-5(e) reaper (grants/invites/
  unclaimed shadows).

**Plane C — owner-tenant-lifecycle (owner JWT, single-tenant), 9 routes:**
- `#60 POST /api/owner/onboarding/start` (`onboarding.ts:35`) — **borderline S2** (calls `bootstrap_owner()`
  SECURITY DEFINER, mints the first membership — an auth-shaped op); assigned S10, **needs council
  confirmation before S2 or S10 flips** (map flag).
- `#61–64` — onboarding state / step-complete / step-skip / complete (`onboarding.ts:144/174/247/315`).
- `#102–104` — activation status / pickup / publish (`activation.ts:58/72/89`) — the draft→live gate.
- `#180 POST /api/owner/onboarding` (`spa-proxy.ts:758`) — **flat duplicate** of the onboarding family
  **AND a REV-7 two-writer** on `products`/`locations`/`location_themes` (tables S3-Rust owns): **STAYS
  NODE even after S10 flips** (map flag + breaker MEDIUM).

**Plane D — public-claim (single-use token, no session), 3 routes:**
- `#193 POST /api/claim/accept` (`public/claim.ts:17`) — ownership transfer via `claim_transfer` DEFINER.
- `#194 POST /api/claim/request` (`:49`) · `#195 POST /api/claim/decline` (`:69`) — token-only decline-erase.

**NOT S10 (explicit boundary):**
- **The backup CRON worker** (`workers/backup/index.ts` — `BackupCronWorker`, pg-boss schedules) — a
  **worker process = S8**, flips as a process, not a request. S10 owns only the *admin trigger* routes
  that invoke `runRestoreVerify`, not the scheduled backup engine.
- **GDPR export/erase DEFINER fns** — **S9**; S10's `retention/sweep` and `hard-delete` reuse the same
  erasure primitives but are the acquisition-lifecycle reaper, not the owner GDPR surface.
- **`cutover_flags` table + the front-door shim** — the harness's own mechanism (its ADR); S10 is gated
  BY that table (§9) but does not own it.
- **No schema change** — the DB is frozen. `platform_admins`, `platform_admin_audit_log`,
  `provision_grants`, `claim_invites`, `acquisition_sources`, `backup_metadata`, `backup_audit_log` all
  exist; S10 ports handlers over the existing schema.

**Back-of-envelope (why boring wins, and where the real ceiling is).**
- **Scale:** S10 is the **lowest-traffic surface in the whole system.** N ≈ 10–50 tenants. Platform-ops
  runs at **< 1 req/s** (B4 §, ~10 checkouts/min system BOE) — a founder opens the backups list, triggers
  a drill a few times a week. Acquisition-ops is **ops-batch** — a handful of provisionings per day at
  most, driven by an operator/CLI, not public traffic. Claim is human-round-trip (72h invite TTL).
  Onboarding/activation fire **once per tenant lifetime.** Peak S10 request rate is a rounding error.
- **The real ceiling is NOT QPS — it is the DR-drill's long-held resources.** `runRestoreVerify`
  (`backup-verify.ts:286`) holds **ONE dedicated pooled client for the lock's full lifetime** (F2, up to
  the 30-min `TIMEOUT_MS`), **spawns `pg_restore`** as a subprocess, opens a **scratch pool (max 3)**
  against `DATABASE_URL_ADMIN` (`restore-sandbox.ts:13` — a superuser-adjacent cred), and streams a full
  encrypted dump from R2. A single drill is a heavy, minutes-long, subprocess-and-superuser-cred
  operation. **This, not throughput, is why the drill orchestration should stay on Node (§5, Q3):**
  re-porting a 400-line `pg_restore`/decrypt/stream/sandbox pipeline to Rust for a 2-endpoint cold path
  is pure over-engineering; the Rust admin route should be a **thin authenticated trigger** that gates +
  audits + invokes (queue/DEFINER) the existing Node drill.
- **Cutover connection budget:** S10 is last, so during its overlap all nine prior surfaces are already
  Rust and Node is being drained. The S10 overlap adds **only the admin/acquisition cold-path draw**
  (negligible) plus the drill's dedicated client (already accounted for on Node today). The scaling-gate
  concern is *not* S10's steady state; it is that **S10's flip is the signal to shed the Node front-door
  entirely** (Phase-D, §9).
- **Conclusion:** boring monolith-in-`api` (no new runtime) is correct. The engineering risk is entirely
  **privilege correctness** (no owner→admin, no gate bypass), **cross-tenant isolation** (the all-locations
  reads post-B3), **destructive-op safety** (restore target, provisioning target, backup key), and
  **the closing-item Phase-D ownership** — not throughput.

---

## 3. Concern 1 — Platform-admin authz (Q1 🔴 — REUSE B4, do not invent a 4th role)

**The B4 verdict is settled law; S10 PORTS it, it does not re-open it.** The single sharpest correctness
fact of this whole packet: **platform-admin is NOT a JWT role.** B4's Alternative A (a 4th
`platform_admin` claim) was **explicitly REJECTED** — it would bake authority into a 24h token
(insider-removal regresses to a token-lifetime window at the highest privilege tier), require a mint site
(a self-serve-escalation surface), and ripple the red-line discriminated union. Authority is a
**server-side fact in the `platform_admins` allowlist** (`lib/platform-admin.ts:20`), re-read on **every**
admin request, keyed on `request.user.userId`.

**The port contract (each clause a red→green vector at DoD):**

1. **The Rust `Claims` enum stays 3-role (owner/courier/customer).** It is already ported that way
   (`rebuild/crates/api/src/auth/claims.rs`), and the `unknown_role_is_rejected` test already refuses a
   `"superadmin"`/4th-role token. **RED LINE: the port must NOT add a `PlatformAdminClaims` variant.**
   The admin principal is an `OwnerClaims`-authenticated user whose `user_id` is in `platform_admins`.
2. **The allowlist re-read ports verbatim.** `isPlatformAdmin(pool, user_id)` →
   `SELECT 1 FROM platform_admins WHERE user_id = $1 AND revoked_at IS NULL` (`platform-admin.ts:20`).
   The table is **non-tenant, NO-RLS** (protected by GRANTs), so the read returns identical rows under
   BYPASSRLS (today) and NOBYPASSRLS (post-B3) — **genuinely B3-independent**, no GUC, no DEFINER fn to
   inherit a pool-posture bug. `platform_admins` is granted **SELECT-only** to the operational role →
   self-serve escalation is structurally impossible (carry the GRANT posture).
3. **Fail-closed is a WIRING property, ported verbatim** (`requirePlatformAdmin`, `platform-admin.ts:33`):
   - No resolvable `userId` (a courier/customer token carries none, or no token) → **401**.
   - Allowlist miss → **403**.
   - **DB blip on the re-check → 503, fail CLOSED** — never fail-open at the top privilege tier.
   Prove it with a wired integration test (the re-check-throw→503 case), not an assertion.
4. **Immediate revocation** (`revoked_at IS NULL` re-read per request) → setting `revoked_at` denies the
   next request, no token-lifetime window (ADR-0004's philosophy). Carry.
5. **The structural plane-gate is the hard port problem (Q1a 🔴).** B4's authority is a **root-instance
   `onRequest` hook** (`registerAdminPlaneGate`, `platform-admin.ts:76`) that gates any request whose
   **matched route pattern** (`request.routeOptions.url`, NOT the raw URL — immune to case/`%2e`/`%2f`/
   trailing-slash tricks and the `/api/administrators` lookalike) is under `/api/admin`. This flows into
   **every** route context by construction, so it gates children, siblings, AND future routes with zero
   detection — the round-2 boot-guard was deleted as unrealizable (Fastify can't introspect
   context-inherited hooks). **Axum has no equivalent post-routing `onRequest` hook keyed on the matched
   template.** The Rust port must reproduce the *property* (no admin route can escape the gate) with a
   different mechanism: **nest ALL `/api/admin` routes under ONE `Router` carrying a `route_layer`
   (tower middleware = the gate)**, so registration *outside* that nested router is the only way to
   escape, and a **compile-/test-time tripwire** (the clippy/test analogue of B4's enforced eslint rule)
   plus a **re-proven sibling-closure test** (a throwaway ungated admin route → 403) is the coverage
   authority. This is the one place the port cannot be a verbatim carry — Fastify's runtime hook becomes
   Rust's structural router nesting. Q1a is 🔴 because a gate-escape at this tier is a cross-tenant breach.
6. **Uniform platform-admin-only on all 6 routes** — no tenant-vs-platform branching inside a handler
   (that branching *is* the BOLA anti-pattern B4 removed). Owner self-views are a deferred owner-plane
   seam, not built.
7. **The audit trail ports verbatim** (`platform-admin.ts:97–141`): `auditCtx` (actor_id =
   `OwnerClaims.user_id`, hashed ip/ua only — **no raw PII**), **write-ahead `started` row in its OWN
   committed tx** before any destructive drill (`auditStart`), `auditFinish` to completed/failed,
   best-effort `auditCompleted` for reads (a read must not fail on an audit blip).

**Failure-first:** the gate replies-and-returns on deny (handler never reached); a throw → 503 deny; a
non-uuid `backupId` → 400 before the drill; a drill already in flight → 409. No path reaches a handler
without passing the gate.

## 4. Concern 2 — Provisioning / acquisition (Q2 🔴)

**The acquisition-ops plane (`/internal/acquisition/*`) is a SECOND, DISTINCT gate — carry it distinctly.**

1. **The `PROVISION_OPS_SECRET` gate ports verbatim** (`ops-auth.ts`, `route.ts:56`): a single
   `onRequest` hook, `x-provision-ops-secret` header, **`crypto.timingSafeEqual`** length-checked
   compare, **fail-closed 404** (existence hidden) when the secret is unset/empty. It is **decoupled from
   the dev-login family AND from the platform-admin allowlist** on purpose (B4): enabling provisioning in
   prod must not re-arm the mock-auth owner-JWT backdoor (ADR-0003), and the secret is read from
   `process.env` at registration — **NOT the `@deliveryos/config` red-line Zod schema** — so it composes
   without touching the dev-bypass prod-offenders guard. The Rust port carries all three properties
   (timing-safe, fail-closed-404, env-not-schema).
2. **Is `/internal/*` reachable externally? YES today — and that is the residual (Q2a 🔴).** The
   acquisition routes are mounted on the **same axum/Fastify server** as the public surface; nothing
   network-isolates them. The **only** boundary is the ops-secret. B4's Alternative C (a network-isolated
   internal ops service, mTLS/segmentation) was **REJECTED as primary, KEPT as future defense-in-depth**
   (B4 R10, at a headcount/tenant threshold). **Recommendation: CARRY the secret-gated posture; DEFER
   network isolation as a documented, owned residual** — but the packet must (a) confirm the Rust port
   does not accidentally *widen* reachability, and (b) name the threshold trigger. Q2a is 🔴 because
   `/internal/acquisition/provision/*` is a cross-tenant *write* front door protected by one shared
   secret; a leaked/brute-forced secret is a mass-provisioning capability. Belt: rate-limits (30/min mint/
   spine, 10/min extract) are carried verbatim.
3. **Cross-tenant provisioning safety — the shadow-spine writes THROUGH RLS, not around it (carry
   verbatim).** `provisionShadowSpine` (`provisioning.ts:142`) is already B3-safe by construction:
   - Seats a **txn-local `app.provision_token` GUC** (`set_config(..., true)`) that the `provision_shadow`
     policy (mig 069) reads — the write is admitted by a **narrow additive policy**, never BYPASSRLS.
   - `SELECT … FOR UPDATE` on the grant row (0 rows → ROLLBACK); **state-pinned `advance(ENRICHED→
     PROVISIONED)`** (a racing second runner gets 0 rows → ROLLBACK, undoing the spine); **consume the
     grant LAST** (single-use; 0 rows → ROLLBACK). Pre-generated UUIDs, **no `RETURNING`** (organizations
     has no SELECT policy). A shadow is `owner_id NULL` + `status='closed'` + `published_at NULL` — a
     complete, non-live, non-orderable tenant.
   The port carries the ORDERING and the GUC dance exactly — an out-of-order or context-free port would
   either match 0 rows or admit a second concurrent spine.
4. **Ownership transfer — P6 flagged it "needs work"; the port carries the DEFINER carve-out + the theft
   guard (Q2b).** `acceptClaim` (`claim.ts:97`) transfers a shadow to an authenticated owner via the
   **token-gated SECURITY DEFINER `claim_transfer`** (mig 071) — **one atomic statement**; the **token is
   the sole authority**, org/location derived from the matched invite *inside* the fn (never the request →
   no IDOR/enum); leaves `published_at NULL` (no auto-publish, B3); erases the raw scraped blob; voids
   outstanding provisioning grants. **The theft guard (R3-1): the WEB claim path REFUSES a token-only
   (NULL `invited_contact_hash`) invite → `CONTACT_REQUIRED`** (`claim.ts:113`) — a leaked token-only
   invite would otherwise bind ownership to ANY authenticated account, because `claim_transfer` only
   enforces the recipient match when the hash is non-NULL; token-only invites are operator/CLI-only. The
   port carries this refusal verbatim. **The "needs work" residual** (the exact hardening P6 deferred) is
   surfaced as an accepted-risk row with an owner (§11 Q-CLAIM-TRANSFER, Q2b), not silently re-shipped.
5. **Born-with-erasure ports verbatim** (`hardDeleteShadow`, `provisioning.ts:213`): NULLs
   `place_raw`+`menu_draft` (the ingested PII), drops FK links, and erases the member-keyed shadow rows
   via the **`erase_shadow_tenant` DEFINER fn** whose **`owner_id IS NULL` guard means a CLAIMED tenant is
   never erased here** (the erasure-can't-propagate-post-B3 fix). Carry the guard — it is what stops a
   decline from nuking a claimed tenant.
6. **The acquisition state-machine ports like `order_status.rs` did for S5** (`state-machine.ts`): the
   `SOURCED→…→CLAIMED` matrix, `assertTransition` throwing a typed error on an illegal edge, the "every
   non-terminal has an exit" invariant, and `REQUIRES_REASON` on exit states. Port the matrix as a
   verified table (the S5 precedent), wire handlers onto it.
7. **Onboarding/activation (Plane C) rides the owner tenant plane** — owner JWT, single-tenant,
   `with_user`/membership-scoped (the S3 posture), NOT the platform-admin gate. **Row 60
   (`onboarding/start`) is borderline S2** (`bootstrap_owner()` DEFINER mints the first membership) —
   **council must confirm whether it flips with S2 or S10 before either flips** (map flag; Q2c). **Row 180
   (flat `/api/owner/onboarding`) is a REV-7 two-writer and STAYS NODE** (§9).

## 5. Concern 3 — Backup/DR triggers + the backup keys (Q3 🔴)

**The single largest rebuild "gap" is a truth about what was never built: there is NO restore-to-prod
endpoint.** The three admin backup routes are **list, drill, drill-report** — none of them restores over
production:

| Route | What it actually does | Target |
|---|---|---|
| `GET /api/admin/backups` | Lists `backup_metadata` + restore-test results (read) | — |
| `POST /api/admin/backups/verify` | `runRestoreVerify` — a restore **DRILL** | a **SANDBOX** db (`createSandboxDatabase`, `DATABASE_URL_ADMIN`, dropped after) |
| `GET /api/admin/backups/dr-report` | `runRestoreVerify({fullHash:true})` — fleet drill report | a **SANDBOX** db |

**The confirmation-gated real restore (restore R2 → PROD) does not exist as code — it is a manual DR
runbook** (an operator with DB creds running `pg_restore` against prod, out of band). This is the
**largest rebuild gap named in the brief, and the correct disposition is: DO NOT invent a restore-to-prod
endpoint in the Rust port.** Building an authenticated "restore over prod" HTTP trigger would be a **new,
weaponizable, irreversible capability** the current system deliberately does not expose. The port keeps
restore-to-prod as a runbook (Q3a); it ports only the *drill* trigger. Any future confirmation-gated
restore endpoint is its **own** council (double-confirmation, backup-target-pin, blast-radius), never a
side effect of the S10 port.

**Port contract for the drill trigger:**

1. **Keep `runRestoreVerify`'s orchestration ON NODE (Q3b — the boring-wins recommendation).** The drill
   is a 400-line subprocess/stream/crypto pipeline (`backup-verify.ts`): R2 download, AES-256-GCM decrypt,
   plaintext-checksum, `spawn('pg_restore')`, sandbox create/drop against a superuser-adjacent cred,
   smoke-checks. Re-porting this to Rust for **two cold-path endpoints** is over-engineering with a large
   new attack surface (a Rust `pg_restore` shell-out, a Rust R2/crypto/superuser-cred path). **The Rust
   `/api/admin` route becomes a thin authenticated trigger**: it runs the platform-admin gate + the
   write-ahead audit + the uuid validation + the rate-limit, then **invokes the existing Node drill** (via
   a job enqueue or a narrow internal call). This is a **permanent-Node carve-out inside S10** (the REV-7
   pattern), stated explicitly, not a silent gap.
2. **The single-flight lock ports as the FIX, not the pre-fix bug** (`backup-verify.ts:65–84`): **ONE
   advisory lock (key 3), held on ONE dedicated client for the whole drill, unlock-THEN-release on the
   SAME session.** The prior code released the client while the session lock was held → permanent
   leak/self-DoS (every later drill 409s). If any trigger orchestration does move to Rust, the sqlx
   session-lock-on-a-held-connection semantics are a **named port hazard** — a pool-checkout lock that is
   returned to the pool while held is the exact Node bug, re-expressible in Rust.
3. **The DR-drill hardening ports verbatim** (B4 §3/§4): platform-admin gate **AND** Zod-uuid `backupId`
   (400 non-uuid, `backups.ts:78`) **AND** per-actor rate-limit (3/5min) **AND** single-flight (409) **AND**
   the write-ahead audit row **AND** the `ADMIN_DRILLS_ENABLED` kill-switch (scopes ONLY the two heavy
   drills — the recovery reads `backups` list + `fallback/health` are **never darkened during an incident**).
4. **Backup encryption keys — never logged, never committed, fail-loud (Q3c 🔴, the secrets-incident):**
   - `BACKUP_ENCRYPTION_KEY` / `BACKUP_KEYRING` are read from **`process.env` directly, NOT the Zod
     schema** (`encrypt.ts:44–71`) so the operator-gated secret VALUE lands without a code change. Carry.
   - **`resolveBackupKey` FAILS LOUD on an unknown keyId** (`encrypt.ts:66`) — it refuses to "restore"
     with the wrong/only key and silently produce garbage. This is the primary **restore-to-wrong-target**
     control (S10-T5). Carry the throw.
   - **Every drill error/log/bus event is PII/secret-redacted** (`redactPII`, `backup-verify.ts:51,395`;
     Sentry tags truncated). The Rust trigger (and any log line it emits) must never print the key, the
     R2 secret, or `DATABASE_URL_ADMIN`. The secrets-exposure-incident is the standing reason: a key in a
     log is a key in the log aggregator is a key one breach from the ciphertext.
5. **The drill targets a SANDBOX, proven (S10-T5).** `createScratchPool(sandboxUrl)` must target the
   freshly-restored scratch db — the LC7 fix-2 bug (the factory silently discarded `sandboxUrl` and ran
   smoke-checks against **PROD**) is the canonical restore-to-wrong-target defect; the port asserts the
   smoke pool's connection string is the sandbox, not prod.

## 6. Concern 4 — RLS / tenancy: the most dangerous cross-tenant surface by design (Q4 🔴)

**Platform-admin operates ACROSS tenants — that is its job, and it is the single most dangerous
cross-tenant read/write surface in the system.** How each plane traverses RLS safely:

1. **The gate itself is B3-independent** (§3): `platform_admins` is non-tenant, no-RLS; the point-read
   returns identical rows under BYPASSRLS and NOBYPASSRLS. **No GUC, no DEFINER fn, no `app.user_id`
   dependency.** This is deliberate (B4 RA2-3 killed the round-1 DEFINER-fn design because it only
   relocated the BYPASSRLS dependency to the fn owner). The port must NOT "improve" this into a
   GUC/DEFINER path.
2. **The cross-tenant READS are the un-fixed B3 dependency (Q4a 🔴, B4 open item R1 — NOT built).**
   `fallback/health` (`fallback.ts:14`) and `r2-check` (`:48`) run a `SELECT … FROM locations` (and a
   join over `owner_notification_targets`) **across ALL tenants** — they work **today ONLY because the
   pool is BYPASSRLS.** Post-B3 (NOBYPASSRLS) these reads return **0 rows** unless they run via an
   **explicit platform-read mechanism** (a SECURITY-DEFINER fn or a platform-read role — B4 R1, still
   unbuilt). `backup_metadata` already has a system policy; `locations`/`owner_notification_targets` do
   not have a platform-read path. **This is a HARD cross-dependency the S10 flip inherits: S10 cannot flip
   onto NOBYPASSRLS without the platform-read path existing**, exactly as B4 R11 made `requireLocationAccess`
   a hard B3 blocker. **Recommendation: build the platform-read DEFINER fn (or role) as an S10 cutover
   prerequisite, co-owned by the architect + the B3 owner.** Q4a is 🔴 because it is the difference between
   an admin recovery read working during an incident and returning 0 rows at the worst moment.
3. **The provisioning writes already traverse RLS correctly** (§4.3): the `provision_shadow` policy + the
   `app.provision_token` GUC admit the shadow spine; `claim_transfer`/`erase_shadow_tenant` are DEFINER
   carve-outs with explicit guards. **No provisioning write depends on BYPASSRLS** — this half of S10 is
   already B3-safe. The port carries the GUC/policy contract; a context-free port would break it.
4. **The audit/allowlist tables** (`platform_admins`, `platform_admin_audit_log`) are non-tenant no-RLS
   with SELECT/INSERT GRANTs — belt (no tenant data) + suspenders (the app gate). Carry. (B4 R12: these
   trip Supabase linter 0013 — a cosmetic advisory; enabling RLS would re-introduce the RA2-3 trap.)
5. **Uniform, no per-tenant branch** (§3.6): the cross-tenant nature is *by design and total* — an admin
   read is either allowed (allowlisted) or 403; there is no "this admin sees only tenant X" mode. That
   mode would be the BOLA anti-pattern. The port keeps admin reads unscoped-by-tenant and audited.

## 7. Concern 5 — Cutover: the LAST surface + the Phase-D trigger (Q5 🔴)

**S10 is the tenth and final strangler flip; its completion is the trigger for Phase-D decommission
(REV-C10).** The cutover facts and controls:

1. **Atomic per-surface flip (REV-7) is HARDEST here (§1).** The whole S10 family flips together behind
   the proxy — but "the family" is **four planes with four auth mechanisms**, so the DoD gate is the
   *conjunction* of: platform-admin gate parity (§3) + ops-secret gate parity (§4) + owner-lifecycle
   parity (Plane C) + claim-token parity (Plane D). A split flip (e.g. `/api/admin/*` on Rust but
   `/internal/acquisition/*` on Node) is acceptable **only** if each plane's gate is independently proven
   — but the recommendation is to flip the whole label atomically to avoid a half-authenticated surface.
2. **Two carve-outs stay on Node inside the S10 flip** (stated, not silent): (a) the **DR-drill
   orchestration** (§5, a thin Rust trigger invokes the Node drill), and (b) **row 180 the flat
   `/api/owner/onboarding` two-writer** (REV-7, writes S3-Rust's tables). "S10 flipped" means the
   *authority + triggers* are Rust; these two heavy/hazardous sub-capabilities remain Node by design.
3. **Row 60 (`onboarding/start`) is a pre-flip council item** — its `bootstrap_owner()` DEFINER is
   auth-shaped (borderline S2). The council must decide S2-vs-S10 ownership **before either flips** (§4.7,
   Q2c), or the first-membership mint has two divergent implementations.
4. **The cutover-flags bootstrap subtlety (Q5a).** The harness's `cutover_flags` table is itself gated by
   a **platform-admin policy** (FORCE RLS + platform-admin policy; harness proposal §, threat T11). S10 is
   the surface that ports the very platform-admin authority the flip mechanism depends on. During the S10
   overlap **both stacks must agree on platform-admin authority** — which they do trivially, because both
   read the same non-tenant no-RLS `platform_admins` table (B3-independent), so a flip flag written under
   one stack's gate is honored by the other. Low risk, but named.
5. **Rollback = proxy flag-flip** (no data migration): both stacks read the same allowlist + provisioning
   tables through the same policies, so a Rust-provisioned shadow is a normal Node-readable row and vice
   versa — **provided the gate-parity + provisioning-ordering gates are green.**
6. **The Phase-D decommission — the closing item (Q5b 🔴, REV-C10).** Once S10 is green + stable N days,
   **all ten surfaces are Rust**; the cutover-harness front-door shim has done its job and must be cut
   (Phase D — the front-door role migrates to Rust, the Node shim is deleted). Counsel's REV-C10 finding:
   the shim has **no dated cut-trigger and no named owner** → the "temporary" vine becomes the permanent
   incumbent (the exact lock-in the rebuild exists to escape; the handoff's `терпіння↔прив'язаність`
   open item). **This packet's closing act: record a dated Phase-D trigger + a named owner NOW** — e.g.
   *"S10 flipped + all-ten stable ≥ N days ⇒ front-door role migrates to Rust, Node shim deleted, by
   `<owner>` before `<date>`."* Naming the owner is an **operator decision** (this agent cannot self-assign
   it); the packet surfaces it as the un-cut-vine owner for the human to fill. Q5b is 🔴 because it is the
   one long-horizon irreversibility (a permanent Node incumbent) that no per-surface gate catches.

## 8. Tenancy summary — the four gates in one table

| Plane | Routes | Gate | RLS traversal | B3 posture |
|---|---|---|---|---|
| A platform-ops | `/api/admin/*` (6) | `platform_admins` allowlist re-read (userId) | non-tenant no-RLS table (gate); **cross-tenant `locations` reads need a platform-read path** | gate B3-independent; **reads need R1 (unbuilt) — Q4a 🔴** |
| B acquisition-ops | `/internal/acquisition/*` (9) | `PROVISION_OPS_SECRET` header (timing-safe, 404) | writes THROUGH `provision_shadow` policy + `app.provision_token` GUC; DEFINER carve-outs | already B3-safe (writes through policy) |
| C owner-lifecycle | `/api/owner/onboarding\|activation/*` (9) | owner JWT (`OwnerClaims`) | `with_user`/membership-scoped (S3 posture) | inherits S3's B3 seat; row 60 borderline S2, row 180 stays Node |
| D public-claim | `/api/claim/*` (3) | single-use opaque claim token | `claim_transfer` DEFINER (token-derived org, no IDOR) | DEFINER carve-out, B3-safe |

## 9. Operability, degradation, rollback, scaling-gate

- **Health degraded-vs-down:** the gate's 503 (DB blip on the allowlist re-check) is **degraded** (fail
  closed — admin locked out, tenant plane unaffected), NOT down. The recovery reads (`backups` list,
  `fallback/health`) are **never** kill-switched (B4) so an operator can diagnose during an incident.
  The drill kill-switch (`ADMIN_DRILLS_ENABLED`) darkens ONLY the two heavy drills.
- **Observability < 1 min:** every admin action writes a `platform_admin_audit_log` row (actor_id, action,
  target, status, hashed ip/ua) — the write-ahead `started` row means a destructive drill is visible
  BEFORE it runs. Provisioning/claim emit structured-log health signals (`acquisition.shadow_declined`,
  `acquisition.complaint`). Backup drill failures fire Sentry + a `backup.verify.failed` bus event
  (redacted).
- **Rollback:** proxy flag-flip back to Node (§7.5), no data migration.
- **Scaling-gate / flip-gate:** the S10 flip is gated by `readiness_ok=true` (all four planes' DoD green)
  + operator sign-off (harness T11) + the **platform-read path (Q4a) built** as a B3 prerequisite. S10's
  flip is itself the scaling-gate signal to begin Phase-D (shed the Node front-door).

## 10. Prior-council interactions (no re-litigation)

- **B4 (ADR-admin-platform-authz)** — the platform-admin authority, the plane-gate, the DR-drill
  hardening, the write-ahead audit. **PORTED verbatim; Alternative A (4th role) stays rejected.** Open
  items B4 R1 (platform-read path) and B4 R11 (B3 blocker) are inherited as S10 cutover prerequisites.
- **P6 provisioning/claim councils** — the `provision_shadow` policy, single-use tokens, `claim_transfer`
  DEFINER, born-with-erasure, Art-14 notice. **PORTED verbatim;** the "ownership transfer needs work"
  residual is surfaced as an owned accepted-risk (Q2b), not re-shipped silently.
- **secrets-exposure-incident** — the backup key redaction/env-only/fail-loud posture is a standing
  red-line the port carries (§5.4).
- **cutover-harness REV-C10** — the Phase-D cut-trigger + owner, this packet's closing item (Q5b).
- **S2-auth** — the 3-role `Claims` enum; the admin gate keys on `OwnerClaims.user_id`; no 4th variant.
- **S3 REV-7** — the atomic-flip posture + the two-writer carve-out (row 180, drill orchestration).

## 11. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for a 🔴 security/correctness issue or a build-correctness bug, each with an explicit
test/E2E delta. Everything else CARRIES.**

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-ADMIN-NO-CLAIM | platform-admin is a server-side allowlist fact, NOT a JWT role (B4 Alt-A rejected; `platform-admin.ts:20`) | **CARRY the model** — allowlist re-read keyed on `OwnerClaims.user_id`; **RED LINE: no `PlatformAdminClaims` variant** (the 3-role enum + `unknown_role_is_rejected` test already forbid it) (🔴 S10-T1) |
| Q-ADMIN-PLANE-GATE | root `onRequest` hook keyed on matched route pattern gates children/siblings/future (`registerAdminPlaneGate`, `:76`) | **FIX-IN-PORT the mechanism (Q1a 🔴):** axum has no post-routing hook — nest ALL admin routes under ONE gated `Router` (`route_layer`) + a clippy/test tripwire + re-proven sibling-closure test (ungated sibling → 403) (🔴 S10-T8) |
| Q-ADMIN-FAILCLOSED | 401 no-userId / 403 miss / **503 on DB blip (fail CLOSED)** (`:33`) | **CARRY verbatim** — never fail-open at the top tier; wired 503-on-throw test |
| Q-ADMIN-REVOKE | `revoked_at IS NULL` re-read per request → immediate insider-removal (ADR-0004) | **CARRY verbatim** |
| Q-ADMIN-GRANT | `platform_admins` SELECT-only to the operational role → self-serve escalation structurally impossible | **CARRY verbatim** — the GRANT posture is the escalation control |
| Q-ADMIN-AUDIT | write-ahead `started` row in its OWN committed tx before a destructive drill; hashed ip/ua only (`:116`) | **CARRY verbatim** — no side-effect without a pre-committed trail; no raw PII in audit |
| Q-ADMIN-ERR-LEAK | notification-audit does NOT leak `err.message` (B4 F6, `:50`); fallback errs generic | **CARRY verbatim** — a schema/internal detail never reaches the envelope |
| Q-DRILL-RESTORE-RUNBOOK | **NO restore-to-prod endpoint exists** — real DR restore is a manual runbook (DB creds + `pg_restore`) | **DO NOT BUILD ONE (Q3a).** The port keeps restore-to-prod a runbook; a confirmation-gated restore endpoint is its OWN council, never an S10 side effect. Accepted-gap row + owner (🔴 S10-T5) |
| Q-DRILL-NODE-CARVEOUT | `runRestoreVerify` is a 400-line subprocess/stream/crypto pipeline for 2 cold-path endpoints (`backup-verify.ts:286`) | **KEEP ON NODE (Q3b).** Rust admin route = thin authenticated trigger (gate+audit+uuid+ratelimit) → invokes the Node drill; permanent-Node carve-out (REV-7) |
| Q-DRILL-SINGLEFLIGHT | ONE advisory lock (key 3), held on a dedicated client, unlock-THEN-release on the same session (`:65–84`) | **CARRY the FIX verbatim** — the prior release-while-held leak = permanent self-DoS; if any orchestration ports to Rust, the sqlx held-connection lock is a named hazard |
| Q-DRILL-HARDENING | platform-admin AND uuid `backupId` AND rate-limit(3/5min) AND single-flight(409) AND `ADMIN_DRILLS_ENABLED` (`backups.ts:73`) | **CARRY verbatim** — the kill-switch scopes ONLY the 2 heavy drills; recovery reads never darkened (🔴 S10-T7) |
| Q-BACKUP-KEY | `BACKUP_ENCRYPTION_KEY`/`BACKUP_KEYRING` from `process.env` (not Zod schema); `resolveBackupKey` FAILS LOUD on unknown keyId (`encrypt.ts:44,66`) | **CARRY verbatim** — env-only, never logged/committed (secrets-incident); fail-loud = the restore-to-wrong-target control (🔴 S10-T3/T5) |
| Q-DRILL-REDACT | every drill error/log/bus event `redactPII`'d; Sentry tags truncated (`backup-verify.ts:51,395`) | **CARRY verbatim** — no key/R2-secret/`DATABASE_URL_ADMIN` in any log line |
| Q-DRILL-SANDBOX | the drill targets a SANDBOX (`createSandboxDatabase`, `DATABASE_URL_ADMIN`), dropped after; LC7 fix-2 (smoke once ran on PROD) | **CARRY the FIX + prove** — assert the smoke pool's connection string is the sandbox, never prod (🔴 S10-T5) |
| Q-PROVISION-SECRET | `/internal/acquisition/*` gated by `PROVISION_OPS_SECRET` (timing-safe, fail-closed-404, env-not-schema, decoupled from dev-login) (`ops-auth.ts`) | **CARRY verbatim** — a DISTINCT gate from platform-admin; do not merge the two gates |
| Q-INTERNAL-NETWORK | `/internal/*` is reachable externally today; only the ops-secret bounds it (B4 Alt-C network isolation deferred) | **CARRY + DEFER (Q2a 🔴):** keep secret-gated; DEFER network isolation as an owned residual with a named threshold; port must not widen reachability (🔴 S10-T2) |
| Q-PROVISION-RLS | shadow-spine written THROUGH `provision_shadow` policy + `app.provision_token` GUC; FOR-UPDATE grant, state-pinned advance, consume-LAST, no RETURNING (`provisioning.ts:142`) | **CARRY verbatim** — the ORDERING + GUC dance are load-bearing; already B3-safe; a context-free port matches 0 rows or admits a 2nd spine (🔴 S10-T6) |
| Q-CLAIM-TRANSFER | ownership transfer via token-gated DEFINER `claim_transfer` (token sole authority, org derived in-fn, no IDOR, no auto-publish); web REFUSES token-only invites → `CONTACT_REQUIRED` (`claim.ts:97,113`) | **CARRY verbatim + FLAG (Q2b):** the theft guard (R3-1) is the control; P6's "ownership transfer needs work" residual = accepted-risk row + owner (🔴 S10-T6) |
| Q-SHADOW-ERASE | born-with-erasure `hardDeleteShadow` NULLs `place_raw`/`menu_draft`; `erase_shadow_tenant` DEFINER guards `owner_id IS NULL` (`provisioning.ts:213`) | **CARRY verbatim** — the `owner_id IS NULL` guard stops a decline erasing a CLAIMED tenant |
| Q-ACQ-STATE-MACHINE | `SOURCED→…→CLAIMED` matrix; illegal edge throws; every non-terminal has an exit; exit states REQUIRES_REASON (`state-machine.ts`) | **CARRY as a ported verified table** — the S5 `order_status.rs` precedent |
| Q-XTENANT-READ | `fallback/health`/`r2-check` SELECT over ALL `locations` — works ONLY via BYPASSRLS today (`fallback.ts:14`) | **BUILD the platform-read path (Q4a 🔴):** a platform-read DEFINER fn/role before S10 flips onto NOBYPASSRLS (B4 R1, unbuilt) — else 0 rows at incident time (🔴 S10-T4) |
| Q-ROW60-ONBOARDING | `onboarding/start` calls `bootstrap_owner()` DEFINER (auth-shaped, borderline S2) (row 60) | **COUNCIL-CONFIRM S2-vs-S10 ownership before either flips (Q2c)** — else the first-membership mint has two impls |
| Q-ROW180-TWOWRITER | flat `/api/owner/onboarding` (`spa-proxy.ts:758`) writes `products`/`locations`/`location_themes` (S3-Rust's tables) | **STAYS NODE (REV-7)** — never flips despite the S10 label; named carve-out |
| Q-CUTOVER-FLAGS-GATE | `cutover_flags` is platform-admin-gated (FORCE RLS + policy); S10 ports the authority the flip mechanism depends on | **CARRY** — both stacks read the same non-tenant `platform_admins` (B3-independent), so a flag written under one gate is honored by the other (Q5a) |
| Q-PHASE-D-VINE | the front-door shim has no dated cut-trigger + no owner (REV-C10 un-cut vine) | **RECORD NOW (Q5b 🔴):** dated Phase-D trigger + named owner; the un-cut-vine owner is an OPERATOR decision — surfaced for the human to fill |

## 12. Cutover DoD (REBUILD-MAP §3, this surface — the LAST)

The B4 admin-authz E2E net green (owner→403 ×6 asserting JSON-403-not-SPA-200 · platform-admin→200 ×6 ·
courier/customer→401/403 · drill 429+409 · write-ahead audit row · **structural sibling-closure**: an
ungated `/api/admin` sibling → 403; `/api/administrators` lookalike NOT gated) · the P6 provisioning/claim
E2E (acquisition→live · `provision_shadow` RLS admits only owner-NULL shadows · concurrent double-spine →
one wins/one ROLLBACK · `claim_transfer` token-only → `CONTACT_REQUIRED` · decline-and-erase clears
`place_raw`/`menu_draft` · `erase_shadow_tenant` never touches a CLAIMED tenant) · the restore-drill proof
(SANDBOX target asserted · plaintext-checksum vs manifest · fail-loud unknown keyId · redacted error path ·
single-flight 409) · `openapi-diff` empty for the S10 namespaces · invariant-cluster red→green:

- **Platform-admin gate parity** — the 3-role `Claims` enum unchanged (no 4th variant); allowlist
  re-read → 401/403/503 wired; revoke → next-request-deny; **the axum plane-gate sibling-closure re-proven
  in Rust** (Q1a).
- **Ops-secret gate parity** — timing-safe compare; fail-closed-404 when unset; env-not-schema; the port
  did not widen `/internal/*` reachability (Q2a).
- **Provisioning ordering + GUC** — `app.provision_token` seated; FOR-UPDATE → state-pinned advance →
  consume-LAST; a racing runner → ROLLBACK; the DEFINER carve-outs' guards intact (Q2b).
- **Backup key safety** — env-only, never in a log/commit (a grep-gate that no drill line prints the key/
  R2-secret/`DATABASE_URL_ADMIN`); fail-loud keyId; sandbox-target asserted (Q3c).
- **Cross-tenant read path** — the platform-read DEFINER fn/role built + proven under NOBYPASSRLS
  (`fallback/health` returns fleet rows, not 0) BEFORE S10 flips (Q4a 🔴).
- **Restore-to-prod** — NO endpoint added (a coverage assertion that the S10 namespace exposes no
  restore-over-prod route) (Q3a).
- **Carve-outs named** — the drill orchestration + row 180 confirmed staying on Node; row 60 S2-vs-S10
  council decision recorded.
- **Phase-D** — a dated cut-trigger + named owner recorded in the cutover-harness ADR (Q5b 🔴).

map-coverage zero-diff for the S10 namespaces · **council sign-off + rollback plan** (atomic proxy
flag-flip of the whole S10 family back to Node) · **the Phase-D decommission trigger + owner named.**
**No 🔴 S10 row builds before this packet is APPROVED and the 🔴 questions (Q1/Q1a/Q2a/Q2b/Q3/Q4a/Q5b) are
operator-signed.**

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1a
axum-plane-gate / Q2a `/internal` network boundary / Q2b ownership-transfer residual / Q3
restore-runbook + backup-key / Q4a platform-read path (B3 blocker) / Q5b Phase-D decommission owner).
**packet-status: 🟡 DRAFT.**
