# Design Proposal — Soft Access Gate (public "register interest" CTA)

> Status: REVISED (post-Council R1 + STOP-ETHICS R2 + Breaker R3 RESOLVE) · Author:
> System Architect (Triad) · Date: 2026-06-20
> Branch: feat/golive-remediation · Migration head at authoring: `1790000000040_order-tip`
> Companion ADR: `docs/adr/ADR-soft-access-gate.md`
> Resolution log: `docs/design/soft-access-gate/resolution.md`
> Human decisions: `docs/design/soft-access-gate/ethical-decisions.md`

Design artifact only. No production code. Code authored by others against this spec.

> **R3-RESOLVE banner (Breaker R3, 2026-06-20).** Two new R3-HIGH closed by mechanism (full
> table in `resolution.md` §"R3 resolution"):
> - **R3-1 (HIGH): the new cron's `boss.schedule` is `.catch`-wrapped best-effort**, NOT the
>   un-`.catch`'d anonymizer shape — so a schedule throw **cannot abort `main()` before
>   `fastify.listen()`** (`server.ts:905`) and produce a live-but-HTTP-dead zombie under the
>   `:129` kept-alive guard. Failure is made visible by a **post-`listen()` fail-fast
>   boot-assert** (`process.exit(1)` if either cron's `pgboss.schedule` row is missing → Fly
>   restarts, deploy shows red). The R2-1 "copy anonymizer, no `.catch`, so it surfaces to
>   boot" instruction is **superseded** (it backfired given the real boot topology). 12mo
>   auto-erase is never on a zombie path. Legacy un-`.catch`'d crons → **defer-flag** (owner
>   platform), NOT widened here.
> - **R3-3 (HIGH) split:** (a) **silent legit-user lead-loss** closed **at the client** —
>   `success` now requires locally-confirmed `consent===true`-in-sent-body **+** 2xx (not a
>   bare 200), with a negative UI proof case (no-consent send → NOT success). (b) **The "zero
>   non-200 for any body" claim was an over-promise** — a malformed-**JSON** body gets a
>   framework 400 at the content-type parser (`server.ts:516`), which the route cannot reach.
>   That 400 is **app-wide and leaks nothing about email existence** → the anti-enumeration
>   invariant is **reformulated honestly** ("indistinguishable along the email-existence axis",
>   §1/ADR Decision-6) and the framework-400 is **accept-risk** (owner security), not an
>   email-oracle.
> - **R3-4 (MED, critical for STOP-1): `ACCESS_GATE_PUBLIC_ENABLED` gates the backend
>   `fastify.register`**, not just the frontend render — `POST /api/access-requests` **404s
>   while the flag is off**, so the capture endpoint is not publicly POST-able before
>   invite-gating ships (§8). CI proof: flag off → 404, on → 200.
> - **R3-2 (LOW):** `Fly-Client-IP` warn re-arms on a throttled in-process 60s cadence
>   (accept-risk, owner platform). **R3-5 (LOW): VERIFIED-CLOSED** — retention DELETE ×
>   notify/reconcile is in-flight-safe (B8.1 CAS).

> **R2-RESOLVE banner (Breaker R2 + Counsel R2, 2026-06-20).** Re-attack disposed; material
> changes from the R2 STOP-ETHICS draft below (full table in `resolution.md` §"R2 resolution"):
> - **R2-3 + R2-8 (HIGH): the consent-400 is GONE.** No-consent is now a **silent
>   uniform-200 honeypot-class drop** (no INSERT, no `consent_at`), gated by a **structural
>   Zod `z.literal(true)`** parse — not a slip-prone hand `if`. This kills the consent-400
>   DoS-amplifier + route-fingerprint oracle the Breaker proved, and removes the *only*
>   non-200 the route used to emit for a well-formed body. The "consent-400" wording
>   throughout §1/§5/§6/§7 and ADR §Decision-6 is **superseded** — lawful basis still holds
>   (no row is ever written without a structurally-validated `consent === true`; *no row =
>   no processing*).
> - **R2-2 (HIGH): `clientIp()` reads `Fly-Client-IP` ONLY.** The spoofable
>   `X-Forwarded-For[0]` trust-fallthrough is **removed**. Non-prod fallback = `request.ip`
>   (deterministic, never client-controlled XFF); prod with no `Fly-Client-IP` fails closed
>   to a shared bucket + boot warn (degrade, never trust a spoofable header).
> - **R2-1 (HIGH): retention/reconcile cron scheduling is PROVEN, not assumed.** 16 live
>   `boss.schedule(...)` callers (incl. `anonymizer.retention`) write `pgboss.schedule` on
>   prod today under the same runtime role; `pgboss.schedule` DML is grant-clean
>   (`0009`/`0011` loops + `ALTER DEFAULT PRIVILEGES`). No new migration. The crons inherit
>   the proven pattern; boot-assert added.
> - **R2-5 (HIGH): `/privacy` added to `SPA_ROUTES`** (`server.ts:870`) → 200 for any
>   `Accept`, not a JSON 404 to regulator/curl/crawler.
> - **R2-6/8/9/10 (MED/LOW): mechanical backstops** — privacy-notice content-hash CI test
>   (R2-6); reconcile bounded-attempt guard + `notify_attempts` column (R2-9);
>   `ACCESS_GATE_PUBLIC_ENABLED` flag + banned-strings CI test (R2-10).
> - **Counsel R2 #2:** `/privacy` carries a **reachable erasure-request contact**.

> **R2 revision banner (STOP-ETHICS human decisions, 2026-06-20).** The owner ruled on the
> two ETHICAL-STOPs. Material changes from R1:
> - **STOP-1 → INVITE-GATING FIRST (blocking prerequisite).** The former defer-flag
>   `owner-onboarding-invite-gating` is promoted to a **hard prerequisite**: soft-access-gate
>   does **not** launch in prod until invite-gating ships. Sequencing is fixed:
>   invite-gating council → invite-gating shipped → this feature. Invite-gating is a
>   separate auth-flow change designed by a **separate Council** — its surface is recorded
>   here as an input, **not** designed here. (§9 prerequisite, §10)
> - **STOP-2 → lawful basis is EXPLICIT CONSENT** (withdrawable), not legitimate interest.
>   New consent-capture columns `consent_at` + `privacy_version` (§4); mandatory
>   server-validated `consent: true` literal (§5/§7); a required consent checkbox gates the
>   submit button on the frontend (§10). The frictionless invariant is restated honestly:
>   **one field + one consent checkbox + one button.** (§1, §4, §5, §7, §10)
> - **STOP-2 → retention 12 months + auto-erase IN SCOPE.** A pg-boss cron
>   `access-request.retention-sweep` `DELETE`s rows older than 12 months (§5/§6/§8),
>   mirroring `anonymizer.retention`. The day-one manual erasure path stays; the sweep is
>   automation *on top*, not a deferred replacement. (§4, §5, §6, §8)
> - **STOP-2 → minimal `/privacy` page IN SCOPE** (sq/en, design tokens), added as one SPA
>   route in `apps/web/src/main.tsx`. `privacy_version` versions the notice the user
>   consented to. (§8, §10)
> - **STOP-2 → `user_agent` confirmed dropped** (already dropped in R1; re-affirmed). (§4)

> **R1 revision banner.** This revision incorporates Breaker findings B1–B12 and the two
> Counsel ETHICAL-STOPs. Material changes from the PROPOSED draft:
> - **B1 fixed**: the queue is **pre-created by a migration under the migration role**
>   (mirroring `1790000000011`), not by runtime `createQueue` at boot. (§4, §8)
> - **B2 fixed**: route gets a **`keyGenerator` that reads the real client IP** from
>   `Fly-Client-IP`/`X-Forwarded-For`; `ip_hash` hashes the real client IP. The latent
>   telemetry/otp `request.ip` bug is **defer-flagged** (not in scope). (§2, §7)
> - **B3 fixed**: a **reconciliation sweep** (`notified_at IS NULL`) is brought **into
>   scope** as a tiny pg-boss cron — it closes the enqueue-after-commit gap and the B1
>   residue. (§5, §6)
> - **B5 fixed**: enqueue moved **off the critical path** (fire-and-forget *after* the
>   response is sent) so new vs duplicate are timing-indistinguishable. (§5, §7)
> - **B6 fixed**: the ops email is sent by a **direct EmailAdapter call** (system channel,
>   no `locationId`), **not** through the tenant dispatcher. (§7)
> - **STOP-1 (Counsel) revised**: all user-facing copy reframed to **"register interest /
>   keep me posted."** No "waitlist / request access / position #N / reviewed." (§10)
> - **STOP-2 (Counsel) revised**: **day-one erasure path** (operator runbook + a single
>   parameterized `DELETE` grant), a **stated lawful basis + retention window** in privacy
>   micro-copy, and **`user_agent` dropped** (no named use). (§4, §7, §8)

---

## 1. Problem + Non-Goals

### Problem
We want a **frictionless public expression-of-interest CTA**: a visitor types **one
email**, ticks **one consent checkbox** (lawful basis = consent, STOP-2), and presses
**one button**. We persist that interest — together with the consent timestamp and the
privacy-notice version consented to — in a new **non-tenant** table `access_requests`,
fire a **best-effort** email to the operator (`WAITLIST_NOTIFY_EMAIL` via Resend), and
stop. Manual admission happens out of band.

> **Frictionless invariant (restated honestly post-STOP-2).** "Frictionless" no longer
> means a single field. Consent is a deliberate, regulator-required micro-friction: the
> minimum lawful surface is **email + consent checkbox + button** (the submit button is
> disabled until consent is ticked). This is the floor; we add nothing beyond it.

> **Launch prerequisite (STOP-1, blocking).** This feature does **not** ship to prod until
> `owner-onboarding-invite-gating` ships (separate Council). Until then the door is open
> self-serve, so the CTA is interest-capture, not a gate. "waitlist/approved" copy is
> forbidden until invite-gating lands (§9, §10).

### Non-Goals (explicit)
- **Zero auto-provisioning.** A submit never touches `users`, `organizations`,
  `locations`, `memberships`, or any access grant. Submit ≠ account.
- No admin UI to review/triage requests (defer-flag).
- No double-opt-in / email confirmation (defer-flag).
- No CAPTCHA/Turnstile (honeypot + rate-limit only for v1; defer-flag).
- No status transition automation (`new→invited→declined` are manual, out of scope).
- No change to the owner auth/onboarding flow (see §10 precondition defer-flag).

### Hard invariants (red lines)
- DB is source-of-truth for the submit; the email is best-effort — **email failure
  never rolls back the submit**.
- **Claim-check**: queue payload carries `{ requestId }` only — zero PII (no email) in
  the queue. Zero email in logs as plaintext.
- **Anti-enumeration (R3-3 — invariant stated honestly).** The **load-bearing** property is
  **indistinguishability along the email-existence axis**: new / duplicate / honeypot /
  no-consent / malformed-**email** all return **byte-identical `200 {ok:true}` with no
  observable timing delta** (B5: enqueue is fire-and-forget *after* the response), so the
  response **never reveals whether an email is already stored.** This is the invariant that
  matters, and it **holds**. Malformed-**email** (Zod) returns `200 {ok:true}` too — the route
  does its own lenient parse on a *valid-JSON* body and never surfaces a 400 to a submitter
  (B5 fix; see §7). **Consent (STOP-2; R2-3/R2-8):** a missing/false/truthy-string `consent`
  returns the **same `200 {ok:true}`** as the honeypot path — a **silent uniform-200 drop with
  no INSERT and no `consent_at`** — so it adds **no** branch on the email-existence axis. The
  earlier consent-400 was removed because the Breaker proved it was (a) the *cheapest* path →
  an optimal DoS-amplification target, and (b) a status+latency **route-fingerprint** distinct
  from every 200 branch. Lawful basis is unharmed: a row is **only ever written on a
  structurally-validated `consent === true`** (`z.literal(true)`, R2-8); *no row = no
  processing*.
  > **R3-3b — over-promise retracted (accept-risk, not a fix).** The earlier wording "the
  > route emits **no non-200 for any well-formed-or-malformed body**" was **too strong**. A
  > body that is **not valid JSON** is rejected by Fastify's content-type parser → global
  > error handler → **400** (`server.ts:516-523`) **before the route handler runs** — the
  > route physically cannot make that a 200. **But this 400 leaks nothing about email
  > existence:** it is a **framework-level, app-wide** response (the identical 400 a client
  > gets POSTing garbage to `/api/telemetry` or any JSON route), so it does **not** distinguish
  > a new vs existing email and does **not** even reveal that *this* path is special (the route
  > name already does). The anti-enumeration invariant — *email-existence does not leak* —
  > therefore **stays intact**. **Decision: ACCEPT-RISK** on the framework malformed-JSON 400
  > (owner: security; rationale: not an email-existence oracle; the route is a publicly-known
  > path; suppressing it would require app-wide content-type-parser surgery — over-engineering
  > against a non-leak). The only *route-emitted* non-200 is a transport failure (DB 503/500,
  > §6). Justified in §5/§7.
- Anonymous `SELECT` on the table returns **0 rows** (RLS proof, §7). NOTE (B4, honest
  framing): RLS here is a **linter gate + future-BYPASSRLS guard**, not row-level
  isolation. The **real** access boundary is the GRANT layer (anon/authenticated revoked,
  no schema USAGE). We do **not** claim per-row PII isolation from RLS.
- **Lawful basis = explicit consent (STOP-2).** No row is written without a server-validated
  `consent === true`. The row records **`consent_at`** (when) and **`privacy_version`**
  (which notice). Consent is **withdrawable** (= erasure, below).
- **Erasure exists on day one** (STOP-2): the operational role holds a `DELETE` grant and
  there is a one-line parameterized erasure path + operator runbook. PII is **not**
  collected without an exit.
- **Retention = 12 months, then auto-erase (STOP-2).** A pg-boss cron
  `access-request.retention-sweep` `DELETE`s rows older than 12 months. The manual erasure
  path is the day-one floor; the sweep is the standing automation on top.
- Secrets (`WAITLIST_NOTIFY_EMAIL`, `RESEND_API_KEY`) live in env/secrets, never in
  repo or logs.

---

## 2. Back-of-Envelope

### Submission volume
A soft gate on a pre-launch landing page. Realistic envelope for an Albania-market
single-product launch:

| Scenario | Submits/day | Submits/min peak | Notes |
|----------|-------------|------------------|-------|
| Steady state | 10–50 | < 1 | organic landing traffic |
| Launch spike / PR mention | 500–2,000 | 20–60 (bursty) | the case to design for |
| Adversarial (bot flood) | unbounded | capped by rate-limit | honeypot + per-IP limit absorb |

**Design target: comfortably absorb a 2,000/day spike (≈ peak 1/sec sustained,
60/min bursts) with zero operator pager noise.** This is a trivial write workload —
the risk is *abuse volume + PII surface*, not throughput.

### Rate-limit numbers (chosen) — B2 FIX (real client IP required)
**Precondition the original draft missed:** Fastify is constructed with **no
`trustProxy`** (`server.ts:138-142`), so `request.ip` is the **Fly edge proxy socket**,
identical for every external client. A per-route `max: 5/min` keyed on `request.ip`
would be a **single global bucket of 5/min for the whole planet** (B2 break A) — it would
429 legitimate users during the very launch spike we design for. And `ip_hash =
sha256(proxyIP)` would be a useless constant (B2 corollary).

**Fix — a managed `keyGenerator` on this route only (R2-2 hardened: `Fly-Client-IP` ONLY,
no spoofable XFF trust):**

```
function clientIp(req): string {
  // PROD: Fly injects Fly-Client-IP — a single value the Fly edge SETS and OVERWRITES on
  // every request, so a client cannot spoof it (unlike X-Forwarded-For, which the client
  // controls). This is the ONLY trusted source. We deliberately do NOT read X-Forwarded-For
  // here: trusting XFF[0] (the R1 draft) lets an attacker send a random XFF per request →
  // unbounded rate-limit keys + attacker-chosen ip_hash (hash-flood). Removed.
  const fly = req.headers['fly-client-ip'];
  if (typeof fly === 'string' && fly) return fly;
  // PROD with no Fly-Client-IP (proxy misconfig / bypass): FAIL CLOSED to a single shared
  // bucket — degrade, never fall through to a spoofable header. Boot warn fires (below).
  if (process.env.NODE_ENV === 'production') return 'no-fly-client-ip';
  // NON-PROD only (local/staging without Fly): request.ip is the socket peer — deterministic
  // and NOT client-controlled. Never the XFF value.
  return req.ip;
}
```

- The same `clientIp(req)` value feeds **both** the rate-limit key **and** `ip_hash =
  sha256(clientIp).slice(0,16)`, so the forensic column hashes the **real** client IP.
- **Why `Fly-Client-IP` is trustworthy:** it is documented Fly edge behaviour — the proxy
  **sets and overwrites** it on every inbound request, so a client-injected value never
  survives to the app (this is the structural difference from `X-Forwarded-For`, which the
  client *can* pre-populate and the proxy *appends* to). Trusting only `Fly-Client-IP`
  closes R2-2 / B2-break-B: there is no spoofable input left in the key path.
- **Boot-assert (proof obligation):** on the first real prod request the route asserts
  `Fly-Client-IP` was present; if it is ever absent in prod, a one-time ops warn fires
  ("access-request: Fly-Client-IP absent — rate-limit degraded to shared bucket") so the
  degrade is **visible, not silent**. The fail-closed shared-bucket means a Fly config
  change throttles everyone (safe, noticed) rather than silently re-opening the spoof hole.
- **R2-2 contrast with the rest of the repo:** `websocket.ts:118` reads raw
  `x-forwarded-for` *for a log line only* — not a security key; `Fly-Client-IP` appears in
  **zero** files today, so this route is the first to read it. That is fine — it is the
  *correct* header; the absence of precedent is why we add the boot-assert. We do **not**
  flip global `trustProxy` (would change `request.ip` for ~40 routes — defer-flag).
- **Per-IP: 5 requests / minute** (`config.rateLimit.max = 5, timeWindow = '1 minute',
  keyGenerator: clientIp`). A human submits once; 5/min tolerates a fat-finger
  double-tap + retry, caps a single real IP to ≤ 7,200/day. Mirrors the tight per-route
  limits on `/onboarding/start` (`max:3`) and `/auth/refresh` (`max:5`).

> **DEFER-FLAG (separate, NOT in this scope — R2-2 confirms these stay deferred):**
> `telemetry.ts:47` (`hashIp(request.ip)`, `rateLimit:false`) and `otp.ts:36`
> (`keyGenerator: req.body?.phone || req.ip`) carry the **same latent `request.ip`=proxy
> bug**. otp is partly shielded (keys on phone first); telemetry's `ip_hash` is currently a
> constant. This route does the **right** thing (`Fly-Client-IP`-only, no spoofable XFF);
> those two are **not** fixed here. Owner: platform/security. A future fix is the same
> hardened `clientIp()` helper (`Fly-Client-IP`-only — **not** the R1 XFF version) or a
> vetted global `trustProxy: <Fly-hop-count>`.

### Table size / year
Row ≈ 0.3 KB (email + 16-char ip_hash + small text cols + `consent_at` timestamptz +
short `privacy_version`; **`user_agent` dropped** per Counsel minimization). Even a
sustained 50/day genuine + heavy dedup → **< 20k rows/year ≈ < 6 MB/year**, and the
**12-month retention sweep** caps the steady-state table at ≈ one year of rows. With
`ON CONFLICT` dedup on email, repeat submits add **zero** rows. Indices: PK +
unique(email) + `created_at` for ops listing + `(notified_at IS NULL)` partial for the
notify sweep. No partitioning, no archival needed.

> **Consent columns — minimization check (STOP-2).** Two columns are the minimum to
> evidence lawful consent: **when** (`consent_at`) and **to-what** (`privacy_version`,
> the notice the user agreed to — needed because the notice text/retention can change and
> a stored boolean alone cannot prove *what* was consented to). We do **not** add a
> separate `consent boolean` column: a present `consent_at` *is* the proof of consent
> (NULL would never occur because the route rejects `consent !== true` before INSERT), so
> a boolean would be redundant. Two columns, no more.

### Email volume + Resend cost — B7 FIX (one aggregated alert, not a pager-storm)
A new row enqueues a notify job. Email volume = distinct-new-emails/day. An aggressive
launch → ≤ 2,000 distinct/day. Resend free tier = 100/day; paid $20/mo = 50,000/month.
**A launch spike exceeds the free 100/day cap**, and the naive design (one job per new
row, each retrying 5× over ~15min on a 4xx daily-cap rejection) would emit **hundreds of
exhausted-retry ops alerts/day** to Telegram-ops — alert-fatigue, the opposite of the §2
goal. **Fix (B7):**
- **Per-notify is best-effort and does NOT alert on its own exhaustion.** A single
  notify job that exhausts retries sets nothing and logs a non-PII warn; it does **not**
  page. (Removes the per-email pager-storm at the source.)
- **The reconciliation sweep (now in scope, §5/§6) is the single aggregated signal.** It
  runs on a low cadence (e.g. every 15 min) and, if it finds `notified_at IS NULL` rows
  **older than a grace window**, emits **one** coalesced ops alert: *"N access requests
  un-notified (oldest Xm). Check dashboard list."* One signal regardless of whether 1 or
  900 emails are backed up. This is the boring rate-limited-alert pattern.
- A Resend daily-cap rejection therefore manifests as `notified_at IS NULL` rows that
  the operator reads in bulk via `status='new'` — **not** data loss, **not** 900 pages.
- **Present-but-invalid `RESEND_API_KEY`** (typo'd secret): the EmailAdapter performs a
  cheap **one-time boot-time validation ping** (or first-failure classification) and, on
  a 401, emits **one** ops alert "email channel misconfigured (auth)" and the adapter
  reports degraded — so a bad key surfaces once at boot, not as a silent per-job 401
  storm. (B7 second half.)

### Retry budget (pg-boss `access-request-notify`)
- `retryLimit: 5`, `retryBackoff: true`, `retryDelay: 30s` → spread ≈ 30s, 60s, 120s,
  240s, 480s ≈ **~15 min total** before exhaustion. Exhausted → ops alert (Telegram-ops),
  not a user-facing error. Job is small (`{requestId}`), so retry storage is negligible.

### Connection budget (unchanged)
No new pool. Notify worker reads via the existing operational pool (`max: 8`,
`db/src/index.ts:20`) using a single `WHERE id=$1` point-read. Net new steady
connection pressure: ~0.

---

## 3. Options & Tradeoffs

### Option set A — How to write a non-tenant table under RLS FORCE

The repo has **two coexisting canonical patterns** for non-tenant tables; I verified
both in source:

- **Pattern P1 — RLS ENABLE+FORCE, no permissive policy, app role is BYPASSRLS, API
  roles (`anon`/`authenticated`) hard-revoked.** This is the *current* end-state for
  `users`, `auth_refresh_tokens`, `ops_worker_heartbeat`
  (`1780421100065_lockdown-nontenant-api-surface.ts:26-31`). The writer is a session/
  operational pool role with BYPASSRLS; FORCE+no-policy means deny-by-default to
  everyone else and the linter is satisfied.
- **Pattern P2 — RLS ENABLE+FORCE + a single permissive `FOR ALL USING(true)` policy.**
  Used for `ops_worker_heartbeat` policy (`1780691408625_ops-heartbeat-policy.ts`) so a
  **NOBYPASSRLS** operational role (`deliveryos_operational_user`,
  `1790000000015_operational-pool-role.ts:19`) can still operate the row.
- **Pattern P3 — non-tenant whitelist, RLS *not* forced** (`analytics_events`,
  `1790000000012_analytics-events.ts`), written by the operational pool from a public
  anon route (`telemetry.ts:54`). Explicitly documented as acceptable *because the data
  is non-PII*.

Decision driver: **`access_requests` stores PII (email).** P3 is therefore disqualified
on the PII red line — we must FORCE RLS. Choice is P1 vs P2, and it hinges on *which
pool role writes the row and whether that role bypasses RLS.*

| Concept | How write succeeds under FORCE | Tradeoff | Verdict |
|---|---|---|---|
| **(A1) BYPASSRLS app role, no policy** (P1) | Writer role has BYPASSRLS, so FORCE is moot for it; everyone else denied. | Cleanest for FORCE+linter. **But** the public route uses the *operational* pool, which is the **NOBYPASSRLS** `deliveryos_operational_user` (guardrail in `db/src/index.ts:32` literally crashes if it ever connects as superuser/BYPASSRLS). So under the operational pool, an INSERT with no policy would be **denied**. | Rejected for this route's pool. |
| **(A2) FORCE + single ops policy** (P2) | NOBYPASSRLS operational role passes the permissive `FOR ALL USING(true) WITH CHECK(true)` policy; `anon`/`authenticated` are revoked at the GRANT layer so they never reach the policy at all. | One extra policy object. Matches exactly how `ops_worker_heartbeat` is operated by the same NOBYPASSRLS pool. | **CHOSEN.** |
| **(A3) SECURITY DEFINER function** | A `SECURITY DEFINER` insert function owned by a privileged role. | Over-engineered: a function wrapper for a single-table insert, plus a new SQL surface to audit. Violates "boring & proven" and "schema rich, runtime minimal." | Rejected (over-engineering). |

**Decision A → A2.** Reproduce the `ops_worker_heartbeat` pattern exactly: `ENABLE`+`FORCE`
RLS, one `FOR ALL USING(true) WITH CHECK(true)` policy named `allow_ops_access_requests_all`,
and **hard-revoke** `anon`/`authenticated`/`service_role` GRANTs + (belt-and-suspenders)
the schema-level revoke from `1780421100065` already covers future tables via
`ALTER DEFAULT PRIVILEGES`. The public route writes through the **operational pool**
(NOBYPASSRLS), consistent with `telemetry.ts`.

> Why the policy is `USING(true)` and still safe: the operational role is the *only*
> role that ever has table GRANTs (anon/authenticated are revoked and have no schema
> USAGE — `1780421100065:51`). RLS FORCE silences the linter and prevents accidental
> future BYPASSRLS leakage; the GRANT layer is the actual access boundary. This is the
> identical, already-shipped reasoning for `ops_worker_heartbeat`.

> **B4 — honest framing (accept-risk, not a fix).** A reviewer correctly noted that
> `USING(true)` on a **PII** table gives **zero row-level containment**, and that the
> single operational `pool` (`server.ts:247`) is shared by ~40 routes — so any SQLi/logic
> flaw in *any* route on that pool can `SELECT *` the email list. This is true. The
> `ops_worker_heartbeat` analogy holds for the *write mechanics* (NOBYPASSRLS role must
> pass a permissive policy) but **not** for the *isolation claim* (heartbeat is non-PII
> infra state; this is PII). I therefore state plainly:
> - **RLS contributes no PII isolation here.** Its only jobs are (a) satisfy the FORCE
>   linter and (b) block a future accidental BYPASSRLS role from silently reading rows.
> - The **real** boundary is the GRANT layer, which is **app-wide**, not route-scoped.
>
> **Could we narrow it?** A dedicated lower-privilege role for this one INSERT (own pool,
> `INSERT`-only, no `SELECT`) would give real containment for *writes* — but the **notify
> worker and the erasure path must `SELECT`/`DELETE`**, and the ops list must `SELECT`,
> so the email is readable by the operational pool regardless. A separate write-only role
> would add a **second pool + connection-budget pressure + a new role to provision/audit**
> to protect a single anonymous-INSERT route, while the read surface (the actual PII
> exposure) stays on the shared pool. That is **over-engineering** against "schema rich,
> runtime minimal" for a ≤20k-row, single-column-PII table.
> - **Decision: ACCEPT-RISK.** Keep A2 (shared operational pool, `USING(true)`), document
>   that the boundary is GRANT-layer + the standard SQLi defenses (parameterized queries
>   everywhere — already the repo norm), **not** RLS row isolation. Owner: security.
> - **Compensating controls already in scope:** claim-check (email never in queue/logs),
>   `ip_hash` not raw IP, day-one erasure path, PII column count minimized to 1 (email).

### Option set B — Sync-in-handler vs claim-check-via-queue for the notification

| Concept | Mechanism | Tradeoff | Verdict |
|---|---|---|---|
| **(B1) Synchronous Resend call in the HTTP handler** | After INSERT, `await resend.send(...)` before replying. | Couples submit latency + success to a 3rd-party network call. Resend slow/down → either we block the user behind a 5s timeout (bad UX, and tempts a rollback) or we swallow it inline (then where do retries live?). PII (email) is in the hot request path's error logs risk. No durable retry. | Rejected. |
| **(B2) Claim-check via pg-boss queue** | INSERT → on new row, `enqueue('access-request-notify', { requestId })`. Worker reads the row by id, sends email, sets `notified_at`. | Submit latency = one INSERT + one tiny enqueue. Email decoupled, durably retried by pg-boss with backoff. **Zero PII in the queue** (claim-check: payload is just the id; the email is re-fetched by the trusted worker from the DB). Best-effort contract is structurally enforced — the HTTP path cannot fail on Resend. | **CHOSEN.** |

**Decision B → B2 (claim-check).** This is what the prompt mandates, and it is the
correct call: it is the only option that simultaneously satisfies *best-effort email*,
*zero-PII-in-queue*, *durable retry*, and *constant submit latency*. The repo already
runs every notification through pg-boss workers (`notify.dispatch` etc.), so this is
the boring, proven path.

> Enqueue-failure subtlety (handled in §5): if the enqueue itself throws (queue down),
> we **do not** fail the submit — the row is already durably written, which is the
> source of truth. The **reconciliation sweep (now in scope)** re-enqueues `notified_at
> IS NULL` rows, so a missed notification self-heals. The submit's success contract
> depends only on the INSERT.

> **B5 — enqueue is off the critical path.** To kill the timing oracle (new path =
> INSERT + enqueue; duplicate path = INSERT only → measurable latency delta that leaks
> existence), the handler **replies first, then fire-and-forgets the enqueue** in a
> detached continuation (`reply.send(...)` returns, then `void enqueue(...).catch(...)`).
> The response latency is therefore **INSERT-only on every path** — new, duplicate, and
> honeypot are timing-indistinguishable. A lost enqueue (process killed before the
> detached send runs) is caught by the sweep (B3). See §5/§7.

> **B3 — transactional gap, decided.** A true transactional outbox (enqueue inside the
> INSERT tx) is **not** available: the repo's enqueue is a plain `boss.send`
> (`queue-provider.ts:42`), not a DB-row outbox, so INSERT+enqueue cannot be atomic as
> built. Rather than build an outbox table (over-engineering for this volume), we choose
> the **reconciliation sweep** (option (b)): a tiny pg-boss cron that re-enqueues
> `notified_at IS NULL AND created_at < now()-grace`. This is **in scope** (it is cheap,
> and it simultaneously closes B1 residue, B3, B5's lost-enqueue case, and feeds B7's
> single aggregated alert). Rejected: outbox (over-eng), bare accept-risk (leaves a real
> silent-loss path with no v1 recovery, since the admin UI is deferred).

---

## 4. Data / Migrations

### Migration A: `1790000000041_access-requests.ts` (forward-only, atomic)

Next free number after head `1790000000040_order-tip`. DDL (parameterless, runs under
the migration superuser role):

```sql
CREATE TABLE access_requests (
  id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email           text NOT NULL UNIQUE,          -- stored trim+lower (normalized in code)
  source          text,
  locale          text,                          -- app locales are 'sq'|'en' (see note)
  status          text NOT NULL DEFAULT 'new',   -- new|invited|declined (manual transitions)
  ip_hash         text,                          -- sha256(realClientIp).slice(0,16), NEVER raw IP
  -- ── Consent capture (STOP-2: lawful basis = explicit consent) ──
  consent_at      timestamptz NOT NULL,          -- when the user consented (set in handler)
  privacy_version text        NOT NULL,          -- which privacy notice was consented to
  -- user_agent DROPPED (B-Counsel minimization: no named abuse-defense use; ip_hash
  -- already covers per-IP forensics). Do not add it back without a stated purpose.
  notified_at     timestamptz,
  notify_attempts smallint    NOT NULL DEFAULT 0,  -- R2-9: bounded reconcile re-feed guard
  created_at      timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_access_requests_status_created
  ON access_requests (status, created_at DESC);   -- ops listing of status='new'
CREATE INDEX idx_access_requests_unnotified
  ON access_requests (created_at) WHERE notified_at IS NULL;  -- sweep predicate (B3/B7/R2-9: + notify_attempts cap)
CREATE INDEX idx_access_requests_created
  ON access_requests (created_at);                -- 12-month retention sweep predicate (STOP-2)

-- ── Non-tenant, PII-bearing: ENABLE + FORCE RLS, single ops policy (Pattern A2) ──
-- NOTE (B4): RLS here is a linter/BYPASSRLS guard, NOT row isolation. The GRANT layer
-- below is the real boundary. See §3 B4 accept-risk.
ALTER TABLE access_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE access_requests FORCE  ROW LEVEL SECURITY;

CREATE POLICY allow_ops_access_requests_all ON access_requests
  FOR ALL USING (true) WITH CHECK (true);

-- ── Remove from Data API perimeter (mirror 1780421100065) ──
REVOKE ALL PRIVILEGES ON TABLE access_requests FROM anon, authenticated, service_role;
-- anon/authenticated already lack USAGE on schema public (1780421100065:51); future-table
-- default privileges already revoke anon/authenticated (1780421100065:37-44).

-- ── Grant the operational (NOBYPASSRLS) role exactly the DML it needs ──
-- DELETE included (STOP-2): a day-one erasure path must exist for a PII store.
GRANT SELECT, INSERT, UPDATE, DELETE ON access_requests TO deliveryos_operational_user;
```

Notes:
- **DELETE granted (STOP-2 fix).** A PII store collected on day one must have an erasure
  exit on day one. The grant + the single parameterized erasure statement
  (`DELETE FROM access_requests WHERE lower(email)=lower($1)`) + the operator runbook
  (§8) satisfy a GDPR right-to-erasure *today*, not at the deferred Stage-30. (Stage-30
  later automates retention; the manual path is the day-one floor.)
- **Consent columns are `NOT NULL` (STOP-2; R2-8 structural gate).** The handler validates
  `consent` via a strict `z.literal(true)` parse *before* INSERT (R2-8 — not a hand
  `if (body.consent)` that would let `"true"`/`1` through) and only then supplies
  `consent_at = now()` + `privacy_version = <current notice version constant>`. A failed
  literal-`true` parse takes the **silent uniform-200 honeypot path** (R2-3 — no INSERT, no
  `consent_at`), so a row can never exist without a structurally-validated consent. The
  `NOT NULL` constraint is a belt-and-suspenders DB guarantee; the **load-bearing** gate is
  the `z.literal(true)` parse (the `NOT NULL` alone would *not* have caught a truthy-string
  slip, because the handler always sets `consent_at` once it reaches INSERT — R2-8).
- **Consent × idempotency (B8 interaction).** `INSERT … ON CONFLICT (email) DO NOTHING`
  means a **duplicate** submit does **not** update `consent_at`/`privacy_version` — the
  row keeps the consent evidence from the *first* submit. This is correct and intentional:
  the first consent is the lawful record; we do not silently overwrite it (and a duplicate
  submit still re-affirms consent client-side, but mutating the stored timestamp would be
  a needless write and would muddy the audit trail). If a user re-consents under a newer
  `privacy_version`, that is a deliberate future feature (re-consent flow), out of scope.
- `email text NOT NULL UNIQUE` is the idempotency anchor. Normalization (trim+lower)
  happens in the handler *before* INSERT so the unique index dedups case/whitespace
  variants. We do **not** use a functional `lower(email)` index because the stored value
  is already canonical — simpler and matches how the app reasons about the column.
- **Locale column mismatch flagged (NOT fixed here):** the prompt spec says `'al'|'en'`,
  but the app's actual locale enum is `'sq'|'en'|'uk'` (`packages/ui/src/lib/i18n.ts:1`;
  Albanian = `sq`). The column is free `text` (not a CHECK enum) precisely so we store
  the *real* app locale value (`'sq'`/`'en'`) without a migration coupling. Decision:
  store the live locale string; do not invent an `'al'` value the UI never emits.
- **`down()` (B10).** Forward-only convention: `down()` is destructive-only and
  `DROP TABLE access_requests` would take the PII with it. We keep the repo convention.
  The B10 concern (rollback *without* drop leaves an un-erasable PII puddle) is moot now
  that DELETE is granted day one — the erasure runbook works whether or not the table is
  dropped. Accept-risk: rollback PII handling is covered by the same runbook.

### Migration B: `1790000000042_access-request-notify-queue.ts` — B1 FIX (durable queue)

**This is the B1 fix and it is mandatory.** The original draft chose "rely on runtime
idempotent `createQueue` at boot." That path is **structurally blocked**: pg-boss v10
backs each queue with its own partition table created via DDL, and
`1790000000009_pgboss-revoke-runtime-ddl.ts:20` does `REVOKE CREATE ON SCHEMA pgboss FROM
PUBLIC`. The runtime operational role is NOBYPASSRLS with **no CREATE**, so
`createQueue('access-request.notify')` for a genuinely-new queue **throws**, and the boot
loop **swallows it** (`server.ts:290` `.catch(console.warn)`) → 100% of notifications
silently lost, with no exhausted-retry alert (nothing ever enqueues).

The queue must therefore be pre-created **under the migration role**, exactly as
`1790000000011` pre-creates `APP_QUEUES`. We add a new forward-only migration that opens
pg-boss under the migration connection and creates the one queue + its partition table:

```ts
// 1790000000042_access-request-notify-queue.ts  (mirrors 1790000000011's pattern)
const MIGRATION_DB_URL = process.env.DATABASE_URL_MIGRATIONS || process.env.DATABASE_URL_SESSION;
export async function up(pgm) {
  pgm.noTransaction();
  await pgm.db.query('COMMIT');                       // make prior pgboss schema visible
  const boss = new PgBoss({ connectionString: MIGRATION_DB_URL, schema: 'pgboss',
                            application_name: 'access-request-queue-bootstrap', max: 2, migrate: true });
  boss.on('error', (e) => console.error('[access-request-queue] pg-boss error:', e));
  await boss.start();
  await boss.createQueue('access-request.notify');            // notify (per-row email)
  await boss.createQueue('access-request.reconcile');         // notify-gap sweep cron (B3)
  await boss.createQueue('access-request.retention-sweep');   // 12-month auto-erase cron (STOP-2)
  await boss.stop({ graceful: true, wait: true });
  // re-apply DML grants to the runtime role on the new partition table (idempotent)
  await pgm.db.query(`DO $g$ DECLARE t text; BEGIN
    FOR t IN SELECT table_name FROM information_schema.tables
             WHERE table_schema='pgboss' AND table_type='BASE TABLE' LOOP
      EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON pgboss.%I TO PUBLIC', t);
    END LOOP; END $g$;`);
  await pgm.db.query('BEGIN');                        // restore node-pg-migrate bookkeeping tx
}
export async function down() { /* no-op: 1790000000006.down drops pgboss CASCADE */ }
```

- **Why durable now:** the partition table exists after migrate, created by the role that
  *can* DDL. At boot, runtime `createQueue('access-request.notify')` becomes a **no-op**
  (queue already present) instead of a swallowed throw. `boss.send(...)` lands in a real
  queue; the worker drains it; `notified_at IS NULL` rows are reconciled by the sweep.
  The §6 "jobs accumulate durably" invariant is now **true**.
- **`APP_QUEUES` in `1790000000011`**: also add **all three** —
  `'access-request.notify'`, `'access-request.reconcile'`,
  `'access-request.retention-sweep'` — to that array (verified shape: the array is a flat
  `readonly string[]` of dotted names, `1790000000011:57-83`). This does **not** re-run on
  prod (it is applied) — its only purpose is **fresh-provision parity** so the CI
  fresh-provision smoke test creates the queues on a clean DB without needing Migration B.
  Both are belt-and-suspenders; **Migration B is the load-bearing fix for
  already-provisioned environments (prod/staging).**
- Idempotent: `boss.createQueue` is a no-op if the queue exists, so re-running Migration B
  on a DB that already has it is harmless.

---

## 5. Consistency + Idempotency

### Consent gate (STOP-2; R2-3 + R2-8 REVISED) — structural Zod, silent uniform-200 on fail
A **dedicated strict Zod parse for the non-email control fields** runs *before* the
uniform-200 email contour. It is the **structural** lawful-basis gate (R2-8 — replaces the
slip-prone hand `if`):

```
const ControlFields = z.object({
  consent: z.literal(true),                 // R2-8: "true"/1/missing all FAIL structurally
  website: z.string().max(0).optional(),    // honeypot: any non-empty value fails
  locale:  z.string().optional(),
});
// email is parsed SEPARATELY and leniently (never gated) so email-existence stays un-enumerated.
```

- **R2-3 (no 400).** If `ControlFields` parse **fails** (consent not literal `true`, or
  honeypot filled) the route takes the **silent uniform-200 honeypot path**: same
  `200 {ok:true}`, an equivalent cheap no-op for timing parity, **no INSERT, no
  `consent_at`, no enqueue**. There is **no `400`** — the earlier consent-400 was a
  cheapest-path DoS-amplifier + a status/latency route-fingerprint (R2-3); folding it into
  the uniform-200 contour removes both. The route now emits **no non-200 for any
  well-formed-or-malformed body**.
- **Lawful basis preserved.** A row is **only** written when `consent: z.literal(true)`
  passes; *no row = no processing*, so silently dropping a no-consent body collects
  nothing. The `z.literal(true)` is the authority — `"true"`, `1`, `"false"` (truthy
  string), and missing all fail structurally, which a hand `if (body.consent)` would not
  have caught (R2-8). The frontend still disables the button until ticked (UX), but the
  server no longer depends on the client and no longer leaks a distinct branch.
- On a passing parse the handler sets `consent_at = now()`, `privacy_version =
  PRIVACY_NOTICE_VERSION` (a build-time constant; **R2-6** content-hash CI test fails if the
  notice prose changes without bumping it), then proceeds to the INSERT path below.

### Repeat submit (same email, sequential)
`INSERT ... ON CONFLICT (email) DO NOTHING RETURNING id`. New email → `RETURNING`
yields a row → enqueue. Existing email → zero rows returned → **no enqueue**. Response
is `200 {ok:true}` either way (anti-enumeration). Idempotent by construction.

### Race (two identical emails concurrently)
The `UNIQUE(email)` constraint serializes the two INSERTs at the DB. Exactly one wins
the `RETURNING id`; the loser's `ON CONFLICT DO NOTHING` returns zero rows → only one
enqueue. No double email. No app-level lock needed — Postgres is the arbiter. (This is
the same race-safety the repo relies on for `telegram_login_tokens` single-use flips,
`auth.ts:206`.)

### At-least-once job delivery → idempotent notify — B8 FIX (claim-before-send + tolerate missing row)
pg-boss is at-least-once; a job may run twice (worker crash after send/before ack; or
visibility-timeout hand-off to a second worker). The worker is made idempotent by a
**claim-before-send compare-and-swap**, and is **tolerant of a missing row** (erasure):

```
-- 1. CLAIM: atomically flip notified_at; only the winner proceeds to send.
UPDATE access_requests
   SET notified_at = now()
 WHERE id = $1 AND notified_at IS NULL
RETURNING email, locale;            -- claim-check re-fetch happens HERE, in the same CAS
```

Worker logic:
1. Run the CAS UPDATE above.
   - **0 rows + row exists with `notified_at NOT NULL`** → already claimed/sent by a
     prior delivery → **ack, send nothing** (no duplicate email).
   - **0 rows + row absent** (erasure landed between enqueue and pickup — B8.1) →
     **ack as a no-op, do NOT throw.** A `SELECT count` distinguishes the two; or simply:
     "0 rows updated → nothing to do → ack." The worker **must never crash on a missing
     row.** This is recorded as a hard worker obligation so the deferred Stage-30 erasure
     cannot turn into a latent crash (the original draft's "no scenario where the worker
     reads a missing row" was only true *because* erasure was deferred — fragile).
   - **1 row** → we won the claim → proceed to send.
2. Send the email (`EmailAdapter`, `AbortSignal.timeout(5000)`).
   - **Send fails** → we already flipped `notified_at`; we must **roll it back**:
     `UPDATE ... SET notified_at = NULL WHERE id=$1` and **throw** so pg-boss retries.
     (Claim-then-release-on-failure keeps the CAS honest: a failed send must not leave a
     row marked notified.) Net effect: send precedes nothing user-facing; the row's
     `notified_at` truthfully reflects a *successful* send.

This makes double-delivery **at-most-one-email** even in the two-worker race (only one
CAS wins; the loser sends nothing) — strictly better than the original "send-then-guard"
ordering, which the Breaker correctly flagged as still allowing two sends. Trade: a send
that succeeds-but-fails-to-ack re-runs and finds `notified_at NOT NULL` → no second send.
The CAS is the cheap, boring idempotency key.

> Worth noting the CAS does the **claim-check re-fetch** in the same statement
> (`RETURNING email, locale`), so the email is read from the trusted DB by the worker,
> never carried in the queue payload (`{ requestId }` only) — claim-check intact.

### Submit vs notify ordering — B3/B5 FIX (reply-first, sweep-backstopped)
The row is committed before enqueue, and (B5) the **enqueue is fired *after* the HTTP
reply** (fire-and-forget). Two consequences:
- A crash between COMMIT and the detached enqueue loses the *notification*, not the
  *data*. This is **recovered by the reconciliation sweep** (now in scope — B3 fix):
  a pg-boss cron `access-request.reconcile` (e.g. every 15 min) re-enqueues
  `notified_at IS NULL AND created_at < now() - interval '<grace>' AND notify_attempts <
  $cap` (**R2-9 bounded guard:** rows whose notify worker has exhausted retries `$cap`
  times — e.g. a permanently-bad address — stop being re-fed and surface as stuck in the
  single aggregated alert, instead of an infinite re-enqueue treadmill). This is **not** a
  defer-flag anymore; it is the v1 recovery path (the admin dashboard stays deferred, so
  the sweep is the only automated backstop — that is exactly why it had to come in-scope).
- **R2-1 — the reconcile cron PROVABLY schedules.** Its `boss.schedule(...)` write to
  `pgboss.schedule` runs under the same runtime operational role that already drives **16
  live schedulers** on prod (`anonymizer-retention.ts:27`, `server.ts:452`,
  `courier-cron.ts:21,26`, `backup/index.ts:40-46`, `reconciliation.ts:41`,
  `settlement-cron.ts:26`, `dwell-monitor.ts:26`, …). `pgboss.schedule` DML is grant-clean
  (`0009:31-36`/`0011:128-132` loops + `0009:45`/`0011:139` `ALTER DEFAULT PRIVILEGES`). So
  the "lost enqueue self-heals" chain is no longer coupled to an unproven cron (R2-4 accept).
- The sweep also emits the **single aggregated B7 ops alert** when a backlog persists
  past the grace window.

### Retention TTL sweep (STOP-2) — `access-request.retention-sweep` cron
A second pg-boss cron auto-erases PII at the 12-month retention boundary. **Grounded in
the existing `anonymizer.retention` pattern** (`workers/anonymizer-retention.ts:21-28`):
a worker registered at boot that `boss.work` + `boss.createQueue` (no-op post-Migration B)
+ `boss.schedule(QUEUE, cron, null, { singletonKey })`, with a **`pg_try_advisory_lock`**
guard inside `run()` so only one instance fires per tick (mirrors the advisory-lock(4)
pattern at `anonymizer-retention.ts:33`).

> **R2-1 — `boss.schedule` durability is PROVEN, not assumed (12-month auto-erase will
> actually run).** The Breaker flagged that the retention cron's `boss.schedule` write to
> `pgboss.schedule` was never separately reasoned about. It is now: the **identical**
> runtime `boss.schedule` write under the **same** NOBYPASSRLS operational role already
> drives `anonymizer.retention` (and 15 other crons) on prod today — so `pgboss.schedule`
> is writable by the runtime role *by demonstration*, and the new cron inherits the proven
> path. `pgboss.schedule` DML is covered by the `0009`/`0011` grant-loops **and** by
> `ALTER DEFAULT PRIVILEGES … ON TABLES` (`0009:45`, `0011:139`), so it is grantable whether
> the table predates or postdates the loop — the Breaker's "schedule table postdates the
> grants" break is closed. **No new migration** is needed for the cron path (only the queue
> partition table from Migration B, already covered).
>
> **R3-1 — boot-safety of the schedule call (SUPERSEDES the R2-1 "copy anonymizer, no `.catch`"
> instruction).** The R2-1 plan said to copy `anonymizer-retention.ts:27`'s **un-`.catch`'d**
> `boss.schedule` "so a genuine failure surfaces to boot." The Breaker (R3-1) proved — and I
> re-verified against the working tree — that this **backfires** given the real boot topology:
> worker `.start()` calls are **bare top-level `await`s in `main()` with no surrounding
> try/catch** (`server.ts:401`), `fastify.listen()` is **after** them (`:905`), and `main()`
> is invoked **bare** (`:913`) so any reject becomes an `unhandledRejection` that the `:129`
> guard **logs-and-keeps-alive**. An un-`.catch`'d schedule throw therefore aborts `main()`
> **before `listen()` ever runs** → a **live process that serves no HTTP** (zombie: Fly sees
> a running machine failing its health check / hanging requests), and a brand-new queue's
> *first* schedule is the least-proven write in the system. That is strictly worse than the
> swallow it replaced. **Resolution (R3-1 FIX):**
> 1. **`.catch`-wrap the new cron's `boss.schedule`** (and the reconcile cron's) — copy the
>    `rates-refresh.ts:25` / `reconciliation.ts:41` **majority** shape, **not** the
>    `anonymizer` minority shape. A schedule throw is logged best-effort and **cannot poison
>    boot** (`fastify.listen` always runs; HTTP always comes up).
> 2. **Make failure VISIBLE via a fail-fast boot-assert, not via a poisoned boot.** Add a
>    dedicated post-`fastify.listen()` assertion that `SELECT`s `pgboss.schedule` for **both**
>    `access-request.retention-sweep` and `access-request.reconcile`; **if either row is
>    missing → `console.error(...)` + `process.exit(1)`** (clean non-zero exit) so Fly
>    restarts the machine and the **deploy shows red** — a *visible* fail-fast, the thing the
>    un-`.catch` was trying (and failing) to achieve. This is the boot-assert as a **real
>    runtime guard**, not merely a CI test. Order: `listen()` first (so the failed deploy
>    still answers `/livez` long enough to be observed) → assert → exit on miss.
> 3. **Do not put the 12-month-erase cron on the zombie path.** Because (1) guarantees HTTP is
>    up and (2) guarantees a missing schedule is a *clean restart*, the auto-erase is never
>    silently lost to a half-booted process: either it schedules, or the deploy fails loudly.
>
> **Defer-flag (separate, owner: platform):** the *existing* `anonymizer-retention.ts:27`
> (and any other un-`.catch`'d `boss.schedule` before `listen()`) carry the **same latent
> zombie pattern**. It has not bitten because those schedules have run on prod for months, but
> the topology is dirty. A separate ticket should `.catch`-wrap the legacy crons and/or add a
> `main().catch(() => process.exit(1))` at `server.ts:913`. **NOT widened into this scope** —
> the NEW worker is done correctly here; the legacy fix is its own change.
>
> **Proof obligation (revised, §"Proof obligations"):** after boot, assert both schedule rows
> exist in `pgboss.schedule` **and** assert the boot-assert path exits non-zero when a row is
> absent (inject a missing-schedule condition → expect `process.exit(1)`), proving the
> fail-fast guard is live, not aspirational.

```
-- run() body, under a distinct advisory lock id (e.g. pg_try_advisory_lock(5)):
DELETE FROM access_requests
 WHERE created_at < now() - $1::interval        -- $1 = ACCESS_REQUEST_RETENTION (default '12 months')
```

- **Cron cadence:** daily, low-traffic hour — reuse the anonymizer convention `'0 3 * * *'`
  via a config key `ACCESS_REQUEST_RETENTION_CRON` (default `'0 3 * * *'`), exactly like
  `ANONYMIZER_RETENTION_CRON` (`anonymizer-retention.ts:25`). The **window** is a separate
  config `ACCESS_REQUEST_RETENTION` (default `'12 months'`) so the retention number is
  parameterized, not hard-coded — and must equal the number stated in `/privacy`.
- **Idempotent + safe:** `DELETE … WHERE created_at < now()-interval` is naturally
  idempotent (already-erased rows are gone); the advisory lock prevents concurrent
  double-runs; it is a bounded set-based `DELETE` on an indexed predicate
  (`idx_access_requests_created`). No batching needed at < 20k rows/year, but a
  `LIMIT`-batch loop is a trivial future add if volume ever warrants (mirrors the
  anonymizer `BATCH_SIZE` ergonomics) — **not** built now (YAGNI at this scale).
- **Erasure × consent-withdrawal:** retention auto-erase **is** the standing mechanism
  for "consent expired"; the day-one manual `DELETE` script is the on-demand
  consent-withdrawal / right-to-erasure path. Both hit the same `DELETE` grant.
- **Worker tolerance:** a retention `DELETE` that removes a row mid-notify is already
  handled — the notify worker's CAS returns 0 rows on a missing row and **acks without
  throwing** (B8.1). The retention sweep introduces **no new** crash path: it is the same
  "row may vanish" tolerance B8 already designed for, now also triggered by the cron (not
  only by manual erasure). Recorded as a NEW-risk cross-check in §9.
- **Alerting:** the retention sweep logs a non-PII count (`access_request.retention.erased
  = N`); it does **not** page (routine housekeeping). A sweep *error* (e.g. lock contention
  forever, DB error) publishes to `WORKER_FAILED` like the anonymizer
  (`anonymizer-retention.ts:77`).

### Queue/worker registration (STOP-2 additions)
- `packages/shared-types/src/queue-names.ts` `QUEUE_NAMES` (dotted convention, verified
  `:1,:12,:25`): add `ACCESS_REQUEST_NOTIFY: 'access-request.notify'`,
  `ACCESS_REQUEST_RECONCILE: 'access-request.reconcile'`,
  `ACCESS_REQUEST_RETENTION_SWEEP: 'access-request.retention-sweep'`.
- New worker `apps/api/src/workers/access-request-retention.ts` (mirror
  `anonymizer-retention.ts` structure) registered at server boot alongside the other
  cron workers.

---

## 6. Failures + Degradation (failure-first)

Every external call gets timeout + graceful fallback; zero cascade into the user
response.

| Failure | Behavior | Rationale |
|---|---|---|
| **Resend down / 5xx / timeout** | Worker call wrapped in `AbortSignal.timeout(5000)` (mirrors `telegram.ts:32`). Failure → return non-delivered → pg-boss retry w/ backoff (≤5, ~15min). Submit already committed; **never rolled back**. | Email is best-effort by contract. |
| **Retries exhausted** | Job fails terminally → emit ops alert via the notification seam to Telegram-ops (event-style, like `ops.backup_failed` in `provider.ts:22`). Row stays `notified_at IS NULL`, visible in ops list. **No exception in the HTTP response** (the response already returned). | Operator still sees the request; just didn't get the push. |
| **Queue unavailable at enqueue** | Enqueue is **fire-and-forget after the reply** (B5); its `.catch` logs a **non-PII** warn (`{ requestId }` only). Submit already returned `200`. The **reconciliation sweep** (in scope) re-enqueues the `notified_at IS NULL` row. | Do **not** fail the submit on a notification-transport problem; sweep self-heals. |
| **Queue partition table absent at boot (B1)** | **Cannot happen** post-fix: the queue + partition table are pre-created by Migration B under the migration role. Runtime `createQueue` is a no-op. If a fresh env somehow lacks it, the sweep keeps re-enqueuing and its aggregated alert fires — visible, not silent. | B1 fix makes "jobs accumulate durably" actually true. |
| **Worker not started** | Jobs accumulate durably in the **real** pg-boss queue (created by Migration B); processed when the worker boots. No data loss. | At-least-once durability (now structurally valid). |
| **DB INSERT timeout** | Operational pool has `statement_timeout=10s` (`db/src/index.ts:30`) + `connectionTimeoutMillis=5000`. On timeout the handler returns a generic `503`/`500` with **no email echoed**. The visitor sees the form `error` state and may retry (rate-limit permitting). | The one case where we *do* surface failure — because the source-of-truth write itself failed. |
| **Honeypot filled** | Silent `200 {ok:true}`, **no INSERT, no enqueue, no log of the value** — but to preserve the B5 timing invariant the path performs an **equivalent cheap no-op** (so honeypot latency ≈ real latency; no fast-reject oracle). See §7. | Anti-bot + anti-enumeration. |
| **Malformed email (Zod)** | The route uses a **lenient self-parse** and returns `200 {ok:true}` even for an unparseable email (no INSERT) — never a global 400 (B5). The form does light client-side validation for UX, but the server never leaks a 400 that distinguishes input classes. | Anti-enumeration uniformity (B5). |
| **Missing/false `consent` (STOP-2; R2-3/R2-8)** | **Silent `200 {ok:true}`** via the honeypot path — `z.literal(true)` parse fails → equivalent cheap no-op, **no INSERT, no `consent_at`, no enqueue**. **No 400** (the consent-400 was a cheapest-path DoS-amplifier + route-fingerprint, R2-3). Lawful basis holds: no row without structurally-validated consent; *no row = no processing*. **R3-3a:** the *client* still renders `error`/retry (never false success) if its own sent body lacked real `consent===true`, so a legit consenter is never silently lost. | Lawful-basis enforcement *without* an email-existence branch; anti-enumeration intact (§5); legit-user-loss closed at client (R3-3a). |
| **Malformed JSON body (not valid JSON at all) — R3-3b** | Rejected by **Fastify's content-type parser → global handler → 400** (`server.ts:516`), **before** the route. The route cannot turn this into 200. **Accept-risk:** this 400 is framework-level/app-wide (same for every JSON route) → leaks **nothing** about email existence; not an enumeration oracle. The anti-enumeration invariant (email-existence non-leak) is intact (§1). | Framework boundary; honest reformulation, not an email-oracle (§1, ADR Decision-6). |
| **Retention sweep cron errors / lock held** | `pg_try_advisory_lock` miss → skip-this-tick (logs, retries next cron). Any DB error → publish `WORKER_FAILED` (like anonymizer), no user-facing surface. PII simply persists one extra day until the next successful tick — bounded, not lost. | Routine housekeeping; never blocks submits. |
| **Retention DELETE removes a row mid-notify** | Notify worker CAS returns 0 rows → **acks without throwing** (B8.1, already designed). No new crash path. | Erasure tolerance reused. |
| **Reconciliation sweep finds backlog** | Emits **one** coalesced ops alert (B7), re-enqueues un-notified rows. Never per-row paging. | Aggregated signal, no pager-storm. |

No failure here can cascade: the only blocking dependency on the user path is the single
INSERT; everything else is fire-and-forget after the reply, behind the durable queue and
the sweep.

---

## 7. Security + Tenant Isolation

### Why non-tenant is correct
An access request exists **before** any org/location/membership. There is no tenant to
scope it to — it is platform-level intake. This is structurally identical to
`auth_refresh_tokens` and `users` (cross-tenant identity/auth state): non-tenant,
RLS-forced, off the Data API perimeter.

### RLS proof (anon SELECT = 0 rows)
Three independent layers, any one sufficient:
1. `anon`/`authenticated` have **no `USAGE` on schema `public`** (`1780421100065:51`) →
   cannot even reference the table.
2. All privileges **revoked** for `anon`/`authenticated`/`service_role` in the migration.
3. RLS `ENABLE`+`FORCE` with a single ops-scoped policy; no permissive policy grants
   anon read.

Verification hook: extend `packages/db/scripts/verify-rls.ts` /
`apps/api/tests/phase5/rls-adversarial.test.ts` with an assertion that a connection as
`anon` returns 0 rows / permission-denied on `access_requests`. (Cited as the existing
RLS adversarial harness — proof obligation per the Mandatory Proof Rule.)

### Anti-enumeration — B5 FIX (no timing oracle, uniform 200)
Identical `200 {ok:true}` for new / duplicate / honeypot / malformed. The original draft
leaked existence via a **timing branch**: the new path did `INSERT + enqueue`; the
duplicate path did `INSERT` only → measurable latency delta = a new-vs-existing oracle.
Closed by:
1. **Enqueue is off the critical path** — fired *after* the reply, fire-and-forget
   (§5). Response latency is **INSERT-only on every path**.
2. **Honeypot does equivalent cheap work** before its uniform `200`, so it is not a fast
   reject distinguishable by latency.
3. **Malformed email** → lenient self-parse → uniform `200`, never a 400 (the global Zod
   compiler returns 400 at `server.ts`, so this route does its **own** body parse and
   never relies on the schema validator to gate — it always sends the same 200 shape). The
   email is parsed **separately** from the strict `ControlFields` gate (R2-8), so the
   structural consent/honeypot validation never leaks the email class.
4. **No-consent and honeypot both take the SAME silent `200` path** (R2-3): the consent-400
   is gone, so there is **no branch** — by status or by latency — that distinguishes a
   no-consent/honeypot/malformed body from a real new/duplicate submit. The route emits a
   single response shape for every body; the only non-200 is a DB transport failure.
5. No "email already registered" copy ever. Plus per-IP rate-limit (5/min, **`Fly-Client-IP`
   only** — R2-2) and honeypot.

### PII minimization
- **`ip_hash`** = `sha256(clientIp).slice(0,16)` where `clientIp` is the **real client
  IP** from **`Fly-Client-IP` only** (R2-2; §2) — **not** `request.ip` (= proxy) and
  **not** the spoofable `X-Forwarded-For` (that fallthrough was removed in R2-2, because an
  attacker-chosen XFF would make `ip_hash` attacker-controlled garbage / a hash-flood). In
  non-prod, `clientIp` degrades to the deterministic socket `request.ip`; in prod with no
  `Fly-Client-IP` it fails closed to a constant + boot warn (never to XFF). The hash is
  non-reversible; raw IP is never stored or logged. Named purpose: abuse forensics / per-IP
  rate-limit. (Kept; `user_agent` dropped — see below.)
- **`user_agent` DROPPED** (Counsel minimization): a fingerprinting-adjacent field with
  no named abuse-defense use. `ip_hash` already covers per-IP forensics. Removed from the
  schema (§4) and never collected.
- **Claim-check**: queue payload = `{ requestId }` only. The email never enters the
  queue, never enters job-failure logs.
- **No email in logs as plaintext** anywhere in handler or worker. Logs key on
  `requestId`.
- Secrets `WAITLIST_NOTIFY_EMAIL`, `RESEND_API_KEY` added to the Zod env schema in
  `packages/config/src/index.ts` as `.optional()` (so absence disables the email leg
  gracefully, like `TELEGRAM_BOT_TOKEN`), and supplied via Fly secrets — never in repo.

### Lawful basis = explicit consent (STOP-2, human decision)
- **Basis: consent (withdrawable).** No row without server-validated `consent === true`;
  the row stores `consent_at` + `privacy_version` as the consent evidence (§4/§5).
- **Withdrawing consent = erasure.** A withdrawal request is satisfied by the same
  day-one `DELETE` path below — there is no half-state "consent off but row kept."

### Right-to-erasure (STOP-2 FIX — exists on day one, automated at 12 months)
- The operational role has a **`DELETE` grant** (§4). The single parameterized erasure
  statement is `DELETE FROM access_requests WHERE lower(email) = lower($1)` (idempotent;
  0 or 1 row).
- **Operator runbook (day one):** an erasure / consent-withdrawal request → operator runs
  the one DELETE via the ops console / a tiny `pnpm` script
  `scripts/erase-access-request.ts <email>` that wraps exactly that statement on the
  operational pool. No admin UI needed for v1.
- **Standing automation (in scope, STOP-2):** the `access-request.retention-sweep` cron
  (§5) auto-erases rows older than **12 months** (`ACCESS_REQUEST_RETENTION`, default
  `'12 months'`). The manual path is the day-one floor; the sweep is the standing TTL.
- The worker tolerates a row deleted mid-flight by either path (B8.1: CAS 0 rows → ack
  no-op).
- **Lawful basis + retention + rights** are stated in the `/privacy` page the form links
  to (§8, §10), versioned by `privacy_version`. The basis (consent) and window (12 months)
  are **decided** (STOP-2 human ruling) — no longer open.

### Resend channel — B6 FIX (direct system-channel adapter, NOT the tenant dispatcher)
Resend is **not yet wired** as a notification channel (verified: only doc/frontend
references exist; `provider.ts` knows only `telegram | push | whatsapp`). The original
draft routed the ops email through `NotificationDispatcher` with a synthetic target
`{ channel:'email', address: WAITLIST_NOTIFY_EMAIL }`. **That breaks the seam contract**
(verified in source):
- `NotificationTarget.locationId: string` and `NotificationData.locationId: string` are
  **required, non-optional** (`provider.ts:5,:37`). An access request has **no location**.
- `writeAudit` inserts into `notification_outbox_audit` where
  `location_id uuid NOT NULL` (`1790000000007:16`; `audit.ts:38` passes
  `entry.locationId`). A tenant-shaped audit write for this email would either **throw on
  NOT NULL** (turning best-effort email into a job failure / retry storm) or be skipped
  (losing observability for the channel). Faking a sentinel `locationId` would pollute
  the tenant audit log with a non-tenant row.

**Fix — a thin, self-contained `EmailAdapter` invoked directly (no dispatcher, no
tenant audit):**
- New `EmailAdapter` (`apps/api/src/notifications/adapters/email.ts`) exposing a
  **system-channel** method `sendOps({ to, subject, text/html }): Promise<NotifyResult>`
  that calls Resend's REST API with `AbortSignal.timeout(5000)` and returns
  `{ delivered, reason, retryAfter }`. It does **not** implement the tenant
  `NotificationProvider.notify(target, event, data)` signature, so it never needs a
  `locationId` and never touches `notification_outbox_audit`.
- The `access-request.notify` worker calls `emailAdapter.sendOps({ to:
  WAITLIST_NOTIFY_EMAIL, ... })` **directly**. Observability for this channel lives in
  the **`access_requests` row itself** (`notified_at`) + the §8 counters
  (`access_request.notify.*`), **not** in the tenant audit table. This is the correct
  seam: a platform-level ops alert is **not** a tenant notification and must not borrow
  the tenant contract.
- Gated on `RESEND_API_KEY` presence (absent → `sendOps` returns
  `{ delivered:false, reason:'email-disabled' }`; submits still persist; sweep + bulk
  `status='new'` is the fallback).
- We do **not** add `'email'` to `NotificationTarget.channel` and do **not** add an
  `ops.access_request` `NotificationEventType` — keeping the tenant dispatcher unchanged
  avoids forcing `locationId`-shaped contracts onto a non-tenant channel and avoids
  breaking the existing telegram/push/whatsapp consumers. (If a *tenant* email channel is
  ever needed, that is a separate design that must answer the `locationId`/audit shape.)

> Why direct-call is the right "boring & proven" here: the dispatcher's value is
> tenant-scoped dedup/prefs/quiet-hours/audit — **none** of which apply to a single
> system ops alert. Reusing it would be reuse-for-reuse's-sake at the cost of breaking
> its invariants. A direct adapter call is simpler and contract-honest.

---

## 8. Operability

### Routing convention (decided)
Repo public routes are registered with **`/api/...` prefixes baked into the route file**
(e.g. `telemetry.ts` declares `/api/telemetry` internally and is registered with no
extra prefix, `server.ts:568`). The prompt asks for `POST /api/v1/access-requests`.
**Decision: register at `/api/access-requests`** to match the repo's actual convention
(no `/v1` segment exists anywhere — `auth`, `telemetry`, `customer`, `owner` are all
un-versioned under `/api`). Introducing a lone `/v1` would be an inconsistent, unproven
divergence. The route file declares its own full path `/api/access-requests` and is
registered like telemetry: `fastify.register(accessRequestRoutes, { db: pool, queue })`.
> Recorded as an explicit deviation-from-prompt decision in the ADR.

### Queue registration — B1 FIX (durable, not boot-best-effort)
Three synced touchpoints; the **migration is the load-bearing one**:
1. `packages/shared-types/src/queue-names.ts` `QUEUE_NAMES` (verified dotted convention,
   `:1,:12,:25`) → add `ACCESS_REQUEST_NOTIFY: 'access-request.notify'`,
   `ACCESS_REQUEST_RECONCILE: 'access-request.reconcile'`, **and**
   `ACCESS_REQUEST_RETENTION_SWEEP: 'access-request.retention-sweep'` (dotted, matching
   every existing queue name — *not* `access-request-notify`). All appear in the boot
   loop → `createQueue` (now a **no-op**, because…).
2. **Migration B `1790000000042` pre-creates the queue + partition table under the
   migration role** (§4). This is the actual fix for B1 — runtime `createQueue` cannot
   DDL (`REVOKE CREATE ON SCHEMA pgboss`, `1790000000009:20`), so the original
   "rely on runtime createQueue at boot" plan was **structurally blocked** and would have
   silently lost 100% of notifications. The migration also creates the
   `access-request.reconcile` cron queue.
3. `1790000000011 APP_QUEUES` → add both names for **fresh-provision parity** only (it
   does not re-run on prod). Belt-and-suspenders for the CI fresh-provision smoke test.

> Verification (proof obligation): a test asserting `pgboss` has the partition table for
> `access-request.notify` after migrations, and that `boss.send('access-request.notify',
> {requestId})` followed by a worker drain sets `notified_at`. A swallowed boot-warn is
> **not** acceptable proof.

### Observability (<1 min to see truth)
- **New requests**: `SELECT count(*) FROM access_requests WHERE status='new'` (indexed).
  The ops list query is `... ORDER BY created_at DESC` on the same index.
- **Un-notified**: `WHERE notified_at IS NULL` (partial index) surfaces email-delivery
  gaps. The **reconciliation sweep** queries exactly this and re-enqueues stale rows.
- **Metrics/logs** (zero PII): structured logs key on `requestId`, counters for
  `access_request.submitted`, `access_request.duplicate`, `access_request.honeypot`,
  `access_request.notify.failed`, `access_request.reconcile.requeued`. No email, ever.
- **Single aggregated ops alert (B7)**: the sweep emits **one** coalesced Telegram-ops
  alert when un-notified backlog persists past the grace window — *not* per failed email.
  Plus a **one-time boot alert** if `RESEND_API_KEY` is present-but-invalid (401).

### Launch gate (STOP-1 — blocking prerequisite; R2-10 + Counsel R2 #1: now a CI ASSERTION, not a comment)
**This feature is gated on `owner-onboarding-invite-gating` shipping first.** The R1/R2
draft enforced this with "a release-sequencing gate, **not** a code flag" — i.e. a sentence
in this doc. The Breaker (R2-10, then R3-4) and Counsel (R2 #1) flagged that the strongest
invariant in the design rested on the weakest enforcement — and R3-4 proved the R2-10 flag
as specified was **render-only**, leaving the backend endpoint live. **Three mechanical
gates replace the comment:**
1. **Backend route-registration gate (R3-4 — THE load-bearing STOP-1 enforcer).**
   `ACCESS_GATE_PUBLIC_ENABLED` (Zod env, default `false`) gates the
   **`fastify.register(accessRequestRoutes, { db: pool, queue })` call itself** (§"Routing
   convention", `server.ts`):
   ```
   if (env.ACCESS_GATE_PUBLIC_ENABLED) {
     fastify.register(accessRequestRoutes, { db: pool, queue });
   }
   ```
   While the flag is off the route is **never mounted** → `POST /api/access-requests` falls
   through to `setNotFoundHandler` (`server.ts:871`) and returns the **same
   `404 {error:'Not found'}` as any unknown path**. So **before invite-gating ships the
   capture endpoint is NOT publicly POST-able** — a scripted client cannot seed
   `access_requests` or trigger operator emails. This closes R3-4 ("frontend hidden but
   backend live, defeating the STOP-1 prerequisite the flag was promoted to enforce"). The
   migrations (table + 3 queues) still ship — additive and harmless with no route to write
   them; only the *route* is gated. **This — not the render flag — is what satisfies STOP-1.**
2. **Frontend render gate.** The same flag also omits the CTA from the SPA (no exposed UI).
   Secondary to (1): render-hiding alone never closed the reachable surface (R3-4).
3. **CI banned-strings assertion.** A test greps the access-request i18n keys (en + sq) for
   the banned scarcity strings (`waitlist`, `request access`, `early access`, `position #`,
   `approved`, `under review`, `application`) and **fails the build** if any appear while a
   companion flag `ACCESS_GATE_INVITE_GATING_SHIPPED` is unset. So "waitlist/approved" copy
   is *mechanically impossible* to ship before invite-gating lands — the sequencing promise
   becomes a sequencing **proof** (Counsel R2 #1).

**Launch sequence (mechanically enforced):** migrations may ship anytime (route 404s while
flag off, so PII pipeline is unreachable) → invite-gating Council → invite-gating shipped →
flip `ACCESS_GATE_PUBLIC_ENABLED=true` (route mounts) → CTA live. The single launch act is the
flag flip, behind a reviewable change + the §"Proof obligations" CI assertion that
`POST /api/access-requests` returns **404 when the flag is off** and **200 when on**. The
release-sequencing checklist (verify invite-gating is live before flipping the flag) **remains**
as the human layer on top, but the reachable-surface gate is now mechanical, not a comment.

### Rollback / scaling-gate
- Feature is additive and flag-able: if `RESEND_API_KEY`/`WAITLIST_NOTIFY_EMAIL` unset,
  the email leg no-ops (logs + the single aggregated alert), submits still persist. The
  whole CTA can be hidden client-side without touching the API.
- Migration is forward-only; rollback = leave the table in place. B10: any rows remain
  erasable via the day-one DELETE runbook (the grant survives rollback-without-drop).

### GDPR — consent basis + day-one erasure + 12-month auto-erase (STOP-2, decided)
`access_requests.email` is a **PII store** with a complete GDPR posture, all in scope:
- **Lawful basis = explicit consent**, evidenced per-row by `consent_at` + `privacy_version`
  (§4/§5). Consent is withdrawable = erasure.
- **Day-one manual erasure**: operational role `DELETE` grant +
  `scripts/erase-access-request.ts <email>` + runbook (§4/§7). Right-to-erasure /
  consent-withdrawal honored **today**.
- **12-month auto-erase**: `access-request.retention-sweep` cron (§5), `DELETE WHERE
  created_at < now() - '12 months'`, mirroring `anonymizer.retention`. Retention window is
  config (`ACCESS_REQUEST_RETENTION`) and **must match the number stated in `/privacy`**.
- **Stage-30 (still deferred, now redundant-but-harmless):** folding `access_requests`
  into `AnonymizerService` is no longer needed for retention (the dedicated sweep handles
  it). If a future consolidation wants a single retention engine, that is a refactor, not
  a launch blocker. Demoted from the critical path.

### `/privacy` page routing (STOP-2 — in scope; R2-5 server fix)
**Verified:** `apps/web` uses a flat SPA `<Routes>` block in `main.tsx:39-50`
(`BrowserRouter` → `/`, `/start`, `/login`, `/auth/callback`, `/s/:slug/*` (client),
`/admin/*`, `/courier/*`, `*` 404). There is **no `/privacy` route today** (only a string
in `CheckoutPage.tsx`) — a link-to-404 would itself be a GDPR failure.

> **R2-5 — the SSR/server fallback must serve `/privacy` to NON-browser clients too.** The
> Breaker confirmed the SPA shell is served by `server.ts:871` `setNotFoundHandler`, whose
> guard is `request.headers.accept?.includes('text/html') OR request.url matches SPA_ROUTES`
> — and `SPA_ROUTES` (`server.ts:870`) does **not** contain `/privacy`. So today a
> regulator/curl/crawler/unfurler sending `Accept: */*` gets a hard JSON `404`, not the
> notice. **Fix: add `'/privacy'` to `SPA_ROUTES`.** Because the `SPA_ROUTES` branch is
> evaluated **independently of `Accept`** (it is the OR-arm), a `GET /privacy` with **any**
> `Accept` then returns `200` + `index.html`. This one-line list edit is the complete fix —
> no SSR rework needed. **Proof obligation:** `GET /privacy` with `Accept: */*` → 200 (not
> 404). (Belt-and-suspenders: the SPA shell is static HTML; even a non-JS crawler that does
> not hydrate still receives a 200 document, satisfying "the link resolves.")

**Add one SPA route (client) + the server `SPA_ROUTES` entry (above):**
- New page `apps/web/src/pages/PrivacyPage.tsx` (static, no data fetch), rendered via
  `<Route path="/privacy" element={<PrivacyPage />} />` in `main.tsx` (add it next to
  `/login` — eagerly imported is fine; it is a tiny static page, but `lazy()` is equally
  acceptable matching the `ClientRoutes` pattern at `main.tsx:14`).
- Bilingual via the existing `I18nProvider`/`packages/ui/src/lib/i18n.ts` (`sq`/`en`),
  design tokens via the existing `ThemeProvider`. No new dependency, no router refactor.
- **Versioning + drift backstop (R2-6).** The page renders a visible notice version that
  equals the `PRIVACY_NOTICE_VERSION` constant written into `privacy_version` on submit
  (§5). A date string (e.g. `'2026-06-20'`) — no version table (YAGNI). **R2-6 fix:** a CI
  test computes a **content hash of the rendered notice prose** (the i18n notice strings)
  and **fails the build** if the hash changed without `PRIVACY_NOTICE_VERSION` being bumped.
  This binds the *label* to the *text* mechanically — closing the "edit the prose, forget to
  bump the date" gap the Breaker found (a label===label test would have passed while the
  consent record attests text the user never saw). Owner: frontend + security.
- Content (minimal, §10): lawful basis (consent), data collected (email, ip_hash,
  consent timestamp), purpose (contact about access/launch), retention (**12 months from
  first contact**, R2-7), rights (access / erasure / withdraw consent), and a **reachable
  erasure-request contact** (Counsel R2 #2 — a real `privacy@…`/operator address or contact
  route, **not** a placeholder and **not** the internal `WAITLIST_NOTIFY_EMAIL`; mirrors the
  existing `CheckoutPage.tsx:913` "contact … for removal" convention). "Withdraw anytime"
  must point at a channel a person can actually use, or the right is paper-only. Nothing more.

---

## 9. Open / Accepted Risks + Defer-Flags

### HARD PREREQUISITE (STOP-1 — promoted from defer-flag; BLOCKING)
**Owner onboarding is OPEN self-serve today — the soft gate does not actually gate
anything until this changes.** Per the STOP-1 human ruling, `owner-onboarding-invite-gating`
is now a **blocking prerequisite**: soft-access-gate does **not** launch to prod until
invite-gating ships. Load-bearing evidence (the surface the *future, separate* Council
must address — recorded here as INPUT, **not designed here**):
- **Google login**: any visitor with a Google account →
  `INSERT INTO users ... ON CONFLICT (google_sub) DO UPDATE` then immediately
  `signAuthToken({ role: 'owner', userId })` — `apps/api/src/routes/auth.ts:112-118`
  and `auth.ts:138`. No invite/allowlist check. **(Future-Council touchpoint.)**
- **Telegram login**: same — first login creates an owner and mints an owner token,
  `auth.ts:184-188` and `auth.ts:213`. **(Future-Council touchpoint.)**
- **Onboarding** only checks `requireRole(['owner'])` (`onboarding.ts:30`), and that role
  is granted to *everyone* who logs in (above). **(Future-Council touchpoint.)**

> **Scope boundary (explicit).** The *design* of invite-gating — where the allowlist/invite
> check is inserted (at token-mint in `auth.ts:138`/`:213` vs at onboarding
> `onboarding.ts:30`), what backs the allowlist (`access_requests.status='invited'` vs a
> separate invite store), first-owner bootstrap, RS256 claim shape — is a **separate
> serious auth-flow change for its own Triad Council**. This proposal lists the surface as
> an entry point and asserts the **sequencing dependency**; it does **not** decide any of
> the above. **Sequencing: invite-gating Council → invite-gating shipped → soft-access-gate
> shipped.** Owner: auth/architecture.

> **Copy rule tied to the prerequisite.** User-facing copy may stay "register interest /
> we'll be in touch" at **any** state (it is truthful whether or not gating exists). But
> "waitlist / approved / queue position" copy is permitted **only after** invite-gating
> ships — because only then is the scarcity real. Until then, banned-string list (§10)
> stands.

### B9 — "gate gates nothing" (cross-ref STOP-1)
The precondition above is the engineering ground-truth behind Counsel **ETHICAL-STOP-1**.
This proposal **nowhere** claims the feature controls access — it is an interest capture
over an open self-serve door. The user-facing narrative is therefore reframed to
"register interest / keep me posted" (§10), removing any "waitlist / request access /
position / reviewed" language that would imply a scarcity the product does not have.
The defer-flag (owner-onboarding-invite-gating) is the *only* thing that would make the
"gate" framing true; until it ships, the copy must stay interest-framed. (B9 = MED,
honest-by-design; no code defect in this design — a product-truth the council retains.)

### Accepted risks
| Risk | Decision | Owner |
|---|---|---|
| Resend daily-cap exceeded on a launch spike | Accept: email best-effort; un-notified rows readable in bulk via `status='new'`; the sweep emits **one** aggregated alert (B7), not a storm. | Ops |
| Rare duplicate operator email on two-worker race | Accept (reduced by B8 claim-before-send → at-most-one in the common case): low-value mail, not transactional. | Ops |
| No CAPTCHA → bots fill table within rate-limit | Accept for v1: honeypot + 5/min/**real-IP** + unique(email) cap damage; table cheap, PII = 1 column, no raw IP, day-one erasure. | Security |
| **B4** — `USING(true)` RLS gives no row isolation; shared operational pool reads all emails | Accept: real boundary is GRANT-layer + parameterized-query norm; a dedicated write-only role doesn't shrink the read surface (worker/erasure/ops-list all SELECT) → over-engineering for ≤20k rows / 1 PII column. | Security |
| **B11** — honeypot autofill false-negative (password managers) | Accept (LOW): honeypot is **secondary**; rate-limit (real IP) is the primary bot control. Honeypot uses `autocomplete="off"` + off-screen; a tripped honeypot is best-effort, not the load-bearing defense. | Security |
| **B12** — localStorage "already submitted" guard | Accept (LOW): documented as **UX nicety only, not a security control** — bots bypass it trivially and a legit user with two emails can clear it; it must not be relied on for anything. | Frontend |
| **B10** — rollback leaves PII table | Accept (LOW): mooted by day-one DELETE grant + erasure runbook (rows erasable regardless of table presence). | Ops |
| **R2-4** — fire-and-forget enqueue lost on single-machine deploy/OOM | Accept (MED): recovered by `access-request.reconcile` — whose scheduling is now **proven** (R2-1), not "may not schedule." Bounded loss ≤(15min cadence + grace), self-healed, never data loss (row committed). | Ops |
| **R2-9** — permanent send-failure re-fed by reconcile (treadmill) | Fix + accept residual: reconcile predicate gains `notify_attempts < $cap` (R2-9) → bad-address rows stop being re-enqueued and surface as stuck in the one aggregated alert. Residual = one stale alert until operator acts. | Ops |
| **R3-2** — fail-closed shared-bucket boot-warn is one-time-on-first-request → a `Fly-Client-IP` loss that begins *after* the first good request degrades the planet to one 5/min bucket with no fresh warn | Accept (LOW) + cheap re-arm: the degrade fails **safe** (over-throttle, never under-protect, never re-opens the spoof hole) and is config-change-only. Instead of a one-time latch, the `clientIp()` path **re-warns on a throttled cadence** — a module-level `lastFlyMissingWarnAt` timestamp + a 60s interval: every request that hits the `'no-fly-client-ip'` fail-closed branch logs the ops warn **at most once per 60s** (`if (Date.now() - lastFlyMissingWarnAt > 60_000) { warn(); lastFlyMissingWarnAt = Date.now() }`). So a mid-life `Fly-Client-IP` loss re-surfaces within ≤60s of the next request, not "never," and a flood of misconfigured requests still emits ≤1 warn/min (no log-storm). Pure in-process, no new probe/cron/cross-worker plumbing (the `LivenessChecker` runs in the worker-drain context and never sees the HTTP request path's headers, so it is the wrong seam). Residual = ≤60s blind window on a config-change-only, fail-safe degrade. | Platform |
| **R3-3b** — framework malformed-JSON `400` at the Fastify content-type parser (`server.ts:516`) before the route → "zero non-200 for any body" over-promise | Accept (LOW): **not an email-existence oracle** — the 400 is app-wide/framework-level (identical for `/api/telemetry` and every JSON route), reveals nothing about whether an email is stored, and the route is a publicly-known path. The anti-enumeration invariant (email-existence non-leak) is **intact**; only the over-broad wording was retracted (§1, ADR Decision-6). Suppressing it = app-wide parser surgery (over-eng against a non-leak). | Security |

### NEW risks — R2 dispositions (post-Breaker-R2 re-attack)
| # | Risk | Disposition | Owner |
|---|---|---|---|
| **N1 → R2-3** | **Consent-400 IS a DoS amplifier + route-fingerprint** (Breaker R2-3 proved the accept was inverted: the 400 is the *cheapest* path → optimal flood target, and a status/latency branch distinct from every 200). | **FIX (not accept): the 400 is REMOVED.** No-consent → silent uniform-200 honeypot drop (§5). Cheapest-path target and fingerprint both eliminated; no non-200 for any well-formed body. | Security |
| **N2 → R2-6** | **`privacy_version` drift from prose.** A "single-sourced" label-===-label test passes while an engineer edits the prose without bumping the version → consent record attests text the user never saw. | **FIX: content-hash CI test** — hashes the notice prose, fails the build if the hash moved without a version bump (§8). Binds label→text mechanically. | Frontend + Security |
| **N3** | **Retention DELETE racing notify/reconcile.** | Accept (unchanged): covered by B8.1 (CAS 0 rows → ack no-op); the reconcile predicate returns nothing for a deleted row. Confirmed the tolerance fires from the cron path, not only manual erasure. | Ops |
| **N4 / R2-7** | **Consent × duplicate (`ON CONFLICT DO NOTHING`) + retention clock.** First-consent is pinned (correct); but retention `DELETE WHERE created_at < now()-12mo` measures from **first** submit, so a month-11 re-submitter is erased at month 12 from *original* contact. | **ACCEPT-RISK (retention from first contact, justified):** the lawful interest dates from first contact; 12mo-from-first is within the defensible band (Counsel). Refreshing `created_at`/`consent_at` on duplicate would *forge* a consent record (the dishonest move N4 rejects). **Documented as deliberate**; `/privacy` copy tightened to "12 months **from when you first contact us**" (R2-7) so it can't read as last-contact. Re-consent flow stays defer-flag. | Security |
| **N5 → R2-5** | **`/privacy` hard-404 to non-`text/html` clients** (Breaker confirmed broken at `server.ts:870` — `/privacy` absent from `SPA_ROUTES`). | **FIX: add `'/privacy'` to `SPA_ROUTES`** → 200 for any `Accept` (§8). Proof: `GET /privacy` `Accept: */*` → 200. | Frontend |

### Defer-flags (tracked, not built)
- **owner-onboarding-invite-gating** — **PROMOTED to a blocking prerequisite (STOP-1),
  separate Council.** No longer a "ship-around" defer-flag: soft-access-gate does not
  launch until it ships. (B9 / §9 prerequisite.)
- **admin review UI** for `access_requests` (list/triage/transition `new→invited`).
- **Cloudflare Turnstile / hCaptcha** if bot volume materializes.
- **double-opt-in / email confirmation** (currently single-step capture).
- **re-consent flow** under a newer `privacy_version` (N4) — first-consent is the record;
  re-consent is a deliberate future feature, not in v1.
- **telemetry/otp `request.ip`=proxy fix** (B2 corollary) — the same hardened
  `Fly-Client-IP`-only `clientIp()` helper (R2-2, **not** the spoofable XFF version) or a
  vetted global `trustProxy`. Owner: platform/security. **NOT widened into this scope.**
- **`locale` enum reconciliation** (`'al'` in spec vs app's `'sq'`).

> **Promoted OUT of defer (now in scope, STOP-2):** lawful basis = **consent** (capture
> columns); **12-month retention auto-erase** cron (`access-request.retention-sweep`) —
> Stage-30 anonymizer folding demoted to redundant-but-harmless; minimal **`/privacy`
> page** (sq/en).

> **Promoted OUT of defer (now in scope):** the **re-notify / reconciliation sweep**
> (`access-request.reconcile` cron) — it was deferred in the PROPOSED draft; the Breaker
> (B3) showed that with the admin UI also deferred there would be **no v1 recovery
> signal** for the enqueue-after-commit gap. It is cheap and closes B1-residue, B3, B5's
> lost-enqueue case, and feeds B7's single alert — so it is now part of v1.

---

## 10. Frontend (design notes)

`AccessRequestForm` (`apps/web/src/...`, repo design tokens; locale via
`packages/ui/src/lib/i18n.ts` — note: keys `sq`/`en`, not `al`):
- One email input + **one required consent checkbox** + one submit button.
  Tap-target ≥ 44px. No cookies.
- **Consent checkbox (STOP-2, required).** The submit button is **`disabled` until the
  checkbox is ticked**. The checkbox label states consent + links to `/privacy`
  (`accessRequest.consentLabel`, below). The POST body carries `consent: true` only when
  ticked; an unticked submit cannot fire (button disabled). The server independently
  re-validates `consent` via a strict `z.literal(true)` parse (§5, R2-8); a failed parse
  yields the **same silent `200`** as the honeypot (no 400 — R2-3), so the client `disabled`
  is UX and the server is authority *without* a distinguishable rejection branch.
- States: `idle | submitting | success | error`, with **distinct copy for 429
  (rate-limit), timeout, and 5xx** so the user knows whether to wait or retry.
- **Honeypot (B11): secondary, not load-bearing.** Off-screen `website` field,
  `tabindex={-1}`, `autocomplete="off"`, `aria-hidden="true"`, off-screen positioning
  (not `display:none`). Because password managers can ignore `autocomplete="off"` and
  autofill a `website` field → a *legit* user could trip it and get a silent success they
  didn't earn (false-negative). Mitigation: **the rate-limiter (real client IP) is the
  primary bot control**; the honeypot is best-effort. To reduce autofill collisions, name
  the field something autofill ignores (e.g. `company_url_hp`) rather than `website`, and
  treat a tripped honeypot as low-confidence. Do **not** add a security guarantee that
  rests on the honeypot.
- **localStorage "already submitted" guard (B12): REMOVED as a control.** It is bypassed
  by bots and blocks a legit user with two emails (work + personal). At most keep it as a
  *cosmetic* "thanks again" hint that never blocks a resubmit. Documented as UX-only.
- a11y: `<label>` bound to input, `aria-live` region for state, focus management to the
  result message on submit.
- **Consent label + `/privacy` link (STOP-2):** the checkbox label carries the consent
  text + a link to `/privacy`. The `/privacy` route is now **in scope** (§8 routing) — a
  new SPA route + `PrivacyPage.tsx`. The link must resolve (no link-to-404, which would be
  a GDPR failure). Privacy copy states: basis = consent, data = email/ip_hash/consent
  time, purpose = contact about access, retention = **12 months**, rights =
  access/erasure/withdraw-consent, contact.
- POSTs `{ email, website?, locale, consent: true }` to `/api/access-requests`; treats
  any non-2xx as the `error` state; never displays whether the email already existed
  (anti-enumeration parity with the server). The server returns uniform `200` even for
  malformed input, so client-side email validation is **UX-only**, never the authority.
  There is **no `consent_required` 400** anymore (R2-3) — a no-consent body returns the same
  uniform `200`, so the client only ever sees `200` (success) or a transport error (429 /
  timeout / 5xx).
- **R3-3a — `success` is gated on locally-confirmed consent-in-body + 2xx, NOT a bare 200
  (counsel-R3 fix for silent legit-user lead-loss).** The R2-3 inversion (no-consent →
  silent 200) means a **legit, consenting** user whose body drifts (`consent: true` →
  `"true"` via a serialization quirk, a JSON-lib coercion, an autofilled honeypot, or any
  client bug) gets a server `200` while **no row is written** — a *silent* lead-loss the user
  never sees. To close this **on the human's side**, the client shows the `success` state
  **only when both** (a) the request it actually sent carried a body it locally verified as
  `consent === true` (a strict, post-serialize check of the JSON it is about to POST — "I
  know I sent real consent") **and** (b) the response is 2xx. If consent was *not* truly in
  the sent body (the user did tick the box but a bug dropped/mutated it), the client renders
  the **`error`/retry** state — **never a false success** — even on a 200. This is *not* the
  server leaking consent state (the server stays uniform-200, anti-enumeration intact); it is
  the **client trusting its own send**, which it is entitled to do. It closes the B11-class
  silent-false-negative on a real person: a genuine consenter is never told "we'll be in
  touch" while their row silently fails to exist.
- **Negative UI proof case (R3-3a, added to Proof obligations):** a Playwright/unit case that
  **submits with consent NOT present in the sent body** (e.g. mutate the payload to drop or
  string-coerce `consent`) and asserts the form **does NOT show the success copy** — it shows
  `error`/retry. This proves the success state cannot be reached on a no-consent send,
  catching the exact serialization-drift class R3-3 named.

### STOP-1 FIX — user-facing copy (interest, not scarcity)
All wording is reframed from "waitlist/access" to "register interest / keep me posted."
No number, queue, position, "approved," or "reviewed" language anywhere. Proposed i18n
keys (final wording is a **human sign-off** item):

| Key | en (proposed) | sq (proposed) |
|---|---|---|
| `accessRequest.heading` | "Be the first to know" | "Bëhu i pari që e di" |
| `accessRequest.sub` | "Leave your email and we'll reach out when we're ready for you." | "Lër emailin dhe do të të shkruajmë kur të jemi gati." |
| `accessRequest.emailLabel` | "Email" | "Email" |
| `accessRequest.cta` | "Keep me posted" | "Më mbaj në dijeni" |
| `accessRequest.success` | "Thanks — we've got your email and we'll be in touch." | "Faleminderit — e morëm emailin dhe do të të kontaktojmë." |
| `accessRequest.err429` | "One moment — too many tries. Please wait a minute." | "Një moment — shumë përpjekje. Prit pak." |
| `accessRequest.errGeneric` | "Something went wrong. Please try again." | "Diçka shkoi keq. Provo sërish." |
| `accessRequest.consentLabel` | "I agree to be contacted by email about access, and to the [Privacy Notice](/privacy)." | "Pranoj të kontaktohem me email për qasjen dhe [Njoftimin e Privatësisë](/privacy)." |
| `accessRequest.privacy` | "We store your email with your consent to contact you about launch, for up to 12 months from when you first contact us. You can withdraw consent or ask us to delete it anytime — email [privacy contact]." | "Me pëlqimin tënd ruajmë emailin që të të kontaktojmë për nisjen, deri në 12 muaj nga kontakti i parë. Mund ta tërheqësh pëlqimin ose të kërkosh fshirjen kurdo — shkruaj te [kontakti i privatësisë]." |

> **Lawful basis = consent (STOP-2).** Copy says "with your consent" / "me pëlqimin tënd"
> — **not** "legitimate interest." Retention is the concrete **12 months from first contact**
> (R2-7 — measured from `created_at`, which `ON CONFLICT DO NOTHING` never refreshes; the
> copy is explicit so it cannot be mis-read as last-contact). The withdrawal/erasure line
> names a **reachable contact** (Counsel R2 #2 — `[privacy contact]` is a real address, not a
> placeholder, to be provisioned before launch). The consent text + `/privacy` link sit on
> the **checkbox label**; the button is disabled until ticked.
> Banned strings (must NOT appear in any locale **until invite-gating ships**): "waitlist",
> "request access", "early access", "position #", "you're approved", "under review",
> "application". The success state celebrates *being heard*, not *being selected*.

---

## 11. Counsel steel-man (inbox-only) — recommendation

Counsel steel-manned "**no table — just email the operator, persist nothing.**" It is the
cleanest answer to both STOPs (no retention question, no erasure gap, no RLS surface) and
the most ponytail. **Recommendation: the table still wins, narrowly, for these named
reasons** — but only *because* we now answer STOP-2:
- **Survives a Resend outage.** Inbox-only loses the lead entirely if the one best-effort
  email fails; the table keeps the row and the sweep retries. This is the decisive reason.
- **Dedup** (`unique(email)` / `ON CONFLICT`) — an inbox re-pages on every resubmit.
- **Queryability** — `status='new'` bulk view is the v1 ops surface (admin UI deferred);
  an inbox is not queryable for "how many un-handled."
- **Anti-enumeration + claim-check** make the PII surface genuinely small.

The steel-man's real force is the bar it sets: **persisting PII is defensible only with an
erasure answer** — which is exactly why STOP-2's day-one DELETE path is now mandatory and
in scope. *Persisting without erasure would be strictly worse than the inbox on every
ethical axis.* With erasure in, the table is the right call. (If the team wanted absolute
minimalism pre-launch, inbox-only is a legitimate fallback — recorded, not chosen.)

---

## Proof obligations (Mandatory Proof Rule)
- **B1 durability (new)**: integration test asserting (a) after migrations the `pgboss`
  partition table for `access-request.notify` **exists**, and (b) `boss.send(...)` →
  worker drain → `notified_at` set. A swallowed boot-warn is NOT proof.
- **API**: an E2E/integration test asserting `POST /api/access-requests` returns
  `200 {ok:true}` for new + duplicate + honeypot + **malformed-email** bodies, and that a
  second identical email produces **no second row** (DB assertion) and **no second
  enqueue**.
- **B5 timing (new)**: assert new-vs-duplicate response latency is indistinguishable
  (enqueue off critical path) — at minimum assert the handler returns before the enqueue
  is awaited (no `await enqueue` on the response path).
- **B2/R2-2 real-IP (revised)**: request with a spoofed `X-Forwarded-For` **and** a distinct
  `Fly-Client-IP` → assert the rate-limit key and stored `ip_hash` derive from
  `Fly-Client-IP` **only**, never from `X-Forwarded-For` (the XFF fallthrough is removed).
  Second case: request with **no** `Fly-Client-IP` in a prod-like env → assert it falls
  closed to the shared bucket (not to the XFF value) and emits the boot warn.
- **R2-1 cron-schedule durability (new)**: after server boot, assert a row exists in
  `pgboss.schedule` for **both** `access-request.reconcile` and
  `access-request.retention-sweep` (proves the runtime `boss.schedule` write actually
  landed — not just that the queue partition exists). A swallowed boot warn is NOT proof.
- **R3-1 boot fail-fast (new)**: assert that the new cron's `boss.schedule` is `.catch`-wrapped
  (a schedule throw does NOT propagate out of `main()` / does NOT prevent `fastify.listen`),
  AND assert the post-`listen()` boot-assert **fail-fast-exits**: inject a missing
  `pgboss.schedule` row (e.g. stub the schedule check to return absent) → expect
  `process.exit(1)` (clean non-zero), NOT a live process serving no HTTP. Proves the
  failure-visibility is a real runtime guard, not a half-booted zombie or a CI-only test.
- **R3-3a negative UI consent-in-body (new)**: submit with consent NOT present in the *sent*
  body (mutate payload to drop / string-coerce `consent`) → assert the form shows
  `error`/retry, **NOT** the success copy — even though the server replies 200. Proves a
  no-consent send can never reach a false success on the human side.
- **R3-4 route-registration gate (new)**: with `ACCESS_GATE_PUBLIC_ENABLED=false`, assert
  `POST /api/access-requests` returns **404** (route unmounted, falls to `setNotFoundHandler`)
  — i.e. the backend capture endpoint is NOT publicly POST-able pre-gating; with the flag
  `=true`, assert it returns **200**. Proves the STOP-1 prerequisite is enforced at the
  reachable surface, not just frontend render.
- **B8 idempotency (new)**: deliver the same job twice → assert exactly one email send;
  delete the row then deliver the job → assert worker **acks without throwing**.
- **RLS**: extend `apps/api/tests/phase5/rls-adversarial.test.ts` to assert anon
  `SELECT` on `access_requests` = 0 rows / permission denied.
- **Erasure (STOP-2)**: `scripts/erase-access-request.ts <email>` removes the row;
  asserted by a follow-up count = 0.
- **Consent gate (STOP-2, R2-3/R2-8 REVISED)**: `POST` with `consent` missing / `false` /
  `"true"` / `1` → assert **`200 {ok:true}`** (NOT a 400) and **no row inserted** (DB count
  unchanged, no `consent_at` written); `POST` with `consent: true` (literal) → row inserted
  with non-null `consent_at` and `privacy_version === PRIVACY_NOTICE_VERSION`. Assert the
  no-consent response is **byte-identical** to a real-submit `200` (no status/shape branch).
  Assert the `z.literal(true)` rejects the truthy-string `"true"` (R2-8 — proves the
  structural gate, not a hand `if`).
- **Retention sweep (STOP-2, new)**: insert a row with `created_at = now() - '13 months'`,
  trigger `access-request.retention-sweep` (`boss.send`/manual run) → assert the row is
  **DELETEd** and a row at `now() - '11 months'` **survives**. Assert the worker uses the
  advisory-lock guard (no double-fire).
- **Privacy version drift (N2 → R2-6, revised)**: a CI test that hashes the rendered
  `/privacy` notice **prose** and **fails** if the hash changed without
  `PRIVACY_NOTICE_VERSION` being bumped (binds label→text, not just label→label); plus the
  privacy copy's retention number === `ACCESS_REQUEST_RETENTION`.
- **R2-5 `/privacy` reachability (new)**: `GET /privacy` with `Accept: */*` (no `text/html`)
  → assert **200** + an HTML body (not the JSON `404 {error:'Not found'}`). Repeat with
  `Accept: application/json` → still 200.
- **R2-10 launch-gate (new)**: a CI test that greps the access-request i18n keys for the
  banned scarcity strings and **fails the build** if any appear while
  `ACCESS_GATE_INVITE_GATING_SHIPPED` is unset; assert the public CTA does not render when
  `ACCESS_GATE_PUBLIC_ENABLED=false`.
- **UI**: Playwright against `https://dowiz.fly.dev` — assert the consent checkbox is
  **not** checked on initial render (`not.toBeChecked()`, Counsel R2 #4 — proves no
  pre-check dark-pattern); assert the **submit button is disabled until the consent checkbox
  is ticked**; tick → fill the form, submit, `expect(success message).toBeVisible()`; fill
  honeypot, submit, assert same success copy (no enumeration); assert **no banned scarcity
  strings** render; assert the `/privacy` link resolves to a real page (not 404) and shows
  the consent basis, "12 months from first contact", and a working erasure contact.
- No task is complete without pasted proof output.
