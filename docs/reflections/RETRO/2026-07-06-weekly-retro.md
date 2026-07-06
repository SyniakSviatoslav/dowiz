# Weekly harness retro — 2026-07-06

Curation run over `docs/reflections/INBOX/` (19 reflections, 2026-07-02 … 2026-07-05). Council
critics: cause-critic (Stage 10), pattern-critic, ratchet-critic. Advisory synthesis — deterministic
gates and the human decide; this doc enacts only doc/lesson-level outputs and lists guardrail-level
outputs as proposals.

## Council verdicts (summary)

- **cause-critic:** all 19 WHYs **CONFIRMED** — each shows a concrete, reproducible mechanism, none
  a mere correlate / coincidence / parallel-deploy. Weakest-but-still-confirmed: `astro-scaffold-
  served-humans` (gate-scope boundary inferred, not read from gate code), `readonly-session-trips-
  qualified-gate` (two equally-viable fix directions), `advisory-arm-revival` (confirms the known
  row-#48 law rather than a fresh root). No back-fill or rejection required; no unfilled-WHY
  placeholders found in INBOX.
- **pattern-critic:** 4 systemic roots (below).
- **ratchet-critic:** every systemic root's cheapest deterministic artifact is a **Tier-1 guardrail
  = operator proposal** this run (hooks / CI / eslint / ledger are outside the curation write-scope).
  Two Tier-2 pre-edit lessons were enactable; one was enacted (see Enacted).

## The 4 systemic roots (pattern-critic, all cause-confirmed)

1. **Contract defects are invisible until the LIVE surface is driven** (7 reflections:
   `cutover-sqlx-bind-decode-class`, `cutover-harness-staging-proof`, `astro-asset-404-incident`,
   `astro-scaffold-served-humans`, `ci-pre-prod-verification`, `trace-config-source-before-mutating`,
   `lc1-inclusive-tax-mirror-oracle`). One shape: a proof harness measures a **proxy** (a unit test,
   a fresh DB, a payload-parity datum, serde rules, a psql literal, a mirror-oracle) not the **deploy
   target**; a defect that lives only at the surface under load is structurally invisible until the
   surface is driven. In-flight guardrails: ledger #77 (`rust-live-pg` CI draft), #80 (storefront-
   styles E2E), #56 (money oracle-independent vectors), the `ci-*preflight` scripts.
2. **Proxy signals drift silently from ground truth; acted on without re-verification** (5:
   `proxy-signals-drift-from-ground-truth`, `plane-maintainer-env-probe`, `plane-telemetry-closed-
   loop`, `trace-config-source-before-mutating`, `rebuild-program-complete-retro`). Hand-authored
   proxies (memory prose, filename-grouped estimates, cached remote state, config assumptions,
   council-packet claims) trusted without re-reading the artifact — drift measured at up to 27×, or
   inverting a resolved/unresolved verdict. In-flight: 3 memory-measurement scripts (cbf0d088).
3. **Discipline-triggered steps die; only hook-enforced artifacts survive** (4: `governance-gates-
   rot-open`, `advisory-arm-revival`, `swarm-mergeback-rot`, `readonly-session-trips-qualified-gate`).
   The row-#48 law: an obligation with no durable artifact + no checker goes invisible within a week.
   In-flight: ledger #47 (gate-armament), #48 (harness-events + health-pass), #68 (sandbox-staleness),
   #69 (meta-controller).
4. **Shared mutable state without ownership boundaries → silent collisions** (2: `design-system-
   prune-collision`, `money-lane-impl` lane collision). Staged-but-uncommitted git index / overlapping
   lane file-scope, no lock or manifest → concurrent committer inherits the stage; HEAD unbuildable
   40 min in one case. No guardrail yet.

## Enacted this run (doc/lesson-level, within write-scope)

- **NEW lesson (Tier-2):** `docs/lessons/2026-07-05-sqlx-bind-decode-cast-parity.md`, TRIGGER
  `rebuild/crates/api/**/*.rs` — the sqlx bind/decode cast-parity rule (Root 1). Enacted because the
  class has **no live guardrail yet** (#77 is only a drafted CI job, `.github` protect-path), so a
  pre-edit nudge is the only active backstop. ratchet-critic-approved; well-scoped (api crate only,
  not the pure `domain` crate); red→green = drop a `::<enumtype>`/`::bigint` cast and the class 500s.
- **PRUNED orphan:** `docs/lessons/2026-06-29-docs-only-no-staging-deploy.md` deleted. Its `docs/**`
  INDEX row was **deliberately removed 2026-07-04T20:49** (last of 485 injections — it fired on every
  single docs edit = nag-noise); the file was left orphaned (no row → the hook can never inject it),
  i.e. dead weight. The rule (docs-only diff → skip staging deploy) already lives in CLAUDE.md Ship
  Discipline scoping. Net lesson store: **10 → 10 (flat).**
- **INBOX drained 19 → 0**; all reflections archived to `docs/reflections/ARCHIVE/`.

### Considered but NOT enacted (promotion beats storage)

- **Mirror-oracle lesson** (Root 1, from `lc1-inclusive-tax-mirror-oracle`) — ratchet-critic
  recommended it, but the money mirror-oracle class is **already a live Tier-1 guardrail (ledger
  #56:** oracle-independent literal vectors + a definitional property test) plus the money-council
  M4/P2 ratchets and the test-integrity mirror-lock banned class. A new lesson would grow the store
  for marginal gain over an existing guardrail → declined per the librarian's promote-over-hoard law.

## Guardrail-level PROPOSALS (operator — outside curation write-scope: `.claude/**`, CI, eslint, ledger)

1. **Wire the `rust-live-pg` CI job** (Root 1; ledger #77 draft at `docs/design/ci-rust-live-pg/`,
   `OPERATOR-APPLY.md` ready): reuse `fresh-provision` PG16 + roles + migration-chain, then
   `cargo test --features dev-routes -- --include-ignored` in `rebuild/`. Collapses the whole sqlx
   bind/decode + serde-parity class from discover-on-prod to discover-in-CI. Ratchet-refinements from
   `cutover-sqlx`: don't apply 086/087 before the refund "degrades-while-unapplied" arm; seed a
   canonical location for the create/erasure probes (not `Uuid::new_v4()`); single runner for the
   advisory-lock cron tests.
2. **Council-packet `path:line` linter — G1** (Root 2; from `rebuild-program-complete-retro`): flag a
   🔴 red-line claim in a council packet that carries no `file:line` citation, and require the breaker
   to independently re-read each cited line. Promotes verify-artifact-not-proxy from advisory to gate.
3. **serious-gate `rebuild/crates/**` red-line glob — G2** (Root 3; from `rebuild-program-complete-
   retro`): the PreToolUse serious-gate matches auth/money/RLS/migrations by PATH, but all rebuild
   red-line code lives under `rebuild/crates/**`, which is outside the regex — so S6–S10 red-line
   writes went ungated by the per-file gate (covered instead by SSG review + council). One-line,
   deterministic, no judgment. **Edits `.claude/hooks/serious-gate.sh` → operator-applied.**
4. **Diff-aware red-line scan** (Root 3; from `cutover-harness-staging-proof` #3 + `money-lane-impl`
   #2): `post-edit-gates.sh` greps the WHOLE file for red-line patterns, so a pre-existing field-name
   line (e.g. `BACKUP_PII_FIELDS` containing `customer_phone`) trips on every future edit — a
   false-positive that trains operators to ignore red. Make the scan added-lines-only, or add an
   inline allowlist marker for field-name literals. **`.claude/hooks/**` → operator-applied.**
5. **Qualified-change gate scope** (Root 3; from `readonly-session-trips-qualified-gate`): scope the
   Stop-hook qualified-change detector to the **session-authored diff**, not raw `git status`, so a
   read-only orientation session doesn't inherit a prior session's uncommitted qualified surface — OR
   pair the tree-based check with a session-end "commit or explicitly park" step. **`.claude/**`.**
6. **Lane-ownership manifest + PreToolUse hook, or worktree-by-default** (Root 4; from `money-lane-
   impl` + `design-system-prune-collision`): a lock file listing lane→glob claims a PreToolUse hook
   consults, so a second lane touching a claimed path gets red-line-style friction; or parallel
   sessions get their own worktree by default. **Needs a human/council call (may need session
   attribution) — `.claude/**` + orchestration.**
7. **UX-parity flip-DoD gate + feature-presence E2E census** (Root 1; from `astro-scaffold-served-
   humans` + `astro-asset-404-incident`): a surface flip that replaces UI (not just API) requires a
   UX-parity gate — feature matrix green + visual-regression diff vs the Node baseline + core flows
   driven E2E on the candidate stack + a test-id census (compare-toggle, macro-lens, cart, tabs).
   **Belongs in REBUILD-MAP / cutover runbook — council owns.**

## Open items carried forward (not silently dropped)

- `design-system-prune-collision` item 3: re-verify commit `06471162` (landed with a one-off
  `--no-verify` while `guardrail-hook-matchers` was red) against the full gate now that guard-bash
  registration has landed. → Council retro owns (this doc); action still open.
- Escalations (council-class, NOT librarian-fixable): `routes/owner/dashboard.ts:379-429` broadcasts
  `status:'PICKED_UP'` on the dashboard WS channel but never persists `orders.status` (UI/DB
  divergence — from `token-router-probes`); pre-existing staging storefront checkout-flow breakage +
  the 3-messenger-kind 422 (from `rebuild-wave2-channel-integration`).
