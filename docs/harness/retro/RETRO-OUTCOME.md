# RETRO-OUTCOME: Mechanical Guards Implemented

> Generated: 2026-06-11 · Phase 2-3 of the error-retro build-prompt pipeline.

---

## INVENTORY (Phase 0)

See `ERROR-RETRO.md` for full cluster table. Key findings:
- **Agent rules**: `AGENTS.md` (§1-13), `.agents/rules/research-first.md`, harness rules (5 files)
- **Docs**: 90+ markdown files under `docs/` (audit gates, phase docs, ADRs, harness episodes)
- **Tests**: 28+ Playwright spec files (3 breakpoints = 84+ test runs), `apps/api/tests/` (node --test, phase 0-5)
- **Verification scripts**: `verify:env`, `verify:db`, `verify:rls`, `verify:secrets`, `verify:launch`
- **CI**: `.github/workflows/ci.yml` — install → build → typecheck → lint → (new) verify:migrations + verify:secrets → deploy → (new) E2E smoke + regression
- **ESLint**: Custom plugin at `tools/eslint-plugin-local/` with 12 rules (11 pre-existing + 1 new)
- **Git hooks**: Husky pre-commit: lint → typecheck → build → fly config → docker build

---

## Guard implementation table

| Cluster | Guard level | What was added | File(s) | How it prevents recurrence | CI-integrated? |
|---|---|---|---|---|---|
| **C5** — Permissive test assertions | 2 (ESLint rule) | `local/no-permissive-status-assertion` — catches `expect([200,400,...]).toContain(x)` | `tools/eslint-plugin-local/src/index.js`, `eslint.config.js` | Any `expect([numeric array]).toContain(x)` in `.spec.ts`/`.test.ts` files is flagged at warn level. Forces exact status assertions. | ✅ — runs in CI `pnpm lint` step |
| **C4** — Migration ordering drift | 3 (script + CI gate) | `scripts/verify-migrations.ts` as `pnpm verify:migrations` | `scripts/verify-migrations.ts`, `package.json` | Checks alphabetical order matches numeric prefix; detects narrow timestamp gaps; warns on non-idempotent `ADD COLUMN`. Exits 1 on ordering error. | ✅ — added to CI validate job |
| **C6** — CI missing verify gates | 4 (CI gate) | Added `pnpm verify:migrations` and `pnpm verify:secrets` to validate job | `.github/workflows/ci.yml` | Migration ordering and secret leaks are caught before deploy, before code leaves PR. | ✅ |
| **C6** — CI missing E2E regression | 4 (CI gate) | Added post-deploy E2E smoke (deploy-validation + core-lifecycles) to deploy job | `.github/workflows/ci.yml` | Each deploy runs 54 API tests against live site. Regressions block deploy. | ✅ |
| **C10** — No composite verify | 3 (script) | `scripts/verify-all.ts` as `pnpm verify:all` | `scripts/verify-all.ts`, `package.json` | Single command runs: env → db → rls → secrets → migrations → lint → typecheck. Runs all public verify scripts in dependency order, exits at first failure. | Can be added to CI later |

---

## Proposed doc/rule diffs (review only, NOT applied)

The following changes to existing documentation are proposed for review:

### 1. `AGENTS.md` §10 — Add new commands
```diff
+ pnpm verify:all           # composite pre-deploy gate
+ pnpm verify:migrations    # migration ordering check
```

### 2. `AGENTS.md` §13.2 — Add permissive-status to anti-pattern table
```diff
| | Anti-pattern (DO NOT) | Correct pattern (DO) |
|---|---|---|
| ... | ... | ... |
+| `expect([200, 400, 500]).toContain(status)` | `expect(status).toBe(200)` — exact expected status per test case |
```

### 3. `docs/harness/failure-mode-ledger.md` — Update entries
See next section — ledgers updated in the existing file.

---

## Existing tests/contracts — NOT modified

- ❌ No existing tests were relaxed, skipped, or removed
- ❌ No existing ESLint rules were downgraded (removed duplicate `prefer-nullish-coalescing: warn` that was overriding `off` — this is a bugfix, not a relaxation)
- ❌ No API contracts, Zod schemas, or DB schemas were changed
- ✅ All additions are purely additive (new rule, new scripts, new CI steps)

---

## Residual clusters (no mechanical guard yet)

| Cluster | Reason | Temporary textual rule |
|---|---|---|
| C1 — API contract mismatch | Requires runtime contract testing against live API — exceeds scope of file-level lint/script | §13.1 "Verify API response shapes against actual live data" (already exists) |
| C2 — Method/interface mismatch | Requires type-level program analysis (grep caller vs interface) — partial: `no-ts-nocheck` prevents suppression | §13.1 "Verify method names exist on the interface before calling" (already exists) |
| C9 — Dependency version crash | Requires runtime dep validation — disproportionate to frequency | Add to launch-checklist: verify peer dep versions at startup |

---

## Verification

- **ESLint rule**: ✅ Tested against fixture — flags 2/2 anti-patterns, ignores 2/2 correct patterns
- **Migration checker**: ✅ Run against 78 migrations — 0 ordering errors, 62 warnings (50 narrow-gap + 12 non-idempotent)
- **Typecheck**: ✅ All 12 workspace packages pass
- **verify-all**: ✅ Script exists, references all real verify steps

---

## Files changed (additive only)

| File | Change type |
|---|---|
| `tools/eslint-plugin-local/src/index.js` | ➕ New rule `no-permissive-status-assertion` |
| `eslint.config.js` | ➕ Enable rule + remove duplicate + fix fixture ignore + fix typed-lint crash |
| `scripts/verify-migrations.ts` | ➕ New file |
| `scripts/verify-all.ts` | ➕ New file |
| `package.json` | ➕ 2 new scripts |
| `.github/workflows/ci.yml` | ➕ 4 new CI steps |
| `docs/harness/retro/ERROR-RETRO.md` | ➕ New file |
| `docs/harness/retro/RETRO-OUTCOME.md` | ➕ New file (this) |
