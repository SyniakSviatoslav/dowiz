# 13 — Scripts / Guardrails / CI-CD / Ops: Inventory & Rebuild Map (Lane D)

- **Date:** 2026-07-04 · **Lane:** D (rebuild-mapping program, per `06-complete-rebuild-stack.md`)
- **Scope:** every operational script, quality gate, hook, workflow, budget, and test harness in the
  repo, each mapped **PORT / KEEP / RETIRE** for the Rust(axum/sqlx) + Astro/Svelte stack.
- **Reading key:** the frontend REMAINS TypeScript (Astro + Svelte islands) — so FE-facing ESLint
  rules **port as ESLint** (rewritten for Svelte AST where needed), NOT to clippy. Only API-facing
  rules cross the language boundary. The Playwright E2E suite is the **language-independent parity
  oracle** and is KEEP by decree (06 §1).

## 0. Machine-verifiable extraction commands (reconciliation block)

| Census | Extraction command (repo root) | Count |
|---|---|---|
| Root npm scripts | `node -e "console.log(Object.keys(require('./package.json').scripts).length)"` | **70** |
| — of which `verify:*` / `guardrail:*` / `test:*` | same, filtered by prefix | 19 / 6 / 17 |
| `scripts/` top-level files | `find scripts -maxdepth 1 -type f \( -name '*.mjs' -o -name '*.ts' -o -name '*.sh' -o -name '*.js' -o -name '*.py' \) \| wc -l` | **69** |
| `scripts/` subdir scripts | `find scripts/automation scripts/*-pilot -type f \( -name '*.mjs' -o -name '*.sh' \) \| wc -l` | **9** (Σ 78) |
| `apps/api/scripts/` entries | `ls apps/api/scripts \| wc -l` | **15** |
| Guardrail scripts | `ls scripts/guardrail-*.mjs \| wc -l` | **11** (+1 `.test.mjs` proof) |
| verify-all gate registry | `grep -c "name:" scripts/verify-all.ts` | **25** gates (20 armed in `--ci`) |
| eslint-plugin-local rules | `grep -cE "^    '[a-z-]+': \{" tools/eslint-plugin-local/src/index.js` | **26** |
| GitHub workflows | `ls .github/workflows \| wc -l` | **3** (5 jobs total) |
| Husky pre-commit steps | `.husky/pre-commit` (read) | **9** steps |
| E2E specs | `find e2e -name '*.spec.ts' -not -path '*node_modules*' \| wc -l` | **174** |
| Visual snapshot assertions | `grep -c toHaveScreenshot e2e/visual/*.spec.ts` | **27** (×3 viewports ≈ 81–162 shots; net sized "~180" incl. lang loop) |
| Visual baseline PNGs in tree | `find e2e/visual -name '*.png' \| wc -l` | **0** (baselines live in the pinned-renderer CI flow, gitignored) |
| Stage/phase tsx harnesses | `ls apps/api/tests/test-stage*.ts apps/api/tests/test-phase*.ts \| wc -l` | **26** |
| Playwright configs | `find . -name 'playwright*.config.ts' -not -path '*node_modules*' -not -path '*worktrees*'` | **3** |
| Classified env vars | `grep -cE '^\|' compliance/env-classification.md` − 2 header rows | **32** |
| Loop cards | `ls loops/*.yaml \| wc -l` | **20** (5 CERTIFIED) |
| `.claude/hooks` files | `ls .claude/hooks \| wc -l` | **9** (protected — never modified by this lane) |

---

## 1. Root scripts census (70 npm scripts)

Categories: **G**=guardrail · **V**=verify-gate · **B**=backup/DR · **D**=dev · **T**=test-harness ·
**M**=meta/harness · **O**=ops. Disposition tallies at §6.

| npm script | What it does | Cat | Disposition → new-stack equivalent |
|---|---|---|---|
| `build` | `pnpm -r build` all workspaces | D | **RETIRE** → `cargo build` + `astro build` |
| `typecheck` | `pnpm -r typecheck` | V | **PORT** → `cargo check` (BE) + `svelte-check`/`tsc` (islands) |
| `lint` | eslint whole repo | V | **PORT** → `cargo clippy -- -D warnings` (BE) + eslint (FE) |
| `lint:gates` | run eslint-plugin-local against its red fixtures (rules stay armed) | G | **PORT** → keep for FE eslint rules; Rust gates get red→green `#[test]`/fixture proofs in CI |
| `format` | prettier | D | **PORT** → `cargo fmt` + prettier(FE) |
| `verify:env` | Zod-validate env completeness | V | **PORT** → Rust config crate: serde env deserialization at boot + `--check-env` subcommand |
| `verify:db` | DB structural sanity (packages/db) | V | **KEEP** (speaks SQL; re-point connection) |
| `seed` | seed dev DB | D | **KEEP** (SQL seeding; later a Rust `xtask`) |
| `verify:rls` | 🔴 adversarial RLS probes per table | V | **KEEP** (pure SQL probes — language-agnostic; council-gated any change) |
| `verify:migrations` | migration ordering/idempotency check | V | **PORT** → sqlx/refinery checksum ordering natively + a slim forward-only CI script |
| `verify:fresh-provision` | brand-new PG → migrate → seed → boot → /health 200 → menu served | V | **KEEP** (shell; swap boot cmd to the Rust binary) |
| `compliance:gate` | privacy-invariants ↔ compliance/ SoT parity | G | **KEEP** (docs-driven, language-free) |
| `guardrail:owner-active` | ADR-0004: `role='owner'` queries must pair `status='active'` | G | **PORT** → single sqlx membership-query module + CI grep over `*.rs`/`*.sql` for unpaired filters |
| `guardrail:spike-boundary` | spikes/ code never imported by execution code | G | **KEEP** (repo-layout grep, language-free) |
| `guardrail:deliver-v2` | cash-as-proof completion parity + no raw cancel | G | **PORT** → Rust order-state-machine unit tests + source gate |
| `guardrail:corpus-reachability` | injection corpus actually reaches the menu-parser | G | **PORT** → corpus fixtures become `cargo test` inputs of the Rust parser |
| `guardrail:license` | copyleft/forbidden-dep fence + env-classification fail-closed | G | **PORT** → `cargo-deny` licenses/bans (Rust deps) + script kept for FE deps & env-classification manifest |
| `guardrail:hook-matchers` | .claude edit-hooks cover Edit\|Write\|MultiEdit | M | **KEEP** (harness-plane, stack-independent) |
| `verify:all` | runs the 25-gate registry (`--ci` = 20 static gates) | V | **PORT** → keep runner, re-point registry entries to cargo/astro equivalents |
| `test:unit` | node --test over api/worker/web/packages/tools | T | **PORT** → `cargo test` (BE) + vitest/node--test (islands) |
| `backup:restore` | restore encrypted backup | B | **KEEP** |
| `migrate:create` / `migrate:up` / `migrate:down` | node-pg-migrate lifecycle | D | **RETIRE** ×3 → new-era tool (sqlx::migrate vs refinery, Lane C); history frozen at 157 |
| `bundle` | esbuild single-file CJS artifacts (build-apps.ts) | D | **RETIRE** → `cargo build --release` is the bundler |
| `dev:api:1` / `dev:api:2` / `dev:worker` / `dev:ui` / `dev:all` | local dev processes | D | **RETIRE** ×5 → `cargo run` / `bacon`/`watchexec` + `astro dev` |
| `test:phase0` … `test:phase5*`, `test:nx-notification-infra`, mega-chain (13 entries) | per-stage builder harnesses (see §5) | T | **RETIRE** ×13 → intent absorbed by Rust integration tests + Playwright parity slices |
| `verify:secrets` | no secrets committed | G | **KEEP** (+ add gitleaks; see incident memory — history scrub is separate) |
| `free-tier:watch` | Fly/Supabase free-tier quota watch | O | **KEEP** |
| `verify:launch` | pre-launch live checklist | V | **KEEP** (HTTP-driven; re-point) |
| `test:phase5-rls-adversarial` | 🔴 RLS adversarial suite | T | **PORT** → parity oracle during transition, then Rust/SQL integration test (council-gated) |
| `test:phase5-jwt-rotation` | 🔴 JWT kid rotation proof | T | **PORT** → Rust auth-crate integration test |
| `test:phase5-integrity` | 🔴 money/state integrity checks | T | **PORT** → Rust integration test |
| `backup:verify` / `backup:drill` / `backup:list` | verify restorability / timed drill / list backups | B | **KEEP** ×3 |
| `verify:nx` | notification (NX) flow wiring | V | **PORT** → Rust integration test over the notification pipeline |
| `verify:i18n-coverage` | UI strings covered by catalog | V | **PORT** → re-target new i18n catalog (Paraglide or ported SSOT, Lane B) |
| `verify:contrast` | WCAG contrast over token pairs | V | **KEEP** (token files carry over unchanged) |
| `verify:error-contract` | error-envelope shape parity (ADR-0010) | V | **PORT** → OpenAPI 3.1 SSOT: envelope schema in utoipa + contract test; drift impossible by construction |
| `verify:ccc-secrets` | ccc tool never egresses secrets | M | **KEEP** (harness tool) |
| `verify:menu-parse` | menu-parser corpus run | V | **PORT** → Rust parser test w/ same corpus |
| `ccc` | context-compiler harness CLI | M | **KEEP** |
| `verify:event-wiring` | every emitted event has a consumer | V | **PORT** → Rust integration test / exhaustive-match on event enum (compiler-assisted) |
| `verify:schema-queries` | live-DB check that queried columns exist | V | **RETIRE** → **absorbed, stronger**: `cargo sqlx prepare --check` proves every query against the schema at compile time |
| `verify:connection-lifecycle` | WS/pool connection lifecycle invariants | V | **PORT** → Rust integration test (tokio) |
| `verify:n2` | N+1 query detector | V | **PORT** → query-count assertion via sqlx logging in integration tests |
| `verify:privacy` | PII-leak detector vs API responses | V | **KEEP** (black-box HTTP test — language-independent) |
| `check:core` / `check:core:ci` | metric-core deterministic "done" checks | M | **KEEP** ×2 (re-point cmd entries to cargo) |
| `size:check` | size-limit JS budgets | V | **KEEP** (tool is dist-agnostic; new globs for Astro output — see §3) |
| `lhci` | Lighthouse CI vs staging storefront | V | **KEEP** |
| `storefront:guard` | admin code not in storefront eager graph | G | **PORT** → rewrite against Astro/Vite manifest (islands make the graph explicit) |
| `prepare` | husky install | M | **KEEP** |

### 1b. `scripts/` file census (78 files: 69 top-level + 9 in subdirs)

Files already covered above (wired via npm) are not repeated; dispositions for the rest:

| File | Purpose (1-line) | Cat | Disposition |
|---|---|---|---|
| `acquisition-bulk-provision.mjs` | CSV of restaurants → shipped /internal provisioning pipeline | O | **KEEP** (HTTP-driven) |
| `agent-health-pass.mjs` | harness measuring itself (counsel Duty B) | M | **KEEP** |
| `analyze-bottlenecks.py` | one-off perf analysis | O | **RETIRE** (one-off) |
| `bootstrap-pgboss.ts` | install pg-boss schema | D | **RETIRE** → new queue bootstraps via migrations (Lane A) |
| `build-apps.ts` | esbuild bundler for dist/{api,worker,migrate} | D | **RETIRE** → cargo |
| `check-auth-schema.js` / `check-jwt.js` / `check-route.js` | one-off debug probes | D | **RETIRE** ×3 |
| `check-contracts.mjs` | contract-map placeholder (echo in metric-core) | V | **RETIRE** → absorbed by openapi-diff gate |
| `ci-connection-preflight.mjs` | prove DB URLs+SSL work before migrate/deploy | V | **KEEP** (drop `_pg-loader` hack; use psql or a tiny Rust bin) |
| `ci-migration-preflight.mjs` | pending migrations apply against PROD's actual schema first | V | **KEEP** |
| `ci-schema-drift.mjs` | compare column sets of two DBs, fail on divergence | V | **KEEP** |
| `demo-builder.mjs` / `demo-from-wolt.mjs` | prospect → polished claimable demo storefront | O | **KEEP** ×2 |
| `deploy-staging.sh` | canonical staging deploy carrying dark-launch VITE build-args | O | **KEEP** (build-arg names change with Astro flags) |
| `docker-disk-guard.sh` | reclaim Docker layers so local PG disk never fills | O | **KEEP** |
| `guardrail-definer-search-path.mjs` (+`definer-baseline.json`) | every SECURITY DEFINER fn pins search_path (ledger #33) | G | **KEEP** (scans SQL migrations — extend to new migration dir) |
| `guardrail-gate-armament.mjs` | governance gates are ARMED, not just registered (P0 rearm) | M | **KEEP** |
| `guardrail-ledger-integrity.mjs` | REGRESSION-LEDGER `#N` rows unique | M | **KEEP** |
| `guardrail-no-set-cookie.mjs` | bearer-only invariant: no Set-Cookie anywhere (B3 P0-5) | G | **PORT** → cargo-deny ban `cookie`/`tower-cookies` crates + CI grep for `set-cookie` in `*.rs` |
| `guardrail-sandbox-staleness.mjs` (+`.test.mjs`) | no work in stale sandbox worktrees (meta-controller-proposed) | M | **KEEP** ×2 |
| `harness-curation-local.sh` | local weekly self-improvement run | M | **KEEP** |
| `i18n-add.ts` / `i18n-parity.ts` | add key to catalog SSOT / al-en parity gate | V | **PORT** ×2 → re-target Lane-B i18n format; parity gate preserved as CI check (06 §1) |
| `live-multi-courier-check.mjs` | 3 concurrent couriers vs deployed env (GPS + WS) | T | **KEEP** (black-box) |
| `loops-registry-sync.mjs` | registry.md → runs/registry.json derivation | M | **KEEP** |
| `meta-controller.mjs` (+`.test.mjs`) | L5 gated self-modification loop, immutable core | M | **KEEP** ×2 |
| `migrate-runner.ts` | bundled release-command migrator | D | **RETIRE** → sqlx::migrate/refinery binary as `release_command` |
| `new-dep-scan.mjs` | detect newly-added libs for plane-maintainer review | M | **PORT** → extend to `Cargo.lock` diffing |
| `offer-builder.mjs` / `radar-scout.mjs` | prospect outreach packet / venue scouting | O | **KEEP** ×2 |
| `openrouter-implement.ts` | OpenRouter codegen experiment (slugs dead) | M | **RETIRE** |
| `_pg-loader.mjs` | resolve workspace `pg` from pnpm store for scripts/ | D | **RETIRE** (Node-packaging hack, obsolete) |
| `plane-guard.mjs` | 11 memory-corpus meta-patterns as ONE deterministic gate | M | **KEEP** |
| `plane-report.mjs` / `plane-telemetry.mjs` (+`.test.mjs`) | plane status digest / single telemetry egress choke-point | M | **KEEP** ×3 |
| `platform-admin-grant.ts` | grant platform-admin role | O | **PORT** → admin subcommand of the Rust binary |
| `run-promo-migration.js` / `test-promo-create.js` | one-off promo migration + probe | D | **RETIRE** ×2 |
| `sandbox-swarm-gate.mjs` | multi-agent swarm orchestration aid (gated) | M | **KEEP** |
| `secrets-env.sh` | load SOPS-encrypted local-dev secrets into shell | O | **KEEP** |
| `song-of-singularity.mjs` | weekly ethical infusion verse (deterministic by ISO week) | M | **KEEP** |
| `ui-verify-floor.sh` | mechanical floor of the UI build-verification loop | M | **KEEP** |
| `verify-courier-pins.mjs` | one-off rendered-pin data proof (superseded by Playwright spec) | T | **RETIRE** |
| `verify-stage9.ts` / `verify-stage10.ts` | historical stage verifiers | T | **RETIRE** ×2 |
| `automation/{notify,tier1-run,tier2-overnight,tier3-batch}.sh` | tiered automation runners + notifier | M | **KEEP** ×4 |
| `decepticon-pilot/authorized-scope-attest.mjs`, `scrape-pilot/scraping-conduct-attest.mjs`, `skyvern-pilot/{count-items,no-credential-attest}.mjs`+`run-measurement.sh` | ethics/scope attestation gates for tool pilots | M | **KEEP** ×5 |

### 1c. `apps/api/scripts/` (15 entries — gate-relevant subset)

`verify-{contrast,error-contract,event-wiring,i18n-coverage,nx-flow,schema-queries,connection…}` are
dispositioned in §1 via their npm wrappers. Remainder: `verify-no-raw-status-update.ts` (order status
changes only via the transition fn — **PORT**: Rust state-machine enum makes raw updates uncompilable +
CI grep on `UPDATE orders SET status`), `verify-orphans.ts` (**KEEP**, SQL), `config-drift.ts` (**KEEP**,
env-vs-Fly drift), `release-gate.ts` (**KEEP**), `erase-access-request.ts` GDPR (**PORT** → admin
subcommand, council-gated), `seed-demo-from-prod.mjs` (**KEEP**), `paddle-ocr.py` (**KEEP** — OCR sidecar
candidate per Lane A), `phase0-probe.ts` (**RETIRE**), `radar/` (**KEEP**, ops).

---

## 2. Guardrail census → intent survival map

### 2a. Guardrail scripts (11) — dispositions in §1/§1b; all 11 intents survive (none dropped)

`corpus-reachability · definer-search-path · deliver-v2 · gate-armament · hook-matchers ·
ledger-integrity · license · no-set-cookie · owner-active-membership · sandbox-staleness ·
spike-boundary` — 5 PORT (cross language), 6 KEEP (SQL-/docs-/harness-plane, language-free).

### 2b. eslint-plugin-local — all 26 rules, intent → new-stack enforcement

**No intent maps to nothing.** Legend: **ESLint(FE)** = rule survives as ESLint on the Astro/Svelte
app (FE stays TS); **ESLint(E2E)** = survives on the kept Playwright suite; **Rust** = compiler/
clippy/cargo-deny/type-design; **CI-grep** = deterministic text gate (weaker teeth — flagged).

| # | Rule | Intent (incident lineage) | New-stack enforcement |
|---|---|---|---|
| 1 | `no-hardcoded-string` | all UI copy via `t('key','fallback')` (i18n SSOT) | **ESLint(FE)** rewritten for Svelte AST; + i18n-parity CI gate |
| 2 | `no-raw-sql` | no SQL string interpolation (injection) | **Rust**: `sqlx::query!`/`query_as!` macros only; clippy.toml `disallowed-methods` for runtime `sqlx::query(` string forms; sqlx offline check proves every query |
| 3 | `no-hardcoded-color` | hex colors → `var(--brand-*/--color-*)` tokens | **ESLint(FE)** + stylelint on Astro styles |
| 4 | `no-hardcoded-tailwind-color` | no raw Tailwind palette classes | **ESLint(FE)**; optionally Tailwind theme restriction (empty default palette) |
| 5 | `no-arbitrary-tailwind` | bracket values bypass design scale | **ESLint(FE)** |
| 6 | `no-raw-form-control` | native `<select>/<textarea>` → shared DS atoms | **ESLint(FE)** (eslint-plugin-svelte custom) |
| 7 | `no-arbitrary-font-size` | `text-[10px]` → type-scale tokens | **ESLint(FE)** |
| 8 | `no-ts-nocheck` | never disable the type checker | **ESLint(FE)**; Rust analog: `#![forbid(unsafe_code)]` + `clippy::allow_attributes_without_reason` (no silent lint escapes) |
| 9 | `no-raw-any` | `as any` kills type safety | **ESLint(FE)** (@typescript-eslint); Rust analog: `clippy::as_conversions`/cast lints in deny-set |
| 10 | `no-duplicate-import` | merge imports | **ESLint(FE)**; Rust: rustfmt `imports_granularity` — cosmetic, absorbed |
| 11 | `require-auth-hook` | 🔴 owner/courier routes must be auth-gated | **Rust type-state**: handlers take `OwnerClaims`/`CourierClaims` axum extractors — an ungated handler cannot compile; + CI **401-sweep test** iterating OpenAPI paths asserting 401/403 without token (parity oracle) |
| 12 | `no-empty-catch` | never silently swallow errors | **ESLint(FE/E2E)**; Rust: `#[must_use]` Results + `clippy::let_underscore_must_use`, `clippy::map_err_ignore` |
| 13 | `no-process-exit` | no exit() in library code | **Rust**: `clippy::exit` (restriction lint, direct equivalent) |
| 14 | `no-permissive-status-assertion` | test must assert EXACT status, not `[200,500].toContain` | **ESLint(E2E)** unchanged; Rust tests: `assert_eq!(status, 200)` idiom + review |
| 15 | `no-tautological-assertion` | `expect(true)` can never fail (217-instance class) | **ESLint(E2E)**; Rust: `clippy::assertions_on_constants` (direct equivalent) |
| 16 | `no-swallowed-catch` | `.catch(() => {})` vanishes failures | **ESLint(FE/E2E)**; Rust: `clippy::map_err_ignore` + must_use |
| 17 | `no-truthy-on-identifier` | token/id/url must be shape-asserted (JWT/UUID) | **ESLint(E2E)** unchanged |
| 18 | `no-prod-base-in-test` | tests never default BASE to prod host | **ESLint(E2E)** + **CI-grep** over Rust test files for prod-host literals |
| 19 | `no-mock-in-prod` | mock/fake/stub data out of prod paths | **ESLint(FE)**; Rust: `#[cfg(test)]` compiles test doubles out — absorbed by the language |
| 20 | `no-insecure-random` | Math.random for tokens/secrets (dev-login backdoor, ADR-0003) | **Rust**: auth/token crates depend ONLY on `ring`/`getrandom` (cargo-deny per-crate ban of `rand` in those crates) + **CI-grep** `thread_rng` near security identifiers — *grep-strength until a dylint lands* |
| 21 | `no-direct-websocket` | one shared FE WS client owns reconnect+ordering | **ESLint(FE)** rewritten for the Svelte WS store |
| 22 | `no-raw-courier-ws-send` | 🔴 ADR-0013: fan-out only via relay guard (C1 reassign leak) | **Rust module visibility**: the member's `UnboundedSender` is private to the relay-guard module — raw sends uncompilable outside it; + WS-authz integration tests (council-gated port) |
| 23 | `no-admin-prefix-register` | 🔴 B4: /api/admin mounts ONLY via the gated index | **Rust visibility**: admin sub-routers `pub(super)` — only `admin/mod.rs` (which applies the platform-admin layer) can mount them; + **CI-grep** for `/api/admin` nesting outside the module |
| 24 | `no-recipe-only-allergen-read` | 🔴 #12: allergen surface = declared∪recipe, never recipe-only | **Absorbed by API contract**: the Rust API computes `allergen_surface` server-side; OpenAPI schema exposes ONLY the unioned field, no recipe-only accessor ships to FE; + unit test on the union fn |
| 25 | `no-voice-engine-callback` | ADR-0015: voice engine yields readonly data, no write closures | **ESLint(FE)** unchanged (voice package stays TS) |
| 26 | `no-voice-app-import` | voice engine imports no app/fetch/Cart mutator | **ESLint(FE)** unchanged (+dependency-cruiser optional) |

**GAP count: 0** unmappable. **3 flagged weaker-teeth** (CI-grep until a dylint/custom lint is
invested): #18(Rust side), #20, #23(grep half). Each still has a deterministic gate from day one.

### 2c. verify-all registry (25 gates, 20 CI-armed)

Runner **PORT**s intact; each entry re-points per §1. The registry pattern itself (one script = one
exit-code authority list) is a keeper — in the new stack it becomes the single `verify:all --ci` CI
step invoking cargo/astro/script gates.

---

## 3. Hooks / CI census

### 3a. `.husky/pre-commit` (9 steps) — KEEP the hook, re-point steps

1. eslint on staged JS/TS (manual staged-file lint — **no lint-staged package**; `package.json` has no
   `lint-staged` key) → PORT: + `cargo fmt --check` & clippy on staged `*.rs`
2. guardrail-corpus-reachability (blocking) → per §1
3. guardrail-license (blocking) → per §1
4. guardrail-hook-matchers → KEEP
5. i18n-parity (only when catalog staged) → PORT (new catalog path)
6. `pnpm -r typecheck` → PORT (`cargo check` + svelte-check)
7. `pnpm -r build` → PORT (`cargo build` + `astro build` — consider check-only locally for speed)
8. `flyctl config validate` → KEEP
9. Docker build check + `docker-disk-guard.sh` 15/20GB tiers → KEEP (image build gets ~10× smaller)

### 3b. `.claude/hooks` (9 files — PROTECTED, harness plane, all KEEP as-is)

`guard-bash.sh · loop-detector.sh · post-edit-gates.sh · pre-edit-lessons.sh · protect-paths.sh ·
red-line-doubt-gate.sh · require-classification.sh · route-request.sh · serious-gate.sh` — these gate
the *agent workflow*, not the product language; fully stack-independent. **KEEP ×9.**

### 3c. GitHub workflows (3 files, 5 jobs)

| Workflow · job | Steps (today) | Disposition |
|---|---|---|
| `ci.yml` · **validate** | install → build → typecheck → lint → lint:gates → `verify:all --ci` (20 gates) → verify:migrations → verify:secrets → compliance:gate | **PORT** → §7 PR pipeline |
| `ci.yml` · **fresh-provision** | PG16+Redis services, create migrator/app roles, throwaway RS256 keys, `verify:fresh-provision` (migrate→seed→boot→/health→menu) | **PORT** → same shape; Redis service DROPPED (no-Redis decision), boot the Rust binary |
| `ci.yml` · **deploy** (main only) | migrate → `flyctl deploy --remote-only` → 4 post-deploy Playwright suites vs prod (deploy-validation, flow-core-lifecycles, telegram-webhook, telegram-full-flow) | **PORT** → §7 merge/prod pipeline; the 4 post-deploy specs **KEEP verbatim** |
| `visual.yml` · **visual** | path-filtered; PG+Redis, migrate, throwaway secrets, boot API, run visual suite inside **pinned** `mcr.microsoft.com/playwright:v1.60.0-jammy` (byte-identical rendering), upload diff artifacts on fail | **KEEP** (suite is the parity oracle); boot target swaps to Rust binary + Astro output; re-lock baselines once at cutover per surface |
| `skill-security.yml` · **scan-changed-skills** | SkillSpector `--no-llm` on changed `.agents/.claude` skill dirs, SARIF upload | **KEEP** (harness plane) |

### 3d. Budgets & audits

- **`.size-limit.json`** (2 budgets): storefront entry no-map **250 kB**; lazy map chunk **1.2 MB** —
  **KEEP** tool; new globs → Astro per-island budgets. Rebuild target: storefront island JS
  **< 50 kB** (islands ship near-zero JS for static menu), keep 250 kB as the ratchet ceiling.
- **`lighthouserc.cjs`**: staging `/s/demo` + checkout, 3 runs, perf ≥ 0.8, a11y ≥ 0.9, LCP ≤ 2500 ms,
  CLS ≤ 0.1, TBT ≤ 300 ms — **KEEP verbatim** (post-deploy job). Rebuild should beat these; ratchet
  thresholds upward after cutover (perf ≥ 0.95, LCP ≤ 1200 ms candidate).
- **`compliance:gate`**, **`storefront:guard`** — see §1.

---

## 4. Ops / DR census

| Artifact | Today | Disposition |
|---|---|---|
| `backup-restore/verify/drill/list.ts` | encrypted backup lifecycle vs R2 (BACKUP_ENCRYPTION_KEY) | **KEEP** ×4 (talk to PG+R2; later fold into a Rust ops subcommand) |
| `free-tier-watch.ts` | quota watch | **KEEP** |
| `fly.toml` | **ONE app** (`dowiz`, fra; staging = same file deployed `-a dowiz-staging` via `deploy-staging.sh`); processes `web`(512 MB)+`worker`(256 MB); `release_command = dist/migrate/index.cjs`; health = `/livez` 15 s/3 s (NOT /health — 11-query check would drop WS on restart); SIGTERM/30 s | **PORT**: processes → `web = "/app/server"`, `worker = "/app/worker"` (or one binary, subcommands); `release_command` → migrate subcommand of the Rust binary (fail-safe abort preserved); `/livez` cheap-liveness invariant preserved; VM targets shrink 512→256 MB / 256→128 MB (expect ~10–30 MB RSS) |
| `Dockerfile` | 2-stage node:22-slim; esbuild bundles; runtime `npm install argon2 sharp @aws-sdk/*` (native deps) | **RETIRE** → new 3-stage: `cargo-chef` (dep-layer cache) → `rustc` musl/static → `scratch` (or distroless-static if CA certs/tz needed) with the Astro `dist/` baked in; **~15–25 MB image**, no runtime npm install, no native-module step |
| dark-launch VITE build-args (6 ARG in Dockerfile + `deploy-staging.sh`) | flags baked at vite build time, default OFF | **PORT** → Astro `PUBLIC_*` build-time flags; the "staging deploys MUST pass build-args or features bake OFF" trap carries over verbatim — document in the deploy script |
| SOPS vault | `.sops.yaml` (age, `secrets/*.enc.env`) + `secrets-env.sh` + `secrets/README.md`; recipient keys still `age1REPLACE_…` placeholder | **KEEP** (⚠ flag: recipients never seeded — operator TODO predates rebuild) |
| Seeds | `packages/db/scripts/seed.ts`, `seed-empty.ts`, `apps/api/scripts/seed-demo-from-prod.mjs` | **KEEP** (SQL against unchanged schema; later xtask) |
| Env contract | `.env.example` (16 vars) + **32 classified envs** in `compliance/env-classification.md`, fail-closed via guardrail-license | **KEEP** manifest + gate; **PORT** the env-extraction regex from `packages/config/src/index.ts` to the Rust config crate source |
| CI preflights | `ci-connection-preflight` / `ci-migration-preflight` / `ci-schema-drift` (2026-07-03 prod-saga closures) | **KEEP** ×3 (SQL-speaking; drop `_pg-loader`) |
| `docker-disk-guard.sh` | tiered layer reclaim protecting local PG disk | **KEEP** |

---

## 5. Test-harness census

| Harness | Count / extraction | Disposition |
|---|---|---|
| Stage/phase tsx harnesses | **26** files (`ls apps/api/tests/test-{stage,phase}*.ts \| wc -l`), wired via 13 npm entries + 1 mega-chain | **RETIRE** — builder-era scaffolding; their *invariants* are the port checklist for Rust integration tests, and the E2E net is the runtime oracle. The 3 security-critical phase5 suites (rls-adversarial, jwt-rotation, integrity) **PORT** as living tests (council-gated). |
| `test:unit` glob | node --test over api/worker/web/packages/tools | **PORT** → cargo test + vitest/node--test |
| Playwright configs | **3**: root (`e2e/tests`, 5 projects: mobile/tablet/desktop + webkit-smoke ×2, DEV_AUTH_SECRET header, forbidOnly, workers=1), `playwright.visual.config.ts` (3 viewport projects, pinned-renderer contract), `e2e/lifecycle-e2e/` (L0–L11 order-trace gate, 120 s timeout) | **KEEP** ×3 — the parity oracle (06 §1). Per-surface cutover = its E2E slice green against the Rust backend. |
| E2E specs | **174** (`find e2e -name '*.spec.ts' -not -path '*node_modules*' \| wc -l`): 149 in `e2e/tests` root + 10 client + 6 admin + 3 courier + 2 a11y + 3 visual + 1 lifecycle | **KEEP** all; prune duplicates (e.g. `storefront.smoke` vs `storefront-smoke`) *after* cutover, never before |
| Visual net | 27 `toHaveScreenshot` assertions × 3 viewports (× lang loop) ≈ the "~180" net; **0 baseline PNGs in tree** — determinism contract is the pinned `playwright:v1.60.0-jammy` image in `visual.yml` | **KEEP**; re-lock baselines per surface at cutover (Astro rendering ≠ React pixel-identical — expect one deliberate re-baseline per surface, reviewed) |
| metric-core (`check:core`) | 7 checks in `checks.config.mjs`: tsc, lint, check-money, check-rls, check-contracts(placeholder), playwright-smoke, verify-env | **KEEP** — re-point cmds (tsc→cargo check, contracts placeholder→openapi-diff) |
| DeepEval / ccc | `verify:ccc-secrets`, ccc CLI | **KEEP** (harness) |
| Loops registry | **20** yaml cards (5 CERTIFIED: error-fix-convergence, design-convergence, autoupgrade, mobile-polish, acquisition-bulk-provision, demo-builder — 6 actually marked CERTIFIED), `loops/runs/` telemetry, `loops-registry-sync --check` in verify:all | **KEEP** — loop harness is stack-independent; loop cards referencing pnpm commands get re-pointed |
| E2E aux | `e2e/chaos/bad-luck.mjs` (chaos), `e2e/journeys/` (audit records), `e2e/geo-validate.mjs` | **KEEP** |

---

## 6. Disposition tallies

| Census | PORT | KEEP | RETIRE | Total |
|---|---|---|---|---|
| Root npm scripts | 23 | 23 | 24 | **70** |
| `scripts/` files | 14 | 49 | 15 | **78** |
| eslint-plugin-local rule intents | 26 survive (13 ESLint-FE/E2E as-is or rewritten · 10 Rust compiler/clippy/type-design · 3 absorbed-by-construction) | — | **0 dropped · 0 GAP** | **26** |
| Guardrail scripts | 5 | 6 | 0 | **11** |
| Workflows (jobs) | 3 | 2 | 0 | **5** |
| Weaker-teeth flags (CI-grep pending dylint) | — | — | — | **3** (#18-Rust, #20, #23) |

RETIREs are exclusively: Node build/packaging machinery (esbuild, pg-boss bootstrap, node-pg-migrate
wrappers, `_pg-loader`), dev-process runners, builder-era stage harnesses, and dead one-offs. **No
incident-derived intent is retired without a named absorber.**

---

## 7. New-stack CI/CD pipeline (docs-level design)

### 7.1 PR pipeline — target **< 10 min warm** (~6–9 min typical)

| # | Job (parallel lanes) | Contents | Budget |
|---|---|---|---|
| 1 | **rust-check** | `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` (deny-set = ported intents §2b) → `cargo test` (unit + integration vs PG16 service container) → **`cargo sqlx prepare --check`** (offline metadata drift = the compile-time replacement of verify:schema-queries) | 4–6 min warm |
| 2 | **supply-chain** | `cargo-deny check` (licenses/bans/advisories — absorbs guardrail-license Rust half + no-set-cookie crate ban + insecure-random crate ban); `cargo-vet` on a weekly schedule, not per-PR | 1 min |
| 3 | **fe-check** | `astro build` → `svelte-check` + eslint (ported plugin-local FE rules + red fixtures `lint:gates`) → `size-limit` island budgets → storefront-graph gate (Astro manifest) | 2–3 min |
| 4 | **contract-gate** | generate `openapi.json` from utoipa → **openapi-diff vs main** (breaking = fail) → regenerate FE client, fail on uncommitted drift | < 1 min |
| 5 | **static-gates** | `verify:all --ci` registry (KEEP/PORT guardrail scripts: definer-search-path, owner-active grep, spike-boundary, compliance:gate, i18n-parity, verify:secrets + gitleaks, env-classification, plane/harness gates) | 1–2 min |
| 6 | **fresh-provision + smoke** | scratch image build (cached) → new PG16 → migrate (release binary) → boot → `/livez` 200 → menu served → **Playwright smoke slice** (~10 specs: storefront, auth, checkout, 401-sweep) | 4–5 min |

### 7.2 Merge pipeline (push to main) — target **< 25 min**

1. All PR jobs (re-validated).
2. **Image build**: cargo-chef → musl static → scratch + Astro dist (**~15–25 MB**); push to registry.
3. **Migration preflight** (ci-migration-preflight equivalent vs staging schema snapshot) →
   `flyctl deploy -a dowiz-staging` (release_command = migrate subcommand; abort = old code serves).
4. **Full Playwright E2E vs staging** (174-spec net, desktop project + webkit-smoke; visual suite when
   render paths changed) + lhci scheduled post-deploy.
5. **Prod gate**: GitHub *environment approval* (manual, preserves "prod only on explicit approval") →
   `flyctl deploy` prod → the 4 post-deploy suites KEEP verbatim (deploy-validation,
   flow-core-lifecycles, telegram-webhook, telegram-full-flow).

### 7.3 Build-cache strategy

- **Rust**: `Swatinem/rust-cache` (or sccache + GHA cache backend) keyed on `Cargo.lock` + toolchain;
  `cargo-chef` recipe layer in Docker so dep compilation caches across image builds. Cold ~15–20 min,
  warm ~3–5 min compile.
- **FE**: pnpm store cache + Astro/Vite cache; Playwright browsers cached by version key.
- **sqlx**: committed `.sqlx/` offline metadata — no DB needed for the check job itself.
- CI concurrency groups cancel superseded PR runs (as `visual.yml` does today).

### 7.4 Ratchet wiring (from Phase A, per 06 §3)

clippy deny-set + cargo-deny config + sqlx offline check + the ported guardrail scripts are wired in
the **first spike PR** — the ratchet culture ports before the code does. Every 🔴 surface port
(auth/JWT, money/orders, RLS, WS authz, migrations) additionally requires its Triadic Council +
red→green proof before merge (unchanged).
