# S10-PLATFORM-ADMIN / PROVISIONING Port — Council Packet · BREAKER FINDINGS

> **Seat:** system-breaker (adversarial). Axis: *where does it break*, not *is it nice*. Read-only
> verification of the S10 packet (`proposal.md` / `open-questions.md` / `threat-model.md`) against
> ground truth (`routes/admin/*`, `modules/acquisition/*`, `workers/backup/*`, `server.ts`,
> `lib/platform-admin.ts`, the RLS migrations, `rebuild/crates/api/src/auth/*`). No fixes proposed;
> the architect fixes. Every finding is file:line-grounded and demonstrable.
>
> **Verdict: PROCEED-WITH-REVISIONS.** The port carries most B4/P6 controls faithfully (verified below).
> But **one CRITICAL** (the packet's own Q1a headline over-claims that its proposed mechanism reproduces
> the fail-safe property — it demonstrably does not) and **one HIGH** (the Q4a B3-blocker rests on a
> factually wrong RLS premise, and its DoD test is a false-green) must be dispositioned before the 🔴
> questions are signed.

**Severity counts: CRITICAL 1 · HIGH 1 · MEDIUM 2 · LOW 1.**

---

## What the packet gets RIGHT (verified — do not re-litigate)

Confirmed against ground truth so the council spends its friction on the real gaps:

- **3-role `Claims` enum, no 4th variant.** `rebuild/crates/api/src/auth/claims.rs:152-155` — `Owner/Courier/Customer` only; `superadmin` token rejected (test `:392`). Only `OwnerClaims` has `user_id` (`:55`); `CourierClaims`/`CustomerClaims` carry `sub` but **no `user_id`** (`:89-135`) → the allowlist gate keyed on `userId` denies courier/customer principals by construction. **Owner→admin escalation via a role token is structurally closed.** (S10-T1 ✅)
- **`platform_admins` is SELECT-only to the app role.** `packages/db/migrations/1790000000080_grant-hardening.ts:14` — `REVOKE INSERT, UPDATE, DELETE, TRUNCATE ON platform_admins FROM dowiz_app`. Self-serve escalation via the operational role is structurally impossible. (Q-ADMIN-GRANT ✅)
- **Fail-closed gate wiring.** `lib/platform-admin.ts:33-54` — no userId→401, miss→403, DB-throw→503 (fail CLOSED). Verified verbatim. (Q-ADMIN-FAILCLOSED ✅)
- **Ops-secret gate is timing-safe + fail-closed-404.** `modules/acquisition/ops-auth.ts:17-29` — `crypto.timingSafeEqual` with length pre-check; `!secret → false → 404`. **No fail-open path** (unlike the telegram-webhook class the brief warned about). (Q-PROVISION-SECRET ✅)
- **Provisioning ordering + GUC dance intact.** `modules/acquisition/provisioning.ts:148-198` — `set_config('app.provision_token',…,true)` → `FOR UPDATE` grant → INSERT org/location/menu_versions → state-pinned `advance(ENRICHED→PROVISIONED)` → consume-LAST; ROLLBACK on any 0-row. No `RETURNING`. (Q-PROVISION-RLS ✅)
- **`erase_shadow_tenant` `owner_id IS NULL` guard + `CONTACT_REQUIRED` theft guard.** `provisioning.ts:235`, `claim.ts:113`. Both present verbatim. (Q-SHADOW-ERASE, Q-CLAIM-TRANSFER theft-guard ✅)
- **No restore-to-prod endpoint.** `apps/api/src/scripts/restore.ts` is a **CLI** (`process.argv`, `require.main === module`), not an HTTP route; the three admin backup routes are list/drill/dr-report only (`routes/admin/backups.ts:13,73,100`). Packet's Q3a claim confirmed. (S10-T5 ✅)
- **`resolveBackupKey` fails loud + never prints the key.** `workers/backup/encrypt.ts:44-68` — throws on unknown keyId; the error carries the *keyId name*, not the key value. (Q3c ✅)
- **`no-admin-prefix-register` eslint rule exists + enabled.** `tools/eslint-plugin-local/src/index.js:619`, `eslint.config.js:45` (`'error'`). (Note: the code comments in `server.ts:828` / `routes/admin/index.ts` call it `no-admin-register-outside-plane` — a stale name; the real rule is `no-admin-prefix-register`. Cosmetic.)

---

## CRITICAL

### C1 · B-SEC / B-OPS · Q1a axum plane-gate: the proposed mitigation does NOT reproduce B4's fail-safe property — it converts a fail-safe-by-construction gate into a fail-OPEN-by-construction one behind an advisory lint that a normal axum refactor defeats.

**Packet claim (attacked):** proposal §3.5 / Q1a option (a) / threat S10-T8 — "nest ALL `/api/admin` routes under ONE `Router` (`route_layer`) + a clippy/test tripwire + a re-proven sibling-closure test … reproduce the *property* (no admin route can escape the gate)" and (§3.5) it gates "children, siblings, AND future routes with zero detection."

**Ground truth of what actually gives Node "zero detection":** `lib/platform-admin.ts:76-83` — a **root-instance `onRequest` hook** that fires for EVERY request and gates on the **matched route pattern** (`request.routeOptions.url`, `:64`). This is fail-safe: *any* handler whose matched pattern is `/api/admin*` is gated **regardless of how or where it was registered** — inline `fastify.get('/api/admin/x')`, a nested prefix, a merged plugin. The packet itself admits (§3.5, Q1a) **axum has no post-routing matched-pattern hook** and this "is the one place the port cannot be a verbatim carry."

**Why the substitute is strictly weaker (demonstrable):**
1. The cited coverage-authority precedent, the eslint rule, is **FORM-SPECIFIC.** `tools/eslint-plugin-local/src/index.js:630-645` fires ONLY on a `.register()` CallExpression whose 2nd arg is an ObjectExpression with a **string-literal** `prefix` starting `/api/admin`. It does **not** see `fastify.get('/api/admin/x', …)`, a variable/computed prefix, or nested-prefix composition. In Node that blind spot is harmless — the runtime root-hook covers all of them. In Rust there is **no runtime root-hook to cover them.**
2. Ordinary axum composition produces an `/api/admin/*` route with **no `/api/admin` literal anywhere** for a clippy analogue to match: e.g. `Router::new().nest("/api", api)` where `api = Router::new().nest("/admin", gated).merge(other)` and `other` carries `.route("/admin/newthing", …)`. The resolved path is `/api/admin/newthing`; it is **outside** the gated sub-router (axum `route_layer`/`layer` apply only to routes defined on that router, not to merged/sibling routers); and the string `/api/admin` never appears. → **the handler ships ungated.**
3. The **sibling-closure test proves only the ONE throwaway route it declares is gated.** It cannot prove a *future, not-yet-written* route is gated — which is exactly the "future routes, zero detection" property the packet claims to reproduce. A static test is not a structural invariant.

**Break scenario:** post-flip, a future S10.x feature adds an admin endpoint via `.merge()`/`.nest()` outside the gated router (the natural way to add a route module in axum). No clippy lint keyed on a literal prefix flags it; the existing sibling-closure test still passes (it tests its own route). The endpoint is reachable **without the `platform_admins` allowlist check** → an unauthenticated or any-authenticated principal reads/acts across every tenant. At the highest-privilege tier this is a **cross-tenant breach**.

**Invariant violated:** TB-2 (unauthenticated → `/api/admin/*` boundary is *structural*, not per-route) + B4's "no admin route can escape the gate, with zero detection." The packet asserts equivalence; the mechanism it proposes is not equivalent — it removes the runtime fail-safe and backstops it with a lint that is provably blind to the escape and a test that cannot cover future routes. **This is not a live exploit in shipped code — it is a design-adequacy break in the packet's Q1a resolution:** the council must not accept "reproduce the property" as satisfied by router-nest + clippy + a single closure test. The property is *fail-safe future-route coverage*; the proposal delivers *fail-open future-route coverage + advisory detection*.

---

## HIGH

### H1 · B-DATA / B-CONSIST · Q4a is built on a factually wrong RLS premise: the cross-tenant admin reads do NOT "return 0 rows" post-B3 — one route is unaffected and the other returns FULL rows with silently-zeroed health data, and the packet's DoD test is a false-green.

**Packet claim (attacked):** proposal §6.2 / §8 table / §12 / Q-XTENANT-READ / open-questions Q4a / threat S10-T4 — "`fallback/health` + `r2-check` SELECT over ALL `locations` … they return **0 rows** unless run via an explicit platform-read mechanism," DoD = "prove `fallback/health` returns fleet rows, not 0, under NOBYPASSRLS before S10 flips," fix = "build the platform-read DEFINER fn/role **for `locations`**."

**Ground truth:** `locations` carries a permissive **`public_select ON locations FOR SELECT USING (true)`** — `packages/db/migrations/1780338909301_public-locations-rls.ts:7`. There is **no `AS RESTRICTIVE` policy anywhere** (grep: zero matches) and the NOBYPASSRLS phase migration (`1790000000077`) does **not** drop `public_select`. Postgres OR's permissive policies → under NOBYPASSRLS the operational role reads **every** `locations` row via `public_select`. Therefore:

- **`r2-check` (`routes/admin/fallback.ts:47-52`) reads ONLY `locations`** (`SELECT COUNT(*) … FROM locations`). Post-B3 it returns the **full fleet count**, correct coverage %. **It does NOT break and does NOT need a platform-read path.** The packet lists it (#218) as a 🔴 Q4a route that "returns 0 rows" — **wrong.**
- **`fallback/health` (`fallback.ts:13-24`) LEFT JOINs `owner_notification_targets`**, which DOES have FORCE tenant-isolation RLS (`1790000000077:99` `tenant_isolation … FOR ALL`; `1790000000080:24` `FORCE ROW LEVEL SECURITY`). Post-B3 the `locations` rows still come back (public_select), but every `ont.*` column is NULL → `telegram_active`, `push_active`, `dead_channels` are **silently 0 for every location.** It returns **full rows with wrong health data**, not 0 rows.

**Break scenario / why worse than the packet's framing:** the packet's DoD gate is "returns fleet rows, not 0." That test **passes without the fix** (rows arrive via `public_select`) → the council gets a green light to flip while `fallback/health` reports "every tenant: 0 active channels, 0 dead channels." During an incident (the exact moment the recovery read exists for), the operator sees a uniform, silent all-zero — a misleading all-clear/all-dead. A **loud** 0-rows failure is self-evident; a **silent** partial-wrong answer is not. And the prescribed prerequisite (a platform-read DEFINER **for `locations`**) is aimed at the wrong table — `locations` already reads fine; the real gap is `owner_notification_targets`.

**Invariant violated:** B3-blocker correctness (the packet's own Q4a premise) + the DoD verification must fail-when-wrong (Mandatory Proof Rule). The current DoD assertion is a tautology that survives the bug it is meant to catch. Severity HIGH (operational recovery correctness + a false-green cutover gate; not a breach — admin-only, gated).

---

## MEDIUM

### M1 · B-SEC / B-CONSIST · Plane D is mischaracterized as "no session" — but `/api/claim/accept` REQUIRES verifyAuth and derives the transfer recipient from the SESSION, not the request. A port built to the packet's summary tables risks an IDOR (tenant granted to an attacker-chosen account) or a broken transfer.

**Packet claim (attacked):** proposal §2 Plane D + §8 table row D — authority = "a **single-use opaque claim token (no session)**." (proposal §1 repeats "no session.")

**Ground truth:** `routes/public/claim.ts:17-23` — `/api/claim/accept` is registered with `preValidation:[verifyAuth]` and derives `const userId = request.user?.sub` (401 if absent). The recipient of the ownership transfer is taken from the **authenticated session**, NEVER the request body — this is precisely what prevents an IDOR in `acceptClaim → claim_transfer($token, $userId)` (`modules/acquisition/claim.ts:97-117`). Only `/claim/decline` and `/claim/request` are truly no-session.

**Break scenario:** a Rust port implemented to the §2/§8 "no session, token-only" description would have no session principal to transfer to, and the natural (wrong) fix is to accept a request-supplied `user_id` — a direct **IDOR: any holder of a claim token grants the whole tenant to an attacker-chosen account id.** The packet's own threat-model TB-4 correctly says "transfers to an *authenticated* owner," so the packet is internally inconsistent; the load-bearing detail is buried and contradicted by the scope tables the port will scaffold from.

**Secondary hazard (same route):** accept is `verifyAuth`-ONLY (no `requireRole(['owner'])`) and keys on `sub`, which is present on **all three** claim variants (`claims.rs` — Owner/Courier/Customer all have `sub`). So a **courier or customer token reaches `acceptClaim`**. Whether that is exploitable depends on `claim_transfer`'s internal contact-binding — which is exactly the P6 "ownership transfer needs work" residual (Q2b). The port must NOT assume `OwnerClaims` on the claim-accept plane; the packet's clean "4 planes / 4 mechanisms" table hides that Plane D's principal is *any* authenticated subject.

**Invariant violated:** TB-4 (org/location derived in-fn, recipient from session not request) — under-specified in the scope tables the port keys off.

### M2 · B-SEC · The audit trail keys the actor on `request.user.userId`, but `auditCtx` falls back to the literal string `'unknown'` — a destructive drill triggered on any code path where `userId` is absent writes an unattributable write-ahead row rather than failing closed.

**Ground truth:** `lib/platform-admin.ts:104` — `actorId: (request.user as {userId?}).userId ?? 'unknown'`. The gate (`:33`) already 401s a missing userId, so in the *current* wiring a drill never reaches `auditStart` without a real actor. **But the packet moves the drill trigger to a "thin Rust route → invoke the Node drill" carve-out (Q3b).** If the Rust trigger invokes the Node drill via a job-enqueue / internal call whose request context does not carry the original `request.user`, `auditCtx` silently stamps `'unknown'` and the write-ahead "who triggered this destructive drill" trail (A8, the *only* cross-tenant action record) is lost to a string constant — no throw, no failure.

**Break scenario:** post-carve-out, the DR-drill's write-ahead row reads `actor_id='unknown'` for every drill, defeating S10-T9 ("a destructive drill is visible … BEFORE it runs" *with an actor*). The packet's Q3b carve-out crosses a process boundary that the `?? 'unknown'` fallback silently tolerates.

**Invariant violated:** A8 / S10-T9 (write-ahead actor attribution) — the port's Node-carve-out boundary is the exact place `userId` can go missing, and the fallback fails *open* (writes a row) instead of *closed* (refuses the drill).

---

## LOW

### L1 · B-SCALE · Q2a cites the wrong control as the brute-force belt for the ops secret.

**Packet claim:** proposal §4.2 / Q2a — "a leaked/brute-forced secret is a mass-provisioning capability. **Belt: rate-limits (30/min mint/spine, 10/min extract) are carried verbatim.**"

**Ground truth:** those are **per-route** `config.rateLimit` (`modules/acquisition/route.ts:64,79,92,…`) applied AFTER routing — but the ops-secret gate is an `onRequest` hook that returns **404 before** any per-route config runs (`route.ts:56-60`). A wrong-secret guess never reaches the per-route limiter. The only control actually throttling secret-guessing is the **global** limiter `max:100/min` per client-IP (`server.ts:360-361,369`), which an attacker defeats by IP rotation. The real anti-brute-force property is the secret's entropy + the timing-safe compare — not the cited per-route limits.

**Impact:** low (128-bit secret entropy makes brute-force infeasible regardless), but the packet's reasoning attributes the mitigation to a control that provably does not fire on the attack path; the port should not carry the per-route limits *believing* they bound secret-guessing.

---

## Regression check vs prior council law

- B4 platform-admin authority, fail-closed wiring, write-ahead audit, GRANT posture: **carried faithfully** (verified above). No regression.
- P6 provision_shadow ordering/GUC, claim_transfer DEFINER, born-with-erasure, CONTACT_REQUIRED: **carried faithfully**. No regression.
- The genuinely NEW risks the packet names (S10-T8 gate mechanism, S10-T4 B3 read, S10-T10 Phase-D) are the ones with findings: **C1 sharpens S10-T8 (the mechanism is inadequate, not merely "re-prove it")**; **H1 corrects S10-T4 (the premise + DoD are wrong)**; Q5b/Phase-D is a counsel item (irreversibility), not a breaker mechanical break — deferred to counsel.

**packet-status remains 🟡 DRAFT.** C1 and H1 require architect disposition before the 🔴 Q1a / Q4a sign-offs; M1/M2 are port-scaffolding hazards to fold into the DoD.
