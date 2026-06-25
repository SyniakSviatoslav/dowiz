# ADR — Soft Access Gate (public "register interest" CTA)

- Status: ACCEPTED (post-Council R1 + STOP-ETHICS human decisions + Breaker/Counsel R2
  RESOLVE + Breaker R3 RESOLVE). STOP-1 & STOP-2 resolved by owner ruling; R2 + R3 re-attacks
  disposed (see `resolution.md` §"R2 resolution" + §"R3 resolution"). Residual = normal copy
  polish only.
- **R3-RESOLVE deltas (supersede earlier wording below):** (R3-1) the new cron worker's
  `boss.schedule` is **`.catch`-wrapped (best-effort)** — it copies the `rates-refresh.ts:25`
  shape, **not** the un-`.catch`'d `anonymizer-retention.ts:27` shape — so a schedule throw
  **cannot abort boot before `fastify.listen()`** (`server.ts:905`); a separate post-listen
  **boot-assert** fail-fast-exits (`process.exit(1)`) if the two cron schedules are absent, so
  a failed cron registration is a **visible deploy failure**, not a half-booted zombie. The
  prior "copy the anonymizer shape, do NOT `.catch`" instruction (R2-1) is **superseded**.
  (R3-3) Decision-6's anti-enumeration invariant is **reformulated** to "indistinguishable
  along the email-existence axis" (held), and the "zero non-200 for any body" over-promise is
  retracted: framework malformed-JSON 400 is **accept-risk** (not an email-oracle). Frontend
  success is **gated on locally-confirmed `consent===true`-in-body + 2xx**, not a bare 200, so
  a serialization-drift/bug no-consent body shows `error`/retry, not a false success. (R3-4)
  `ACCESS_GATE_PUBLIC_ENABLED` gates **backend `fastify.register`** (route 404s while off),
  not merely the frontend render — the STOP-1 prerequisite is enforced at the reachable
  surface.
- **R2-RESOLVE deltas (supersede earlier wording below):** consent-400 **removed** → silent
  uniform-200 honeypot drop + structural `z.literal(true)` gate (R2-3/R2-8); `clientIp()`
  reads **`Fly-Client-IP` only**, no spoofable XFF (R2-2); `/privacy` added to `SPA_ROUTES`
  (R2-5); retention/reconcile cron scheduling proven via 16 live schedulers, no new
  migration (R2-1); launch gate = `ACCESS_GATE_PUBLIC_ENABLED` flag + banned-strings CI test
  (R2-10); privacy-notice content-hash CI test (R2-6); reconcile bounded by `notify_attempts`
  (R2-9); `/privacy` carries a reachable erasure contact (Counsel R2 #2).
- Date: 2026-06-20
- Deciders: System Architect (Triad); Council R1 (Breaker B1–B12 + Counsel STOP-1/2);
  owner ruling on both ETHICAL-STOPs (`ethical-decisions.md`, 2026-06-20).
- Supersedes / conflicts: none (additive); does NOT alter ADR-006 (Data API
  perimeter — anon/authenticated stay off `public`).
- **Blocking prerequisite (STOP-1):** `owner-onboarding-invite-gating` must ship first
  (separate Council). This ADR does **not** launch to prod until it does.
- Companion: `docs/design/soft-access-gate/proposal.md` ·
  `docs/design/soft-access-gate/resolution.md` ·
  `docs/design/soft-access-gate/ethical-decisions.md`

## Context

We want a frictionless public capture: visitor enters one email, **ticks one consent
checkbox** (lawful basis = explicit consent, STOP-2), presses one button → a row in a new
**non-tenant** `access_requests` table (with consent timestamp + privacy-notice version)
→ best-effort operator email (Resend → `WAITLIST_NOTIFY_EMAIL`). Submit ≠ account: zero
auto-provisioning. Manual admission is out of band. The table stores PII (email), which
forces stricter isolation than the existing non-tenant analytics tables **and** a full
GDPR posture: consent basis, day-one erasure, 12-month retention auto-erase, a `/privacy`
notice.

**Launch is gated (STOP-1):** owner onboarding is open self-serve today, so the human
ruling is to ship `owner-onboarding-invite-gating` (separate Council) **before** this CTA
goes live — making the gate true rather than softening copy.

## Decision

1. **Non-tenant table `access_requests`** (migration `1790000000041_access-requests.ts`,
   next free after head `1790000000040_order-tip`; forward-only). `email text NOT NULL
   UNIQUE` as the idempotency anchor (stored trim+lower in code). `ip_hash` (sha256
   prefix of the **real client IP**, B2), never raw IP. **Consent capture (STOP-2):**
   `consent_at timestamptz NOT NULL` + `privacy_version text NOT NULL` — the per-row
   evidence of explicit consent (no `consent boolean` column: a present `consent_at` is
   the proof). `user_agent` **dropped** (no named use).

2. **RLS write pattern = A2 (ENABLE+FORCE + single `FOR ALL USING(true)` ops policy).**
   The public route writes via the **operational pool**, whose role
   `deliveryos_operational_user` is **NOBYPASSRLS** (`db/src/index.ts:32` guardrail).
   A no-policy/BYPASSRLS pattern (used by `users`) would deny that role's INSERT, so we
   reproduce the `ops_worker_heartbeat` policy pattern
   (`1780691408625_ops-heartbeat-policy.ts`). `anon`/`authenticated`/`service_role` are
   hard-revoked and lack schema USAGE (`1780421100065:51`) — the GRANT layer is the real
   boundary; FORCE silences the linter and blocks future BYPASSRLS leakage. We reject
   the `analytics_events` (RLS-not-forced) pattern because that is explicitly for
   non-PII data only.

3. **Notification = claim-check via pg-boss** (`access-request.notify`, dotted name per
   `queue-names.ts` convention). Payload is `{ requestId }` only — **zero PII in the
   queue**. The worker **claims-before-send** (CAS `UPDATE … SET notified_at=now() WHERE
   id=$1 AND notified_at IS NULL RETURNING email` — re-fetch in the same statement),
   sends, and **rolls `notified_at` back + throws on send failure** so pg-boss retries.
   The worker **acks-without-throwing on a missing row** (erasure tolerance). [B8]
   - **B1 FIX — queue durability**: **three** queues (`access-request.notify`,
     `access-request.reconcile`, `access-request.retention-sweep`) + their partition
     tables are pre-created by a new forward-only migration `1790000000042` **under the
     migration role** (mirroring `1790000000011`), because the runtime role has no CREATE
     on `pgboss` (`1790000000009:20`). Runtime `createQueue` at boot is a no-op, **not**
     the provisioning path. The original "rely on runtime createQueue" plan was
     structurally blocked → would have silently lost all notifications. All three names
     added to `QUEUE_NAMES` (`queue-names.ts`) and to `APP_QUEUES` (`1790000000011`) for
     fresh-provision parity.
   - **Retention sweep (STOP-2)**: a third pg-boss cron `access-request.retention-sweep`
     (`DELETE … WHERE created_at < now() - '12 months'`) auto-erases at the retention
     boundary, new worker `workers/access-request-retention.ts` mirroring
     `anonymizer-retention.ts` for the **`run()`/advisory-lock** body, but with a **R3-1
     boot-safety divergence**: its `boss.schedule(...)` (and the reconcile cron's) is
     **`.catch`-wrapped** (`rates-refresh.ts:25` shape) so a schedule throw at boot is a
     **logged best-effort failure**, never an uncaught reject that propagates through the
     un-guarded `main()` (`server.ts:401`-block) and aborts before `fastify.listen()`
     (`:905`) — which the `unhandledRejection`-kept-alive guard (`:129`) would otherwise mask
     as a live-but-HTTP-dead zombie. **Failure is still visible**, not swallowed: a dedicated
     post-listen **boot-assert** queries `pgboss.schedule` for both cron rows and, if either
     is missing, calls `process.exit(1)` so Fly restarts and the deploy **shows red**
     (fail-fast > silent zombie). See Decision-3 R3-1 note + proposal §5/§9.
   - **B3/B5 FIX — enqueue off the critical path + reconciliation sweep**: the handler
     **replies first, then fire-and-forgets** the enqueue (kills the new-vs-duplicate
     timing oracle). A pg-boss cron `access-request.reconcile` re-enqueues `notified_at
     IS NULL` rows (now **in scope** — closes the enqueue-after-commit gap that, with the
     admin UI deferred, otherwise has no v1 recovery).
   Rejected: synchronous in-handler Resend; transactional outbox table (over-eng for the
   volume — sweep is cheaper).

4. **Resend channel = DIRECT system-channel adapter, NOT the tenant dispatcher** [B6 FIX].
   `NotificationTarget`/`NotificationData` require `locationId` and
   `notification_outbox_audit.location_id` is NOT NULL — an access request has no location.
   So the worker calls a thin `EmailAdapter.sendOps({to,subject,…})` **directly**
   (`AbortSignal.timeout(5000)`), gated on `RESEND_API_KEY`. We do **not** add an `'email'`
   channel/`ops.access_request` event to the tenant dispatcher and do **not** write the
   tenant audit table. Observability for this channel = the `access_requests` row
   (`notified_at`) + counters. **B7**: per-job exhaustion does **not** page; the sweep
   emits **one aggregated** ops alert on a persistent backlog; a present-but-invalid key
   surfaces one boot-time alert.

5. **Route = `POST /api/access-requests`** (NOT `/api/v1/...`). Deviation from prompt:
   the repo has no `/v1` segment anywhere; every public route is un-versioned under
   `/api` with the path baked into the route file (`telemetry.ts` → `/api/telemetry`,
   registered at `server.ts:568`). A lone `/v1` would be an unproven inconsistency.

6. **Anti-enumeration + abuse** (R3-3 — invariant reformulated honestly): the load-bearing
   invariant is **indistinguishability along the email-existence axis** — a new vs duplicate
   vs honeypot vs no-consent vs malformed-email body all return **byte-identical
   `200 {ok:true}` with no timing delta** [B5], so the response **never leaks whether an
   email is already stored**. This is the property that matters and it **holds**. The earlier
   wording "the route emits no non-200 for *any* well-formed-or-malformed body" was an
   **over-promise (superseded, R3-3b)**: a body that is **not valid JSON** is rejected by
   Fastify's content-type parser → global error handler **400** (`server.ts:516-523`)
   **before the route handler ever runs** — the route physically cannot turn that into a 200.
   That 400 is **framework-level and identical for every JSON route in the app**, so it leaks
   **nothing** about email existence (it is the same 400 a regulator gets POSTing garbage to
   `/api/telemetry`); it only signals "this path expects JSON," which the route name already
   reveals. **ACCEPT-RISK (R3-3b): framework malformed-JSON 400** — not an email-existence
   oracle; the route is a publicly-known path; no fix (fixing it would require intercepting
   the content-type parser app-wide, an over-engineering against a non-leak). Owner: security.
   Identical `200 {ok:true}` for new/duplicate/honeypot/no-consent/malformed-**email** (route
   self-parses a *valid-JSON* body; no global 400 on those) [B5]; per-IP rate-limit `5/min`
   keyed by a **managed `keyGenerator` reading the REAL client IP from `Fly-Client-IP`
   ONLY** (R2-2 — the spoofable `X-Forwarded-For[0]` fallthrough is **removed**; Fly sets &
   overwrites `Fly-Client-IP` so it is not client-injectable; non-prod degrades to
   `request.ip`, prod-with-no-header fails closed to a shared bucket + boot warn). Fastify
   has **no `trustProxy`** (so `request.ip` = proxy); `ip_hash` hashes the same `Fly-Client-IP`.
   Honeypot is **secondary** (rate-limit primary) [B11]; localStorage guard is UX-only [B12].
   We do **not** flip global `trustProxy`; telemetry/otp `request.ip` is a **separate
   defer-flag** (stays deferred per R2-2).
   - **Consent (STOP-2; R2-3/R2-8 REVISED — NO 400):** missing/false/truthy-string
     `consent` → **silent uniform `200 {ok:true}`** via the honeypot path (no INSERT, no
     `consent_at`, no enqueue), gated by a structural `z.literal(true)` Zod parse (R2-8 —
     not a slip-prone hand `if`). The earlier consent-400 was **removed** (R2-3): the
     Breaker proved it was the *cheapest* path → a DoS-amplification target **and** a
     status/latency route-fingerprint distinct from every 200 branch. Folding no-consent
     into the uniform-200 contour kills both. Lawful basis holds: a row is **only** written
     on structurally-validated `consent === true`, and *no row = no processing*. The route
     now emits **no non-200 for any well-formed-or-malformed body** (only a DB-503 transport
     failure). This supersedes the prior "explicit 400" decision.

8. **PII posture + lawful basis (honest)** [B4]: RLS `USING(true)` is a linter/anti-BYPASSRLS
   guard, **not** row isolation; the real boundary is the GRANT layer (app-wide, shared
   operational pool). Accepted — a dedicated write-only role would not shrink the read
   surface (worker/erasure/ops-list all SELECT).
   - **Lawful basis = EXPLICIT CONSENT (STOP-2, withdrawable).** Per-row evidence
     `consent_at` + `privacy_version`; server validates `consent === true`. Withdrawing
     consent = erasure (same DELETE path).
   - **Day-one erasure** (operational role `DELETE` grant + `scripts/erase-access-request.ts
     <email>` + runbook) **plus** a **12-month retention auto-erase** cron
     `access-request.retention-sweep` (`DELETE … WHERE created_at < now() - '12 months'`,
     config `ACCESS_REQUEST_RETENTION`), mirroring `anonymizer.retention`
     (`workers/anonymizer-retention.ts`), advisory-lock guarded. Stage-30 anonymizer-folding
     demoted to redundant.
   - **Minimal `/privacy` page (STOP-2)** in scope: new SPA route in `apps/web/src/main.tsx`
     + `PrivacyPage.tsx` (sq/en), stating basis(consent)/data/purpose/retention(12mo)/
     rights/contact; renders the version === `PRIVACY_NOTICE_VERSION` written on submit.
   - `user_agent` **dropped** (no named use); `ip_hash` retained, real client IP.

9. **STOP-1 — invite-gating FIRST + copy is "register interest", not scarcity.** Per the
   owner ruling, `owner-onboarding-invite-gating` is a **blocking prerequisite** (separate
   Council): this feature does not launch until it ships. All user-facing wording reframed
   to "keep me posted / we'll be in touch." Banned strings (until invite-gating ships):
   waitlist, request access, early access, position #, approved, under review, application.
   "waitlist/approved" copy is permitted **only after** invite-gating makes the scarcity
   real. *Final copy = normal human polish (CLAUDE.md).*
   - **R2-10 + R3-4 + Counsel R2 #1 — the gate is now MECHANICAL at the API layer, not just
     render.** Three enforcers replace the prior "release-sequencing gate, not a code flag"
     sentence:
     1. **Backend route-registration gate (R3-4 — load-bearing for STOP-1).**
        `ACCESS_GATE_PUBLIC_ENABLED` (Zod env, default `false`) gates the
        **`fastify.register(accessRequestRoutes, …)` call itself** (`server.ts`). While the
        flag is off the route is **not mounted** — `POST /api/access-requests` returns the
        same `404 {error:'Not found'}` as any unknown path (it falls through to
        `setNotFoundHandler`, `server.ts:871`). The capture endpoint is therefore **not
        publicly POST-able before invite-gating ships** — closing R3-4's "frontend hidden but
        backend live" hole. The migrations (table + queues) still ship (additive, harmless);
        only the *route* is gated. **This is the STOP-1 enforcement** — not the render flag.
     2. **Frontend render gate.** The same flag also omits the CTA from the SPA (no exposed
        UI). Secondary to (1) — render-hiding alone never satisfied STOP-1 (R3-4).
     3. **CI banned-strings test** — fails the build if any banned scarcity string appears in
        the access-request i18n keys while `ACCESS_GATE_INVITE_GATING_SHIPPED` is unset.
     **Launch sequencing (fixed):** migrations may ship anytime (route stays 404 while flag
     off) → invite-gating Council → invite-gating shipped → flip `ACCESS_GATE_PUBLIC_ENABLED`
     on (route mounts) → soft-access-gate live. The flag flip is the single, reviewable,
     CI-gated launch act. The sequencing promise is now a sequencing **proof** at the
     reachable surface, not just the visible one.

7. **Best-effort email is contractual**: email failure (Resend down, queue down at
   enqueue, retries exhausted) **never** rolls back the committed submit. DB is the
   source of truth. Exhausted retries → ops alert (Telegram-ops), not a user error.

## Consequences

- Positive: frictionless capture; PII-minimal (hashed IP, claim-check, no email in
  queue/logs); durable, decoupled notification; reuses proven RLS + queue + dispatcher
  patterns; additive and flag-able (unset secrets → email leg no-ops, submits persist).
- Negative / accepted: operator email is best-effort (launch-spike surplus discoverable
  via `status='new'` dashboard, not push); rare duplicate operator mail on double job
  delivery; honeypot+rate-limit only (no CAPTCHA) for v1.

## BLOCKING PREREQUISITE (STOP-1 — must ship before this feature launches)

Owner onboarding is **open self-serve today**: Google login (`auth.ts:112-118,:138`)
and Telegram login (`auth.ts:184-188,:213`) both create a `users` row and immediately
mint an `owner` token with no invite/allowlist check; onboarding only checks
`requireRole(['owner'])` (`onboarding.ts:30`). The owner ruled (STOP-1) to **make the gate
true**: `owner-onboarding-invite-gating` is promoted from defer-flag to a **blocking
prerequisite** — this feature does **not** launch to prod until invite-gating ships.

**Surface for the separate Council (recorded as input, NOT designed here):** where the
allowlist/invite check is inserted (token-mint at `auth.ts:138`/`:213` vs onboarding
`onboarding.ts:30`), what backs the allowlist (`access_requests.status='invited'` vs a
separate invite store), first-owner bootstrap, RS256 claim shape. **Sequencing:**
invite-gating Council → invite-gating shipped → soft-access-gate shipped.

## Defer-flags (tracked)

admin review UI · Turnstile/hCaptcha · double-opt-in · re-consent flow (newer
`privacy_version`) · `locale` enum reconciliation (`'al'` spec vs app `'sq'`).

> **Promoted out of defer (now in scope):** owner-onboarding-invite-gating →
> **blocking prerequisite** (above, separate Council); reconciliation sweep
> `access-request.reconcile` (R1); explicit-consent capture, **12-month retention
> auto-erase** (`access-request.retention-sweep` — anonymizer Stage-30 folding now
> redundant), minimal `/privacy` page (R2 / STOP-2).
