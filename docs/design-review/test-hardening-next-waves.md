# Test-Hardening — next waves (post usage-reset) · 2026-06-27

Deterministic, no-staging-or-Council work queued for when the usage limit resets (10:20pm UTC).
Ordered by leverage. Source of truth for the remaining file list: `scratchpad/harden-map.json`.

## WAVE B1 — 🔴 Wire the test suites into CI (HIGHEST LEVERAGE — the root cause)

**Finding:** `.github/workflows/ci.yml` runs build · typecheck · lint · lint:gates · verify:migrations ·
verify:secrets · compliance:gate — **but NEVER runs `node --test` (the ~90 unit `*.test.ts`) or
`playwright test` (the 151 e2e specs).** The entire suite is dead-green: it cannot fail a merge, which
is *why* 2,023 false-greens accumulated unnoticed. Fixing this makes every other hardening actually bite.

Steps:
1. Add a root `test:unit` script: `node --test --import tsx "apps/api/tests/**/*.test.ts"
   "apps/worker/tests/**/*.test.ts" "packages/**/*.test.ts" "tools/**/tests/**/*.test.ts"` (+ the
   `_env-stub` already lets the pure ones run infra-free; infra-gated ones skip cleanly).
   ⚠️ root `package.json` is protect-paths-gated → **escalate / operator applies** (proposal queued).
2. Add a CI job step `- run: pnpm test:unit` after Typecheck.
3. Add a CI job (or nightly) `- run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright
   test --grep-invert @needs-staging` against a deployed staging — at minimum the deterministic specs;
   tag the `needs_staging` ones `@needs-staging` and run those only when a 2nd-tenant fixture exists (Wave A).
4. Prove red→green: a known-failing assertion must fail CI before this is "done".

## WAVE B2 — Relocate the one dead Playwright spec

- `apps/api/e2e/api-integrity.spec.ts` is OUTSIDE both testDirs (`e2e/tests`, `e2e/lifecycle-e2e`) → never
  runs. Unlike phase2 it has REAL assertions (health/CSP/401/SPA/menu-schema) + one in-memory tautology
  section (the OWNER_BE_PRODUCT_FIELDS array-vs-array, ~L195-221) + an unguarded `categories[0]` (~L83).
- Action: move to `e2e/tests/api-integrity.spec.ts`; replace the in-memory comparison with a live
  authenticated `GET /api/owner/menu/products` field-presence assertion; guard `categories[0]`; add
  `requireStaging(BASE)`; verify green on staging. (Needs the post-reset usage + a staging run.)

## WAVE B3 — The remaining ~199 CRIT/HIGH files

- Most are already lint-0; their leftover findings are **needs_staging** (see Wave A). Re-running dry
  agents yields ~no new edits. Only run a final ≤2-workflow pass to catch the ~23 that were usage-limited
  mid-run (their lint/route-determinable fixes); everything else routes to Wave A.

## Concurrency rule (learned)
15 workflows (240 agents) → API rate-limit; 5 (80) borderline; **2 workflows (~32) = safe**. Never exceed.

## Escalated (review queue — NOT in these waves; need Council/infra)
money-integer-guard · subdomain-dot-tld · dev-guard-exact · routing-test-seam (red-line product fixes) ·
C cross-tenant-2nd-tenant · D red-line money/RLS/PII vacuous proofs. → Wave A provides the 2nd-tenant fixture.
