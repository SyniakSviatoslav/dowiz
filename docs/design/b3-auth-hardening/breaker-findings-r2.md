# B3 Auth Hardening — Breaker RE-ATTACK (round 2, regression + scope-leak)

Scope of this round (per conductor): **(1) regression** — did any r1 FIX introduce a NEW break?
**(2) scope-leak** — is anything dangerous still accidentally inside Phase-0 (RLS enforcement change /
lockout / irreversible-on-prod), i.e. is the flip *truly* deferred and untripped?
All facts re-verified against live source on 2026-07-03 (line cites below). No fixes proposed.

---

## Verdict up front

- **The NOBYPASSRLS flip is genuinely deferred.** No P0 item runs `ALTER ROLE … NOBYPASSRLS`,
  `SET LOCAL ROLE`, creates `dowiz_app_rls`, or otherwise turns on enforcement. P0-4's boot-guard is
  correctly gated on role existence → no-op on today's prod. **Confirmed: no P0 code item trips the flip.**
- **BUT Phase-0 is NOT deploy-safe on prod as scoped.** The r1 resolution deferred the *drift
  reconciliation* and the *flip*, yet left the Phase-1 policy migration `1790000000077` sitting in the
  **forward-only** migration set. The Phase-0 deploy vehicle (release_command) will attempt it, and it
  contains **two unguarded migrate-time throw triggers** against prod's actual role/schema. This is the
  scope-leak: the deploy that "ships Phase-0 NOW" can hard-abort on prod. **1 HIGH.**
- Two MEDIUM regressions in the r1 fix *specs* (P0-3 close-on-fail, P0-2 C2 txn conversion) — both
  grounded in the actual client/handler code the fix will touch.

---

## [HIGH] B-DATA / B-OPS · scope-leak — Phase-0's deploy runs migration `1790000000077`, which throws at migrate-time on prod against a role and a column that may not exist there

**What r1 did:** deferred the drift reconciliation (§4 item 6, marked MISSING; `grep RENAME COLUMN` = 0 →
no reconciliation migration exists) and deferred the flip. It did **not** remove, guard, or hold the
Phase-1 policy migration `packages/db/migrations/1790000000077_rls-nobypassrls-phase1-policies.ts`, which
is already committed and **forward-only**. Any prod deploy of this branch (P0-1 cred rotation needs a
coordinated redeploy; P0-2/3/4 ship code) runs `release_command` → node-pg-migrate applies every *pending*
migration → `1790000000077` executes if not already applied on prod.

**Trigger 1 — role does not exist (B-OPS role-drift).** `1790000000077` line 28/30 create policies
`CREATE POLICY ops_all ON users FOR ALL TO dowiz_app …` and line 42 `GRANT EXECUTE … TO dowiz_app`.
These are **bare, un-guarded** (unlike `080`, whose REVOKEs are wrapped in `BEGIN EXECUTE … EXCEPTION
WHEN OTHERS THEN END`). Proposal §"Verified seams" + §8 state **prod runs `deliveryos_api_user`, staging
uses `dowiz_app`** ("promote `dowiz_app`… retire `deliveryos_api_user` — staging's model"). If `dowiz_app`
does not exist on prod, `CREATE POLICY … TO dowiz_app` raises `role "dowiz_app" does not exist` → the
whole `077` transaction rolls back → deploy aborts. (This also proves `077` cannot have "already run"
cleanly on prod unless `dowiz_app` exists there.)

**Trigger 2 — column does not exist (B-DATA drift, M4 UNVERIFIED).** Same migration, lines 104-106:
`CREATE POLICY owner_isolation ON telegram_connect_tokens FOR ALL USING (owner_id = app_current_user())
WITH CHECK (owner_id = app_current_user())` — **unconditional `owner_id` reference, no
`information_schema` column-existence guard.** r1 itself marks the "prod keys on `user_id`" claim
**UNVERIFIED** (M4) and the reconciliation that would converge the column is **deferred**. If prod's
column is `user_id`, this throws `column "owner_id" does not exist` at migrate time.

**Scenario:** operator ships Phase-0 to prod (e.g., to complete P0-1's secret repoint). `release_command`
reaches pending `1790000000077`. Trigger 1 and/or Trigger 2 fires → migration transaction aborts → Fly
deploy aborts. Phase-0 **cannot reach prod** until the drift is introspected and `077`'s applied-state
confirmed — exactly the reconciliation r1 deferred. This is the 2026-06-20 schema-drift class, inverted in
ordering: the dependent policy migration is live in-tree while its prerequisite is deferred.

**Why HIGH not CRITICAL:** node-pg-migrate wraps each migration in a transaction (clean rollback) and a
failed `release_command` aborts the deploy while the **old** version keeps serving → no data-plane outage.
But it defeats the resolution's headline ("ship the truly-safe Phase-0 NOW") and is an unaddressed
role/schema-drift landmine in the exact deploy path Phase-0 must use.

**Violated invariant:** forward-only migrations must be re-run-safe against the *target* environment's
actual schema/roles (introspection-first; drift = deploy-blocker). r1's own R-4/M4 state this, but the
Phase-0 scope leaves the offending migration executable ahead of its deferred prerequisite.

**Verification needed (cannot reach prod from here):** `SELECT 1 FROM pg_roles WHERE rolname='dowiz_app'`
and `SELECT column_name FROM information_schema.columns WHERE table_name='telegram_connect_tokens'` on
**prod**, plus prod's applied-migration list. If `dowiz_app` exists on prod AND `owner_id` exists AND
`077` already applied → this finding collapses to informational. Nothing in the proposal/resolution
asserts any of the three; the 273-commit prod↔branch divergence (MEMORY) makes "077 pending on prod"
the likely state.

---

## [MEDIUM] B-FAIL · regression — P0-3 "close socket on failed URL-token verify" is designed against a wrong client model; it can kill a live `useWebSocket` session that has a valid message-auth behind the failed URL token

**Proposal claim (§"Verified seams", P0-3):** "`apps/web/src/lib/useWebSocket.ts` uses **message** auth"
— implying it does NOT send a URL token, so closing on URL-verify-failure would only affect URL-only
cached clients.

**Actual source — `apps/web/src/lib/useWebSocket.ts` dual-sends:**
- line 50: `if (token) url.searchParams.set('token', token)` → **URL token IS sent** by the live web client.
- line 61: `ws.send(JSON.stringify({ type: 'auth', token }))` → message-auth sent too.

The token is read from storage **twice** — line 48 (for the URL, at `connect`) and line 59 (for the
message, at `onopen`). Today's server treats a failed URL verify as **non-fatal** (`websocket.ts:352-354`
only logs; the client can still authenticate via the subsequent message). P0-3 changes that to an
immediate `ws.close(1008)`.

**Scenario:** a background token refresh writes `dos_access_token` between line 48 and line 59 (owner
24h-token boundary + a WS reconnect). The URL carries the stale token, the message carries the fresh one.
Under P0-3: URL verify fails → socket closed 1008 → the valid message-auth (line 61) is never honored →
the owner/courier dashboard's live channel is killed even though it holds a valid credential. Today this
client is resilient precisely because the URL token is best-effort with a message fallback; "close on
failed URL verify" converts a redundant belt-and-suspenders into a single point of fatal failure.

Note the customer status client `apps/api/src/client/status/ws.ts` is URL-only (line 17) with no
message fallback and gives up on 1008 (lines 40-42) — for it the close-early vs 5s-timeout end-state is
identical, so no regression there. The regression is specific to the **dual-send** `useWebSocket` client
the proposal mis-modeled.

**Violated invariant:** a fallback path must not cascade into fatal failure while a valid alternative
credential is in flight. The r1 blast-radius reasoning ("only cached PWAs send URL tokens") is factually
wrong — the current live web client sends URL tokens on every connect.

---

## [MEDIUM] B-CONSIST / B-FAIL · regression — the P0-2 C2 "explicit BEGIN…COMMIT, is_local=true" conversion collides with pre-existing transaction structure in both target handlers

The C2 fix (P0-2) offers "wrap each provisioning write-sequence in an explicit `BEGIN…COMMIT` with
`is_local=true`" as a co-equal option to "route through the canonical seam." The explicit-txn option is a
trap in **both** files because neither is a flat single-transaction sequence.

**`onboarding.ts` — a pre-existing nested seed transaction relies on the session GUC surviving.** The one
`set_config('app.user_id',…,false)` at **line 75** is relied on by two *separate* execution spans:
(a) the autocommit `UPDATE locations SET onboarding_state` at line 85, and (b) an explicit menu-seed
transaction at **lines 102 (`BEGIN`) → 121 (`COMMIT`) / 126 (`ROLLBACK`)** (deliberately its own txn:
comment lines 90-94). Converting line 75 to `is_local=true` inside a wrapper that COMMITs before line 96
means the seed txn at 102 starts with **no `app.user_id`** → under future enforcement the seed is denied /
0-row, i.e. the fix **fails its own stated goal** ("so the subsequent menu seed … are admitted under
NOBYPASSRLS"). Alternatively, wrapping *everything* (75-129) in one outer `BEGIN` produces a **nested
`BEGIN`** at 102 (Postgres warns "already a transaction in progress", no-op) and the inner `COMMIT` at 121
**commits the outer txn early** / the inner `ROLLBACK` at 126 rolls back the onboarding_state write too —
an atomicity change. Today inert (BYPASSRLS) so no live break, but the C2 fix spec ignores this dual-txn
shape.

**`spa-proxy.ts` — early-return error paths have no ROLLBACK.** The sequence at **771-818** runs on a
checked-out client in autocommit today. Wrapping it in an explicit `BEGIN…COMMIT` without adding
`ROLLBACK` to every exit poisons the pooled connection: the `23505` slug-conflict catch at **782-783**
(`reply.sendError(409); return`) and the validation early-return at **774** leave an **open/aborted
transaction** when `finally` (line 819) calls `client.release()` — node-postgres does not auto-rollback on
release, so a poisoned backend returns to the operational pool. `spa-proxy.ts` is a 99.4th-percentile
churn hotspot with multiple such early returns. Today (autocommit) there is no txn to poison; the explicit-
txn conversion *introduces* the hazard.

**Violated invariant:** a correctness fix must preserve existing transaction/atomicity boundaries and
must not leave pooled connections in an aborted-txn state. The "explicit BEGIN…COMMIT" option is unsafe in
both handlers unless it subsumes the existing nested seed txn (onboarding) and rolls back on every early
return (spa-proxy) — neither is called out in the C2 fix. The safer "route through the canonical seam"
alternative avoids both, but the seam (`withTenantContext`, H2) is itself **DEFERRED** to Phase-1, so
Phase-0 is pushed onto the trap-laden explicit-txn path.

---

## [LOW] B-OPS · P0-4 boot-guard — safe on prod as designed; one implementation precondition

Confirmed **no mis-fire**: current guard (`packages/db/src/index.ts:32-38`) rejects only
`current_user==='postgres'`; prod's `deliveryos_api_user` passes unchanged. P0-4 adds a catalog check
`SELECT rolbypassrls FROM pg_roles WHERE rolname='dowiz_app_rls'` **gated on role existence**; since
`dowiz_app_rls` does not exist in Phase-0, it is a no-op → no false FATAL on correctly-configured prod.
Precondition to hold: the empty-result case (role absent) must be treated as **no-op**, not silently as
`rolbypassrls = false → pass` in a way that masks a future misconfig; and this runs inside
`pool.on('connect')` (per-connection) — an unhandled rejection here can crash boot (pre-existing pattern at
line 32-38), so the added query must be wrapped like the existing one. Advisory only; not a break.

---

## [LOW] B-SEC / B-OPS · P0-1 credential rotation is the one prod-mutating Phase-0 step — correctly decoupled, but it is a live connection cutover, not an RLS change

Rotating `deliveryos_api_user`'s password + repointing the secret is a coordinated prod redeploy; a
mis-sequenced cutover loses DB connectivity (this is exactly the H1 hazard). It is **not** an RLS
enforcement change and does not trip the flip. It is already scoped as standalone incident response with a
separately-recorded human-go, so this is confirmation, not a new finding: P0-1 is the only Phase-0 item
that can affect running prod, and its risk is connection-loss (recoverable by secret repoint), not
tenant-isolation. Note the *superuser* rotation has no runtime consumer → lowest risk.

---

## Regression sweep summary (r1 fixes vs new breaks)

| r1 fix | New break introduced? | Finding |
|---|---|---|
| C1 role model (membership grant) | design-only, dark, not in Phase-0 | none (deferred) |
| **C2 explicit-txn / is_local=true (P0-2)** | **yes — nested-txn collision (onboarding) + no-rollback pool poison (spa-proxy)** | MEDIUM |
| H1 two-role rotation (P0-1) | no (decoupled, human-gated) | LOW (confirm) |
| M1 boot-guard two-mode (P0-4) | no (gated on role existence) | LOW (precondition) |
| M5 keep in-memory limiter (P0-6) | no behavior change | none |
| **L2 close-on-failed-URL-verify (P0-3)** | **yes — kills dual-send `useWebSocket` in refresh race; wrong client model** | MEDIUM |
| W2-W6 guardrail locks (P0-5) | test/grep-gate only, zero runtime | none |

## Scope-leak sweep summary (does Phase-0 trip the flip / lockout / irreversible?)

| P0 item | Enforcement change? | Lockout risk? | Irreversible on prod? |
|---|---|---|---|
| P0-1 rotation | no | connection-loss only (H1), gated | password change (recoverable via repoint) |
| P0-2 C2 fix | no (inert under BYPASSRLS) | no | no |
| P0-3 WS | no | narrow WS-session drop (see MEDIUM) | no |
| P0-4 boot-guard | no (no-op until role exists) | no | no |
| P0-5 guardrails | no | no | no |
| P0-6 limiter | no | no | no |
| **deploy vehicle (release_command)** | no (policies inert under bypass) | **no RLS lockout, but migrate-time deploy-abort** | migration is forward-only |

**Bottom line:** the flip is truly deferred and no P0 *code* item enforces RLS. The single real
scope-leak is transitive — Phase-0's deploy runs the in-tree forward-only policy migration `077` ahead of
its deferred reconciliation prerequisite, which can hard-abort a prod deploy (HIGH). Fix the migration's
guard/hold or confirm prod's role+column state before treating Phase-0 as shippable to prod.
