# Resolution R2 — audit-fix-rls-reliability (Council RESOLVE round 2, re-attack on v2)

- **Status:** RESOLVED-DRAFT R2 — every R2 breaker finding dispositioned. DESIGN-ONLY, docs-only.
  Nothing here is self-certified; every disposition names the proof that must go red→green, and none
  of those proofs is claimed run in this round.
- **Date:** 2026-07-03
- **Inputs:** `breaker-findings-r2.md` (re-attack), `proposal.md` (v2), `resolution.md` (v1 RESOLVE),
  `ADR-audit-fix-rls-reliability.md` (context; **not edited** — sync flags in §4).
- **Live-source re-verification** performed this round against HEAD of `fix/audit-remediation`
  (file:line log in §0). **Caveat:** a live Lane-A implementation is concurrently editing
  `apps/api/src/workers/anonymizer-gdpr.ts` and `apps/api/src/lib/anonymizer/index.ts`; the line
  numbers below are current-on-disk at read time and may drift under Lane-A. No code/migration files
  were touched in this round.
- **Rule applied:** each finding → **FIX** (concrete design) / **ACCEPT-RISK** (justified + owner) /
  **DEFER-FLAG** (parked + owner + re-entry trigger).

---

## 0. Verification log (grounds N1/N2/N3 against live source — not the proposal's citations)

| Claim | Verified basis (HEAD `fix/audit-remediation`) | Verdict |
|---|---|---|
| `customers` has **no** `app.current_tenant` policy arm | `tenant_isolation USING (location_id IN (SELECT app_member_location_ids()))` (`packages/db/migrations/1780310074262_orders.ts:76-77`, member-only, **no WITH CHECK**); `anonymous_update`/`anonymous_select USING (app_current_user() IS NULL)` (`1780338981782_customer-anonymous-update.ts:6-11`); `anonymous_insert WITH CHECK (app_current_user() IS NULL)` (`1780315000000_customer-rls.ts:16-17`). The `app.current_tenant` (RC4) arm exists **only** on `orders`/`delivery_trace`/`courier_cash_ledger` (`1790000000077_rls-nobypassrls-phase1-policies.ts:44-67`). | **CONFIRMED** |
| The anonymizer runs on its **own context-free** pool connection | `anonymizeCustomer`: `this.pool.connect()` (`apps/api/src/lib/anonymizer/index.ts:131`) → `BEGIN` (`:133`) → `SELECT anonymized_at, location_id FROM customers WHERE id=$1 AND location_id=$2 FOR UPDATE` (`:134-137`) → `UPDATE customers …` (`:148-156`) → `COMMIT` (`:196`). No `set_config`/GUC anywhere in the file. `anonymizeOrder` is the same shape at `:220`. `anonymize()` takes no client (`:50`). | **CONFIRMED** (breaker's :115/:192 shifted to :131/:220 under Lane-A edits; structure identical) |
| Worker writes `completed` **regardless of `result.skipped`** | `anonymizer-gdpr.ts`: `anonymize({scope:'gdpr', subject:{customerId, locationId: row.location_id}})` (`:62-65`); then **unconditional** `UPDATE … SET status='completed'` (`:67-72`), audit INSERT (`:86-103`), `gdpr.erasure_completed` publish (`:105-108`). `result` is never inspected for `skipped`. | **CONFIRMED** |
| Post-flip visibility on the anonymizer's connection | On a context-free connection under NOBYPASSRLS+MIG-2: `tenant_isolation` → `app_member_location_ids()` on NULL `app.user_id` → ∅; scoped `anonymous_*` → `app_current_user() IS NULL AND location_id = <app.current_tenant=NULL>` → NULL → ∅. `SELECT … FOR UPDATE` matches **0 rows** → `index.ts:138` ROLLBACK → `{anon:false, skipped:true}`. | **CONFIRMED (by RLS semantics; must be pinned by the new P-proof, §1 N1)** |
| N2 arm blast radius | Both `gdpr_tenant_isolation` and `anonymization_audit_tenant_isolation` are `FOR ALL` (SELECT+INSERT+UPDATE+DELETE), USING+WITH CHECK, member-only (`1780421100060_anonymization-seam.ts:49-51,:57-59`); `gdpr_erasure_requests` carries `subject_phone`/`customer_id`/`reason` (`:16-17`); `anonymization_audit_log` is documented append-only (`:70`). `app.current_tenant` is set by the courier-shift/webhook lane (`1790000000077:44-45` comment; telegram/payments webhooks per proposal §0.1). | **CONFIRMED** |
| N3 failed-login INSERT | `courier/auth.ts:269-273` INSERT into `courier_audit_log` runs on `db.connect()` (`:246`), no `app.current_tenant`; `failLocationId` may be the zero-UUID fallback (`:268`). Policy `isolate_courier_audit_log USING (location_id = NULLIF(current_setting('app.current_tenant',true),'')::uuid)` (`1790000000077:71-73`), `FOR ALL`, USING-only → INSERT WITH CHECK inherits USING → `location_id = NULL` fails → 42501. | **CONFIRMED** |

**Critical nuance for scheduling (not in the breaker text, load-bearing for §2):** N1's silent
false-completion is gated on **both** the NOBYPASSRLS flip **and MIG-2**. *Pre-MIG-2* but post-flip,
the anonymizer's context-free connection is treated as the anonymous principal: the *unscoped*
`anonymous_select`/`anonymous_update` (`app_current_user() IS NULL`) **admit**, so the erasure still
runs (scoped only by the query's own `AND location_id=$2` — the specific row is erased correctly,
though the anon lane on a context-free connection is broadly over-permissive). N1's *silent
false-completion* manifests only once MIG-2 scopes the anon policies to require `app.current_tenant`.
So N1 is a **flip + MIG-2 latent** — even further downstream than the flip alone — which is why it
does not block Lane-A *shipping* but does force a Lane-A *rework* of the completion logic (§2).

---

## 1. Dispositions

| # | Sev | Disposition | Lane / owner |
|---|-----|-------------|--------------|
| N1 | CRIT | **FIX** (combined: fail-loud backstop now + structural context fix + data-level P-proof) | Lane A (backstop + P-proof + LC4-MIG DEFINER); structural-success gate → GATE-FLIP-E2E |
| N2 | HIGH | **FIX** (DEFINER-ize the worker's two-table writes → **no arm added**; if arm retained, command-split, no DELETE) | Lane A (LC4-MIG) |
| N3 | MED | **FIX** (firebreak `courier_audit_log` INSERT + exercise the *failed* login path in P1b/P9) | Lane B (MIG-1 + conversion) |
| N4 | MED | **FIX** (reconciler always passes full policy from `QUEUE_POLICY`; wrapper forbids partial `updateQueue`) | Lane A′ |
| N5 | LOW | **FIX** (helper transaction-locally RESETs both GUC families to `''` before setting ctx keys) | Lane 0 |
| N6 | LOW | **DEFER-FLAG** to reliability H7 (+ recommended FIX-lite: level-triggered boot-completeness DRIFT via a static expected-roster diff) | H7 owner; FIX-lite optional in Lane A |
| N8 (INFO-1) | INFO | **ACCEPT-RISK** (pre-existing, not worsened by v2) + **DEFER-FLAG** (make the Telegram secret-token header check mandatory) | Telegram/security lane (proposal §8 non-goal) |

### N1 (CRITICAL) — FIX. The erasure must actually run under enforcement AND the worker must never write `completed` on a non-erasure.

**Broken invariant (confirmed §0):** proposal §3 "the erasure path itself survives the flip"; v1
resolution F3 "per-row work runs in `withTenantTx({tenantId})` … correct post-flip." The bookkeeping
survives (via LC4-MIG's arm on the two GDPR tables), but the **data erasure** runs on the
anonymizer's private context-free connection and is invisible to `customers` RLS post-MIG-2, while
the worker writes `completed` unconditionally.

**The three fix options weighed (task-directed):**

- **(a) Thread the erasure through `withTenantTx`** — pass the worker's tenant-context client into
  `anonymize()` for the by-id GDPR path so the `SELECT … FOR UPDATE`/`UPDATE customers` run inside
  the worker's transaction. **Insufficient alone, and confirmed so by §0:** the worker's context is
  `app.current_tenant = row.location_id` (courier/webhook/worker lane, proposal §3.2), but `customers`
  has *no* `app.current_tenant` arm — so even a correctly-threaded client sees **∅**. Threading only
  helps if it is combined with a policy arm that matches the context. It also collides with the
  anonymizer's shared **retention** path (`index.ts:96-111`, cross-tenant nightly batch), which cannot
  run inside one tenant transaction — so (a) can apply to the by-subject path only, via an optional
  caller-supplied client. Net: necessary plumbing, not by itself a fix.

- **(b) Add a `customers` `app.current_tenant` arm mirroring `orders.courier_tenant_update`
  (`1790000000077:50-53`).** Mechanically restores visibility, but **repeats N2 at larger scale**:
  `customers` is the *primary* PII table; a table-wide `app.current_tenant` arm hands every
  courier-shift/webhook principal SELECT/UPDATE on all customers at their location. **REJECTED as the
  primary mechanism** — it trades a legal-red-line liveness hole for a legal-red-line confidentiality
  hole. (If ever used, it must be command-split — SELECT+UPDATE only, no INSERT/DELETE — and even then
  the blast radius is unacceptable for `customers`.)

- **(c) Make `completed`/audit/event CONDITIONAL on `result.skipped===false` AND a post-erasure
  re-read assertion `customers.anonymized_at IS NOT NULL`; else `failed` + `error_message` + DLQ.**
  Worker-local, no migration, and it **removes the word "silent" from "silent false-completion"**:
  post-flip a non-erasure lands `failed` + `anonymizer.gdpr.dlq` + O-GDPR level-trigger (loud, owned,
  re-alerting), never falsely `completed`. This directly reverses proposal §3.4 / §3.7-F7's
  "regardless of `result.skipped`."

**Recommended combination — makes silent false-completion structurally impossible two independent ways:**

1. **Structural success (visibility) — a SECURITY DEFINER erasure function**, e.g.
   `gdpr_erase_customer(p_customer uuid, p_location uuid)` (search_path-pinned per ledger #33),
   returning the resulting `anonymized_at` (and any `avatar_key` to purge). This runs as the function
   owner, so RLS visibility is a non-issue; scoping is the function's own `WHERE id=p_customer AND
   location_id=p_location` predicate — the *same* discipline the anonymizer already uses (`index.ts:135`).
   It is **exactly the "tiny auditable DEFINER ingress-resolver" convention the proposal already
   embraces** (`gdpr_claim_due`, `resolve_telegram_chat` §1.7, `resolve_location_slug` §2.2.4a,
   `app_member_location_ids()`), and — decisively — it **fixes N1 and N2 together** (see N2 below: no
   `customers` arm, no arm on the two GDPR tables). The app-layer side effects that a SQL function
   cannot do (R2 avatar `storage.delete`, bus publish) stay in the worker, keyed off the function's
   returned `avatar_key`. This rides **LC4-MIG (Lane A's own migration)** — no Lane-B dependency.
   *Lighter alternative if the DEFINER refactor of `lib/anonymizer/index.ts` is too large for the
   in-flight Lane-A edit:* run the anonymize connection under the **anon-lane context**
   `withTenantTx({anonymous:true, tenantId: location_id})` and rely on MIG-2's scoped anon policies to
   admit exactly that location — reuses the existing anon-lane GUC mechanism (proposal §2.2.2) with no
   `customers` arm, but couples erasure correctness to MIG-2 landing and semantically overloads the
   anon lane. DEFINER is preferred; either avoids option (b)'s blast radius.

2. **Fail-loud backstop (option c) — mandatory now, independent of (1).** Even if (1) regresses or
   MIG-2 ordering slips, the worker asserts data-level erasure before writing a terminal success:
   `completed` + audit + event fire **only** when `result.skipped===false` **and** a re-read confirms
   `anonymized_at IS NOT NULL`; otherwise the row goes `failed` (+ `error_message='erasure produced
   no effect'`) → `anonymizer.gdpr.dlq` → O-GDPR level-trigger. The claim-token CAS (F7) still dedups
   the terminal write to exactly one — but now to the *correct* terminal, not the false `completed`.

3. **P-proof (new — the data-level erasure gate that P5 lacks):** on a **NOBYPASSRLS + MIG-2**
   rehearsal DB, drive one GDPR erasure end-to-end and assert **at the data level**: the target
   `customers.anonymized_at IS NOT NULL` and `phone` is nulled/tokenised (`index.ts:148-156` effect).
   Add the **negative**: with the DEFINER function absent / context withheld, the request must land
   `failed` + a job in `anonymizer.gdpr.dlq`, **never `completed`**. This proof goes **red on the
   current design** (unconditional `completed`, no data assertion) and green only on the combined fix.
   It is a strict extension of P5 (which today asserts only status/audit-count/event-count/reclaim,
   confirmed §0) — and must be a **red→green REGRESSION-LEDGER row** (legal red-line; test-integrity
   rule: no green on a false completion).

**Why the combination is structurally sufficient:** (1) makes the erasure *succeed* under
enforcement without a broad policy arm; (2) makes any *failure* of (1) loud and owned rather than a
`completed`; (3) makes both properties falsifiable at the exact end-state. Silent false-completion
requires all three to be absent simultaneously — the design no longer permits that.

### N2 (HIGH) — FIX. Do not widen the two legal-red-line tables at all; route the worker's writes through DEFINER.

The v2 mechanism that resolved F3 (add an `app.current_tenant` arm to `gdpr_tenant_isolation` +
`anonymization_audit_tenant_isolation`) is what creates N2: both are `FOR ALL`, member-only today
(`1780421100060:49-51,:57-59`), and the arm grants the entire courier-shift/webhook principal class
CRUD — including **DELETE on erasure requests** and **INSERT/DELETE on the append-only audit log**
(§0). There is no way to scope a GUC arm to "only the worker."

**Fix (dovetails with N1's recommended mechanism):** the worker's terminal writes (`status`
transitions on `gdpr_erasure_requests`) and its `anonymization_audit_log` INSERT run through
**SECURITY DEFINER functions** (the claim is *already* DEFINER — `gdpr_claim_due`, proposal §3.1 —
so extend the same lane: a DEFINER `gdpr_finalize(request_id, claim_token, outcome, …)` that performs
the CAS terminal write + audit INSERT as owner). Then **LC4-MIG adds no `app.current_tenant` arm to
either table** — the member-only policies (`1780421100060:49-51,:57-59`) are preserved, and the N2
blast radius disappears. This also removes the last reason the *worker bookkeeping* needed
`withTenantTx({tenantId})` to match those two tables, simplifying the design.

**Fallback if the arm is retained** (e.g. the team prefers `withTenantTx` bookkeeping over a
`gdpr_finalize` DEFINER): the arm MUST be **command-split** — `FOR SELECT` + `FOR UPDATE` on
`gdpr_erasure_requests` (no INSERT/DELETE — inserts come from the member-context owner route
`routes/owner/gdpr.ts`), and `FOR INSERT` only on `anonymization_audit_log` (never UPDATE/DELETE —
preserve append-only). This prevents the courier/webhook class from deleting erasure requests or
forging/erasing audit rows, converting N2 from "full CRUD" to "read+append at own location." Add a
**P8-hygiene assertion**: no `FOR ALL`/DELETE `app.current_tenant` arm may exist on either table.
Prefer the DEFINER route (zero arm) — it is the same convention N1 already recommends.

### N3 (MEDIUM) — FIX. Firebreak the failed-login audit INSERT; prove the failed path.

Confirmed (§0): `courier/auth.ts:269-273` INSERTs `login.failed` into `courier_audit_log` on a
pre-context `db.connect()`; post-flip the RC5 policy's WITH-CHECK (inherited from USING,
`1790000000077:71-73`) rejects `location_id=NULL` → 42501 → the throw turns a wrong-password **401**
into a **500**. F1's firebreak (MIG-1) covers `couriers`/`courier_sessions`/`courier_locations`, not
`courier_audit_log`.

**Fix:** extend **MIG-1** with a role-restricted write arm for the pre-context audit INSERT —
`CREATE POLICY courier_auth_write ON courier_audit_log FOR INSERT TO dowiz_app WITH CHECK (true)`
(mirrors MIG-1's `courier_auth_read FOR SELECT TO dowiz_app` on `courier_locations`, proposal §2.2.3).
Login has no authoritative tenant here (the zero-UUID fallback at `auth.ts:268` proves it), so a
tenant predicate is the wrong tool — the firebreak (role-restriction) is. Then **P1b/P9 must exercise
the *failed* login path** (wrong password) under the NOBYPASSRLS probe → expect **401, not 500**; the
current P1b scope is the *valid* path only (confirmed proposal §5 P1b). This is a FORCE-table writer
the `grep set_config` inventory misses — **P10** must classify it. Lane B (rides MIG-1 + conversion),
not Lane A. MEDIUM: availability/observability regression on the failure path, flip-gated.

### N4 (MEDIUM) — FIX. The reconciler always passes the full policy; forbid partial `updateQueue`.

The breaker's HEAD trace (pg-boss@10.4.2 `manager.updateQueue` `const { policy =
QUEUE_POLICIES.standard } = options` → `UPDATE … SET policy = COALESCE($2, policy)`) means an omitted
`policy` **resets** a `'short'` queue to `'standard'`, silently disabling `singletonKey` dedup. (Not
re-verified against `node_modules` this round — accepted as a designed mitigation regardless.)

**Fix:** the queue-policy reconciler reads the intended policy from the single `QUEUE_POLICY` map
(proposal §4.2) and passes it **complete on every `updateQueue` call** — never a partial options
object. Add a thin project-local `updateQueue` wrapper that **requires** `policy` (compile/lint
error if omitted), so a future partial call cannot regress dedup. **P6** gains an assertion: after a
reconcile that targets only `retryLimit`/`deadLetter`, a pre-existing `'short'` queue **remains
`'short'`**. Lane A′; low effort.

### N5 (LOW) — FIX. The helper transaction-locally resets both GUC families before setting the ctx.

`BEGIN` does not clear a session-scoped GUC left by a prior Shape-B borrower (proposal documents the
leak at `onboarding.ts:75`, `spa-proxy.ts:771`), so `{anonymous:true}` *inherits* a leaked
`app.user_id`/`app.current_tenant` rather than being context-free — the marker asserts a property it
does not enforce.

**Fix:** `withTenantTx` **always** issues transaction-local resets first —
`set_config('app.user_id','',true)` and `set_config('app.current_tenant','',true)` — then sets only
the keys the ctx names. Transaction-local `''` shadows any leaked session value for the txn's
duration, so `{anonymous:true}` is genuinely context-free and *every* context is hardened against
inheriting a leaked session GUC (defense-in-depth for the pre-flip window before all Shape-B leaks
land). Nearly free, lives in Lane 0's one helper. **P3** extends to assert that an `{anonymous:true}`
txn on a pool connection deliberately pre-poisoned with a session GUC still reads `current_setting`
as empty for both families. LOW.

### N6 (LOW) — DEFER-FLAG (H7) + recommended FIX-lite.

The registry watch-set correctly cannot heartbeat-monitor a worker that never started (proposal §4.4
dropped the static list *for liveness* — correct). So a worker whose own registration throws is
detected only by a one-shot boot DRIFT + a degraded `/health`, and the level-signal for *never-run*
workers rests on H7 paging on `/health degraded` — **out of this proposal's scope** (§8 non-goal).

**Disposition: DEFER-FLAG** the paging to reliability **H7** (owner: reliability lane; re-entry
trigger: H7 lands, or a never-started worker escapes detection in a soak). **Recommended FIX-lite
Lane A can adopt cheaply:** keep a **static expected-roster** used *only* for a **level-triggered
boot-completeness DRIFT** — at each reconciliation run, `expected − actually-started = missing → DRIFT
+ Sentry` (mirrors O-GDPR's level-trigger, proposal §3.7). This is a *different* set from the liveness
watch-set (which stays the actually-started registry), so it does not re-introduce the false-DRIFT
problem F12 fixed — it closes the never-started seam without waiting for H7. LOW.

### N8 / INFO-1 (INFO) — ACCEPT-RISK + DEFER-FLAG.

The chat-based tenant anchor (`from.id`) is the same principal the current code uses; the webhook is
gated by a shared URL secret and an *optional* secret-token header that processes even when absent
(`telegram-webhook.ts:57-60`). **Pre-existing; v2 only reorders resolve→act and does not worsen it**
(the resolve→act TOCTOU is bounded by the guarded `updateOrderStatus` rowcount check, proposal §1.7).
**ACCEPT-RISK** (owner: telegram/security lane; the v2 redesign inherits but does not introduce the
property). **DEFER-FLAG one cheap hardening** to that lane: make the Telegram secret-token header
check **mandatory** (reject on absent/mismatched header) instead of optional — removes URL-secret-only
spoofability. Not in this proposal's scope (§8).

---

## 2. Does N1 block the live Lane-A implementation? — REWORK, not pause.

**Shipping is not blocked; the completion logic must be reworked before Lane-A's exit proof is accepted.**

- **N1 is a flip + MIG-2 latent** (§0 nuance): BYPASSRLS masks it today, and even the bare flip
  (pre-MIG-2) still erases correctly via the unscoped anon policies. The *silent false-completion*
  cannot occur until NOBYPASSRLS **and** MIG-2 — both are downstream **Lane B**, gated by
  GATE-FLIP-E2E. So Lane A does **not** have to *pause* — the erasure is correct under today's runtime.

- **But the unconditional `completed`/audit/event write is a TODAY correctness bug, narrow but real,
  and Lane A is redesigning exactly this logic right now.** Under BYPASSRLS, `result.skipped` can be
  `true` on a *non*-idempotent path today: if the resolved `customerId` (from `row.customer_id`,
  `anonymizer-gdpr.ts:43`) does not match the request's `location_id`, the anonymizer's
  `WHERE id=$1 AND location_id=$2` (`index.ts:135`) returns 0 rows → `skipped:true` (`index.ts:138-140`)
  → the worker still writes `completed` (`:67-72`) though nothing was erased. (The already-anonymized
  case — `index.ts:143` — is also `skipped:true`, but there `completed` is *semantically* correct.)
  The bad edge requires a data inconsistency / hard-delete race, so it is rare today — but it is
  **structurally the same write** that becomes a *certain* legal-compliance failure at the MIG-2+flip
  end-state.

- **The proposal bakes the hole in.** Proposal §3.4 / §3.7-F7 explicitly write `completed` + audit +
  event "regardless of `result.skipped`," and **P5 does not assert data-level erasure** (confirmed
  §0). If Lane A ships the redesign as written, it ships (i) a foundation for permanent silent
  false-completion and (ii) a proof (P5) that goes **green on a false completion** — a test-integrity
  red-line on a legal surface.

**Therefore Lane A must REWORK, within its own file surface, before its DoD is credited:**

1. **Reverse "regardless of `result.skipped`"** → the fail-loud backstop (N1 option c): `completed`
   + audit + event only on `skipped===false` **and** `anonymized_at IS NOT NULL` re-read; else
   `failed` + DLQ. *Worker-local (`anonymizer-gdpr.ts`) — the file Lane A already owns.* This alone
   downgrades N1 from "silent, permanent, undetected" to "loud, owned, level-triggered failure" — and
   is sufficient to make Lane A **safe to ship** even before the structural fix.
2. **Structural success fix** (DEFINER `gdpr_erase_customer` preferred; anon-lane context as the
   lighter alternative) so post-flip erasures *succeed* rather than fail-loud-every-time — rides
   **LC4-MIG (Lane A's own migration)**, no Lane-B dependency. If the team prefers to stage this, it
   may be a **DEFER-FLAG gated by GATE-FLIP-E2E** (owner: flip-prep), because the fail-loud backstop
   makes the interim safe (failures are visible, not silent). Recommendation: land it in Lane A since
   the migration and the anonymizer file are already open.
3. **Extend P5 with the data-level P-proof** (§1 N1.3) as a red→green REGRESSION-LEDGER row before
   Lane A's exit is accepted.

**One-line answer:** Lane A need not pause, but it **must not ship the redesign with unconditional
`completed`/audit/event writes or a P5 that omits the data-level assertion**; the fix is a rework of
the in-flight redesign, entirely within Lane-A's own files (`anonymizer-gdpr.ts`,
`lib/anonymizer/index.ts`) plus LC4-MIG.

---

## 3. needs-human register (R2 additions)

1. **N1 correction of record (binding):** the v2 claim "the erasure path itself survives the flip"
   is **false as designed** — the data erasure runs on a context-free connection invisible to
   `customers` RLS post-MIG-2, and the worker writes `completed` regardless of erasure. Operator
   signs one decision-log line acknowledging this and that **GATE-FLIP-E2E now includes the
   data-level erasure P-proof** (target `customers.anonymized_at IS NOT NULL`, negative-case
   `failed`+DLQ). Analogous to ES-2's correction-of-record.
2. **Lane-A rework acknowledgement:** the Lane-A go (already a needs-human per v1 §4.1) is
   re-scoped — its DoD now requires the N1 backstop + extended P5. Recorded, not granted here.
3. **Structural-fix mechanism choice** (DEFINER `gdpr_erase_customer` vs anon-lane context vs — 
   rejected — a `customers` arm): a red-line migration-shape decision (`packages/db/migrations/**`),
   operator-gated. Recommendation on record: DEFINER (fixes N1+N2 together, no blast-radius arm).
4. **N3 firebreak on `courier_audit_log`** and **N2's DEFINER-ization / command-split** are migration
   red-lines — each individually operator-gated (standing rule), on top of the existing MIG-1..4 /
   LC4-MIG gates.

## 4. ADR sync flags for the lead (do NOT edit the ADR in this round)

`docs/adr/ADR-audit-fix-rls-reliability.md` must be synced by the lead:

- **Decision #3 (GDPR erasure liveness):** currently states per-row work in `withTenantTx({tenantId})`
  + "all terminal writes are claim-token CAS-guarded and side effects fire only on CAS win —
  exactly-once." **Amend** to: the erasure **data path** runs under an enforcement-valid mechanism
  (DEFINER `gdpr_erase_customer`, or anon-lane context) — *not* the worker's `app.current_tenant`
  context, which `customers` RLS does not honour; and terminal `completed`/audit/event are
  **conditional on `result.skipped===false` AND `anonymized_at IS NOT NULL`, else `failed`+DLQ**
  (reverse "regardless of `result.skipped`"). This also re-opens v1 dispositions **F3 and F7**, which
  the R2 breaker notes "faithfully dedup N1's *wrong* outcome."
- **Decision #3 / LC4-MIG scope:** add the DEFINER erasure (and, per N2, DEFINER `gdpr_finalize` for
  the terminal write + audit INSERT so **no `app.current_tenant` arm is added** to
  `gdpr_erasure_requests` / `anonymization_audit_log`; or command-split arms if retained).
- **Decision #2 / MIG-2:** record that adding a `customers` `app.current_tenant` arm is **REJECTED**
  (N2-repeat on the primary PII table); MIG-2's anon scoping interacts with N1 (post-MIG-2 the
  context-free anonymizer sees ∅).
- **Decision #2 / MIG-1:** add the `courier_auth_write FOR INSERT TO dowiz_app` firebreak on
  `courier_audit_log` (N3).
- **Decision #1 (`withTenantTx`):** add the transaction-local reset of both GUC families (N5).
- **Decision #4 (pg-boss):** add "reconciler always passes full policy; partial `updateQueue`
  forbidden" (N4); note the N6 boot-completeness level-DRIFT + H7 paging dependency.
- **Verification block:** **P5 extended** with data-level erasure assertion + negative `failed`+DLQ
  case (new P-proof); **P1b/P9** exercise the *failed* courier login path (N3); **P8** asserts no
  `FOR ALL`/DELETE `app.current_tenant` arm on the two GDPR tables (N2); **P3** anon-reset assertion
  (N5); **P6** short-stays-short after partial reconcile (N4).
- **Consequences / needs-human:** add the N1 correction-of-record (§3.1) and the re-scoped Lane-A DoD.

Proposal sync (companion, for the lead): §3.4, §3.7-F7 ("regardless of `result.skipped`"), §3.9
(anonymizer scoping is *visibility*, not just a sink predicate), §5 (P5 + new P-proof), §6 (Lane A
contents), §2.3 (MIG-2 / rejected customers arm), and resolution.md F3/F7 (re-open).

---

## 5. Six-line summary

1. **N1 verified against live source and stands as CRITICAL:** `customers` has no `app.current_tenant`
   arm (`1780310074262:76-77`; RC4 arm is orders-only, `1790000000077:44-67`); the anonymizer erases
   on its own context-free connection (`index.ts:131,220`); the worker writes `completed` regardless
   of `result.skipped` (`anonymizer-gdpr.ts:67-72`) → silent false Art.17 completion at NOBYPASSRLS+MIG-2.
2. **N1 = FIX (combination):** fail-loud backstop now (conditional `completed` + `anonymized_at`
   assertion → else `failed`+DLQ) **+** a DEFINER `gdpr_erase_customer` for real post-flip visibility
   (rejecting a `customers` arm as an N2-repeat) **+** a new data-level P-proof under NOBYPASSRLS+MIG-2;
   the three make silent false-completion structurally impossible.
3. **N2 = FIX:** DEFINER-ize the worker's two-table writes so LC4-MIG adds **no** `app.current_tenant`
   arm to the PII/append-only GDPR tables (fallback: command-split, no DELETE) — the DEFINER route
   fixes N1 and N2 together and matches the proposal's own convention.
4. **N3 FIX (Lane B — firebreak `courier_audit_log`, prove the failed login path), N4 FIX (Lane A′ —
   full policy on every `updateQueue`), N5 FIX (Lane 0 — helper resets both GUCs).**
5. **N6 = DEFER-FLAG to H7** (+ recommended level-triggered boot-completeness DRIFT); **N8/INFO-1 =
   ACCEPT-RISK** (pre-existing) + DEFER-FLAG the mandatory Telegram secret-token header to that lane.
6. **Lane A: REWORK, do not pause** — the erasure is correct under today's runtime, but Lane A must
   reverse "regardless of `result.skipped`" + extend P5 (both inside its own files) before its exit
   proof is credited; the structural DEFINER fix rides LC4-MIG with no Lane-B dependency.
