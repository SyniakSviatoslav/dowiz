# Breaker Findings — dev-login-backdoor-hardening

**Breaker:** System Breaker DeliveryOS
**Target:** `docs/design/dev-login-backdoor-hardening/proposal.md` + `docs/adr/0003-dev-login-fail-closed.md`
**Date:** 2026-06-22
**Verdict:** Option **C is non-implementable as written** (the existing signer cannot produce a `dev`-kid token). Scope of B is **understated** — the design names 3 call sites but there are **≥6 token-mint paths and 2 unguarded `/dev/mock-auth` handlers** the gate change does not touch. Two findings are CRITICAL because they leave the live prod backdoor partially open after the design ships.

Read before concluding: `packages/platform/src/auth/jwt.ts`, `apps/api/src/plugins/dev-guard.ts`, `apps/api/src/server.ts:513-520,639-714,868-892`, `apps/api/src/routes/dev/mock-auth.ts`, `apps/api/src/routes/auth/local.ts`, `apps/api/src/plugins/auth.ts:62-70`, `packages/config/src/index.ts`, `fly.toml`, `.github/workflows/ci.yml`.

---

## CRITICAL

### [CRITICAL] B-SEC · Option C (dev-kid) is non-implementable: `signAuthToken` cannot emit a non-prod kid; it always stamps the prod kid in the JWT header

The entire C backstop rests on "sign dev tokens under a `dev` kid so prod's verifier rejects them on kid mismatch." The real signer does **not** support this.

`packages/platform/src/auth/jwt.ts:27-41`:
```ts
export async function signAuthToken(payload: ... & { kid?: string }, expiresIn) {
  const k = getKid();                       // = env.JWT_KID, always
  const jwtPayload = { ...payload, kid: payload.kid || k };   // body claim
  const jwt = new SignJWT(jwtPayload as any)
    .setProtectedHeader({ alg: 'RS256', kid: k })             // HEADER kid = prod kid, HARDCODED
```
The `payload.kid` override only lands in the **JWT body claim**, never the **protected header**. The verifier checks the **header** (`jwt.ts:55  if (protectedHeader.alg... ) ... protectedHeader.kid !== getKid()`). So:
- A "dev-kid" token minted by the staging app still carries `header.kid = env.JWT_KID` of staging.
- If staging shares prod's `JWT_KID` (proposal §2 flags this as unverified; CI literally uses `JWT_KID=1`, the prod kid — `.github/workflows/ci.yml:103`), prod's verifier sees `header.kid === 1 === getKid()` and **accepts it**.

**Break scenario:** A token minted today by the prod backdoor (`kid:1`) — the live leak — has `header.kid=1`. C claims prod rejects it "by construction." It does not: `verifyAuthToken` accepts `kid=1` because that IS the prod kid. C as designed invalidates **zero** leaked tokens. To make C real, `signAuthToken` must be changed to put the override kid in `setProtectedHeader`, AND prod must rotate to a kid the dev keypair does not use — a code change the design declares it "reuses existing verifier check, no new sign logic" (§3 C-pros, §4). False.

**Invariant violated:** "already-minted leaked dev token fails verification on prod today, no clock-wait" (proposal §7 row 3, §8). It does not fail — it verifies fine. The CRITICAL backdoor's already-issued owner token remains valid for its full 1-day exp regardless of this design.

---

### [CRITICAL] B-SEC · Gate-change scope is wrong: the two `/dev/mock-auth` owner/courier minters never call `devLoginAllowed` — fixing the gate function does NOT close them

Design §2/§4: "one change in the gate function (`devLoginAllowed`) covers all three call sites." Grep of the actual minting surface (`grep -rn signAuthToken apps/api/src`) shows the live image mints owner/courier tokens from **at least 6 sites**, and the two highest-power ones are **not gated by `devLoginAllowed` at all**:

- `apps/api/src/server.ts:641` inline `/api/dev/mock-auth` — mints `role:'owner'` (line 711), `role:'courier'` (650), and a fresh-owner with **no membership** (664) — and **inserts** rows into `users`/`memberships`.
- `apps/api/src/routes/dev/mock-auth.ts:8` second `/dev/mock-auth` handler — mints owner/courier the same way.

Neither references `devLoginAllowed`. Their only guard is the path-based onRequest hook (`server.ts:513-520` → `isDevRequestAuthorized`), which is **exactly the `!!secret` check the design condemns** (`dev-guard.ts:47  if (!configuredSecret) return false`). So after B ships and flips `devLoginAllowed` to require `ALLOW_DEV_LOGIN`, a prod with a leaked `DEV_AUTH_SECRET` (today's state) still serves `POST /api/dev/mock-auth {role:'owner'}` with the right `x-dev-auth-secret` header → real prod-kid owner token + a written `memberships` row. The `onboarding/start` self-escalation the design centers on is reachable through this path too (`owner/onboarding.ts:30  requireRole(['owner'])`).

**Break scenario:** prod still has the leaked `stg-e2e-secret`. Attacker who also has that secret (it leaked once) POSTs `/api/dev/mock-auth` with header `x-dev-auth-secret: stg-e2e-secret` → 200, owner JWT, `kid:1`, verifies on prod. Backdoor still open.

**Invariant violated:** "Production fails closed for all dev-login / mock-auth paths" (proposal §1 Goals, line 40). The design fixes the `local/login` family but leaves the mock-auth family on the old `!!secret` gate. Severity is CRITICAL because the *named* blast radius (owner token → onboarding) is reachable post-fix.

---

## HIGH

### [HIGH] B-OPS · D (boot fail-fast) cannot fire on staging/CI as currently configured — NODE_ENV is absent there, so D is a no-op exactly where the dangerous combo lives

`packages/config/src/index.ts:4` makes `NODE_ENV` required (no default). D triggers on `NODE_ENV==='production' AND (flag OR secret)`. But:
- `.github/workflows/ci.yml` writes the fresh-provision env-file with `JWT_KID=1`, `DEV_AUTH_SECRET=ci-fresh-secret` and **no `NODE_ENV`** (grep confirms zero `NODE_ENV` lines). If CI boot currently works, `NODE_ENV` is being injected elsewhere as `test`/`development` — meaning D never fires in CI (correct intent) but ALSO means the only place D ever evaluates `production` is prod itself, where the secret being set is the live bug. D therefore guards the one box that is already compromised and is silent everywhere the operator would actually test the guard.
- `fly.toml` has **no `[env]` block** and the Dockerfile sets no `NODE_ENV` (both confirmed). So D's trigger condition on prod depends entirely on an out-of-band `fly secrets` value the repo cannot see (design admits this, §2). If prod's `NODE_ENV` is unset → app won't boot (fine) but D never ran; if it's `development` → D silently does not fire while the backdoor is live (proposal's own R-2).

**Break scenario:** Operator sets `ALLOW_DEV_LOGIN=true` on prod by mistake while prod `NODE_ENV` is (unknown, possibly `development`). D does not throw. B's default-off is overridden by the explicit `true`. C is broken (see CRITICAL #1). Result: full backdoor reopened with zero boot failure. The design's claim "misconfig becomes a crash-on-deploy, detectable <1 min" (§3 D, §9) holds only if prod `NODE_ENV==='production'`, which is unverified and unverifiable from the repo.

**Invariant violated:** "Make misconfiguration loud at boot, not silently exploitable at runtime" (§1 Goals line 44).

---

### [HIGH] B-FAIL / Regression · CI E2E logs in repeatedly from one runner IP — a 5/min rate-limit on `/api/auth/local/login` will flake the 14-spec suite

Design §4 item 5 / §7 last row: add `config.rateLimit` ~5/min to the inline login. The global limiter is already `max:100, timeWindow:'1 minute'` (`server.ts:483-485`); auth routes use `max:5–10` per minute keyed by IP. CI runs `~14` specs (proposal §2) from a **single GitHub Actions runner egress IP**, many specs authenticating in `beforeAll`/`beforeEach`. Parallel workers (Playwright default = CPU count, often 2–4) × multiple specs each logging in = easily >5 login POSTs/min from one IP.

**Number:** 14 specs, assume each logs in 1–3 times, 2–4 parallel workers → 14–40+ login attempts in the suite's opening minute from one IP. A 5/min cap returns HTTP 429; tests asserting 200 fail intermittently. This is a self-inflicted CI break, not a security gain (the dev creds are gone per item 4, so brute force is moot in non-prod).

**Invariant violated:** "Staging + CI E2E keep working unchanged" (§1 Goals line 42). The design does not specify the rate-limit key (IP vs flag-aware) or an allowlist for the dev path, so the default IP key will throttle CI.

---

### [HIGH] B-SEC · Leaked prod owner token (kid:1, exp 1d) is NOT invalidated by this design — only the operator's out-of-band key rotation kills it

Because C is broken (CRITICAL #1), the only thing that invalidates the already-minted leaked owner JWT is R-6's operator key rotation, which the design explicitly puts **out of its control** (§2 non-goals, §10 R-6). The design text repeatedly claims C handles "steady state" and rotation is mere "belt-and-suspenders" (§8). With C inoperative, rotation is the **sole** kill mechanism, not a backup. If rotation is skipped, the leaked token lives up to 24h and — via the still-open mock-auth path (CRITICAL #2) — fresh equivalents can be minted.

**Break scenario:** Design ships, operator believes C "invalidates by construction," defers rotation. Leaked owner token keeps creating/reading tenant data for up to 24h; attacker re-mints before expiry.

**Invariant violated:** "Make any already-minted leaked dev token rejectable on prod (kid strategy)" (§1 Goals line 45).

---

## MEDIUM

### [MED] B-CONSIST · `getEnv()`/`getKid()` is a module-level singleton cache — a dev-kid signer and the prod verifier in the same process would read the same cached kid

`jwt.ts:6-11`: `_env` is cached on first `loadEnv()`; `getKid()` returns `_env.JWT_KID`. There is one kid per process. If the eventual fix tries to make C work by reading a separate `JWT_DEV_KID` for signing while verification still calls `getKid()`, both go through the same singleton; any "switch kid for dev mint" must thread an explicit override into `setProtectedHeader` (which today is hardcoded to `k`). The design assumes a clean per-token kid selection that the current single-kid module shape does not provide. Read-after-config consistency between sign and verify in one process is not addressed.

**Invariant touched:** server-authoritative kid selection (§6) — currently kid is process-global, not per-token.

### [MED] B-ANTIPATTERN · `empty@dowiz.com / empty123456` second cred pair lives only in the latent `local.ts` duplicate, which is dead (404s) but ships in the image — design plans to "align or delete," leaving it open which

Proposal §1 table calls `routes/auth/local.ts` a "latent duplicate" that "currently 404s." Confirmed: it is a Fastify plugin route `'/auth/local/login'` (no `/api` prefix shown registered), while the live path is the inline `server.ts:868` `/api/auth/local/login`. But the `empty@dowiz.com` cred bypass (`local.ts:43-45`) only exists in this file. The design says "align or delete the duplicate" (§4 item 5, R-5) without deciding — if "align" is chosen, the second hardcoded cred could get promoted into a live path. The ambiguity is a hazard: the safe outcome (delete) is not committed to.

**Invariant touched:** "exactly one dev-bypass path exists" (§4 item 5) — design leaves two acceptable resolutions, one of which (align) preserves a second cred.

### [MED] B-OPS · `NODE_ENV` enum has no `staging` value, so any environment that needs to mirror prod cannot — D's prod-check is coupled to an enum that forbids the natural staging value

`config/src/index.ts:4  z.enum(["development","test","production"])`. Staging must run as `development` to keep dev-login (proposal §9, R-1). This means staging is config-indistinguishable from a developer laptop, and a real "staging-as-prod-mirror" is impossible without an enum edit that the design flags (R-4) but does not resolve. Operationally the team cannot test prod's D-firing behavior on staging without setting staging `NODE_ENV=production`, which (per §7 row "NODE_ENV mistakenly production on staging") deliberately breaks staging boot. So D's prod path is effectively **untestable pre-prod**.

**Invariant touched:** "misconfig detectable <1 min" is only ever exercised in prod, never rehearsed.

---

## LOW

### [LOW] B-SEC · No rate-limit on the unguarded `/api/dev/mock-auth` minters either; design's rate-limit item targets only `local/login`

Even setting aside CRITICAL #2, the design's rate-limit (§4 item 5) names only the inline `/api/auth/local/login`. The mock-auth handlers (`server.ts:641`, `routes/dev/mock-auth.ts:8`) that also mint owner tokens get none. In a non-prod env this is low impact (creds removed, secret-gated), but it contradicts the implied "one dev-bypass path, rate-limited."

### [LOW] B-ANTIPATTERN · Dev keypair provisioning for C adds two-environment secret management for a mechanism that (as shown) doesn't work without a signer change — net new moving parts with no current security gain

§9 requires provisioning a dev keypair + dev kid in staging and CI (and threading into the sign path). Until CRITICAL #1's signer change lands, this is pure operational overhead (two more secrets to rotate, CI env-file lines) delivering zero prod-rejection. Sequencing risk: if the keypair is provisioned but the `setProtectedHeader` change is forgotten, the team believes C is live when it is not.

---

## Regression summary (vs current behavior)
- **CI/staging E2E:** at risk from the 5/min login rate-limit on a shared IP (HIGH) and from any accidental staging `NODE_ENV=production` (D breaks boot, by design).
- **Dev-kid (`header.kid`) change** would change every token's header; existing in-flight staging tokens minted under the old kid would be rejected after a kid switch — a transient auth break window the design does not call out.
- **Removing hardcoded creds:** fine, but the live `test@dowiz.com` token-mint depends on the seeded user existing (`server.ts:875` looks it up); removing the literal only closes the *password*, not the secret-gated mock-auth mint.

## What would actually close the live CRITICAL (stated as gaps, not fixes)
The design does not, on its own, (a) reject the already-leaked `kid:1` token, nor (b) close the `/api/dev/mock-auth` owner-mint path on a prod that still holds the leaked secret. Both remain reachable. The only mechanisms that close them in this design are the operator's out-of-band secret-unset + key rotation (R-6) — i.e. the parts explicitly declared out of scope.

---

# RE-ATTACK — Round 2 (regression check vs revised proposal.md + resolution.md)

**Date:** 2026-06-22
**Scope:** the two prior CRITICALs were "fixed" in the DESIGN (proposal+resolution rewritten). No product code changed — `dev-guard.ts`, `jwt.ts`, `config/index.ts`, `server.ts`, Dockerfile, ci.yml are all still in their pre-design state (verified live this round). So this re-attack is against the REVISED DESIGN as it would ship, read against real source it must integrate with.
**Verdict:** The two prior CRITICALs are no longer the same hole, but the C.1/C.2/D rewrite opened **one new CRITICAL** (CI's own prod-gate calls the very `/dev/*` path the design closes on prod → green CI becomes impossible or the secret stays on prod) and **one new HIGH** (the "prod runtime stage" the Dockerfile pin depends on does not exist — single runtime stage shared by staging). Plus MED/LOW regressions below.

## NEW findings

### [CRITICAL] B-OPS / B-ANTIPATTERN · The CI prod-deploy gate itself calls `/api/dev/mock-auth` against PROD — G.2 closing `/dev/*` on prod makes the prod deploy job un-greenable (or forces the dev secret to stay on prod, re-opening the hole the design exists to close)

The design's load-bearing claim is "prod fails closed for the entire `/dev/*` family" (G.2: flag off on prod → `/dev/*` 404s). But the CI `deploy` job (`.github/workflows/ci.yml:127-177`, runs only on push to `main`, deploys **prod** `dowiz`) runs two post-deploy E2E suites **against `https://dowiz.fly.dev`** that depend on `/api/dev/mock-auth`:
- `deploy-validation.spec.ts:13-14` — test **"0.1 — mock-auth returns valid owner token"** POSTs `${BASE}/api/dev/mock-auth` to prod.
- `flow-core-lifecycles.spec.ts:31` — POSTs `${BASE}/api/dev/mock-auth {role:'owner'}` to prod, and `:351` POSTs `/api/dev/create-assignment` to prod.
- The secret is injected globally by `playwright.config.ts:16` (`extraHTTPHeaders` from `process.env.DEV_AUTH_SECRET`), and CI passes `DEV_AUTH_SECRET: ${{ secrets.DEV_AUTH_SECRET }}` to all three prod E2E steps (`ci.yml:162,169,176`).

**Break scenario:** Design ships. Operator removes the prod `DEV_AUTH_SECRET` + sets `ALLOW_DEV_LOGIN` absent (the whole point). Next push to `main` deploys prod, then runs `deploy-validation.spec.ts` → `POST https://dowiz.fly.dev/api/dev/mock-auth` → **404** (G.2 fail-closed) → test 0.1 fails → **prod deploy job goes red on every push.** The only ways to make CI green again are: (a) keep `DEV_AUTH_SECRET` + `ALLOW_DEV_LOGIN=true` on prod — which is exactly the dangerous combo D is supposed to make un-bootable (and which re-opens the backdoor), or (b) rewrite/delete the prod-targeted mock-auth E2E — a change the design never mentions in its proof obligations or §9 CI section. The resolution's §9 says only "CI fresh-provision adds `ALLOW_DEV_LOGIN=true`" and "post-deploy E2E … unchanged (the `x-dev-auth-secret` protocol is untouched)" — but the protocol being untouched is irrelevant; the *endpoint returns 404 on prod regardless of the header*. The design asserts "Staging + CI E2E keep working unchanged" (proposal §1 goal) while simultaneously closing the prod endpoint those CI steps hit.

**Invariant violated:** "Staging + CI E2E keep working unchanged" (proposal §1 Goals) AND "Production fails closed for all dev-login / mock-auth paths" — the two are in direct contradiction for the prod-targeted CI suite, and the design resolves neither. The scaling-gate/flag does NOT "really lock" without breaking the deploy gate that proves prod is alive.

### [HIGH] B-OPS · D's reliability rests on `ENV NODE_ENV=production` in "the prod runtime stage" — but the Dockerfile has ONE shared runtime stage used by both prod and staging; pinning it there forces staging/CI to override or inherit `production`

Proposal §9 + resolution §3 step 1 repeatedly say: pin `ENV NODE_ENV=production` in **"the production runtime stage of the Dockerfile"** so it is "in-repo and greppable" and staging/CI "build the same image but override." Verified against `Dockerfile`: there are exactly two `FROM` lines — `builder` (L2) and one unnamed runtime `FROM node:22-slim` (L30). **There is no prod-specific runtime stage.** `fly.toml` is a single file (`app="dowiz"`); staging deploys the *same* Dockerfile/fly.toml via `flyctl deploy -a dowiz-staging` (per CLAUDE.md ship-discipline + saved staging token). So:
- Pinning `ENV NODE_ENV=production` in the only runtime stage makes **every** environment — prod, staging, CI-built images, and any local `docker run` — default to `production` unless each separately overrides it.
- The design's own R-1 names the trade (staging must set `NODE_ENV=development` or D fires), but understates the surface: it's not "staging gets an explicit Fly secret," it's **every non-prod consumer of this image must remember to override**, and a forgotten override fails toward `production` + (if the dev flag/secret is set) a **boot-throw** (D) — i.e. staging/CI go down loudly. The design frames this as "correct fail direction," but it converts a missing-env-var mistake on staging from "works" today into "refuses to boot" — a real availability regression for the non-prod fleet, on a single shared image.
- Greppability claim is also weakened: a single `ENV NODE_ENV=production` in a shared stage does NOT encode "prod only"; it encodes "default for all images from this Dockerfile," which is misleading in-repo.

**Invariant violated:** "misconfiguration loud at boot, not silently exploitable" is preserved for prod, but "Staging + CI E2E keep working unchanged" is now contingent on an override that the single-stage Dockerfile makes the *unsafe default* for every non-prod box. The proposal asserts a "prod runtime stage" that does not exist — the implementer would have to invent multi-stage targeting the design never specs.

### [MED] B-SEC · C.3's "dev keypair is absent from prod" is the SAME class of guarantee as "DEV_AUTH_SECRET is unset on prod" — which already failed once (the live leak). C.2 fails OPEN if a future operator copies the dev keypair to prod, exactly as they copied the staging secret

C.2 makes prod-rejection of dev-kid tokens rest on "key material isolation (the dev keypair is absent from prod), with NODE_ENV as a second lock" (resolution C.2). But the entire incident that spawned this design is `DEV_AUTH_SECRET` leaking from staging to prod ("leaked from staging's `stg-e2e-secret`", proposal §1). The dev keypair (`JWT_DEV_PRIVATE_KEY`/`JWT_DEV_PUBLIC_KEY` + `JWT_DEV_KID`) is distributed to staging Fly secrets + CI env-files — the identical distribution channel and copy-paste hazard. If `acceptDevKid = NODE_ENV!=='production' && !!JWT_DEV_KID`, then the moment prod's `NODE_ENV` is *also* wrong (the R-2 residual, unverifiable from repo) AND someone has pasted the dev keypair onto prod, prod **accepts dev-kid tokens** — and now any holder of the (shared, non-prod, lower-trust) dev private key forges prod-valid owner tokens. The design treats key-isolation as a stronger guarantee than NODE_ENV; in this org's actual operational history it is the *same* guarantee that already broke. Not a fail-open in the common case, but the design overstates "by construction" — it is "by construction, assuming the exact discipline that already failed once."

**Invariant violated:** the design's own §2 finding that "a NODE_ENV-only gate is only as trustworthy as an invisible Fly setting" applies verbatim to "a key-absence gate is only as trustworthy as nobody copying the dev keypair" — the design relies on it anyway.

### [MED] B-CONSIST · C.1 stamps `payload.kid || getKid()` into the protected header, but does NOT change `getPrivateKey()` — a dev-kid token would still be SIGNED with the prod private key unless the signer also switches keys, which the design assigns to the call sites, not the signer

`signAuthToken` (`jwt.ts:36-41`) signs with `getPrivateKey()` = `env.JWT_PRIVATE_KEY` unconditionally. C.1 (resolution) changes only `setProtectedHeader({...kid: headerKid})`. So a token with `header.kid = JWT_DEV_KID` would still be **signed by the prod/main private key** unless the dev-mint sites *also* sign with the dev private key — which resolution C.3 says they do ("sign with the dev private key"), but the C.1 signer snippet has no `privateKey` parameter and `getPrivateKey()` is hardcoded to the env key. The design splits the signing-key choice (C.3, "dev-mint sites pass…and sign with the dev private key") from the kid choice (C.1, signer arg) without showing how the call site overrides the *key* through a `signAuthToken` whose key source is a hardcoded singleton. Result risk: a dev-kid token signed with the *prod* key → on staging the dev verifier (holds dev public key) **rejects it** (sig mismatch), and on prod the verifier (prod public key) **accepts** the signature but reads `header.kid=JWT_DEV_KID` → rejects on kid (good) UNLESS `acceptDevKid` is wrongly true. The header/key split is under-specified and the proof obligation ("token whose header kid = JWT_DEV_KID, minted by C.1, accepted by non-prod verifier") cannot pass unless the signer also threads the private key — which neither C.1 nor the signature change describes.

**Invariant violated:** "C is actually wired" (proof obligation) — the design proves the header kid path but leaves the signing-key path to "the call sites," which cannot reach `getPrivateKey()`'s hardcoded singleton without a further signer change the design omits.

### [LOW] B-FAIL · Rate-limit "exempt-when-gate-open" cannot be triggered by an attacker header (good) — but the email+IP key on the real argon2 path is a weak enumeration oracle and the design leaves the limiter store-failure behavior as a "should," not a spec

Two sub-points, both LOW because they harden a path that is out of primary scope:
- Exemption is keyed on `devLoginAllowed(env)` (server-side env), NOT on any request header — so an attacker cannot flip into the exempt branch by sending a header. **Confirmed safe; no bypass.** (This sub-attack HOLDS for the design.)
- email+IP keying: an attacker rotating IPs gets a fresh 10/min bucket *per email*, so it does not protect a single targeted account from a distributed guess; and from one IP, attempting many *distinct* emails each gets its own bucket — bounded only by the global 100/min IP cap, so ~100/min of email-enumeration-by-timing is still possible (valid-email vs invalid-email response differ: `local.ts:34` 401 "Invalid email or password" vs `:55/:59` "Account uses another sign-in method"). The current `local.ts` already leaks account-existence via distinct error strings; email+IP keying does not close that and the design doesn't note it.
- "Rate-limiter store unavailable → should fail-closed" (proposal §7 last row) is left as advisory ("should"), not a spec'd behavior with a proof obligation — so the implementer may ship fail-open.

**Invariant touched:** "client total/identity not trusted" holds; but online-enumeration hardening is weaker than claimed and store-failure direction is unspecified.

## Regression verdict per PRIOR finding

- **[CRITICAL] #1 — C non-implementable (signer hardcodes header kid):** HOLDS-as-fixed-in-design, with caveat. C.1 correctly relocates the kid to `setProtectedHeader` and the design no longer claims "no new sign logic." BUT the signing-*key* override is under-specified (see NEW MED B-CONSIST above) — the header is fixed, the key is not. Design-level fix is directionally correct; proof obligation as written is not yet satisfiable.
- **[CRITICAL] #2 — gate scope (6 sites, mock-auth ungated):** HOLDS-as-fixed. G.2 folds `ALLOW_DEV_LOGIN` into `isDevRequestAuthorized`, which is the actual guard on sites #2/#3/#5 (`server.ts:520` calls it for all `/dev/*`). All six sites now route through G.1 or G.2 under one flag. No mint site remains on bare `!!secret`. Adding the flag to `isDevRequestAuthorized` does NOT break a non-mint `/dev` path (the only non-mint dev paths — create-assignment/seed — are *meant* to be closed on prod too). The fix is sound — BUT it is precisely what triggers the NEW CRITICAL above (prod CI E2E hits these closed endpoints).
- **[HIGH] #3 — D silent on staging/CI (no NODE_ENV):** PARTIALLY-HOLDS / REOPENED-as-HIGH. The unit-test rehearsal of D's prod path is a real fix (D is now provable in CI). But the mechanism that makes D *fire in prod* — `ENV NODE_ENV=production` in "the prod runtime stage" — rests on a Dockerfile stage that does not exist (NEW HIGH above). The detector is testable; its prod-arming is mis-specified.
- **[HIGH] #4 — 5/min rate-limit flakes CI:** HOLDS-as-fixed. Exempt-when-gate-open removes the per-route cap on the dev path (no 429 from one CI IP); real argon2 path keyed email+IP. CI flake is resolved. (Residual enumeration weakness is a new LOW, not a reopening of #4.)
- **[HIGH] #5 — leaked kid:1 token not invalidated:** HOLDS. Design now states plainly that C does NOT reject the already-minted prod-kid token and that operator rotation (R-6) is MANDATORY and the SOLE kill — no longer "belt-and-suspenders." Honest. No downgrade-by-wording.
- **[MED] #6 — singleton kid sign/verify:** HOLDS-as-fixed-for-kid, but exposes #C.1 key gap (the kid became a per-token arg; the *key* did not — see NEW MED).
- **[MED] #7 — empty@dowiz.com 2nd cred pair:** HOLDS-as-fixed. Resolution commits to DELETE (not "align"); `local.ts:43-46` bypass branch + both cred pairs removed; one dev-bypass path remains. Verified the `empty@` pair exists only at `local.ts:44`. Clean.
- **[MED] #8 — NODE_ENV enum lacks staging / D untestable:** HOLDS-as-fixed. Enum stays 3-value; D rehearsed by unit test. (Coupled to the #3 prod-arming gap, but the testability concern itself is resolved.)
- **[LOW] #9 — no rate-limit on mock-auth minters:** HOLDS (accept-risk; flag-gated + secret-gated).
- **[LOW] #10 — dev-keypair overhead before C.1:** HOLDS (sequencing C.1→C.2→C.3 + proof gate mandated).

## One-line bottom line
The gate-scope CRITICAL (#2) is genuinely closed in design — and closing it is what surfaces the NEW CRITICAL: the project's own prod-deploy CI gate authenticates via `/api/dev/mock-auth` against `dowiz.fly.dev`, so "prod fails closed for `/dev/*`" and "CI E2E unchanged" cannot both be true until the prod-targeted mock-auth E2E is redesigned. The C-rewrite is directionally right but its signing-*key* path and its Dockerfile "prod runtime stage" are mis-specified against real source.

---

# RE-ATTACK — Round 2 (regression check vs RESOLVE round 2)

**Date:** 2026-06-22
**Scope:** Attack the NEW surface introduced by RESOLVE round 2 — the CI deploy-pipeline redesign
(R2-1: prod unauth smoke + staging-e2e pre-deploy gate), the per-app NODE_ENV Fly secret +
deploy-assert (R2-2, replacing the dead Dockerfile pin), and the `signDevToken`/`devSigningParams`
helper (R2-4). Read against live source: `ci.yml`, `e2e/tests/deploy-validation.spec.ts`,
`e2e/tests/flow-core-lifecycles.spec.ts`, `e2e/tests/telegram-webhook.spec.ts`,
`e2e/tests/telegram-full-flow.spec.ts`, `e2e/lifecycle-e2e/playwright.config.ts`,
`packages/platform/src/auth/jwt.ts`, `apps/api/src/plugins/dev-guard.ts`,
`apps/api/src/routes/health.ts`, `Dockerfile`, `fly.toml`. No product code changed since R1 (verified
`git status` clean for `apps/api/src`, `packages/platform`, `packages/config`, `Dockerfile`, `ci.yml`,
`e2e/`) — this attacks the REVISED DESIGN as it would ship.

**Verdict:** The R2-1 deploy redesign is directionally right but its spec-split is **materially
incomplete against real source** — one NEW HIGH (the two telegram specs and the bulk of
`deploy-validation.spec.ts` cannot move to staging as the proposal describes, because the telegram
specs ignore `VITE_BASE_URL` and `deploy-validation` is a serial auth-chained suite). One NEW MEDIUM
(the prod NODE_ENV deploy-assert as spec'd is fail-open / non-existent: `/health` has no NODE_ENV
field, and the asserting step runs AFTER `flyctl deploy` puts the new artifact in service → TOCTOU /
not-before-traffic). The "does staging-e2e validate the PROD artifact" attack lands as an explicitly
accepted residual, not a NEW finding. No NEW CRITICAL.

## NEW findings

### [HIGH] B-OPS / B-ANTIPATTERN · The R2-1 spec-split is incomplete against real source: the two telegram specs hardcode `BASE='https://dowiz.fly.dev'` (ignore `VITE_BASE_URL`), and `deploy-validation.spec.ts` is a serial auth-chained suite — none can be "moved to staging" or "kept as unauth prod smoke" without source edits the proposal does not spec

The R2-1 decision (proposal §9.A, resolution R2-1) says: split `deploy-validation.spec.ts` (negative-auth+health+storefront → prod smoke; test 0.1 mock-auth + authenticated → staging suite) and "move" `flow-core-lifecycles` + the telegram suites to a staging-e2e gate that runs against `https://dowiz-staging.fly.dev`. Real source breaks this in two concrete ways:

1. **Both telegram specs are NOT env-driven — they hardcode the prod URL.** `e2e/tests/telegram-webhook.spec.ts:3` and `e2e/tests/telegram-full-flow.spec.ts:3` are literally `const BASE = 'https://dowiz.fly.dev';` (no `process.env.VITE_BASE_URL` fallback — unlike `deploy-validation.spec.ts:3` and `flow-core-lifecycles.spec.ts:4` which DO read the env var). Setting `VITE_BASE_URL=https://dowiz-staging.fly.dev` on the new `staging-e2e` job has **zero effect** on these two — they will hammer **prod** `dowiz.fly.dev` (`POST https://dowiz.fly.dev/api/dev/mock-auth` at telegram-full-flow.spec.ts:51, webhook at telegram-webhook.spec.ts:5) from a job whose entire premise is "authenticated work happens on staging, never prod." So either (a) the staging gate still authenticates against the prod backdoor — defeating R2-1 — or (b) the implementer must EDIT both spec files to make `BASE` env-driven, a source change the proposal's proof-obligations and §9.A never list (it claims the suites "keep their current mock-auth calls **unchanged**" and "no spec change for C"). The split is not free; it requires editing two specs the design declares unchanged.

2. **`deploy-validation.spec.ts` cannot be cleanly bisected — it is `mode:'serial'` with a shared `authToken`.** The proposal says keep tests 1.1–1.3 (the 401 negatives), health (8.1), storefront (3.1/5.1/9.1) as the unauthenticated prod smoke. But the file is `test.describe.configure({ mode: 'serial' })` (`:8`) and **test 0.1 assigns the module-level `authToken` and `locationSlug`** that tests 2.1, 2.2, 3.1, 4.x, 5.1, 6.2, 7.1, 11.1, 13.1, 14.x all consume (`grep -c authToken|Authorization` = the auth tests dominate the file; tests 2.x–14.x carry `Authorization: Bearer ${authToken}`). The "storefront-read" tests the proposal wants to keep on prod (3.1, 5.1) themselves depend on `locationSlug`, which is populated by **3.1 calling the AUTHENTICATED `/api/owner/settings`** (`:67`, `Authorization: Bearer ${authToken}`), not by an anonymous read. So "keep health + storefront-read + the 401 negatives as the prod smoke" is not a clean subset — pulling test 0.1 out strands `locationSlug` and most "storefront" assertions. The prod smoke must be **rewritten**, not merely "split," and the proposal's claim that the existing `:1.1–1.3` negatives "need NO backdoor" is true only for those three tests — they are 3 of ~30 in a suite the rest of which is auth-chained.

**Break scenario:** Implementer follows §9.A literally: sets `VITE_BASE_URL=staging` on `staging-e2e`, runs the four authenticated suites there, points prod at the "split" `deploy-validation`. Result: telegram-webhook + telegram-full-flow still POST to `https://dowiz.fly.dev/api/dev/mock-auth` (hardcoded BASE) → after the prod backdoor closes (step 3), those two staging-gate steps **fail against prod** (404) even though they were supposed to run on staging. And the prod-smoke half of `deploy-validation` either drags the auth chain (needs the backdoor → contradicts the goal) or 30 tests collapse to ~5 standalone ones, silently dropping the bulk of prod deploy coverage with no proof obligation flagging it.

**Invariant violated:** "Staging + CI E2E keep working unchanged" + the R2-1 proof obligation "the rewired prod-smoke spec passes against a prod-config build with **no DEV_AUTH_SECRET**." The proposal under-specs the actual code delta: it treats the split as a CI-config change when real source requires editing ≥2 hardcoded-URL specs and rewriting the auth-chained validation suite. Back-of-envelope: **4 prod E2E steps**, of which **2 (telegram) are URL-hardcoded** and **1 (deploy-validation) is a 30-test serial auth chain** — only the 3 standalone 401 negatives survive "as-is." The §9.A "split" is ~3 reusable tests out of ~50 across the four files.

### [MEDIUM] B-OPS · The prod NODE_ENV deploy-assert (R2-2) as spec'd is not runnable and runs after traffic: `/health` exposes NO `NODE_ENV` field, and the asserting step is sequenced AFTER `flyctl deploy` has already put the new artifact in service — so it neither validates before-traffic nor fails the rollout

R2-2 / proposal §9 replaces the (non-existent) Dockerfile pin with "the deploy job asserts `fly ssh console -a dowiz -C 'printenv NODE_ENV' == production` (or a `/api/health` field) **before** running post-deploy validation; mismatch = red deploy." Two concrete gaps against live source:

1. **The `/api/health` fallback does not exist.** `apps/api/src/routes/health.ts` returns `{status, checks:{postgres,settlement,telegram,...}}` (verified: `withTimeout` results, no environment field). `grep NODE_ENV apps/api/src/routes/health.ts` = zero. So the "or a `/api/health` field" half of the assertion is not implementable without ALSO adding a NODE_ENV field to the public health payload — which the design does not spec and which would itself leak environment posture on an **unauthenticated** endpoint (health.ts:38 explicitly drops detail strings as "a recon leak"; advertising `NODE_ENV` reverses that hardening). That leaves only `fly ssh console`.

2. **`fly ssh console` runs against the ALREADY-DEPLOYED machine — this is post-traffic, not pre-traffic, and does not gate the rollout.** In `ci.yml:149-184` the order is: `flyctl deploy --remote-only` (`:151`, which runs `release_command` migrate + swaps the new machine into service) → THEN the post-deploy E2E steps. Any NODE_ENV assertion added "before running post-deploy validation" still runs **after** `flyctl deploy` has put the new artifact live and after `release_command` ran migrations. So a prod box that boots with wrong/missing NODE_ENV: if NODE_ENV is **unset**, the app won't boot → `flyctl deploy`'s own health-check phase fails the release (old code serves) — fine, but that is the existing boot-enum guard, NOT the new assert. If NODE_ENV is wrongly **`development`**, the app **boots fine and serves traffic**, the deploy "succeeds," and the new assert (running afterward) goes red — but by then the development-mode artifact is **already in service**. The assert is a post-hoc red X, not a gate; combined with D's R-2 residual (NODE_ENV=development → D cannot fire → dev-login can be on), the window the design claims to close ("verifiable, checked every deploy, stronger than greppable") is checked only *after* the misconfigured box is live. The proposal's failure-table row "prod's NODE_ENV dropped/never set → deploy-assert also catches it → red deploy, old code serves" conflates the unset case (boot-fail, real) with the wrong-value case (boots+serves, assert is post-traffic).

**Break scenario:** Operator sets prod `NODE_ENV=development` by fat-finger (the exact R-2 residual). Push to main: `flyctl deploy` succeeds (app boots in dev mode, serves), migrations run, the new NODE_ENV-assert step runs via `fly ssh console`, sees `development`, marks the deploy job red. But prod is now serving a development-mode artifact (D inert, dev-login gated only by B's flag-default), and `flyctl deploy` already swapped machines — there is no automatic rollback from a failed *post-deploy* CI step. The claimed "before serving" / "old code keeps serving" property does not hold for the wrong-value case.

**Invariant violated:** R2-2's stated property "the deploy job asserts the value **before** running validation … mismatch = red deploy, old code serves" and "verifiable, checked every deploy." Against `ci.yml` ordering, the assert is after `flyctl deploy` (post-traffic), the `/health` fallback field is absent, and a failed post-deploy step does not revert the live machine. This is a fail-open relative to the design's claim, for the wrong-value (not unset) case.

## Attacks that did NOT yield a NEW finding (explicitly checked, holds or already-disclosed)

- **"Does gating prod deploy on a staging-e2e run validate the PROD artifact?"** — The proposal explicitly concedes this: option (b) "validates the *image* on staging, not the literal prod box … acceptable because it is the same image (one Dockerfile)" and routes prod-specific config to the (a) smoke + NODE_ENV assert. Verified: `Dockerfile` is one shared runtime image, `fly.toml` is single-app, staging deploys the same image. A prod-ONLY regression (config/secret/data divergence not exercised by the unauth smoke) could indeed ship staging-green/prod-broken — but this is a **named, accepted residual** (proposal §9.A option-b Con), not a hidden hole. No downgrade-by-wording; not re-raised as NEW. (The *real* gap here is that the (a) prod smoke is thinner than claimed — that is the NEW HIGH above, not this.)
- **TOCTOU between staging-e2e gate and prod deploy.** `deploy needs: [validate, staging-e2e]` means prod deploys the commit/image that staging-e2e validated (same `github.sha`). No separate rebuild between gate and deploy in the spec'd flow (`flyctl deploy --remote-only` builds from the same checkout). The image-vs-box gap (above) is the residual, not a build-time TOCTOU. Holds.
- **"Does any prod step still depend on the secret after removal?"** — R2-1 step 2 removes `DEV_AUTH_SECRET` from the prod deploy-job E2E steps. Confirmed the only current prod consumers are the 4 E2E steps (`ci.yml:162,169,176,183`) + the header injection in both playwright configs (`lifecycle-e2e/playwright.config.ts:15`, root `playwright.config.ts` extraHTTPHeaders). Migrate step uses `DATABASE_URL_MIGRATIONS`, deploy uses `FLY_API_TOKEN` — neither needs the dev secret. So *if* the prod-smoke spec is genuinely unauthenticated, no prod step needs the secret. Holds **conditionally on the NEW HIGH being fixed** (i.e., on the smoke actually being unauth-only — which the serial-chain reality undercuts).
- **C.1 `signDevToken`/`devSigningParams`: does it change the prod (NORMAL) signature path?** — Verified `jwt.ts:27-42`: today `signAuthToken` always uses `getKid()` in the header and `getPrivateKey()` (env `JWT_PRIVATE_KEY`). The R2-4 helper is additive: non-dev callers "pass neither → `getKid()` + `getPrivateKey()` unchanged." Design-level this preserves prod behavior. Holds (design intent sound; the prior under-spec is now resolved by moving kid+key as a pair).
- **"Can a caller invoke the dev signer in prod?"** — `signDevToken()` reads `devSigningParams(env) → {kid: JWT_DEV_KID, key: JWT_DEV_PRIVATE_KEY}`. On prod those envs are absent (and D fail-fasts if `JWT_DEV_KID` is set on a NODE_ENV=production box). If a dev-mint call site executes on prod with the keys absent, `devSigningParams` yields undefined kid/key → falls back to prod kid/key (signs a *prod-kid* token) OR throws on missing key — the design does not pin which. This is the same residual as R2-3 (keypair-paste) and R-2 (wrong prod NODE_ENV), already disclosed. Not a NEW finding, but flagged: the design should pin "missing JWT_DEV_* on a path that calls signDevToken → throw, not silent fallback to the prod key," else a stray dev-mint call site on prod could mint a prod-kid token. (Borderline LOW; noting, not raising — depends on unwritten code.)

## Regression verdict per PRIOR finding (R1 + R2)

- **[CRITICAL] R2-1 — prod-deploy CI gate calls `/dev/*` on prod:** HOLDS-as-addressed-in-design, BUT the fix is **under-specified** — see NEW HIGH. The decision (a)+(b) is correct in concept; the spec-split it rests on does not survive contact with the real spec files (2 hardcoded URLs + 1 serial auth chain). The CRITICAL is downgraded to "addressed but incompletely spec'd," not reopened to CRITICAL — closing it is no longer a backdoor-reopen, it is a CI-coverage/implementation-completeness gap (HIGH).
- **[CRITICAL] R1 #1 — C non-implementable (signer header kid):** HOLDS-as-fixed. C.1 relocates kid to the protected header AND (R2-4) moves the signing key with it via one helper. Proof obligation now satisfiable.
- **[CRITICAL] R1 #2 — gate scope (6 sites, mock-auth ungated):** HOLDS-as-fixed-in-design. Live `dev-guard.ts:19-21,41-49` is still bare `!!configuredSecret` (unchanged, as expected for design-time) — G.1/G.2 fold `ALLOW_DEV_LOGIN` into both functions covering all 6 sites. No new gap. (This is the fix that surfaced R2-1; that tension is real and now in scope.)
- **[HIGH] R2-2 — Dockerfile prod runtime stage does not exist:** HOLDS-as-redirected (no Dockerfile pin; per-app secret) — but the *replacement* mechanism (deploy-assert) is itself fail-open/non-existent as spec'd → NEW MEDIUM above. Verified `Dockerfile` still has one unnamed runtime FROM (`:30`), no `ENV NODE_ENV`; `fly.toml` single-app, no `[env]`. The redirect away from the pin is correct; the new assert is not yet sound.
- **[HIGH] R1 #3 — D silent on staging/CI:** HOLDS-as-fixed (unit-test rehearsal). Prod-arming now via per-app secret (R2-2), whose assert is the NEW MEDIUM.
- **[HIGH] R1 #4 — 5/min rate-limit flakes CI:** HOLDS-as-fixed (exempt-when-gate-open + email+IP for argon2).
- **[HIGH] R1 #5 — leaked kid:1 token not invalidated:** HOLDS. Rotation stated as mandatory + sole kill. No downgrade-by-wording.
- **[MED] R1 #6 — singleton kid sign/verify:** HOLDS-as-fixed (R2-4 helper moves kid+key as a pair).
- **[MED] R2-3 — dev keypair = same copy-paste class:** HOLDS as ACCEPT-RISK(reduced); D fail-fast on `JWT_DEV_KID`@prod catches the cheap half. Disclosed, not hidden.
- **[MED] R2-4 — C.1 switches kid not key:** HOLDS-as-fixed via the single `signDevToken`/`devSigningParams` helper; new key-path proof obligation added.
- **[MED] R1 #7 — empty@dowiz.com 2nd cred pair:** HOLDS-as-fixed (DELETE committed).
- **[MED] R1 #8 — NODE_ENV enum lacks staging / D untestable:** HOLDS-as-fixed (unit-test rehearsal; enum unchanged).
- **[LOW] R1 #9 — no rate-limit on mock-auth minters:** HOLDS (accept-risk).
- **[LOW] R1 #10 — dev-keypair overhead before C.1:** HOLDS (sequencing + proof gate).
- **[LOW] R2-5a/5b — enumeration oracle / store-failure:** HOLDS (5a accept-risk; 5b promoted to fail-closed spec).
- **Counsel #1–#4 forensic-query corrections:** HOLDS (BYPASSRLS pre-req, customers join, paid-enum filter, CASCADE-ordering). Not re-attacked this round (out of breaker scope; they are query-correctness, verified against migrations in RESOLVE round 2).

## One-line bottom line (R2 re-attack)
No NEW CRITICAL. The R2-1 deploy redesign is the right shape but its spec-split is incomplete against
real source — the two telegram specs hardcode `dowiz.fly.dev` and `deploy-validation.spec.ts` is a
serial auth chain, so "move to staging / keep unauth prod smoke" needs source edits the proposal calls
unchanged (NEW HIGH); and the NODE_ENV deploy-assert that replaced the dead Dockerfile pin is itself
fail-open as spec'd — `/health` has no NODE_ENV field and the `fly ssh` assert runs after `flyctl
deploy` puts the artifact in service (NEW MEDIUM). All prior CRITICAL/HIGH/MED/LOW HOLD as fixed or
disclosed; the only severity movement is the R2-1 CRITICAL becoming an implementation-completeness HIGH.
