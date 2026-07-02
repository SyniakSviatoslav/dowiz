# CI test gates — manual apply (`.github/workflows/ci.yml` is a protected zone)

Closes the review finding: **the 133-file unit suite and `verify:all` run nowhere in CI** —
the only automated test execution today is 4 Playwright specs fired at prod *after* deploy.
Everything below is proven locally (2026-07-02):

- `pnpm test:unit`: 857 tests / 0 fail on a fresh migrated+seeded DB (serial run; see note 3)
- `pnpm exec tsx scripts/verify-all.ts --ci`: ALL PASSED (the new `--ci` flag runs the static
  subset — env/db/rls need a provisioned environment; lint/typecheck already run as CI steps)

## 1. `validate` job — add after the "Compliance gate" step

```yaml
      # The project's own named pre-deployment gate (guardrails, i18n coverage, contrast,
      # DEFINER search_path, …). --ci = static subset: verify:env/db/rls need a provisioned
      # environment; lint/typecheck already ran as dedicated steps above.
      - name: Guardrail gate (verify:all --ci)
        run: pnpm exec tsx scripts/verify-all.ts --ci
```

## 2. `fresh-provision` job — add after the "Verify fresh-from-scratch provisioning" step

The job already has postgres+redis services, roles, migrations, seed — exactly what the
DB-backed unit/integration tests need. `--test-concurrency=1` because parallel test FILES
share one DB and interfere (proven flake: access-requests consent gate vs rate-limit state).

```yaml
      - name: Unit + integration tests (fresh provisioned DB)
        env:
          SAG_TEST_DB_URL: postgresql://dowiz_migrator:migrator_pw@127.0.0.1:5432/dowiz_freshcheck?sslmode=disable
        run: >
          node --test --test-concurrency=1 --import tsx
          'apps/api/tests/**/*.test.ts' 'apps/worker/tests/**/*.test.ts'
          'apps/web/src/**/*.test.ts' 'packages/**/*.test.ts' 'tools/**/tests/**/*.test.ts'
```

(Direct `node --test` invocation instead of `pnpm test:unit` only to pass
`--test-concurrency=1` — package.json is also protect-paths. If you prefer, add
`"test:unit:ci": "node --test --test-concurrency=1 …"` to package.json and call that.)

## 3. `deploy` job — gate on the tests

```yaml
  deploy:
    needs: [validate, fresh-provision]
```

(currently `needs: validate` only — a fresh-provision/test failure would not stop a prod deploy.)

## Notes

- The live-stack tests (phase5/, websocket-churn, test-stage*) skip themselves with an explicit
  reason when `DATABASE_URL_*` is absent — they do NOT fail the validate job.
- Backlog: `proposed-sense4-ci/APPLY.md` (2026-06-26, LHCI schedule trigger) was never applied —
  consider batching both in one reviewed ci.yml commit.
