# Resolution — audit-fix-rls-reliability (Council RESOLVE round)

- **Status:** RESOLVED-DRAFT v2 — every breaker finding dispositioned; both counsel ETHICAL-STOPs
  addressed; `proposal.md` revised in place. Awaiting conductor re-attack; nothing here is
  self-certified.
- **Date:** 2026-07-03
- **Inputs:** `proposal.md` (v1), `breaker-findings.md`, `counsel-opinion.md`, fact sheets
  (`site-inventory.md`, `rls-state.md`, `pgboss-state.md`), all re-verified against source at HEAD
  of `feat/phase0-safety-hardening` during this round.
- **Rule applied:** each finding → **FIX** (design revised), **ACCEPT-RISK** (justified + owner),
  or **DEFER-FLAG** (explicitly parked with owner + re-entry trigger). Each ETHICAL-STOP → revised
  design or **needs-human**.

---

## 0. Fact corrections found during resolution (they change the fixes)

1. **`rls-state.md §4` misread the firebreak.** Migration `1780421100065` does ENABLE **+ FORCE**
   on `users`/`auth_refresh_tokens`/`ops_worker_heartbeat` (up(), STEP A2 — "Enable + Force RLS");
   the `NO FORCE` lines at `:78-83` are in `down()`. Mig `1790000000077` RC2 then adds the
   role-restricted permissive policy `ops_all FOR ALL TO dowiz_app USING (true) WITH CHECK (true)`
   (`:27-30`). So the **actual** firebreak convention is: **FORCE + role-restricted-to-`dowiz_app`
   policy** — deny-by-default for every other role, credential access confined to the app role,
   pre-context auth reads keep working post-flip. This is stronger than the "ENABLE-no-FORCE"
   the fact sheet described, and it is precisely the pattern MIG-1 now mirrors (F1 fix).
   `rls-state.md` carries a correction note.
2. **Notification job payloads already carry `locationId`**
   (`apps/api/src/notifications/workers/index.ts:100` destructures `{ orderId, locationId, event }`)
   — the F9 fix needs no payload-contract change, only a two-context handler.
3. **Telegram chat→location mapping already exists in data**
   (`owner_notification_targets.address` = chat id, `telegram_connect_tokens`) — tenant for the
   webhook can be resolved from the *chat* (the authenticated principal), not discovered from an
   unscoped order read. This unlocks the F5 fix.
4. **`courier_locations` was rewritten missing-ok by mig 077** (`:80-82`) — so the post-flip
   session-validation EXISTS fails **closed as 401**, not 500 as v1 of the breaker text implied for
   the throw-on-unset dialect. Lockout stands either way; F1.3 remains valid.

---

## 1. Breaker findings — dispositions

| # | Sev | Disposition | One-line resolution |
|---|-----|------------|---------------------|
| F1 | CRIT | **FIX** | MIG-1 redesigned: couriers/courier_sessions become **firebreak credential tables** (FORCE + `ops_all TO dowiz_app`), not tenant tables; `courier_locations` gets `FOR SELECT TO dowiz_app`; new proof **P1b** tests the key, not just the lock; tenant-scoped `dowiz_app_rls` policies → **MIG-1b, DEFER-FLAG** to the B3 courier-lane flag |
| F2 | CRIT | **FIX** | Conversion gate redefined from `grep(set_config)` to a **FORCE-table access inventory** (scripted scan → CI gate **P10**) + a **flip-rehearsal proof suite (P9)** + named deploy gate **GATE-ANON-E2E**; checkout/track/auth paths enumerated as first-class conversion work |
| F3 | HIGH | **FIX** | LC4 context model corrected: cross-tenant scan/claim moves into a SECURITY DEFINER `gdpr_claim_due()`; per-row work runs in `withTenantTx({tenantId})`; **LC4-MIG** adds a missing-ok `app.current_tenant` arm to the two `app_member_location_ids()` policies |
| F4 | HIGH | **FIX** | Stale-reclaim keyed on `metadata->>'claimed_at'` stamped at claim time (existing jsonb — **no new column**); false-reclaim neutralized by a **claim-token CAS** on every terminal write; the v1 "no schema change" claim is **dropped** and restated: *one additive migration (policy arms + DEFINER fn), zero new columns* |
| F5 | HIGH | **FIX** | Telegram handlers redesigned around the invariant **no DB txn open across external HTTP**: chat-based tenant resolution (DEFINER `resolve_telegram_chat`) → short `withTenantTx` for authz+guarded UPDATE → COMMIT → Telegram sends post-commit; 409 re-read in its own short txn; proof **P11** asserts commit-before-HTTP ordering |
| F6 | MED | **FIX** | "Wrap the client block" replaced by **semantics-preserving conversion classes**: reads → wrap; single logical write → wrap (atomic correct); multi-write best-effort flows (spa-proxy onboarding product seed) → phase txns + per-row SAVEPOINT, preserving today's partial-success behavior |
| F7 | MED | **FIX** | Terminal-write **CAS** (`WHERE status='in_progress' AND claim_token=$mine`) is the dedup: audit-log INSERT + `gdpr.erasure_completed` publish happen **only on CAS rowCount=1** → exactly one audit row / event per request, regardless of double-trigger |
| F8 | MED | **FIX** (+1 open spike) | Boot-time **queue-policy reconciler** (read `pgboss.queue` stored policy, converge on drift); P6 rewritten to seed a **pre-existing `standard` queue** first (prod mirror) so it fails while reconciliation is a no-op; **OPEN-V1**: verify v10.4.2 `updateQueue` supports policy change — fallback designed: send-side `singletonSeconds` (the one dedup proven to work on `standard`, `bootstrap/messaging.ts:43`); drop/recreate = break-glass only (job loss) |
| F9 | MED | **FIX** | Notify handler uses **two contexts** from data it already has: order read under `withTenantTx({tenantId: job.data.locationId})`, devices read under `withTenantTx({userId})`; **P2 pinned to the end-state**: runs against the flip-rehearsal DB (all MIGs applied + enforcement on), removing the ordering-dependent false-green |
| F10 | LOW | **FIX** | **`.dlq` is canonical** (code convention wins — `deadLetterQueueName()` `queue-provider.ts:52-54`); all `.dead` prose corrected; the monitor derives its subscription list from the `QUEUE_POLICY` map, not a glob |
| F11 | LOW | **FIX** | "Zero runtime risk" claim withdrawn: the v10 type-pin lane **includes fixing the direct `boss.work` array-shape sites** (settlement-cron, dwell, lifecycle-handlers, anonymizer-gdpr) — they are runtime-wrong today; route them through the provider's normalizing `queue.work()`; the anonymizer's fix **rides Lane A** (same file as LC4 — the `job.data` retry path is only correct with the array shape handled) |
| F12 | LOW | **FIX** | "Detectors register FIRST" dropped; replaced by: per-registration isolation for **every** worker, boot registry of actually-started workers **feeds the liveness/reconciliation watch-set** (no static 8), and a failed registration itself publishes a boot DRIFT + degrades `/health` — no false DRIFT, no amputation |
| BN-1 | note | adopted | Settlements conversion proceeds as designed; per counsel #4 the money lane uses the **named veneer `withCourierTx`** so a wrong-GUC-family mistake is a compile error, not a runtime hope |
| BN-2 | note | recorded | No deploy-time outage: MIG-1..4 inert on the BYPASSRLS pool — confirmed; all RLS breaks are flip-time; the design's gates are therefore all flip-side |

**DEFER-FLAG register:**
- **MIG-1b** — tenant-scoped courier visibility policies `TO dowiz_app_rls`
  (`couriers: id IN (SELECT courier_id FROM courier_locations WHERE …)`,
  `courier_sessions: active_location_id = <tenant>`). Not needed for flip-correctness of auth
  (firebreak covers bare `dowiz_app`); **required before the B3 courier-lane flag enables**.
  Owner: B3 lane. Re-entry trigger: courier-lane `RLS_ENFORCE_COURIER` flag work starts.
- **OPEN-V1** — pg-boss 10.4.2 `updateQueue` policy-change support: 30-min spike in Lane A′
  before the reconciler is coded; both outcomes have a designed path (reconciler vs
  `singletonSeconds` fallback).
- **OPEN-V2 / OPS-READ-CHECK** — MIG-1 takes effect **immediately** for the already-NOBYPASSRLS
  operational read pool (`deliveryos_operational_user`, `1790000000015:19`). Precondition before
  MIG-1 applies anywhere: verify zero legitimate operational-pool reads of
  `couriers`/`courier_sessions`/`courier_locations` (scripted grep + staging soak). If any exist:
  convert them or add a scoped read policy for that role — decided at implementation, gated.

**ACCEPT-RISK register:**
- **`failed` status label kept** (counsel #3b suggested renaming). Renaming means a CHECK-constraint
  migration + code churn for a semantic nicety. Risk accepted because the pathology it guards
  (triage-closed-as-ordinary-failure) is compensated three ways: O-GDPR is **level-triggered**
  (re-fires every reconciliation run while any `failed`/overdue row exists — a `failed` row cannot
  go quiet), the owner GDPR list surfaces `failed` as requires-action, and a 72h resolution SLA is
  bound to the operator. Owner: operator. Revisit if a `failed` row ever ages past SLA unnoticed.

---

## 2. Counsel ETHICAL-STOPs — resolutions

### ES-1 (LC4 must not inherit the flip lane's latency) → **REVISED — adopted in full; one needs-human ratification**
- **Lane A ships now, standalone**, with no dependency on any conversion or migration in the flip
  lane: LC4 redesign + LC4-MIG (policy arms + DEFINER claim fn — the lane's only operator gate) +
  the anonymizer v10 array fix + **pg-boss boot isolation** (counsel #5: the O-GDPR detector must be
  un-amputatable *first*) + real `.dlq` wiring for the GDPR queue + O-GDPR reconciliation check +
  owner-facing terminal surface.
- **Dead-letter is not terminal-with-no-owner:** (a) O-GDPR is level-triggered (re-alerts every run
  until the row is resolved); (b) resolution owner = **operator**, SLA = **72h** from `failed`,
  runbook entry added (`docs/backup/runbooks.md` gets the GDPR-failed procedure); (c) the owner GDPR
  list (`routes/owner/gdpr.ts` — surface already exists) renders `failed` as *requires action*, and
  a terminal-state notification goes to `owner_notification_targets`.
- **Art. 12 receipt (the counsel's open question #5):** v1 receipt is **controller-facing** — the
  *owner* is the GDPR controller (dowiz is processor); on terminal state the request record carries
  a machine-readable receipt (status, requested_at, completed_at, scope of erasure, failure reason
  if any) retrievable via the existing GET endpoint, plus the owner notification above; the owner
  relays it to the subject. **Direct-to-subject delivery is needs-human** (see §4) — after erasure
  the system may no longer *hold* the subject's contact channel; automating a message to data we
  just erased is itself a retention/PII decision no agent should default.
- **needs-human:** the standalone Lane-A go (its migration is a red-line) must be a **recorded
  operator decision separate from any approval of Lane B** — exactly what ES-1 demanded. This
  resolution stages it; it does not grant it.

### ES-2 (false belief: "deferred B3 flip isolates credentials") → **REVISED — correction of record written; open-source gate added; needs-human acknowledgment**
- **Correction of record (also §9 of proposal.md):** as staged today, the deferred
  `ALTER ROLE dowiz_app NOBYPASSRLS` flip isolates **nothing** on `couriers` (password_hash,
  encrypted PII) and `courier_sessions` (token_hash, family_id) — those tables have **no RLS
  enabled anywhere** (`1780421029538`, `1780421032856`); a table without RLS has no policies for
  the flip to enforce. Any prior statement or belief that "B3 staged ⇒ credential tables isolated"
  is **false** and is hereby corrected. Same for the ENABLE-only token-bearing tables
  (`customer_track_grants`, `provision_grants`, `claim_invites` — MIG-4 scope).
- **New named gate GATE-OSS-RLS:** the ADR-020 open-source flip may not proceed on any isolation
  premise until **MIG-1 (firebreak form) and MIG-4 have landed and P1 + P1b are green**. This is
  additive to the existing open-source gates (secrets scrub + EUTM filing per memory).
- **needs-human:** the operator signs the correction (one line in the decision log) so the false
  belief cannot silently survive into the irreversible open-source act.

### Counsel non-blocking advice — all five adopted
#1 three-lane split (below) · #2 MIG-2/3 hard edge mechanized as **GATE-ANON-E2E** (reuse the
staging anon-checkout E2E as a *named blocking precondition* — no new machinery) · #3 adopted as
ES-1 items (a)(c) + ACCEPT-RISK on (b) · #4 `withCourierTx` named veneer on the money lane ·
#5 boot isolation rides Lane A. The counsel's ALS steel-man is harvested as the **hybrid**: the
explicit helper stays the only production mechanism; an **ALS-backed dev/test tripwire** ("a
FORCE-table query ran with no context in scope → throw in test env") generalizes §1.5.4 and ships
in Lane 0.

---

## 3. Revised scope split (what ships now vs flip-gated)

| Lane | Contents | Flip dep | Gate |
|------|----------|----------|------|
| **0 — enabler (now, small)** | `withTenantTx` (+ `anonymous` ctx, + `withCourierTx` money veneer) · `no-bare-set-config` lint (lands red) · ALS dev/test no-context tripwire | none | lint red→green trajectory starts; helper unit tests |
| **A — legal + detector integrity (now, fast-track; ES-1)** | LC4 redesign (DEFINER claim, claim-token CAS, targeted retry, metadata `claimed_at` reclaim) · LC4-MIG (policy arms + `gdpr_claim_due`) · anonymizer v10 array fix · per-registration boot isolation + boot registry watch-set · `.dlq` wiring + DLQ monitor for the GDPR queue · O-GDPR level-triggered check · owner terminal surface + notification + receipt fields | none | P5 (extended) · P7 (extended) · operator go recorded separately (ES-1) |
| **A′ — queue-policy sweep (follows A, no flip dep)** | OPEN-V1 spike → queue-policy reconciler (or `singletonSeconds` fallback) · `QUEUE_POLICY` map · retry/backoff/expire defaults · remaining direct-`boss.work` array fixes · generic `.dlq` monitor for all queues | none | P6 (rewritten) |
| **B — flip preconditions (large, staged, operator-gated per step)** | 49-site conversion + the FORCE-table readers the grep missed (orders.ts checkout, customer/track.ts, plugins/auth.ts, courier/auth.ts, telegram tenant-resolution) · telegram txn-boundary redesign · MIG-1 (firebreak form; after OPS-READ-CHECK) · MIG-4 · public-lane conversion · MIG-2 · MIG-3 · B3 per-lane ramp preconditions | **all of it** | P1 · P1b · P2 · P3 · P9 · P10 · GATE-ANON-E2E · GATE-FLIP-E2E |

Named deploy gates (deterministic, not prose):
- **GATE-ANON-E2E** — MIG-2/MIG-3 may not apply to prod unless the staging anon-checkout + track
  E2E is green on staging **with MIG-2/3 already applied there** (and staging enforcement
  rehearsal on).
- **GATE-FLIP-E2E** — the NOBYPASSRLS flip (staging Phase 3, prod Phase 4) requires the full
  3-role lifecycle E2E green on flipped staging + P9 suite green + P10 = 0 unconverted.
- **GATE-OSS-RLS** — ADR-020 open-source flip requires MIG-1 + MIG-4 landed, P1 + P1b green (ES-2).

---

## 4. needs-human register

1. **Lane-A standalone go** (LC4-MIG is a migrations red-line) — recorded separately from Lane B (ES-1).
2. **ES-2 correction sign-off** — one decision-log line acknowledging the credential tables are not
   isolated today and GATE-OSS-RLS binds.
3. **Direct-to-subject erasure receipt channel** — product + PII decision (contact data may itself
   be erased); v1 ships controller-facing only.
4. **MIG-1..4, LC4-MIG prod application** — each individually operator-gated (standing red-line rule).

## 5. Updated proof list (delta from v1 — full table in proposal.md §5)

- **P1** extended: isolation asserts now include *deny-by-default for non-`dowiz_app` roles* on
  couriers/courier_sessions (firebreak semantics), not tenant-scoping.
- **P1b (new, kills the F1 class):** under a NOBYPASSRLS'd `dowiz_app` probe — courier login SELECT
  by email_hash returns the row; session-validation query (incl. the `courier_locations` EXISTS)
  succeeds; **fails if MIG-1 is ever rewritten tenant-scoped.** The proof of the key, not the lock.
- **P2** pinned to the flip-rehearsal DB (all MIGs + enforcement on) — end-state validity, no
  ordering-dependent false-green.
- **P5** extended: + CAS concurrency proof (double-trigger ⇒ exactly 1 audit row + 1 event) ·
  + stale-reclaim both ways (aged `claimed_at` reclaimed; in-window active row NOT reclaimed) ·
  + O-GDPR level-trigger (asserts a second reconciliation run re-fires on an unresolved `failed` row).
- **P6** rewritten against a pre-existing `standard` queue + reconciler (F8's false-green removed).
- **P7** extended: watch-set = actually-started registry; injected registration failure ⇒ later
  workers live + boot DRIFT + **no false liveness DRIFT**.
- **P8** amended: permissive-policy hygiene = "references the tenant key **OR** is role-restricted
  (`TO <role>`) and on the firebreak allowlist (users, auth_refresh_tokens, ops_worker_heartbeat,
  couriers, courier_sessions, courier_locations-SELECT)".
- **P9 (new):** flip-rehearsal integration suite — anon checkout INSERT, track-token exchange,
  courier login + session validation, owner login — under enforcement. The falsifiable
  "public-lane conversion complete" gate (F2).
- **P10 (new):** `scripts/rls-force-access-scan.mjs` CI gate — every reader/writer of a
  FORCE/to-be-FORCE table is helper-wrapped or allowlisted-with-reason; UNCONVERTED = 0 before
  MIG-2/3-prod and before any flip step. A new bare reader turns CI red.
- **P11 (new):** telegram txn-boundary proof — with the Telegram API mocked, assert COMMIT/release
  happens **before** the first external call (spy ordering), and the 409 path answers from its own
  short txn.

## 6. Six-line summary

1. Both breaker CRITICALs are **FIXED by redesign**: MIG-1 now mirrors the real firebreak
   (FORCE + `ops_all TO dowiz_app` — couriers/courier_sessions are credential tables, proven by new
   P1b "the key still turns"), and the conversion gate is falsifiable (P9 rehearsal suite + P10
   FORCE-access scan + GATE-ANON-E2E) instead of a grep-bounded inventory.
2. All three LC4 contradictions are **FIXED**: DEFINER claim + `app.current_tenant` policy arm
   (F3), `metadata.claimed_at` + claim-token CAS instead of a phantom `updated_at` (F4), and
   CAS-gated side effects (F7); "no schema change" honestly restated as one additive migration.
3. Telegram/provisioning are **FIXED by txn-boundary design**, not naive wrapping: no DB txn ever
   spans external HTTP (chat-resolved tenant → short txn → post-commit sends, P11), and multi-write
   flows keep today's best-effort semantics via phase txns + savepoints.
4. pg-boss: **FIXED** — `.dlq` canonical, queue-policy reconciler with P6 rewritten against a
   prod-mirror `standard` queue (one OPEN-V1 spike + designed `singletonSeconds` fallback),
   v10-pin scope honestly includes the direct-`work` array bugs, watch-set from the boot registry.
5. Both ETHICAL-STOPs **REVISED-in**: Lane A (LC4 + boot isolation + `.dlq` + owned dead-letter +
   controller-facing receipt) ships now decoupled; the ES-2 correction-of-record is written and
   GATE-OSS-RLS binds the open-source flip to MIG-1/MIG-4 + P1/P1b.
6. Still open (flagged, owned, none blocking Lane 0/A): MIG-1b (`dowiz_app_rls` courier policies —
   B3 courier-lane), OPEN-V1 (v10 `updateQueue` spike), OPS-READ-CHECK (pre-MIG-1), and four
   needs-human items (§4).
