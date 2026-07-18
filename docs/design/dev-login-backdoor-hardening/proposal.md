# Design Proposal — Dev-Login Backdoor Hardening

**Slug:** `dev-login-backdoor-hardening`
**Status:** RESOLVED (post-breaker R2, post-counsel R2) — design-time only (no production code written)
**Resolution:** `docs/design/dev-login-backdoor-hardening/resolution.md` (prior 2 CRITICALs fixed; re-attack NEW CRITICAL [deploy-pipeline] + NEW HIGH [Dockerfile stage] resolved in RESOLVE round 2; counsel stops → NEEDS-HUMAN-DECISION)
**Scope note (R2):** the NEW CRITICAL forces a **deploy-pipeline redesign** into this change — closing `/dev/*` on prod breaks the prod deploy validation gate, which authenticates via prod `/api/dev/mock-auth`. The CI workflow rewrite (post-deploy validation no longer mints owner/courier tokens on prod) is now **part of this remediation**, not a follow-up. See §9.A.
**Companion ADR:** `docs/adr/0003-dev-login-fail-closed.md`
**Severity:** CONFIRMED live CRITICAL on production (see §1)
**Author:** System Architect (DeliveryOS)
**Date:** 2026-06-22

---

## 1. Problem + non-goals

### Problem (confirmed, live)
A dev-only password backdoor is **active on production**. `POST https://dowiz.fly.dev/api/auth/local/login`
with `{"email":"test@dowiz.com","password":"test123456"}` returns **HTTP 200 with a real
`role:'owner'` JWT signed by the prod key (`kid:1`)**. The gate is purely "is `DEV_AUTH_SECRET`
set?" (`apps/api/src/plugins/dev-guard.ts:19-21`, `devLoginAllowed` returns `!!configuredSecret`),
and the prod secret is currently set (leaked from staging's `stg-e2e-secret`).

The handler (`apps/api/src/server.ts:868-892`):
- never reads `password_hash` — the password is a **hardcoded literal** in shipped code;
- mints a real owner JWT under the **prod signing key/kid**, valid 1 day;
- has **no rate limit**, **no Zod body schema** (`request.body as any || {}`).

**Blast radius — full tenant compromise, not read-only.** Even when the seeded user has no
membership (`activeLocationId` null), the minted `role:'owner'` token satisfies
`POST /api/owner/onboarding/start` (`apps/api/src/routes/owner/onboarding.ts:35`), which requires
only `role=owner` and **creates org + location + owner membership**. The token self-escalates to a
fully provisioned owner account on prod.

There are **six** dev/test-identity mint sites, gated by **two** different broken guards, not one
(corrected from the original "three" after the breaker round — verified by
`grep signAuthToken apps/api/src`):

| # | Site | Mints | Current gate |
|---|---|---|---|
| 1 | `server.ts:888` inline `/api/auth/local/login` | owner, 1d — the live exploit | `devLoginAllowed` = `!!secret` |
| 2 | `server.ts:650/664/711` inline `/api/dev/mock-auth` | courier, fresh-owner (no membership), owner+**writes memberships**, 1d | **`isDevRequestAuthorized`** (path `/dev/`, `!!secret`) — **does NOT call `devLoginAllowed`** |
| 3 | `routes/dev/mock-auth.ts:16/72` duplicate `/dev/mock-auth` | courier, owner + **writes memberships**, 1d | **`isDevRequestAuthorized`** (`!!secret`) — **does NOT call `devLoginAllowed`** |
| 4 | `routes/auth/local.ts:108` plugin `/auth/local/login` | role-resolved, 15m + refresh row; **2nd cred `empty@dowiz.com`** | `devLoginAllowed` (bypass branch only); 404s today but ships in image |
| 5 | `routes/dev/mock-auth.ts:138/158/77` seed/repair/assignment helpers | (no JWT) write targets/memberships/assignments | `isDevRequestAuthorized` (`!!secret`) |
| 6 | `apps/api/src/plugins/auth.ts:66` mock-auth honor | accepts a courier token with no session `jti` | `devLoginAllowed` |

**This is the breaker's CRITICAL #2:** the `/dev/*` mock-auth family (sites 2/3/5) rides
`isDevRequestAuthorized` (`dev-guard.ts:47`), NOT `devLoginAllowed`. Hardening only `devLoginAllowed`
leaves the owner-minting mock-auth path open on a prod that holds a leaked secret. The fix must
harden **both** guard functions under one shared flag (see §4).

### Goals
- Production **fails closed** for all dev-login / mock-auth paths regardless of whether
  `DEV_AUTH_SECRET` is accidentally set, and regardless of NODE_ENV reliability.
- Staging + CI E2E **keep working** unchanged (they depend on dev-login + `/dev/*`).
- Remove hardcoded credentials from shipped code.
- Make misconfiguration **loud at boot**, not silently exploitable at runtime.
- Prevent any **future** dev/mock token from verifying on prod (dev-kid strategy, §C). The
  **already-minted** leaked `kid:1` token is killed by operator key rotation (R-6) — see §8/R-6;
  the design prevents recurrence, it does not retroactively reject a token bearing the prod kid.

### Non-goals
- Replacing the real password-login (argon2) path in `local.ts` — out of scope.
- Building a full secrets-rotation pipeline — JWT key rotation is noted as an accepted follow-up.
- The **immediate mitigation** (operator unsets the prod `DEV_AUTH_SECRET` + rotates the JWT
  signing key to invalidate the leaked token) is being handled separately. This design is the
  **permanent** hardening so the hole cannot reopen.

---

## 2. Back-of-envelope

**Scale here is not request volume — it is configuration surface and deploy targets.** The
"capacity" budget is: how many call sites, env knobs, and deploy environments must stay consistent.

- **Dev/test mint sites:** **6** (table in §1), gated by **two** guard functions — `devLoginAllowed`
  (sites 1/4/6) and `isDevRequestAuthorized` (sites 2/3/5). A fix touching only `devLoginAllowed`
  leaves the three `/dev/mock-auth`-family sites open. The fix routes **all six through one shared
  `ALLOW_DEV_LOGIN` flag folded into both guard functions** (§4 G.1+G.2). Not "one function covers
  all" — two guards, one flag, six sites.
- **Hardcoded credential literals:** 2 pairs (`test@dowiz.com`/`test123456`,
  `empty@dowiz.com`/`empty123456`), in 2 files.
- **Deploy environments (3):**
  - **prod** = Fly app `dowiz` (`dowiz.fly.dev`). Must end with dev-login OFF.
  - **staging** = Fly app `dowiz-staging` (`dowiz-staging.fly.dev`). Sets `DEV_AUTH_SECRET=stg-e2e-secret`; must keep dev-login ON.
  - **CI fresh-provision** = ephemeral PG/Redis (`ci.yml:51-125`); sets `DEV_AUTH_SECRET=ci-fresh-secret`. Must keep ON.
- **E2E surfaces depending on the secret:** ~14 spec files + `e2e/lifecycle-e2e/playwright.config.ts:15`
  send `x-dev-auth-secret` and call `/dev/mock-auth` or dev-login (grep: `DEV_AUTH_SECRET` across `e2e/`).
  Changing the **gate** must not change the **header protocol** these rely on. **But** (R2 re-attack):
  the *gate change closes the prod endpoint regardless of the header* — the protocol being untouched is
  irrelevant to the prod-targeted suites; see next bullet.
- **PROD-targeted CI gate (the R2 CRITICAL surface):** the CI `deploy` job (`ci.yml:127-184`, push-to-main,
  deploys prod `dowiz`) runs **four** post-deploy E2E steps against `https://dowiz.fly.dev`, all of which
  authenticate via the prod backdoor:
  - `deploy-validation.spec.ts:13` — `POST /api/dev/mock-auth` (test 0.1 "mock-auth returns valid owner token");
  - `flow-core-lifecycles.spec.ts:31` — `POST /api/dev/mock-auth {role:'owner'}`, `:351` — `POST /api/dev/create-assignment`;
  - `telegram-webhook.spec.ts` + `telegram-full-flow.spec.ts` (ci.yml:172-184) — same mock-auth setup.
  The secret is injected per-step via `DEV_AUTH_SECRET: ${{ secrets.DEV_AUTH_SECRET }}` (ci.yml:162,169,176,183)
  and the header by `playwright.config.ts:15`. **So the prod backdoor is LOAD-BEARING for deploy validation,
  not an accidental leak.** Closing `/dev/*` on prod (G.2) turns all four steps red on every push to main.
  Budget item: **4 CI deploy-job steps** must be re-pointed off the prod backdoor (see §9.A).
- **E2E reuse budget (R3-corrected — honest):** of the **~50 tests** across the 4 prod-targeted spec
  files, only **~3 are reusable against a backdoor-closed prod as-is** — the standalone 401 negatives
  `deploy-validation.spec.ts:1.1–1.3`. Everything else **moves to staging or gets rewritten**:
  `deploy-validation.spec.ts` is a **22-test serial chain** (10 carry `Bearer ${authToken}`; its
  "storefront" test reads the slug from the **authenticated** `/api/owner/settings`, not a public read)
  → retired from the prod job, runs on staging; both **telegram specs hardcode `BASE='https://dowiz.fly.dev'`
  with no env fallback** → must be **source-edited** to read `VITE_BASE_URL` before they can target
  staging; `flow-core-lifecycles.spec.ts` is already env-driven → moves to staging with no edit. The
  **prod unauth smoke is a NEW standalone spec** (health/livez + a public `/s/:slug` read by seeded slug +
  the 3 extracted negatives). **This enlarges the E2E work — it is a spec rewrite, not a CI-env tweak**
  (see §9.A R3).

### NODE_ENV reliability — the load-bearing assumption (VERIFY)
- `packages/config/src/index.ts:4` — `NODE_ENV: z.enum(["development","test","production"])`,
  **required, no default**. `loadEnv()` throws if NODE_ENV is unset or any other value
  (e.g. `"staging"`). So a running app *has* a valid NODE_ENV — but the repo does **not** prove
  which one prod uses:
  - **Dockerfile sets NO `NODE_ENV`** (verified: runtime stage `apps/.../Dockerfile` has no
    `ENV NODE_ENV`).
  - **fly.toml has NO `[env]` block** (verified).
  - Therefore prod's NODE_ENV comes **only from a Fly secret/env** set out-of-band. **It is not
    in the repo and not verifiable from source.** `.env.example:1` is `NODE_ENV=development`.
  - **Consequence:** a NODE_ENV-only gate is only as trustworthy as an invisible Fly setting. If a
    future `fly secrets` edit drops it, the app won't boot (good) — but if anyone ever adds
    `ENV NODE_ENV=production` to the Dockerfile, **staging would also boot as production and lose
    dev-login**, breaking E2E. The gate must not depend on NODE_ENV *alone*.

**Assumptions to verify with the operator (cannot be read from repo):**
1. `dowiz` Fly app has `NODE_ENV=production` set as a secret/env. (If unset → boot already fails;
   if it boots, it's set.)
2. `dowiz-staging` has `NODE_ENV` = `production`, `staging`-rejected (would fail boot), or
   `development`. **This determines whether a NODE_ENV-gate alone would break staging E2E.**
3. The prod `JWT_KID` value (currently `1`) and whether it equals staging's. If they share a kid,
   kid-segregation alone is insufficient.

---

## 3. Options (≥2, named, with tradeoffs)

### Option A — "NODE_ENV Gate" (the proposed baseline)
Gate `devLoginAllowed` on `NODE_ENV !== 'production'` AND a configured secret.
- **Concept:** environment-aware feature gate.
- **Pros:** one-line change in the gate; covers all 3 call sites; zero new env vars.
- **Cons:** **single point of trust is an invisible Fly setting** (§2). If staging runs
  NODE_ENV=production (plausible, to mirror prod), dev-login dies on staging → E2E breaks. If prod's
  NODE_ENV is ever lost, the only backstop is boot-failure. Does **not** invalidate already-leaked
  tokens. **Rejected as the sole mechanism** — too coupled to one unverifiable knob.

### Option B — "Explicit Allow-Flag" (`ALLOW_DEV_LOGIN`)
Introduce a dedicated `ALLOW_DEV_LOGIN` env (`'true'|'false'`, default `'false'`). Dev-login is
permitted only when it is explicitly `'true'` AND `DEV_AUTH_SECRET` is set.
- **Concept:** explicit opt-in capability flag (fail-closed by default).
- **Pros:** intent is **explicit and greppable**; default-off means prod is safe even if someone
  copies the staging secret; decoupled from NODE_ENV's reliability; staging/CI just set the flag.
- **Cons:** a new env var to set in 2 environments (staging + CI); a sloppy operator could set it
  true in prod (mitigated by Option D boot-guard); doesn't invalidate already-leaked tokens.

### Option C — "Dev-Kid Segregation" (cryptographic) — REWRITTEN to be implementable
**Original C was non-implementable (breaker CRITICAL #1, verified):** `signAuthToken`
(`jwt.ts:28,37`) hardcodes the **protected-header** kid to `getKid()` (the env kid); the `payload.kid`
override (`jwt.ts:33`) lands only in the **body claim**. `verifyAuthToken` checks the **header**
(`jwt.ts:55`). CI sets `JWT_KID=1` (= prod kid, `ci.yml:103`). So a "dev-kid" token still carries
`header.kid = its env JWT_KID`, and the verifier rejection C relied on never triggers. C as originally
written invalidated **zero** tokens. The "reuses existing check, no new sign logic" claim was false.

Mint dev/mock tokens under a **separate dev keypair + dev kid**, with three concrete changes:
- **C.1 — Signer (kid AND key together — R2 B-CONSIST fix):** the original C.1 moved only the
  *kid* into the protected header but left `getPrivateKey()` hardcoded to `env.JWT_PRIVATE_KEY`, so a
  "dev-kid" token would still be **signed by the prod key** — the staging dev-verifier (dev public key)
  would then reject it on signature, and the proof obligation could never pass. **Resolved:** the signer
  takes an optional `signingKey` alongside the kid, and the two move as a pair —
  `headerKid = payload.kid || getKid()`; `key = signingKey || getPrivateKey()`;
  `setProtectedHeader({ alg:'RS256', kid: headerKid }).sign(key)`. **A single helper picks BOTH from one
  source** so kid and key can never diverge: `devSigningParams(env) → { kid: JWT_DEV_KID, key: JWT_DEV_PRIVATE_KEY }`
  used only by dev-mint sites; everyone else passes neither → `getKid()` + `getPrivateKey()` (unchanged
  prod behavior). The dev-mint call sites do NOT individually thread a key — they call one
  `signDevToken()` wrapper that reads the pair, eliminating the "call site can't reach the hardcoded
  singleton" gap. (Also resolves the singleton-kid MED: both kid and key are per-token args at the
  signer for the dev path.)
- **C.2 — Verifier:** accept the dev kid **only in non-prod** —
  `acceptDevKid = NODE_ENV!=='production' && !!JWT_DEV_KID`; accept `header.kid === getKid()` OR
  (`acceptDevKid && header.kid === JWT_DEV_KID`). Prod never sets `acceptDevKid`. A wrong prod
  NODE_ENV fails **safe** because the dev keypair is absent from prod (C.3) — prod-rejection rests on
  **key-material isolation**, with NODE_ENV as a second lock.
- **C.3 — Keys:** new non-prod envs `JWT_DEV_KID` + `JWT_DEV_PRIVATE_KEY`/`JWT_DEV_PUBLIC_KEY`
  (staging Fly secret; CI extends the existing throwaway-keypair gen `ci.yml:112-118`). **Prod has
  none.** The `x-dev-auth-secret` request protocol is **untouched** — C changes only the signing
  kid/keypair, not the auth header the 14 specs send. No spec change for C.
- **Concept:** key/kid namespacing — future dev tokens are bound to a keypair prod does not hold.
- **Pros:** future dev/mock tokens are **prod-rejected by construction**; survives env-config
  mistakes (key isolation, not just NODE_ENV).
- **Cons / honest scope:** does NOT reject the **already-minted** `kid:1` leaked token (it bears the
  prod header kid — only R-6 rotation kills it); adds a dev keypair to two non-prod environments;
  requires the C.1/C.2 code changes (NOT free). Sequencing: C.1→C.2→C.3 together, gated on the
  "dev-kid token rejected by prod-kid verifier" proof, or the team may *believe* C is live when it
  is not (LOW B-ANTIPATTERN #10).
- **Honest limit of the "by construction" claim (R2 MED B-SEC):** prod-rejection of dev-kid tokens
  rests on the dev keypair being **absent from prod** — which is the *same class of guarantee* as
  "`DEV_AUTH_SECRET` is unset on prod," and that guarantee **already failed once** (the live leak was a
  staging secret copied to prod). The dev keypair travels the **identical distribution channel** (Fly
  secrets / CI env-file) and carries the **identical copy-paste hazard**. So C is "prod-rejected by
  construction" **only under the discipline that no one pastes the dev keypair onto prod AND prod's
  NODE_ENV is correct** — exactly the discipline that broke. Residual stated, not hidden (R-10).
  Two structural mitigations that DO reduce the residual below the secret's: (1) the dev keypair is a
  *distinct artifact* from the prod signing key (copying it does not require touching the prod key,
  unlike a single shared secret), and (2) D's boot-guard treats `JWT_DEV_KID` set on prod as a
  fail-fast dangerous combo — so the *kid* (the cheap-to-paste half) on a correctly-NODE_ENV'd prod
  refuses boot. The keypair-on-prod fail-open requires BOTH a pasted keypair AND a wrong prod
  NODE_ENV; the secret leak required only one copy. Lower residual, not zero. **Owner: Operator
  (secret hygiene); Architect (the D fail-fast that catches the common half).**

### Option D — "Boot Fail-Fast" (config invariant) — orthogonal hardening
Make `loadEnv()` **throw at boot** when the dangerous combination is present in prod
(NODE_ENV=production AND dev-login-enabling config set). This is not an alternative to A/B/C; it is
the **detector** that converts a silent runtime exploit into a loud boot failure.
- **Concept:** fail-fast config invariant (the same pattern as the schema-drift boot-guard).
- **Pros:** misconfig becomes a crash-on-deploy (Fly aborts rollout → old code keeps serving, per
  the existing `[deploy] release_command` fail-safe), detectable in <1 min; cheap.
- **Cons:** relies on NODE_ENV being correctly `production` on prod to fire (so it backstops B, it
  doesn't replace it). If NODE_ENV is wrong on prod, the guard is silent — hence we combine.

---

## 4. Decision + rationale (ADR-format → `docs/adr/0003`)

**Adopt B + C + D as defense-in-depth. Reject A-alone.**

- **B (Explicit Allow-Flag)** is the primary runtime gate, folded into **both** guard functions so
  it covers all six mint sites (not three):
  - **G.1 — `devLoginAllowed(env)`** returns true only when `ALLOW_DEV_LOGIN === 'true'` **and**
    `DEV_AUTH_SECRET` is non-empty (covers sites 1/4/6). Signature changes `(secret)`→`(env)`.
  - **G.2 — `isDevRequestAuthorized(url, provided, env)`** also requires `ALLOW_DEV_LOGIN === 'true'`
    before the secret check (covers sites 2/3/5 — the `/dev/mock-auth` family). This is the
    breaker-CRITICAL-#2 fix: the `/dev/*` family was gated by *this* function, not by
    `devLoginAllowed`, so the original "one function" fix left it open.
  Default-off means prod is fail-closed across all six sites even if the secret leaks again.
  Explicit + greppable beats an invisible NODE_ENV.
- **C (Dev-Kid Segregation)** is the cryptographic backstop for **future** dev tokens: dev/mock
  tokens are signed under a `dev` kid + dev keypair (C.1 signer change), and the prod verifier
  refuses the dev kid (C.2 — `acceptDevKid` is false on prod). This makes future dev tokens
  prod-rejected by construction. It does **not** reject the already-minted `kid:1` token — that is
  killed by operator key rotation (R-6). (Original C's "reuses existing check, no new sign logic"
  was false — see §3 C.)
- **D (Boot Fail-Fast)** is the detector: `loadEnv()` throws when `NODE_ENV==='production'` AND
  (`ALLOW_DEV_LOGIN==='true'` OR `DEV_AUTH_SECRET` is set). Turns the next misconfig into a visible
  failed rollout instead of a silent backdoor.
- **NODE_ENV** is used **only inside D's invariant** (where a wrong value merely weakens a backstop),
  **never as the sole gate** (per the §2 reliability finding). This is the explicit answer to the
  "NODE_ENV-only is worthless" blind spot.

**Why this is "boring & proven, smallest that holds":** the change lives in existing seams — the
two dev guard functions (`devLoginAllowed` + `isDevRequestAuthorized`, both in `dev-guard.ts`), the
signer/verifier (`jwt.ts`, minimal kid-arg threading), and `loadEnv()` (existing fail-fast pattern).
No new infrastructure. We add `ALLOW_DEV_LOGIN` (one flag, both guards read it) + an optional dev
keypair/kid (`JWT_DEV_*`), all default-absent → prod-safe.

Also in scope (per mandate, not a separate option):
- **Remove hardcoded creds + delete the 2nd cred pair** (item 3, resolved): move `test@dowiz.com`
  out of the shipped literal into env/test fixtures; **delete** the `routes/auth/local.ts` dev-bypass
  branch entirely (both cred pairs incl. `empty@dowiz.com`), leaving `local.ts` a pure real-argon2
  login. Exactly **one** dev-bypass path remains (the inline handler). Decision is **delete, not
  align** (resolves MED #7). Delete the false "always rejects in prod" comments (`server.ts:871-873`,
  `local.ts:42`) rather than soften them (counsel honesty point).
- **Rate-limit + Zod schema** (item 4, resolved — see §7): add a Zod body schema unconditionally;
  for rate-limit, **exempt-when-gate-open** for the dev bypass (prod has no live dev path to
  brute-force; non-prod has no real secret worth guessing) and key the **real argon2 path** by
  **`email + IP`** at ~10/min — hardens prod online-guessing without 429-flaking CI's single
  runner IP (resolves HIGH #4). NOT a blanket 5/min IP cap.

This does not contradict any existing ADR. It **extends** ADR-0001-era conventions (Postgres-first,
fail-fast boot) and the `dev-guard` fail-closed intent (which was correct in spirit but
under-specified — `!!secret` was never sufficient).

---

## 5. Data / migrations

**None.** No schema change, no table, no data migration. The dev keypair (Option C) is **key
material delivered via Fly secrets / CI env**, not stored in the DB.
- Forward-only: N/A (no migration).
- RLS FORCE: N/A (no new tenant table). The existing RLS posture is untouched.
- Integer-money: N/A.

Stated explicitly so the implementer does not invent a migration.

---

## 6. Consistency + idempotency

- **No persistent state** is created by dev-login; it only reads `users`/`memberships` and mints a
  JWT. There is nothing to make idempotent in the auth path itself.
- **Idempotency of the fix:** the gate is a **pure function of env** — same env in, same boolean
  out, on every machine/process (web + worker). No per-request state, no drift between the web and
  worker processes (both load the same `loadEnv()`).
- **The real consistency risk is the escalation, not the login:** `POST /onboarding/start` creating
  org+location must remain idempotent/guarded on its own merits. Closing the dev-login gate removes
  the *anonymous* path to it; the onboarding endpoint's own guards are unchanged and out of scope.
- **Server stays authoritative:** the fix does not move any trust to the client; the token's
  authority (role, kid) is decided server-side as today.

---

## 7. Failure modes + degradation

Per failure-first: design the failure paths before the happy path.

| Scenario | Behaviour under this design |
|---|---|
| **Prod, secret accidentally re-set, flag unset (today's leak class)** | **Both** guards return false (G.1 + G.2). `/api/auth/local/login` → 401; `/api/dev/mock-auth` family → 404. **All six sites fail-closed.** |
| **Prod, secret set AND flag set true (worst operator error)** | `loadEnv()` **throws at boot** (D, firing on prod's per-app `NODE_ENV=production` secret) → app fails boot; AND the `release_command` NODE_ENV assert (§9 R3) runs **before** that artifact serves → Fly aborts rollout → old (safe) code keeps serving. Loud, detectable <1 min. |
| **Prod deploy after operator unsets `DEV_AUTH_SECRET` (immediate mitigation)** | If pipeline already rewired (§9.A step 1): green via unauthenticated prod smoke + staging gate. If NOT yet rewired: the four prod mock-auth E2E steps go 404 → red post-deploy validation, but `flyctl deploy` itself still succeeds (code ships). Sequence step 1 first to stay green (R-11). |
| **Prod's `NODE_ENV` secret dropped/never set (unset)** | App won't boot (enum required, no default). Independently, the `release_command` NODE_ENV assert (§9 R3) exits nonzero **before traffic** → Fly aborts rollout, old code serves. Safe. |
| **Prod's `NODE_ENV` wrongly `development` (fat-finger, NOT unset — the R-2 residual)** | The `release_command` assert (§9 R3) sees `FLY_APP_NAME==='dowiz'` AND `NODE_ENV!=='production'` → **exits nonzero before the new artifact serves** → Fly aborts the release, old (production-mode) code keeps serving. This is the inverse-direction gate D could not provide (D only fires when NODE_ENV *is* production). Closed pre-traffic. (Prior R2 text wrongly treated this as caught by a post-deploy CI assert — that ran after traffic; corrected R3.) |
| **Staging forgets `NODE_ENV=development` (now that image has no NODE_ENV default)** | Staging boot fails the enum (NODE_ENV unset) → loud, caught in staging deploy. NOT defaulted to production (image is NODE_ENV-agnostic per R2 fix) → no silent prod-mode-on-staging. Operator sets the staging secret. |
| **A FUTURE dev-kid token presented to prod** | Prod verifier refuses the dev kid (C.2 — `acceptDevKid` false on prod; dev keypair absent) → 401. Prod-rejected by construction. |
| **The ALREADY-MINTED `kid:1` leaked owner token presented to prod** | **Verifies** (it bears the prod header kid). C does NOT reject it. Killed only by operator JWT key rotation (R-6) → all `kid:1` tokens invalid. Until rotation, lives to its 24h exp. **Mandatory rotation, not optional.** |
| **NODE_ENV unset on prod** | App **won't boot** (config enum is required, no default). Existing fail-fast. Dev-login can't be exploited on a non-running app. |
| **NODE_ENV is `'staging'` (invalid value)** | `loadEnv()` throws — the enum has no `staging`. Boot fails; safe. (Implication: anyone introducing a `staging` NODE_ENV must also add it to the enum **and** to D's prod-check — flagged as a risk.) |
| **NODE_ENV mistakenly `'production'` on staging** | D would fire and **staging would refuse to boot with the dev flag set** — loud, not silent. Operator sets staging NODE_ENV to `development`. With the R2 fix (image is NODE_ENV-agnostic, value is a per-app Fly secret), this requires an active wrong-value secret on staging, not merely a forgotten override — less likely than under the rejected Dockerfile-pin design. Documented in §9. |
| **A deploy forgets `NODE_ENV=production` on prod** | Two outcomes, both safe: unset → boot fails (won't serve); set to `development` → B still gates on the flag (default off) so dev-login stays closed unless the flag is *also* wrongly set, in which case **D cannot fire** (NODE_ENV≠production) — this is the one residual gap, mitigated by B's default-off + C's kid rejection. Accepted risk R-2. |
| **CI runs ~14 specs from one runner IP** | Dev bypass is **exempt** from a per-route cap (prod has no live dev path; non-prod has no real secret to guess) → no 429 flake. Real argon2 path keyed by `email+IP` (~10/min), bounded by the global 100/min IP cap. (Resolves HIGH #4.) |
| **Rate-limiter store unavailable** | **SPEC (R2-5b, not advisory):** the real-login per-route limiter **fails-closed (rejects)** when its store is unavailable — proof obligation, not a "should". The exempt-when-gate-open dev path has **no per-route limiter to fail** (it relies only on the global IP cap), so store-failure does not flake CI. No cascade — single route, no downstream calls. |

**No external calls** are added by this design, so there is no new timeout/circuit-breaker surface;
the only "external" dependency is the rate-limiter store, handled above.

---

## 8. Security + tenant isolation

- **Escalation chain closed at the source:** with dev-login fail-closed (B) and dev tokens
  kid-rejected (C), the anonymous path to `role:'owner'` → `/onboarding/start` → org/location
  creation is severed. Tenant isolation (RLS) was never the failure here — the failure was a forged
  *authenticated* identity; C makes the forgery cryptographically invalid on prod.
- **Already-minted leaked token (honest, corrected):** Option C does **NOT** invalidate it — the
  leaked owner token bears the prod header kid (`kid:1`), so the prod verifier accepts it. The
  **only** kill for the already-minted token is the **operator's mandatory JWT key rotation** (new
  `JWT_KID`), which invalidates *all* tokens minted under the old key. C handles **future** dev
  tokens (prod-rejected by construction); rotation handles the **existing** leak. **Both are
  required, not either** (R-6 is mandatory, not belt-and-suspenders). Note: rotation kills the
  *token* but does **not** un-create org/location/membership rows it may have written — those are a
  forensic question (counsel STOP-1, NEEDS-HUMAN-DECISION; see resolution.md).
- **Key/kid strategy:** prod keeps its own `JWT_KID` (e.g. rotate `1`→`2` on remediation). Dev/CI
  sign under a distinct `dev` kid + dev keypair. Prod verifier (`getKid()` == prod kid) rejects any
  `dev`-kid token. Staging may either share prod's verifier kid (then C gives no staging↔prod
  isolation but still rejects on prod — which is the goal) or run its own; operator decides.
- **Credentials out of code:** hardcoded `test@`/`empty@` creds removed from shipped artifacts;
  sourced from env/test fixtures. Closes the "secret in git" anti-pattern even though these were
  test creds — they were also a working prod password while the gate was open.
- **No PII, no cookies, JWT RS256-only** — all unchanged.

---

## 9. Operability

### NODE_ENV contract (who sets it, what value) — resolves HIGH #3 + MED #8 + R2 NEW HIGH (Dockerfile stage)
The breaker proved `NODE_ENV` is absent from every Dockerfile / `fly.toml` / `ci.yml` (CI injects
`development` in `scripts/verify-fresh-provision.sh:79`).

**R2 correction — there is NO prod-specific runtime stage to pin.** The original §9 said "pin
`ENV NODE_ENV=production` in the Dockerfile prod runtime stage." Verified false: `Dockerfile` has
exactly two `FROM` lines — `builder` (L2) and **one unnamed runtime stage** (L30) — and the *same*
image deploys to both prod (`dowiz`) and staging (`dowiz-staging`) (`flyctl deploy -a dowiz-staging`).
Pinning `ENV NODE_ENV=production` in that single shared stage would default **every** box (staging,
CI-built images, local `docker run`) to `production`, so a forgotten staging override triggers D's
boot-throw — converting a missing-env-var into a staging/CI **availability regression** on a shared
image. That is the wrong place for the pin.

**Resolved: prod NODE_ENV is a per-app Fly value, NOT a Dockerfile ENV.** The image stays
NODE_ENV-agnostic (Dockerfile sets NO `NODE_ENV` — unchanged). Each app sets it explicitly per-app,
and a **CI/deploy assertion verifies the prod value before serving** (replacing the "in-repo greppable
ENV" property we lost):

| Env | Required NODE_ENV | Set by | Verified by |
|---|---|---|---|
| **prod** (`dowiz`) | `production` | `fly secrets set NODE_ENV=production -a dowiz` (operator, one-time, recorded in runbook) | **`release_command` asserts it PRE-TRAFFIC** (see R3 below); mismatch = nonzero exit → Fly aborts the release → old code keeps serving. Plus the startup log line `dev-login: DISABLED`. |
| **staging** (`dowiz-staging`) | `development` | `fly secrets set NODE_ENV=development -a dowiz-staging` (operator) | staging boot + E2E (dev-login must stay ON) |
| **CI fresh-provision** | `development` | `scripts/verify-fresh-provision.sh:79` (unchanged) | fresh-provision job |
| **local** | `development` | `.env` (`.env.example:1`) | n/a |

**R3 — the deploy-assert must be PRE-TRAFFIC, not a post-deploy CI step (R2-2 NEW MEDIUM corrected).**
The R2 text "deploy job asserts `fly ssh console -a dowiz -C 'printenv NODE_ENV' == production` (or a
`/api/health` field) before running post-deploy validation" was **fail-open / post-hoc** against real
source — verified this round:
- **The `/api/health` fallback does not exist and must not be added.** `apps/api/src/routes/health.ts`
  returns `{ status, timestamp, checks }` with **no environment field** (verified — `grep NODE_ENV` =
  zero). Adding a `NODE_ENV` field would **reverse the deliberate recon-leak hardening** the file
  documents at `:37-38` and `:320-323` ("`/health` is unauthenticated … raw driver text … is a recon
  leak"; "public payload is minimal by construction"). Advertising the environment posture on an
  unauthenticated endpoint is exactly what that hardening forbids. **Rejected.**
- **A `fly ssh console` CI step runs AFTER `flyctl deploy`** has already swapped the new machine into
  service (`ci.yml:151` deploy → then E2E steps). For NODE_ENV **unset**, the app won't boot (config
  enum, no default) and `flyctl deploy`'s own health phase fails the release — but that is the existing
  boot-enum guard, not the new assert. For NODE_ENV wrongly **`development`** (the R-2 residual,
  fat-finger), the app **boots, serves traffic, the deploy succeeds**, and a *post-deploy* assert only
  reds the CI job — with no automatic rollback, the development-mode artifact is **already live**. So
  the assert as spec'd does **not** gate the rollout for the wrong-value case.

**Resolved — assert NODE_ENV inside `release_command` (the real pre-serving gate).** `fly.toml` already
has `[deploy] release_command = "dist/migrate/index.cjs"` (verified `:14-15`), which Fly runs in a
one-off machine **before** the new machine takes traffic; a **nonzero exit aborts the release** (the
same fail-safe the schema-drift guard relies on — old code keeps serving). Add a pre-migrate assertion
**in the migrator entrypoint** (`dist/migrate`, built from `scripts/build-apps.ts`):
```
// at the top of the migrate entrypoint, BEFORE applying migrations:
if (process.env.NODE_ENV !== 'production') {
  console.error(`FATAL release_command: NODE_ENV must be 'production' on prod, got '${process.env.NODE_ENV ?? '(unset)'}'`);
  process.exit(1);   // nonzero → Fly aborts the release, new artifact never serves
}
```
This runs in the prod app's `release_command` (the prod app's secrets/env are in scope there), exits
nonzero on a wrong/missing prod NODE_ENV, and **aborts the rollout before any traffic** — old code keeps
serving. It is genuinely pre-traffic, needs no `/health` field, and is checked on **every** deploy
(stronger than greppable). **Staging reconciliation (shared image):** the migrator runs on staging too
(same `release_command`), and staging legitimately runs `NODE_ENV=development`. So the assertion must
**not** unconditionally demand `production` — it demands `production` **only when a prod marker is
present**. The marker is the prod app identity, available in the `release_command` env as Fly's
`FLY_APP_NAME` (set by the platform): assert `NODE_ENV==='production'` **iff `FLY_APP_NAME === 'dowiz'`**;
on `dowiz-staging` the check is inert (and a separate inverse-direction line may assert
`NODE_ENV !== 'production'` when `FLY_APP_NAME === 'dowiz-staging'`, catching a prod-mode-on-staging
fat-finger pre-traffic too). This is the **inverse-direction fail-closed boot condition** the finding
asked for: D (boot-guard) already fail-fasts the **dangerous** direction (NODE_ENV=production + dev
flag/secret set → throw before listen); the **inverse** gap (prod NODE_ENV ≠ production, so D never
fires and the /dev/* closure the design leans on stays open) is now closed by the `release_command`
asserting prod's NODE_ENV **is** production before the prod artifact ever serves.

Why `release_command` beats the rejected options: option (a) `/health` field — rejected (reverses recon
hardening, runs post-traffic anyway); option (b') a pure boot-sequence assert in `server.ts` — would
fire on every box including the worker process and any local `docker run`, and a failed app-boot does
not abort a Fly release as cleanly as a failed `release_command` (a crash-looping new machine vs. a
clean rollout-abort with old code serving). `release_command` is the **one** hook that (1) runs once,
(2) on the target app with that app's env, (3) **before** traffic, and (4) aborts the release on
nonzero exit — exactly the pre-serving gate the design needs.

Why per-app-secret + `release_command` assert beats the Dockerfile pin: the image is shared, so a
Dockerfile default is the **unsafe default for the larger (non-prod) fleet**. A per-app secret keeps
prod=production and staging=development without making `production` the silent default anywhere, and the
**pre-traffic `release_command` assertion** restores the lost "verifiable, not invisible" property —
prod's NODE_ENV is now *checked before every prod deploy serves*, stronger than greppable. The trade
(R-1) flips direction vs. the rejected pin: a forgotten/wrong **prod** NODE_ENV aborts the release (old
code keeps serving); a forgotten **staging** secret leaves staging on `development` (its intended value)
— no availability regression on the non-prod fleet. This is the corrected, reconciled-with-D answer to
the §2 "NODE_ENV is an invisible knob" finding.

**D's prod path is proven everywhere via a unit test** (call the boot-guard with
`NODE_ENV='production'` + each dangerous combo, assert throw) — so it is rehearsed in CI on every
push, not only on a real prod boot (resolves MED #8 "untestable pre-prod"). The enum stays
`[development,test,production]` — no `staging` value (would re-introduce R-4).

### 9.A — Prod deploy validation WITHOUT a live owner-minting backdoor (R2 NEW CRITICAL) — resolves the core tension
**The conflict, stated plainly:** "prod fails closed for `/dev/*`" (G.2) and "the prod deploy job
authenticates via prod `/api/dev/mock-auth`" (ci.yml:158-184, four steps) are mutually exclusive.
The moment the operator unsets the prod `DEV_AUTH_SECRET` (the immediate mitigation), **the next push
to main runs `deploy-validation.spec.ts:13` → `POST https://dowiz.fly.dev/api/dev/mock-auth` → 404 →
red deploy** — even before G.2 ships, because the secret is what those steps depend on. So the deploy
pipeline redesign is **part of this remediation, not a follow-up**: prod can be EITHER backdoored OR
unvalidated by the current pipeline, never both safe; we must change how prod is validated.

**Options weighed:**
- **(a) Prod smoke = unauthenticated only.** Post-deploy prod validation asserts only health,
  storefront-read (`/s/:slug` SSR), and the **negative** auth assertions that already exist
  (`deploy-validation.spec.ts:1.1-1.3` already assert 401 on owner/courier/customer endpoints
  *without* a token — these need NO backdoor). *Pro:* zero owner-minting on prod; reuses existing
  negative tests; smallest prod surface. *Con:* prod no longer E2E-exercises authenticated owner flows
  — those move to staging (option b covers it).
- **(b) Full mock-auth E2E runs ONLY against staging; gate prod deploy on the staging run.** The
  authenticated lifecycle suites (`flow-core-lifecycles`, telegram) run against `dowiz-staging` (which
  *keeps* dev-login ON) as a **pre-prod gate**; prod deploy proceeds only if staging E2E is green, then
  prod gets the (a) smoke. *Pro:* full auth coverage preserved, on the env that legitimately has the
  backdoor; prod deploy still gated on real authenticated behavior. *Con:* validates the *image* on
  staging, not the literal prod box — acceptable because it is the **same image** (one Dockerfile),
  and prod-specific config is covered by the (a) smoke + the NODE_ENV deploy-assert.
- **(c) Ephemeral prod validation tenant via legitimate authenticated APIs.** Provision+tear-down a
  real owner via the genuine signup/onboarding path, no dev bypass. *Pro:* truest prod signal.
  *Con:* requires a real-credential signup flow callable in CI, writes real rows to prod on every
  deploy (provenance noise — directly fights the STOP-1 forensics), teardown-failure leaves orphan
  prod tenants. **Rejected** — over-engineered and pollutes the exact tables the incident is
  forensically examining.
- **(d) Separate time-boxed narrow validation token mechanism on prod.** A distinct, short-TTL,
  read-scoped validation credential separate from the owner backdoor. *Pro:* some authenticated prod
  signal. *Con:* it is *still a credential-minting bypass on prod* — a smaller version of the exact
  thing we are removing; re-introduces a "fail-closed by assumption" path (counsel's regenerating
  pattern). **Rejected** — violates the design's own goal.

**DECISION: (a) + (b).** Prod gets an unauthenticated smoke (health + storefront-read + the existing
negative-auth 401 assertions — no token minted on prod). Full authenticated lifecycle E2E moves to a
**staging gate that runs BEFORE prod deploy**. Concept: *validate the authenticated image on the env
that legitimately holds the dev backdoor (staging); validate prod liveness with zero-privilege probes.*

**CI workflow change (`ci.yml`):**
1. New job `staging-e2e` (or extend `validate`): `needs: validate`, on push-to-main, deploys the
   built image to `dowiz-staging`, runs `flow-core-lifecycles` + `telegram-*` against
   `https://dowiz-staging.fly.dev` with `DEV_AUTH_SECRET=stg-e2e-secret` + `ALLOW_DEV_LOGIN=true`.
   These are the *authenticated* suites — they keep their current mock-auth calls **unchanged**
   (staging has the backdoor). This is the prod-deploy gate.
2. `deploy` job: `needs: [validate, staging-e2e]`. Migrate (`release_command`) → assert prod
   `NODE_ENV==production` **inside `release_command`** (§9, pre-traffic) → deploy prod → run a **prod
   smoke** spec that is unauthenticated only. Remove the `DEV_AUTH_SECRET` env from the prod
   deploy-job E2E steps (no longer needed; must not be present on a prod step). The four current prod
   steps collapse to: one unauthenticated prod smoke (no secret) + the authenticated work done earlier
   on staging.

   **R3 — the spec-split is a SOURCE-EDIT, not a CI-config change (R2-1 NEW HIGH corrected).** The R2
   text "split `deploy-validation.spec.ts` … keep negative-auth + health + storefront as the prod
   smoke; move test 0.1 + authenticated → staging; specs unchanged" was **wrong against real source**.
   Verified (this round) against the four prod-targeted spec files:
   - `e2e/tests/deploy-validation.spec.ts` is `test.describe.configure({ mode: 'serial' })` (`:8`) with
     **22 tests**. Test `0.1` (`:13`) assigns the module-level `authToken`; **10 tests carry
     `Authorization: Bearer ${authToken}`**. Only **1.1/1.2/1.3** (`:28/:35/:40`) are truly
     unauthenticated 401 negatives. The "storefront" test `3.1` (`:66`) is **NOT a public read** — it
     calls the **authenticated** `/api/owner/settings` (`:67`, `Bearer ${authToken}`) to *obtain*
     `locationSlug`, then probes the public menu/theme. So `3.1/5.1/9.1/12.1/13.1` all depend on
     `authToken`/`locationSlug` that only test 0.1 + 3.1 populate. **Serial shared-state makes the
     "keep" subset non-portable** — pulling 0.1 strands the chain.
   - `e2e/tests/flow-core-lifecycles.spec.ts:4` reads `process.env.VITE_BASE_URL || 'https://dowiz.fly.dev'`
     — **env-driven**, can target staging by env. Authenticated → moves to staging suite, no edit.
   - `e2e/tests/telegram-webhook.spec.ts:3` and `telegram-full-flow.spec.ts:3` are **`const BASE =
     'https://dowiz.fly.dev'`** with **NO `VITE_BASE_URL` fallback**. Setting the env on `staging-e2e`
     has **zero effect** — they hammer prod regardless. They authenticate via `/api/dev/mock-auth`
     (`telegram-full-flow:51`). After step 3 closes the prod backdoor, these two go 404 against prod.

   **Re-specified concretely (the honest, enlarged E2E delta):**
   - **(1) Prod unauth smoke — a NEW standalone non-serial spec** (e.g. `prod-smoke.spec.ts`), env-driven
     (`process.env.VITE_BASE_URL || 'https://dowiz.fly.dev'`), mints **no token**. Exact assertions:
     `GET /livez` → 200 and `GET /health` → 200/503 by liveness (no NODE_ENV field — see §9); a **public
     storefront read** via the SSR route `GET /s/:slug` (or the public API `GET /public/locations/:slug/menu`
     + `GET /api/public/theme/:slug`) against a **known seeded public slug passed by env**
     (`PROD_SMOKE_SLUG`) — NOT obtained from `/api/owner/settings`; and the three negatives **extracted**
     from `deploy-validation.spec.ts:1.1–1.3` (`GET /api/owner/locations` → 401, `/api/courier/me/assignments`
     → 401, `/api/customer/orders` → 401). These three are non-serial and need no backdoor — but they are
     **3 of 22 tests** in a serial file, so they must be **lifted into the new standalone spec** to be
     portable. The old `deploy-validation.spec.ts` is **retired from the prod job** (it keeps running on
     staging where the backdoor lives, unchanged).
   - **(2) Telegram specs MUST be edited** to read `const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev'`
     (one line each) so the `staging-e2e` job actually points them at staging. **The proposal's earlier
     "specs unchanged / no spec change" claim was wrong — list of spec files that MUST change:**
     `e2e/tests/telegram-webhook.spec.ts` (BASE → env), `e2e/tests/telegram-full-flow.spec.ts` (BASE →
     env), **NEW** `e2e/tests/prod-smoke.spec.ts` (created), `e2e/tests/deploy-validation.spec.ts`
     (no longer in the prod job — re-pointed to staging only, optionally trimmed). `flow-core-lifecycles.spec.ts`
     needs **no** source edit (already env-driven).
   - **(3) The `staging-e2e` gate runs the FULL authenticated suite** (`flow-core-lifecycles` +
     edited `telegram-*` + `deploy-validation`) against `dowiz-staging` **before** the prod deploy job,
     with `ALLOW_DEV_LOGIN=true` + `DEV_AUTH_SECRET=stg-e2e-secret` + the dev keypair. Prod deploy
     proceeds only if this is green.

**ORDER OF OPERATIONS (so prod is never both backdoored AND unvalidated):**
1. **First (code, this PR):** split the validation specs (prod-smoke = unauthenticated; authenticated
   → staging suite) and rewire `ci.yml` (`staging-e2e` gate + prod-smoke). This is the *enabling*
   change and ships dark — it does not yet close anything on prod. After this, the prod deploy job
   no longer requires the prod backdoor.
2. **Second:** ship G.1+G.2+C+D behind the flag (prod flag stays absent/false; D will throw if the
   secret is still set on prod — so step 3 must precede a prod deploy of this code).
3. **Third (operator, coordinated with step 2's prod deploy):** unset prod `DEV_AUTH_SECRET` **and**
   set/verify prod `NODE_ENV=production`, **then** deploy step-2's code. Because the pipeline was
   already rewired in step 1, this deploy validates green via the unauthenticated prod smoke + the
   pre-deploy staging gate — prod is validated *without* a backdoor.
4. **Fourth:** operator rotates the prod JWT kid (R-6) to kill the already-minted leaked token.
   Forensics (STOP-1) run **before** any cred/user-row cleanup (see resolution).

**What breaks the moment the operator unsets the prod secret BEFORE step 1 lands:** the four prod
deploy-job E2E steps go red (mock-auth 404). Therefore the operator's immediate mitigation (unset
prod secret) MUST be sequenced AFTER step 1 (pipeline rewired) — OR, if the secret is unset
*immediately* for incident reasons, the operator must accept that the prod deploy job is red until
step 1 lands (deploys still happen — `flyctl deploy` succeeds; only the post-deploy validation step
fails). Stated explicitly: **unsetting the secret first = red-but-deploying pipeline until step 1;
landing step 1 first = clean throughout.** Recommend step 1 first. This is a tracked dependency, owner
below (R-11).

- **Staging keeps E2E working:** set `ALLOW_DEV_LOGIN=true` + keep `DEV_AUTH_SECRET=stg-e2e-secret`
  + `NODE_ENV=development` on `dowiz-staging`. Provision `JWT_DEV_KID` + dev keypair so C.2-verified
  dev-kid tokens validate on staging.
- **CI keeps working:** `ci.yml` fresh-provision env-file adds `ALLOW_DEV_LOGIN=true` next to the
  existing `DEV_AUTH_SECRET=ci-fresh-secret` (line 110); the post-deploy E2E steps passing
  `DEV_AUTH_SECRET` are unchanged (the `x-dev-auth-secret` protocol is untouched). The C dev keypair
  + `JWT_DEV_KID=dev` are added to the CI env-file (throwaway, like the existing keypair gen at
  ci.yml:112-118).
- **Misconfig detection (<1 min):** D's boot-throw surfaces as a failed Fly release (rollout
  aborted, old code serves) — visible in deploy logs immediately. Add a one-line
  startup log of the effective dev-login state (`dev-login: DISABLED` in prod) for positive
  confirmation in logs/observability.
- **Rollback:** pure config + small code change; revert is a redeploy. No data to roll back.
- **Scaling/flag gate:** `ALLOW_DEV_LOGIN` is itself the flag — default off; flipping it on prod is
  blocked by D. No scaling implications.
- **Operator runbook delta:** document that staging/CI need `ALLOW_DEV_LOGIN=true`; prod must have
  it absent/false; rotating JWT keys on suspected leak is the kill-switch for minted tokens.

---

## 10. Open / accepted risks

| ID | Risk | Disposition | Owner |
|---|---|---|---|
| R-1 | NODE_ENV is now a **per-app Fly secret** (R2: image is NODE_ENV-agnostic — NO Dockerfile pin, the assumed prod runtime stage does not exist). Prod must have `NODE_ENV=production`, staging `=development`. | **Verify + set per-app.** Prod value asserted **pre-traffic in `release_command`** (R3 §9) — nonzero exit aborts the release, old code serves; this catches the wrong-value (`development`) case the prior post-deploy CI assert missed. Staging unset → boot fails (caught in staging deploy). No silent prod-default on the shared image. | Operator + Implementer |
| R-2 | If prod's NODE_ENV is ever wrongly `development` AND the flag is wrongly set, D cannot fire (NODE_ENV≠production). | **Accepted, mitigated** by B default-off + C kid-rejection. Residual = handler runs but tokens are prod-rejected. | Architect |
| R-3 | Sharing one `dev` kid between staging and CI means a staging-minted dev token verifies in CI and vice-versa. | **Accepted** — both are non-prod; the security boundary is prod-rejection, which holds. | Architect |
| R-4 | A future `staging` NODE_ENV value would need adding to the config enum AND to D's check, or boot breaks / D mis-scopes. | **Open** — documented; gate any such change through this ADR. | Architect |
| R-5 | Removing the `local.ts` duplicate could break a spec that hits it directly. | **Verify** which specs call `/api/auth/local/login` vs `/dev/mock-auth`; align before deleting. | Implementer |
| R-6 | The already-minted leaked `kid:1` owner token (1d TTL) is NOT rejected by C (it bears the prod kid). | **Accept-risk, MANDATORY mitigation** — operator MUST rotate the prod JWT key/kid; this is the sole kill for the existing token (no owner-token session table to revoke against). Design prevents recurrence; rotation kills the live leak. | Operator |
| R-7 | Forensics: was the live backdoor used? Org/location/membership rows survive key rotation. | **NEEDS-HUMAN-DECISION** (counsel STOP-1) — exact `SELECT count(*)` queries in resolution.md; run before declaring "closed" or record accepted residual uncertainty. | Operator (human) |
| R-8 | Disclosure obligation if real PII was in blast radius during the live window. | **NEEDS-HUMAN-DECISION** (counsel STOP-2) — depends on R-7 + the open question; record decision in `/compliance`. | Operator / data-steward (human) |
| R-9 | Was prod dark / any real users + paid orders in the exposure window? (Determines CRITICAL-with-victims vs near-miss.) | **NEEDS-HUMAN-DECISION** (counsel open question) — count query in resolution.md (R2-corrected: run as BYPASSRLS role, join customers for phone, filter on paid status); operator defines window-start = when prod secret was first set. | Operator (human) |
| R-10 | C's prod-rejection rests on the dev keypair being absent from prod — same copy-paste guarantee class that already failed for `DEV_AUTH_SECRET`. | **ACCEPT-RISK, reduced.** Lower residual than the secret (distinct artifact from prod key; D fails-fast if `JWT_DEV_KID` is on a NODE_ENV=production box — catches the cheap half). Fail-open needs BOTH a pasted keypair AND wrong prod NODE_ENV. | Operator (hygiene) + Architect (D) |
| R-11 | Prod deploy validation depends on the prod `/api/dev/mock-auth` backdoor (4 CI steps). Closing it / unsetting the secret turns the prod deploy job red until the pipeline is rewired. | **FIX (in scope, §9.A).** Sequence: rewire CI (prod smoke = unauthenticated + staging gate) **first**, then close prod + unset secret. If secret unset first for incident reasons, deploys still ship; post-deploy validation is red until rewire lands. | Implementer (pipeline) + Operator (sequencing) |
| R-12 | The §9.A spec-split is a **source rewrite, not a CI-config tweak** (R3): 2 telegram specs hardcode `dowiz.fly.dev` (must be edited to read `VITE_BASE_URL`), `deploy-validation.spec.ts` is a 22-test serial auth-chain (only 3 negatives portable), and the prod smoke is a **new standalone spec** reading a seeded public slug. Of ~50 prod-targeted tests, ~3 reusable as-is. | **FIX (in scope, §9.A R3).** Enlarges the E2E work; proof obligation added (prod-smoke spec passes with no secret; edited telegram specs prove they hit staging). | Implementer (E2E) |
| R-13 | The NODE_ENV pre-traffic gate lives in the **migrator entrypoint** (`release_command`), keyed on `FLY_APP_NAME==='dowiz'`. If Fly does not populate `FLY_APP_NAME` in the `release_command` env, the prod-marker check is inert and the inverse gate silently no-ops. | **Verify** `FLY_APP_NAME` is present in the `release_command` env (Fly platform-set); if absent, substitute an explicit env marker set as a prod-only Fly secret (e.g. `DEPLOY_TARGET=prod`). Proof obligation covers both exit paths. | Implementer (verify) + Operator (marker secret if needed) |

---

## Proof obligations (for the implementer — not part of this design doc)
Per the Mandatory Proof Rule, the eventual implementation must include:
- API assertion — **both** families: `POST /api/auth/local/login` (test creds) AND
  `POST /api/dev/mock-auth {role:'owner'}` (with the secret header) → **401/404** on a prod-config
  build (NODE_ENV=production, flag off), and **200** on a staging-config build (flag on). The
  mock-auth case is the breaker-CRITICAL-#2 regression guard.
- Unit on `devLoginAllowed(env)` (G.1): flag off → false; flag on + secret → true; flag on + no
  secret → false. Unit on `isDevRequestAuthorized` (G.2): `/dev/*` with flag off → false even with a
  matching secret.
- Boot test: `loadEnv()` throws when NODE_ENV=production AND (flag set OR secret set OR `JWT_DEV_KID`
  set) — the unit rehearsal of D's prod path (resolves "D untestable pre-prod").
- Verifier test (C.1+C.2): a token whose **protected header** kid = `JWT_DEV_KID`, minted by C.1, is
  **accepted** by a non-prod verifier and **rejected** by a verifier with `NODE_ENV=production`.
  (Proves C is actually wired into the header, not just the body claim.)
- Sequencing gate: C is not "done" until the verifier test above passes (LOW #10).
- **C.1 key-path proof (R2):** a dev-mint token must be signed with the **dev private key** (not the
  prod key) — assert it **verifies against the dev public key** and **fails signature** against the
  prod public key. Proves kid AND key moved together (the `signDevToken()` wrapper), not kid alone.
- **Deploy-pipeline proof (R2 CRITICAL + R3 HIGH):** the **NEW standalone** `prod-smoke.spec.ts`
  (env-driven, non-serial) passes against a prod-config build with **no `DEV_AUTH_SECRET`** — asserts
  `/livez` + `/health` + a **public `/s/:slug` read by a seeded `PROD_SMOKE_SLUG`** (NOT via
  `/api/owner/settings`) + the **extracted** 1.1–1.3 401 negatives, minting no token. Separately, the
  **edited** `telegram-webhook.spec.ts` / `telegram-full-flow.spec.ts` (now reading `VITE_BASE_URL`) +
  `flow-core-lifecycles.spec.ts` + `deploy-validation.spec.ts` pass against **staging-config** (flag on,
  `VITE_BASE_URL=https://dowiz-staging.fly.dev`) — proving they actually hit staging and not prod. Prove
  the prod deploy job is green with the prod backdoor closed AND that no prod-targeted step still calls
  `/api/dev/*` — the regression guard for the R2 CRITICAL and the R3 spec-split HIGH.
- **NODE_ENV pre-traffic assert proof (R2 HIGH + R3 MEDIUM):** a test of the migrator entrypoint
  (`dist/migrate`) asserts it **exits nonzero** when `FLY_APP_NAME==='dowiz'` AND `NODE_ENV!=='production'`
  (both unset and `development` cases), and **exits zero** when `NODE_ENV==='production'` — proving the
  gate runs in `release_command` (pre-traffic), not as a post-deploy CI step, and that no `/health`
  NODE_ENV field is relied upon.
- **Rate-limit store-failure proof (R2-5b):** with the limiter store down, the real-argon2 login route
  returns a reject (429/503), not an allow — and the dev-bypass path is unaffected (no per-route limiter).
