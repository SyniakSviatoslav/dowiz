# Resolution — dev-login-backdoor-hardening

**Role:** System Architect (DeliveryOS) — RESOLVE round
**Date:** 2026-06-22
**Inputs:** `proposal.md`, `docs/adr/0003-dev-login-fail-closed.md`, `breaker-findings.md`, `counsel-opinion.md`
**Outcome:** proposal.md + ADR-0003 revised to match the resolved design below. Two breaker CRITICALs
properly fixed (not hand-waved). All HIGH/MED resolved. Both counsel ETHICAL-STOPs + the open
question marked **NEEDS-HUMAN-DECISION** with exact cheap checks specified (not decided here).

I re-verified every load-bearing claim against live source before resolving (see "Verification log"
at the end). Both CRITICALs are real.

---

## Part A — Breaker findings

### [CRITICAL] B-SEC #1 — Option C (dev-kid) is non-implementable as written → **FIX (design rewritten)**

**Verified true.** `packages/platform/src/auth/jwt.ts:28,37`:
```ts
const k = getKid();                              // = env.JWT_KID, always
new SignJWT(jwtPayload).setProtectedHeader({ alg: 'RS256', kid: k })   // header kid HARDCODED to env kid
```
The `payload.kid` override (`jwt.ts:33`) lands only in the **body claim**, never the protected
header. `verifyAuthToken` (`jwt.ts:55`) checks `protectedHeader.kid !== getKid()`. CI sets
`JWT_KID=1` (`ci.yml:103`) — the same value as prod. So a "dev-kid" token minted by a non-prod app
still carries `header.kid = its env JWT_KID`, and a leaked prod `kid:1` token verifies fine on prod.
C as written invalidates **zero** leaked tokens. The proposal's "reuses existing verifier check, no
new sign logic" was false.

**Resolution — make C concretely implementable, with the exact signer + verifier + key-distribution
spec.** C is retained because the cryptographic prod-rejection property is worth having, but it now
carries the real change set:

**C.1 — Signer change (`signAuthToken`).** Stamp the override kid into the *protected header*, not
just the body:
```ts
// resolved shape (design intent — implementer writes the code)
export async function signAuthToken(payload: ... & { kid?: string }, expiresIn) {
  const headerKid = payload.kid || getKid();          // was: hardcoded getKid()
  const jwtPayload = { ...payload, sub, kid: headerKid };
  new SignJWT(jwtPayload).setProtectedHeader({ alg: 'RS256', kid: headerKid })  // header now honors override
}
```
The body-claim `kid` is kept consistent with the header (no divergence). All non-dev callers pass no
`kid`, so they keep stamping `getKid()` — **zero behavior change for prod tokens**. This addresses
the MED B-CONSIST singleton point too: the kid is now a per-token argument at the signer boundary,
not the process-global `getKid()`, for the dev path only; the verifier still reads the single
`getKid()` because each process verifies against exactly one expected kid (the env it runs in).

**C.2 — Verifier change (`verifyAuthToken`) — accept dev-kid ONLY in non-prod, reject in prod.** The
verifier must accept *either* the env kid *or* a dev kid, and the dev kid is accepted only when the
process is non-prod:
```ts
// resolved shape
const acceptDevKid = env.NODE_ENV !== 'production' && !!env.JWT_DEV_KID;
const ok = protectedHeader.kid === getKid()
        || (acceptDevKid && protectedHeader.kid === env.JWT_DEV_KID);
if (!ok) throw new Error('Invalid Key ID');
```
Crucial property: **prod's verifier never sets `acceptDevKid`** (`NODE_ENV==='production'` short-
circuits it), so a dev-kid token is rejected on prod regardless of what `JWT_DEV_KID` is. This is
the only place C now leans on NODE_ENV, and a *wrong* NODE_ENV here fails **safe**: if prod's
NODE_ENV were wrongly non-production, the verifier would accept dev-kid tokens — but the dev keypair
is not on prod (C.3), so no attacker can mint a dev-kid token prod's public key validates. The
prod-rejection therefore rests on **key material isolation** (the dev keypair is absent from prod),
with the NODE_ENV check as a second lock, not the only one.

**C.3 — Key distribution without breaking the `x-dev-auth-secret` protocol.** New optional env on
**non-prod only**: `JWT_DEV_KID` (string) + `JWT_DEV_PRIVATE_KEY` / `JWT_DEV_PUBLIC_KEY` (the dev
keypair). Dev-mint sites pass `kid: env.JWT_DEV_KID` and sign with the dev private key; the dev
verifier accepts the dev public key. Distribution:
- **staging:** `fly secrets set` the dev kid + dev keypair on `dowiz-staging`.
- **CI:** extend the existing throwaway-keypair generation (`ci.yml:112-118`) to also emit a second
  RSA keypair into `.env.ci-fresh` as `JWT_DEV_*`, and add `JWT_DEV_KID=dev`. Pure additive lines.
- **prod:** these envs are **absent**. Prod keeps only `JWT_KID` + `JWT_*_KEY`.
- The **`x-dev-auth-secret` header protocol is untouched** — C changes only what *kid* the dev mint
  stamps and which keypair signs it; the request authorization header that the 14 specs send is the
  same. No spec change is required for C.

**C — residual:** because the dev verifier (C.2) must hold *both* keys to validate dev-kid tokens,
staging/CI verify both the env keypair and the dev keypair. Accepted — both are non-prod; the
boundary that matters is prod, which holds neither the dev keypair nor `acceptDevKid`.

**C — alternative considered and rejected (kept for the record):** *Drop C entirely; rely on
operator rotation of kid:1 + short dev-token TTL.* This was the breaker's offered fallback. Rejected
as the *sole* steady-state mechanism because it makes every future leaked dev token live for its
full TTL until a human rotates — i.e. it re-creates the exact "out-of-band human action is the only
kill" gap the design exists to remove. **However**, because C does NOT retroactively help the
*already-minted* `kid:1` leaked token (it has the prod header kid — see B-SEC #5 below), operator
rotation remains mandatory for the *current* leak. So the resolved position is: **C for steady state
(future dev tokens are prod-rejected by construction once C.1–C.3 ship), operator kid rotation for
the one already-leaked token.** Both, not either.

---

### [CRITICAL] B-SEC #2 — Gate scope is wrong: ≥6 mint sites, the two `/dev/mock-auth` minters never call `devLoginAllowed` → **FIX (single hardened gate over ALL mint sites)**

**Verified true.** `grep signAuthToken apps/api/src` — full inventory of dev/test-identity mint
sites and their *current* gate:

| # | Site | Mints | Current gate | Real production? |
|---|---|---|---|---|
| 1 | `server.ts:888` inline `/api/auth/local/login` | owner, 1d | `devLoginAllowed(secret)` = `!!secret` | path NOT in `/dev/`, so NOT covered by `isDevRequestAuthorized` — gate is `!!secret` only |
| 2 | `server.ts:650/664/711` inline `/api/dev/mock-auth` | courier, fresh-owner (no membership), owner+membership, all 1d | **none** — only `isDevRequestAuthorized` (path `/api/dev/`, `!!secret`) | mints + **writes users/memberships** |
| 3 | `routes/dev/mock-auth.ts:16/72` `/dev/mock-auth` | courier, owner, 1d | **none** — only `isDevRequestAuthorized` (`!!secret`) | duplicate of #2; **writes users/memberships** |
| 4 | `routes/auth/local.ts:108` `/auth/local/login` (plugin) | role-resolved, 15m + refresh row | `devLoginAllowed(secret)` for the *bypass branch only* | latent (404s today) but a 2nd cred pair ships |
| 5 | `routes/dev/mock-auth.ts:138/158/77` seed/repair/assignment helpers | (no JWT) writes targets/memberships/assignments | `isDevRequestAuthorized` (`!!secret`) | data-mutating dev helpers |
| 6 | `plugins/auth.ts:66` mock-auth *honor* | accepts a courier token with no `jti` | `devLoginAllowed(secret)` | accepts forged-shape courier tokens when gate open |

(Real production mint sites — `routes/auth.ts` Google/refresh, `routes/courier/auth.ts` — are NOT
dev paths and are out of scope. Inventory complete.)

The proposal's "one change in `devLoginAllowed` covers all 3 call sites" was wrong on two counts:
there are **6 sites**, and sites #2/#3/#5 don't call `devLoginAllowed` at all — they ride the
path-based `isDevRequestAuthorized`, which is the *same condemned `!!secret` check*
(`dev-guard.ts:47`). After B ships, prod with a leaked secret still serves
`POST /api/dev/mock-auth {role:'owner'}` → real `kid:1` owner token + a written `memberships` row.

**Resolution — route EVERY dev path AND every dev-mint through ONE hardened gate that includes the
flag, not merely the secret.** Two coordinated changes:

**Gate change G.1 — harden `devLoginAllowed` (covers sites #1, #4, #6):**
```ts
export function devLoginAllowed(env): boolean {
  return env.ALLOW_DEV_LOGIN === 'true' && !!env.DEV_AUTH_SECRET;
}
```
Signature changes from `(secret)` to `(env)` so it reads both the flag and the secret — one source
of truth. (B from the original proposal, made concrete.)

**Gate change G.2 — harden `isDevRequestAuthorized` (covers sites #2, #3, #5 — and any future
`/dev/*` route) by folding the SAME flag into the path guard:**
```ts
export function isDevRequestAuthorized(url, providedSecret, env): boolean {
  if (!isDevPath(url)) return true;
  if (env.ALLOW_DEV_LOGIN !== 'true') return false;     // NEW — flag-gated, not just secret
  if (!env.DEV_AUTH_SECRET) return false;
  return secretMatches(providedSecret, env.DEV_AUTH_SECRET);
}
```
This is the load-bearing fix the original design missed: the `/dev/*` family is gated by
`isDevRequestAuthorized`, **not** by `devLoginAllowed`, so hardening only the latter left the
mock-auth minters open. By adding the *same `ALLOW_DEV_LOGIN` flag* to the path guard, **all six
sites are now fail-closed under one flag**: with the flag absent/false (prod default), both the
`/auth/local/login` family (G.1) and the entire `/dev/*` family (G.2) reject — the `/dev/*` routes
return 404 (their existing fail-closed behavior), the login family returns 401.

Net: the design no longer claims "one function covers all." It claims **two guard functions, one
shared flag, six sites covered** — and enumerates each site (table above) with its covering guard.
The `dev-guard.ts` module comment ("a single guard keyed on the path closes all of them — present
and future — in one place") stays true and now actually holds, because the flag lives inside that
same path guard.

---

### [HIGH] B-OPS #3 — D (boot fail-fast) is silent on staging/CI (no NODE_ENV there) → **FIX (NODE_ENV matrix specified; D scoped to fire in prod, gate stays testable elsewhere)**

**Verified true.** Zero `NODE_ENV` in any Dockerfile / `fly.toml` / `ci.yml`. CI injects it in the
*runner script*: `scripts/verify-fresh-provision.sh:79  export ... NODE_ENV=development`. So CI boots
as `development` → D never evaluates the prod branch in CI. D's prod branch only ever runs on prod,
the one box already compromised.

**Resolution — make NODE_ENV an explicit, owned, per-environment contract, and decouple D's
*detector* from the *gate* it backstops:**

**The NODE_ENV matrix (who sets it, what value):**
| Env | Required NODE_ENV | Set by | Why |
|---|---|---|---|
| **prod** (`dowiz`) | `production` | `fly secrets set NODE_ENV=production` on `dowiz` (operator) — *and* mirror it as `ENV NODE_ENV=production` in the **prod runtime build stage** so it cannot be silently dropped (see below) | D must fire here; verifier must NOT accept dev-kid here |
| **staging** (`dowiz-staging`) | `development` | `fly secrets set NODE_ENV=development` on `dowiz-staging` (operator) | dev-login must stay ON; D must NOT fire; dev-kid verify must work |
| **CI fresh-provision** | `development` | already `scripts/verify-fresh-provision.sh:79` (unchanged) | dev-login ON; D off; dev-kid verify on |
| **CI post-deploy E2E** | N/A (runs against deployed prod) | — | tests hit `dowiz.fly.dev`, which is `production` |
| **local dev** | `development` | `.env` (`.env.example:1`) | dev-login ON |

**The decoupling that makes D both fire-in-prod AND testable elsewhere:** the original D triggered on
`NODE_ENV==='production' AND (flag OR secret)`. The flaw: NODE_ENV is the *only* selector, and it is
invisible/out-of-band. Two changes:

1. **Pin prod NODE_ENV in the build, not only in Fly secrets.** Add `ENV NODE_ENV=production` to the
   **production runtime stage** of the Dockerfile (the stage prod ships). This removes the "invisible
   Fly secret is the sole source" hazard the breaker named: prod's NODE_ENV is now in-repo and
   greppable, and a dropped Fly secret can't silently flip prod to development. Staging/CI build the
   *same* image but **override** NODE_ENV at runtime via Fly secret / runner export (`development`) —
   override of an env var is normal and supported. This directly answers the §2 "NODE_ENV is an
   invisible knob" finding: it stops being invisible.
   - *Trade named (R-1):* the same image now defaults to `production` if no override is set. Staging
     and CI **must** set `NODE_ENV=development` (they already do for CI; staging gets an explicit Fly
     secret). If staging forgets, it boots as production → D fires → staging refuses to boot with the
     dev flag set → **loud break, not silent prod exploit.** Correct fail direction.

2. **D's invariant fires on the resolved NODE_ENV**, now reliable in prod (pinned in the image) and
   correctly inert in dev/CI:
   ```
   loadEnv() throws when NODE_ENV === 'production'
     AND ( ALLOW_DEV_LOGIN === 'true' OR DEV_AUTH_SECRET is set OR JWT_DEV_KID is set )
   ```
   (Added `JWT_DEV_KID` to the dangerous set — a dev keypair on prod is itself a misconfig.)

**Testability of D's prod path pre-prod (answers MED B-OPS #8):** D is a pure function of env. The
implementer adds a **unit test** that calls the boot-guard logic with `NODE_ENV='production'` +
each dangerous combo and asserts it throws — no need to actually boot a staging box as production.
This rehearses D's prod behavior in CI on every push, closing "D's prod path is untestable pre-prod."
So D fires in prod (real boot) and is *proven* everywhere (unit test), without breaking staging.

---

### [HIGH] B-FAIL #4 — 5/min rate-limit on `/api/auth/local/login` flakes the 14-spec CI from one runner IP → **FIX (flag-aware exemption + account+IP key, justified)**

**Verified plausible.** Global limiter is `max:100, timeWindow:'1 minute'` keyed by IP
(`server.ts:483`). CI runs ~14 specs from one GitHub Actions egress IP with 2–4 parallel Playwright
workers → easily >5 login POSTs/min from one IP. A blanket 5/min returns 429 and flakes the suite.

**Resolution — the rate-limit must harden the *prod* threat (online password guessing) without
touching the *non-prod* test path, because in non-prod the hardcoded creds are gone (item 4) and the
path is secret-gated, so brute-force is moot there.** Chosen design:

- **Exempt-when-gate-open.** When `devLoginAllowed(env)` is true (non-prod only — flag + secret),
  the login route applies the **global** 100/min limiter only (no extra per-route cap). When the
  gate is closed (prod), the route returns 401 immediately *before any limiter matters* — there is
  no real dev-login path to brute-force in prod because the branch is dead and the creds are gone.
- **For the *real* `local.ts` argon2 login path** (out of scope to rewrite, but it shares the route
  family): key the limiter by **`email + IP`** (not IP alone) at a modest cap (e.g. 10/min per
  email+IP). This hardens online guessing against a *single account* without one shared CI IP
  starving all logins, and without letting an attacker rotating emails from one IP escape (the
  global 100/min IP cap still bounds them).

**Justification vs the breaker's three options:** "higher limit" alone still risks flake at high
parallelism and weakens prod; "key by account+IP" is right for the real argon2 path; "exempt-when-
gate-open" is right for the dev bypass path because in prod the gate is closed so there is nothing to
rate-limit, and in non-prod there is nothing worth rate-limiting (no real secret to guess — the dev
secret is shared test material). Combining both is the smallest change that hardens prod and never
429s CI. The Zod body schema (item 4) is added unconditionally — it has no rate impact.

---

### [HIGH] B-SEC #5 — Leaked `kid:1` owner token (1d TTL) is NOT invalidated by this design → **ACCEPT-RISK (operator rotation is the sole kill for the *already-minted* token; design kills all *future* ones)**

**Verified true and now stated without euphemism.** Because the leaked token carries the prod header
kid (`kid:1`), C — even after the C.1–C.3 fix — does NOT reject it: it IS a valid prod-kid token.
C's prod-rejection property applies to **future dev-kid tokens**, not to the one already minted under
the prod kid before C ships.

**Explicit disposition (replaces the prior "C handles steady state, rotation is belt-and-suspenders"
framing, which the breaker correctly called false):**
- **Already-minted leaked `kid:1` owner token:** invalidated **only** by operator rotation of the
  prod JWT signing key/kid (R-6). There is no in-design mechanism that kills it (no auth session
  table to revoke against — confirmed: couriers bind to a session `jti`, but owner tokens do not).
  Until rotation, it lives up to its 24h exp. **Owner: Operator. Disposition: accept-risk, mitigated
  by mandatory rotation in the immediate-mitigation runbook (not optional).**
- **All future dev/mock tokens:** prod-rejected by construction once C.1–C.3 ship (dev kid + prod
  verifier refuses dev kid), AND un-mintable on prod once G.1+G.2 ship (flag default-off closes all
  six sites). So the *steady state* needs no operator action.

The design no longer claims to invalidate the existing token. The honest statement: **rotation is
mandatory for the live leak; the design prevents recurrence.** Both required.

---

### [MED] B-CONSIST #6 — `getEnv()`/`getKid()` singleton: dev signer + prod verifier share one cached kid → **FIX (folded into C.1)**

Resolved by C.1: the kid becomes an explicit per-token argument at the signer (`headerKid =
payload.kid || getKid()`), so the dev mint no longer relies on the process-global `getKid()`. The
verifier legitimately stays single-kid-per-process plus the explicit dev-kid acceptance (C.2) —
because a process only ever verifies against the one env it runs in. No cross-process drift: web and
worker both `loadEnv()` the same values. **Disposition: fixed in C.1/C.2.**

---

### [MED] B-ANTIPATTERN #7 — `empty@dowiz.com` second hardcoded cred pair → **FIX (DELETE both the duplicate handler and the cred)**

**Verified.** `empty@dowiz.com / empty123456` exists only in `routes/auth/local.ts:43-45`, the
latent plugin duplicate. The original "align or delete" left it ambiguous.

**Resolution — DELETE, do not align.** Decision: collapse to exactly one dev-bypass surface.
- The inline `server.ts:868` `/api/auth/local/login` is the live path; it keeps the single
  `test@dowiz.com` bypass (now flag-gated via G.1) **with the literal moved out of shipped code** —
  the test email/password come from env (`DEV_LOGIN_EMAIL` / handled via the existing test fixture),
  not a code literal. (Counsel honesty point: the comforting-but-false comments at `server.ts:871-873`
  and `local.ts:42` are **deleted**, not softened.)
- The `routes/auth/local.ts` plugin: its **dev-bypass branch (lines 41-46, both cred pairs)
  is deleted**. The real argon2 password-verify path in that file stays (it is the genuine
  production login path counsel noted is worth keeping for E2E coverage). So `local.ts` becomes a
  *pure real-login* handler with no hardcoded creds; the `empty@` pair ceases to exist anywhere.

This satisfies "exactly one dev-bypass path" (the inline handler) and removes the second cred class
entirely. **Disposition: fixed — delete.** (R-5 spec audit still required before deleting: confirm
no spec asserts against `empty@` or against the plugin's bypass branch.)

---

### [MED] B-OPS #8 — NODE_ENV enum has no `staging`; D's prod path untestable pre-prod → **FIX (folded into #3)**

Resolved by #3: staging runs `NODE_ENV=development` (intended — staging is config-equivalent to a
dev box on purpose, that is what keeps dev-login on), and D's prod-firing behavior is proven by a
**unit test** in CI, not by booting staging as production. We do **not** add a `staging` enum value —
that would require threading it through D and every NODE_ENV check (the R-4 hazard). The enum stays
`["development","test","production"]`. **Disposition: fixed via unit-test rehearsal; enum unchanged.**

---

### [LOW] B-SEC #9 — no rate-limit on the `/api/dev/mock-auth` minters → **ACCEPT-RISK**

After G.2, the mock-auth minters are flag-gated (prod fail-closed). In non-prod they are
secret-gated test endpoints with no real credential to guess. A rate-limit adds nothing in non-prod
and the path is dead in prod. **Disposition: accept-risk. Owner: Architect.** (Re-evaluate only if a
mock-auth path is ever exposed without the flag.)

### [LOW] B-ANTIPATTERN #10 — dev-keypair provisioning is overhead until C.1 lands → **FIX (sequencing)**

Resolved by sequencing: the implementation order is **C.1 signer change → C.2 verifier change → C.3
provisioning**, committed together. The proof obligation "a `dev`-kid token is rejected by a prod-kid
verifier" must pass before C is declared done, so C cannot be "believed live but isn't." **Disposition:
fixed via mandated sequencing + proof gate.**

### Regression note (from breaker summary) — kid switch rejects in-flight tokens
Rotating prod `kid:1 → kid:2` (R-6) rejects all in-flight prod tokens (including legit owner
sessions) at rotation time → users re-auth. This is the intended kill of the leak; called out in the
runbook so it is not a surprise. The dev-kid introduction (C) does NOT cause this — dev tokens only
exist in non-prod. **Disposition: documented in §9 operability.**

---

## Part B — Counsel ETHICAL-STOPs and the open question (NEEDS-HUMAN-DECISION)

Per mandate, I do **not** resolve these. I mark each NEEDS-HUMAN-DECISION and specify the exact cheap
checks, verified to exist (or verified absent) in this codebase.

### ETHICAL-STOP-1 — Forensic determination before "closed" → **NEEDS-HUMAN-DECISION**
The design closes the hole and (with R-6) kills the token, but does not establish whether the
backdoor was *used*. The self-escalation chain **writes durable rows that survive JWT key rotation**.
Operator must run the forensic pass (or consciously record accepting the residual uncertainty given
thin logs). Exact cheap checks (all `SELECT count(*)`-class, verified against live schema):

> **CRITICAL EXECUTION PRE-REQ (R2 counsel RE-EXAMINE #1) — run as BYPASSRLS/superuser, or every
> count lies clean.** `organizations`, `locations`, `memberships`, `customers`, `orders` are all
> `ENABLE` + **`FORCE ROW LEVEL SECURITY`** with `tenant_isolation` policies keyed on `app.user_id` /
> `app_member_location_ids()`. A forensic query run as the **normal app role with `app.user_id` unset**
> returns **ZERO rows BY POLICY**, not by fact — a real breach reads as clean. **These queries MUST run
> as a `BYPASSRLS`/superuser role** (e.g. the Supabase `postgres` superuser, or a role created
> `WITH BYPASSRLS`), NOT as the app role. Verify with `SELECT current_user, current_setting('is_superuser');`
> before trusting any count. Do NOT `SET ROLE` to the app role. This is the difference between a
> trustworthy close and false reassurance.

1. **Orgs/locations/memberships with implausible provenance** — the single most important check
   (the escalation writes these and they outlive rotation). **Run as BYPASSRLS/superuser (above):**
   ```sql
   -- accounts that should never have created tenants
   SELECT u.id, u.email, count(DISTINCT o.id) orgs, count(DISTINCT l.id) locs, count(m.*) memberships
   FROM users u
   LEFT JOIN organizations o ON o.owner_id = u.id
   LEFT JOIN locations l ON l.org_id = o.id
   LEFT JOIN memberships m ON m.user_id = u.id
   WHERE u.email IN ('test@dowiz.com','dev@deliveryos.com','empty@dowiz.com')
      OR u.email LIKE 'fresh-%@e2e.dowiz'      -- the fresh-owner mock mints these
   GROUP BY u.id, u.email;
   -- AND: orgs/locations created in the exposure window with no plausible human owner
   SELECT id, owner_id, created_at FROM organizations WHERE created_at >= '<window-start>' ORDER BY created_at;
   ```
2. **`auth_refresh_tokens` tied to the test/dev users** — the *inline* handler writes none (thinnest
   trace), but the `local.ts` bypass reuses the refresh insert, so a row here is a positive signal
   that the plugin path was hit:
   ```sql
   SELECT user_id, created_at FROM auth_refresh_tokens
   WHERE user_id IN (SELECT id FROM users WHERE email IN ('test@dowiz.com','empty@dowiz.com'));
   ```
3. **No auth/login audit table exists** (verified: settlement/backup/courier have audit tables; auth
   does not). So *absence of evidence is not evidence of absence* here — the human must decide knowing
   logs are thin. (This is itself a finding; a follow-up to add a minimal auth audit log is noted as
   non-blocking advice, not part of this remediation.)

4. **ORDERING — forensics BEFORE any user-row deletion (R2 counsel RE-EXAMINE #4).** `memberships`
   and `auth_refresh_tokens` both **`ON DELETE CASCADE` from `users`**. The remediation's "delete
   `empty@dowiz.com`" means **delete the code literal ONLY — NEVER the prod `users` row.** Deleting the
   user row would cascade-destroy exactly the `memberships` / `auth_refresh_tokens` rows STOP-1 must
   inspect — silent evidence erasure. **Mandatory sequence: run all STOP-1 forensic queries above and
   record results FIRST; only then (if ever) consider any prod user-row cleanup.** A literal-minded
   implementer running "delete empty@" against prod must be stopped by this note. The hardening itself
   touches no prod data rows — it removes a code literal, not a database row.

**Owner: Operator (incident). Architect supplies the queries above; human runs them as a
BYPASSRLS/superuser role and records the result before declaring "closed." No user-row deletion before
forensics.**

### ETHICAL-STOP-2 — Disclosure obligation → **NEEDS-HUMAN-DECISION**
Whether a breach-disclosure duty attaches depends on STOP-1's findings AND on whether real
customer/owner PII was within blast radius during the live window. This org has a `/compliance` SoT +
RoPA + privacy gate — a data-steward posture. The human must, based on STOP-1, record a conscious
decision (notify / don't-notify-and-why) in `/compliance`. **Owner: Operator / data-steward (human).
Not decided here.**

### Open question — "was prod dark / any real users + orders in the live window?" → **NEEDS-HUMAN-DECISION**
This is the single fact that collapses most uncertainty (CRITICAL-defect with zero victims vs. actual
breach). **Run as BYPASSRLS/superuser** (same RLS-FORCE caveat as STOP-1 — `users`, `orders`,
`customers` are tenant-isolated; the app role returns zero by policy). Corrected queries
(R2 counsel RE-EXAMINE #2 + #3 — verified column names against
`packages/db/migrations/1780310074262_orders.ts`):
```sql
-- real (non-test, non-seed) users
SELECT count(*) FROM users
WHERE email NOT IN ('test@dowiz.com','dev@deliveryos.com','empty@dowiz.com')
  AND email NOT LIKE 'fresh-%@e2e.dowiz' AND email NOT LIKE '%@e2e.dowiz';

-- real PAID orders in the exposure window (R2 #3: filter on payment outcome/status, NOT created_at
-- alone — an abandoned/PENDING cart is not a victim-bearing paid order and would inflate severity).
-- GROUNDED enum values (verified in 1780310044710_extensions-and-enums.ts:14,16 — note the exact
-- casing/spelling; there is NO 'paid' value, and order_status is UPPERCASE):
--   order_status     = PENDING|CONFIRMED|PREPARING|READY|IN_DELIVERY|DELIVERED|REJECTED|CANCELLED|SCHEDULED|PICKED_UP
--   payment_outcome  = pending|paid_full|paid_partial|refused_payment|refused_goods|customer_cancelled_on_door
SELECT count(*), min(created_at), max(created_at)
FROM orders
WHERE created_at >= '<window-start>'
  AND status NOT IN ('PENDING','CANCELLED','REJECTED','SCHEDULED')   -- exclude unconfirmed/abandoned
  AND payment_outcome IN ('paid_full','paid_partial');              -- settled payment only (real enum values)

-- distinct real customers who placed paid orders (R2 #2: orders has NO customer_phone — it has
-- customer_id uuid REFERENCES customers(id) [1780310074262_orders.ts:24]; phone is customers.phone
-- [:11]). Join, don't guess.
SELECT count(DISTINCT c.phone)
FROM orders o
JOIN customers c ON c.id = o.customer_id
WHERE o.created_at >= '<window-start>'
  AND o.status NOT IN ('PENDING','CANCELLED','REJECTED','SCHEDULED')
  AND o.payment_outcome IN ('paid_full','paid_partial');
-- (count(DISTINCT o.customer_id) is an equivalent phone-free alternative if preferred.)
```
The operator must define `<window-start>` = when the prod `DEV_AUTH_SECRET` was first set. Enum
literals above are the **actual** `order_status`/`payment_outcome` values (verified against the live
migrations, not guessed). **Owner: Operator. Architect supplies queries; human runs as BYPASSRLS,
interprets, and records.**

---

## Verification log (re-checked against live source this round)
- `packages/platform/src/auth/jwt.ts:28,33,37,55` — signer hardcodes header kid; override only in body claim; verifier checks header kid. **CRITICAL #1 confirmed.**
- `apps/api/src/plugins/dev-guard.ts:19-21,41-49` — `devLoginAllowed = !!secret`; `isDevRequestAuthorized` gate is `!!configuredSecret`. **CRITICAL #2 confirmed.**
- `apps/api/src/server.ts:519-523` (`/dev/*` → `isDevRequestAuthorized` only), `:641-714` (inline mock-auth mints + writes), `:868-892` (inline local-login, hardcoded cred, no schema, no per-route limit). Confirmed.
- `apps/api/src/routes/dev/mock-auth.ts:8-74` duplicate mock-auth minter; `:138/158/77` data-mutating helpers. Confirmed.
- `apps/api/src/routes/auth/local.ts:42-46,108` — `empty@dowiz.com` pair + false "always rejects" comment; 15m token + refresh insert. Confirmed.
- `apps/api/src/plugins/auth.ts:62-69` — courier no-jti honor gated on `devLoginAllowed`. Confirmed.
- `packages/config/src/index.ts:4` — NODE_ENV enum required, no `staging`, no default; `DEV_AUTH_SECRET` optional. Confirmed.
- `.github/workflows/ci.yml:103,110` — `JWT_KID=1` (= prod kid), `DEV_AUTH_SECRET=ci-fresh-secret`, **no `NODE_ENV`**. `scripts/verify-fresh-provision.sh:79` injects `NODE_ENV=development`. **HIGH #3 confirmed.**
- No `NODE_ENV` in any Dockerfile / fly.toml / workflow (grep, zero hits). Confirmed.
- `server.ts:483-486` — global rate-limit `max:100/min`, default IP key. **HIGH #4 confirmed.**

---

## Disposition summary

| Finding | Severity | Disposition | Owner |
|---|---|---|---|
| #1 Option C non-implementable (signer hardcodes header kid) | CRITICAL | **FIX** — C.1 signer header-kid override + C.2 verifier accepts dev-kid only in non-prod + C.3 dev keypair distribution | Implementer / Operator (keys) |
| #2 Gate scope wrong; ≥6 mint sites, mock-auth ungated | CRITICAL | **FIX** — harden BOTH `devLoginAllowed` (G.1) AND `isDevRequestAuthorized` (G.2) with shared `ALLOW_DEV_LOGIN`; six sites enumerated | Implementer |
| #3 D silent on staging/CI (no NODE_ENV) | HIGH | **FIX** — NODE_ENV matrix; pin prod NODE_ENV in Dockerfile prod stage; D proven via unit test | Operator + Implementer |
| #4 5/min rate-limit flakes CI | HIGH | **FIX** — exempt-when-gate-open for dev bypass + email+IP key (10/min) for real argon2 path | Implementer |
| #5 Leaked kid:1 token not invalidated by design | HIGH | **ACCEPT-RISK** — mandatory operator kid rotation (R-6); design prevents recurrence, not the existing token | Operator |
| #6 Singleton kid sign/verify | MED | **FIX** — folded into C.1 (per-token kid arg) | Implementer |
| #7 `empty@dowiz.com` 2nd cred pair | MED | **FIX** — DELETE the plugin bypass branch + both creds; one dev-bypass path | Implementer |
| #8 NODE_ENV enum lacks `staging`; D untestable pre-prod | MED | **FIX** — folded into #3 (unit-test rehearsal; enum unchanged) | Implementer |
| #9 No rate-limit on mock-auth minters | LOW | **ACCEPT-RISK** — flag-gated + secret-gated, prod-dead | Architect |
| #10 Dev keypair overhead before C.1 | LOW | **FIX** — mandated sequencing C.1→C.2→C.3 + proof gate | Implementer |
| STOP-1 Forensics before "closed" | — | **NEEDS-HUMAN-DECISION** — queries specified (R2-corrected: BYPASSRLS + ordering) | Operator |
| STOP-2 Disclosure obligation | — | **NEEDS-HUMAN-DECISION** — depends on STOP-1; record in /compliance | Operator / data-steward |
| Open Q: prod dark / real users+orders in window | — | **NEEDS-HUMAN-DECISION** — count query specified (R2-corrected) | Operator |

---

# RESOLVE round 2 (2026-06-22) — re-attack NEW CRITICAL + NEW HIGH + remaining findings + counsel RE-EXAMINE

The re-attack confirmed both prior CRITICALs are **closed in design** but opened a NEW CRITICAL and a
NEW HIGH from the C/D rewrite, plus three MED/LOW residuals; counsel RE-EXAMINE flagged the forensic
queries as not-yet-trustworthy. I re-verified every R2 claim against live source before resolving
(see R2 verification log below). All findings confirmed true.

## R2-1 — [CRITICAL B-OPS] Closing `/dev/*` on prod breaks the prod-deploy CI gate → **FIX (deploy-pipeline redesign, IN SCOPE)**

**Verified true, and broader than reported — FOUR prod E2E steps, not two.** `ci.yml:158-184` (deploy
job, push-to-main, deploys prod `dowiz`) runs against `https://dowiz.fly.dev`:
`deploy-validation.spec.ts:13` (`POST /api/dev/mock-auth`), `flow-core-lifecycles.spec.ts:31,351`
(`mock-auth` + `create-assignment`), **plus** `telegram-webhook.spec.ts` and
`telegram-full-flow.spec.ts` (ci.yml:172-184) which also rely on mock-auth setup. Secret injected per
step (`ci.yml:162,169,176,183`), header by `e2e/lifecycle-e2e/playwright.config.ts:15`. So the prod
backdoor is **load-bearing for deploy validation** — G.2 / unsetting the prod secret turns all four
red.

**This forces a scope change, stated honestly: the deploy-pipeline redesign is now PART of this
remediation, not a follow-up.** Resolution (full detail in proposal §9.A; ADR decision #4):
- **Options weighed:** (a) prod smoke = unauthenticated only (health + storefront-read + the existing
  401 negative-auth assertions, which already need NO token — `deploy-validation.spec.ts:1.1-1.3`);
  (b) full mock-auth E2E runs only against **staging**, gating prod deploy on the staging run;
  (c) ephemeral real prod tenant via legit APIs — **rejected** (writes real rows every deploy →
  pollutes the exact STOP-1 forensic tables; teardown-failure orphans); (d) separate narrow prod
  validation token — **rejected** (still a credential-minting bypass on prod, the very thing removed).
- **DECISION: (a) + (b).** Prod gets an unauthenticated smoke (mints no token on prod); the
  authenticated lifecycle + telegram suites move to a **staging gate that runs BEFORE prod deploy**
  (staging keeps the backdoor legitimately). `deploy-validation.spec.ts` is split: negative-auth +
  health + storefront → prod smoke; test 0.1 (mock-auth) + authenticated assertions → staging suite.
  `DEV_AUTH_SECRET` is **removed from prod deploy-job E2E steps**.
- **ORDER OF OPERATIONS (so prod is never both backdoored AND unvalidated):**
  1. (code, dark) split specs + rewire `ci.yml` — new `staging-e2e` gate (`needs: validate`, deploys
     image to `dowiz-staging`, runs authenticated suites), `deploy` job `needs: [validate, staging-e2e]`
     + prod-smoke. After this, prod deploy no longer needs the backdoor.
  2. ship G.1+G.2+C+D behind the flag (prod flag absent/false).
  3. operator unsets prod `DEV_AUTH_SECRET` + sets/verifies prod `NODE_ENV=production`, then deploys
     step-2 code → validates green via prod smoke + staging gate.
  4. operator rotates prod kid (R-6); forensics (STOP-1) before any cred/user-row cleanup.
- **Interaction with the operator's immediate mitigation (unset prod secret):** if the secret is unset
  **before** step 1 lands, the four prod mock-auth E2E steps go 404 → **post-deploy validation red**,
  but `flyctl deploy` itself still succeeds (code ships; only validation fails). If unset **after**
  step 1, the pipeline is green throughout. **Recommend step 1 first.** If incident urgency forces
  unset-first, accept a red-but-deploying validation step until the rewire lands. **Disposition: FIX,
  in scope. Owner: Implementer (pipeline) + Operator (sequencing). R-11.**

## R2-2 — [HIGH B-OPS] The "prod runtime stage" the NODE_ENV pin assumes does not exist → **FIX (per-app Fly secret + deploy-assert, NOT a Dockerfile ENV)**

**Verified true.** `Dockerfile` has exactly two `FROM` lines — `builder` (L2) and one **unnamed
runtime stage** (L30) — and the same image deploys to prod (`dowiz`) and staging (`dowiz-staging`).
Pinning `ENV NODE_ENV=production` there defaults the **whole non-prod fleet** to production; a
forgotten staging override trips D's boot-throw → staging/CI availability regression on a shared image.

**Resolution (proposal §9 table; ADR decision #3, reconciled with D):** the image stays
**NODE_ENV-agnostic** (no Dockerfile ENV — unchanged). Prod sets `NODE_ENV=production` as a **per-app
Fly secret**, and the **deploy job asserts the value before running validation** (`fly ssh ... printenv
NODE_ENV` == `production`, or a `/api/health` field) → red deploy on mismatch. This restores the lost
"verifiable, not invisible" property and makes it **stronger than greppable** (checked every deploy).
Fail directions: forgotten **prod** secret → deploy-assert catches it (red, old code serves); forgotten
**staging** secret → enum fails at boot (caught in staging deploy), NOT silently defaulted to production.
**Disposition: FIX. Owner: Operator (set per-app) + Implementer (deploy-assert). R-1 rewritten.**

## R2-3 — [MED B-SEC] C.3 "dev keypair absent from prod" is the same copy-paste class that failed for the secret → **ACCEPT-RISK (reduced), owner stated**

**Acknowledged, not hand-waved.** Key-absence-on-prod is the same guarantee class as
"secret-unset-on-prod," which already failed (staging secret pasted to prod). The dev keypair travels
the identical Fly-secret/CI-env channel. **Strengthened rather than just accepted:** (1) the dev
keypair is a *distinct artifact* from the prod signing key — copying it does not touch the prod key,
unlike one shared secret; (2) **D's boot-guard now treats `JWT_DEV_KID` set on a NODE_ENV=production
box as a fail-fast dangerous combo** — so the cheap-to-paste half (the kid) on a correctly-configured
prod refuses boot. Fail-open therefore requires BOTH a pasted keypair AND a wrong prod NODE_ENV (the
leak required only one copy) → lower residual than the secret, not zero. The "by construction" claim is
downgraded to "by construction **under secret hygiene**" in proposal §3-C and ADR #2. **Disposition:
ACCEPT-RISK (reduced). Owner: Operator (secret hygiene) + Architect (D fail-fast). R-10.**

## R2-4 — [MED B-CONSIST] C.1 switches the header kid but not the signing KEY → **FIX (kid AND key move together via one helper)**

**Verified true.** `signAuthToken` (`jwt.ts:36-41`) signs with `getPrivateKey()` = `env.JWT_PRIVATE_KEY`
unconditionally; original C.1 changed only `setProtectedHeader({...kid})`. A dev-kid token would be
**signed by the prod key** → the staging dev-verifier (dev public key) rejects it on signature → the
proof obligation could never pass. **Resolution (proposal §3-C C.1; ADR #2 C.1):** the signer takes an
optional signing key alongside the kid, and **both are selected by a single
`devSigningParams(env)`/`signDevToken()` helper** so kid and key can never diverge; non-dev callers
pass neither → `getKid()`+`getPrivateKey()` unchanged. Dev-mint call sites call one wrapper rather than
threading a key each — eliminating the "call site can't reach the hardcoded singleton" gap the breaker
named. New proof obligation added: a dev-mint token verifies against the **dev** public key and **fails
signature** against the prod public key. **Disposition: FIX. Owner: Implementer.**

## R2-5 — [LOW B-FAIL] email+IP rate-limit enumeration oracle + store-failure direction → **PARTIAL-FIX + ACCEPT-RISK**

Two sub-points, both on the out-of-scope real-argon2 path:
- **Enumeration oracle:** `local.ts` already leaks account-existence via distinct error strings
  ("Invalid email or password" vs "Account uses another sign-in method"); email+IP keying does not
  close that, and ~100/min enumeration-by-timing remains under the global IP cap. **Disposition:
  ACCEPT-RISK — pre-existing, out of this change's scope; noted for a follow-up to unify the error
  string.** The rate-limit change neither creates nor worsens it. Owner: Architect (tracked).
- **Store-failure direction:** proposal §7 left it as a "should fail-closed." **FIX — promote to spec:**
  the real-login per-route limiter **must fail-closed (reject)** when its store is unavailable, AND the
  exempt-when-gate-open branch (dev path) must NOT be affected (it has no per-route limiter to fail, so
  no CI-flake risk) — proposal §7 row updated to a spec with a proof obligation, not advisory. Owner:
  Implementer.

## Counsel RE-EXAMINE — forensic queries corrected (these feed the NEEDS-HUMAN decisions) → **FIX (all four)**

All in Part-B above, now corrected against live schema:
1. **RLS-FORCE silent-zero (most dangerous):** added an explicit **CRITICAL pre-req** that all forensic
   queries run as a **BYPASSRLS/superuser** role with a `current_user`/`is_superuser` verification — the
   app role returns zero by policy, manufacturing false-clean. **FIXED.**
2. **`customer_phone` non-existent column:** open-question query rewritten to `JOIN customers c ON
   c.id = o.customer_id` and `count(DISTINCT c.phone)` (verified `orders.customer_id` →
   `customers.phone`). **FIXED.**
3. **"real paid order" under-filtered:** added `status NOT IN ('PENDING','CANCELLED','REJECTED',
   'SCHEDULED')` and `payment_outcome IN ('paid_full','paid_partial')` — using the **real enum values**
   (verified `extensions-and-enums.ts:14,16`; there is NO `'paid'` value, `order_status` is UPPERCASE),
   so abandoned/pending carts no longer inflate severity. **FIXED.**
4. **CASCADE evidence-erasure ordering:** added STOP-1 item #4 — `memberships`/`auth_refresh_tokens`
   `ON DELETE CASCADE` from `users`; "delete empty@" means the **code literal ONLY, never the prod
   user row**; **forensics MUST run before any user-row deletion.** **FIXED.**

## Regression verdict on prior R2 findings (per re-attack)
- Prior CRITICAL #1 (C signer): the kid path was right; R2 closed the **key** path (R2-4). Proof
  obligation now satisfiable.
- Prior CRITICAL #2 (gate scope): HOLDS-as-fixed — and is precisely what surfaced R2-1. Resolving R2-1
  (deploy redesign) is the cost of keeping #2 fixed; accepted as in-scope.
- HIGH #3 (D on staging/CI): the unit-test rehearsal HOLDS; the prod-arming mechanism is corrected by
  R2-2 (per-app secret + deploy-assert, not the non-existent Dockerfile stage).
- HIGH #4, #5, MED #7, #8, LOW #9/#10: unchanged, still resolved as in round 1.

## R2 verification log (re-checked live this round)
- `ci.yml:127-184` — deploy job push-to-main, FOUR prod E2E steps each with `DEV_AUTH_SECRET`
  (`:162,169,176,183`); deploy step `flyctl deploy --remote-only` (`:151`). Confirmed.
- `e2e/tests/deploy-validation.spec.ts:13-25` — test 0.1 `POST ${BASE}/api/dev/mock-auth`; `:28-38`
  existing **unauthenticated 401** assertions (reusable as prod smoke, no token). Confirmed.
- `e2e/tests/flow-core-lifecycles.spec.ts:31` `POST /api/dev/mock-auth {role:'owner'}`, `:351`
  `/api/dev/create-assignment`. Confirmed.
- `e2e/lifecycle-e2e/playwright.config.ts:15-16` — `extraHTTPHeaders` injects `x-dev-auth-secret` from
  `DEV_AUTH_SECRET`. Confirmed.
- `Dockerfile` — two `FROM` (builder L2, unnamed runtime L30); no `NODE_ENV`; same image for prod +
  staging. **R2 HIGH confirmed — no prod-specific stage.**
- `packages/platform/src/auth/jwt.ts:36-41` — `getPrivateKey()` hardcoded; C.1 key path confirmed open.
- `packages/db/migrations/1780310074262_orders.ts:11,24,27,35` — `customers.phone`, `orders.customer_id`,
  `status order_status DEFAULT 'PENDING'`, `payment_outcome DEFAULT 'pending'`. Confirmed (R2 query #2/#3).
- `packages/db/migrations/1780310044710_extensions-and-enums.ts:14,16` — exact `order_status` /
  `payment_outcome` enum values. Confirmed (grounds the corrected query literals).
- `*.ts` migrations — `customers`/`orders`/`organizations`/`locations`/`memberships` are FORCE RLS
  (counsel #1). Confirmed pattern (`order_status_history` shown `FORCE ROW LEVEL SECURITY` at
  `1780338982015_order_history.ts:18`).

## R2 disposition summary (every R2 finding)

| Finding | Severity | Disposition | Owner |
|---|---|---|---|
| R2-1 Closing `/dev/*` breaks prod-deploy CI gate (4 steps) | CRITICAL | **FIX (in scope)** — deploy redesign: prod smoke (unauth) + staging pre-deploy gate; specs split; secret removed from prod steps; sequenced order of ops | Implementer (pipeline) + Operator (sequencing) |
| R2-2 Assumed Dockerfile prod runtime stage does not exist | HIGH | **FIX** — per-app Fly `NODE_ENV` secret + deploy-time assert; image stays NODE_ENV-agnostic; reconciled with D | Operator + Implementer |
| R2-3 Dev keypair absence = same copy-paste class that failed | MED | **ACCEPT-RISK (reduced)** — distinct artifact + D fail-fast on `JWT_DEV_KID`@prod; needs BOTH paste AND wrong NODE_ENV | Operator (hygiene) + Architect (D) |
| R2-4 C.1 switches kid not signing KEY | MED | **FIX** — one `signDevToken()`/`devSigningParams` helper moves kid+key together; new key-path proof | Implementer |
| R2-5a email+IP enumeration oracle | LOW | **ACCEPT-RISK** — pre-existing error-string leak, out of scope; follow-up to unify strings | Architect (tracked) |
| R2-5b limiter store-failure direction | LOW | **FIX** — promote to spec: real-login limiter fails-closed; dev path unaffected | Implementer |
| Counsel #1 RLS silent-zero | — | **FIX** — BYPASSRLS pre-req + verification added | Architect (queries) → Operator (run) |
| Counsel #2 `customer_phone` bad column | — | **FIX** — join customers, count DISTINCT c.phone | Architect (queries) |
| Counsel #3 paid vs created over-count | — | **FIX** — real enum filter (`paid_full/paid_partial`, exclude PENDING/CANCELLED/REJECTED) | Architect (queries) |
| Counsel #4 CASCADE evidence-erasure ordering | — | **FIX** — code-literal only, never prod user row; forensics before any deletion | Operator |

## Scope-change statement (honest)
B-OPS forced a scope change. The **deploy-pipeline redesign (R2-1) and the per-app NODE_ENV
deploy-assert (R2-2) are now part of this remediation**, not tracked follow-ups — because the prior
CRITICAL #2 fix (closing `/dev/*` on prod) cannot ship without them or the prod deploy job goes red
(or the backdoor must stay open, defeating the change). The remediation is therefore larger than the
original gate-function edit: it now spans `dev-guard.ts` (G), `jwt.ts` (C + the key helper),
`config/index.ts` + boot-guard (D), `local.ts`/`server.ts` (cred removal), **`ci.yml` + the E2E spec
split (deploy redesign)**, and per-app Fly NODE_ENV secrets. Two items remain tracked follow-ups with
owners: the auth-audit-log gap (advice, non-blocking) and the enumeration error-string unification
(R2-5a). Everything else is FIX-in-scope or NEEDS-HUMAN-DECISION as tabled.

---

# RESOLVE round 3 (2026-06-22) — final convergence

The R2 re-attack left **two findings open**, both **operability** (not security): R2-1 (HIGH —
spec-split incomplete against real source) and R2-2 (MEDIUM — NODE_ENV deploy-assert fail-open /
post-hoc). I re-verified both against live source this round before resolving (R3 verification log
below). Both confirmed exactly as the breaker stated.

## R3-1 — [HIGH B-OPS] R2-1 spec-split is a SOURCE rewrite, not a CI-config change → **FIX (E2E work enlarged, spec'd concretely)**

**Verified true, in full.** Live source this round:
- `e2e/tests/deploy-validation.spec.ts:3` reads `process.env.VITE_BASE_URL || 'https://dowiz.fly.dev'`
  (env-driven) but `:8` is `test.describe.configure({ mode: 'serial' })` with **22 tests**; test `0.1`
  (`:13`) `POST ${BASE}/api/dev/mock-auth` assigns module-level `authToken`; **10 tests** carry
  `Authorization: Bearer ${authToken}` (grep count = 10). Only **1.1/1.2/1.3** (`:28/:35/:40`) are
  truly unauthenticated (401 negatives). The "storefront" test `3.1` (`:66`) is **authenticated** — it
  calls `/api/owner/settings` with `Bearer ${authToken}` (`:67`) to obtain `locationSlug`, then reads
  the public menu/theme; `5.1/9.1/12.1/13.1` likewise depend on the auth-derived slug. So the proposal's
  "keep negative-auth + health + storefront as the prod smoke" is **not a clean subset** — the serial
  shared-state makes the 3 negatives the only portable tests, and even those must be **extracted** into
  a standalone non-serial file to run without the chain.
- `e2e/tests/flow-core-lifecycles.spec.ts:4` is env-driven (`VITE_BASE_URL || ...`) — moves to staging
  by env, **no source edit**.
- `e2e/tests/telegram-webhook.spec.ts:3` and `telegram-full-flow.spec.ts:3` are **`const BASE =
  'https://dowiz.fly.dev'`** with **NO env fallback** (verified). `VITE_BASE_URL` on the new `staging-e2e`
  job has **zero effect** — they hit prod regardless and authenticate via `/api/dev/mock-auth`
  (`telegram-full-flow:51`). The R2 claim "specs unchanged / no spec change for C" was **wrong**.

**Resolution — re-specified concretely (proposal §9.A R3, ADR Decision #4 R3):**
1. **Prod unauth smoke = a NEW standalone non-serial spec** (`e2e/tests/prod-smoke.spec.ts`), env-driven,
   minting no token. Exact assertions: `GET /livez` → 200; `GET /health` → 200/503 (no NODE_ENV field —
   R3-2); a **public storefront read** via `GET /s/:slug` SSR (or `/public/locations/:slug/menu` +
   `/api/public/theme/:slug`) against a **seeded public slug passed by env** (`PROD_SMOKE_SLUG`, NOT
   read from the authenticated `/api/owner/settings`); and the **extracted** 1.1–1.3 401 negatives. These
   are the only "reusable" prod assertions and must be **lifted out** of the serial file to be portable.
2. **Spec files that MUST change** (correcting the false "unchanged" claim): edit
   `e2e/tests/telegram-webhook.spec.ts` and `telegram-full-flow.spec.ts` to read
   `const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev'` (one line each); **create**
   `e2e/tests/prod-smoke.spec.ts`; **retire** `deploy-validation.spec.ts` from the prod job (it runs on
   staging unchanged). `flow-core-lifecycles.spec.ts` needs **no** edit.
3. **The `staging-e2e` gate runs the FULL authenticated suite** (`flow-core-lifecycles` + edited
   `telegram-*` + `deploy-validation`) against `dowiz-staging` (`ALLOW_DEV_LOGIN=true`,
   `DEV_AUTH_SECRET=stg-e2e-secret`, dev keypair) **before** the prod deploy job; prod proceeds only on green.

**Back-of-envelope (honest, R3-corrected):** of **~50 tests** across the 4 prod-targeted files, only
**~3 reuse as-is** (the 1.1–1.3 negatives); the rest move to staging (`flow-core` no-edit;
`deploy-validation` whole serial suite) or require source edits (2 telegram BASE lines) or are
newly written (prod-smoke). New proof obligations added (proposal Proof-obligations; ADR Verification).
**This enlarges the E2E work — stated plainly, it is a spec rewrite, not a CI-env tweak. Disposition:
FIX, in scope. Owner: Implementer (E2E). R-12.**

## R3-2 — [MEDIUM B-OPS] NODE_ENV deploy-assert is fail-open / post-hoc → **FIX (pre-traffic gate in `release_command`)**

**Verified true.** Live source this round:
- `apps/api/src/routes/health.ts` returns `{ status, timestamp, checks }` — **no NODE_ENV field**
  (verified, `grep NODE_ENV` = 0). The file explicitly minimizes the **unauthenticated** public payload
  to avoid a recon leak (`:37-38`, `:320-323`). Adding a `NODE_ENV` field would **reverse that deliberate
  hardening** — rejected.
- `ci.yml:151` `flyctl deploy --remote-only` swaps the new machine into service; the post-deploy E2E
  steps (incl. any `fly ssh console` NODE_ENV assert) run **after**. So a wrong (`development`) prod
  NODE_ENV **boots, serves, deploy succeeds**, and a post-deploy assert only reds the CI job — no
  rollback; the dev-mode artifact is already live. Confirmed: the R2 assert was post-traffic, fail-open
  for the wrong-value case.
- D (boot-guard) **already** fail-fasts the **dangerous** direction (NODE_ENV=production + dev flag/secret
  → throw before listen). The **remaining gap is the INVERSE**: prod NODE_ENV ≠ production → D never fires
  → the `/dev/*` closure the design relies on stays open, silently.

**Options weighed:**
- **(a) `/health` NODE_ENV field** — **rejected**: does not exist, must not be added (reverses recon
  hardening), and still runs post-traffic.
- **(b) boot-sequence assert in `server.ts`** — partial: fires on the prod marker, but on **every** box
  (web + worker + local `docker run`) and a crash-looping new machine is a messier failure than a clean
  rollout-abort; also fires *after* the machine is chosen to serve.
- **(b') `release_command` assert** — **CHOSEN.** `fly.toml:14-15` already has
  `release_command = "dist/migrate/index.cjs"`, which Fly runs in a one-off machine **before** the new
  machine takes traffic, and **a nonzero exit aborts the release** (the same fail-safe the schema-drift
  guard relies on; verified `fly.toml:7-15`). Add to the migrator entrypoint, before applying migrations:
  ```
  if (process.env.FLY_APP_NAME === 'dowiz' && process.env.NODE_ENV !== 'production') {
    console.error(`FATAL: prod NODE_ENV must be 'production', got '${process.env.NODE_ENV ?? '(unset)'}'`);
    process.exit(1);   // nonzero → Fly aborts the release, new artifact never serves
  }
  ```
  This is **genuinely pre-traffic**, runs once on the target app with that app's env, aborts the rollout
  on a wrong/unset prod NODE_ENV (old code keeps serving), needs no `/health` field, and is checked on
  **every** prod deploy. **Staging reconciliation (shared image):** the migrator also runs on
  `dowiz-staging` (same `release_command`), which legitimately runs `development` — so the check is
  keyed on `FLY_APP_NAME === 'dowiz'` and is **inert on staging** (an optional inverse line asserts
  non-production when `FLY_APP_NAME === 'dowiz-staging'`, catching a prod-mode-on-staging fat-finger
  pre-traffic too).

This turns the **inverse** condition (prod NODE_ENV ≠ production) into a **fail-closed pre-traffic boot
condition** — exactly what the finding asked for: D covers the dangerous direction at app-boot; the
`release_command` covers the inverse direction before the artifact ever serves. **Disposition: FIX.
Owner: Implementer (assert) + Operator (set per-app NODE_ENV). R-1 rewritten; R-13 added (verify
`FLY_APP_NAME` is in the `release_command` env, else use an explicit `DEPLOY_TARGET=prod` marker secret).**

## Regression verdict on prior CRITICAL/HIGH (R3 — no regression)
- **R1 CRITICAL #1 (C signer header-kid):** HOLDS-as-fixed (C.1 + R2-4 key path). Untouched by R3.
- **R1 CRITICAL #2 (6 mint sites / mock-auth ungated):** HOLDS-as-fixed (G.1+G.2, one flag, two guards).
  R3 changes are downstream (CI/E2E + deploy gate) and do not weaken the gate. No regression.
- **R2-1 CRITICAL→HIGH (prod-deploy gate calls `/dev/*`):** the decision (a)+(b) HOLDS; R3-1 completes
  its under-specified spec-split. Now fully specified; no longer a coverage hole.
- **R2-2 HIGH (Dockerfile prod stage non-existent):** HOLDS-as-redirected; R3-2 makes the replacement
  mechanism (was a fail-open post-deploy assert) into a sound pre-traffic `release_command` gate.
- All other HIGH (#3 D testability, #4 rate-limit, #5 leaked token) and MED/LOW: unchanged, still
  resolved. No prior CRITICAL/HIGH regressed.

## R3 verification log (re-checked live this round)
- `e2e/tests/deploy-validation.spec.ts:3` env-driven; `:8` `mode:'serial'`; 22 `test('` blocks; `0.1`
  assigns `authToken`; 10 `Authorization: Bearer ${authToken}`; `3.1:67` authenticated `/api/owner/settings`
  derives `locationSlug`. Confirmed.
- `e2e/tests/telegram-webhook.spec.ts:3` + `telegram-full-flow.spec.ts:3` — `const BASE =
  'https://dowiz.fly.dev'`, **no `VITE_BASE_URL`**. Confirmed.
- `e2e/tests/flow-core-lifecycles.spec.ts:4` — `process.env.VITE_BASE_URL || ...` (env-driven). Confirmed.
- `apps/api/src/routes/health.ts` — `{status,timestamp,checks}`, no NODE_ENV field; recon-leak hardening
  comments `:37-38,320-323`. Confirmed.
- `fly.toml:14-15` — `[deploy] release_command = "dist/migrate/index.cjs"`; `:7-13` documents nonzero exit
  aborts the rollout, old code keeps serving (pre-traffic fail-safe). Confirmed — this is the real
  pre-serving gate.

## R3 disposition summary

| Finding | Severity | Disposition | Owner |
|---|---|---|---|
| R3-1 (R2-1) spec-split is a source rewrite, not CI-config; ~3 of ~50 tests reusable | HIGH | **FIX (in scope)** — new standalone `prod-smoke.spec.ts` (seeded-slug public read + extracted 1.1–1.3); edit both telegram specs to read `VITE_BASE_URL`; retire `deploy-validation` from prod job; full auth suite on staging-e2e gate | Implementer (E2E) |
| R3-2 (R2-2) NODE_ENV deploy-assert fail-open / post-hoc | MEDIUM | **FIX** — pre-traffic assert inside `release_command` (`FLY_APP_NAME==='dowiz' && NODE_ENV!=='production' → exit 1`); no `/health` field; inverse-direction fail-closed gate | Implementer (assert) + Operator (set NODE_ENV) |
| R-13 verify `FLY_APP_NAME` present in `release_command` env | (risk) | **VERIFY / fallback marker** — else explicit `DEPLOY_TARGET=prod` secret | Implementer + Operator |

---

# FINAL FINDING-DISPOSITION TABLE (all rounds)

| Round | Finding | Severity | Final disposition | Owner |
|---|---|---|---|---|
| R1 | #1 Option C non-implementable (signer hardcodes header kid) | CRITICAL | **FIX** (C.1 header-kid + R2-4 key path) | Implementer / Operator (keys) |
| R1 | #2 Gate scope wrong; 6 mint sites, mock-auth ungated | CRITICAL | **FIX** (G.1+G.2, one flag, two guards) | Implementer |
| R1 | #3 D silent on staging/CI (no NODE_ENV) | HIGH | **FIX** (NODE_ENV matrix + D unit-tested) | Operator + Implementer |
| R1 | #4 5/min rate-limit flakes CI | HIGH | **FIX** (exempt-when-gate-open + email+IP) | Implementer |
| R1 | #5 Leaked `kid:1` token not invalidated by design | HIGH | **ACCEPT-RISK** (mandatory operator kid rotation R-6) | Operator |
| R1 | #6 Singleton kid sign/verify | MED | **FIX** (per-token kid+key via helper) | Implementer |
| R1 | #7 `empty@dowiz.com` 2nd cred pair | MED | **FIX** (DELETE bypass branch + both creds) | Implementer |
| R1 | #8 NODE_ENV enum lacks `staging`; D untestable pre-prod | MED | **FIX** (unit-test rehearsal; enum unchanged) | Implementer |
| R1 | #9 No rate-limit on mock-auth minters | LOW | **ACCEPT-RISK** (flag+secret-gated, prod-dead) | Architect |
| R1 | #10 Dev-keypair overhead before C.1 | LOW | **FIX** (sequencing C.1→C.2→C.3 + proof gate) | Implementer |
| R2 | R2-1 Closing `/dev/*` breaks prod-deploy CI gate (4 steps) | CRITICAL→HIGH | **FIX (in scope)** — deploy redesign (a)+(b); spec-split fully specified in R3-1 | Implementer + Operator (seq) |
| R2 | R2-2 Assumed Dockerfile prod runtime stage does not exist | HIGH | **FIX** — per-app Fly secret; pre-traffic `release_command` assert (R3-2) | Operator + Implementer |
| R2 | R2-3 Dev keypair absence = same copy-paste class | MED | **ACCEPT-RISK (reduced)** (distinct artifact + D fail-fast on `JWT_DEV_KID`@prod) | Operator (hygiene) + Architect |
| R2 | R2-4 C.1 switches kid not signing key | MED | **FIX** (`signDevToken`/`devSigningParams` moves kid+key together) | Implementer |
| R2 | R2-5a email+IP enumeration oracle | LOW | **ACCEPT-RISK** (pre-existing error-string leak; follow-up to unify) | Architect (tracked) |
| R2 | R2-5b limiter store-failure direction | LOW | **FIX** (real-login limiter fails-closed; dev path unaffected) | Implementer |
| R3 | R3-1 spec-split is a source rewrite (~3/~50 reusable) | HIGH | **FIX (in scope)** (new standalone prod-smoke + telegram BASE edits + staging gate) | Implementer (E2E) |
| R3 | R3-2 NODE_ENV deploy-assert fail-open / post-hoc | MED | **FIX** (pre-traffic `release_command` inverse-direction gate) | Implementer + Operator |
| Counsel | #1 RLS silent-zero on forensic queries | — | **FIX** (BYPASSRLS pre-req + verification) | Architect (queries) → Operator (run) |
| Counsel | #2 `customer_phone` bad column | — | **FIX** (join customers, count DISTINCT c.phone) | Architect (queries) |
| Counsel | #3 paid vs created over-count | — | **FIX** (real enum filter `paid_full/paid_partial`, exclude PENDING/CANCELLED/REJECTED) | Architect (queries) |
| Counsel | #4 CASCADE evidence-erasure ordering | — | **FIX** (code-literal only, never prod user row; forensics first) | Operator |
| ETHICAL-STOP-1 | Forensics before "closed" | — | **NEEDS-HUMAN** — run BYPASSRLS row-provenance + refresh-token queries (Part B), record before "closed"; no user-row deletion first | Operator (human) |
| ETHICAL-STOP-2 | Disclosure obligation | — | **NEEDS-HUMAN** — depends on STOP-1; record notify/don't-notify in `/compliance` | Operator / data-steward (human) |
| Open-Q | Real users + paid orders in window | — | **NEEDS-HUMAN** — run BYPASSRLS count query (Part B); operator defines window-start | Operator (human) |

## Hard-exit statement

**The design is at hard-exit.** Unresolved CRITICAL = 0; unresolved HIGH = 0 (R2-1→R3-1 and R2-2→R3-2
are the last two HIGH/MED, both now FIX-in-scope with concrete specs + proof obligations). The three
ethical/forensic items remain **NEEDS-HUMAN-DECISION** with correct, schema-verified queries (BYPASSRLS
pre-req, customers-join, paid-enum filter, CASCADE-before-deletion ordering) — these are by-design
human-judgment gates, not engineering gaps. R-6 (operator kid rotation) remains a mandatory accepted-risk
mitigation owned by the operator. No prior CRITICAL/HIGH regressed across R1→R2→R3.
