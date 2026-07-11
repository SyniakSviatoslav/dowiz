# Full Project Audit — dowiz (DeliveryOS) + bebop-repo — 2026-07-11

> Read-only audit of `/root/dowiz` and `/root/bebop-repo`, cross-checked against the canonical
> living-memory corpus (`/root/.claude/projects/-root-dowiz/memory/`, 152 files), git history in
> both repos, the working trees as found (nothing stashed/committed/reset), live prod/staging HTTP
> probes, and direct execution of the repo's own gates where cheap (meta-controller test suite).
> Every claim below is labeled **VERIFIED** (checked against code/git/live endpoint),
> **CLAIMED-UNVERIFIED** (asserted in docs/memory, not independently checkable here), or
> **CONTRADICTED** (docs/memory say X, code/git says not-X).

---

## 1. Executive Summary

**Overall health: technically strong, strategically scattered.** The dowiz codebase is one of the
most heavily instrumented small codebases imaginable — deterministic CI gates that actually run
(`pnpm verify:all --ci` wired at `.github/workflows/ci.yml:41`, 25+ steps, the L5 meta-controller
immutable-core proof runs green today, re-verified live in this audit: 9/9 pass), 162 forward-only
migrations, FORCE-RLS, integer money, a 141KB regression ledger, and a living-memory corpus that is
genuinely the best source of truth in the project. Production is up and healthy
(`https://dowiz.fly.dev/livez` 200 in 0.24s, health "degraded" only on the `fallback` sub-check —
VERIFIED live 2026-07-11 16:11Z).

**But the audit's single most important correction is about what the two repos are.**

- The **Rust/Astro rewrite of dowiz** ("the rebuild", REBUILD-MAP program, S1–S10 strangler
  surfaces) does **not** live in `/root/bebop-repo`. It lives **inside `/root/dowiz`** in the
  `rebuild/` directory (69,270 lines of Rust, 1,041 `#[test]` fns in the current tree), with its
  complete 10-surface history on `origin/fix/audit-remediation` and a content-superset evolved copy
  on the current branch `feat/paleo-dinosaur-digs`. **VERIFIED** via `git branch --contains
  2ebdf513` (S10 "10/10 SURFACES COMPLETE" commit → only `origin/fix/audit-remediation`) and route
  tree diffs.
- `/root/bebop-repo` is a **different, newer project**. Its first commit is 2026-07-08 14:17:
  *"feat: Bebop — your own coding agent (AGPL-3.0, guard OS + living memory + PQ identity +
  telemetry governor)"* (**VERIFIED**, `git log --reverse`). In 104 commits over 3 days it evolved
  through a decentralized delivery-protocol research phase into `bebop2/` — a greenfield,
  zero-dependency, post-quantum cryptography core (Ed25519/ChaCha20/Poly1305/SHA-3/ML-KEM-class
  primitives hand-implemented against RFC KATs). It implements **none** of the dowiz product
  surfaces (no axum/sqlx HTTP API, no Astro frontend, no Playwright parity suite — grep-verified
  zero hits for axum/sqlx/utoipa/supabase/astro/svelte/playwright/traceability across all
  Cargo.tomls, source, and docs).
- Judged **as its own project**, bebop is substantive, not skeletal: ~25K LOC of Rust across three
  live crates, and **`cargo check --workspace` passes in 2.5s; `cargo test --workspace` passes
  384/384 (275 bebop + 19 rust-core + 90 bebop2-core), 0 failures — both executed live during this
  audit**. The bebop2 crypto work is disciplined (RFC/FIPS known-answer tests committed, a
  three-model peer-review gate enforced by a real pre-commit hook, reviewer-rejection artifacts in
  `.review/`). What it lacks is any connection to revenue, any deployment story, and any memory
  corpus.

**Rewrite progress (the real one, in dowiz/rebuild/): ~50% by the program's own definition, and
stalled since 2026-07-05.** The "build half" is genuinely complete — all 10 surfaces built dark,
council-reviewed, with the cutover front-door harness deployed dark and 9/10 surfaces proven live
on staging (state frame `docs/ops/rebuild-cutover-h_t.json`, dated 2026-07-05). The "cutover half"
— prod flips, migrations 085–089 formal placement, the ~58-route red-line strangler tail, Astro FE
parity (scaffold is 3/27 islands) — never started in prod and has had **zero commits since
2026-07-05**. Attention moved to the Sovereign Core MVP (07-05→07-07, substantially delivered on
staging), then to harness/token-economy enforcement, then out of the repo entirely into bebop
(07-08→07-11).

**Biggest wins (verified):**
1. All 10 rebuild surfaces built dark in Rust with byte-frozen money/state-machine domain logic,
   and a reversible cutover mechanism proven end-to-end on staging (flip, parity, 2.4s rollback,
   auto-degrade firing for real).
2. Sovereign Core: `kernel::decide` at `rebuild/crates/domain/src/kernel.rs:306` is real, the 0b-5
   inject-deploy-revert RED proof on staging (v265→v266) is the strongest "deployed-reality" proof
   in the project, and Phases 1.1/1.2/1.5 + 2.2/2.3 are implemented with adversarial money tests.
3. The harness is not aspirational: verify:all runs in CI, the meta-controller is wired and green,
   hooks are registered in `.claude/settings.json`, pre-commit runs 17 armaments.
4. Production Node stack is stable and deployed (v410 lineage, migrations→084/086, Tier-1 authz
   hardening and deliver-v2 live since 2026-07-03).

**Biggest risks (verified, ranked):**
1. **Three GDPR/security fixes are still not in production** — `5ded9f19` (erasure leaves the
   delivery photo in R2 — an ETHICAL-STOP class finding), `58caf4f4` (GDPR Art.17 erasure fan-out),
   `d6b3473e` (Telegram webhook fail-closed) are **NOT ancestors of `origin/main`** and the code is
   absent from prod's tree (**VERIFIED**: `git show origin/main:apps/api/src/lib/anonymizer/index.ts`
   has zero `delivery_photo_key` references). Prod erasures today still leave doorway photos
   publicly addressable by key.
2. **Serial-pivot pattern / program fragmentation.** Four competing "futures" exist with no single
   arbiter doc: (a) the S1–S10 rebuild cutover, (b) the Sovereign Core MVP + EXPANSION-PLAN,
   (c) the open-source ADR-020 flip, (d) bebop (agent/protocol/crypto). Each was declared the
   program while active; none is closed; the newest one (bebop) has consumed all attention since
   07-08 while carrying zero shared memory.
3. **Git history bifurcation.** The 2026-07-05 secrets scrub rewrote local history; post-scrub
   branches (`feat/sovereign-core-phase-zero`, `feat/paleo-dinosaur-digs`) no longer share hashes
   with `main`/`origin/main` (paleo is "565 behind / 104 ahead" of local main purely by hash). A
   straight merge to main produced 500+ add/add conflicts (HANDOFF-2026-07-07). The remote still
   holds the old secret-bearing history on 26 branches (creds rotated dead, but the ADR-020 gate
   force-push is open).
4. **The rewrite's parity oracle and keep-set are rotting silently.** Staging has since been
   redeployed repeatedly from sovereign-core-lineage branches; whether the 9/10 Rust cutover flags
   still reflect reality on staging today is unknown (the h_t frame is 6 days old and pinned to a
   branch head that is no longer anyone's working lineage).
5. **Bus factor 1, memory gap on the new repo.** 27 Claude sessions have run in `/root/bebop-repo`
   (`/root/.claude/projects/-root-bebop-repo/*.jsonl`) with **0 memory files** — the discipline
   that makes dowiz auditable simply does not exist for the project that currently gets all the
   attention. Cross-repo detritus recurred within 24h of being "fixed" (9 new crypto-research files
   untracked in `/root/dowiz` dated 07-10/07-11, after todo-map item T1 was closed on 07-10).
6. **Known false-green in CI**: `pnpm verify:secrets` silently skips — gitleaks is not installed
   (**VERIFIED**: `which gitleaks` empty; `scripts/verify-secrets.ts:22` prints "gitleaks not
   installed, skipping"). Known since 07-08, still unfixed, flagged in the repo's own skill file as
   "highest value / zero risk" — a one-line fix nobody has landed.
7. **Live product bug, known since 07-04, still shipping:** the checkout Communication selector
   offers 6 messenger kinds but `packages/shared-types/src/legacy.ts:48` accepts only
   `telegram|whatsapp|viber` → Phone/Signal/SimpleX users get 422 at order create (**VERIFIED**
   still in the current tree). Fix drafted in an unmerged worktree, operator-gated.

**Done/planned/unfinished counts (this audit's ledger):** ~40 verified-done items, ~35
planned-not-built items, ~30 unfinished/gated items — detailed in §§4–6.

---

## 2. Ground truth: what the two repos actually are

### 2.1 /root/dowiz — the live product + its own rewrite + the harness

A pnpm monorepo (TS ~109K LOC app code + 46K LOC tests + 69K LOC Rust in `rebuild/`), 2,073
commits across all refs, on branch `feat/paleo-dinosaur-digs` with uncommitted work (see §6.1).
Four layers coexist:

| Layer | Where | State |
|---|---|---|
| **Prod Node/React product** | `apps/api`, `apps/web`, `packages/*` | LIVE at dowiz.fly.dev, deployed from `origin/main` (tip `c8b2d5a0`, 2026-07-03) |
| **Rust rebuild (S1–S10)** | `rebuild/crates/{domain,api}` + `apps/api/src/lib/cutover/` | Built dark; 9/10 staged on staging (07-05); **0% cut over in prod**; stalled |
| **Sovereign Core MVP** | `rebuild/crates/domain/src/kernel*`, `rebuild/crates/api/src/{routes/orders/checkout.rs,modules/customer_management}` | 0b-1..0b-5 + 1.1/1.2/1.5 + 2.2/2.3 done on staging (v266); prod merge deferred |
| **Harness / operating system** | `scripts/`, `.claude/`, `tools/vsa`, `tools/bebop`, `docs/operating-model/` | Live and enforced (see §4.4) |

**Naming hazard — "bebop" means four different things** (a real audit finding, it has already
caused cross-repo file misplacement twice):
1. `[data-skin="bebop"]` — the Warm Cosmo-Noir **brand skin** + landing page in dowiz
   (`docs/design/dowiz-brand/BRAND-BIBLE.md`).
2. `tools/bebop/` in dowiz — the **L5 telemetry governor** for agent dispatch (commit `28cf82eb`,
   "PID+ICIR+resonance+Landauer", touches `tools/bebop/src/governor.ts`, not the product API).
3. `rebuild/crates/bebop` in dowiz — a Rust crate in the rebuild workspace (noted PRE-EXISTINGLY
   broken in the operating-system skill: missing `pricing::PriceInputs` export).
4. `/root/bebop-repo` — the separate repository (below).

### 2.2 /root/bebop-repo — a 3-day-old separate project, not the dowiz rewrite

**VERIFIED timeline** (`git log --reverse` / `git log`):
- 2026-07-08 14:17 — first commit: "Bebop — your own coding agent (AGPL-3.0, guard OS + living
  memory + PQ identity + telemetry governor)". Full OSS scaffolding same day (GOVERNANCE.md,
  SECURITY.md, CITATION.cff, DCO.md, MCP server, in-repo wiki, "maintainer note (blocked from
  money platforms)").
- 2026-07-08→09 — physics/control-theory layer: wavefield connection-graph, damped graph-wave
  change-impact gate (flag-OFF), Kalman/limit-cycle loop health, k-d+BFS+A*/CH hybrid dispatch
  engine, "decoupled-matcher protocol" — i.e., a **decentralized delivery-protocol research
  direction** (matcher/sequencer decentralization map, PoD, oracle, SDK, identity gaps).
- 2026-07-10 — operator pivot: "First-Principles + Physicality-as-Truth global rules"; then
  `669bdea` "greenfield zero-dep post-quantum core — skeleton + C8 fix + architecture" = **bebop2**.
- 2026-07-10→11 — hand-rolled crypto primitives against spec KATs: SHA-512/SHA3 (FIPS 180-4/202),
  ChaCha20 CSPRNG + HChaCha20 (RFC 8439), Poly1305 carry fixes (RFC 8439 §2.5.1), Ed25519 RFC 8032
  §7.1 KAT green, pq_kem schoolbook rewrite. Last commit `8012b57` 2026-07-11 14:17 — ~2h before
  this audit.
- Working tree: uncommitted edits to `bebop2/core/src/{kdf,pq_dsa,pq_kem,sign}.rs` + `AGENTS.md`,
  plus 3 untracked Fable-research docs dated 2026-07-11; branch `feat/wire-native-core` is 5
  commits ahead of origin.

**What is actually in the repo (VERIFIED by the survey):**

| Area | Content | LOC / tests |
|---|---|---|
| `crates/bebop/` | The live host CLI ("Bebop — your own coding agent"): guard kernel driver, VSA living memory, PQ node identity/vault (ML-KEM-768⊕X25519, ML-DSA-65⊕Ed25519, Argon2id, XChaCha20-Poly1305 via host crates), router/copilot/multipilot, graph-PDE field planner, TUI, MCP server — plus the **delivery-protocol primitives** `matcher.rs` (pure deterministic `match_orders()`, fail-closed), `pod.rs` (proof-of-delivery attribution), `reputation.rs`, `ledger.rs` (Σbalance==0), `zkvm.rs`, `zenoh.rs` | ~16,080 LOC; 275 tests |
| `rust-core/` | `bebop-core` — deterministic graph-PDE/VSA field core, zero-dep, compiled to `bebop_core.wasm` (raw C-ABI, committed artifact) | 1,043 LOC; 19 tests |
| `bebop2/core/` | The greenfield **zero-dependency, no_std+alloc, post-quantum rewrite** ("NOT a refactor — a parallel implementation that at the end simply REPLACES the old one; old = oracle"): from-scratch SHA-512/SHA3, ChaCha20 CSPRNG, XChaCha20-Poly1305, Argon2id (RFC 9106, in working tree), Ed25519 (RFC 8032, KAT green), ML-KEM-768 (FIPS 203), ML-DSA-65 (FIPS 204, in working tree), FFT/Chebyshev/Kalman/Lyapunov spectral math, VSA circular convolution, committed KAT vectors | 6,685 LOC; 90 tests |
| `archive/` | The retired TypeScript implementation (bebop was originally extracted from the dowiz monorepo as a TS dev-tool) | not built |
| `docs/design/delivery-protocol/` | Decentralized delivery **protocol** design (DECOUPLED-MATCHER, MATCHER-API "kills DANGER #1", PROTOCOL-CENTRALIZATION-MAP, SYSTEM-ARCHITECTURE-AUDIT — thesis: "trust is the binding constraint"; explicitly protocol-not-platform) + `fable-protocol-2026-07-11/` 10-angle pivot review | docs only |
| `crates/core-legacy/` | Deprecated old guard kernel, excluded from the workspace | excluded |
| `delivery/`, `examples/` | Nearly/completely **empty** | — |

Governance in bebop-repo is real but different from dowiz's: `AGENTS.md` mandates a
**three-model peer review** ("threelaterition", builder ≠ reviewer ≠ overlap) enforced by a git
pre-commit hook, plus Verified-by-Math with RFC-anchored KATs; `.claude/settings.json` is a
read-only permission profile (denies Edit/Write/commit/push to agent sessions). Notably this
retains *proxy-model review* — the exact layer dowiz purged on 07-07 under "ground truth over
proxy" — an unreconciled philosophical fork between the two repos' operating rules.

**Relationship to dowiz:** the design bridge is `dowiz/docs/design/dowiz-agent-cli/CORE.md`
("Bebop Core = the agentic-CLI / independent-node realization of the Grand Plan's seams — where
the Grand Plan says dowiz-core, read Bebop kernel"). The living-memory skill calls bebop-repo "the
Rust/WASM sovereign core", which is **imprecise**: the sovereign core (`kernel::decide`) lives in
`dowiz/rebuild/crates/domain`; bebop-repo hosts the agent-CLI/protocol/crypto exploration. The
audit-task framing of bebop-repo as "the in-flight complete rewrite of dowiz (axum/sqlx +
Astro/Svelte, same Supabase schema, 174-spec parity oracle)" is **CONTRADICTED by both repos'
ground truth** — that program lives in `/root/dowiz/rebuild` + `apps/api/src/lib/cutover`, and
bebop-repo contains no axum, no sqlx, no utoipa, no Astro, no Playwright parity suite.
**VERIFIED by exhaustive grep**: `axum|sqlx|utoipa|supabase|fastify` = 0 hits in every Cargo.toml
and all Rust source; `.astro`/`.svelte` files = 0; `playwright.config`/`*.spec.ts` = 0;
`174-spec|parity oracle|traceability.csv|strangler|cutover` = 0 hits across `docs/**/*.md`. The
only `orders`/`couriers` references are domain examples inside the protocol matcher
(`crates/bebop/src/matcher.rs`, `enrich.rs`). The only dowiz references: the author email, the
"richer §0·GP retriever lives in the dowiz monorepo (optional add-on)" note, and archived notes
describing bebop's extraction from dowiz as a dev tool.

### 2.3 Attention timeline (evidence: commit dates, memory file dates, docs/research dates)

| Window | Focus | Evidence |
|---|---|---|
| 06-14 → 07-02 | Product build-out, MVP hardening, security sweeps, councils | ATTIC index (~85 closed arcs), prod v399→v405 |
| 07-02 → 07-03 | Merge-to-main saga → **prod deploy of 275-commit integration** (v405/v410); 6-lane audit (222 findings); secrets incident found | `merge-to-main-plan-2026-07-02.md`, origin/main `c8b2d5a0` |
| 07-04 → 07-05 | **Rebuild decision + all 10 Rust surfaces built dark + staging cutover 9/10** + token-economy arc + history scrub | `rebuild-decision-rust-astro-2026-07-04.md`, h_t frame |
| 07-05 → 07-07 | Sovereign Core MVP (0b-1..0b-5, 1.1/1.2/1.5, 2.2/2.3), council purge, VbM rule, model-routing churn v3→v3.4, brand/landing | PROGRESS.md, HANDOFF-2026-07-07, commits 56f1f872 etc. |
| 07-08 | Agents mesh, headroom proxy, **bebop-repo created**; last product-adjacent dowiz commit streak ends | agents-mesh memory; bebop first commit 2d8ccf7 |
| 07-09 → 07-10 | Paleo audit + DIGs 1/2/3/A in dowiz (last dowiz commits, 07-10 12:26); bebop delivery-protocol + bebop2 pivot | paleo docs; bebop log |
| 07-11 | bebop2 crypto KATs; crypto research PDFs accumulate **untracked in dowiz** | bebop log 8012b57; dowiz `git status` |

The drift is monotonic: product → rewrite → kernel → harness → new repo. Each layer is left in a
gated-but-unclosed state.

---

## 3. What the last full analysis found (2026-06-27) and what changed

`full-project-analysis-2026-06-27.md` (memory) reported: health 7.85/10, 42 alert files, bus-factor
1 on all files, orders.ts/server.ts god-files; 6/7 factors remediated (deps, dead code, security
index.ts deletion, tests restored, money invariant verified clean); orders.ts 999→836 and server.ts
1080→838 via 9 staging-validated slices; remaining: DB-owner migrations, deeper security items
(courier hash collision, CORS wildcard on POST /api/orders, pii-cipher no key rotation).

**Since then — shipped/changed (verified):**
- The decomposition work merged and went to prod in the 07-03 merge (prod v405+).
- The three staged DB-owner items evolved into the 085–089 draft set — **still drafts**
  (`docs/design/*/migration-drafts/`), applied to STAGING DB only via the shim runner; highest
  formally-placed migration is `1790000000086_order-ratings-customer-insert-policy.ts`
  (**VERIFIED**, 162 files in `packages/db/migrations/`).
- Bus-factor 1 is **unchanged** (CODEOWNERS exists as mitigation only); if anything it worsened —
  the knowledge now spans two repos and one of them has no memory corpus.
- The "deeper security (lower pri)" trio from 06-27 (courier hash same-input collision at
  `courier/auth.ts:243`, CORS wildcard on order-create, pii-cipher key rotation) — **no evidence
  any of the three was addressed**; not mentioned in any later memory/ledger entry. Likely
  forgotten (see §7.9).
- Everything else that happened (rebuild, sovereign core, bebop) post-dates that analysis and is
  covered below.

---

## 4. Actually Done / Shipped — VERIFIED, with evidence

### 4.1 In PRODUCTION (dowiz.fly.dev, deployed from origin/main @ c8b2d5a0, 2026-07-03)

| Item | Evidence |
|---|---|
| Node/Fastify/React product, 236+ routes, live and healthy | live probe 200/0.24s; `/health` all sub-checks ok except `fallback` degraded; `/s/demo` 200 |
| 275-commit integration merge (deliver-v2 cash-as-proof, event wiring, Tier-1 authz: orders IDOR fix, WS revocation, spa-proxy recheck, courier-invite predicate, rate-limit real-IP) | `merge-to-main-plan-2026-07-02.md` deploy log (v405 2026-07-03, then v410 boot-fix `5cee7611`); origin/main tip date matches |
| Migrations applied through 084 on prod (066..084 saga incl. the `dowiz_app` bare-role fix; 077 skipped-marked on prod due to staging drift — a known permanent divergence) | same memory, `docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md` |
| Credential rotation LANDED (deliveryos_api_user→dowiz_api rename; postgres pw rotated; leaked creds dead) | `secrets-exposure-incident-2026-07-03.md` §REMEDIATED |
| 12 shadow demo storefronts (Durrës venues) provisioned + claimable | `og-preview-demo-upgrades-2026-07-06.md`; demo list hard-coded in `scripts/report-demos-to-telegram.mjs` |

### 4.2 On STAGING only (dowiz-staging.fly.dev — verified live 200)

| Item | Evidence |
|---|---|
| **Rust rebuild S1–S10 built dark** — per-surface commits all exist: S1 `22842dbf`, S2 `b28b1764`, S3 `8a787feb`, S4 `cca5606e`, S5 `0fa028cc`, S6 `e13e1434`, S7 `4703558d`, S8 `b9a0c0d9`, S9 `b795e7d1`, S10 `2ebdf513` ("10/10 SURFACES COMPLETE") | **VERIFIED** `git cat-file` all 10 + messages; contained only in `origin/fix/audit-remediation`; content present in current paleo tree (route superset check) |
| **Cutover front-door harness** in the Node app: `apps/api/src/lib/cutover/{matcher,flags,front-door,route-templates.generated,node-keep.generated,ws-router}.ts` | **VERIFIED** all present in current tree; commit `a9a80584`; inert unless `CUTOVER_RUST_UPSTREAM` set (URL-typed env, `packages/config/src/index.ts:63`) |
| **9/10 surfaces live on Rust on staging (2026-07-05)** incl. full lifecycle L0–L7+L11 green, S5 money create 201 with correct integer money, S9 GDPR erasure end-to-end via Rust, live rollback 2,384ms, auto-degrade fired for real | `docs/ops/rebuild-cutover-h_t.json` (in tree) + `docs/ops/reliability-gate-cutover-2026-07-05.md`; S2+S8 deliberately held on Node (mixed-account login parity gap; webhook secret-parity gap) |
| **Sovereign Core 0b-1..0b-3**: money boundary, event vocabulary/Envelope, corridors behind single `decide` door | commits `c10814ab`/`e3e30ac1`/`31520e8a`; `kernel::decide` **VERIFIED at `rebuild/crates/domain/src/kernel.rs:306`**; validate layer at `kernel/validate.rs` |
| **0b-5 deployed-reality RED proof** — inject CorridorBreach → deploy staging v265 → observe refusal → revert → v266 clean | `red-proof-0b5-completed-2026-07-07.md`; commit `69293616` **VERIFIED exists** |
| **Phase 1.1 (sales_channels registry), 1.2 (order_events dual-write + replay-parity), 1.5 (attribution dashboard)** | commits `9a113ce8`, `3649cb84`, `888f6202` in paleo history (**VERIFIED** in `git log`) |
| **Phase 2.2 direct checkout via kernel::decide** (`rebuild/crates/api/src/routes/orders/checkout.rs`, server-priced, idempotent, 4 adversarial money tests) + **2.3 customer ownership** (NOBYPASSRLS membership assert, erasure oracle, `modules/customer_management/`) | commits `56f1f872`/`162ef1ec`/`03a7031a`; files **VERIFIED present** in current tree; migration `1780350000002_customers-consent-flags.ts` |
| **Reliability gate L0–L11 PASS on staging** + 2 real bugs fixed (courier READY push channel `orderStatusService.ts:300`; dashboard today-scoped counts) | HANDOFF-2026-07-07-SESSION.md; commits `c020f509`/`0ba681cf` in paleo history |
| **Rich per-venue OG unfurl for the 12 demos** (server-side sharp SVG→PNG card `/og/:slug.png`; Node + Rust storefront tags rewired) | `6a89d6e8` on sovereign-core branch; memory og-preview §Status |
| **Warm Cosmo-Noir brand: `[data-skin="bebop"]` tokens + cinematic landing at `/`** (HorizonDrift, city-pop easter egg) | commits `330ff4ed`/`9b2c...` lineage in paleo log; `docs/design/dowiz-brand/HANDOFF.md` |
| **Paleo DIGs 1/2/3/A** — registry.ts fail-closed `assertVocabulary()`, message-bus gradual `degradeLevel` 0..1, config-drift boot assertion, courier-sweep content-addressed idempotency (flag-OFF dark) | commits `6668618e` + `f869fccb` (**VERIFIED**, tip of paleo) |

### 4.3 GDPR/security fixes — built + staging-validated but **NOT in prod** (CONTRADICTS "shipped" language in places)

`5ded9f19` (delivery_photo_key R2 purge on erasure, ETHICAL-STOP lift), `58caf4f4` (erasure
fan-out to orders/GPS/ratings/subject_phone), `d6b3473e` (Telegram webhook fail-closed):
**VERIFIED all three commits exist AND all three are NOT ancestors of origin/main**; prod's
anonymizer code has zero `delivery_photo_key` handling. Memory correctly tags them
"prod-merge operator-gated" — but any doc reading "shipped" without that qualifier overstates.
This is the most consequential done-but-not-done item in the project.

### 4.4 Harness / operating system (in-repo, enforced — this is real, not aspirational)

| Claim | Verdict |
|---|---|
| L5 meta-controller wired into verify-all | **VERIFIED** — `scripts/verify-all.ts:59` runs `node --test scripts/meta-controller.test.mjs` (ci:true), `:64` runs the sandbox-staleness guard it proposed; `pnpm verify:all --ci` runs in CI (`.github/workflows/ci.yml:41`); this audit ran the suite live: **9/9 pass** |
| Meta-controller never auto-applies; immutable core refuses Charter/gate/hooks/AGENTS.md/itself | **VERIFIED by its own test suite** (discriminating predicate proven red→green per META-CONTROLLER.md §probe); no `apply` command in `scripts/meta-controller.mjs` |
| 9 hooks registered (protect-paths, red-line-doubt-gate, guard-bash, agent-dispatch-gate, post-edit-gates, distill-nudge, subagent-return-guard, context-budget-guard @30% of 1M, require-classification) | **VERIFIED** in `.claude/settings.json` hooks block |
| Council/serious-gate REMOVED (operator f1255ad5, "ground truth over proxy") | **VERIFIED** — `.claude/hooks/serious-gate.sh` absent + unregistered; survives only in the two stale worktrees. The uncommitted `scripts/plane-guard.mjs` diff converts P7 from "serious-gate exists" to "serious-gate cleanly removed" (see §6.1) |
| Dynamic pre-commit (heavy steps only on build-relevant staged paths) + 17 armaments | **VERIFIED** `.husky/pre-commit` (1.4–1.4g steps + BUILD_RELEVANT gate) |
| Token stack: `tools/vsa` (codec/route/orchestrate/viz), headroom compression proxy (systemd, ~1,619 tok/req measured), codebase-memory graph, model routing v3.4 (Haiku lead pin **staged not applied**), agent-dispatch DENY on model-less + fable | **VERIFIED** tools/vsa exists with bench docs; routing enforcement hooks live; the Haiku settings pin is in `docs/operating-model/proposed-settings/` awaiting operator `cp` |
| Cross-agent mesh (hermes→opencode→goose→aider→openhands) with fail-through + no-auto-approve posture | **VERIFIED** `scripts/agents-mesh.sh` exists; RED/GREEN dry-run proofs recorded in memory; inert until operator adds API keys |
| Living memory: 152-file corpus + MEMORY.md index + ATTIC + `sync-memory-to-hermes.mjs` mirror | **VERIFIED**; last corpus update 2026-07-10 12:24 |

### 4.5 bebop-repo / bebop2 — done (verified)

| Item | Evidence |
|---|---|
| Whole workspace compiles + tests green: `cargo check --workspace` PASS (2.51s, 16 dead-code warnings in bebop, 0 errors); **`cargo test --workspace` 384/384 pass, 0 fail** (275 bebop / 19 rust-core / 90 bebop2-core; bebop2 KATs are compute-heavy — 448s debug, 32.8s release) | **executed live in this audit** |
| Bebop host CLI: guard OS over `bebop_core.wasm` (deny auth/money/secrets/migrations without human approval), VSA memory with forget mechanism, PQ hybrid identity vault, router/copilot/multipilot, field-as-cost-surface planner, TUI, MCP server, telegram notify scripts, PTY session recordings (`docs/footage/`) | crate modules + 275 tests; README + CHANGELOG 0.4.0 |
| Full OSS scaffolding day one: AGPL-3.0, GOVERNANCE/SECURITY/SUPPORT/CITATION/DCO, CI, wiki, llms.txt | commits of 2026-07-08 |
| Delivery-protocol primitives as tested library code: deterministic fail-closed `match_orders()`, PoD attribution, reputation, conservation-checked ledger, zkvm commit/verify boundary | `crates/bebop/src/{matcher,pod,reputation,ledger,zkvm}.rs` |
| bebop2 zero-dep crypto core with committed FIPS/RFC KAT vectors: SHA-512/SHA3 (FIPS 180-4/202), ChaCha20 CSPRNG + HChaCha20 (RFC 8439), XChaCha20-Poly1305 (incl. Poly1305 per-block hibit fix isolated against a Python oracle), Ed25519 RFC 8032 §7.1 KAT green with 3 RFC-deviation closures (reject S≥L, reject non-canonical y, wrong-pubkey RED KAT), ML-KEM-768 schoolbook | commits `cc265f8`/`0de78a1`/`cccec00`/`5f988a6`/`8012b57`; `bebop2/core/src/*` + `kat/` |
| Three-model review gate operating for real: overlap reviewer REJECTED the Ed25519 signature work for malleability deviations, builder fixed, final APPROVE recorded | `.review/{reviewer,overlap}-findings-sign.md` |
| Zero-dependency claim holds: `bebop2/core/Cargo.toml` has no deps/dev-deps; `cargo check -p bebop2-core` 0.49s | verified |

**Honest caveat on "done":** all of this is library/CLI code on a feature branch, 5 commits
unpushed, with no deployment, no users, and no binary release. "Done" here means
compiles-and-KAT-green, which for hand-rolled cryptography is necessary but far from sufficient
(see §7.10).

---

## 5. Planned / Roadmapped (not yet built)

### 5.1 Rebuild program (REBUILD-MAP.md phase plan) — the entire "cutover half"
- **Phase B completion in prod:** per-surface prod flips (S5-money + S9-GDPR = explicit operator
  confirm), migrations 085–089 formal placement into `packages/db/migrations/`, S2 §3 re-ratify
  signature, S8 webhook secret-parity council, Phase-D decommission owner+date (REV-C10 HARD GATE —
  still blank).
- **Strangler tail:** ~58 red-line keep-routes still on Node (money settlements/refunds/order-
  actions, FORCE-RLS S8 notifications/push/signals, provisioning/activation, OTP) + ~8 clean +
  5 council-deferred (`h_t.json` §strangler_tail; `node-keep.generated.ts`).
- **Astro/Svelte FE everywhere** (operator directive 07-05: React interim-only): storefront
  UX-parity matrix (77 features, `docs/design/rebuild-plan/astro-parity-matrix.md`) then
  admin+courier matrices. Current Astro app = Phase-A scaffold, 3/27 islands, no
  checkout/images/tracking; JS floor 14.3kB gz vs the 8kB budget — an **unresolved operator
  decision** (revise budget vs vanilla-JS islands, `rebuild/web/ISLAND-BUDGET-OPTIONS.md`).
- **Phase C** channel-hub heads (feeds/JSON-LD, MCP/UCP stub, conversational heads) and **Phase D**
  decommission + 48h soak + full 174-spec run — not started.

### 5.2 EXPANSION-PLAN (the declared current north star, `docs/design/dowiz-brand/EXPANSION-PLAN.md`)
- **Layer 0 (release gates, all open):** 0.1 secrets history force-push (operator, BLOCKING),
  0.2 license flip Apache-2.0→AGPLv3 + TRADEMARK.md + DCO, 0.3 README/SECURITY truth-pass,
  0.4 design-system unification (Paper deletion, bebop-skin canonical), 0.5 security scanners in CI
  (semgrep/trivy/gitleaks/cargo-deny), 0.6 RUSTSEC rsa/num-bigint remediation.
- **Layer 1:** Better Auth + Telegram login (red-line, council-gated); entry-point doors in order
  QR → Telegram Mini App → WhatsApp **Cloud API** (open-wa explicitly banned for prod); telemetry
  alerter → Telegram. Brand Pass-2 remainder (flip `paperSkinAttr()`, migrate admin/courier).
- **Layer 2:** voice UI (local), live translations, observability, smart devices.

### 5.3 Sovereign Core MVP — remaining exit-gate steps
1.3 transport-agnostic sync port, 1.4 signed-event envelope, 2.1 multi-channel distribution
artifacts, 2.4 aggregator read-only stub, 0b-6 CI sovereign gate as a required check
(staged in `proposed-sovereign-core-ci/`), plus the full staging validation list from
HANDOFF-2026-07-07 (owner data-hub flow, customer tracking e2e, courier flow, full Playwright vs
staging) and finally the main merge + prod.

### 5.4 Open-source program (ADR-020)
AGPLv3 + trademark filing (EUTM, "DeliveryOS is a weak descriptive mark" — brand decision needed),
DCO, pricing-v2 landing, split private financial docs, Sponsors button. Hard-gated on secrets
history scrub + EUTM + explicit operator go.

### 5.5 Demo/outreach arc (PLAN.md WS3/WS4)
QR per demo, operator-identity footer, own domain (porosite.al), Fly cold-start warm; video montage
pipeline (Revideo/FFmpeg + Whisper subtitles + R2 mp4) — researched, not built.

### 5.6 Paleo digs — remaining dinosaurs
Registry.ts wave-native decomposition (the "primary dig", deliberately deferred), courier-sweep
pass parallelization (soft), and ratification of the flag-OFF grace-window auto-cancel
(R-NEEDS-HUMAN-1) — audit doc explicitly scoped these out of the 07-10 fixes.

### 5.7 bebop-repo roadmap (from its own docs — none built yet)

- **bebop2 completion + swap**: only `bebop2/core/` exists; the README's planned `kernel/`, `cli/`,
  and `reloop/` directories are absent. The stated end-state — bebop2 *replaces* `crates/bebop`
  with the old implementation as an equivalence oracle — has no equivalence-test harness yet.
- **ARCHITECTURE pillars** (operator-directed 07-10): vectors→waves (spectral coefficients over
  dense buffers), kill middleware on the hot path (no wasm-bindgen/serde on `decide`/`fold`/
  `replay`), AGC-class envelope (2.048MHz/2K RAM framing), better math per function
  (Lanczos/Krylov/Chebyshev/sqrt-Kalman) — partially realized in the spectral modules, not
  system-wide.
- **Decentralized delivery protocol**: dispute arbitration/jury (F2), hidden-centralization
  removal (F3), StoryBrand UX + 50%-courier-drop stress test (F4), pseudonymous PoD via
  per-round keys, reputation ledger — design docs + primitives only; no network layer, no nodes,
  no deployment; `delivery/` dir is empty.
- **Five-tool integration backlog** (the untracked reports sitting in the WRONG repo,
  `dowiz/docs/design/five-tool-integration-report.md` + `integration-research-report.md`):
  Video-use/Torlink/Cochlea INTEGRATE verdicts, Sentinel Pro REFUSED (surveillance line),
  Shattermind DEFERRED (unverifiable source) — RED+GREEN specs written, nothing implemented.

---

## 6. Unfinished / In-Progress / Partial / Gated-off

### 6.1 Live working-tree state (both repos, as found — nothing touched)

**dowiz (`feat/paleo-dinosaur-digs`):**
- Modified, uncommitted: `scripts/plane-guard.mjs` (+28/−1: P7 rewritten from "serious-gate must
  exist" to "serious-gate must be *cleanly removed* — red if resurrected-unwired or dangling"),
  `scripts/verify-all.ts` (comment/name updated for the same removal), `apps/web/src/lib/
  reactAction.test.ts` (non-null assertions for `noUncheckedIndexedAccess` strictness). These are
  coherent, finished-looking changes that **de-red the gates after the council purge — but they are
  uncommitted**, so CI on any other checkout of this branch presumably still fails P7. This is
  precisely GH issue **#9** territory ("verify:all / plane-guard reference 9 guardrail scripts
  missing from main — 3 real hard fails"). Land them.
- Untracked (9+ files): `chacha.pdf`, `poly1305.pdf`, `xsalsa.pdf`, `xchacha2.pdf`,
  `xchacha_draft.html`, `crypto-primitives-research.md`, `hybrid-routing-sota.md`,
  `platform-vs-protocol-logistics.md`, `web3-logistics-postmortem.md`, plus
  `docs/design/five-tool-integration-report.md` and `docs/design/integration-research-report.md` —
  **all bebop-repo research written into the dowiz tree**, i.e., the exact cross-repo-detritus
  class the 07-10 todo map declared fixed (T1 "DONE"). It recurred within a day. The standing rule
  ("files referencing bebop belong in /root/bebop-repo") is prose, not a hook — it does not hold.

**bebop-repo (`feat/wire-native-core`, 5 commits unpushed):** uncommitted edits to
`bebop2/core/src/{kdf,pq_dsa,pq_kem,sign}.rs` + `AGENTS.md`; 3 untracked research docs dated
2026-07-11. The uncommitted diff is large and load-bearing: `kdf.rs` +616 (a complete from-scratch
**Argon2id RFC 9106** with in-tree BLAKE2b, replacing a 2-line stub), `pq_dsa.rs` +698 (a complete
**ML-DSA-65 FIPS 204** replacing a stub), `pq_kem.rs` (exposes the Keccak XOF), `sign.rs` (doc
tweaks), `AGENTS.md` (+10, the new §0 "multipilot-native" workflow). In other words: **two whole
post-quantum crypto implementations currently exist only in an uncommitted working tree** on an
unpushed branch on one machine — the single largest unprotected work-in-progress found in this
audit.

### 6.2 Feature-flagged / dark (default-off, verified in `packages/config/src/index.ts` and call sites)

| Flag / gate | Surface | State |
|---|---|---|
| `GOOGLE_OAUTH_ENABLED`, `OTP_ENABLED`, `MEDIA_RICH_ENABLED`, `BACKUP_ENABLED`, `DWELL_TIER3_ENABLED`, `DISPATCH_OWNER_GRACE_ENABLED`, `ACCESS_GATE_PUBLIC_ENABLED` | default `'false'` in EnvSchema | dark |
| `CUTOVER_RUST_UPSTREAM` (URL, unset ⇒ front-door inert), `CUTOVER_FORCE_ALL_NODE` | cutover harness | inert in prod |
| `TMA_ENABLED` (raw `process.env` read at `telegram-webhook.ts:39` — **comment claims EnvSchema but it is NOT in packages/config**, a known red-line-gated cleanup) + `VITE_TMA_ENABLED` | Telegram Mini App | dark; CSP still blocks the TG bridge script |
| `VITE_CHANNEL_KIT_ENABLED` | QR/NFC attribution kit | dark |
| Voice: `@deliveryos/voice` dep + `apps/web/src/lib/voice/*` landed, **MicFab NOT mounted anywhere** (repo-wide grep: no JSX usage) | voice FE | held on STOP-DESIGN-B (operator must pre-commit flip-ON condition + delete-if-unmounted decision) |
| `hub_checkout` (sovereign checkout) | Phase 2.2 | wired, default OFF, "verify in next session" never recorded as done |
| Courier-sweep grace auto-cancel (DIG-A adjacent) | money/ethics | flag-OFF dark, awaiting STOP-ETHICS ratification |
| Promotions/discounts | product | **POTEMKIN** (counsel's word): full owner Promotions CRM UI with `discountTotal=0` hardcoded — no redemption runtime; carried as accepted-risk with a re-scope trigger |

### 6.3 Operator-gated queue (blocked on a human, some for 7+ days)

1. Prod merge of `5ded9f19`/`58caf4f4`/`d6b3473e` (GDPR/webhook — §4.3). **Oldest, highest-harm.**
2. Migrations 085–089 formal placement (085 settlement watermark had a "2026-07-10 HARD gate" note
   — that date has now PASSED with no placement recorded; the S5 breaker later downgraded 085 to
   draft-status with a different real guard, but nobody has recorded a post-07-10 disposition).
3. Secrets REMOTE scrub force-push over 26 dirty origin branches (+ gitleaks CI gate).
4. S2 cutover re-ratification signature; Phase-D owner+date; S5/S9 prod flip go.
5. MessengerKind unification + DB CHECK migration (fixes the live 3-kind 422; drafts sit in
   unmerged worktree `agent-aee00a7da688fe62c` — worktree fate itself unverified).
6. `openapi-contracts/` → `contracts/` rename + git add (blocked by guard-bash substring rule).
7. Haiku model pin `cp` (proposed-settings), `rebuild/Dockerfile` creation, TMA Dockerfile ARG.
8. OG-preview push (rich-unfurl commits unpushed per memory), demo-tenant rename-back
   ("E2E-Test-Location-1783030883177" — an E2E run renamed the demo venue; still flagged).
9. 10 `docs/operating-model/proposed-*` directories of staged-but-unapplied harness artifacts.

### 6.4 Known-broken / needs-investigation (open, some stale)

- **Staging checkout-flow break** (add-to-cart→checkout fails at `checkout-phone` testid) — flagged
  2026-07-04 as "needs an investigate loop, NOT this change"; no later memory closes it.
- Staging E2E vs rate-limiter (100 req/min/IP false-fails the matrix by construction) — strategy
  (test-token bypass or serialized runs) never decided.
- Owner pickup proxy WS-broadcasts PICKED_UP without persisting orders.status
  (`dashboard.ts:379-429`) — escalation opened 07-05, no closure found.
- Degrade-storm ratchet (boot-grace + restart-regression-test + alert-on-degrade, task #15) —
  cutover auto-degrade silently flipped all non-money surfaces to Node on a restart; discovery was
  accidental. Open.
- Pre-commit hook >8min hang class (P1, "move Docker→CI") — the dynamic-scope fix landed for
  docs-only commits, but build-relevant commits still carry it.
- `pnpm lint:gates` broken in this environment (`@eslint/js` ERR_MODULE_NOT_FOUND, hit by 3 agents
  per PROGRESS.md BLOCKERS).
- 2 stale git worktrees from 2026-07-02 (`/root/dowiz-wt-phase0`, `-phase5`) each with 11
  untracked/modified files — 9 days stale, exactly the STALE_SANDBOX class the meta-controller
  exists to flag; they also still carry the deleted serious-gate.sh.
- GH **#19**: cloud sandbox 403 egress blocks all plane-maintainer staging deploys + Telegram
  reports (infra/policy, needs operator).

### 6.5 bebop-repo — unfinished / partial

- Argon2id + ML-DSA-65 implementations: written, KAT-anchored, **uncommitted** (§6.1).
- 5 commits unpushed on `feat/wire-native-core`; `main` also 5 ahead of `origin/main`.
- bebop2 `kernel/`, `cli/`, `reloop/`: planned in README, do not exist; no equivalence harness
  against the old-bebop oracle.
- Doc drift: `docs/ARCHITECTURE.md` and `CHANGELOG.md` still describe the retired **TypeScript
  runtime** ("433 TS tests", `src/copilot.ts` paths) — contradicts the README's "no TypeScript in
  the live path"; the TS lives in `archive/` and is not built. **CONTRADICTED-by-code (stale
  docs).**
- Test-count claims: README/AGENTS say "294 Rust tests" — actual live suite is **384** (the 294
  figure = bebop+rust-core only, excluding bebop2's 90; internally consistent but stale framing).
- Delivery protocol: primitives + design only; `delivery/` dir empty except an empty
  `telegram-pending/`; no network/deploy layer.
- 16 dead-code warnings in `crates/bebop` (`in_deg`, `recall_at`, `build_oracle`, `remove_node`…)
  — small but real rot signals in a 3-day-old codebase.
- **No living-memory corpus** (0 files) despite 27 sessions and its own README advertising "living
  memory" as a headline feature — the repo does not eat its own dog food yet.

---

## 7. Cross-cutting Findings & Risks

### 7.1 The program has no single spine (highest-order finding)
Four program-level narratives each carry an "authoritative" marker: REBUILD-MAP ("the program
spine", 07-04), GRAND-PLAN → superseded by EXPANSION-PLAN ("AUTHORITATIVE", 07-07), ADR-020 ("FINAL
goal", 07-03), and bebop2's architecture docs (07-10, operator-directed pillars). The rebuild's
cutover half, the MVP's exit gate, the OSS flip, and bebop all compete for the same single
operator/agent bandwidth. Memory tracks each arc honestly, but nothing ranks them; the todo map
(07-10) is the closest thing and it predates the bebop2 crypto pivot.

### 7.2 "Done" language inflation at the edges
The corpus itself is honest (build-dark ≠ flip is stated repeatedly), but derived docs drift:
`project-state-2026-07-08.md` says "**MVP is SHIPPING-READY**" while the same week's HANDOFF says
"MVP ~40% (5 of 12 phases)" — the skill patched this to "phases 2.2/2.3 done late that night", yet
1.3/1.4/2.1/2.4 remain open and the staging-validation checklist (owner hub flow, customer
tracking, courier flow, full Playwright) was never recorded green. "Shipping-ready" is
**CLAIMED-UNVERIFIED at best**. Similarly the As-Built v1 doc (HS256, 67 migrations) and
CONTEXT-INDEX ("START HERE") are two product-generations stale and still presented as entry points.

### 7.3 Production carries known, fixed-elsewhere harms
§4.3 (GDPR photo purge, erasure fan-out, webhook fail-closed) + the 3-kind 422 checkout bug + the
POTEMKIN promotions surface. All are known, all have fixes or drafts, none are in prod. The gating
discipline is working as designed — but the queue has no SLA, and GDPR erasure completeness is not
a feature, it's a legal obligation (the repo's own counsel called it an ETHICAL-STOP).

### 7.4 History bifurcation + dirty remote (structural debt)
Two hash-lineages coexist locally (pre-scrub main/fix-audit-remediation vs post-scrub
sovereign/paleo); local `fix/audit-remediation` and its origin twin have diverged 997/941 commits.
The eventual "merge to main = prod deploy" is now a **rewrite-aware operation** nobody has designed
(HANDOFF's "don't force-push without understanding" is correct and unanswered). Meanwhile origin
still hosts the old secret-bearing history (dead creds, but ADR-020 blocks OSS on it).

### 7.5 L5/meta-controller: wired and enforced, doc slightly stale
Enforcement verdict: **real**. `verify:all --ci` in CI runs the immutable-core proof and the
sandbox-staleness guard; hooks are registered; plane-guard runs 11 meta-patterns. Two caveats:
(a) META-CONTROLLER.md still lists `serious-gate` among the immutable authority hooks — that hook
was deliberately removed on 07-07; the doc needs the same P7-style annotation the uncommitted
plane-guard diff has. (b) The stale-worktree guard evidently tolerates the two 07-02 worktrees
(commits on 07-10 passed pre-commit) — either they're outside its glob or below its threshold;
either way the exact gap it was built for (at-risk untracked work in stale sandboxes) currently
exists ×2 and is unflagged.

### 7.6 Memory system: excellent in dowiz, absent in bebop-repo
dowiz: 152 files, indexed, ATTIC'd, mirrored to HERMES.md, with an explicit canonical-store rule —
genuinely the best-maintained artifact in the project. bebop-repo: **27 session transcripts, 0
memory files** (`/root/.claude/projects/-root-bebop-repo/memory/` empty). All bebop knowledge
lives in git messages, in-repo docs, and the operator's head. The dowiz corpus also has **no entry
newer than 07-10 12:24**, so the entire bebop2 crypto pivot (the current center of gravity) is
invisible to the memory system on both sides. The cross-repo detritus recurrence (§6.1) is the
predictable symptom.

### 7.7 Process churn as a cost center
In 5 days the governance layer went: council mandatory (serious-gate) → council optional (07-05)
→ council + critics + advisory hooks purged (07-07, f1255ad5) — while model routing went v2 (no
Fable) → v3 (Opus reasoning) → v3.1 (Fable deny) → v3.2 (deny→warn) → v3.3 (Sonnet) → v3.4 (Haiku)
in ~4 days. Each step is documented and operator-driven, and the end state (deterministic gates
only, cheap models, VbM) is coherent — but the thrash consumed sessions, left half-updated
artifacts (P7, META-CONTROLLER.md, worktree hooks), and the promised follow-ups from the fable
audit (`subagent-return-guard` exists — landed; guard-bash 83% FP fix, loop-registry parity
circuit — no evidence landed).

### 7.8 The business-validation gap (the repo's own sharpest self-criticism)
`DeliveryOS-Business-Value-Sort.md` Tier-4 warns the process apparatus risks becoming "elegant
procrastination" vs the one validating event: **a real restaurant placing a real paid order**.
Since that was written: a complete Rust rewrite was built dark, a kernel was event-sourced, a
crypto library was hand-rolled — and the audit found **no evidence of a single real
(non-demo, non-operator) production order or a claimed venue**. The 12 outreach demos remain
shadow tenants; outreach upgrades (OG cards) shipped to staging; WS3/WS4 outreach never built.
This is the largest risk in the project and it is not a code risk.

### 7.9 Security posture — solid core, specific open edges
Confirmed solid (07-02 sweep + later councils): RS256 double-pinned, parameterized SQL, FORCE-RLS
40+ tables, Zod strict, SSRF-guarded, secrets rotated. Open edges, all known, none closed:
`verify:secrets` no-op (§1 risk 6); RUSTSEC-2023-0071 rsa Marvin (no upstream fix; remediation =
swap the web-push/jwt crypto path, queued in Layer 0.6); yanked num-bigint; courier hash
same-input collision (`courier/auth.ts:243`), CORS wildcard on POST /api/orders, pii-cipher key
non-rotation — the 06-27 trio, apparently forgotten; OR-1 "is the live operational pool
BYPASSRLS?" was partially answered by the dowiz_api rotation saga but the B3 NOBYPASSRLS flip
(make dowiz_app the real operational role) is still open; anonymizer GDPR gap #1 (orders.metadata
ip-hash copy) still open (photo purge was gap #2 and is fixed-on-branch only).

### 7.10 bebop-specific risks

- **Hand-rolled cryptography.** bebop2's explicit mandate is from-scratch, zero-dependency
  implementations of ML-KEM-768, ML-DSA-65, Ed25519, Argon2id, XChaCha20-Poly1305. The discipline
  applied (RFC/FIPS KATs, RED cases, three-model review that actually rejected a malleability bug)
  is far above hobby grade — but KAT-green is not constant-time, not side-channel-audited, and not
  a substitute for the years of review the replaced crates (`ml-kem`, `ed25519-dalek`) have had.
  The repo's own ARCHITECTURE.md warns PQ schemes "must NOT be optimized into insecurity"; the same
  logic argues against shipping self-written primitives for anything money- or identity-bearing
  without an external audit. Risk is acceptable for a research core, serious if bebop identities
  ever guard real value (the delivery-protocol design puts them exactly there: PoD, reputation,
  escrow).
- **Review-philosophy fork vs dowiz** (proxy 3-model review mandatory here, purged there) —
  whichever is right, having both as "binding" rules invites rule-shopping.
- **Bus factor 1, no memory, unpushed/uncommitted crown jewels** (§6.1, §6.5).
- **Unbounded scope gravity.** In 3.5 days the repo has been, in order: a coding agent, a physics
  planner, a decentralized delivery protocol, and a post-quantum crypto library. Each pivot is
  documented and internally reasoned, but the repo now contains four ambitions and zero users; the
  same pattern that stalled the dowiz rebuild is running faster here.

---

## 8. dowiz ↔ rebuild ↔ bebop2 Parity Assessment

**dowiz Node (prod) vs Rust rebuild:**
- Routes: 236 inventoried; all 10 surfaces have Rust implementations built dark; on staging as of
  07-05, 9/10 surfaces served from Rust with a keep-set of ~61–76 routes still Node (the red-line
  tail). **Prod: 0 routes on Rust — 0% cutover.**
- Parity oracle: 179 E2E spec files exist (**VERIFIED count** — the "~174" inventory figure is
  slightly stale in the other direction); the live-PG bind/decode batch ran 833 pass / 3 fail
  (test-infra) on 07-05; lifecycle L0–L7+L11 green via Rust on staging. The oracle has not been
  re-run since 07-05 and staging has since been redeployed from a different branch lineage —
  **current staging cutover state is unknown**.
- Domain: money/order_status/tenant modules byte-frozen across all 10 surface ports (asserted per
  commit and re-verified at S-gates; **CLAIMED-UNVERIFIED here** — not re-diffed in this audit).
- FE: Astro shell 3/27 islands; humans are served the React SPA everywhere. FE parity ≈ 10%.
- Overall rebuild program: build-half ~100%, cutover-half ~15% (mechanism proven, S1 mechanics
  only), FE ~10%, decommission 0%. Call it **~50% of REBUILD-MAP, frozen for 6 days**.

**dowiz vs bebop2: no parity relationship in code.** bebop2 shares zero product surface, zero
schema, zero specs with dowiz. If the operator's intent is that bebop2 eventually *hosts* a
sovereign delivery protocol that dowiz plugs into, that bridge exists only as research documents
(`platform-vs-protocol-logistics.md`, `web3-logistics-postmortem.md`, `crypto-primitives-
research.md` — currently sitting untracked in the wrong repo) and `docs/design/dowiz-agent-cli/`.
Treating bebop-repo as "the dowiz rewrite at N%" would be a category error; **as a dowiz rewrite it
is 0%; as its own project it is a healthy, fast-moving research codebase** (384/384 tests green,
compiles clean, disciplined review) whose bebop2 rewrite-of-itself is roughly: crypto core ~85%
(2 primitives still uncommitted), kernel/CLI/reloop 0%, equivalence harness 0%, protocol layer
design-only.

**The delivery-protocol thread is the only place the two repos' futures genuinely intersect**: the
protocol docs (matcher/PoD/reputation) describe a trustless dispatch layer that a platform like
dowiz could one day plug into, and the research for it (`platform-vs-protocol-logistics.md`,
`web3-logistics-postmortem.md` — the latter cataloguing why prior Web3 logistics attempts died) is
currently sitting untracked in dowiz's working tree. If that intersection is the actual strategy,
it exists nowhere as a committed, operator-signed document.

---

## 9. Recommendations (prioritized)

1. **Ship the GDPR/webhook trio to prod** (`5ded9f19`, `58caf4f4`, `d6b3473e`). Design the
   bifurcated-history merge once (rewrite-aware: merge by tree, not by hash — e.g., a curated
   squash of the staging-validated set onto origin/main), staging-rehearse, deploy. This is the
   only item on the list with live-user legal exposure.
2. **Write the arbiter doc** (one page, operator-signed, in living memory + repo): rank rebuild-
   cutover vs MVP-exit vs OSS-flip vs bebop; give each a "next session does X" and an explicit
   PARKED marker for the rest. The corpus is excellent at recording state and silent on priority.
3. **Land the uncommitted gate fixes** (plane-guard P7 + verify-all + reactAction test) and update
   META-CONTROLLER.md's immutable-core table for the council removal — closes GH #9's class and
   stops the doc/gate drift.
4. **Install gitleaks** (kills the verify:secrets false-green — one line, pre-approved) and then
   execute the remote history force-push + branch prune (ADR-020 gate 1), which also collapses the
   26-branch dirty remote.
5. **Fix the 3-kind 422** (apply the MessengerKind unification + DB CHECK draft) — a live checkout
   failure for 3 of 6 advertised contact methods is a conversion leak in the one flow that matters.
6. **Bootstrap `/root/bebop-repo` living memory now**: create `memory/MEMORY.md` + a
   `bebop2-pivot-2026-07-10.md` capturing the coding-agent→protocol→crypto arc, and add a
   deterministic hook (the repo's own preferred tool) that blocks bebop-topic files landing in
   /root/dowiz — the prose rule has failed twice.
7. **Re-baseline or formally mothball the cutover**: one session to re-probe staging's actual
   flag/upstream state vs `rebuild-cutover-h_t.json`, then either resume the tail or write a dated
   PARKED entry (incl. disposition of the passed 085 watermark date). A half-flipped staging with
   an unknown state is the worst of both.
8. **Harvest or prune the stale worktrees** (`dowiz-wt-phase0/5`, 07-02, 11 untracked files each)
   and the unmerged channel-adapter/IG worktrees — and make the sandbox-staleness guard actually
   see them (it demonstrably doesn't).
9. **Point one week at business validation** (the Tier-1 wedge): WS3 outreach (QR + footer +
   domain), claim flow on the 12 demos, one concierge onboarding. The stack has out-run the market
   test by a month; every further infrastructure layer raises the cost of learning the answer.
10. **Close the forgotten 06-27 security trio** (courier hash collision, CORS wildcard on
    order-create, pii-cipher rotation) or explicitly accept-risk them in the ledger so they stop
    being silently carried.

---

## Appendix A — Evidence quick-index

- Prod probe: `dowiz.fly.dev/livez` 200 (0.24s), `/health` degraded only on `fallback`, `/s/demo`
  200 — 2026-07-11 16:11Z.
- Rebuild commits: S1 `22842dbf` … S10 `2ebdf513`, cutover `86049799`/`a9a80584` — all exist;
  containment: `origin/fix/audit-remediation` only.
- Branch topology: local main `2be9e692` (06-25); origin/main `c8b2d5a0` (07-03);
  `fix/audit-remediation` 433 ahead of local main, tip `a8e5844e` (07-05); local-vs-origin
  audit-remediation diverged 997/941; paleo tip `e5eb3d03` (07-10) contains sovereign-core tip,
  does NOT contain (by hash) the S-surface commits, but its `rebuild/` route tree is a strict
  superset of origin/fix/audit-remediation's (55 vs 45 files).
- Sovereign core: `rebuild/crates/domain/src/kernel.rs:306`; checkout at
  `rebuild/crates/api/src/routes/orders/checkout.rs`; NOT called from the TS `apps/api`
  (grep-verified — the kernel flip lives in the Rust shell only).
- Gates: `.github/workflows/ci.yml:41` (`pnpm verify:all --ci`); `verify-all.ts:59,64`
  (meta-controller + staleness); meta-controller test run live in this audit: 9/9 pass;
  `.claude/settings.json` 9 hooks; serious-gate absent (worktrees only).
- Migrations: `packages/db/migrations/` = 162 files, highest `1790000000086_…`; 085–089 exist only
  under `docs/design/*/migration-drafts/`.
- E2E: 179 spec files (e2e paths), 208 `*.test.ts(x)` unit/integration files; Rust: 1,041 `#[test]`
  fns in current tree.
- TODO debt: 7 TODO/FIXME/HACK hits total across apps+packages (remarkably low).
- Telegram send pattern (used for this audit's summary): `report-demos-to-telegram.mjs` —
  `TELEGRAM_BOT_TOKEN` from `.env`, POST sendMessage, `chat_id -1003901655568`,
  `message_thread_id 13`; sendMessage only (getUpdates/deleteWebhook forbidden per standing
  memory).
- bebop-repo: first commit `2d8ccf7` 2026-07-08 14:17; 104 commits; last `8012b57` 2026-07-11
  14:17; branch `feat/wire-native-core` +5 unpushed; memory dir empty with 27 session transcripts.
- bebop build/test (executed in this audit): `cargo check --workspace --offline` PASS 2.51s
  (16 warnings, 0 errors); `cargo check -p bebop2-core` PASS 0.49s; `cargo test --workspace
  --offline` = bebop 275/275 + bebop-core 19/19 + bebop2-core 90/90 = **384 pass, 0 fail**.
- bebop LOC: `crates/` 17,390 (bebop ~16,080), `rust-core/` 1,043, `bebop2/` 6,685.
- bebop negative-space greps (all 0 hits): axum, sqlx, utoipa, supabase, fastify in Cargo.tomls;
  `.astro`/`.svelte` files; playwright configs/specs; REBUILD-MAP/traceability/parity-oracle/
  strangler/cutover in docs.
- bebop working tree: `git diff --stat` = 5 files, +1,331/−9 (kdf.rs +616 Argon2id, pq_dsa.rs +698
  ML-DSA-65, pq_kem.rs, sign.rs, AGENTS.md §0); untracked: 2 fable-research docs +
  `docs/design/fable-protocol-2026-07-11/`.
- dowiz↔bebop detritus in dowiz working tree (untracked): chacha/poly1305/xsalsa/xchacha PDFs,
  `crypto-primitives-research.md`, `hybrid-routing-sota.md`, `platform-vs-protocol-logistics.md`,
  `web3-logistics-postmortem.md`, `docs/design/five-tool-integration-report.md`,
  `docs/design/integration-research-report.md`.
- GitHub (queried live via `gh`): open issues #19 (sandbox egress) and #9 (guardrail scripts
  missing from main) — matching the todo map exactly.

---

*Audit performed 2026-07-11 by a read-only analysis session. The only file created is this report.
Working trees, branches, stashes, and worktrees in both repos were left untouched.*
