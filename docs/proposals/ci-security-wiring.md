# CI security wiring — operator paste-in (security-hardening batch 2026-07-02)

**Status:** DESIGNED, NOT APPLIED. `.github/workflows/ci.yml` and root `package.json` are protect-path
(hook-blocked). Operator copies the exact blocks below. Source of truth: `resolution.md` §"#3" and §"#2",
`proposal.md` §10 + §"SAFE riders".

Three changes, all in `.github/workflows/ci.yml` except (b)'s override line in root `package.json`:

- **(a)** Run the DB-backed adversarial guardrails (`phase5-rls-adversarial` + `jwt-rotation` +
  `integrity`) in the **`fresh-provision`** job — which already stands up Postgres + roles. This is the
  highest-value free fix: the IDOR / cross-tenant `rls-adversarial` guardrail has **never run in CI**
  (it is `--env-file=.env`, absent from `verify:all --ci`). A provisioned DB already exists in that job.
- **(b)** Add `pnpm audit --audit-level high` + the `tmp@>=0.2.6` pnpm override (SAFE rider, `proposal.md`
  §"SAFE riders" — DEV/CI-only `@lhci/cli` path-traversal, not a prod risk).
- **(c)** Add the runtime **definer proconfig probe** (`verify:rls`) to the `fresh-provision` job — the
  only CI job with a live DB, so it can prove the `#3` search_path pin is actually present at runtime
  (the static `guardrail-definer-search-path.mjs` in `verify:all --ci` catches new offenders but cannot
  prove the LIVE pin — `resolution.md` §"#3" honest limitation).

---

## (a) + (c) — `fresh-provision` job: run adversarial guardrails + verify:rls against the provisioned DB

The `fresh-provision` job today provisions a bare Postgres, creates `dowiz_migrator` + `dowiz_app`
roles, generates a non-prod secrets env-file (`.env.ci-fresh`), and runs `pnpm verify:fresh-provision`.
Those tests read `--env-file=.env`, so we point them at a `.env` that carries the provisioned DB URLs.

### BEFORE (tail of the `fresh-provision` job — last step)

```yaml
      - name: Verify fresh-from-scratch provisioning
        env:
          SUPERUSER_URL: postgresql://postgres:postgres@127.0.0.1:5432/postgres
          SECRETS_ENV_FILE: .env.ci-fresh
          REDIS_URL: redis://127.0.0.1:6379
        run: pnpm verify:fresh-provision
```

### AFTER (append these steps to the END of the `fresh-provision` job, after the step above)

```yaml
      - name: Verify fresh-from-scratch provisioning
        env:
          SUPERUSER_URL: postgresql://postgres:postgres@127.0.0.1:5432/postgres
          SECRETS_ENV_FILE: .env.ci-fresh
          REDIS_URL: redis://127.0.0.1:6379
        run: pnpm verify:fresh-provision

      # security-hardening 2026-07-02 (a)+(c): the DB-backed guardrails have never run in CI because
      # they are `tsx --env-file=.env …`. The fresh-provision job is the ONLY CI job with a live,
      # migrated Postgres — wire them here. Build a `.env` from the CI secrets + the provisioned DB URLs
      # so the `--env-file=.env` scripts resolve. `dowiz_app` is BYPASSRLS here (bare PG, matches Case A),
      # so the adversarial suite runs against the app pool exactly as staging/prod does.
      - name: Compose .env for DB-backed guardrails
        run: |
          cp .env.ci-fresh .env
          {
            echo "DATABASE_URL_MIGRATIONS=postgresql://dowiz_migrator:migrator_pw@127.0.0.1:5432/postgres"
            echo "DATABASE_URL_OPERATIONAL=postgresql://dowiz_app:app_pw@127.0.0.1:5432/postgres"
            echo "DATABASE_URL_SESSION=postgresql://dowiz_app:app_pw@127.0.0.1:5432/postgres"
            echo "REDIS_URL=redis://127.0.0.1:6379"
          } >> .env

      # (c) runtime definer proconfig probe — proves the #3 search_path pin is LIVE (not just that a new
      # offender would be caught by the static gate). Fails if app_member_location_ids lacks the pin.
      - name: Verify RLS + definer search_path pin (runtime)
        run: pnpm verify:rls

      # (a) adversarial IDOR / cross-tenant guardrail — the C1/#1/#7 isolation proofs. Never ran in CI.
      - name: Phase5 RLS adversarial (cross-tenant IDOR)
        run: pnpm test:phase5-rls-adversarial

      - name: Phase5 JWT rotation
        run: pnpm test:phase5-jwt-rotation

      - name: Phase5 integrity
        run: pnpm test:phase5-integrity
```

> **Operator note:** confirm `verify:fresh-provision` (`scripts/verify-fresh-provision.sh`) leaves the
> Postgres migrated and the two roles present at these credentials (it creates them earlier in the job at
> `migrator_pw` / `app_pw`). If that script tears the DB down at exit, move the `.env` compose + the four
> new steps to run against the same service *before* any teardown, or re-run `pnpm migrate:up` in the
> compose step. Each of the four new steps is red→green provable: they FAIL on an un-migrated / unpinned /
> IDOR-leaking DB and PASS on the correctly-provisioned one.

---

## (b) — `pnpm audit` gate + `tmp@>=0.2.6` override

### package.json — root `pnpm.overrides` (protect-path)

**BEFORE**

```json
  "pnpm": {
    "overrides": {
      "form-data@<4.0.6": ">=4.0.6",
      "undici@<6.27.0": ">=6.27.0"
    }
  },
```

**AFTER**

```json
  "pnpm": {
    "overrides": {
      "form-data@<4.0.6": ">=4.0.6",
      "undici@<6.27.0": ">=6.27.0",
      "tmp@<0.2.6": ">=0.2.6"
    }
  },
```

> pnpm override key syntax is `tmp@<0.2.6` (the vulnerable range) → `>=0.2.6` (the resolution), matching
> the two existing entries. `tmp` is a transitive dev-only dep (`@lhci/cli` path-traversal), so this does
> not touch the prod bundle/image.

### ci.yml — add a `pnpm audit` step to the `validate` job

**BEFORE** (start of the `validate` job steps, after install)

```yaml
      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Build
        run: pnpm -r build
```

**AFTER**

```yaml
      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      # security-hardening 2026-07-02 (b): fail CI on a known HIGH/CRITICAL advisory in the dep tree.
      # The tmp@>=0.2.6 override (root package.json) clears the current @lhci/cli finding so this is
      # green today; new HIGH advisories then break the build instead of rotting silently.
      - name: Audit dependencies (high+)
        run: pnpm audit --audit-level high

      - name: Build
        run: pnpm -r build
```

> **Operator note (audit-level tuning):** if a transitive advisory with no available fix would block
> merges, either add a targeted `pnpm.overrides` bump (preferred — same pattern as `tmp`) or, as a last
> resort, downgrade this one step to `|| true` with a tracked ledger row — never weaken it silently. Keep
> `--audit-level high` (not `moderate`) so it stays a genuine gate, not noise.

---

## Apply order for this file

1. Add the `tmp@<0.2.6` override to root `package.json`; run `pnpm install` so the lockfile updates.
2. Paste the `pnpm audit` step (b) and the four `fresh-provision` steps (a)+(c) into `ci.yml`.
3. Push a branch / open the PR — the new steps run on PR (both `validate` and `fresh-provision` fire on
   `pull_request`). Confirm all four DB-backed steps and the audit step are GREEN before merge.
