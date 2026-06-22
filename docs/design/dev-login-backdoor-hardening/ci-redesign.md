# CI redesign (ADR-0003 R2-1 / R3-1) — proposed `ci.yml` changes

`ci.yml` is a protected path (governance zone), so this is the ready-to-apply spec for
manual approval. Goal: the **prod** deploy job must not depend on a live owner-minting
backdoor. Authenticated E2E moves to a **staging gate that runs before prod deploy**;
prod gets an **unauthenticated** smoke.

The `fresh-provision` env-file change (flag + dev keypair) already landed in this commit
(it's not a protected path). Only the `deploy` job + a new `staging-e2e` job remain.

## A. NEW job: `staging-e2e` (authenticated suites, against dowiz-staging)
Insert after `fresh-provision`:

```yaml
  staging-e2e:
    needs: validate
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v3
        with: { version: 9.4.0 }
      - uses: actions/setup-node@v4
        with: { node-version: 22, cache: 'pnpm' }
      - run: pnpm install --frozen-lockfile

      - name: Deploy to staging (release_command runs migrations)
        uses: superfly/flyctl-actions/setup-flyctl@master
      - run: flyctl deploy -a dowiz-staging --remote-only
        env:
          FLY_API_TOKEN: ${{ secrets.FLY_API_TOKEN_STAGING }}

      - run: npx playwright install chromium

      - name: Authenticated E2E against staging
        run: |
          npx playwright test \
            e2e/tests/deploy-validation.spec.ts \
            e2e/tests/flow-core-lifecycles.spec.ts \
            e2e/tests/telegram-webhook.spec.ts \
            e2e/tests/telegram-full-flow.spec.ts \
            --project=desktop --reporter=list
        env:
          VITE_BASE_URL: "https://dowiz-staging.fly.dev"
          DEV_AUTH_SECRET: ${{ secrets.DEV_AUTH_SECRET_STAGING }}
        timeout-minutes: 15
```

## B. CHANGE `deploy` job — gate on staging-e2e, drop the backdoor, unauth smoke only
- `needs: validate` → `needs: [validate, staging-e2e]`
- Delete the four `Post-deploy E2E …` steps (deploy-validation, flow-core-lifecycles,
  telegram-webhook, telegram-full-flow) — and with them every `DEV_AUTH_SECRET` env on
  the prod job.
- Replace with a single unauthenticated smoke:

```yaml
      - name: Post-deploy prod smoke (unauthenticated)
        run: npx playwright test e2e/tests/prod-smoke.spec.ts --project=desktop --reporter=list
        env:
          VITE_BASE_URL: "https://dowiz.fly.dev"
          PROD_SMOKE_SLUG: "demo"   # a seeded PUBLIC location slug
        timeout-minutes: 3
```

## C. Required GitHub repo secrets (operator)
- `FLY_API_TOKEN_STAGING` — deploy token scoped to `dowiz-staging`.
- `DEV_AUTH_SECRET_STAGING` — `stg-e2e-secret`.
- The staging Fly app already carries `ALLOW_DEV_LOGIN`, `JWT_DEV_KID`, the dev keypair,
  and `DEV_LOGIN_EMAIL/PASSWORD` (set this session) — no GitHub secret needed for those;
  they live on the app.

## Net effect
- Prod deploy validated by health + public storefront read + 401 negatives — **no token
  minted on prod**, so unsetting prod `DEV_AUTH_SECRET` no longer reds the pipeline.
- Authenticated coverage preserved on the same image via the staging gate, run **before**
  prod ships (prod proceeds only if staging is green).
