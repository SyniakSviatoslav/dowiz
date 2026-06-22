# ADR 0003: Dev-Login Fail-Closed — Explicit Flag + Dev-Kid + Boot Guard

**Status:** RESOLVED — proposed (awaiting human acceptance; counsel ETHICAL-STOPs → NEEDS-HUMAN-DECISION)
**Resolution:** `docs/design/dev-login-backdoor-hardening/resolution.md`
**Supersedes:** nothing · **Extends:** the `dev-guard` fail-closed intent, the `loadEnv()`
fail-fast boot pattern, and the RS256 `kid`-mismatch rejection in `verifyAuthToken`.
**Companion design:** `docs/design/dev-login-backdoor-hardening/proposal.md`
**Severity:** CONFIRMED live CRITICAL on production.

## Context

The dev-only password backdoor `POST /api/auth/local/login {test@dowiz.com / test123456}` returns a
real `role:'owner'` JWT signed by the **prod** key (`kid:1`) on `dowiz.fly.dev`. The gate
(`apps/api/src/plugins/dev-guard.ts:19`, `devLoginAllowed = !!configuredSecret`) only checks whether
`DEV_AUTH_SECRET` is set — and it is set in prod (leaked from staging). The minted token
self-escalates via `POST /api/owner/onboarding/start` (needs only `role=owner`) to create
org+location+owner membership. Blast radius: full tenant creation on prod.

There are **six** dev/test-identity mint sites gated by **two** broken guards (corrected from the
original "three" by the breaker round): `/api/auth/local/login` inline (`server.ts:888`), the inline
`/api/dev/mock-auth` (`server.ts:650/664/711`, also **writes memberships**), the duplicate
`/dev/mock-auth` (`routes/dev/mock-auth.ts:16/72`), the latent plugin `/auth/local/login`
(`routes/auth/local.ts:108`, with a 2nd cred pair `empty@dowiz.com`), the dev seed/repair/assignment
helpers, and the mock-auth honor (`plugins/auth.ts:66`). Critically, the `/dev/mock-auth` family is
gated by **`isDevRequestAuthorized`** (`dev-guard.ts:47`, also `!!secret`), NOT by `devLoginAllowed`
— so hardening `devLoginAllowed` alone leaves the owner-minting mock-auth path open on a prod that
holds a leaked secret. The inline handler also hardcodes credentials in shipped code, has no rate
limit, and no Zod body schema.

**NODE_ENV is not a trustworthy sole gate here.** The Dockerfile sets no `NODE_ENV` and `fly.toml`
has no `[env]` block, so prod's NODE_ENV comes only from an out-of-band Fly secret that cannot be
verified from the repo. A NODE_ENV-only fix would be coupled to an invisible knob and could
silently break staging E2E (if staging mirrors prod's NODE_ENV) or silently fail open (if prod's
NODE_ENV is lost/wrong).

Constraint: staging (`dowiz-staging`) and CI fresh-provision **must keep** dev-login + `/dev/*`
working for ~14 E2E specs that send `x-dev-auth-secret`.

## Decision

Adopt **defense-in-depth (B + C + D)**, never NODE_ENV alone:

1. **B — Explicit Allow-Flag, folded into BOTH guard functions.** Add `ALLOW_DEV_LOGIN`
   (`'true'|'false'`, default `'false'`).
   - **G.1** `devLoginAllowed(env)` returns true only when `ALLOW_DEV_LOGIN === 'true'` AND
     `DEV_AUTH_SECRET` is non-empty (covers `local/login` inline + plugin bypass + the courier honor).
   - **G.2** `isDevRequestAuthorized(url, provided, env)` also requires `ALLOW_DEV_LOGIN === 'true'`
     before the secret check (covers the `/dev/mock-auth` family + dev helpers).
   Two guards, one shared flag, **six** sites — NOT "one function covers all". Default-off ⇒ prod
   fail-closed across all six even if the secret leaks again.
2. **C — Dev-Kid Segregation (concretely implementable).** The original C was non-implementable:
   `signAuthToken` hardcodes the **protected-header** kid to the env kid (`jwt.ts:37`); the
   `payload.kid` override only reaches the body claim; the verifier checks the header; CI uses
   `JWT_KID=1` = prod kid. So C as written rejected nothing. Resolved via three changes:
   - **C.1 signer (kid AND key together — R2 fix):** the override moves *both* the protected-header
     kid and the signing key as a pair via a single `signDevToken()`/`devSigningParams(env)` helper
     (`{ kid: JWT_DEV_KID, key: JWT_DEV_PRIVATE_KEY }`); non-dev callers pass neither →
     `getKid()`+`getPrivateKey()` unchanged. (Original C.1 moved only the kid, leaving the token
     signed by the prod key — the dev verifier would reject it on signature; fixed by threading the
     key through one helper so kid/key cannot diverge.)
   - **C.2 verifier:** accept the dev kid **only in non-prod** (`NODE_ENV!=='production' &&
     header.kid === JWT_DEV_KID`); prod never accepts it; the dev keypair is absent from prod, so
     prod-rejection rests on key-material isolation (NODE_ENV is a second lock, fails safe).
     **Residual (R2):** key-absence-on-prod is the *same copy-paste guarantee class* that already
     failed for `DEV_AUTH_SECRET`. Accepted as reduced (R-10): the dev keypair is a distinct artifact
     from the prod key, and D fail-fasts on `JWT_DEV_KID` set on a NODE_ENV=production box — so the
     fail-open requires BOTH a pasted keypair AND a wrong prod NODE_ENV (the leak required only one
     copy). Not "by construction" in the absolute sense — "by construction under secret hygiene."
   - **C.3 keys:** non-prod-only `JWT_DEV_KID` + dev keypair (`JWT_DEV_*`); prod has none; the
     `x-dev-auth-secret` protocol is untouched.
   C makes **future** dev tokens prod-rejected by construction. It does **NOT** reject the
   already-minted `kid:1` leaked token (it bears the prod kid) — that is killed only by operator key
   rotation (Open items / R-6).
3. **D — Boot Fail-Fast.** `loadEnv()` throws when `NODE_ENV === 'production'` AND
   (`ALLOW_DEV_LOGIN === 'true'` OR `DEV_AUTH_SECRET` is set OR `JWT_DEV_KID` is set). **Prod NODE_ENV
   is a per-app Fly secret, NOT a Dockerfile ENV (R2 correction).** The Dockerfile has one unnamed
   runtime stage (L30) shared by prod and staging — pinning `ENV NODE_ENV=production` there would
   default the whole non-prod fleet to production and trip D on a forgotten staging override (an
   availability regression). Instead the image stays NODE_ENV-agnostic; prod sets `NODE_ENV=production`
   as a Fly secret.
   **R3 — the prod NODE_ENV assert is PRE-TRAFFIC via `release_command`, not a post-deploy CI step
   (R2-2 MEDIUM correction).** The R2 "deploy job asserts `fly ssh … printenv NODE_ENV` (or a `/health`
   field) before validation" was fail-open: `/health` has **no** NODE_ENV field and adding one reverses
   the deliberate recon-leak hardening (`health.ts:37-38,320-323`); and a `fly ssh` CI step runs **after**
   `flyctl deploy` already swapped the new machine into service — so a wrong (`development`) prod NODE_ENV
   **boots, serves, and the assert only reds the job post-hoc** (no rollback). D fail-fasts the *dangerous*
   direction (NODE_ENV=production + dev flag/secret → throw before listen); the **inverse** gap (prod
   NODE_ENV ≠ production, so D never fires and the `/dev/*` closure stays open) is closed by asserting,
   **inside the prod `release_command`** (`fly.toml:14-15`, runs in a one-off machine **before** traffic;
   nonzero exit aborts the release, old code serves), that `NODE_ENV==='production'` **when
   `FLY_APP_NAME==='dowiz'`** (inert on staging, which legitimately runs `development`; an optional
   inverse line asserts non-production on `dowiz-staging`). This is a real pre-serving gate, needs no
   `/health` field, and is checked on every prod deploy.
   D's prod path is **unit-tested in CI** (call the guard with `NODE_ENV='production'` + each
   dangerous combo), so it is rehearsed without booting a staging box as production.
4. **Deploy-pipeline redesign (R2 NEW CRITICAL — in scope).** The prod deploy job (`ci.yml:127-184`)
   currently authenticates **four** post-deploy E2E steps via prod `/api/dev/mock-auth`, so closing
   `/dev/*` on prod (G.2) / unsetting the prod secret turns the deploy job red. Resolved by splitting
   validation: **prod gets an unauthenticated smoke** (health + a public `/s/:slug` read + the existing
   401 negative-auth assertions — mints no token on prod); **the authenticated lifecycle + telegram
   suites move to a staging gate that runs before prod deploy** (staging keeps the backdoor). Options
   (c) ephemeral prod tenant and (d) a separate prod validation token were rejected (pollutes
   forensics / re-introduces a prod bypass).
   **R3 — the spec-split is a SOURCE rewrite, not a CI-config tweak (R2-1 HIGH correction).** Verified:
   `deploy-validation.spec.ts` is `mode:'serial'` (22 tests, 10 carry `Bearer ${authToken}`, its
   "storefront" test reads the slug from the **authenticated** `/api/owner/settings` — only 1.1–1.3 are
   true unauth negatives); the two telegram specs **hardcode `const BASE='https://dowiz.fly.dev'` with no
   `VITE_BASE_URL` fallback**. So: the prod unauth smoke is a **NEW standalone non-serial spec**
   (`prod-smoke.spec.ts`: `/livez`+`/health`, a public `/s/:slug` read by a seeded `PROD_SMOKE_SLUG`, and
   the **extracted** 1.1–1.3 negatives — they are non-portable inside the serial file); the **telegram
   specs must be edited** to read `VITE_BASE_URL`; `deploy-validation.spec.ts` is retired from the prod
   job and runs on staging; `flow-core-lifecycles.spec.ts` already env-driven, moves with no edit. Of
   ~50 prod-targeted tests, ~3 reuse as-is — this enlarges the E2E work (R-12). Order of operations (so
   prod is never both backdoored AND unvalidated): (1) rewire CI + spec edits/new prod-smoke (dark),
   (2) ship G/C/D behind the flag, (3) operator unsets prod secret + sets/verifies prod NODE_ENV,
   deploy, (4) rotate prod kid (R-6); forensics (STOP-1) before any cred/user-row cleanup.

Additionally (in scope, not optional):
4. Move the `test@dowiz.com` literal out of shipped code into env/test fixtures, and **delete** the
   `routes/auth/local.ts` dev-bypass branch entirely (both cred pairs incl. `empty@dowiz.com`),
   leaving exactly one dev-bypass path (the inline handler). Delete the false "always rejects in
   prod" comments (`server.ts:871-873`, `local.ts:42`) rather than soften them.
5. Add a Zod body schema unconditionally. Rate-limit: **exempt-when-gate-open** for the dev bypass
   (prod has no live dev path; non-prod has no real secret to guess) and key the **real argon2 path**
   by `email + IP` (~10/min) — hardens prod online-guessing without 429-flaking CI's single runner
   IP. NOT a blanket 5/min IP cap (which the breaker showed flakes the 14-spec suite).

## Consequences

**Positive**
- Prod fails closed against the leak class regardless of NODE_ENV reliability (flag default-off).
- **All six** mint sites are covered by one shared flag in two guard functions (G.1+G.2).
- Future dev tokens are prod-rejected by construction (C.1–C.3). (The existing leaked `kid:1` token
  needs operator rotation — see Negative + Open items.)
- Misconfiguration is detectable in <1 min as a failed deploy; D's prod path is unit-tested in CI.
- No schema change, no migration, no new infrastructure; one new flag + one optional dev keypair.

**Negative / costs**
- Two non-prod environments (staging + CI) must set `ALLOW_DEV_LOGIN=true`, set
  `NODE_ENV=development`, and provision `JWT_DEV_KID` + a dev keypair.
- NODE_ENV is a **per-app Fly secret** (R2): prod=`production` (deploy-asserted), staging/CI=`development`.
  The image is NODE_ENV-agnostic (no Dockerfile ENV) — a forgotten prod secret is caught by the
  deploy-assert (red deploy); a forgotten staging secret fails the enum at boot (caught in staging
  deploy). No silent `production` default on the shared image.
- **The deploy pipeline changes are part of this change** (R2 CRITICAL): prod validation moves to an
  unauthenticated smoke + a pre-prod staging gate. ~4 prod CI steps re-pointed off the backdoor;
  `deploy-validation.spec.ts` is split (prod-smoke vs. staging-authenticated).
- **The already-minted `kid:1` leaked owner token is NOT killed by this design** — only by operator
  key rotation (Open items / R-6). Org/location/membership rows it may have written survive rotation
  (forensic question → NEEDS-HUMAN-DECISION).
- C's prod-rejection is "by construction under secret hygiene," not absolute — dev-keypair-on-prod is
  the same copy-paste hazard that already failed once (R-10, accepted as reduced).

**Data / migrations:** none. Dev keypair is key material via Fly secrets / CI env, not DB-stored.
No RLS change (no new tenant table); existing RLS posture untouched.

## Alternatives considered (rejected)

- **Option A — NODE_ENV-only gate.** Rejected as sole mechanism: coupled to an invisible,
  repo-invisible Fly setting; can break staging E2E or fail open; cannot reject already-minted
  tokens. (Used only inside D's invariant.)
- **Status quo `!!secret`.** Rejected: this is the live CRITICAL.

## Verification (proof obligations on the implementation)

- API — **both** families: `POST /api/auth/local/login` (test creds) AND `POST /api/dev/mock-auth
  {role:'owner'}` (with secret header) → **401/404** on prod-config (flag off), **200** on
  staging-config (flag on). The mock-auth case guards against the breaker-CRITICAL-#2 regression.
- Unit: `devLoginAllowed(env)` — flag off ⇒ false; flag on + secret ⇒ true; flag on + no secret ⇒
  false. `isDevRequestAuthorized` — `/dev/*` with flag off ⇒ false even with a matching secret.
- Boot: `loadEnv()` throws when NODE_ENV=production AND (flag set OR secret set OR `JWT_DEV_KID` set).
- Verifier (proves C is wired into the header, not the body claim): a token whose **protected
  header** kid = `JWT_DEV_KID` ⇒ accepted by a non-prod verifier, **rejected** by a verifier with
  NODE_ENV=production.
- C.1 key-path (R2): a dev-mint token verifies against the **dev public key** and **fails signature**
  against the prod public key — proves kid AND key moved together, not kid alone.
- Deploy-pipeline (R2 CRITICAL + R3 HIGH): the **new standalone** `prod-smoke.spec.ts` (env-driven,
  non-serial) passes against a prod-config build with NO `DEV_AUTH_SECRET` — `/livez`+`/health` + a
  public `/s/:slug` read by seeded `PROD_SMOKE_SLUG` (NOT via `/api/owner/settings`) + the extracted
  1.1–1.3 negatives, minting no token; the **edited** telegram specs + `flow-core-lifecycles` +
  `deploy-validation` pass against **staging-config** (`VITE_BASE_URL=staging`, flag on) — proving they
  hit staging, not prod. The prod deploy job is green with the backdoor closed and no prod step calls
  `/api/dev/*`.
- NODE_ENV pre-traffic assert (R2 HIGH + R3 MEDIUM): a test of the migrator entrypoint (`dist/migrate`)
  exits **nonzero** when `FLY_APP_NAME==='dowiz'` AND `NODE_ENV!=='production'` (unset and `development`),
  **zero** when `production` — proving the gate runs in `release_command` (pre-traffic), not as a
  post-deploy CI step, and relies on no `/health` NODE_ENV field.

## Open items (owner)
- R-1: set the NODE_ENV matrix via **per-app Fly secrets** — prod `production` (asserted **pre-traffic
  in `release_command`**, R3), staging/CI `development`. No Dockerfile ENV (the assumed prod runtime
  stage does not exist). (operator + implementer)
- R-12: the §9.A spec-split is a **source rewrite** — edit 2 telegram specs to read `VITE_BASE_URL`,
  add the new standalone `prod-smoke.spec.ts`, retire `deploy-validation.spec.ts` from the prod job; ~3
  of ~50 prod-targeted tests reuse as-is. Enlarges the E2E work. (implementer E2E)
- R-13: verify `FLY_APP_NAME` is populated in the prod `release_command` env (Fly platform-set); if
  absent, use an explicit prod-only marker secret (`DEPLOY_TARGET=prod`) so the pre-traffic NODE_ENV
  gate is not silently inert. (implementer verify + operator)
- R-5: confirm which specs hit `/api/auth/local/login` vs `/dev/mock-auth` and whether any assert
  against `empty@dowiz.com` or the plugin bypass branch, before deleting (implementer).
- R-6: **MANDATORY** operator rotation of the prod JWT key/kid — the sole kill for the
  already-minted `kid:1` leaked owner token (this design prevents recurrence but cannot reject that
  token). (operator)
- R-10: dev-keypair-absent-from-prod is the same copy-paste guarantee class that failed for the
  secret — **accept-risk (reduced)**: D catches `JWT_DEV_KID` on a NODE_ENV=production box; fail-open
  needs a pasted keypair AND wrong prod NODE_ENV. (operator hygiene + architect)
- R-11: **prod deploy validation depends on the prod backdoor (4 CI steps)** — **FIX in scope (§9.A)**:
  rewire CI (prod smoke unauthenticated + staging gate) FIRST, then close prod + unset secret. If the
  secret is unset first for incident reasons, deploys still ship but post-deploy validation is red
  until the rewire lands. (implementer pipeline + operator sequencing)

## NEEDS-HUMAN-DECISION (counsel ETHICAL-STOPs + open question — NOT decided in this ADR)
- **STOP-1 — Forensics before "closed":** run the row-provenance + refresh-token queries
  (resolution.md) to establish whether the backdoor was used; org/location/membership rows survive
  key rotation. No auth/login audit table exists (verified) → absence of evidence ≠ evidence of
  absence. Owner: operator (human).
- **STOP-2 — Disclosure:** if real PII was within blast radius, record a conscious notify/don't-notify
  decision in `/compliance`. Depends on STOP-1 + the open question. Owner: data-steward (human).
- **Open question:** count real (non-test) users + paid orders in the exposure window (query in
  resolution.md) — determines CRITICAL-with-victims vs near-miss. Owner: operator (human).
