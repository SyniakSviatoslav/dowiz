# G12 — The known-broken / needs-investigation queue (8 open items, some stale a week)

**Date:** 2026-07-11 · **Status:** research + execution blueprint (no code changed; read-only session)
**Sources:** audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` §6.4 (the queue), §7.5(b)
(staleness-guard blind spot), §9 rec 8 (harvest/prune worktrees); memory corpus
`/root/.claude/projects/-root-dowiz/memory/` (`rebuild-decision-rust-astro-2026-07-04.md:124-131`,
`session-resume-2026-07-05.md:35-38`); `docs/ops/rebuild-cutover-h_t.json:49` (degrade storm);
`gh issue view 19` (live query this session). Sibling blueprint cross-ref: `G03-checkout-422-messenger-kinds.md`.

---

## 1. Gap & evidence

Eight items sat in "known-broken / needs-investigation" with no closure loop. Each was flagged with
evidence at the time (07-02 → 07-08), none was re-verified since, and at least one (the staleness
guard) is a guard whose *own blind spot* is the thing it was built to prevent. This blueprint
re-grounds every item to 2026-07-11 truth and maps the fixes.

| # | Item | Flagged | Age |
|---|------|---------|-----|
| 1 | Staging checkout E2E break at `checkout-phone` testid | 2026-07-04 | 7d |
| 2 | Staging E2E vs 100 req/min/IP rate limiter — strategy undecided | 2026-07-05 | 6d |
| 3 | Owner pickup proxy broadcasts `PICKED_UP` without persisting `orders.status` | 2026-07-05 | 6d |
| 4 | Degrade-storm ratchet (task #15): boot-grace + restart-regression-test + alert-on-degrade | 2026-07-05 | 6d |
| 5 | Pre-commit >8 min hang class for build-relevant commits ("move Docker→CI", P1) | ~2026-07-04 | ~7d |
| 6 | `pnpm lint:gates` ERR_MODULE_NOT_FOUND `@eslint/js` (hit by 3 agents) | ~2026-07-05 | ~6d |
| 7 | Stale worktrees `dowiz-wt-phase0`/`-phase5` (07-02) unflagged by the staleness guard | 2026-07-02 | 9d |
| 8 | GH #19: cloud-sandbox 403 egress blocks plane-maintainer deploys + Telegram | 2026-07-08 | 3d, 5 recurrences |

---

## 2. Research findings (per-item ground truth, verified this session)

### 2.1 Checkout `checkout-phone` E2E break — STILL-REAL as *test-infra debt*; product break is G03's, not this

- The testid **does not exist anywhere in `apps/web`** (whole-tree grep: 0 hits in `apps/`+`packages/`),
  and `git log -S "checkout-phone" -- apps/web` on the paleo lineage returns **nothing** — in this
  lineage the FE never had it. The phone field was folded into the Communication selector (ADR-0016):
  the live testids are `checkout-communication` (kind `<select>`) and `checkout-comm-handle` (handle
  input) — `apps/web/src/pages/client/checkout/ContactInfoSection.tsx:82,98` (comment at :71-73
  states the fold explicitly).
- **12 e2e files / 18 references** still target the dead testid: `e2e/lifecycle-e2e/support/selectors.ts:7`
  (`phoneInput: 'checkout-phone'`), `e2e/tests/client/checkout.spec.ts:86`,
  `client/client-checkout-happy-path.spec.ts:73`, `flow-ui-client-checkout.spec.ts` (×4),
  `flow-ui-client-order-full.spec.ts` (×2), `visual/client-path.visual.spec.ts` (×3), `visual/harness.ts`,
  `flow-simpl-s1-sheet-checkout.spec.ts`, `flow-simpl-s1-velocity-frictionless.spec.ts`,
  `mobile-polish.spec.ts`, `channel-attribution.spec.ts`, `golive-remediation.spec.ts`. Every one of
  these fails at the selector **by construction** — exactly the 07-04 symptom
  (memory `rebuild-decision-rust-astro-2026-07-04.md:125-126`: "fails identically at
  getByTestId('checkout-phone')").
- The *product* flow: statically, checkout works end-to-end for telegram/whatsapp/viber kinds; the
  phone/signal/simplex kinds 422 at order-create because the FE always sends `messenger_kind`
  (`CheckoutPage.tsx:329`) against the 3-kind Zod enum (`packages/shared-types/src/legacy.ts:48`,
  used by `apps/api/src/routes/orders.ts:93`). That half is **G03's gap** — do not double-fix here.
- **Verdict: the "investigate loop" is closable — root cause found. Fix = selector migration + a
  selector-parity guardrail so a testid rename can never silently strand 12 specs again.** Live
  staging reproduction was not run (order placement = mutation; out of scope for this session).

### 2.2 Staging E2E vs rate limiter — STILL-REAL, strategy genuinely undecided; half a mitigation is in place

- Config verified: global `@fastify/rate-limit` `max: 100, timeWindow: '1 minute'`, keyed on the
  real client IP (`Fly-Client-IP`) — `apps/api/src/server.ts:362-379`. **No `allowList`, no bypass
  of any kind exists.** All E2E traffic from one runner shares ONE bucket by design of #9 hardening.
- Partial mitigation already committed: `playwright.config.ts:8` has `workers: 1` (+
  `fullyParallel: false`) — the 07-05 workaround ("run `--project=mobile --workers=1`",
  `session-resume-2026-07-05.md:36`) became the default. Cost: a 179-spec × 3-project matrix run is
  serial and slow, and long lifecycle specs that poll the API can still burn >100 req/min inside one
  worker — false-fails remain possible by construction.
- Useful existing plumbing for the bypass option: E2E already sends `x-dev-auth-secret` on every
  request (`playwright.config.ts:15-18`), CI provides `DEV_AUTH_SECRET` (`.github/workflows/ci.yml`
  post-deploy steps), and the API already validates that header (`server.ts:415`,
  `plugins/dev-guard.ts`). The bypass can ride a secret that is already deployed and rotated.
- **Verdict: still-real; decision needed (see §3/§4 — recommend secret-gated allowList on
  staging, keep serialization as the prod-run posture).**

### 2.3 Owner pickup proxy — STILL-REAL, VERIFIED in current code; and "persist PICKED_UP" would be the WRONG fix

- `apps/api/src/routes/owner/dashboard.ts:379-444` (same location as flagged): the handler updates
  `courier_assignments.status='picked_up'` (:410-413) + audit log, **never touches `orders.status`**,
  then broadcasts to the dashboard channel `type:'order.status', data:{status:'PICKED_UP'}` (:426-429).
  DB truth stays `IN_DELIVERY` (set at assign time via the canonical `updateOrderStatus` — :354).
  A dashboard reload reverts the chip; the WS event claims a state the DB never held.
- Crucially, the state machine (`packages/domain/src/order-machine.ts:28,39,42`) defines `PICKED_UP`
  as a **terminal state for customer-pickup orders only** (`READY → PICKED_UP`; transitions out: `[]`).
  `IN_DELIVERY → PICKED_UP` is illegal. So the escalation's implied fix ("persist it") would corrupt
  the machine. The bug is the **phantom broadcast**, not missing persistence.
- Contrast, same file: assign uses `updateOrderStatus(...'IN_DELIVERY')` (:354), deliver-proxy uses
  the shared `completeDelivery` primitive (:493+). Pickup is the only non-canonical broadcast. The
  courier's own pickup route (`routes/courier/assignments.ts:255-280`) publishes only
  `BUS_CHANNELS.ORDER_PICKED_UP` + the canonical status delta — no fake order.status. Also noted:
  `apps/web/src/pages/admin/DashboardPage.tsx` has **zero** references to `PICKED_UP`, so the fake
  event feeds an unknown status string into the FE merge.
- **Verdict: still-real; fix = align the broadcast with the courier path (assignment-level event /
  real DB delta).**

### 2.4 Degrade-storm ratchet (task #15) — STILL-REAL, none of the three ratchet parts exists

- Incident ground truth (`docs/ops/rebuild-cutover-h_t.json:49`): 2026-07-05 14:02, a Node restart
  auto-degraded ALL non-money surfaces (S1-S4, S6, S10) to Node **silently** while Rust was healthy;
  flags manually restored ~15:5x; discovery accidental. Ratchet demanded: boot-grace +
  restart-regression-test + alert-on-degrade.
- Code verified (`apps/api/src/lib/cutover/front-door.ts`):
  - `UpstreamHealth.start()` (:123-128) probes **immediately** on boot; trips after
    `HEALTH_TRIP_AFTER = 3` consecutive fails (:97,:163); probe timeout 2 s. **No boot-grace of any
    kind.** Interval from `env.CUTOVER_HEALTH_INTERVAL_MS` (`server.ts:444`).
  - Trip callback (:334-340) degrades every rust-flagged non-money surface; there is a second,
    per-request degrade vector at :422 (`upstream-unhealthy-at-request`) with the same no-grace
    exposure.
  - Recovery (:152-158) re-enables *forwarding* but "flags unchanged" — the flag flip is a one-way
    ratchet toward Node (by design, REV-C5); which is exactly why an unguarded boot-window trip is
    *permanent* until a human notices.
  - `flags.ts autoDegrade` (:136-167): money surfaces refused, 30 s debounce, and on success only
    `log.error(...)` — **no bus event, no Sentry, no Telegram**. "Alert-on-degrade" does not exist.
  - Tests: `apps/api/tests/cutover-front-door.test.ts` has a FakePool harness that already records
    `degradeCalls` (:33-41) and covers REV-C5 — **no restart/boot-phase test exists**.
- **Verdict: still-real; all three ratchet parts missing; the existing test harness makes the
  regression test cheap.**

### 2.5 Pre-commit >8 min hang class — STILL-REAL for build-relevant commits (by construction)

- `.husky/pre-commit` verified: guardrails always run (cheap); then the dynamic-scope gate — a commit
  staging anything under `apps/|packages/|package.json|Dockerfile|fly.toml|tsconfig*|vite.config*`
  runs `pnpm -r typecheck` + `pnpm -r build` + `flyctl config validate` + **a full local
  `docker build`** wrapped in two `docker-disk-guard.sh` reclaim passes. Docs-only commits skip
  (that fix landed); build-relevant commits carry the full multi-minute path. The P1 "move
  Docker→CI" was never executed: **`.github/workflows/ci.yml` contains no docker build step at all**
  (grep: 0 hits) — Fly's `--remote-only` cloud build at deploy is currently the only container gate.
- **Verdict: still-real; the fix is a deletion (pre-commit) + an addition (CI docker job), not a
  rewrite.**

### 2.6 `pnpm lint:gates` — STALE / ALREADY-FIXED (reproduced green this session)

- Ran it: exits **0**, prints the expected fixture warnings; `node -e "import('@eslint/js')"`
  resolves. `node_modules/@eslint/js` present (`@eslint+js@9.39.4` in `.pnpm`). The historical
  ERR_MODULE_NOT_FOUND was almost certainly a partially-installed `node_modules` in those agents'
  sandboxes (3 hits recorded in PROGRESS.md BLOCKERS), fixed by a later full `pnpm install`.
- **Verdict: stale-already-fixed. Action = close the BLOCKERS entry with this reproduction as proof;
  optional 1-line hardening (preflight hint) below.**

### 2.7 Stale worktrees + WHY the staleness guard is blind — STILL-REAL; blind spot fully explained (three independent misses)

Guard: `scripts/guardrail-sandbox-staleness.mjs`, wired at `scripts/verify-all.ts:64` (`--ci`) and in
`.husky/pre-commit` §1.4g (no flag). The two worktrees are invisible to it for **three** reasons, any
one of which suffices:

1. **Path filter** — `scanWorktrees()` line 47: `if (!rel.startsWith('.claude/worktrees')) continue;`.
   The worktrees live at `/root/dowiz-wt-phase0` / `-phase5` (siblings of the repo, `git worktree
   list` confirms) — outside the glob, skipped before any staleness math.
2. **Predicate** — `isAtRisk` line 29-31: `behind >= 5 && untracked > 0`. Both worktrees have
   **0 untracked** files; their 11 changed files each are *staged* modifications (`git status
   --porcelain` = `M ` rows; `git diff --stat` empty, `git diff --cached --stat` = 11 files). The
   guard's comment ("modified TRACKED files are recoverable from git") is wrong for staged-only
   state: `git worktree remove --force` deletes the per-worktree index; the staged blobs become
   unreachable loose objects (gc-able) — practically lost.
3. **Enforcement mode** — pre-commit invokes it WITHOUT `--ci` → report mode → `exit 0` always
   (script line 74 path). The only `--ci` invocation is `verify:all --ci` **in CI**, where a fresh
   clone has no worktrees → the strict mode can structurally never fire. This fully explains "commits
   on 07-10 passed pre-commit".

**Worktree inventories (read-only, this session):**

- Both worktrees carry the **identical** 11 staged files (`.agents/tmp/check-*.mjs`,
  `test-connections.cjs`, `apps/api/fix-db.js`, `packages/db/scripts/check-*.ts`,
  `test-connections.js`, `packages/platform/test-notify{2,3}.cjs`) — and every staged hunk does the
  same thing: **re-inserts a real Supabase pooler credential over the `REDACTED` placeholder that
  their HEAD commit contains** (verified on `apps/api/fix-db.js`: `postgres.elxukhxvuycnftqwaghg:…@
  aws-1-eu-central-1.pooler.supabase.com`). These are throwaway debug scripts whose only delta is a
  secret. **Harvest value: zero. Disposition: discard — deliberately.** (Security note for the
  operator: confirm this credential is in the rotated/dead set from the secrets-incident arc before
  treating the discard as closed.)
- Unique committed work (safe: commits live on branches in the **shared** object DB — worktree
  removal cannot destroy them):
  - `feat/phase0-hardening` tip `7a4f7aca` "guarded full restore + reconciled DR runbooks"
    (backup-restore.ts +99, deletes `apps/api/src/scripts/restore.ts`, runbook reconcile).
    **NOT content-contained in HEAD** (HEAD still carries the old `restore.ts`; `git diff HEAD
    7a4f7aca` shows the DR work absent). Harvest-worthy.
  - `feat/phase5-adaptive-gps` tip `07894df1` "stop forcing fresh GPS fixes on every watch tick"
    (7-line `useGeolocation` change: `timeout 5000→15_000`, `maximumAge 0→10_000`,
    `apps/web/src/pages/courier/DeliveryPage.tsx`). **NOT in HEAD** (HEAD still has `maximumAge: 0`).
    Harvest-worthy — but **hunk-level only**: the rest of that file at HEAD is *newer* than the
    worktree base (HEAD has `computeDestinationRoute`, the no-location state, pickup-failure toast;
    the worktree base still has MOCK_* pins). Cherry-picking the whole commit would regress S3/S4/LC9
    fixes.
  - **Lineage warning:** the worktree branches have **no merge-base with paleo HEAD** (verified:
    `git merge-base` empty — they predate the history scrub). Harvest by content (apply/re-type the
    hunks), never by merge.
- **Channel-adapter / IG worktrees (audit §9 rec 8):** gone as worktrees (`git worktree list` = main
  + phase0 + phase5 + one `prunable` dead scratchpad entry `/tmp/.../integrate`). The IG lane's
  MessengerKind draft survives in-tree at `docs/design/checkout-communication/proposal.md`
  (recovery already mapped in G03 §2.2); channel-adapter content is in HEAD
  (`apps/api/src/lib/channel.ts`, referenced from `orders.ts:100`). Residue: **27
  `worktree-agent-*` branches** (07-02→07-04 tips) as clutter — prune list is operator-gated.

### 2.8 GH #19 cloud-sandbox egress — STILL-REAL, infra/policy, now 5 consecutive daily recurrences

- Live `gh issue view 19` (this session): OPEN, created 2026-07-08; proxy answers **403 to CONNECT**
  for `fly.io:443` and `api.telegram.org:443`; `flyctl` absent and its installer fetch hits the same
  403; operator comment 2026-07-10 confirms the **5th** consecutive identical daily failure. Nothing
  the agent can self-fix inside the sandbox — it crossed the charter's N=3 escalation threshold.
- Grounding for the reroute option: `.github/workflows/ci.yml:133-190` already holds a working Fly
  deploy path (`FLY_API_TOKEN` secret, `flyctl deploy --remote-only`) on GH runners, where egress
  works; and GitHub API egress from the sandbox works (the issue itself + its comments prove it).
- **Verdict: still-real. Two viable paths, both operator-gated (§6).**

---

## 3. Options & tradeoffs

| Item | Option A | Option B | Recommendation |
|------|----------|----------|----------------|
| 1 selector break | Mechanical selector migration (12 files → `checkout-communication`+`checkout-comm-handle`), centralize on `lifecycle-e2e/support/selectors.ts` | A + a deterministic **selector-parity guardrail** (extract `getByTestId`/`data-testid` literals from e2e, assert each exists in `apps/web` source) | **B** — the guardrail kills the whole class (this exact rot went unnoticed ≥7 days across 12 files) |
| 2 rate limiter | Keep `workers:1` serialization only (no code change; slow, still false-fails on poll-heavy specs) | Secret-gated `allowList` in the rate-limit config: bypass when `x-dev-auth-secret` matches `DEV_AUTH_SECRET` **and** an explicit `RATE_LIMIT_E2E_BYPASS=true` env is set (staging only; prod never sets it) | **B** — rides already-deployed secret plumbing; restores matrix parallelism; prod posture unchanged. Keep `workers:1` for prod-target post-deploy runs |
| 3 pickup proxy | Persist `orders.status='PICKED_UP'` | Fix the broadcast: publish the courier-parity events (order-channel `ORDER_PICKED_UP` stays; replace the dashboard `order.status: 'PICKED_UP'` fake with a real `fetchOrderDelta`-based delta, or an explicit `assignment.status` event) + FE render of assignment state if the owner needs a "picked up" chip | **B** — A is illegal in the state machine (`IN_DELIVERY→PICKED_UP` not a transition; PICKED_UP is pickup-order terminal) and would corrupt terminal-state logic (`isTerminal`, order-machine.ts:56) |
| 4 degrade storm | Boot-grace only | Full task #15: (i) boot-grace in `UpstreamHealth` (suppress trip-driven **and** request-path autoDegrade until first successful probe OR grace deadline, e.g. 120 s — forwarding still fails safe per-request without flag writes); (ii) alert-on-degrade (bus event `ops.cutover_degrade` + Sentry in `autoDegrade`, alongside the existing log); (iii) restart-regression test in the existing FakePool harness | **B** — the incident doc explicitly demands all three; (ii) is what makes the next storm *observed* instead of accidental |
| 5 pre-commit | Filtered typecheck/build (`pnpm --filter ...`) keeping Docker local | Delete steps 4/5 (Fly validate + Docker build + disk-guards) from pre-commit; add a `docker build` job to CI `validate`; keep `pnpm -r typecheck` local, demote `pnpm -r build` to CI | **B** — matches the standing P1 decision; deletion is the robust fix; Fly's cloud build already gates deploys. (Operator sign-off: local gate gets weaker by design) |
| 6 lint:gates | Close as fixed with reproduction proof | A + a preflight hint in the script (`node -e import('@eslint/js')` → "run pnpm install" message) | **A** (B optional, 5 lines) — the failure mode was environmental |
| 7 guard + worktrees | Patch predicate only | Fix all three misses: scan **all** `git worktree list` entries except the main checkout (drop the `.claude/worktrees` filter or make it a repo-adjacent glob incl. `../dowiz-wt-*`); widen loss-risk to `untracked > 0 \|\| stagedOrModified > 0` when stale (rename signal to `atRisk` reasons); run `--ci` in pre-commit. Plus the harvest/prune map (§4 batch D) | **B** — each miss independently reproduces the blind spot; fixing one leaves the guard demonstrably bypassable |
| 8 egress | Operator adds proxy allowlist for `fly.io`/`api.fly.io`/`api.telegram.org` (+flyctl install host) | Re-route side-effects: staging deploy via a `workflow_dispatch` GH Action (clone of ci.yml deploy job pointed at staging), reports via GH issue/PR comments (egress proven) instead of Telegram | **B as default** (no policy change, deterministic), **A as operator preference** — they compose |

---

## 4. Recommended execution blueprint (sequenced batches)

Grouping by locality: batch A = e2e infra (items 1+2, unblocks every later staging proof); batch B =
guard/hook infrastructure (items 7-guard + 5, same `.husky`/`scripts/` surface); batch C = cutover
module (item 4); batch D = API route (item 3); batch E = closures + operator actions (6, 7-harvest,
8). Estimated total: ~3 focused sessions.

### Batch A — restore E2E ground truth (items 1, 2) — effort M (~1 session)

| Step | Action | Gate marker | VbM falsifiable proof | Effort |
|------|--------|-------------|----------------------|--------|
| A1 | Migrate all 18 `checkout-phone` refs (12 files, §2.1 list) to `checkout-communication` + `checkout-comm-handle`; route them through `lifecycle-e2e/support/selectors.ts` where practical | pre-commit (lint) | Run `client-checkout-happy-path.spec.ts` vs staging **before** (RED at selector, current truth) and **after** (GREEN past the contact step). Paste both | S–M |
| A2 | New `scripts/guardrail-e2e-selector-parity.mjs`: extract `getByTestId('…')` / `data-testid="…"` literals from `e2e/**`, assert each exists in `apps/web/src/**` (allowlist for dynamically-built ids); wire into `verify-all.ts` + pre-commit §1.4g | verify:all `--ci` | RED case: temporarily rename `checkout-comm-handle` in a scratch copy → guard exits 1 naming the orphaned spec. GREEN on current tree post-A1. Ship both runs | S |
| A3 | Rate-limit bypass: `allowList` fn in `server.ts` rate-limit config — `env.RATE_LIMIT_E2E_BYPASS === 'true' && header === env.DEV_AUTH_SECRET` (timing-safe compare); set the env var on **staging only** | red-line adjacent (server.ts) → normal review, no red-line glob touched | Unit test on the allowList fn: with header+flag → not limited at request 101; wrong secret → 429 envelope (`code:'RATE_LIMIT'`) at >100 (RED case); flag unset (prod shape) → 429 regardless of header | S |
| A4 | Record the DECISION (bypass on staging, serialized `workers:1` for prod-target runs) in memory + `docs/operating-model/` note — this closes the "never decided" state | memory write | The decision doc lists the falsifier: if staging matrix still 429s with bypass on, the decision is wrong → reopen | XS |

### Batch B — guard infrastructure (items 7-guard, 5) — effort M (~1 session)

| Step | Action | Gate marker | VbM falsifiable proof | Effort |
|------|--------|-------------|----------------------|--------|
| B1 | `guardrail-sandbox-staleness.mjs`: scan every `git worktree list` entry except the primary checkout (kill the `.claude/worktrees` prefix filter; keep lane naming); widen predicate: at-risk when `behind >= 5 && (untracked > 0 \|\| changed > 0)` with distinct reason labels (`untracked-loss` vs `uncommitted-index-loss`); pre-commit §1.4g gains `--ci` | pre-commit + verify:all | **The live proof is the two real worktrees**: run the fixed guard → MUST print 🔴 ×2 for `dowiz-wt-phase0/5` and exit 1 under `--ci` (this is the "test worktree that MUST be flagged", using reality). Unit RED cases in `guardrail-sandbox-staleness.test.mjs`: `{behind:20, untracked:0, changed:11} → true`; revert predicate to `untracked>0`-only → test fails | S–M |
| B2 | After batch D (harvest/prune) empties the worktrees, add a *synthetic* fixture test: script creates a temp worktree outside `.claude/worktrees`, commits nothing, stages one file, rewinds 5+ commits behind, asserts guard flags it, then removes it (self-cleaning; test-only worktree, allowed) | verify:all | The fixture IS the falsifiable input; deleting the `--ci` flag in pre-commit or re-adding the path filter turns it green-blind → fixture test red in CI | S |
| B3 | Pre-commit slimming: delete steps 4/5 (Fly validate, Docker build, both `docker-disk-guard.sh` calls); keep `pnpm -r typecheck`; move `pnpm -r build` + a new `docker build` job into CI `validate` | operator sign-off (weakens local gate — deliberate) | Timing proof: `time git commit` on a synthetic build-relevant change < 3 min (was >8); CI run shows the docker job executed and RED-capable (introduce a Dockerfile syntax error on a scratch branch → CI fails) | S–M |

### Batch C — degrade-storm ratchet, task #15 (item 4) — effort M

| Step | Action | Gate marker | VbM falsifiable proof | Effort |
|------|--------|-------------|----------------------|--------|
| C1 | `UpstreamHealth`: add boot-grace — record `bootAt`; expose `inBootGrace` (true until first `recordOk()` OR `now - bootAt > BOOT_GRACE_MS` [default 120 s, env-tunable]); trip callback and the `:422` request-path skip `autoDegrade` while `inBootGrace` (forwarding still per-request fail-safe to Node — no behavior loss, no flag writes) | cutover module review (non-money code path; REV-C5 untouched) | New tests in `cutover-front-door.test.ts` FakePool harness: (i) boot + 3 failed probes inside grace → `degradeCalls.length === 0` **(this test is RED on today's code — it IS the restart-regression test)**; (ii) after first OK, 3 fails → `degradeCalls > 0`; (iii) grace expiry with rust never up → degrade fires (no infinite immunity) | M |
| C2 | Alert-on-degrade: in `flags.autoDegrade` success path publish `ops.cutover_degrade` on the message bus + `getSentry()?.captureMessage` (same pattern as the refund-due fold in `orderStatusService.ts:182-195`) | none (additive) | Unit: FakePool + fake bus → one degrade produces exactly one bus event with `{surface, reason}`; RED case: money surface → zero events (REV-C5 refusal precedes) | S |
| C3 | Staging restart drill (operator-observed): restart the Node app on staging with Rust healthy; assert via `cutover_flags` SELECT that **zero** flags flipped and one grace log line appeared | operator (staging touch) | The drill's falsifier is C1(i) reverted: without boot-grace this exact drill flipped 6 surfaces on 07-05 (recorded). Post-fix drill must show 0 | S |

### Batch D — pickup proxy broadcast (item 3) — effort S

| Step | Action | Gate marker | VbM falsifiable proof | Effort |
|------|--------|-------------|----------------------|--------|
| D1 | `dashboard.ts:426-429`: replace the fake `order.status:'PICKED_UP'` dashboard publish with a real delta (reuse the `fetchOrderDelta` pattern — status will truthfully read `IN_DELIVERY`, and include assignment picked-up info as an explicit `assignment` field or a distinct `type:'assignment.status'` event). Order-channel `ORDER_PICKED_UP` (:423) stays — it is courier-parity | contract-adjacent → integration test required (Task-Exit rule) | Integration test (route-level): owner pickup → (a) DB `orders.status === 'IN_DELIVERY'`, `courier_assignments.status === 'picked_up'`; (b) captured dashboard-channel messages contain **no** `order.status` value absent from the DB row. **This assertion is RED on current code** (captures `PICKED_UP` ≠ DB) — ship the red run | S |
| D2 | If the owner UI should show "picked up": render from the assignment field (FE) — separate, optional; DashboardPage today has zero PICKED_UP handling so nothing breaks either way | none | Playwright vs staging: dashboard chip after owner-pickup + after reload are IDENTICAL (today they differ — that divergence is the falsifier) | S |

### Batch E — closures & operator-gated executions (items 6, 7-harvest, 8)

| Step | Action | Gate marker | VbM proof | Effort |
|------|--------|-------------|-----------|--------|
| E1 | Close the `lint:gates` BLOCKERS entry (PROGRESS.md) citing §2.6 reproduction (`exit=0`, `@eslint/js` resolves, 9.39.4 installed) | none | The reproduction run in this doc; falsifier = any agent re-hitting it after full `pnpm install` reopens | XS |
| E2 | **Harvest map (operator or a gated session executes):** (1) apply `07894df1`'s `useGeolocation` hunk (timeout 15 000 / maximumAge 10 000 + its comment) onto HEAD `DeliveryPage.tsx` — hunk-only, NOT the commit; (2) re-land `7a4f7aca` DR content: port `scripts/backup-restore.ts` guarded-restore + runbook reconcile + delete `apps/api/src/scripts/restore.ts` — re-review against HEAD's current backup code, don't blind-apply (no merge-base) | pre-commit; DR piece touches restore path → operator review | (1) unit/manual: geolocation options object asserted in a component test, RED if reverted; (2) restore drill per the reconciled runbook on a scratch DB | S + M |
| E3 | **Prune (DESTRUCTIVE — operator only):** after E2 lands and the credential-rotation check (§2.7) is confirmed: `git worktree remove --force /root/dowiz-wt-phase0` and `-phase5` (discards the 11 secret-bearing staged files — deliberate), `git worktree prune` (clears the dead `/tmp/.../integrate` entry). Branches `feat/phase0-hardening`, `feat/phase5-adaptive-gps` + 27 `worktree-agent-*` branches: keep until E2 verified, then operator decides the branch-prune list | **OPERATOR-GATED** | Post-prune, batch-B guard runs clean AND the B2 fixture still proves it can fire (guard didn't go green by losing its subjects) | XS (execution) |
| E4 | **GH #19 reroute:** add `deploy-staging.yml` (`workflow_dispatch`, clone of ci.yml deploy job → staging app + `FLY_API_TOKEN`); plane-maintainer triggers it via `gh workflow run` (egress-proven) and posts reports as GH issue comments; Telegram push moves to the GH-runner side or waits for E5 | repo CI change → review | Next plane-maintainer run's prediction ledger entry flips gap=hit → gap=miss; a staging deploy run URL exists. Falsifier: a 6th identical 403 ledger entry after the reroute = blueprint failed | M |
| E5 | **Egress allowlist (policy):** operator decides whether to also allow `fly.io`/`api.fly.io`/`api.telegram.org` + flyctl installer in the sandbox proxy | **OPERATOR-GATED (infra policy)** | `curl -sS https://api.telegram.org` from the sandbox returns non-403 | operator |

**Priority order:** A (unblocks all staging proofs; item 1 has sat longest with the most spec surface)
→ B (the guard's blind spot is an active data-loss class; B3 rides the same files) → C (cutover
integrity precondition for any further flip work, cheap given the harness) → D (small, isolated) →
E (closures; E3/E5 operator-scheduled anytime after their preconditions).

---

## 5. Risks & rollback

- **A3 (rate-limit bypass):** risk = bypass leaking to prod. Mitigation: dual condition (explicit
  env flag AND secret header); prod never sets `RATE_LIMIT_E2E_BYPASS`; unit RED case covers the
  flag-unset shape. Rollback: unset the env var — config-only, instant.
- **B1 (widened staleness predicate):** risk = crying wolf on legitimately busy worktrees, teaching
  people to bypass the gate. Mitigation: at-risk still requires `behind >= 5`; fresh WIP stays
  exempt (existing "no crying wolf" test retained). Rollback: revert the script; the guard is
  additive and gates nothing else.
- **B3 (pre-commit slimming):** risk = broken Docker builds reach CI instead of being caught
  locally. Accepted deliberately (the P1 decision); the new CI docker job + Fly cloud build keep two
  container gates. Rollback: `git revert` on `.husky/pre-commit` (hook text is tracked).
- **C1 (boot-grace):** risk = masking a genuinely-down Rust upstream for the grace window. Bounded:
  per-request forwarding still fails safe to Node during grace (users unaffected); grace expiry
  restores full trip behavior; test (iii) pins that. Rollback: set `BOOT_GRACE_MS=0` env.
- **D1 (broadcast change):** risk = any dashboard consumer that *did* rely on the fake `PICKED_UP`
  string. Verified none in `apps/web` (zero grep hits); WS payloads are not a public contract.
  Rollback: revert one publish block.
- **E2/E3 (harvest/prune):** risk = discarding something valuable in the staged files. Mitigated by
  this doc's per-hunk inventory (all 11 staged diffs = credential re-insertions, personally
  inspected); commits survive on branches regardless. The prune itself is irreversible for the
  staged state — hence operator-gated and sequenced strictly after E2 + the rotation check.
- **E4 (deploy reroute):** risk = staging deploy secrets exposure in a dispatchable workflow.
  Mitigation: `workflow_dispatch` restricted to repo collaborators; token already lives in GH
  secrets for the prod deploy job.
- **Cross-cutting:** the dirty `feat/paleo-dinosaur-digs` tree (uncommitted `verify-all.ts` /
  `plane-guard.mjs` gate fixes — audit §9 rec 3) should land BEFORE batch B edits the same files, or
  batch B inherits merge friction. Sequence with G-item owner for rec 3.

---

## 6. Operator decision points

1. **E3 worktree prune** — destructive (`worktree remove --force` discards the secret-bearing staged
   state). Prerequisites: E2 harvest landed + confirmation that
   `postgres.elxukhxvuycnftqwaghg@aws-1-eu-central-1.pooler.supabase.com` credential is rotated/dead.
   Also: approve (or defer) pruning the 27 stale `worktree-agent-*` branches.
2. **E5 / GH #19 policy** — sandbox egress allowlist is infra policy; only the operator can change
   it. Decide: reroute-only (E4), allowlist-only (E5), or both. Until then #19 stays open by design.
3. **B3 pre-commit slimming** — consciously weakens the local gate (Docker/Fly checks move to CI).
   Standing P1 says yes; needs the operator's explicit ack since pre-commit is governance surface.
4. **A3 staging bypass** — confirm staging-only scope for `RATE_LIMIT_E2E_BYPASS` and whether the
   post-deploy **prod** E2E steps in ci.yml keep the serialized no-bypass posture (recommended: yes).
5. **Item 1 vs G03 sequencing** — selector migration (A1) makes checkout specs green for
   telegram/whatsapp/viber kinds only; the phone/signal/simplex 422 remains G03's operator-gated fix.
   Decide whether A1 ships ahead of G03 (recommended: yes — independent surfaces).
6. **Degrade-storm drill (C3)** — restarting staging Node is operator-visible; schedule it.
