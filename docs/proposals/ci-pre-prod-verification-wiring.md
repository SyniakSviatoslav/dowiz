# CI pre-prod verification wiring — operator paste-in

**Status:** DESIGNED, NOT APPLIED. `.github/workflows/ci.yml` and root `package.json` are protect-path
(hook-blocked). Operator applies the blocks below. Rationale + costs:
`docs/design/ci-pre-prod-verification/proposal.md`. Causal analysis:
`docs/reflections/INBOX/ci-pre-prod-verification-2026-07-03.md`. Scripts: `scripts/ci-*.mjs` (implemented,
dry-run + self-test proven).

Four changes: (a) a `preflight` job the `deploy` job `needs:`; (b) a `staging-verify` job on the same
commit that `deploy` also `needs:`; (c) split the deploy-validation smoke by target; (d) branch
protection + a pre-push hook. All new secrets are named; none are hardcoded.

---

## New secrets the operator must add (GitHub → Settings → Secrets → Actions)
- `DATABASE_URL_OPERATIONAL` — runtime app-pool URL (transaction pooler @6543), **with `?sslmode=no-verify`**.
- `DATABASE_URL_SESSION` — runtime session-pool URL, with `?sslmode=no-verify`.
- `PROD_READONLY_URL` — a **SELECT-only** prod role for schema introspection (mint:
  `CREATE ROLE ci_readonly LOGIN PASSWORD '…'; GRANT USAGE ON SCHEMA public TO ci_readonly;
   GRANT SELECT ON ALL TABLES IN SCHEMA public TO ci_readonly; GRANT SELECT ON pg_catalog.pg_roles TO ci_readonly;`),
  with `?sslmode=no-verify`. Used read-only by the migration preflight + schema-drift.
- `STAGING_DATABASE_URL_MIGRATIONS` — staging migrator URL (for the `staging-verify` migrate step).
- (existing) `DATABASE_URL_MIGRATIONS`, `FLY_API_TOKEN`, `DEV_AUTH_SECRET`, plus a
  `STAGING_FLY_API_TOKEN` scoped to `dowiz-staging` if not already present.

---

## (a) NEW `preflight` job — runs BEFORE migrate+deploy; `deploy` needs it

Add this job to `.github/workflows/ci.yml`. It gates P2/P4/P5 (connection preflight) and P3
(migration preflight LIGHT against prod, read-only).

```yaml
  # Pre-prod verification gate (2026-07-03). Fails the pipeline BEFORE migrate:up / flyctl deploy
  # if any DB secret can't connect (P2/P4), SSL is misconfigured (P4/P5), or a pending migration
  # references a table/column/role absent on prod (P3). Uses the SAME secrets the deploy job uses,
  # so a wrong secret fails HERE naming the store — not on prod.
  preflight:
    needs: validate
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v3
        with: { version: 9.4.0 }
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: 'pnpm' }
      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      # P2/P4 — the migrator secret THIS pipeline will use must connect with working SSL.
      - name: Connection preflight (migrator)
        run: node scripts/ci-connection-preflight.mjs
        env:
          DATABASE_URL_MIGRATIONS: ${{ secrets.DATABASE_URL_MIGRATIONS }}

      # P5 — runtime pools must connect under SSL (catches the "block non-SSL" outage class).
      - name: Connection preflight (runtime pools)
        run: node scripts/ci-connection-preflight.mjs --require-all
        env:
          DATABASE_URL_OPERATIONAL: ${{ secrets.DATABASE_URL_OPERATIONAL }}
          DATABASE_URL_SESSION: ${{ secrets.DATABASE_URL_SESSION }}

      # P3 — pending migrations must apply against PROD's ACTUAL schema (read-only).
      - name: Migration preflight (LIGHT — vs prod schema)
        run: node scripts/ci-migration-preflight.mjs
        env:
          SOURCE_URL: ${{ secrets.PROD_READONLY_URL }}

      # P3 — staging must not have drifted from prod on migration-referenced tables.
      - name: Schema drift (staging vs prod)
        run: node scripts/ci-schema-drift.mjs
        env:
          LEFT_URL: ${{ secrets.STAGING_DATABASE_URL_MIGRATIONS }}
          RIGHT_URL: ${{ secrets.PROD_READONLY_URL }}
```

**Optional stronger P3 (Option A / FULL):** add a step that clones prod schema into a scratch PG and
runs the real migrations (needs a `services: postgres:` block like `fresh-provision`, plus `pg_dump`):
```yaml
      - name: Install postgres client
        run: sudo apt-get update && sudo apt-get install -y postgresql-client
      - name: Migration preflight (FULL — pg_dump prod → scratch → migrate)
        run: node scripts/ci-migration-preflight.mjs --full
        env:
          SOURCE_URL: ${{ secrets.PROD_READONLY_URL }}
          SCRATCH_URL: postgresql://postgres:postgres@127.0.0.1:5432/postgres
```

## (b) NEW `staging-verify` job — same commit to staging + E2E; `deploy` needs it (P1, P6-pre)

Encodes Ship Discipline (commit→staging→validate→prod) as CI topology. The mutating deploy-validation
smoke runs HERE against staging — its authored, dev-login-enabled target.

```yaml
  staging-verify:
    needs: [validate, preflight]
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v3
        with: { version: 9.4.0 }
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: 'pnpm' }
      - name: Install dependencies
        run: pnpm install --frozen-lockfile
      - name: Migrate staging DB
        run: pnpm migrate:up
        env:
          DATABASE_URL_MIGRATIONS: ${{ secrets.STAGING_DATABASE_URL_MIGRATIONS }}
      - uses: superfly/flyctl-actions/setup-flyctl@master
      - name: Deploy this commit to staging
        run: flyctl deploy -a dowiz-staging --remote-only
        env:
          FLY_API_TOKEN: ${{ secrets.STAGING_FLY_API_TOKEN }}
      - name: Install Playwright browsers
        run: npx playwright install chromium
      - name: E2E against staging (mutating deploy-validation + core lifecycles)
        run: |
          npx playwright test e2e/tests/deploy-validation.spec.ts e2e/tests/flow-core-lifecycles.spec.ts \
            --project=desktop --reporter=list
        env:
          VITE_BASE_URL: "https://dowiz-staging.fly.dev"
          DEV_AUTH_SECRET: ${{ secrets.DEV_AUTH_SECRET }}
        timeout-minutes: 15
```

## (c) `deploy` job — depend on the new gates + replace the prod smoke (P1, P6-post)

**Change the `deploy` job header** so prod only proceeds after preflight + staging are green on this commit:
```yaml
  deploy:
    needs: [validate, preflight, staging-verify]   # was: needs: validate
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
```

**Replace** the four post-deploy E2E steps (which currently run mutating, dev-login-dependent specs
against prod — P6) with a single **read-only prod smoke** that can actually be green on prod:
```yaml
      # Read-only prod smoke — NO dev-login (prod closes the DEV_AUTH backdoor, ADR-0003), NO writes.
      # Gate the release on symptoms an operator cares about; the deep/mutating E2E already ran on staging.
      - name: Post-deploy prod smoke (read-only)
        run: npx playwright test e2e/tests/prod-smoke.spec.ts --project=desktop --reporter=list
        env:
          VITE_BASE_URL: "https://dowiz.fly.dev"
        timeout-minutes: 3
```
`e2e/tests/prod-smoke.spec.ts` (new — author separately; NOT protect-path, safe to add) asserts, with
only `request.*` / read-only page loads: `GET /health` → 200; a published `/s/:slug` menu renders a
known product; `GET /api/owner/locations` (unauth) → 401. No mock-auth, no create/delete.

> Note (updated 2026-07-03): `deploy-validation.spec.ts` is now **prod-safe and self-partitioning** — it
> no longer throws against prod. Its home is still `staging-verify` (b) where it runs the FULL suite
> (login → create category/product → PATCH → image upload → import preview → API round-trip); but the
> SAME spec, pointed at `VITE_BASE_URL=https://dowiz.fly.dev`, now `test.skip(isProdTarget(BASE), …)`s
> every mutating / owner-token-dependent test (0.1 login, 3.1, 4.x, 5.1, 6.2, 7.1, 11.1) and runs only
> the read-only subset (1.x/2.x auth-401 + 400-not-500, 6.1 upload-401, 8.1 health, 9.1 SSR+menu-version,
> 10.1 dashboard-SPA, 12.1 theme, 13.1 public-menu render). Proven: **11 passed / 9 skipped / exit 0**
> against prod; **full mutating run green on staging** (`isProdTarget` false there). So the read-only
> prod smoke below can either be a dedicated `prod-smoke.spec.ts` OR simply this spec pointed at prod —
> it is now safe to run post-deploy against the live host without writing to the storefront.

## (d) `verify:all` as a required status check + pre-push hook (P1)

**Branch protection (operator, GitHub Settings → Branches → `main`):**
- Require status checks to pass before merging → select **`validate`**, **`preflight`**, **`staging-verify`**.
- Require branches to be up to date before merging (so the checks ran on the merge commit — this is the
  direct fix for "merged to main while never CI-green").
- Restrict who can push to `main`; require PRs.

**Pre-push hook (proposal — add to `.husky/pre-push` or the repo's hook runner):** run a fast subset so
the loop shifts left of CI:
```sh
# .husky/pre-push (proposed) — fail fast before pushing; full verify:all still gates in CI.
pnpm verify:all --ci || { echo "verify:all --ci failed — fix before pushing"; exit 1; }
```
Keep the *full* `verify:all` as the CI-required check (some gates need a DB); the pre-push runs the
static `--ci` subset that already exists, so it's fast and catches the cheap classes before they reach CI.

---

## Resulting DAG
```
validate ─┬─> preflight ─┬─> staging-verify ─┐
          └──────────────┘                    ├─> deploy (prod) ─> read-only prod smoke
                                              ┘
fresh-provision (unchanged, parallel)
```
Nothing reaches prod `migrate:up` / `flyctl deploy` until: all secrets connect (P2/P4/P5), pending
migrations validate against prod's real schema (P3), staging has no drift (P3), and the SAME commit
migrated + passed E2E on staging (P1/P6). The prod smoke is read-only and green-able on prod (P6).
