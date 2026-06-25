# Operator Runbook — dev-login-backdoor-hardening (ADR-0003)

Engineering is done and proven on staging (commits `5da9d136` + `ef0954c9` on
`feat/golive-remediation`). The steps below are **operator-owned** (need prod / GitHub
access this session does not have). **The prod backdoor is LIVE until step 1.**

## 0. (verified) Current state
- `POST https://dowiz.fly.dev/api/auth/local/login {test@dowiz.com/test123456}` → **HTTP 200, owner JWT** (backdoor live; `DEV_AUTH_SECRET` still set on prod).
- Staging (`dowiz-staging`) is fully migrated to the new design: `ALLOW_DEV_LOGIN=true`, `JWT_DEV_KID=dev` + dev keypair, `NODE_ENV=development`, dev-login + mock-auth verified working with dev-kid tokens.

## 1. IMMEDIATE — kill the live backdoor (do now, before anything else)
```
flyctl secrets unset DEV_AUTH_SECRET -a dowiz   # restarts prod; backdoor dies on restart
```
Side effect: the current prod deploy-validation E2E (mock-auth on prod) will 404 on the
next push until the CI redesign (step 4) lands. That is expected — do NOT "fix" it by
re-adding the secret. `flyctl deploy` itself still succeeds.

## 2. Pin prod NODE_ENV (the boot-guard + release-guard depend on it)
```
flyctl secrets set NODE_ENV=production -a dowiz   # if not already set to exactly 'production'
```
After step 4 ships, the release_command will refuse the rollout if this is wrong.

## 3. Rotate the prod signing key — kills the ALREADY-LEAKED kid:1 token (R-6)
The leaked owner token (kid:1, ~24h TTL) is a valid prod-kid token; only key rotation
invalidates it before expiry. Rotate `JWT_PRIVATE_KEY`/`JWT_PUBLIC_KEY` + bump `JWT_KID`
(e.g. `1` → `2`) on `dowiz`. NOTE: this re-auths all live prod sessions (intended).

## 4. Land the permanent fix on prod (via main) + CI redesign
- Merge `feat/golive-remediation` → `main` (CI deploys prod). The fix ships flag-OFF on
  prod (fail-closed); boot-guard D + release_command guard active.
- BEFORE/with the merge, apply the CI redesign in `ci-redesign.md` (this folder) —
  `ci.yml` is a protected path needing manual approval. It moves the authenticated E2E
  to a pre-deploy staging gate and replaces the prod job's 4 backdoor-dependent steps
  with the unauthenticated `e2e/tests/prod-smoke.spec.ts`.
- Add GitHub repo secrets for the staging gate: `FLY_API_TOKEN_STAGING`,
  `DEV_AUTH_SECRET` (staging = `stg-e2e-secret`), `JWT_DEV_KID`, `JWT_DEV_PRIVATE_KEY`,
  `JWT_DEV_PUBLIC_KEY`, `DEV_LOGIN_EMAIL`, `DEV_LOGIN_PASSWORD`.

## 5. Confirm closed
- `prod-smoke.spec.ts` against prod: the "dev-login backdoor is closed (prod)" test
  asserts `/api/auth/local/login {test creds}` → 401.
- Forensics: recorded as **near-miss (prod dark)** in `ethical-decisions.md`. The
  confirmatory BYPASSRLS counts (resolution.md, RESOLVE round 2/3) remain owed whenever
  prod DB access is at hand — run them BEFORE any user-row cleanup (CASCADE erases evidence).
