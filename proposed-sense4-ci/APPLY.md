# Sense 4 — CI wiring (manual apply: `.github/workflows/ci.yml` is a protected zone)

`.github/workflows/ci.yml` is under the protect-paths governance zone, so the
hook blocks an automated edit. Apply the two changes below by hand (or in a
reviewed commit). They are the only CI changes Sense 4 needs.

## 1. Extend the `on:` triggers (add `workflow_dispatch` + a daily `schedule`)

LHCI audits **deployed staging** and needs a live URL — it cannot run against a
fresh PR build dir. So it runs on a schedule / manual dispatch, NOT on every PR.

```yaml
on:
  push:
    branches: [ "main" ]
  pull_request:
    branches: [ "main" ]
  workflow_dispatch:
  schedule:
    # Daily Lighthouse audit of deployed staging (LHCI needs a live URL).
    - cron: '0 6 * * *'
```

## 2. Add the two PR-blocking budget gates to the existing `validate` job

Insert immediately AFTER the existing `- name: Build` step (they consume its
`apps/web/dist` output, so no extra build is needed):

```yaml
      - name: Bundle size budgets (size-limit)
        run: pnpm size:check

      - name: Storefront eager-graph guard (admin-absent)
        run: pnpm storefront:guard
```

## 3. Add a separate (non-blocking) `lighthouse` job

Add as a top-level job (sibling of `validate` / `deploy`). It only runs on the
schedule or a manual dispatch, so it never gates PRs:

```yaml
  lighthouse:
    # LHCI audits the DEPLOYED staging storefront (needs a live URL). Runs on a
    # schedule + manual dispatch only — the PR-blocking budget gates are
    # size:check + storefront:guard inside `validate`.
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
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
      - name: Lighthouse CI (staging storefront)
        run: pnpm lhci
```

## Notes / preconditions
- Requires `pnpm install` first (adds `size-limit`, `@size-limit/file`, `@lhci/cli`).
- `size:check` + `storefront:guard` require `apps/web/dist` — `validate`'s
  `pnpm -r build` step produces it; they run after Build.
- Target URLs live in `lighthouserc.cjs` (staging storefront + checkout). The
  config is `.cjs` (not `.js`) because the root `package.json` is
  `"type": "module"` — a `module.exports` in a `.js` file there loads as an empty
  ESM config. LHCI's autorun config search includes `lighthouserc.cjs`.
