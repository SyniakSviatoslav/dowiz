# Test-Integrity + CI Audit — dowiz/DeliveryOS

**Date:** 2026-07-03 · **Mode:** READ-ONLY (findings only, no edits) · **Branch:** feat/phase0-safety-hardening

Scope: test suites (`apps/api/tests`, `e2e/tests`, `packages/*/tests`), `verify:all` + sub-checks,
guardrail scripts, `eslint-plugin-local` test-integrity rules, `.github/workflows/*`, the pre-commit
hook, the staged CI preflight scripts, and the "prod-safe" post-deploy smoke suites.

Method: direct read of every gate + two parallel subagent deep-dives (E2E suite, API/unit suite).

---

## Severity counts

| Severity | Count |
|----------|-------|
| CRITICAL | 6 |
| HIGH | 8 |
| MED | 7 |
| LOW | 5 |
| **Total** | **26** |

## Single most dangerous false-green

**The CI "Post-deploy E2E regression (core lifecycles)" gate is hollow on prod.**
`e2e/tests/flow-core-lifecycles.spec.ts:54-56` has `test.beforeEach(() => test.skip(isProd, 'mutating
lifecycle — staging only'))` and `:62 if (isProd) return;` in `beforeAll`. The deploy job runs this spec
**only** against prod (`ci.yml:171-176`, `VITE_BASE_URL=https://dowiz.fly.dev`). On prod every one of its
~32 order/courier/settings lifecycle tests skips, and an all-skipped Playwright run **exits 0 = green**.
The named "regression" gate that is supposed to catch a broken prod deploy asserts literally nothing, and
because post-deploy smoke runs *after* `flyctl deploy` with no rollback, a fully broken order pipeline
ships to prod behind a green check.

---

## CRITICAL

### C1 — `flow-core-lifecycles.spec.ts` post-deploy regression gate is a no-op in CI
`e2e/tests/flow-core-lifecycles.spec.ts:54-56,62` · run by `ci.yml:171-176` (prod only).
`beforeEach` skips every test when `isProd`; the deploy job runs the spec exclusively against prod →
0 assertions execute, run reports green. **Risk:** the pipeline's core-lifecycle regression net proves
nothing about the prod deploy it gates; a broken order/courier/settings pipeline is invisible.
**Fix:** run the real (mutating) lifecycle assertions against **staging** in CI, not a fully-skipped spec against prod.

### C2 — `deploy-validation.spec.ts` skips its entire contract surface on the only env CI runs it on
`e2e/tests/deploy-validation.spec.ts` `test.skip(isProd,…)` on 0.1 (l.46), 3.1 (l.100), 4.1/4.2/4.3
(l.120/134/165), 5.1 (l.191), 6.2 (l.233), 7.1 (l.259), 11.1 (l.319) · run by `ci.yml:164-169` (prod only).
Product / recipeLines / attributes / import contract tests all skip on prod. **Risk:** a green
"post-deploy smoke" proves only `/health` 200, SSR 200, and that `/s/demo` still contains "Sushi" — not
that the product/menu contract shipped correctly. **Fix:** point the contract assertions at staging in CI.

### C3 — The entire unit/integration suite never runs in any CI job
`.github/workflows/ci.yml` — `validate` and `fresh-provision` run zero tests; `deploy` runs only 4 E2E
specs. `package.json` `test:unit` (l.35), every `test:phase*` / `test:stage*` (l.46-67), `verify:privacy`
(l.79) are invoked by **no** workflow. **Risk:** the healthiest, genuinely-behavioral tests — money
(`money-tax.test.ts`), order state machine (`order-machine-transitions.test.ts`), authz
(`orders-authz.test.ts` et al.), PII leak detector — are unenforced; they rot and regress silently.
**Fix:** add a CI job that runs `test:unit` (+ the phase5 security tests) against the existing
postgres/redis service already stood up by `fresh-provision`/`visual`.

### C4 — `rls-adversarial` IDOR test still not wired into CI, and silently skips without env
`apps/api/tests/phase5/rls-adversarial.test.ts:8-9,103` — `const PROVISIONED = !!(DATABASE_URL_SESSION
&& DATABASE_URL_OPERATIONAL)` gates `test('H1: Adversarial cross-tenant RLS audit', { skip }, …)`. Same
guard in `integrity.test.ts:8-9`, `websocket-churn.test.ts:9`, `provision-rls.test.ts:24`,
`claim-rls.test.ts:13`. No workflow sets those env vars, and no workflow even calls the script.
**Risk:** cross-tenant RLS isolation — a red-line invariant — has **zero executed proof** in any gate;
the test body is good but never runs. **Fix:** wire `test:phase5-rls-adversarial` into the C3 unit job
with the provisioned service DB URLs exported.

### C5 — `test-stage34.ts` "verifies" security by grepping that the security test files exist
`apps/api/tests/test-stage34.ts:30-39` (R1.2 rls-adversarial), `:71-78` (R2.3 jwt-rotation), `:192-199`
(R7.1 integrity) — each does `readFileSync(<other test>.ts)` + `assert.ok(content)` +
`content.includes('cross-tenant SELECT')`. **Risk:** these pass even if RLS/JWT/idempotency are fully
broken, as long as the *file text* is present — a security-theatre green that masks C4. **Fix:** delete
the file-exists meta-assertions; make the phase5 tests themselves the coverage (C4).

### C6 — Security checks that assert `true` after finding the violation
- `test-stage34.ts:99` (R2.4 "no JWT key defaults in source") collects `violations`, logs a ⚠ if any,
  then `assert.ok(true, 'Checked for JWT key defaults')` — **passes even when it finds hardcoded JWT
  key defaults.** Directly relevant to the known creds-in-source incident.
- `integrity.test.ts:133-163` (R3 "Integer money invariants — CHECK ≥0") only asserts money columns
  *exist* (`:162`), never that any has a CHECK constraint; swallow-catches at `:159,:186`.
- `test-stage13.ts:27-33` — its one real `assert.equal(...func_exists...)` sits in `try/catch` that
  logs and exits 0; `test-stage11.ts` / `test-stage12.ts` are zero-assertion placeholders that always
  pass (`test-stage12.ts:10,25`). **Fix:** replace `assert.ok(true)` / caught-assertion with a failing
  assertion on the real condition; delete placeholder stage files or make them assert.

---

## HIGH

### H1 — Test-integrity ESLint rules exempt the entire `test-stage*` / `test-phase*` suite
`tools/eslint-plugin-local/src/index.js` — every test-integrity rule gates on
`isTestFile = /\.(spec|test)\.(ts|js|tsx|jsx)$/` (`:317,:358,:416,:446`). The 27 files named
`test-stageNN.ts` / `test-phaseN.ts` do **not** match (prefix `test-`, not suffix `.test.`), so
`no-tautological-assertion`, `no-truthy-on-identifier`, `no-permissive-status-assertion`, and
`no-prod-base-in-test` never see them. **Risk:** ~286 grep-style structural assertions and the C6
`assert.ok(true)` violations live precisely in the files the ratchet cannot reach. **Fix:** broaden
`isTestFile` to also match `/(^|\/)test-[^/]*\.ts$/` (and add a rule banning
`assert.ok(<readFileSync>)` / `.includes()` on source reads).

### H2 — `pnpm lint` never fails on warnings → the security-relevant rules are advisory
`package.json:` `"lint": "eslint ."` (no `--max-warnings 0`). Warn-level rules never fail CI:
`no-prod-base-in-test` (warn — comment admits "99 prod-host literals pending"), `no-empty-catch`,
`no-swallowed-catch`, `no-mock-in-prod`, `no-insecure-random`, `no-raw-any`, `no-hardcoded-string`,
`no-direct-websocket` (`eslint.config.js`). **Risk:** the "gate that runs but never gates" class — a test
defaulting its BASE to prod, or a swallowed catch, ships with a yellow squiggle. **Fix:** run
`eslint . --max-warnings 0` in CI, or promote the security-relevant rules to `error`.

### H3 — ~286 grep-style structural assertions across `test-stage31-35`
`test-stage34.ts` (62 `.includes` + 12 file-exists), `test-stage33.ts` (56+7), `test-stage32.ts` (50+5),
`test-stage31.ts` (35+2), `test-stage35.ts` (32+11). Representative: `test-stage33.ts:132-136` (auth hook
"present" = `includes('jwtVerify')`, never rejects a non-owner), `:155-159` (rate limit = `includes('10')`
— any `'10'` passes), `test-stage34.ts:144-154` (security headers grepped from source, no response
inspected), `test-stage31.ts:171-178` (Sentry PII redaction = `includes('email')||includes('phone')`).
**Risk:** these pass whenever the substring exists, regardless of whether the behavior works — false
confidence on auth, rate-limits, CSP, PII redaction. **Fix:** convert to `fastify.inject` / real-request
behavioral assertions (as `health-truthfulness.test.ts` already does correctly).

### H4 — Post-deploy `flow-core-lifecycles` / `deploy-validation` remaining prod assertions are content-coupled
`deploy-validation.spec.ts:348-358` (13.1) asserts `body.toContain('Sushi')` and allergen substrings on
the live `/s/demo` tenant; `9.1/10.1/12.1` silently require `demo` to exist. **Risk:** any demo-content
edit flakes the deploy gate — the class of assertion that gets quietly disabled, further hollowing the
already-thin prod smoke. **Fix:** assert on stable structural/contract shape (status, `x-menu-version`
header shape), not tenant menu text.

### H5 — `verify-secrets` gitleaks scan silently skips; CI never installs gitleaks
`scripts/verify-secrets.ts:22-24` — when `gitleaks` is absent it prints "⚠ gitleaks not installed,
skipping" and does **not** increment `failures`. No workflow installs gitleaks (`ci.yml` "Verify no
secrets in repo" just runs the script). The remaining checks only catch **added `*.env`/`*.pem`/`*.key`
files** (`:84`) + `.env.example` placeholders — **not** secrets embedded in `.ts` source, which is exactly
how the known prod-Supabase-creds incident happened. **Risk:** the secrets gate runs in its weakest form
in CI and would not have caught the incident it exists for. **Fix:** install gitleaks in CI and fail the
step when the scanner is unavailable (skip = fail, not pass).

### H6 — `fresh-provision` job does not gate deploy
`ci.yml:134` `deploy: needs: validate` only; `fresh-provision` (l.57) is a parallel top-level job with no
dependents. **Risk:** a non-bootable fresh DB (the exact bug class the job was built to catch: orders-comma,
dup-policy, MAX(uuid), pgboss-not-installed, missing roles) fails `fresh-provision` but **does not block
prod deploy** on push to main. **Fix:** `deploy.needs: [validate, fresh-provision]`.

### H7 — Deploy runs `migrate:up` on prod with no connection/migration/schema-drift preflight
`ci.yml:150-153` migrates prod directly. The three preflight scripts built for the 2026-07-03 P2/P3/P4
prod-drift saga — `scripts/ci-connection-preflight.mjs`, `ci-migration-preflight.mjs`, `ci-schema-drift.mjs`
— are wired into **no** workflow and referenced by no script (grep: zero refs; confirmed staged-not-applied).
**Risk:** the prod≠staging drift class (RLS keyed on a column prod lacks, `GRANT` to a role prod lacks)
still fails serially on prod, not in CI. **Fix:** add a step running `ci-connection-preflight` +
`ci-migration-preflight` with `SOURCE_URL`=prod (read-only) **before** the `migrate:up` step.

### H8 — `telegram-full-flow.spec.ts` runs in CI vs prod without the prod guard, and its secrets are unset
- The spec was **not** given the `isProdTarget`/`requireStaging` hardening its two siblings got. Its
  `P1-AUTH` posts `/api/dev/mock-auth` (fails closed 404 on prod per `apps/api/src/plugins/dev-guard.ts`),
  and `mode:'serial'` aborts the suite → the CI "Telegram full-flow" step is hard-RED on prod. If mock-auth
  were ever re-enabled it **mutates prod** (creates `tg-e2e-*` location/product) and `P9-CLEANUP` deletes
  only the product, orphaning the location each run.
- `ci.yml:178-190` passes only `VITE_BASE_URL` + `DEV_AUTH_SECRET`; `TELEGRAM_BOT_SECRET`/`TELEGRAM_BOT_TOKEN`
  are never injected → `telegram-webhook.spec.ts:5` builds `…/webhook/telegram/undefined`, so `WEBHOOK-1/9/10`
  can't pass green-for-the-right-reason. **Risk:** two of the four CI-gating specs are broken/misconfigured,
  not safe. **Fix:** apply `requireStaging` to `telegram-full-flow`; inject the telegram secrets or drop the
  steps from the deploy gate.

---

## MED

### M1 — Conditional-skip idiom masks the exact failures the tests guard
Idempotency/lifecycle tests skip green when setup fails: `flow-order-creation.spec.ts:333-337,379-383`
(`if (first.status()!==201){ test.skip(); return }` — broken order creation reports skip, not fail),
`flow-core-lifecycles.spec.ts` Flow 4 (`:177`), Flow 6 (`:203`), Flow 17 (`:408` bare `test.skip()`),
Flow 18 (`:442-443` `.catch(()=>({status:()=>503}))` then skip); same anti-pattern in
`flow-geo-tracking.spec.ts:56`, `flow-customer-track-link.spec.ts:71`, `flow-offline-phone-fallback.spec.ts:69`.
**Fix:** in these, a missing prerequisite from a *should-have-succeeded* setup must `expect(...).toBe(201)`
(fail), not `test.skip`.

### M2 — Placeholder / delegated `assert.ok(true)` in phase/stage tests
`test-phase2.ts:26,31,36,41` (menu_version atomicity, PII redaction, 0-cookies, Zod strictness — all
`assert.ok(true,'Verified via …')`), `test-stage26.ts:412,547`, `test-stage28.ts:685`, `test-stage32.ts:50`
(`… || true`). **Risk:** named coverage for money-atomicity and PII that asserts nothing. **Fix:** implement
the real assertion (route inject / DB check) or delete the subtest.

### M3 — `jwt-rotation.test.ts` tests the `jose` library, not the app
`apps/api/tests/phase5/jwt-rotation.test.ts:11-12,55-59,77-81` — generates its own keys and an inline
`keyLookup`, never imports dowiz's signing/verification/rotation code. **Risk:** R1-R4 prove `jose`
round-trips; the app's kid-rotation behavior is unproven behind a green "JWT rotation test." **Fix:**
import the real verifier and assert old-kid acceptance / unknown-kid rejection.

### M4 — Ledger-integrity gate is currently RED (rows 39-42 duplicated) yet unnoticed
`docs/regressions/REGRESSION-LEDGER.md` — rows 39,40,41,42 each appear twice (lines 27-30 vs 138-141);
`node scripts/guardrail-ledger-integrity.mjs` exits 1, so `verify:all --ci` currently **fails**. There is
**no pre-push hook** and CI only runs on PR/push-to-main, so the red gate has not surfaced — meaning
`verify:all --ci` is not being run pre-merge on this feature branch. **Risk:** the gate works but the tree
violates it; the "#N" cross-reference scheme is ambiguous again. **Fix:** suffix the second batch
(39b-42b); consider a `pre-push` hook running `verify:all --ci`.

### M5 — No gate bans `.only` / bare `test.skip` / `if(!env)return` in node `--test` files
Playwright's root config sets `forbidOnly:true` (good — no `.only` leaks in E2E), but the `node --test`
stage/phase files have no equivalent, and no ESLint rule bans a bare `test.skip(true)` or an
`if(!process.env.X) return` no-op. **Risk:** silent-skip false-greens (C4, M1, M6) are structurally
uncatchable. **Fix:** add a lightweight lint rule / grep gate for bare skips + env-guarded early returns in test files.

### M6 — Silent-skip integration tests (whole test no-ops when env URL unset)
`const maybe = url ? test : test.skip` across `deliver-*.test.ts`, `flow-simpl-*.test.ts`,
`extraction-orchestrator.test.ts`, `acquisition-service.test.ts`, `retention-sweep.test.ts`,
`provision-verifier.test.ts`, `notifications/*.test.ts`, `keyset-pagination.test.ts:56`. Honestly labelled
skips, but combined with C3 (never run in CI) the covered behavior is unproven anywhere. **Fix:** provision
the DB URL in the C3 CI job so these actually execute.

### M7 — Visual + skill-security workflows never run on push to main
`.github/workflows/visual.yml:6-13` and `skill-security.yml:9-14` are `pull_request`-only and path-filtered;
`visual.yml` also misses renderer-affecting changes outside `apps/web`/`packages/ui`. **Risk:** a direct
push to main (or a rendering change via a config file outside the filter) skips visual regression entirely.
**Fix:** add `push: branches:[main]` or rely on enforced branch protection; widen the visual path filter.

---

## LOW

### L1 — Pre-commit Docker build auto-skips on failure (confirmed)
`.husky/pre-commit` (step 5/5) — `docker build … || { echo "Warning: Docker build failed (local
network/env issue) … skipping."; }` (no `exit 1`), plus a low-disk skip and a "Fly CLI not found" skip.
**Risk:** a genuine Dockerfile/build break passes pre-commit; only the cloud build (which gates deploy, but
after commit) catches it. Same auto-skip class as H5. **Fix:** distinguish network errors from build errors,
or downgrade to informational and rely on the cloud build gate explicitly (already the intent — but the
"successful!" path is indistinguishable from the skip in the log).

### L2 — 165 / 169 E2E specs never run in any gate
Only the 4 deploy-job specs execute in CI (2 neutered on prod per C1/C2). The broad regression net
(auth-isolation, security-contracts, order-lifecycle, a11y) rots (hardcoded Tirana coords, `demo` content)
with no detection. **Fix:** a scheduled/staging E2E job over the full suite (`VITE_BASE_URL`=staging).

### L3 — 60 E2E files hardcode the prod host; only 44 `requireStaging` usages
`grep dowiz.fly.dev e2e/tests` = 60 files; `requireStaging` = 44 usages. `no-prod-base-in-test` is warn-only
(H2) so this never blocks. **Risk:** a mutating spec run without a `VITE_BASE_URL` override writes to prod.
**Fix:** finish the `requireStaging` wave and flip the rule to error.

### L4 — Inflated timeouts + fixed sleeps + broad catch-swallows
`rsi-round.spec.ts:20` `setTimeout(600_000)`; several `180_000`; 360 `waitForTimeout` fixed sleeps;
145 `.catch(()=>…)` occurrences (mostly legit cleanup, but the idiom hides a genuinely-failing restore).
**Fix:** replace fixed sleeps with `expect.poll`/`waitFor`; ensure cleanup catches log.

### L5 — Post-deploy smoke has no rollback; `fresh-provision` migrate-count is cosmetic
Deploy-then-smoke with no rollback step means even a truthful red = prod already broken; combined with the
C1/C2 hollow gates, detection is late *and* blind. Minor: `scripts/verify-fresh-provision.sh:102` `APPLIED`
count uses `|| true` (0 doesn't fail — cosmetic only; the real `migrate:up` failure still fails). **Fix:**
add a `flyctl deploy` image-pin rollback on smoke failure.

---

## What is genuinely solid (for contrast, not findings)
- Behavioral, real tests: `money-tax.test.ts`, `order-machine-transitions.test.ts`, `pii-leak-detector.test.ts`,
  `menu-parse/eval.test.ts`, `health-truthfulness.test.ts` (fastify.inject, real 503), and the authz suite
  (`orders-authz.test.ts` et al. self-bootstrap env so they actually run).
- `scripts/verify-fresh-provision.sh` has real end-to-end assertions (migrate→seed→boot→/health 200→menu products).
- The three CI preflight scripts (`ci-connection`/`ci-migration`/`ci-schema-drift`) are well-built and
  read-only — the only gap is that they are unwired (H7).
- The `no-tautological-assertion` / `no-truthy-on-identifier` / `no-permissive-status-assertion` rules are
  correctly `error`-level; the gap is coverage (H1) and warn-only siblings (H2), not the rules themselves.
