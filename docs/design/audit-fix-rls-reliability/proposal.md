# Proposal — GUC/tx discipline, latent-RLS completion, GDPR-erasure liveness, pg-boss reliability

- **Status:** RESOLVED-DRAFT v2 — Council RESOLVE round applied (breaker F1–F12 dispositioned,
  counsel ES-1/ES-2 revised in; see `resolution.md` for the per-finding record). Awaiting
  conductor re-attack.
- **Date:** 2026-07-03 (v1 architect draft; v2 same day, post-breaker/counsel)
- **Author:** system-architect (Council STEP 1)
- **Sources:** `docs/design-review/audit-reliability-2026-07-03.md` (C4, H1–H5, thread 1–2),
  `docs/design-review/audit-security-2026-07-03.md` §E, `docs/design-review/AUDIT-SYNTHESIS-2026-07-03.md`
  (LC4, R-B, R-E, A3, B-3/B-15/B-18), all claims re-verified against source at HEAD of
  `feat/phase0-safety-hardening`.
- **Red-lines touched:** RLS · PII/legal (GDPR) · `packages/db/migrations/**` — every migration and the
  helper conversion on money/courier paths are council + operator gated.
- **Relationship to B3:** this proposal is a **prerequisite** of ADR-b3-deep-auth-hardening's Option-B
  ramp. B3 decision #1 prepends `SET LOCAL ROLE dowiz_app_rls` "inside the existing BEGIN…COMMIT" of
  `withTenant` — but ~half the GUC sites have **no** BEGIN…COMMIT (they are autocommit no-ops today,
  §1 below), and §E policy gaps mean the flip would not isolate the tables that matter even if the
  GUCs worked. Fixing both is what makes the deferred NOBYPASSRLS flip real. (Unblocks B3.)

> Design-time only. No production code in this document. Migration sketches are illustrative shapes,
> not final SQL; final SQL is written in the implementation lane after breaker+counsel.

---

## 0. Verified problem statement (all re-checked against source 2026-07-03)

### 0.1 GUC/tx discipline (R-B / reliability thread 2)

The one correct canonical helper already exists and is exactly right:

- `packages/platform/src/auth/tenant.ts:3-21` — `withTenant(pool, userId, fn)`:
  `connect → BEGIN → set_config('app.user_id',$1,true) → fn → COMMIT/ROLLBACK → release`.

Around it, the full verified inventory (`site-inventory.md`) is **49 sites: 32 canonical-OK,
11 autocommit NO-OPs, 2 session leaks, 4 dev-only** — i.e. the audit's two flagship sites
undercount the class by ~5×. Beyond H4-rls/H5-rls, the NO-OPs include the courier **settlements
money path** (`routes/courier/settlements.ts:25,59,75` — worst shape: `db.query` is *pool*.query,
so the GUC statement and the payout SELECT run on different connections even today), all three
telegram-webhook order-action handlers (`:281,411,631` — precede `updateOrderStatus`), customer
push subscribe/unsubscribe (`customer/push.ts:35,72`), the owner live-map (`owner/couriers.ts:148`,
whose siblings `:26/:207` are correct — same file, coin-flip outcome), and `courier/assignments.ts:111`
(the only one of 9 handlers in its file missing BEGIN). Three violation shapes:

| Shape | Verified example | Behavior today (BYPASSRLS) | Behavior at NOBYPASSRLS flip |
|---|---|---|---|
| **A. `set_config(...,true)` in autocommit** (no BEGIN on the same client) | `apps/api/src/notifications/workers/index.ts:122` — sets `app.user_id` then reads `customer_devices` at `:124-130` | GUC dies with its own implicit txn; query runs context-free but BYPASSRLS returns rows anyway → works by accident | `customer_devices` is FORCE-RLS `customer_owns` (mig `1790000000077`); context-free read → 0 rows → `if (devicesRes.rows.length === 0) return;` at `:132` → **every CONFIRMED / IN_DELIVERY / DELIVERED / CANCELLED customer push silently stops. No error, no log.** |
| **B. `set_config(...,false)` session-scoped on a pooled client** | `apps/api/src/routes/owner/onboarding.ts:75` and `apps/api/src/routes/spa-proxy.ts:771` (comment at `:766` even says "persists across statements" — that persistence is the bug) | `app.user_id` stays on the physical backend after `client.release()` (`onboarding.ts:139`, `spa-proxy.ts:820`); node-pg does not reset session GUCs on release | Any later query on that pooled connection that forgets to set context **inherits the prior owner's id** → latent cross-tenant read/write. Also simply unreliable through Supavisor transaction mode (no session affinity). |
| **C. Correct-by-hand copies** | `lib/courier-room-authz.ts:46-47` (header `:11` documents the autocommit hazard), `workers/courier-events.ts:29-32`, `lib/notificationPrefsService.ts:35-37` | Correct, but the pattern is re-implemented per file — each new site is a coin-flip between A/B/C | Fine, but no single place for B3 to prepend `SET LOCAL ROLE` |

### 0.2 Latent RLS gaps (security §E — the flip does not isolate what it must)

Verified in `packages/db/migrations/` (§2.1 fact table below):
- `orders`/`order_items` anon policies: `USING (app_current_user() IS NULL)` unscoped
  (`1780338981783_anonymous_orders.ts`) — permissive OR nullifies `tenant_isolation` on the public pool.
- `customers` anon UPDATE unscoped and **no WITH CHECK** (`1780338981782_customer-anonymous-update.ts`).
- `couriers` (`1780421029538`) and `courier_sessions` (`1780421032856`) — **no RLS at all**; these hold
  `password_hash`, `token_hash`, encrypted PII. No flip helps a table with RLS off.
- `locations` `public_select USING(true)` (`1780338909301`) exposes phone/custom_domain/coords cross-tenant.
- `backup_metadata`/`backup_audit_log` ENABLEd, never FORCEd (`…048`, `…049`); `…050` grants
  `FOR SELECT TO authenticated USING(true)`.
- Anonymous INSERT `WITH CHECK (app_current_user() IS NULL)` without `location_id` scoping
  (`1780315000000_customer-rls.ts`).

### 0.3 LC4 — GDPR erasure permanently stranded (`apps/api/src/workers/anonymizer-gdpr.ts`)

Verified line-by-line:
- `:29` — scan is `WHERE status = 'pending'` only.
- `:39` — row flips to `in_progress` before processing.
- `:84-98` — catch bumps `metadata.retryCount` (row stays `in_progress`) and
  `boss.send(ANONYMIZER_GDPR, { requestId, retryCount }, { startAfter })`.
- `:22` + `:17` — `run(job)` **never reads `job.data`**; the retry job re-runs the same
  `status='pending'` scan, which by construction cannot see the `in_progress` row.
- `:90` — `retryCount < 3` is therefore evaluated at most once per row (frozen at 1);
  the `Max retries exceeded → failed` terminal at `:100-105` is unreachable.
- The handler returns normally → pg-boss acks the retry job as **success**.

Net: one transient error → the legally mandated erasure is neither `completed` nor `failed`, forever,
with a green queue. **PII/legal red-line.** (Enqueue path: `routes/owner/gdpr.ts:99` sends
`{requestId}` — also ignored by the worker.)

### 0.4 pg-boss reliability (H1–H3, thread 1)

- v10 runtime under v12 types (§3.1 fact table): `work()`-side `singletonKey` is not a v10 dedup
  mechanism, and send-side `singletonKey` dedups only on queues with `policy:'short'` — every queue is
  created default `standard` → the dedup layers the sweeps/notifications were designed around are
  **no-ops** (e.g. `anonymizer-gdpr.ts:17` passes `singletonKey` to `work()` — decorative).
- No `retryLimit/retryDelay/retryBackoff/deadLetter` anywhere except one `retryLimit:3`
  (`lib/signals/velocity-increment.ts:54`): v10 defaults = 2 retries at 0ms, then permanent `failed`
  with only recon-O3 (>10 failed/24h) as detector.
- `bootstrap/workers.ts` is one sequential awaited chain; most `createQueue`/`schedule`/`work`
  registrations are not isolated → one boot throw amputates every later worker **including the
  detectors** (heartbeats, liveness-checker, reconciliation). Recurrence of the 2026-06-21 shape.

---

## 1. Design — `withTenantTx`: one transaction-scoped context helper (converts A, B, C)

### 1.1 Requirements

1. **Single choke point** for tenant/principal context: BEGIN + `set_config(k,v,true)` + COMMIT,
   never session-scoped, never autocommit.
2. Covers **both GUC families** — `app.user_id` (owner/customer lane) and `app.current_tenant`
   (courier/webhook/worker lane) — and the rare combined case (`dev/mock-auth.ts` sets both).
3. **B3 hook point:** the helper is the one place that, when `RLS_ENFORCE_<LANE>` is on, prepends
   `SET LOCAL ROLE dowiz_app_rls` (B3 ADR decision #1). Converting sites to the helper *is* the B3
   ramp prerequisite.
4. Fail-safe defaults: ROLLBACK on throw; `client.release()` always; **no client escapes the callback**
   (the callback receives the client, the helper owns its lifecycle).
5. Supports the two legitimate non-standard shapes found in the wild without inviting regression:
   - multi-transaction flows on one client (`onboarding.ts:102-121` seeds the menu in its own txn
     after `bootstrap_owner`) → nested/sequential helper calls, not one long-lived session GUC;
   - SECURITY DEFINER bootstrap before context exists (`bootstrap_owner`) → run the DEFINER call
     first, then enter the helper for the member-context writes.

### 1.2 Option 1 — one generic helper, context object (RECOMMENDED)

`packages/platform/src/auth/tenant.ts` (extend the existing file; `withTenant` becomes a thin alias):

```ts
// SHAPE ONLY — not production code
type TenantCtx =
  | { userId: string; tenantId?: string }      // owner/customer lane → app.user_id (+ optional app.current_tenant)
  | { tenantId: string }                        // courier/webhook/worker lane → app.current_tenant
  | { anonymous: true };                        // explicit anon: BEGIN with NO guc — makes "no context" a decision, not an accident

async function withTenantTx<T>(pool: Pool, ctx: TenantCtx, fn: (c: PoolClient) => Promise<T>): Promise<T>
// body: connect → BEGIN → [flagged: SET LOCAL ROLE dowiz_app_rls] → set_config(...,true) per ctx key
//       → fn(client) → COMMIT / ROLLBACK → release
// withTenant(pool, userId, fn) ≡ withTenantTx(pool, { userId }, fn)  (kept, deprecated-in-lint)
```

- **Pros:** one implementation, one B3 hook, one lint target ("only tenant.ts may call `set_config`"),
  the `anonymous` variant gives the public-pool paths an auditable marker, `withTenant` call sites
  (22 files) need zero churn.
- **Cons:** a union-typed ctx is marginally less discoverable than named helpers; both lanes share one
  function so a wrong-key mistake is a runtime (not type-system) matter unless the union is strict
  (it is: exclusive union, no optional-both).

### 1.3 Option 2 — named per-principal helpers over a private core

```ts
// SHAPE ONLY
withOwnerTx(pool, userId, fn)        // app.user_id
withCourierTx(pool, locationId, fn)  // app.current_tenant
withCustomerTx(pool, customerId, fn) // app.user_id (customer id in the same GUC — matches current policies)
withWebhookTx(pool, locationId, fn)  // app.current_tenant (payments/telegram)
// all delegate to a non-exported withGucTx(pool, gucs: Record<string,string>, fn)
```

- **Pros:** call sites read as intent (`withCourierTx` on a courier route is self-reviewing); grep-able
  adoption metrics per lane; per-lane B3 flags map 1:1 to helper names.
- **Cons:** 4–5 exported names to keep aligned; the combined owner+tenant case needs either a fifth
  name or reaching into the core; slightly larger surface for the lint rule ("only tenant.ts may
  contain the string `set_config`" still works).

### 1.4 Option 3 (rejected) — AsyncLocalStorage request-context auto-injection

Middleware stores the principal in ALS; a patched `pool.query` opens a txn and injects GUCs
transparently. Rejected: magic hides the transaction boundary (the exact thing that caused A/B),
breaks for workers/webhooks with no request context, and violates "boring, proven, reversible"
(B3 ADR context). Noted for completeness only.

**Recommendation (v2): Option 1 core + one named veneer on the money lane.** The B3 ramp needs
exactly one choke point, and the exclusive union keeps the two GUC families type-separated without
multiplying exports. Per counsel advice #4 (accepted): the courier **money** paths
(`courier/settlements.ts` and payout-adjacent sites) use an exported named wrapper
`withCourierTx(pool, locationId, fn)` (a one-line delegate to `withTenantTx`), so a wrong-GUC-family
mistake on a payout read is a compile error, not a runtime hope. Everything else uses the generic form.

**ALS hybrid (counsel steel-man, harvested):** production keeps the explicit helper only — but the
§1.5.4 dev/test canary generalizes into an **ALS-backed tripwire**: in dev/test builds, a query
touching a FORCE-table with no helper context in AsyncLocalStorage scope throws. This captures the
safe-by-default value of the rejected Option 3 (catching "forgot to wrap at all" — the failure class
the `set_config`-string lint cannot see) with zero production magic. Ships in Lane 0.

### 1.5 Conversion inventory and rules (v2 — surface redefined per breaker F2)

**The flip-safety surface is every reader/writer of a FORCE (or to-be-FORCE) table, NOT the set of
`set_config` call sites.** `site-inventory.md` (49 grep-sites) remains the *GUC-misuse* inventory;
the *conversion-complete* gate is now defined against a second, larger inventory:

- **`force-access-inventory` (new artifact, scripted):** `scripts/rls-force-access-scan.mjs` scans
  SQL literals across `apps/*/src` + `packages/*/src` for FORCE-table names and classifies every
  site: `helper-wrapped` / `definer-resolver` / `allowlisted (reason)` / `UNCONVERTED`. It runs in
  CI (**P10**): UNCONVERTED must be 0 before MIG-2/3 apply to prod and before any flip step — a new
  bare reader turns CI red. Known members the grep missed (breaker F2): **checkout**
  (`routes/orders.ts` — BEGIN at :104/:111, zero `set_config`; INSERTs into
  customers/orders/order_items/velocity_events/idempotency_keys/customer_track_grants),
  **track exchange** (`routes/customer/track.ts:43-53`), **session validation**
  (`plugins/auth.ts:74-83`), **courier login** (`routes/courier/auth.ts:249` — resolved by MIG-1's
  firebreak, §2.2.3, not by conversion), telegram tenant-discovery reads (§1.7).
- The falsifiable runtime complement is **P9** (flip-rehearsal suite) + the named deploy gates
  (§6): the "conversion complete" claim is proven by tests that fail while any covered path is
  unconverted, never by inventory bookkeeping alone.

Conversion rules (v2 — **semantics-preserving**, per breaker F5/F6/F9):

1. **Shape-B sites (session leak — fix FIRST, smallest and most dangerous):**
   `onboarding.ts:75`, `spa-proxy.ts:771` → `withTenantTx(db, { userId }, …)`. For onboarding's
   two-phase flow: DEFINER `bootstrap_owner` call runs before/outside; the member-context writes and
   the best-effort menu-seed become two sequential `withTenantTx` calls (matching the existing
   explicit-BEGIN seed block at `onboarding.ts:102-121`). Semantics note: this **preserves** today's
   behavior (a committed `bootstrap_owner` survives a seed failure, same as now) — it is documented
   as best-effort, not advertised as atomic.
2. **Classify each site by write-semantics before converting — never "wrap the client block":**
   - **reads** → plain wrap (e.g. settlements; see breaker balancing note — clean);
   - **single logical write** → wrap; atomicity is correct and intended;
   - **multi-write best-effort flows** (spa-proxy onboarding `:769-821`: location UPDATE + product
     seed loop + theme INSERT) → **phase transactions + per-row SAVEPOINT** inside the seed loop, so
     one bad product row still yields "storefront minus one item," never a rolled-back provisioning
     500. Today's partial-success semantics are a feature to preserve, not an accident to fix.
   - **flows containing external network calls** (telegram handlers) → txn-boundary redesign per
     §1.7; naive wrapping is forbidden (it would hold order-row locks + a pooled connection across
     `api.telegram.org` round trips — pool starvation today, flip-independent).
3. **Two-context handlers:** `notifications/workers/index.ts` needs BOTH GUC families — the order
   read runs under `withTenantTx({ tenantId: job.data.locationId })` (the payload already carries
   `locationId`, `:100`), then the devices read under `withTenantTx({ userId: order.customer_id })`.
   The v1 claim that the order read "needs no GUC" was wrong post-MIG-2 (breaker F9).
4. **Shape-C sites (hand-rolled correct):** convert opportunistically in the same lane-PR; they are
   correct today, so they are the LAST wave, protected by the lint gate from regressing.
5. **Interim hazard netting** (before all sites are converted): the ALS-backed dev/test tripwire
   (§1.4-hybrid) — any FORCE-table query with no helper context in scope throws in dev/test builds;
   subsumes the v1 checkout NULL-GUC assert and also catches shape-B leaks.

### 1.7 Telegram webhook txn-boundary design (replaces "wrap" for the 3 handlers — breaker F5)

**Invariant: no DB transaction is ever open across an external HTTP call.**

1. **Tenant from the chat, not from an unscoped order read.** The chat is the authenticated
   principal; its location mapping already exists in data (`owner_notification_targets.address`,
   `telegram_connect_tokens`). A small SECURITY DEFINER resolver `resolve_telegram_chat(address)`
   (search_path-pinned per ledger #33) returns the chat's active `{location_id, user_id, locale,
   target_id}` set. This replaces the pre-context discovery reads
   (`telegram-webhook.ts:157` order→location, `:214` target-by-address) that would 0-row post-MIG-2
   — and is a better authz shape (the order id in callback data is *validated against* the chat's
   resolved locations, mirroring the existing `:169` check). Convention note: DEFINER
   ingress-resolvers (slug→location §2.2.4a, chat→location here, track-token→order §2.3-MIG-4 lane)
   are one deliberate pattern, each tiny and auditable.
2. **Handler decomposition:** (a) `answerCallbackQuery` ack — before any txn (no DB state);
   (b) `withTenantTx({ tenantId })`: authz reads (membership `:241`), the guarded
   `updateOrderStatus` UPDATE, fetch fresh display state — **COMMIT**; (c) all Telegram sends
   (`sendMessage`/`editMessageText`) run **post-commit**, best-effort: on a crash between COMMIT and
   send, order state is already correct and Telegram's callback retry lands in the 409 path, which
   reports current state. State is the source of truth; messages are notifications.
3. **The 409 catch-path re-read** (`:289-291`, `:419-422`) runs in its own short
   `withTenantTx({ tenantId })` — never on a stale context-free client.
4. **Proof P11:** with the Telegram API mocked, assert COMMIT/release strictly precedes the first
   external call (spy ordering), and that the 409 path answers from its own txn.

### 1.6 Guardrails (deterministic; the ratchet)

- **G1 · lint:** new `tools/eslint-plugin-local` rule `no-bare-set-config`: the literal `set_config`
  is forbidden outside `packages/platform/src/auth/tenant.ts` (+ migrations). Red→green proof: rule
  added → CI red on the ~25 current files → conversions land → green. Never weakened.
- **G2 · runtime dev-assert:** checkout-time NULL-GUC assert (§1.5.4) as a test-env tripwire.
- **G3 · ledger:** one row per shape (A/B) in `docs/regressions/REGRESSION-LEDGER.md` with the
  red→green test names from §5.

---

## 2. Design — latent-RLS completion (§E): make the flip actually isolate

### 2.1 Verified current state (fact table)

See `rls-state.md` next to this proposal for the migration-by-migration inventory (FORCE list,
ENABLE-only list, policy quotes). Summary of what must change:

| Table | Today | Gap class |
|---|---|---|
| `orders`, `order_items` | anon policy `USING (app_current_user() IS NULL)` — unscoped, OR-combined | E1: anon nullifies tenant_isolation on the public pool |
| `customers` | anon UPDATE unscoped, **no WITH CHECK** | E1 + write-escalation |
| `couriers` | **no RLS** (credential hashes, encrypted PII) | E2: RLS off entirely |
| `courier_sessions` | **no RLS** (`token_hash`, `family_id`) | E2 |
| `locations` | `public_select USING(true)` | E3: over-broad public read |
| anon INSERT (orders/customers) | `WITH CHECK (… IS NULL)` — any tenant | E4: unscoped anon insert |
| `backup_metadata`, `backup_audit_log` | ENABLE, not FORCE; `TO authenticated USING(true)` | E5: owner-bypass + broad read |

### 2.2 Design principles

1. **Forward-only migrations, FORCE on every touched table** (`ENABLE` + `FORCE ROW LEVEL SECURITY`;
   `down()` = no-op comment per forward-only policy). FORCE is what makes the *table-owner* role also
   subject — the exact hole `backup_*` demonstrates.
2. **Anonymous access must be row-scoped, not role-scoped.** `app_current_user() IS NULL` is a role
   test pretending to be a row predicate. The anon lane gets its own **scoping GUC** set by the same
   helper (`withTenantTx(pool, { anonymous:true, tenantId })` variant carrying `app.current_tenant` =
   storefront's resolved `location_id`): anon policies become
   `USING (app_current_user() IS NULL AND location_id = NULLIF(current_setting('app.current_tenant',
   true), '')::uuid)` — the missing-ok inline form is the established mig-077 convention (no
   `app_current_tenant()` function exists; creating one is optional polish, matching
   `app_current_user()`, decided at implementation). The storefront/public pool paths already
   resolve the slug → location before querying; the helper conversion (§1) is what makes this GUC
   available on the public lane. **Ordering consequence: E1/E4 policy migrations land only after
   the public-lane helper conversion** — otherwise anon checkout breaks (fail-closed, availability
   hit, not a leak). NOTE: `order_items`' anon INSERT `WITH CHECK (EXISTS (SELECT 1 FROM orders …))`
   (`1780315000000:8-11`) inherits its scope from the orders SELECT policy — scoping orders scopes
   it transitively; verify in P1 rather than duplicating the predicate.
3. **Credential tables get the FIREBREAK, not tenant policies (v2 — breaker F1 fix).**
   The v1 design was wrong twice: `couriers` has **no `location_id` column** (the mapping lives in
   the `courier_locations` join table — `1780421029538:5-28` — so the v1 policy cannot even apply),
   and login (`courier/auth.ts:249`) + per-request session validation (`plugins/auth.ts:74-83`) are
   **structurally pre-context** reads — any tenant-keyed policy is a total courier-auth lockout at
   the flip. `couriers`/`courier_sessions` are the exact class of `users`/`auth_refresh_tokens` and
   get the exact same treatment — the **real** firebreak convention, which is (fact correction, see
   `resolution.md §0.1`): **ENABLE + FORCE + role-restricted permissive policy**, mirroring mig
   `1780421100065` STEP A2 + mig 077 RC2:
   - `couriers`: `ENABLE+FORCE` + `ops_all FOR ALL TO dowiz_app USING (true) WITH CHECK (true)`.
   - `courier_sessions`: same shape.
   - `courier_locations`: keep its tenant policy, **add** `courier_auth_read FOR SELECT TO dowiz_app
     USING (true)` so the session-validation EXISTS (`plugins/auth.ts:79-81`) keeps working
     pre-context post-flip (mapping table — no secrets/PII; write isolation unchanged).
   What this buys **immediately** (not flip-gated): deny-by-default on the credential tables for
   every role except `dowiz_app` — including the already-NOBYPASSRLS operational read pool and any
   future role. What it deliberately does NOT do: tenant-scope reads made by the app role itself —
   that is app-layer + B3-lane work, exactly as for `users`.
   - **Precondition OPS-READ-CHECK (before MIG-1 applies anywhere):** verify zero legitimate
     operational-pool reads of these three tables (scripted grep + staging soak); if any exist,
     convert them or add a scoped policy for that role — decided at implementation, gated.
   - **MIG-1b (DEFERRED — flagged, owner: B3 courier-lane):** tenant-scoped visibility policies
     `TO dowiz_app_rls` (`couriers: id IN (SELECT courier_id FROM courier_locations WHERE
     location_id = <tenant GUC> OR …member ids…)`; `courier_sessions: active_location_id =
     <tenant GUC>`), required before the B3 **courier-lane flag** enables, not for the firebreak.
   - **Proof P1b (new):** a legitimate courier can still authenticate and validate a session under
     a NOBYPASSRLS'd `dowiz_app` probe — the key, not just the lock. Fails if MIG-1 is ever
     rewritten tenant-scoped.
4. **`locations` public read becomes a column-safe view or a scoped policy.** Two sub-options for
   breaker to attack:
   - (a) keep `public_select` but scope it `USING (id = app_current_tenant())` — storefront resolves
     slug→id via a SECURITY DEFINER `resolve_location_slug(text)` function (tiny, auditable), then
     reads within context;
   - (b) narrow the exposure instead of the rows: move public reads to a `locations_public` view
     (name/slug/hours/theme keys only), revoke direct table SELECT from the app role for the anon lane.
   Recommendation: (a) — it reuses the same GUC mechanism as everything else; (b) adds a second
   authorization vocabulary.
5. **`backup_metadata`/`backup_audit_log`:** add FORCE; replace `TO authenticated USING(true)` with a
   platform-plane predicate (these are ops tables — the owner/courier lanes have no business reading
   them): `USING (app_is_platform_admin())` or role-restricted `TO dowiz_ops`. Aligns with the B3
   firebreak convention (`users`/`auth_refresh_tokens` role-restricted).
6. **Permissive-policy hygiene rule (v2):** on any table with a tenant key, every permissive policy
   must **either** include a tenant-key predicate **or** be role-restricted (`TO <specific role>`)
   and listed on the firebreak allowlist (`users`, `auth_refresh_tokens`, `ops_worker_heartbeat`,
   `couriers`, `courier_sessions`, `courier_locations` SELECT-only). Documented invariant +
   `verify:rls` check (P8). This is the class rule that E1 violated; the allowlist is the sanctioned
   exception class, closed by default.

### 2.3 Migration plan (forward-only, each operator-gated)

- **MIG-1** `rls-couriers-firebreak` (v2): ENABLE+FORCE `couriers`, `courier_sessions` +
  role-restricted `ops_all TO dowiz_app` policies; `courier_auth_read FOR SELECT TO dowiz_app` on
  `courier_locations` (§2.2.3). *Inert for the BYPASSRLS main pool; takes effect immediately for
  non-BYPASSRLS roles (operational read pool) — hence gated on OPS-READ-CHECK. Proofs: P1
  (deny-by-default for other roles) + P1b (courier auth still works under a NOBYPASSRLS probe).*
- **MIG-2** `rls-anon-scoping`: re-create anon policies on `orders`/`order_items`/`customers`
  (+ `WITH CHECK` on the customer UPDATE; + `location_id` scoping on anon INSERTs). **Blocked by
  two deterministic gates, not prose (breaker F2 / counsel #2): P10 = 0 UNCONVERTED in the
  force-access inventory for the anon lane, and GATE-ANON-E2E — MIG-2/3 may not apply to prod
  unless the staging anon-checkout + track E2E is green on staging with MIG-2/3 already applied
  there (and staging enforcement rehearsal on).**
- **MIG-3** `rls-locations-scope`: `public_select` scoped per §2.2.4(a) + `resolve_location_slug` DEFINER.
- **MIG-4** `rls-force-completion`: FORCE `backup_*` + platform-plane policy (drop the
  `TO authenticated USING(true)` reads, `…048:23`/`…049:17`); FORCE the token-bearing
  ENABLE-only tables (`customer_track_grants`, `provision_grants`, `claim_invites`,
  `courier_cash_ledger`, `delivery_trace` — full list in `rls-state.md §4`) and replace the
  `FOR ALL USING(true)` self-mint policies on `provision_grants`/`claim_invites`/`access_requests`/
  `acquisition_sources` with plane-scoped predicates (§E-MED self-mint finding). **Deliberate
  exceptions kept:** `users`/`auth_refresh_tokens`/`ops_worker_heartbeat` keep their firebreak —
  which is (fact correction, `resolution.md §0.1`) ENABLE+**FORCE** + role-restricted
  `ops_all TO dowiz_app` (`1780421100065` up() STEP A2; `1790000000077:24-30` RC2) — the v1 text
  "ENABLE-no-FORCE" misread the migration's `down()`. Not a gap; MIG-1 now joins this class.
- Each migration: staging first (staging DB via `flyctl proxy` recipe), probe suite green, soak, then
  prod on operator approval — per Ship Discipline and the B3 drift rule (policies written against the
  reconciled prod schema, never an assumed column).
- The flip itself stays where it is staged: `docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts`
  (approval-pending, out-of-tree), Phase 3/4 of `docs/design/pg-privilege-hardening/remediation-plan.md`.
  This proposal changes its **preconditions**, not its mechanics.

---

## 3. Design — LC4: GDPR erasure liveness (terminal-resolution invariant)

**Invariant:** every `gdpr_erasure_requests` row reaches `completed` or `failed`; nothing is stranded
`in_progress` beyond one processing window.

### 3.1 Chosen design (v2) — DEFINER claim + claim-token CAS + targeted retry + owned dead-letter

Ships as **Lane A** (ES-1: standalone fast-track, no flip dependency; carries pg-boss boot
isolation with it per counsel #5 — the O-GDPR detector must be un-amputatable first). The v1
"no schema change" claim is **dropped** (breaker F4) and restated precisely: **one additive
migration (LC4-MIG: policy arms + one DEFINER function), zero new columns.**

1. **Atomic claim via SECURITY DEFINER `gdpr_claim_due(batch int)`** (search_path-pinned per ledger
   #33): claims, in one statement (`FOR UPDATE SKIP LOCKED` + `UPDATE … RETURNING`), rows that are
   `pending` **or** stale-`in_progress` (see 3), stamping `metadata.claimed_at = now()` and a fresh
   `metadata.claim_token = uuid` — and returns them. Why DEFINER (breaker F3): the cross-tenant
   scan is a *system* read with no tenant and no membership — under NOBYPASSRLS neither GUC family
   can see all tenants' rows; a tiny DEFINER function is the established pattern
   (`app_member_location_ids()` is itself DEFINER; §2.2.4a's slug resolver; §1.7's chat resolver).
2. **Per-row processing runs in `withTenantTx(pool, { tenantId: row.location_id }, …)`** — and
   **LC4-MIG extends** `gdpr_tenant_isolation` + `anonymization_audit_tenant_isolation` (both
   currently keyed ONLY on `app_member_location_ids()`, `1780421100060:49-51,:57-59` — the v1
   compat claim was false, breaker F3) with a missing-ok arm:
   `… OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid`.
   Inert under BYPASSRLS today; correct post-flip.
3. **Stale-reclaim without a phantom column (breaker F4):** reclaim selects
   `in_progress AND (metadata->>'claimed_at')::timestamptz < now() - interval '30 minutes'`
   (`claimed_at` is stamped at claim time — not `requested_at`, which never moves and would
   false-reclaim busy rows). The 30-min window is ≫ worst-case anonymize; the residual
   "slow row reclaimed while held" race is made harmless by (4).
4. **Claim-token CAS on every terminal write (kills F4-race AND F7 duplicates):** completion and
   failure UPDATEs are `WHERE id=$1 AND status='in_progress' AND metadata->>'claim_token'=$mine`.
   The audit-log INSERT and the `gdpr.erasure_completed` publish execute **only when the CAS
   returns rowCount=1** — so a double-trigger (periodic scan + targeted retry, or a reclaimed-slow
   row) yields exactly **one** audit row and **one** event per request, regardless of
   `result.skipped`. The CAS is the dedup; `anonymized_at` idempotency protects only the data.
5. **Retry path reads `job.data`.** The handler (fixed to the v10 **array** shape — breaker F11;
   this file's array bug rides Lane A because the `job.data` path is only correct with it) branches:
   `job.data?.requestId` present → process exactly that row `WHERE id=$1 AND status IN
   ('pending','in_progress')` via a targeted DEFINER claim; `retryCount` advances 1→2→3 and the
   `:100-105` terminal becomes reachable.
6. **On retryable failure, CAS-reset the row** to `status='pending'` (+ `retryCount` bump) — even a
   lost retry job is picked up by the next scheduled scan.
7. **Dead-letter + alert at cap — owned, not parked (counsel #3, near-ES):** at `retryCount >= 3` →
   `status='failed'` **plus** (a) pg-boss `deadLetter` → **`anonymizer.gdpr.dlq`** (`.dlq` is the
   canonical suffix — `deadLetterQueueName()`, `queue-provider.ts:52-54`; the v1 `.dead` naming was
   wrong, breaker F10); (b) reconciliation check **O-GDPR** — **level-triggered**: it re-fires a
   DRIFT on `ops.reconciliation_drift` + Sentry on EVERY run while any row is non-terminal >24h
   after `requested_at` or any `failed` row exists — a `failed` row cannot go quiet; (c) a bound
   **resolution owner: operator, SLA 72h** from `failed`, with a runbook entry (retry after
   root-cause, or manual erasure + `completed` with `metadata.resolution`); (d) the owner GDPR list
   (`routes/owner/gdpr.ts` — surface exists) renders `failed` as *requires action*, and terminal
   transitions notify `owner_notification_targets`. (`failed` label kept — ACCEPT-RISK, see
   `resolution.md §1`.)
8. **Art. 12 receipt (counsel open question — designed):** v1 is **controller-facing**: the owner is
   the GDPR controller (dowiz is processor); on terminal state the request record carries a
   machine-readable receipt (status, requested_at, completed_at, erasure scope, failure reason)
   via the existing GET endpoint + the owner notification; the owner relays it to the subject.
   Direct-to-subject delivery is **needs-human** — post-erasure the system may no longer hold the
   subject's contact channel, and automating messages to just-erased data is itself a PII decision.
9. **Anonymizer sink predicate:** the `AND location_id=$2` on the sink is security-F2's council
   lane; this proposal only *requires* it lands before or with the flip, and does not respecify it.

**Alternative considered (rejected):** scan-includes-`in_progress` only (no `job.data` path). Simpler
(one WHERE change) but: retryCount still advances only when a scan happens to run, backoff timing is
lost (the `startAfter` job becomes a no-op scan most of the time), and two machines can both reclaim
early. The `job.data` path preserves the intended per-row backoff (`:91`).

---

## 4. Design — pg-boss queue policy (H1–H3)

1. **Resolve the version lie first (v2 — honest scope, breaker F11):** runtime is 10.4.2
   (`packages/platform` `^10.1.5`), types are 12.18.3 (`apps/api` `^12.18.2`), bridged by
   `as unknown as PgBoss` casts and an in-code FLAG at `bootstrap/workers.ts:137-139` (facts:
   `pgboss-state.md §1`). Align on ONE major — **pin types to v10** (matches deployed reality; a
   v12 runtime upgrade is its own future lane with its own soak). The v1 "zero runtime risk"
   framing is **withdrawn**: removing `@ts-nocheck` under v10 types surfaces the real M5 bug — the
   direct `boss.work` sites (settlement-cron, dwell-escalation, lifecycle-handlers,
   anonymizer-gdpr; `pgboss-state.md §7`) are written v12-single-job but v10 delivers **arrays**,
   i.e. they are runtime-wrong TODAY. Fixing them is in-scope for this lane: route them through the
   provider's normalizing `queue.work()` wrapper (`queue-provider.ts:45-52`) or normalize per site,
   with tests. The anonymizer's fix rides **Lane A** (same file as LC4; its `job.data` retry path
   depends on it).
2. **Queue creation policy (per-queue) + the existing-queue trap (breaker F8):** pg-boss stores a
   queue's policy at create time and every prod queue already exists as `standard`
   (`server.ts:252` bare loop) — re-calling `createQueue` with options is NOT guaranteed to alter
   a stored policy. Therefore:
   - a boot-time **queue-policy reconciler**: read `pgboss.queue`'s stored policy per queue;
     on drift, converge via v10 `updateQueue` — **OPEN-V1**: a 30-min spike must verify 10.4.2
     `updateQueue` actually applies a policy change to a live queue BEFORE the reconciler is coded;
   - **designed fallback if OPEN-V1 fails:** keep those queues `standard` and use send-side
     `singletonSeconds` throttling — the one dedup mechanism *proven* to work on `standard` queues
     (`bootstrap/messaging.ts:43`); drop/recreate is break-glass only (queued-job loss);
   - queues whose design relies on `singletonKey` dedup (`courier.dispatch`, `order.timeout*`,
     notification fan-out) → `policy:'short'` (via the reconciler) or the fallback;
   - every queue gets explicit `retryLimit` (default 3), `retryDelay` (≥30s), `retryBackoff:true`,
     and `deadLetter` → **`<queue>.dlq`** (the helper's existing convention,
     `deadLetterQueueName()` `queue-provider.ts:52-54`; the v1 `.dead` naming was a mismatch that
     would have left the monitor watching nothing — breaker F10);
   - long-running handlers (backup) get `expireInSeconds` sized to worst case (reliability M8);
   - one shared `QUEUE_POLICY` table in `packages/shared-types/src/queue-names.ts` next to the
     names, so name and policy cannot drift apart; `createQueue` + the reconciler + the DLQ monitor
     all read it (single source — no glob-vs-suffix drift);
   - drop the decorative `work(..., { singletonKey })` options (e.g. `anonymizer-gdpr.ts:17`).
3. **Dead-letter consumption:** one generic DLQ monitor worker subscribed to the `*.dlq` queues
   **derived from the `QUEUE_POLICY` map** → recon DRIFT + Sentry (no auto-retry from a DLQ — human
   lever; for the GDPR queue the human lever is owned + SLA'd, §3.7). The unreachable
   `err.data`-based archive branch in `notifications/workers/index.ts:517-534` is deleted in favor
   of real `retryLimit`+`deadLetter`.
4. **Per-worker boot isolation (v2 — breaker F12):** verified — `bootstrap/workers.ts` has ~23
   sequential bare-awaited registrations and its ONLY try/catch wraps the FREE_TIER_WATCH handler
   body, not a registration (`pgboss-state.md §3`). Fix at the bootstrap: one
   `startIsolated(name, fn)` wrapper around EVERY registration. The v1 "detectors register FIRST"
   prescription is **dropped** — it fights reconciliation's real dependency on the constructed
   heartbeat set (`bootstrap/workers.ts:160-171`) and would produce false DRIFT at boot. Instead:
   registration order stays dependency-driven; the wrapper feeds a **boot registry**, and the
   liveness/reconciliation **watch-set is the actually-started set** from that registry (never a
   static list); a failed registration itself publishes a boot DRIFT + marks the worker degraded in
   `/health` (pairs with reliability H7). No amputation, no false positives. Ships in **Lane A**
   (counsel #5: LC4's O-GDPR detector runs on this infrastructure).

---

## 5. Proof plan (every fix has a test that fails when the code is wrong)

| # | Proof | Fails when |
|---|---|---|
| P1 | **RLS isolation test (extends `apps/api/tests/phase5/rls-adversarial.test.ts`):** in a probe txn `SET LOCAL ROLE dowiz_app_rls` + tenant-A GUC, assert 0 rows / 42501 on tenant-B `orders`, `order_items`, `customers`, `backup_metadata`; for the firebreak tables (`couriers`, `courier_sessions`) assert **deny-by-default for every role except `dowiz_app`** (firebreak semantics, not tenant-scoping); assert the **anon lane** (`app.user_id` NULL + `app.current_tenant`=A) reads only A and cannot INSERT into B. Runs against the provisioned service DB in CI (the audit's R-C lane un-skips this file — its current silent env-skip is exactly test-integrity C4). | any §E policy is missing/unscoped — this is the test that FAILS TODAY (red) and goes green per MIG-1..4 |
| P1b | **Courier auth survives the flip (the key, not the lock — breaker F1):** under a NOBYPASSRLS'd `dowiz_app` probe with MIG-1 applied: login SELECT by `email_hash` (`courier/auth.ts:249` shape) returns the seeded courier; the session-validation query incl. the `courier_locations` EXISTS (`plugins/auth.ts:74-83` shape) succeeds. | MIG-1 is ever rewritten tenant-scoped, or the firebreak policies are dropped/narrowed — the exact lockout F1 predicted |
| P2 | **GUC no-op regression (pinned to the end-state — breaker F9):** integration test drives `handleCustomerStatus` on the **flip-rehearsal DB** (ALL MIGs applied + enforcement on) with a seeded opted-in device; asserts the push adapter was invoked — which requires both contexts (order read under `{tenantId}`, devices read under `{userId}`). | `set_config` runs outside a txn (today's `:122` shape), OR the order read is left context-free (the F9 relocation) |
| P3 | **Session-leak regression:** checkout a client via the onboarding path, release it, then on the SAME pool assert `current_setting('app.user_id', true) IS NULL` on the next checkout (loop the pool to hit the same backend, `max:1` test pool). | any `set_config(...,false)` remains — red today at `onboarding.ts:75` / `spa-proxy.ts:771` |
| P4 | **Lint gate red→green:** `no-bare-set-config` lands red (~25 files), goes green as conversions land; rule test in `tools/eslint-plugin-local/__tests__`. | anyone reintroduces a bare `set_config` |
| P5 | **GDPR stranding + liveness regression (extended):** seed a request; fault-inject one throw → row returns to `pending` with `retryCount=1`; targeted retry → `completed`. Always-throw → `failed` at cap + a job in `anonymizer.gdpr.dlq` + O-GDPR DRIFT — and a **second** reconciliation run re-fires the DRIFT (level-trigger). **CAS concurrency:** fire the periodic scan and the targeted retry on the same row → exactly **1** audit row + **1** `erasure_completed` event (F7). **Reclaim both ways:** aged `claimed_at` → reclaimed; in-window active row → NOT reclaimed (F4). | the LC4 loop shape recurs; duplicates appear in the compliance audit log; a `failed` row goes quiet; a busy row is stolen |
| P6 | **Dedup proof (rewritten against prod's real object — breaker F8):** create the queue **bare/`standard` first** (prod mirror), run the policy reconciler, then send twice with the same `singletonKey` → assert 1 job. A control case documents that without the reconciler the same sequence yields 2. | the reconciler is a no-op on pre-existing queues (the exact false-green F8 predicted), or policy regresses |
| P7 | **Boot isolation proof (extended — breaker F12):** inject a throwing registration for worker N → workers N+1… still register; the failure publishes a boot DRIFT + degrades `/health`; the liveness watch-set equals the **actually-started** registry (no false DRIFT for the failed worker's absence beyond its own alert). | the sequential-amputation shape recurs, or a static watch-set produces false DRIFT |
| P8 | **verify:rls extension (amended):** policy-hygiene probe — every permissive policy on a tenant-keyed table must reference the tenant key **OR** be role-restricted (`TO <role>`) and on the firebreak allowlist (`users`, `auth_refresh_tokens`, `ops_worker_heartbeat`, `couriers`, `courier_sessions`, `courier_locations`-SELECT); plus the B3 `rolbypassrls` state probe. | a future migration adds an unscoped permissive policy, or a role-blanket policy off-allowlist |
| P9 | **Flip-rehearsal suite (the falsifiable conversion gate — breaker F2):** on a NOBYPASSRLS-enforced rehearsal DB, drive the four public/pre-context flows end-to-end: anon **checkout INSERT** (`routes/orders.ts` path), **track-token exchange** (`customer/track.ts`), **courier login + session validation**, **owner login**. | ANY reader/writer of a FORCE table on these flows is unconverted — fails today by construction; each conversion turns its slice green; green P9 is the precondition "public-lane conversion complete" actually means |
| P10 | **FORCE-access inventory CI gate:** `scripts/rls-force-access-scan.mjs` — every SQL-literal reader/writer of a FORCE/to-be-FORCE table is helper-wrapped, DEFINER-resolved, or allowlisted-with-reason; UNCONVERTED = 0 required before MIG-2/3-prod and any flip step. | a new bare FORCE-table access lands (the class `grep set_config` is structurally blind to) |
| P11 | **Telegram txn-boundary proof:** with the Telegram API mocked, assert COMMIT/release strictly precedes the first external HTTP call (spy ordering), and the 409 path answers from its own short txn. | a txn/lock is held across `api.telegram.org` (the F5 pool-starvation shape), or the catch-path re-read loses context |

Ledger: one `docs/regressions/REGRESSION-LEDGER.md` row per P1/P1b/P2/P3/P5 (red→green evidence pasted).

**Named deploy gates (deterministic, replace prose discipline — counsel #2):**
- **GATE-ANON-E2E** — MIG-2/MIG-3 may not apply to prod unless the staging anon-checkout + track
  E2E is green on staging **with MIG-2/3 already applied there** (+ staging enforcement rehearsal on).
- **GATE-FLIP-E2E** — the NOBYPASSRLS flip (staging Phase 3 / prod Phase 4) requires: full 3-role
  lifecycle E2E green on flipped staging + P9 green + P10 UNCONVERTED=0.
- **GATE-OSS-RLS** — the ADR-020 open-source flip requires MIG-1 + MIG-4 landed and P1 + P1b green
  (ES-2; additive to the existing secrets/EUTM gates).

---

## 6. Scope split + ordering vs the deferred B3 flip (v2 — three lanes, counsel #1 adopted)

**The v1 mega-lane is split into three legible objects with three revert boundaries:**

| Lane | Contents | Flip dep | Exit proof |
|------|----------|----------|-----------|
| **0 — enabler (now, small)** | `withTenantTx` (+ `anonymous` ctx + `withCourierTx` money veneer) · `no-bare-set-config` lint (lands red) · ALS dev/test tripwire | none | helper unit tests; lint red trajectory starts |
| **A — legal + detector integrity (now, fast-track — ES-1; go recorded separately from Lane B)** | LC4 redesign (§3) + **LC4-MIG** (policy arms + `gdpr_claim_due` — the lane's only operator gate) · anonymizer v10 array fix · per-registration boot isolation + boot-registry watch-set (§4.4) · `.dlq` wiring + DLQ monitor for the GDPR queue · O-GDPR level-triggered check · owner terminal surface/notification/receipt | none | P5 (extended) · P7 (extended) |
| **A′ — queue-policy sweep (follows A, no flip dep)** | OPEN-V1 spike → queue-policy reconciler (or `singletonSeconds` fallback) · `QUEUE_POLICY` map · retry/backoff/expire defaults · remaining direct-`boss.work` array fixes · generic `.dlq` monitor for all queues | none | P6 (rewritten) |
| **B — flip preconditions (large, staged, operator-gated per step)** | 49-site conversion + the FORCE-table readers grep missed (checkout `orders.ts`, `customer/track.ts`, `plugins/auth.ts`, telegram §1.7) · MIG-1 firebreak (after OPS-READ-CHECK) · MIG-4 · public-lane conversion · MIG-2 · MIG-3 · B3 ramp preconditions | **all of it** | P1 · P1b · P2 · P3 · P9 · P10 · P11 + the named gates |

```
Lane B critical path                                 B3 ramp (existing ADR)            LATER
─────────────────────────────────────────────────    ───────────────────────────────   ──────────────
1. Lane 0 lands (helper + lint red)                  6. per-lane SET LOCAL ROLE
2. Shape-B fixes (onboarding, spa-proxy)  ─┐            flags (owner→courier→anon),
3. Shape-A/semantic conversions (§1.5v2)   ├─ P2/P3     staging soak per lane          9. ALTER ROLE …
   incl. telegram §1.7 (P11)              ─┘         7. E2E lifecycle per lane            NOBYPASSRLS
4. OPS-READ-CHECK → MIG-1 + MIG-4          ── P1 + P1b red→green                         (converged
5. public-lane conversion (P9/P10 green)             8. GATE-FLIP-E2E + full soak          end-state)
   → GATE-ANON-E2E → MIG-2/3
```

Hard edges — now **deterministic gates, not prose** (counsel #2 / breaker F2): **(2,3) before (6)**
— enforced by P2/P3 red-until-converted; **(5) before MIG-2/3** — enforced by P10 (UNCONVERTED=0)
+ **GATE-ANON-E2E**; **MIG-1..4 before (9)** — enforced by **GATE-FLIP-E2E** (and **GATE-OSS-RLS**
for the ADR-020 open-source decision, ES-2). Lanes A/A′ have **no ordering dependency** on any of
this and do not wait — LC4 is a legal red-line fixable this week (council fast-track per
synthesis B-3), with its go recorded as a separate human decision (ES-1).

---

## 7. Blast radius / rollback

- Helper conversion: semantic-class per-site diffs (§1.5v2 — never blind wraps), each provable by
  P2/P3/P11 + the route's existing tests; reversible per-site by revert (no schema coupling).
  Multi-write flows explicitly **preserve** today's partial-failure semantics (breaker F6) — no
  atomicity regression to roll back.
- MIG-1 (firebreak): inert for the BYPASSRLS main pool; **immediately effective for non-BYPASSRLS
  roles** (operational read pool) — hence OPS-READ-CHECK precedes it; rollback = policy DROP
  migration (break-glass only).
- MIG-2/3/4: additive/behavior-visible per v1 analysis (unchanged); gated by GATE-ANON-E2E +
  P9/P10; failure mode fail-closed, never a leak; flag-revert of the B3 lane restores service in
  seconds (B3 rollback model).
- Queue policy: per-queue options via the reconciler; revert per queue. `.dlq` queues are additive.
  OPEN-V1 bounds the reconciler's risk before any code exists.
- LC4 (corrected — breaker F4): worker-local logic + one recon check + **one additive migration**
  (LC4-MIG: two policy arms + one DEFINER function; zero new columns). Rollback: revert the worker
  logic per-file; policy-arm removal is a break-glass DROP/recreate of the two policies.

## 8. Explicit non-goals (adjacent council lanes, not respecified here)

Security F1 (PATCH IDOR), F2 (anonymizer `location_id` predicate — required-before-flip, owned by its
own lane), F3/F4 (`requireRole`), R-A data-access seam (A2), CI-real (A1 — but P1's un-skip rides it),
health aggregate (H7), backup/DR (LC7, protect-path lane).

## 9. Correction of record (ES-2 — binding, needs operator sign-off)

**As staged today, the deferred B3 flip (`ALTER ROLE dowiz_app NOBYPASSRLS`) isolates NOTHING on
the credential tables it was believed to protect.** `couriers` (password_hash, encrypted PII) and
`courier_sessions` (token_hash, family_id) have **no RLS enabled anywhere**
(`1780421029538`, `1780421032856`); a table without RLS has no policies for a role flip to enforce.
The same holds for the ENABLE-only token-bearing tables (`customer_track_grants`,
`provision_grants`, `claim_invites` — MIG-4 scope). Any prior statement or belief of the form
"B3 staged ⇒ credentials isolated" is **false** and is corrected here. Consequence:
**GATE-OSS-RLS** — no ADR-020 open-source flip proceeds on any isolation premise until MIG-1 +
MIG-4 have landed and P1 + P1b are green. The operator signs this correction as a decision-log
line so the false belief cannot silently survive into an irreversible act.

**needs-human register:** (1) Lane-A standalone go (LC4-MIG is a migrations red-line) — recorded
separately from any Lane-B approval (ES-1); (2) this §9 correction sign-off; (3) direct-to-subject
erasure receipt channel (§3.8); (4) each MIG-1..4 / LC4-MIG prod application (standing rule).

---

*Companion fact sheets (verified source inventories): `site-inventory.md` (every set_config site),
`rls-state.md` (migration-by-migration RLS state — §4 firebreak description corrected per
`resolution.md §0.1`), `pgboss-state.md` (versions, queues, boot chain).*
*Council record: `breaker-findings.md` → `counsel-opinion.md` → `resolution.md` (per-finding
dispositions, DEFER/ACCEPT-RISK/needs-human registers).*
*ADR draft: `docs/adr/ADR-audit-fix-rls-reliability.md` (synced to v2 in this round).*
