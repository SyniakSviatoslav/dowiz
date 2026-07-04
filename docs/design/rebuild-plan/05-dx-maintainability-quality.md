# Lane 5 — DX / Maintainability / Testing / Overall Quality

> Rebuild-plan research, 2026-07-04. Lane 5 of the max-lanes effort. All codebase claims verified
> against the working tree at `fix/audit-remediation`; all timings measured on this sandbox
> (Node v22.22.3, pnpm 9.4.0). Web claims cite 2025–2026 sources. Culture constraint: YAGNI /
> ponytail — every recommendation is the *minimum* that buys the gain; no ceremony for its own sake.

---

## 0. Measured baseline (evidence first)

| Metric | Value | Source |
|---|---|---|
| Workspace | pnpm 9.4, 10 buildable packages + tools/spikes globs, **no task runner** (no `turbo.json`/`nx.json`) | `pnpm-workspace.yaml`, root `ls` |
| TS project references | **None.** No `composite`, no `references` in any tsconfig — the "project refs" assumption is FALSE; each package runs an independent `tsc -p` | `tsconfig.base.json`, all 10 `*/tsconfig.json` |
| Full typecheck (`pnpm -r typecheck`, 9 pkgs, warm) | **18.6 s wall** (35.8 s CPU) | measured |
| Full build (`pnpm -r build`) | **27.8 s wall** (50.5 s CPU) | measured |
| Unit suite (`pnpm test:unit`, node:test via tsx) | **1,198 tests / 53 suites / 26.1 s wall** (81 skipped) | measured |
| Whole-repo `pnpm lint` | **48.4 s wall** (CI-side only; pre-commit lints staged files) | measured |
| Docker build in pre-commit | ~4–5 min + disk-guard prune machinery; caused a real incident (disk fill → local PG WAL crash) | `.husky/pre-commit:39-56`, `docs/incidents/2026-06-28-local-pg-disk-crash.md` |
| Test-script sprawl | 26 hand-numbered `test-stage*.ts`/`test-phase*.ts` scripts; ~20 dedicated `test:phaseN`/`stageNN` entries; root `scripts` block is 70 entries | `package.json:46-67`, `ls apps/api/tests` |
| Unit tests in CI | **ZERO.** `ci.yml` runs build/typecheck/lint/verify-gates/fresh-provision but never `test:unit`; the wiring exists only as a staged operator proposal | `.github/workflows/ci.yml`, `proposed-ci-test-gates/APPLY.md` |
| Test files | 182 `*.test.ts(x)` unit files + 174 Playwright specs in `e2e/` | `find` counts |
| God files | `orders.ts` 980 (max CCN **156**), `server.ts` 890 (CCN 116), `MenuManagerPage.tsx` 1405, plus `MenuPage.tsx` **1811** (now the largest FE file) and `spa-proxy.ts` 885 (CCN **174**) | `wc -l`, repowise `get_health` |
| Repo health | avg 7.84/10; 42 files in "alert" band (16.4k NLOC); worst `courier/shifts.ts` = 1.0 | repowise `get_health` dashboard |
| Dead code | 369 findings; 98 at ≥0.5 confidence ≈ **2,370 deletable lines**; 18 high-confidence (incl. dead authz exports `requireRole`, `softVerifyAuth` in `apps/api/src/plugins/auth.ts`) | repowise `get_dead_code` |
| Harness friction (real log, ~2 days) | 1,040 hook events: **360 lesson injections + 294 nudges + 59 red-line advisories** vs **37 blocks + 1 deny** | `.claude/logs/harness-events.jsonl` |
| Harness footprint | `.claude/` = 1.4 GB **but 1.4 GB is `worktrees/`** (stale sandboxes); actual harness ≈ 20 MB: 9 hooks, 26 local ESLint rules (967-line plugin), 75 `scripts/`, 23-gate `verify:all`, `loops/` 180 files | `du`, `scripts/verify-all.ts` |

Interpretation: local one-shot times are *not* the problem (typecheck 19 s, build 28 s, tests 26 s
are all healthy for 115k LOC of first-party TS). The problems are **(a)** the pre-commit hook
re-pays *all* of it plus a 4–5-min Docker build on **every commit** regardless of change scope,
**(b)** 1,198 unit tests run **nowhere automatically**, **(c)** the stage-test sprawl and god files
tax navigation, and **(d)** the advisory harness injects ~700 advisory events per 2 days into agent
context for 38 actual enforcement decisions.

---

## 1. Monorepo task runner — plain `pnpm -r` → **Turborepo 2.x**

- **From:** `pnpm -r build|typecheck` — zero caching, zero change-scoping. A docs-only commit
  still pays 18.6 s + 27.8 s locally; CI pays a full `pnpm -r build` **twice** (validate +
  fresh-provision jobs) plus full typecheck, with only the pnpm store cached (`ci.yml:20,84,145`).
- **To:** Turborepo (v2.7, Dec 2025) wrapping the existing per-package scripts. `turbo.json`
  ≈ 20 lines (`build` dependsOn `^build` with `dist/**` outputs; `typecheck` dependsOn `^build`;
  `test`). Local cache immediately; free Vercel remote cache (free on all plans, open self-hostable
  spec) or plain `actions/cache` on `.turbo/` in CI; `--affected` (since 2.1) for PR-scoped work.
- **Why Turborepo, not Nx/moonrepo:** Nx is over-scale at 10 packages and carries real 2025–26
  trust damage (Aug 2025 "s1ngularity" npm supply-chain attack; Mar 2026 stolen-token AWS
  escalations; May 2026 compromised Nx Console extension; free self-hosted-cache plugins
  deprecated May 2026) — for a repo with a standing secrets incident and a security-gated
  open-source plan, importing that risk for features we don't need is indefensible. moonrepo is
  maintained but needs per-project `moon.yml` toolchain ceremony for no edge at this scale. pnpm
  itself still does install/linking only — pnpm 10 gains are install speed, not task caching.
  Sources: turborepo.dev/blog/turbo-2-7; turborepo.com/blog/turbo-2-1-0 (--affected);
  turborepo.dev/docs/core-concepts/remote-caching; cloudsmith.com/blog/nx-npm-supply-chain-attack;
  thehackernews.com/2026/03/unc6426-exploits-nx-npm-supply-chain.html.
- **Quantified gain:** cache-hit no-op `turbo build+typecheck` ≈ 1–2 s vs 46 s today (measured
  baseline); single-package edits re-run only that package + dependents (typ. 5–15 s). At the
  current agent-driven commit rate (dozens/day across lanes), that is tens of minutes/day of
  wall-clock, and it is the enabler for §4's fast pre-commit. Industry numbers (up to 70–90% CI
  time cut) are from much larger repos — at 10 packages expect the *floor* win: near-instant
  no-ops and correctly-scoped partial work, not miracles.
- **Effort:** S (half a day incl. CI cache wiring). **Risk:** low — it wraps existing scripts and
  is trivially removable; one gotcha: `tsc --noEmit` has no output files, so cache it as a
  logs-only task, and be strict about `inputs` so guardrail scripts don't poison hashes.
- **Verdict: DO.** Highest leverage-per-effort infrastructure change in this lane.

## 2. Test framework & the stage-test sprawl — consolidate on node:test; **do not** migrate to Vitest

- **From:** two parallel worlds: (a) 182 `*.test.ts` node:test files → healthy: 1,198 tests in
  26 s; (b) 26 hand-numbered `test-stage7..36.ts` / `test-phase*.ts` tsx scripts wired through
  ~20 copy-paste `package.json` entries — including a 20-command `&&` chain
  (`package.json:67`) — plus ~15 bespoke `verify:*` entries. The numbered scripts are frozen
  roadmap artifacts ("stage 23") that no longer map to anything a maintainer can name.
- **Vitest assessment (2026):** Vitest 4.1 (Mar 2026) is excellent — `projects` mode, stable
  browser mode, AST-remapped v8 coverage (vitest.dev/blog/vitest-4). But migration means
  rewriting `node:assert` → `expect()` across 182 files for a suite that already runs in 26 s.
  There is no speed problem to solve; that's churn, not improvement. node:test on Node 22/24 has
  globs, watch, sharding (`run({shard})`, Node 24 docs), and native type-stripping became stable
  by Node 24.11 (nodejs.org/api/typescript.html) — the zero-dep path keeps getting better on its
  own. **Skip Vitest** unless/until real browser-mode component testing is wanted (YAGNI today).
- **Do instead (the actual fixes):**
  1. **Wire `test:unit` into CI** — the staged `proposed-ci-test-gates/APPLY.md` §2 already
     proves it green against the fresh-provision job's DB (857 DB-backed tests, serial). This is
     the single biggest *testing* gap in the repo: the proof-discipline culture writes tests that
     nothing executes. Effort: operator-applies-a-patch. Do it first.
  2. **Consolidate the stage scripts:** move still-meaningful `test-stageNN.ts` bodies into named
     node:test files (`apps/api/tests/lifecycle/*.test.ts` etc.) so the `test:unit` glob owns
     them; delete ones covered by newer named tests (most are — the named suite postdates them).
     Replace ~20 `test:phase*`/`stage*` entries with one `test:integration` (env-gated). Root
     `scripts` block: ~70 → ~35 entries. Fold the 6 `guardrail:*` + 15 `verify:*` one-liners
     behind `verify:all` (already the chokepoint) where no one calls them individually.
  3. Later, on the Node 24 LTS upgrade: drop `tsx` from the test lane (`node --test` strips types
     natively) — one dependency less, no other change.
- **Effort:** M (1–2 days audit+move of 26 scripts). **Risk:** low; the scripts are additive
  checks — deleting a redundant one loses nothing provable (keep any that is the only coverage of
  a lifecycle path; verify by mapping each stage script to the named test that supersedes it).
- **Gain:** discoverability (one command, one glob), CI actually running 1,198 tests, root
  package.json readable again.

## 3. Typecheck speed — keep per-package `tsc --noEmit` under turbo; adopt **tsgo for the check lane** when 7.0 GA lands

- **Facts:** no project references exist (verified §0), so there is no `tsc -b` incremental graph
  to tune. Full check = 18.6 s wall, parallelized by pnpm across 9 packages.
- **Options weighed:**
  - *Introduce project references + composite:* M effort (10 tsconfigs, declaration hygiene,
    `tsc -b` orchestration) to shave a share of 18.6 s that Turborepo's package-level caching
    already shaves for free on unchanged packages. **Skip** — refs solve a problem turbo solves
    cheaper here.
  - *isolatedDeclarations:* buys parallel `.d.ts` emit (reported 3–15× emit speedups on bigger
    repos — jsmanifest.com/typescript-isolated-declarations-monorepo-performance) at the price of
    explicit return types on every export. Our emit isn't the bottleneck (28 s *total* build).
    **Skip** (and tsgo obsoletes most of its motivation).
  - ***tsgo / TypeScript 7:*** the native Go compiler hit **RC on 2026-06-18, folded into
    `typescript@rc`, GA targeted ~July 2026**, with >10× real-world checks (72.8 s → 6.8 s on
    Sentry's codebase) and `--noEmit` explicitly recommended as the safe on-ramp
    (devblogs.microsoft.com/typescript/announcing-typescript-7-0-rc/). Known gaps at RC: no
    stable API until 7.1, no `.d.ts.map`, some legacy flags dropped (none of which
    `tsconfig.base.json` uses — target ES2022, bundler resolution are supported).
- **To:** swap the `typecheck` script to the native compiler for the hot path once 7.0 GA ships
  (expected weeks away): 18.6 s → ~2–3 s expected. Keep tsc 5.6 for `build` emit until 7.x
  proves declaration-emit parity; keep tsc as the CI authority for one release cycle
  (belt-and-suspenders, then drop the double-check).
- **Effort:** S. **Risk:** low with the dual-lane rollout. **Verdict: DO at GA** (July 2026 —
  i.e., during this rebuild). Turbo caching (§1) covers the interim.

## 4. Pre-commit / CI split — evict the Docker build; pre-commit ≤ 30 s

- **From (`.husky/pre-commit`):** staged-file ESLint → 3 whole-tree guardrail scripts → i18n
  parity → **full `pnpm -r typecheck`** → **full `pnpm -r build`** → flyctl config validate →
  **local Docker build ~4–5 min** wrapped in disk-guard prune logic that exists only because this
  hook once filled the disk and crashed the local Postgres into WAL recovery (comment in the hook,
  `docs/incidents/2026-06-28-local-pg-disk-crash.md`). Every commit pays ~5–6 min; the Docker
  step is ~80–90% of it and validates a failure class (Dockerfile/lockfile/build-context drift)
  that changes maybe once a month.
- **To:**
  - **pre-commit (target < 30 s):** staged lint (adopt the already-installed but unused
    `lint-staged` instead of the hand-rolled grep) + the fast guardrail scripts + i18n parity +
    `turbo typecheck` (affected; cache-hit ≈ 2 s, worst case 18 s). *Drop:* build, flyctl
    validate, Docker.
  - **pre-push (optional, tolerable at ~1 min):** `turbo build test` on affected packages.
  - **CI (PR):** full `turbo build typecheck lint` with remote/actions cache + `verify:all --ci`
    + **`test:unit` on the fresh-provision DB (§2.1)** + a **Docker build job** using
    `docker/build-push-action` with `cache-from/to: type=gha` — this restores the exact
    protection the hook gave, off the human/agent hot path, with layer caching the local build
    never had. Fly already builds remotely on deploy (`flyctl deploy --remote-only`), so the
    image is *also* built by the deploy path; the CI job is the PR-time early warning.
  - Matrix is not needed at this scale (one Node version, one platform) — YAGNI.
- **Quantified gain:** commit wall-time 5–6 min → <30 s. At the current multi-lane agent cadence
  (the git log shows double-digit commits/day), this is **1–2 hours/day of unblocked wall-clock**,
  plus deleting the docker-disk-guard machinery from the hot path (root cause removed, not
  guarded). Biggest single DX win in this lane; memory (meta-loop audit P1) already flagged it.
- **Effort:** S (hook edit + one workflow job; `.github/` is operator-gated, so ship it as a
  staged patch alongside `proposed-ci-test-gates/APPLY.md`). **Risk:** Dockerfile breakage now
  surfaces in PR CI (~minutes later) instead of pre-commit — acceptable; keep `docker build` as an
  on-demand script for Dockerfile-touching work.
- **Verdict: DO FIRST.** No dependency on §1 (though §1 makes the typecheck step near-free).

## 5. Module boundaries & god-file decomposition

**Boundaries.** Today layering is enforced only by point ESLint rules for specific incidents
(`local/no-voice-app-import`, `no-admin-prefix-register`, `no-raw-courier-ws-send` —
`eslint.config.js:44-48`). Nothing stops `packages/domain` importing `pg`, or a new `apps/web` →
`apps/api` import. **Add dependency-cruiser** (S: one `.dependency-cruiser.cjs`, ~60 lines, runs
in CI in seconds, off the lint hot path) with exactly these rules — no more:
1. `apps/*` may import `packages/*`, never another app.
2. `packages/domain` + `packages/shared-types` are pure: no `fastify`/`pg`/`react`/`ioredis`, no
   other workspace package except each other (today `domain` and `shared-types` already have zero
   workspace deps — lock that in).
3. `packages/ui` never imports `apps/*` or `packages/db|platform`.
4. No package imports another package's internals via relative path (`../../packages/...` — the
   `@ui/*` tsconfig path alias in `apps/web/tsconfig.json` is the sanctioned exception; consider
   retiring it toward the package entry point later, not now).
eslint-plugin-boundaries would work but couples boundary checks to every editor lint pass;
Nx tags require Nx (rejected §1).

**God-file seams** (behavior-preserving; run each under the existing `refactor-converge` loop with
the unit suite + staging E2E as the freeze harness; verified against repowise skeletons):

- **`apps/api/src/routes/orders.ts` (980 lines, CCN 156, `has_test_file: false`).** The hard part
  is already done — pricing/persistence/canonical-hash/preflight/dispatch live in `lib/`
  (`order-pricing.ts`, `order-persistence.ts`, `order-canonical.ts`, `preflight.ts`,
  `dispatch.ts` — see imports at `orders.ts:8-27`). Remaining seam is *route-level*: split the
  single `orderRoutes` plugin into `routes/orders/{create.ts,status.ts,read.ts,mappers.ts}` — the
  ~870-line `POST /orders` handler (lines 111–980 per skeleton) is `create.ts`; the exported-but-
  unimported `mapItemRow`/`mapOrderRow` go to `mappers.ts`. Same registration signature in
  `server.ts`; zero contract change. Then give each file the test file it lacks.
- **`apps/api/src/server.ts` (890 lines, CCN 116).** The composition-root pattern already exists
  (`bootstrap/{routes,notifications,workers,messaging}.ts`) — `main()` just never finished
  moving in. Extract: (a) HTTP-policy hooks (security headers, CORS + public-route override,
  static cache-header override — `server.ts:179-…`) → `bootstrap/http-policy.ts`; (b) the Zod
  validator/serializer compilers → `bootstrap/validation.ts`; (c) provider construction
  (storage/parsers/translate) → `bootstrap/providers.ts`. Target: `main()` < 200 lines of
  ordered `register` calls.
- **`apps/web/src/pages/admin/MenuManagerPage.tsx` (1405 lines).** The file already contains
  self-contained units with their own tested pure functions: `KitchenBusyToggle`, the
  menu-availability `ScheduleRow` editor, the import preview/commit flow (own Zod schemas at
  top), the undo/redo product-form draft (`ProductFormDraft`, `makeEmptyProductDraft`,
  `productDraftsEqual`, `replaceProductInCategories` — already exported for tests). Seams:
  `pages/admin/menu/{ProductForm.tsx, ImportFlow.tsx, ScheduleEditor.tsx, KitchenBusyToggle.tsx,
  menu-manager-lib.ts}` with `MenuManagerPage.tsx` as layout+data-orchestration via the existing
  `useMenuData` hook.
- **Flag for the same treatment (not in the first wave):** `MenuPage.tsx` (1811 — now the largest
  FE file), `spa-proxy.ts` (885, CCN 174, 99.4th-percentile churn), `courier/shifts.ts`
  (health 1.0, CCN 40, untested).
- **Anti-regrowth ratchet:** `max-lines: ["warn", 800]` repo-wide + error-level for the
  decomposed files (matches the existing burn-to-zero-then-error ratchet convention in
  `eslint.config.js:36-39`).

**Effort:** M per god file (~0.5–1 day each with the freeze harness). **Risk:** medium on
`orders.ts` (money red-line — council-gated per standing rules); low on the FE file.
**Gain:** the four worst health scores (all 1.0) are most of the 4.1/10 hotspot number; route-level
files become unit-testable, and agent edit-collision pressure on hot files drops (two commit
collisions on shared files already logged in memory).

## 6. Code health & consistency

- **Dead code (S, do in one sweep):** delete the 18 high-confidence findings (190 lines) — 
  notably the never-imported `requireRole`/`softVerifyAuth` in `apps/api/src/plugins/auth.ts`
  (dead *authz* surface is a comprehension hazard, not just clutter) and the dead
  `ssr-renderer.ts` components. Then triage the 98 ≥0.5-confidence findings (~2,370 lines) with
  `repowise get_dead_code --safe_only`. Re-verify each against live grep before deletion (index
  is 11 days old).
- **Duplication:** the memory-flagged fetch/WS duplication is mostly *already converged* — 1
  direct `new WebSocket(` remains in FE code and 13 raw `fetch(` calls across 9 files outside the
  `apiClient`/`publicApi` layer. Finish with one `refactor-converge` pass + promote
  `local/no-direct-websocket` from warn to error (it's at zero-adjacent).
- **Lint ratchet:** the plugin (26 rules, 967 lines) is genuinely good — rules encode real
  incident classes (false-green assertions, allergen surface, courier WS relay). The weakness is
  that ~15 rules sit at `warn` forever (`no-raw-sql`, `no-explicit-any`, `no-hardcoded-string`,
  `no-prod-base-in-test` "99 literals pending"…). Warnings that never gate are decoration.
  Adopt `--max-warnings=<current count>` in CI to freeze the number, then burn down 2–3 rules a
  month to `error` per the existing convention. Zero new tooling.
- **Repo-root hygiene (ponytail):** the root directory carries ~15 stray screenshots, one-off
  audit JSONs, `graphify-out/`, `run-history.jsonl`, dated summary MDs. Move to `docs/assets/` /
  `analytics/` or delete; the root is the first screen every human and agent reads. (S, cosmetic,
  real navigation win.)
- **What actually moves the 4.1/10 hotspot score:** §5 decomposition + adding the missing test
  files for `orders.ts`/`server.ts`/`shifts.ts` (all `has_test_file: false` per repowise) —
  coverage-adjacent biomarkers, not formatting, dominate the score.

## 7. The custom harness — honest verdict: **two systems wearing one name**

**The deterministic arm is a net asset — keep it, consolidate it.**
Evidence it works: `verify:all` (23 gates) caught the silently-disarmed-gates class
(gate-armament, ledger #47/#48); the fresh-provision CI job exists because six real
non-bootable-DB bug classes shipped past ordering checks (`ci.yml:52-56`); guard-bash blocked 33
real mutations in 2 days including a live block of an ungoverned `.github/` write; the 26 local
ESLint rules are distilled incidents with red→green ledger rows. This is exactly the "deterministic
artifacts decide" design working. Cost is low: verify:all is CI-side; the ESLint rules ride the
existing lint pass.

**The advisory arm is majority ceremony by its own telemetry — tier it down.**
Evidence: 1,040 hook events in ~2 days = 360 lesson injections + 294 route-request nudges + 59
red-line advisories (~713 context injections) against 38 enforcement decisions — a 19:1
advisory-to-decision ratio, consistent with the memory-logged 69-blocks/274-nudges session. The
advisory learning chain (reflection→critics→librarian) completed end-to-end **once** in its first
three weeks (meta-loop audit 2026-07-02) and had to be resuscitated; `loops/` holds 180 files and
16–20 registered loops while telemetry shows ~2 loops with real usage (demo-builder,
acquisition); three loop registries disagreed until a sync gate was added *to guard the harness
itself* — guardrails guarding guardrails is the ceremony signature.

**Recommended tiering (P2/P3 direction from memory, made concrete):**
1. **Hard-block only red-lines** (auth/money/RLS/`packages/db/migrations/`/bulk-edit + `.github/`
   + the harness's own authority files). Everything else: allow + log. `serious-gate`/
   `require-classification` on ordinary product edits → log-only.
2. **Cap advisory injection:** `pre-edit-lessons` fires once per file per session (not 360×/2
   days); collapse `route-request`'s serious+repeat nudges into one message, red-line globs only.
3. **Archive dormant loops:** any `loops/*.yaml` with zero runs in 30 days moves to
   `loops/archive/`; one registry SoT (already gated — finish it). Keep the breaker + finalize
   telemetry for the 2 loops that actually run.
4. **Keep untouched:** verify:all, the ESLint plugin, the regression ledger, protect-paths,
   red→green promotion discipline, Zod-at-boundary, the Mandatory Proof Rule. These are the parts
   with receipts.
5. **Delete the 1.4 GB of stale `.claude/worktrees/`** once the voice-FE preservation
   (commit a43485d0) is confirmed merged — it is 98% of the harness's disk footprint and already
   caused one near-miss data-loss scramble.

**Net:** friction on safe work drops (fewer blocks *and* ~10× fewer context injections), while
every gate with a proven catch survives. Effort S–M (hook edits + config), risk low if the
red-line list is enacted verbatim rather than re-litigated per hook.

---

## 8. Priority order & recommendation table

| # | From | To | Effort | Risk | Expected gain | Verdict |
|---|---|---|---|---|---|---|
| 1 | Docker build + full build/typecheck in pre-commit (~5–6 min/commit) | pre-commit = staged lint + fast gates + affected typecheck (<30 s); Docker → CI job w/ GHA layer cache; build → pre-push/CI | **S** | Low (Dockerfile breaks surface in PR CI) | ~5 min/commit → 1–2 h/day at current cadence; deletes disk-guard hazard | **DO FIRST** |
| 2 | 1,198 unit tests run nowhere automatically | `test:unit` in CI fresh-provision job (staged patch exists) | **S** (operator apply) | Low (proven green 2026-07-02) | The suite becomes a gate, not a habit | **DO FIRST** |
| 3 | Plain `pnpm -r`, zero caching | Turborepo 2.7 + local/remote cache + `--affected` | **S** | Low, removable | No-op 46 s → ~2 s; scoped rebuilds; CI cache | **DO** |
| 4 | 26 hand-numbered stage/phase scripts, ~70 root scripts | Fold into named node:test files + 1 `test:integration`; **no Vitest migration** | **M** | Low | Root scripts ~70→35; discoverability; one runner | **DO** |
| 5 | tsc 5.6 for typecheck (18.6 s) | tsgo (`typescript@rc`→GA ~Jul 2026) for `--noEmit` lane; tsc keeps emit | **S** | Low (dual-lane) | 18.6 s → ~2–3 s | **DO at 7.0 GA** |
| 6 | No boundary enforcement beyond point rules | dependency-cruiser, 4 rules, CI-side | **S** | Low | Layering can't rot; domain stays pure | **DO** |
| 7 | 4 god files (CCN 116–174, untested) | Seam-split per §5 + max-lines ratchet, under refactor-converge freeze | **M×4** | Med (orders.ts = money red-line, council-gated) | Hotspot health 4.1 → ~7 class; testable routes | **DO (staged)** |
| 8 | 2,370 dead lines; 15 forever-warn rules | High-confidence deletion sweep; `--max-warnings` freeze + monthly promote | **S** | Low | Less noise; ratchet with teeth | **DO** |
| 9 | Advisory harness 19:1 noise ratio; 180-file loops dir | Red-line-only hard blocks; injection caps; archive dormant loops; keep deterministic arm intact | **S–M** | Low | ~10× less agent-context noise, zero safety loss on red-lines | **DO** |
| — | node:test → Vitest | — | M–L | Med (182-file assert rewrite) | None (26 s suite has no speed problem) | **REJECT** |
| — | Project references / isolatedDeclarations | — | M | Med | Overlaps turbo + tsgo | **REJECT** |
| — | Nx / moonrepo | — | M | Elevated (Nx supply-chain history) / niche | Nothing Turborepo lacks at 10 pkgs | **REJECT** |

**Dependency notes:** #1, #2, #6, #8, #9 are independent — start all immediately. #3 feeds #1's
"affected typecheck". #5 waits ~weeks for TS 7.0 GA. #7 is the only council-gated item (money
red-line on `orders.ts`) and should ride the existing refactor-converge loop with the (post-#2)
CI-enforced unit suite as its freeze harness. `.github/` changes (#1, #2) ship as staged operator
patches per the protected-zone rule.
