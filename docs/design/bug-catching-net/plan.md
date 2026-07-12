# Bug-Catching Net — a layered proactive system that catches bugs before clients do

**Status:** DESIGN. No code applied. **Date:** 2026-07-02.
**Goal:** every class of production defect is caught by an automated net *before* a paying customer
hits it. Today the last line of defense is a customer noticing a broken storefront — this replaces
that with five overlapping nets, each catching what the layer above misses.

Grounded in the current tree (read, not assumed):

- `fly.toml:27-33` — Fly's only health check hits **`/livez`** (event-loop only; deliberately does
  **not** touch Postgres) every 15s. `apps/api/src/routes/health.ts` — the rich **`/health`** returns
  **503** when Postgres is down. **Nothing external reads `/health`**, so a pg-down 503 (storefront
  500s) pages *no one*. This is the open **P0** in `docs/design-review/PROJECT-HARDENING-BACKLOG-2026-07-02.md`.
- `apps/api/src/lib/metrics.ts` — a token-gated Prometheus `/metrics` endpoint exists (http rate/latency
  histograms, WS out, pool/queue gauges, RSS/heap). It is **dark by default** (404 unless `METRICS_TOKEN`)
  and **no scraper is wired**. Nobody reads it.
- `apps/api/src/lib/sentry.ts` — Sentry is initialized (PII-redacted `beforeSend`, `tracesSampleRate 0.1`,
  release = git SHA). Errors flow in; there are **no alert rules, no error-budget / burn-rate** on top.
- `.github/workflows/ci.yml` — `validate` (build/typecheck/lint/`verify:all --ci`/migrations/secrets/
  compliance) + `fresh-provision` (stands up PG+Redis+roles, runs `verify:fresh-provision`) + `deploy`
  (main only). The `rls-adversarial` / jwt-rotation / integrity guardrails **never run** (proposal
  `docs/proposals/ci-security-wiring.md` designed, unapplied). `deploy` **`needs: validate` only** — not
  `fresh-provision`. Post-deploy E2E (`deploy-validation`, `flow-core-lifecycles`, telegram) runs **after**
  Fly is already live against `https://dowiz.fly.dev` → **detect, not prevent, and no rollback**.
- `.github/workflows/visual.yml` — the Visual Regression Net (pinned Playwright image, compare mode,
  locked baselines) + `e2e/tests/non-pixel-sweep.spec.ts` (axe + console, 3 roles) + `e2e/helpers/a11y.ts`
  + the console-guard fixture. Runs **on PR only** (paths-filtered), never continuously against prod.

**Tagging (matches the hardening backlog):** **SAFE** = in-repo, outside protected paths, ship-now.
**PROTECT-PATH** = `.github/**`, `Dockerfile`, root `package.json`, `fly.toml` → operator pastes.
**OPERATOR** = needs a secret / external account / prod config only a human holds.
Effort: **S** ≤ half-day · **M** 1-2 days · **L** ≥ 3 days.

---

## The net, layer by layer

```
                          ┌───────────────────────────────────────────────┐
  bug originates …        │  caught by …                                    │
  ─────────────────────── │ ─────────────────────────────────────────────  │
  infra / DB outage       │ L1  external synthetic uptime + journey probe   │  ← the open P0
  regression in a PR      │ L2  CI prevent-gate (rls-adversarial + E2E on   │
                          │     fresh DB) BEFORE deploy; canary + rollback  │
  visual / a11y / console │ L3  continuous visual+a11y+console on crit path │
  slow error-rate creep   │ L4  error-budget / burn-rate off Sentry+/metrics│
  broken user journey     │ L5  synthetic journey bots (staging + prod)     │
                          └───────────────────────────────────────────────┘
```

Each layer is independent and free to build. They compose: L1 catches "is it up", L5 catches "does the
journey work", L2 stops the regression reaching prod at all, L3 catches the silent visual/a11y break,
L4 catches the slow bleed no single probe sees.

---

## L1 — External synthetic uptime + critical-journey monitoring (closes the open P0)

**What exists.** `/livez` (Fly, 15s) and `/health` (11-check, 503-on-pg-down) endpoints — both *server
resident*. `/health` even carries a minimal, unauthenticated, recon-safe payload built exactly for a
public prober (`health.ts:320-329`). A Telegram bot + dispatcher already exists (project memory;
notification workers) — **a paging rail is already built.**

**Gap.** No probe *outside* Fly ever calls `/health`. Fly checks `/livez` (which stays green while
Postgres is down, by design), so the one signal that means "customers are seeing 500s" (`/health` 503)
reaches nobody. No external vantage point → a full-region Fly/DNS/TLS outage is invisible until a client
complains. No status page.

**Concrete build.**
1. **External HTTP prober on `https://dowiz.fly.dev/health`** from outside Fly, alerting on
   `status != 200` OR body `status != "healthy"|"degraded"` OR latency > 3s. Two free options:
   - **Uptime Kuma** (MIT, `github.com/louislam/uptime-kuma`) — self-host one container; HTTP + keyword
     monitor, built-in **Telegram** notifier (reuse the existing bot), public status page included.
   - **cron-job.org** (free tier) or **Gatus** (Apache-2.0, declarative YAML, container) — zero-VM
     alternative; Gatus config is a committed artifact.
   Recommendation: **Uptime Kuma** — status page + Telegram in one, and it can also probe the
   storefront `GET /s/demo` and `GET /public/locations/demo/menu` (menu-pool-starvation class, project
   memory) as keyword monitors, not just `/health`.
2. **Critical-journey probe (scripted, external).** A tiny scheduled job that runs 3-4 real requests end
   to end against prod: real owner login (`POST /api/auth/local/login`, the deploy-validation path) →
   `GET /public/locations/:slug/menu` returns products → SSR `GET /s/:slug` carries `X-Menu-Version`.
   Ship it as a **SAFE** `scripts/synthetic-probe.mjs` (Node fetch, zero deps) that exits non-zero +
   posts to the existing Telegram bot on failure; the **schedule** is a `.github/workflows/synthetic.yml`
   on `schedule: cron` (min 5-min interval) — **PROTECT-PATH**, operator pastes. GitHub Actions cron is
   free.

**SAFE vs gated.** The prober script + Gatus/Kuma config files + the journey `scripts/synthetic-probe.mjs`
are **SAFE** (in-repo, no protected path). The **schedule** (`.github/workflows/synthetic.yml`), the
Uptime-Kuma host, and the Telegram monitor chat-id / bot token are **OPERATOR** (external host + secret;
note token-rotation debt D7). **Effort:** S (Kuma pointed at `/health` + Telegram = ~1h) → M (add the
scripted journey probe + workflow).

---

## L2 — Shift CI from detect → prevent (block the regression before it deploys)

**What exists.** `validate` gates every PR on build/typecheck/lint/`verify:all --ci`/migrations/secrets/
compliance. `fresh-provision` already stands up the exact migrated Postgres + `dowiz_migrator`/`dowiz_app`
roles the adversarial suite needs, then **discards it** without running the suite. Post-deploy E2E
(`deploy-validation.spec.ts`, `flow-core-lifecycles.spec.ts`) runs — but *after* Fly is live.

**Gap (three real ones).**
1. **The cross-tenant IDOR guardrail (`rls-adversarial`) has never run in CI** — backlog **B1**, the
   single highest-value free fix on the board. Same for jwt-rotation + integrity + `verify:rls` (runtime
   DEFINER pin). The paste-in is already written (`docs/proposals/ci-security-wiring.md`).
2. **`deploy` does not depend on `fresh-provision`** (`ci.yml:134` `needs: validate`) — a fresh DB that
   won't boot, or an IDOR leak, does not block the rollout.
3. **Post-deploy E2E is detect-not-prevent with no rollback** — a failed `flow-core-lifecycles` run
   leaves the broken build live and serving customers. It also targets `https://dowiz.fly.dev` (prod),
   and `deploy-validation.spec.ts` *mutates* + self-guards with `requireStaging` — so that mutating suite
   cannot legitimately run against prod at all (verify: it either aborts or is mis-pointed).

**Concrete build.**
1. **Apply `ci-security-wiring.md` (a)+(c):** run `test:phase5-rls-adversarial`, `test:phase5-jwt-rotation`,
   `test:phase5-integrity`, `verify:rls` in `fresh-provision` against its already-provisioned DB.
   red→green provable (fail on an IDOR-leaking/unpinned DB). **PROTECT-PATH**, operator pastes; net-new
   free.
2. **Gate deploy on the full pre-flight:** `deploy.needs: [validate, fresh-provision]`, and run the
   **non-mutating** core-journey E2E (`flow-core-lifecycles`) against a **fresh-provisioned or staging**
   target *before* `flyctl deploy`, not after. **PROTECT-PATH**.
3. **Canary + rollback.** Fly supports rolling/canary strategies and instant rollback to the prior
   release. After `flyctl deploy`, run the post-deploy smoke; on failure run `flyctl releases rollback`
   (or deploy the previous image) so a bad build auto-reverts instead of serving customers. Free
   (`flyctl` is already in the deploy job). **PROTECT-PATH** (`ci.yml` + a `fly.toml` `[deploy] strategy`).
   *(Note `auto_stop_machines = false` + release-command migrations already give fail-safe rollout;
   this adds fail-safe on the app-behavior check, not just boot.)*
4. **Free tooling:** no new deps — Playwright, `flyctl`, GitHub Actions are all present. Optional
   `superfly/flyctl-actions` is already used.

**SAFE vs gated.** Authoring/curating the E2E specs the gate runs (in `e2e/`) is **SAFE**. Every wiring
change lives in `.github/workflows/ci.yml` + `fly.toml` → **PROTECT-PATH / operator**. **Effort:** S for
(1) (paste-in exists) → M for (2)+(3) (rollback logic + re-point E2E to a pre-deploy target).

---

## L3 — Continuous visual + a11y + console-error regression on the critical path

**What exists.** A mature net already: `visual.yml` (pinned renderer, locked baselines, compare mode,
3 breakpoints × locales, `[data-dynamic]` masks), `e2e/tests/non-pixel-sweep.spec.ts` (axe + console,
client/owner/courier), `e2e/helpers/a11y.ts` (`expectNoA11y`), the console-guard auto-fixture
(hard-fail on error/warn/pageerror/hydration + `stripCrossOriginAuth`), `.size-limit.json`,
`scripts/storefront-graph-guard.mjs`, WebKit smoke projects.

**Gap.** All of it runs **pre-merge on PRs only** (path-filtered) against a CI-built app. Nothing runs
the a11y/console/visual senses **against the deployed prod/staging build on a schedule** — so a defect
introduced by a data change, a CDN/font regression, an env-only config, or drift between the CI build and
the deployed image is invisible until the next PR touching those paths. Baselines also only lock in CI
(Docker), so there's an operator to-do (project memory) to commit `e2e/visual/__screenshots__/`.

**Concrete build.**
1. **A scheduled "prod senses" job**: run `non-pixel-sweep.spec.ts` (axe + console-guard, 3 roles) +
   a thin visual subset of the critical path against `VITE_BASE_URL=https://dowiz.fly.dev` on a daily
   `schedule: cron`, paging Telegram on any axe violation / console error / hydration warning. Reuses
   the existing specs verbatim — no new test authoring. Visual-diff against prod is noisier (real data),
   so run **console+a11y continuously** and keep **pixel-diff as the PR gate** (deterministic seed).
2. **Free tooling:** Playwright + `@axe-core/playwright` already in the tree; GitHub Actions cron is free.
3. **Finish the visual baseline lock** (operator Docker to-do) so the PR pixel gate is actually armed.

**SAFE vs gated.** Extending/curating the specs is **SAFE**. The scheduled workflow (`.github/**`) and
the baseline commit (Docker run) are **PROTECT-PATH / OPERATOR**. **Effort:** S (schedule the existing
sweep) → M (curate a stable prod-safe visual subset + baseline lock).

---

## L4 — Error-budget / anomaly alerting off Sentry + /metrics

**What exists.** Sentry ingesting PII-redacted errors + 10% traces (`sentry.ts`). A rich, token-gated
`/metrics` (http rate/latency histograms per route, `byStatus`, WS out, pool/queue-depth gauges,
RSS/heap) — purpose-built for "is the pool saturating *before* it wedges" (the menu-blink incident class,
`metrics.ts:6-9`).

**Gap.** `/metrics` is **dark** (404 without `METRICS_TOKEN`) and **unscraped** — the data exists and
nobody looks. Sentry has **no alert rules and no error-budget / burn-rate** — errors accumulate silently;
there is no "5xx rate over budget → page" and no "burn rate 14×/1h → page" (Google SRE multi-window).

**Concrete build.**
1. **Turn on the scrape.** Set `METRICS_TOKEN` (Fly secret), point a scraper at `/metrics`:
   **Grafana Cloud free tier** (hosted, zero-VM) or self-hosted **Prometheus** (Apache-2.0) + **Grafana**
   (AGPL-3.0) + **Alertmanager** (Apache-2.0) → route pages to the existing Telegram bot.
2. **Burn-rate SLO alerts** off the http histograms: define an availability SLO (e.g. 99.5% of
   `http_requests_total` non-5xx on the storefront + checkout routes) and a latency SLO (p95 from the
   duration histogram). Generate multi-window burn-rate rules with **Sloth** (Apache-2.0) or **Pyrra**
   (Apache-2.0) so a fast burn pages immediately and a slow burn tickets. Feed pool-saturation +
   queue-depth gauges as leading-indicator alerts (catch the wedge *before* the 503).
3. **Sentry alert rules** (free, no infra): issue-frequency spike alerts + a metric alert on error rate,
   routed to Telegram. This is the fastest sub-layer — pure Sentry dashboard config.
4. **Free tooling:** Grafana/Prometheus/Alertmanager/Sloth/Pyrra all OSS; Grafana Cloud + Sentry both
   have usable free tiers.

**SAFE vs gated.** Prometheus scrape config, Grafana dashboards-as-code, and Sloth/Pyrra SLO rule YAML
are committable **SAFE** artifacts. `METRICS_TOKEN` + the scraper host + Sentry alert config + the
Telegram route are **OPERATOR** (secret + external). **Effort:** S (Sentry alert rules alone) → M
(scrape + burn-rate dashboards).

---

## L5 — Synthetic customer-journey bots (staging + prod)

**What exists.** The richest journey coverage in the repo already: `deploy-validation.spec.ts`
(login → menu CRUD → public menu contract → SSR headers → SPA fallback → browser render, 14 groups) and
`flow-core-lifecycles.spec.ts`, plus dozens of `flow-*` specs and the `reliability-gate` skill (traces one
order L0-L11). These run **once per deploy** (L2), not continuously.

**Gap.** No bot re-runs the real customer journey on a **schedule** against staging *and* prod, so a
regression that appears *between* deploys (data drift, expiring cert, upstream LLM/Telegram/R2 outage,
free-tier quota hit) is caught only at the next deploy — possibly days later, after customers hit it.
Staging has no continuous journey exercise at all between manual deploys.

**Concrete build.**
1. **Scheduled journey bot** (`schedule: cron`, e.g. every 15-30 min) that runs a **read-only / self-
   cleaning** journey subset against prod and a **fuller mutating** subset against staging:
   - **prod (read-only):** login → menu loads → SSR renders → theme resolves → `/health` healthy.
     Reuse the non-mutating assertions from `deploy-validation.spec.ts`; never write to the live
     storefront (respect its `requireStaging` guard — mutation stays on staging).
   - **staging (full):** the whole `deploy-validation` + a checkout/order-lifecycle trace
     (`flow-order-lifecycle-trace.spec.ts`), which self-cleans in `afterAll`.
   Page Telegram + open a Sentry event on failure so it lands in the same triage stream as real errors.
2. **Free tooling:** Playwright + GitHub Actions cron (free). For >5-min frequency or multi-region
   vantage, **OneUptime** (open source, runs real Playwright journeys) or **OpenStatus** (AGPL-3.0,
   Cloudflare-Workers synthetic) self-hosted — but GH Actions covers the MVP for free.

**SAFE vs gated.** The journey specs + a prod-safe read-only tag/project in Playwright config are
**SAFE**. The schedule (`.github/**`) + secrets (`DEV_AUTH_SECRET`, Telegram) + any self-hosted OneUptime
host are **OPERATOR**. **Effort:** M (author the prod-safe read-only journey + the two schedules).

---

## Summary matrix

| Layer | Exists | Core gap | Free tool (license) | SAFE now | Operator / protect-path | Effort |
|---|---|---|---|---|---|---|
| L1 external uptime + journey | `/health` 503, Telegram rail | nobody probes `/health` from outside → P0 | Uptime Kuma (MIT) / Gatus (Apache-2.0) / GH cron | prober script + config | Kuma host, Telegram secret, `synthetic.yml` | **S→M** |
| L2 CI prevent + rollback | validate + fresh-provision + post-deploy E2E | rls-adversarial never runs; deploy not gated on it; no rollback | Playwright + flyctl + GH Actions (present) | E2E specs | `ci.yml`, `fly.toml`, `ci-security-wiring.md` paste | **S→M** |
| L3 continuous visual/a11y/console | visual.yml + non-pixel-sweep + console-guard | PR-only, never vs deployed prod on schedule | Playwright + axe-core (present) | curate specs | `.github/**`, baseline Docker lock | **S→M** |
| L4 error-budget / anomaly | Sentry + dark `/metrics` | `/metrics` unscraped; no alert rules / burn-rate | Prometheus + Grafana + Alertmanager + Sloth/Pyrra (Apache/AGPL); Sentry alerts | scrape/dashboard/SLO YAML | `METRICS_TOKEN`, scraper host, Sentry rules | **S→M** |
| L5 journey bots (staging+prod) | deploy-validation + flow-* specs | run once/deploy, not on a schedule | Playwright + GH cron; OneUptime/OpenStatus (OSS) | prod-safe read-only specs | schedules + secrets | **M** |

---

## Smallest, highest-leverage first increment

**Close the open P0 in L1: an external prober on `https://dowiz.fly.dev/health` that pages the existing
Telegram bot on any non-`healthy` result.** One reason it wins: today a Postgres-down 503 — the exact
signal that customers are seeing 500s — reaches *no one*, because Fly only watches `/livez` (green by
design during a DB outage). The endpoint is already built and already returns a recon-safe public
payload; the paging rail (Telegram) is already built. This is ~1 hour of **external configuration with
zero code** and it converts the single loudest blind spot into an alert.

Concretely, in order of ascending cost:

1. **SAFE / operator, ~1h:** stand up **Uptime Kuma** (or a cron-job.org monitor), HTTP monitor on
   `/health` with keyword `"healthy"`/`"degraded"`, Telegram notifier. Add a second monitor on
   `GET /public/locations/demo/menu` (menu-pool-starvation class). Nothing in the repo changes.
2. **SAFE, same day:** commit `scripts/synthetic-probe.mjs` (zero-dep Node fetch: prod login → menu →
   SSR `X-Menu-Version`, exit non-zero + Telegram on failure) so the journey check is versioned and
   reusable by L5.
3. **PROTECT-PATH, operator:** paste `docs/proposals/ci-security-wiring.md` (a)+(c) — wire
   `rls-adversarial` into `fresh-provision`. It's free, the paste-in already exists, and it retires the
   biggest *pre-deploy* blind spot (B1). Pair L1 (catch what escaped) with this (stop it escaping).

Everything else layers on top without rework: the `synthetic-probe.mjs` from step 2 becomes the L5 bot;
the L2 rollback and L4 burn-rate alerts reuse the same Telegram route this increment establishes.

---

### One-paragraph recap for the caller

The net is **five layers**: **L1** external synthetic uptime + journey probing of prod (the open P0 —
nothing outside Fly reads `/health`, so a pg-down 503 pages no one); **L2** shifting CI from detect→prevent
(run the never-run `rls-adversarial` + core E2E on the already-provisioned fresh DB *before* deploy, gate
`deploy` on it, add canary + `flyctl` rollback); **L3** running the existing visual/a11y/console senses
continuously against deployed prod, not just on PRs; **L4** lighting up the dark `/metrics` (Prometheus +
Grafana + Sloth/Pyrra burn-rate) and adding Sentry alert rules; **L5** scheduled Playwright journey bots
on staging (mutating) + prod (read-only). **First increment:** an external prober on `/health` paging the
existing Telegram bot — ~1h, zero code, closes the loudest blind spot. **SAFE-now** (I can build in-repo):
the prober/journey scripts (`scripts/synthetic-probe.mjs`), Gatus/Prometheus/Sloth config artifacts, and
curating the E2E/a11y specs the gates run — all outside protected paths. **Operator-gated:** everything in
`.github/workflows/**`, `fly.toml`, root `package.json`, plus secrets/hosts (`METRICS_TOKEN`, the
Uptime-Kuma/Grafana hosts, and the Telegram monitor chat — note the pending token rotation, D7). Free
tools named with licenses: Uptime Kuma (MIT), Gatus (Apache-2.0), OpenStatus (AGPL-3.0), OneUptime (OSS),
Prometheus/Alertmanager/Sloth/Pyrra (Apache-2.0), Grafana (AGPL-3.0), plus Playwright + flyctl + GitHub
Actions cron already in the repo.

**Sources (external, verified 2026-07):**
[Uptime Kuma alternatives — Better Stack](https://betterstack.com/community/comparisons/uptime-kuma-alternative/) ·
[Open-source monitoring options — DevHelm](https://devhelm.io/blog/best-open-source-monitoring-tools) ·
[Synthetic monitoring with Playwright — qaskills.sh](https://qaskills.sh/blog/synthetic-monitoring-playwright-guide) ·
[Alerting on SLOs — Google SRE Workbook](https://sre.google/workbook/alerting-on-slos/)
</content>
</invoke>
