# PROPOSED — Critical-Path Visual Regression Net (GitHub Actions + Runbook)

> **Status: PROPOSAL.** `.github/` is protect-paths-blocked for the agent, so this
> file is the operator's copy-paste source. Nothing here is live until the operator
> creates `.github/workflows/visual.yml` from the YAML below and commits it.
>
> This is the deploy/CI half of the Critical-Path Visual Regression Net. The test
> assets already exist on this branch and reference this doc by path:
> - `playwright.visual.config.ts` — the visual-only Playwright config (3 viewport projects, perceptual threshold, frozen motion/time/tz).
> - `e2e/visual/global-setup.ts` — seeds fixtures once via `/api/dev/seed-visual-state`.
> - `e2e/visual/harness.ts` — `seedVisualState` / `loginAs` / `applyAuth` / `setLocale` / `MASK` / `settle`.
> - `e2e/visual/__screenshots__/` — committed baselines (created by the runbook below).

---

## Versions used (read from the repo, not guessed)

| Thing | Value | Source |
|---|---|---|
| Postgres | **16** | `.github/workflows/ci.yml` `services.postgres.image: postgres:16` (the existing `fresh-provision` job) |
| Redis | **7** | `.github/workflows/ci.yml` (API boots `ioredis` + pg-boss; `apps/api/src/server.ts:476`) |
| Node | **22** | `package.json` `engines.node: ">=22"` + CI `node-version: 22` |
| pnpm | **9.4.0** | `package.json` `packageManager: "pnpm@9.4.0"` + CI `pnpm/action-setup version: 9.4.0` |
| `@playwright/test` | **1.60.0** (installed) / spec `^1.60.0` | `node_modules/@playwright/test/package.json` + `package.json` devDeps |
| Playwright Docker image | **`mcr.microsoft.com/playwright:v1.60.0-jammy`** | matches the installed `@playwright/test` exactly — the iron rule (renderer parity) |
| Migrate command | **`pnpm migrate:up`** (`node-pg-migrate`, env `DATABASE_URL_MIGRATIONS`) | `package.json` scripts (`scripts/migrate-runner.ts` is the bundled prod twin) |
| API boot | **`pnpm dev:api:1`** → `PORT=3000` | `package.json` `dev:api:1` (the API serves the SPA on :3000; matches `playwright.visual.config.ts` default `baseURL`) |
| Config path | **`playwright.visual.config.ts`** | this branch |
| Baseline dir | **`e2e/visual/__screenshots__/`** | `snapshotPathTemplate` in the config |

> **Renderer-parity is the whole point.** Bump `@playwright/test` and the
> `mcr.microsoft.com/playwright:vX-jammy` tag **together**, in the same PR, then
> regenerate baselines (runbook §3). A version skew between the image and the
> installed package silently shifts AA/font rendering and floods every snapshot
> with false diffs.

---

## 1 · The workflow — copy into `.github/workflows/visual.yml`

```yaml
name: Visual Regression Net

# Run only when something that can change rendering changes. Critical-path
# screens live in apps/web; shared atoms/theme in packages/ui; the suite itself
# in e2e/visual.
on:
  pull_request:
    paths:
      - 'apps/web/**'
      - 'packages/ui/**'
      - 'e2e/visual/**'
      - 'playwright.visual.config.ts'
      - '.github/workflows/visual.yml'

# A newer push to the same PR cancels the older in-flight run.
concurrency:
  group: visual-${{ github.ref }}
  cancel-in-progress: true

jobs:
  visual:
    runs-on: ubuntu-latest
    timeout-minutes: 25

    services:
      # PG 16 — matches prod / the existing fresh-provision job. Bare Postgres,
      # no Supabase roles; we create the migrator + app roles below, same as CI.
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U postgres"
          --health-interval 5s --health-timeout 5s --health-retries 10
      # The API boots ioredis + pg-boss; without Redis it crashes on boot.
      redis:
        image: redis:7
        ports:
          - 6379:6379
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 5s --health-timeout 5s --health-retries 10

    env:
      # Test-only material — NEVER prod secrets.
      DEV_AUTH_SECRET: visual-ci-secret
      # The visual config + harness read these; :3000 is where the API serves the SPA.
      VISUAL_BASE_URL: http://localhost:3000
      # sslmode=disable: bare Postgres is plain TCP; the shared pool otherwise
      # forces ssl and dies with "server does not support SSL connections".
      DATABASE_URL_MIGRATIONS: postgresql://dowiz_migrator:migrator_pw@127.0.0.1:5432/dowiz_visual?sslmode=disable
      DATABASE_URL: postgresql://dowiz_app:app_pw@127.0.0.1:5432/dowiz_visual?sslmode=disable
      REDIS_URL: redis://127.0.0.1:6379

    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v3
        with:
          version: 9.4.0

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: 'pnpm'

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Build workspace packages (API server + SPA bundle)
        run: pnpm -r build

      - name: Install postgres client
        run: sudo apt-get update && sudo apt-get install -y postgresql-client

      # Bare PG has no Supabase roles. Create the migrator (DDL) + app (runtime)
      # roles and a clean database, and pre-create the pgboss schema owned by the
      # app role (pg-boss DDLs its partition tables there at runtime). Mirrors
      # scripts/verify-fresh-provision.sh end-state.
      - name: Provision roles + database
        env:
          PGPASSWORD: postgres
        run: |
          psql "postgresql://postgres@127.0.0.1:5432/postgres" -v ON_ERROR_STOP=1 \
            -c "CREATE ROLE dowiz_migrator LOGIN SUPERUSER PASSWORD 'migrator_pw';" \
            -c "CREATE ROLE dowiz_app LOGIN BYPASSRLS PASSWORD 'app_pw';" \
            -c "CREATE DATABASE dowiz_visual OWNER dowiz_migrator;"
          psql "postgresql://postgres@127.0.0.1:5432/dowiz_visual" -v ON_ERROR_STOP=1 \
            -c "GRANT ALL ON SCHEMA public TO dowiz_app;" \
            -c "ALTER DEFAULT PRIVILEGES FOR ROLE dowiz_migrator IN SCHEMA public GRANT ALL ON TABLES TO dowiz_app;" \
            -c "ALTER DEFAULT PRIVILEGES FOR ROLE dowiz_migrator IN SCHEMA public GRANT ALL ON SEQUENCES TO dowiz_app;" \
            -c "ALTER DEFAULT PRIVILEGES FOR ROLE dowiz_migrator IN SCHEMA public GRANT EXECUTE ON FUNCTIONS TO dowiz_app;" \
            -c "CREATE SCHEMA IF NOT EXISTS pgboss AUTHORIZATION dowiz_app;"

      - name: Run migrations against the service Postgres
        run: pnpm migrate:up
        # pnpm migrate:up reads DATABASE_URL_MIGRATIONS from env (set above).
        # --envPath .env in the script is harmless: real env vars take precedence.

      # The API needs app secrets to boot (JWT RS256 keypair, VAPID, etc.).
      # Throwaway, test-only — generated fresh each run; never prod keys.
      - name: Write throwaway app secrets (.env)
        run: |
          cat > .env <<'EOF'
          NODE_ENV=development
          PORT=3000
          JWT_SIGNING_SECRET=12345678901234567890123456789012
          JWT_KID=1
          GOOGLE_CLIENT_ID=visual-ci.apps.googleusercontent.com
          GOOGLE_CLIENT_SECRET=visual-ci-secret
          VAPID_PUBLIC_KEY=BEu3SBqHCfb9gzfBqkmFeV1WXidH9CgN53_VMDp9mx_61PaFXgPz5gDHuGIXHihtv4IDLKO7aOrvxQpkxaYVP8Y
          VAPID_PRIVATE_KEY=_R-pnIHQ3TvGlHVOwPiWggeqQNQEdd5Ky_QpHQ2y-7E
          IP_HASH_SALT=visual-ci-salt
          COURIER_PII_ENCRYPTION_KEY=0NbuViiJbBjEEm5fcSfulKasFLiqBNPAbU7kBZ7+oFU=
          DEV_AUTH_SECRET=visual-ci-secret
          REDIS_URL=redis://127.0.0.1:6379
          DATABASE_URL=postgresql://dowiz_app:app_pw@127.0.0.1:5432/dowiz_visual?sslmode=disable
          DATABASE_URL_MIGRATIONS=postgresql://dowiz_migrator:migrator_pw@127.0.0.1:5432/dowiz_visual?sslmode=disable
          EOF
          openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out /tmp/jwt_priv.pem 2>/dev/null
          openssl rsa -in /tmp/jwt_priv.pem -pubout -out /tmp/jwt_pub.pem 2>/dev/null
          {
            printf 'JWT_PRIVATE_KEY="'; awk '{printf "%s\\n", $0}' /tmp/jwt_priv.pem; printf '"\n'
            printf 'JWT_PUBLIC_KEY="';  awk '{printf "%s\\n", $0}' /tmp/jwt_pub.pem;  printf '"\n'
          } >> .env

      # Boot the built API (serves the SPA on :3000) in the background, then wait
      # until it answers before handing off to the Docker-contained suite.
      - name: Boot API + SPA on :3000
        run: |
          pnpm dev:api:1 > /tmp/visual-api.log 2>&1 &
          echo "API_PID=$!" >> "$GITHUB_ENV"
          for i in $(seq 1 60); do
            if curl -sf http://localhost:3000/livez >/dev/null 2>&1; then
              echo "✅ API up after ${i}s"; exit 0
            fi
            sleep 1
          done
          echo "❌ API did not become healthy in 60s"; tail -50 /tmp/visual-api.log; exit 1

      # IRON RULE: the snapshot suite runs inside the PINNED Playwright image so
      # rendering (fonts/AA/GPU) is byte-identical to how baselines were locked.
      # --network host lets the container reach the API + globalSetup seed on
      # localhost:3000. No --update: this is compare mode.
      - name: Run visual suite (compare mode, pinned renderer)
        run: |
          docker run --rm --network host \
            -v "$PWD":/work -w /work \
            -e CI=1 \
            -e VISUAL_BASE_URL=http://localhost:3000 \
            -e DEV_AUTH_SECRET=visual-ci-secret \
            mcr.microsoft.com/playwright:v1.60.0-jammy \
            /bin/sh -lc "corepack enable && corepack prepare pnpm@9.4.0 --activate && \
              pnpm exec playwright test -c playwright.visual.config.ts --reporter=list"

      - name: Stop API
        if: always()
        run: '[ -n "${API_PID:-}" ] && kill "$API_PID" || true'

      # On failure, ship the HTML report (includes expected/actual/diff triplets)
      # and the raw diff PNGs so the reviewer can see exactly what moved.
      - name: Upload Playwright HTML report
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: visual-report
          path: e2e/artifacts/visual-report
          retention-days: 14

      - name: Upload diff images + results
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: visual-diffs
          path: e2e/artifacts/visual-results
          retention-days: 14

      - name: Upload API log
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: visual-api-log
          path: /tmp/visual-api.log
          retention-days: 14
```

---

## 2 · Runbook

### The iron rule (non-negotiable)

> **Baselines are ONLY ever generated inside the pinned Playwright Docker image
> (`mcr.microsoft.com/playwright:v1.60.0-jammy`) against a freshly-seeded DB.
> NEVER on a developer laptop, NEVER on a bare CI runner.**

A baseline captured outside the pinned image encodes your machine's fonts,
anti-aliasing, and GPU rasterisation — noise that does not exist on any other
machine. Committing it guarantees false diffs for everyone else. If you ever
see a diff you can't explain, your first suspicion is "was this baseline made
in the image?" — not "did the app change?".

The seed runs via `globalSetup` (`e2e/visual/global-setup.ts`) hitting
`POST /api/dev/seed-visual-state`, which is dev-gated: **`DEV_AUTH_SECRET` must be
set** in the environment (CI and locally) or every dev endpoint 404s and the
suite fails at setup. The config path is always `playwright.visual.config.ts`.

### §3 · Generate / lock baselines the first time

Do this once, locally, **inside the image** (so the bytes match CI), with the
app already booted on `:3000` against a freshly-seeded DB.

1. Bring up Postgres 16 + Redis 7 and the API on `:3000` exactly as the workflow
   does (you can run `scripts/verify-fresh-provision.sh`-style provisioning, or
   `docker-compose.dev.yml` for PG/Redis, then `pnpm migrate:up && pnpm dev:api:1`).
   Export `DEV_AUTH_SECRET` so the seed endpoint is reachable.

2. Generate candidates **in the pinned image** (mirror the CI invocation, just
   add `--update-snapshots`):

   ```bash
   docker run --rm --network host \
     -v "$PWD":/work -w /work \
     -e CI=1 \
     -e VISUAL_BASE_URL=http://localhost:3000 \
     -e DEV_AUTH_SECRET="$DEV_AUTH_SECRET" \
     mcr.microsoft.com/playwright:v1.60.0-jammy \
     /bin/sh -lc "corepack enable && corepack prepare pnpm@9.4.0 --activate && \
       pnpm exec playwright test -c playwright.visual.config.ts --update-snapshots --reporter=list"
   ```

3. **REVIEW every candidate (the spec's review-pass).** `--update-snapshots`
   writes whatever it renders — including bugs. Open each new PNG under
   `e2e/visual/__screenshots__/` and confirm it is the *intended* design: no
   layout breakage, no untranslated strings, no unmasked dynamic content
   leaking in, correct locale/viewport. A baseline is a promise; never commit a
   screenshot you have not eyeballed.

4. Commit only the reviewed baselines:

   ```bash
   git add e2e/visual/__screenshots__/
   git commit -m "test(visual): lock critical-path baselines (rendered in playwright:v1.60.0-jammy)"
   ```

### §4 · Update a baseline intentionally (a design changed)

When a design change is *intended*, the diff is expected — updating the baseline
is a **conscious approval**, not a reflex.

1. Land the design/UI change.
2. Regenerate baselines **in the pinned image** with `--update-snapshots`
   (the §3 command). Optionally scope to the affected suite, e.g. append
   `e2e/visual/storefront.spec.ts` or `-g "<title>"`.
3. **Review the regenerated PNGs the same way as §3** — confirm each diff is the
   change you meant, and nothing else moved.
4. Commit the new baselines *in the same PR as the UI change*, with a message
   that states the conscious approval, e.g.
   `test(visual): rebaseline storefront hero (approved: new CTA spacing)`.
   The PR diff then shows old→new screenshots side by side for the reviewer.

> Never "rebaseline to make CI green." If a diff is unexpected, it is a
> regression to fix, not a baseline to overwrite. Updating a baseline is an
> explicit design decision recorded in the commit.

### §5 · When you bump Playwright

`@playwright/test` and the `mcr.microsoft.com/playwright:vX-jammy` tag move
together (see Versions). In one PR: bump the package, bump the image tag in
`.github/workflows/visual.yml` **and** in this doc's commands, then run §3 to
regenerate **all** baselines in the new image. Renderer changes between
Playwright versions legitimately shift pixels; the rebaseline is expected and
must be reviewed.
```
