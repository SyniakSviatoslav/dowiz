# Deep Architecture "Basement" Attack — dowiz / DeliveryOS

**Date:** 2026-07-03 · **Type:** READ-ONLY foundational structural attack (no code changed)
**Scope:** Beneath the 16 findings of `audit-architecture-2026-07-03.md` and the 4 roots of
`AUDIT-SYNTHESIS-2026-07-03.md` — this doc attacks the *load-bearing assumptions those findings
rest on*. It does not re-report them. Every claim is grounded in a real `file:line`.
**Method:** first-hand reading of the foundation files (packages/db, packages/platform message-bus,
packages/platform tenant seam, server.ts boot sequence, schema-guard, migrations, config) + 3 parallel
deep lanes (realtime durability, deploy/reversal, safety-mechanism accretion). Corroborated against
`recon3-privacy-ops-2026-07-03.md`.

> **The reframing.** The prior audits' verdict was *"a sound skeleton wearing unverified flesh —
> partial adoption of good primitives."* That is true one layer up. One layer *down*, the sharper
> finding is: **several of the "good primitives" are themselves structurally incomplete or internally
> contradictory** — not merely under-adopted. The tenant floor is not one floor with 22/65 coverage;
> it is *three partial floors that do not compose*. The realtime "bus" is not Redis-under-a-different-
> name; it is a lossy transport with *zero durability contract* whose name hides that. The type
> "contract" is not an unwired-but-correct system; the two ends are *two type systems that agree by
> luck*. This changes the remediation math: some of these are not "finish adopting the primitive" —
> they are "the primitive as poured has a crack, decide whether to patch or re-pour."

---

## B1. Tenant isolation — the floor is not app-only, it is THREE partial floors that don't compose

**Foundational assumption:** *"Tenant safety = every query carries a tenant predicate, enforced by
developer discipline; RLS is a future backstop we flip on later."* The synthesis framed this as
"app-layer-only, 22/65 coverage." The deeper truth is worse in one way and better in another.

**What is actually in the basement (evidence):**
- The RLS floor **physically exists** — 150 `ENABLE/FORCE ROW LEVEL SECURITY` statements and ~131
  policies across the migrations (`1780310074262_orders.ts:81-98`, `1780310071220_core-identity.ts:82-102`,
  `1790000000077_rls-nobypassrls-phase1-policies.ts`). It is not missing. It is **inert** because the
  operational pool connects as a `BYPASSRLS` role (`1780691681296_ops-location-alerts-policy.ts:8`
  `ALTER ROLE deliveryos_api_user BYPASSRLS`). The pool guard (`packages/db/src/index.ts:32-39`) only
  rejects the literal `postgres` superuser — it **admits any BYPASSRLS role**, so it does not actually
  guarantee RLS is in force. The floor is poured but the switch is off.
- **The floor is keyed on two incompatible GUC namespaces that were never unified.** The core/owner
  tables isolate via `app.user_id` → `app_current_user()` → `app_member_location_ids()` (membership-
  derived): `orders` policy `location_id IN (SELECT app_member_location_ids())`
  (`1780310074262_orders.ts:83`). But **couriers are not members** (no `memberships` row —
  `1790000000077...:45` comment), so courier/worker/settlement/payments code sets a *different* GUC,
  `app.current_tenant`, and the Phase-1 policies key on it:
  `USING (location_id = NULLIF(current_setting('app.current_tenant', true),'')::uuid)`
  (`1790000000077...:49`). A third identity model exists for anonymous checkout: `anonymous_insert`
  policies keyed on `app_current_user() IS NULL` (`1790000000077...:18-22`).
- **The shared tenant seam does not satisfy its own floor.** `withTenant` (`packages/platform/src/auth/tenant.ts:11`)
  sets **only** `app.user_id`. It therefore satisfies the membership policies but **cannot** satisfy the
  `app.current_tenant` policies. So a single hot table (`orders`) has policies spanning three identity
  models, and the one shared helper covers exactly one of them. The other two are hand-rolled
  `set_config('app.current_tenant', …, true)` scattered across ~15 files (`courier/assignments.ts` ×8,
  `courier/shifts.ts` ×4, `owner/couriers.ts` ×3, `workers/*` ×7, `payments-webhook.ts`, `telegram-webhook.ts`
  ×3, `owner/signals.ts`, `public/funnel.ts`, `spa-proxy.ts`), each with its own BEGIN/COMMIT discipline
  (or lack of it — recon flagged `assignments.ts:109` `set_config` with no transaction to bind `true` to).

**What rests on it:** every multi-tenant safety property; the entire NOBYPASSRLS remediation program
(migrations `...015`, `...077`, security §E); the ~777 raw queries; the money and PII red-lines
(cross-tenant PATCH IDOR, cross-tenant PII erasure — synthesis LC2/LC5 are *symptoms* of this root).

**Failure mode + blast radius:** *Today* — one forgotten predicate = silent cross-tenant read/write
(no DB net). *At the flip* — the flip is **not a config toggle**; it is gated behind unifying two GUC
namespaces + auditing ~40 `set_config` sites for correct-GUC-per-table + correct transaction binding.
Flip with the namespaces still split and every `withTenant`-scoped owner route that transitively touches
a courier/settlement table returns **zero rows** (membership GUC set, tenant GUC empty) — a fleet-wide
silent-empty-result outage that passes every health check. Recon3/synthesis already show couriers'
customer-status pushes die (autocommit GUC no-op) and session GUC can leak across the pooled connection.

**Incrementally fixable, or re-pour?** **Re-pour the seam, keep the policies.** The policies are
salvageable. The *seam* (`withTenant`) is not — it encodes the wrong single GUC. The correct base is
one `withTenant(pool, principal)` where `principal` is a discriminated union `{owner,userId}` |
`{courier,shiftLocationId}` | `{anonymous}` that sets the right GUC(s) inside one audited BEGIN, and a
`NOBYPASSRLS`-by-construction operational role so "forgot the predicate" is a caught error, not a
silent leak. This is a base-level change to the data-access *entry point*, executed incrementally per
surface behind the existing repository refactor (arch F2).

**Highest-leverage intervention (this lane):** **Collapse the two GUC namespaces into one principal-
typed `withTenantTx` and make the operational role `NOBYPASSRLS` the default** (with a break-glass
BYPASSRLS role for migrations only). One helper, one GUC contract, DB-enforced by construction. Until
the two namespaces are one, the flip cannot be attempted safely and the app-layer floor is the *only*
floor — which is the real answer to the prompt's question: **the chosen base is not incrementally
"flippable"; the aspirational floor is itself internally inconsistent and must be unified before it
can bear load.**

---

## B2. Realtime nervous system — not "Redis by another name," but a transport with NO durability contract

**Foundational assumption:** *"Publishing an event and committing the business write are effectively
one thing; if I NOTIFY after COMMIT, subscribers will see it."* The name `RedisMessageBus`
(`packages/platform/src/message-bus.ts:242` `= PgMessageBus`, "aliased to make the test pass") hides
that the transport is Postgres LISTEN/NOTIFY — **fire-and-forget, zero durability, no ack, no replay,
no outbox** (`message-bus.ts:14-240`).

**What is actually in the basement (deep-lane evidence):**
- **Commit and publish are two non-atomic steps on two different connections.** `publish()` NOTIFYs on
  the pool (`message-bus.ts:121-127`), never the business-tx client. Good news: an event cannot fire
  inside an uncommitted tx. Bad news: **every hot call site publishes *after* COMMIT** (`orders.ts`
  POST COMMIT:579 → publish:584/593/606; `courier/assignments.ts` :224→227, :274→276, …; `owner/dashboard.ts`
  :335→336). A crash or NOTIFY error in the window = **event lost forever**, and publish errors are
  **swallowed** (`message-bus.ts:128-130`, catch→console.error, request still 200), behind a 5s
  `Promise.race` that abandons but never cancels the query.
- **No gap recovery.** One listener client per process (`message-bus.ts:16,43`); reconnect is
  indefinite backoff (`:95-114`, the comment admits a prior 5-attempt cap left machines "realtime-
  dead"), but **NOTIFYs delivered while the listener is disconnected are discarded by Postgres — there
  is no replay path**. A client whose WS stays up while only the server's Pg listener drops gets *no*
  reconnect signal → permanently stale UI until manual refresh (dashboards have `onReconnect: fetchOrders`
  but **no poll interval** as a backstop).
- **The 8KB cap is handled by truncation into a dead-end.** `serializeForNotify` (`:140-154`) emits
  `{_truncated:true, type, data:{id}}` — but **no consumer handles `_truncated`** (zero hits in
  `apps/web/src`); a truncated `order.status` frame merges nothing and schedules no refetch.
- **Bus pool = session pool `max:3`** (`server.ts:220-229` → `createSessionPool()`; `packages/db/src/index.ts:48-54`).
  The listener permanently holds 1 of 3; **2 connections serve every publish + health/metrics**. A
  publish burst → pool wait → 5s timeout → swallow → loss. (Correct mode: LISTEN needs :5432 session
  mode; if `DATABASE_URL_SESSION` were ever pointed at the :6543 transaction pooler, LISTEN silently
  breaks — a one-config-line cliff.)

**What rests on it:** three **state-bearing** subscribers, not just UI. `LifecycleHandlers`
(`workers/lifecycle-handlers.ts:26-36`) resolves dwell alerts and **cancels pg-boss escalation jobs**
off bus events — lost event → alert never resolves, escalation still fires. `bootstrap/messaging.ts`
enqueues Telegram + customer-push **only** on bus arrival (`:62,85,134`) — lossy NOTIFY is the trigger
for durable jobs. `CourierEventsWorker` persists `order_routes`/ETA off `COURIER_POSITION_UPDATED`.
(Dispatch itself is correctly a durable `courier_dispatch_queue` journal + 60s sweep — bus-independent.)

**Failure mode + blast radius:** any listener-drop window (deploy, Pg restart, Supabase pooler recycle)
silently drops every event in it → unseen paid orders on the owner dashboard, unresolved dwell alerts,
missing customer status pushes — all with 200 responses and green health. **The "aliased to pass the
test" origin is load-bearing: the real transport has ZERO adversarial coverage.** Both platform tests
use a stub pool (`message-bus-notify.test.ts:26-32`, `message-bus-dispatch.test.ts`) and assert the
NOTIFY SQL string / truncation math — no test touches a real LISTEN/NOTIFY, drop, reconnect, >8KB
end-to-end, or rollback visibility. The tests never validated the thing that ships.

**Incrementally fixable, or re-pour?** **Incrementally fixable — no rewrite; the correct pattern is
already in the repo twice** (transactional enqueue in `orders.ts:541-543` `insertOrderWithItems(client,
queue,…)`; the dispatch journal+sweep). LISTEN/NOTIFY is *sound as UI fan-out* at this scale. It is
*unsound as the trigger for anything durable*.

**Highest-leverage intervention (this lane):** **Move the three state-bearing subscribers off the bus
onto pg-boss jobs enqueued inside the business transaction** (transactional outbox using infra already
running in-process), leaving NOTIFY purely cosmetic; add one dashboard poll interval as the WS
reconciliation backstop. Converts every lossy edge to at-least-once. Then rename the alias.

---

## B3. The type "contract" — two type systems that agree by luck, and the enforcement seam is already installed but never fed

**Foundational assumption:** *"FE and BE share a typed contract, so a shape change on one side is
caught on the other."* False in substance. `packages/shared-types/src/contracts/` exports **131
schemas across 27 files with ZERO importers in `apps/api/src`.** The live contract is mislabeled
`legacy.ts`. Where a contract *is* used, only the **FE** parses it; the API validates a *different*
inline `z.object` (42 route files hand-roll their own).

**The deeper structural fact (evidence):** **the enforcement seam is physically installed and
partially used — it was just never fed the shared contracts.** The project depends on
`fastify-type-provider-zod`; `ZodTypeProvider` is imported and 115 `schema:{…}` blocks exist across
routes (`telegram-webhook.ts:2,794`, `public/theme.ts`, `auth/local.ts`, …). So routes *can* and
*sometimes do* server-validate — but always against a local inline schema, never the shared
`contracts/*`. **Responses are mostly unvalidated on both ends:** 18 handlers `send` raw `RETURNING *`
rows (snake_case, un-parsed) — validated by neither the API nor the FE. This is the precise anatomy of
"agree by luck": FE types against `contracts/`, API validates a hand-copy, response validated by no
one, and the three drift independently (the live `valid_from` required-vs-optional and `order_type`
2-vs-3-values disagreements in arch F3/F4 are the observable proof).

**What rests on it:** every FE↔BE data exchange; ~18 WS event types matched by duplicated string
literals with **no shared discriminated union** (server emits `task_offered`
`owner/dashboard.ts:336`, courier client handles only `task_assigned` `TasksPage.tsx:72` → offered
tasks never appear live). 131 schemas of dead-but-authoritative-looking documentation give false
confidence to every new contributor.

**Failure mode + blast radius:** silent shape drift that no gate catches — a BE field rename, unit
change, or casing flip ships green and manifests as a wrong number / missing live update in
production (synthesis frontend #12/#13 — "500 ALL" rendered "5 ALL", minor-units → 100× prices). As
the system grows, the FE and BE diverge monotonically because each side owns its own copy and nothing
reconciles them.

**Incrementally fixable, or re-pour?** **Incrementally fixable — the seam is already there.** This is
the cheapest structural win in the whole basement: pick ONE source of truth and feed the *existing*
`fastify-type-provider-zod` `schema.body`/`schema.response` blocks with it, so the FE
`apiClient({schema})` and the server validate the **same object**; add `contracts/ws-events.ts`
`z.discriminatedUnion('type', …)` typed on both `publish` and `useWebSocket` ingress.

**Highest-leverage intervention (this lane):** **Wire the chosen contracts into `schema.response`
(not just `schema.body`) on the money/order/status routes first** — response validation is the half
that is entirely absent today and is where the silent 100×/wrong-tenant/wrong-status bugs escape.

---

## B4. The god-hub topology is STRUCTURAL, not accidental — the core domains never got the vertical slice the codebase already knows how to build

**Foundational assumption:** *"Routes organized by role folder + a shared lib/ is enough structure;
features slot in naturally."* The churn data falsifies it: `server.ts` (890 lines, 93 commits/90d),
`MenuPage.tsx` (1768), `CheckoutPage.tsx` (65 commits), `spa-proxy.ts` (885 lines, 59 commits, "proxies
nothing" — ~19 owner endpoints), `MenuManagerPage.tsx` (1292), `i18n-catalog.ts` (**4296 lines, one
flat file**, 46 commits). These are the top churn files *because they are the only seam* — every owner
API feature must edit `spa-proxy.ts`; every string must edit the one flat catalog; every menu change
must edit the 1768-line page.

**The deepest evidence that this is structural, not accidental:** the codebase **already contains the
correct pattern, applied exactly once.** `apps/api/src/modules/acquisition/` is a real vertical slice
— `state-machine.ts` + `service.ts` + `route.ts` + `ops-auth.ts` + `claim.ts` + `provisioning.ts` +
`types.ts` + `menu-extractor.ts` co-located by *domain*, not by *layer*. The team knows how to draw a
module boundary. The core domains (order, menu, owner, courier) were simply never refactored to match —
they remain **horizontal god-layers, each a single file**: one route god-file (`spa-proxy`/`orders`),
one FE page god-file (`MenuPage`/`MenuManager`), one i18n god-file, no repository, no contract. Adding a
feature = touching all of them = merge contention = churn. The `modules/` dir proves the missing seam is
a *choice not yet made*, not a capability the team lacks.

**What rests on it:** every feature's velocity and every merge; the acquisition module is *cheap* to
extend, the core is *expensive* — the churn asymmetry is the daily tax.

**Failure mode + blast radius:** not an outage — a **velocity and correctness tax**. Two owner-API URL
conventions because there is no single home (arch F1); safety-critical allergen logic inline at
`MenuPage.tsx:118-188` inside a 1768-line file where it cannot be unit-tested; the flat i18n catalog
ships all admin+courier strings to the public client bundle. God-files are where bugs hide and reviews
fail.

**Incrementally fixable, or re-pour?** **Incrementally fixable, and the target already exists.** Not a
rewrite — a *migration toward the pattern the acquisition module demonstrates*: `feature = (route plugin
+ repo + contract + FE hook + i18n namespace)`. Do it per domain, highest-churn first (menu, then owner
via dissolving spa-proxy).

**Highest-leverage intervention (this lane):** **Adopt the acquisition-module vertical-slice shape as
the mandated unit for new features, and carve the menu domain out first** (`modules/menu/` with repo +
contract + a `<ProductFormModal>`/`useProductForm` reducer that kills the MenuManager churn root, and an
`i18n/menu.*` namespace). One worked example converts an implicit choice into an enforced standard.

---

## B5. No structural "undo" at any layer — and the one guard that exists actively obstructs the only real rollback

**Foundational assumption:** *"Every migration is additive/backward-compatible, so old-code-on-new-
schema is always safe" — and therefore image rollback is a safe undo.* Nothing enforces the premise the
whole reversal story rests on.

**What is actually in the basement (deep-lane evidence):**
- **Schema undo exists on paper, cannot execute, and the documented path is a trap.** 157 migrations;
  **62 have empty no-op `down()`**, 95 have SQL bodies, 0 throw-stubs — but no prod path runs any
  `down()`: the bundled runner hardcodes `direction:'up'` (`scripts/migrate-runner.ts:65`) and
  `--no-check-order` (`:74`). The only `migrate:down` reference is a *runbook rollback step*
  (`docs/audit/RELEASE-GATE.md:40`) which, against a no-op down, **deletes the pgmigrations head row
  without reverting DDL** → the next deploy re-runs `up()` inside `singleTransaction:true` and aborts
  every subsequent rollout. The documented rollback corrupts the bookkeeping.
- **Two migration lineages, no equivalence assertion.** Prod is incremental with order-checking off and
  **2 platform migrations never recorded** (`migrate-runner.ts:70-73`); fresh-provision runs *with*
  order-checking on a bare PG with roles hand-created in CI (`verify-fresh-provision.sh:99`, `ci.yml:95-101`).
  **Nothing diffs the two schemas** (no `pg_dump --schema-only` comparison). The CI-proven schema is
  definitionally not prod's.
- **No staging gate in CI.** `ci.yml:133-135` `deploy: needs: validate` only — `fresh-provision` is
  **not** in `needs`, and `validate` runs **zero `pnpm test`**. Migrations apply **twice and early**:
  CI runs `migrate:up` against prod (`ci.yml:150-153`) *before* `flyctl deploy` (:157), so new schema is
  live under **old code** for the whole build+rollout window, then `release_command` re-runs.
- **No data undo.** Zero soft-delete across 157 migrations (no `deleted_at`); menu import hard-DELETEs
  (`menu-import.ts:453-469`), anonymizer overwrites in place. Order status + money *are* reconstructible
  (`order_status_history`, payments ledger) — menu and PII are final.
- **The boot-guard obstructs the only real undo.** `schema-guard.ts:38-47` asserts the expected head is
  *present* ("ahead is fine") and **fails open on any transient error** (`:48-56` warn+continue). So it
  does not forbid image rollback (old code's expected head is still present in an ahead DB) — but if an
  operator first runs the `RELEASE-GATE.md:40` `migrate:down`, the head row vanishes and the *new* image
  then FATALs as "behind" (`:59-65`) while old boots — confusing recovery in **both** directions. And it
  checks head *name* presence, not schema *shape*, so an ahead-schema with a destructive migration passes
  the guard and breaks old code anyway.

**What rests on it:** the ability to recover from a bad deploy at all. Image rollback is the *only*
real undo (downs unexecutable, backups inoperable per synthesis LC7, no soft-delete) — and its safety
rests entirely on the unenforced additive-only assumption.

**Failure mode + blast radius:** one destructive migration (DROP/RENAME/SET NOT NULL) → old code 500s
during the deploy window **and** after any rollback attempt; blast radius = full prod outage with no
working undo at any layer. The system is, today, **un-operable-under-failure by construction.**

**Incrementally fixable, or re-pour?** **Incrementally fixable — every gap closes with a lint, a
`needs:` line, a runbook edit, and a Dockerfile line.** No rewrite.

**Highest-leverage intervention (this lane):** **A CI gate that enforces expand/contract** (reject
DROP/RENAME/SET-NOT-NULL-without-default in new migrations). It converts image-rollback from "safe by
convention" to "safe by construction," retroactively validating the only undo the system has. Paired
cheap follow-ups: delete the `migrate:down` instruction from the runbook and replace with a pinned
previous-image-digest rollback; add `fresh-provision` + a real test step to `deploy.needs`; put
`postgresql-client` in the Dockerfile so one restore drill can pass.

---

## B6. Safety-mechanism accretion — no boot-phase contract; TWO opposite failure philosophies wired at once

**Foundational assumption:** *"Each guard is individually reasonable, so adding them makes the system
safer."* They compose into fragility because there is **no single authority that decides what makes the
process refuse to boot, degrade, or die** — guards were added without reconciling the ones they
supersede.

**The single deepest contradiction (direct evidence):** the process wires **two opposite failure
philosophies simultaneously.**
- **Swallow-everything at the event-loop level:** `server.ts:73-80` registers
  `process.on('unhandledRejection'|'uncaughtException')` handlers that **deliberately suppress Node's
  crash-and-exit** — "keep serving … suppresses Node's default crash-and-exit." A corrupted-state
  process keeps taking traffic instead of crashing and letting Fly restart it clean.
- **Die-hard at the guard level:** `schema-guard.ts:65` `process.exit(1)`; `server.ts:886`
  `process.exit(1)` on listen failure; recon3 **O-H1** — `assertAccessRequestSchedules`/
  `assertDeliveryTraceSchedule` `process.exit(1)` fire **after** `fastify.listen()` (`server.ts:877`),
  so a machine goes green on `/livez` (200 the instant listen resolves), takes traffic, then exits —
  a **fleet-wide "goes-green-then-dies" crash-loop.**

These defeat each other: the swallow-guard *disables* the restart-based recovery that the die-guards
and Fly assume, and `/livez` (a bare listen-succeeded probe) **cannot distinguish a healthy process
from a swallow-corrupted one.** Meanwhile the *same* class of failure is handled three ways: migration
`1790000000042:83-93` **swallows** an `insufficient_privilege` pgboss failure and proceeds, while the
post-listen assert that depends on that queue **fatally exits** (recon3 O-H1) — "tolerate & continue"
vs "die if missing" for one condition. The schedule-creation failure is in fact tolerated *twice* then
killed: swallowed at layer 1 (`.catch→warn`, `access-request-retention.ts:47-55`), swallowed at layer 2
(the 3s worker boot-budget, `server.ts:344-350`), then **fatal** at layer 3 post-listen.

**The keep-alive handler silently neuters the one guard that matters most — the RLS check.** The
operational-pool RLS guard (`packages/db/src/index.ts:32-38`) `throw`s inside an **async `'connect'`
event handler** — a *post-listen, per-connection, runtime* throw. That throw becomes an
`unhandledRejection` → **swallowed by `server.ts:73-76`**. Net effect: a superuser/BYPASSRLS connection
produces an infinite connect-destroy churn plus a console line **while the process keeps serving** — the
security guard is demoted to a log message by the sibling keep-alive handler, and it only matches the
literal `postgres` role anyway (any other BYPASSRLS role passes). **Two opposite failure policies for
the same condition also appear at the post-listen asserts vs the schema-guard:** a transient failed
`pgboss.schedule` *read* leaves the assert's set empty → both names "missing" → `process.exit(1)`
(`access-request-retention.ts:150-163`) — a **DB blip after listen kills prod** — while `schema-guard.ts:53`
treats the identical transient error as warn-and-continue. And a **startup race** exists: the boot
budget is 3s (`server.ts:344`) but the retention worker starts last (`bootstrap/workers.ts:145-146`), so
on a fresh DB the post-listen assert can run *before* the schedule row lands and exit(1) for a condition
that self-heals seconds later — survived today only because `pgboss.schedule` rows persist from prior
boots. (The **worker process has no schema guard at all** — `apps/worker/src/index.ts` serves the same
DB with none of the web process's boot assertions.)

**Config/flag floor is porous — the newest features opted out of the boot contract entirely.** The Zod
schema declares ~**118 keys**, but **46 distinct keys are read raw via `process.env` across 37 files**
in apps/api, and **19 of those 46 are absent from the schema entirely** — including two *secrets*
(`PLISIO_SECRET_KEY`, `PROVISION_OPS_SECRET` `server.ts:545`) and the whole newest flag generation
(`PAYMENTS_*` `lib/payments/registry.ts:6-7,14,16`, `VOICE_*` `lib/voice-flag.ts:12` whose comment at
`:8` *explicitly rejects* the schema — "fail-closed without relying on a schema default", `METRICS_TOKEN`,
`COURIER_OFFER_*` `owner/dashboard.ts:323,329`). The other 27 are in-schema but read raw, skipping
coercion/defaults. The boot contract validates a config the runtime then half-ignores; a typo in an
uncontracted key silently disables payments/voice with no boot failure (fail-closed masks the
misconfig). Observability itself is opt-in with no prod guard (recon3 O-M5: `SENTRY_DSN`/`METRICS_TOKEN`
optional, no default) — prod can boot fully blind. Two more silent-config cliffs: JWT keys are checked
for *presence only* (`platform/src/auth/jwt.ts:14-22`) with **no keypair-match boot check** → a bad
rotate = all auth dead post-deploy, ungated; `BACKUP_ENCRYPTION_KEY` is schema-optional (`config:71`) but
`backup-hourly` is on `WORKER_CRITICAL_LIST` (`config:123`) and the missing key throws only *inside the
cron job* (`workers/backup/index.ts:111-112`) → silent no-backups while "critical."

**Drifting secret/config stores, no arbiter — five stores, two migration runners.** Fly secrets
(runtime SoT); a world-writable local `.env` (22 keys, feeds `migrate:up --envPath .env`,
`package.json:38` — subject of the active secrets-exposure incident); GitHub secrets; a SOPS/age vault
**documented as SoT but a scaffold only** (`.sops.yaml:12` still `age1REPLACE_WITH_FIRST_DEV_PUBLIC_KEY`,
no `.enc.env` in `secrets/` — recon3 O-M7); and hardcoded fallbacks. **`DATABASE_URL_MIGRATIONS` exists
in 3 copies (Fly / GitHub / .env) driving 2 independent migration runners** — CI's `pnpm migrate:up`
(`ci.yml:151-153`) with the GitHub copy, then Fly's `release_command` with Fly's copy — so a drift =
migrating the wrong DB, with **no detector**. Nothing keeps copies in sync; the purpose-built
drift/preflight guards (`ci-migration-preflight.mjs`, `ci-schema-drift.mjs`, `ci-connection-preflight.mjs`)
are **orphans** wired to nothing (recon3 O-M2).

**What rests on it:** whether the fleet boots, degrades, or crash-loops under any partial failure; the
operator's ability to reason about "why did the process die / why is it serving garbage."

**Failure mode + blast radius:** the compounding one — a partial dependency failure that *should* be a
clean crash-and-restart instead becomes either (a) a green-then-die crash-loop that keeps flapping the
whole fleet, or (b) a swallow-corrupted process serving 500s while `/livez` says healthy. Each is
individually survivable; wired together with no coordinator, the operator cannot predict which they'll
get.

**Kill-switch inventory is itself incoherent: ≥9 mechanisms, 4 actuation semantics, no inventory doc.**
Env-hot-kill (`VOICE_KILL`, no-store endpoint); schema-flags needing secret-set+restart (`OTP`,
`ACCESS_GATE`, `MEDIA_RICH`, …); raw-env flags outside the contract (`PAYMENTS_*`); a per-tenant DB
column (`busy_mode`, `orders.ts:122,538` — owner-controlled, not ops); the global rate limiter; the
`METRICS_TOKEN` dark gate; the dev-only loop-harness breaker. Incident response requires knowing which
of four different actuation mechanisms each switch uses, and no `docs/ops` kill-switch inventory exists.

**Incrementally fixable, or re-pour?** **Incrementally fixable — the mechanisms are individually sound;
only the composition is missing.** A *supervised startup sequence* with an explicit boot-phase contract
(all fail-closed assertions run **before** listen; nothing fatal after traffic; the RLS check probes one
pooled connection *pre-listen* instead of throwing in an event handler; the swallow-handlers scoped to
genuinely-recoverable request-level rejections). Small coordinating layer, not a guard rewrite.

**Highest-leverage intervention (this lane):** **One `bootstrap/boot.ts` sequencer with declared phases
(validate → connect → verify-schema/schedules/RLS → listen → reconcile), each check tagged
`abort | degrade | alert`, enforced by a single lint rule: nothing calls `process.exit()` after
`listen()`.** That one rule simultaneously converts both FATAL-after-listen asserts to a failing
`/readyz`+alert, rescues the RLS guard from the swallow handler, and gives every future guard a declared
slot instead of a new ad-hoc failure policy. Fold the 19 uncontracted keys into the Zod schema and
memoize `loadEnv` so the config contract is total and single-sourced.

---

## The 3 deepest structural risks, ranked

1. **The tenant floor is three non-composing partial floors, and the only shared seam satisfies just
   one of them (B1).** This is the deepest because *every* money/PII/authz red-line rests on it, and it
   is the one foundation that is not merely under-adopted but **internally inconsistent** — the
   NOBYPASSRLS flip that is supposed to add the structural floor **cannot be performed** until the two
   GUC namespaces are unified. Until then the app-layer is the only floor and "one forgotten predicate =
   silent cross-tenant leak" is the permanent standing risk.

2. **No structural undo at any layer, and the boot-guard obstructs the only real rollback (B5).** The
   system is un-operable-under-failure by construction: downs unexecutable, backups inoperable, no
   soft-delete, no expand/contract enforcement, no staging gate, and a documented rollback that corrupts
   migration bookkeeping. One destructive migration = full outage with no working undo.

3. **The realtime bus has zero durability contract and its real transport has zero test coverage
   (B2).** Three state-bearing subscribers ride a lossy, error-swallowing, gap-recovery-less NOTIFY;
   the "aliased to pass the test" origin means the tests never exercised the shipping transport. Silent
   loss of paid-order visibility, dwell resolution, and customer pushes — all green.

*(B3 contract-drift and B6 boot-incoherence are severe but rank below because both have the enforcement
seam or the coordination point already largely present — they are "wire it" not "the base is cracked.")*

---

## Honest verdict: sound with fixable gaps, or re-pour?

**Mostly sound with fixable gaps — plus TWO places that need targeted re-pouring, not patching.**

- **Re-pour (base-level, cannot be incrementally patched in place):**
  1. **The tenant data-access seam (B1).** `withTenant`'s single-GUC design is wrong at the root; the
     entry point must be replaced with a principal-typed helper + NOBYPASSRLS-by-construction role
     before the RLS floor can bear any load. The *policies* are salvageable; the *seam* is not.
  2. **The reversal spine (B5).** There is genuinely *no undo* today — this is not a gap in an existing
     spine, it is the absence of one. It must be *installed* (expand/contract gate + pinned-image
     rollback + working restore), which is cheap in code but structural in effect.

- **Patch (incremental, the primitive exists and is correct — finish adopting/wiring it):**
  the realtime durability (B2 — outbox pattern already in-repo twice), the contract seam (B3 —
  `fastify-type-provider-zod` already installed), the god-hub decomposition (B4 — the acquisition
  module already demonstrates the target shape), and the boot-phase coordinator (B6 — a small
  sequencing layer over guards that already exist).

The prior audits' "sound skeleton, unverified flesh" holds for ~four of six foundations. But it
**understates two**: the tenant floor and the reversal spine are not unverified flesh — they are
missing/mis-poured *bone*. Those two are the ones to treat as base-level work.

## The single highest-leverage structural fix

**Replace the tenant data-access entry point: one principal-typed `withTenantTx(pool, principal, fn)`
that sets the correct GUC(s) inside one audited transaction, backed by a `NOBYPASSRLS`-by-default
operational role (break-glass BYPASSRLS reserved for migrations).** This is highest-leverage because it
(a) unifies the two GUC namespaces that today make the RLS floor un-flippable, (b) turns "forgot the
tenant predicate" from a silent cross-tenant leak into a DB-enforced error by construction — collapsing
the largest class of live CRITICALs (LC2/LC5 and every latent IDOR) at the root, and (c) is the seam
through which the repository refactor (arch F2) and the correct transaction discipline (synthesis R-B)
both land. It converts the deepest foundation from "developer discipline across 777 queries" into a
structural floor — which is the one thing nothing above it can currently rely on.

---
*Recon-4 basement attack, 2026-07-03. Read-only; no source modified. Builds on and does not re-report
audit-architecture, AUDIT-SYNTHESIS, and recon3-privacy-ops (same date).*
