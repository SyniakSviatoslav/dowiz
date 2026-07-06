# STRUCTURE-UPGRADE — modular Sovereign Core topology + MANDATORY token-reduction stack

> Planning artifact (2026-07-06). Authority: `DECISIONS.md` (D1–D7) + `LEAD-REVIEW.md` (F1–F5).
> This plan does NOT re-plan GRAND-PLAN steps (0b-1…2.4) — it defines (B) the mechanical
> enforcement of the token stack, executed FIRST per operator directive, and (A) the module
> topology those GRAND-PLAN steps land INTO. Strangler, byte-identical, forward-only. No code
> is refactored by this document.

**Ordering (operator-mandated):** Part B ships before any Part A move. B is now AUTONOMOUS work —
the 2026-07-06 operator unlock made `.claude/hooks/` + `.claude/settings.json` agent-editable
(`protect-paths.sh`/`guard-bash.sh` narrowed; `migrations/.env/db/contracts/.github/lockfile` +
human-only `.claude/state` overrides stay protected). Only the CI wiring (`.github/`) remains
operator-apply.

---

# PART B — Token-reduction stack becomes ENFORCED (deny-capable, not advisory)

ANALYSIS.md §C: the compression libraries (`tools/vsa/` codec 34.3% / blind-orch −99.5% /
route/dispatch, distill −92%, TOKEN ROUTER −20…−55%) are BUILT and MEASURED; enforcement is
missing. Error-history class 7 ("discipline-triggered steps die; only hook-enforced artifacts
survive") predicts exactly this rot. Design rules for every gate below, inherited from
`docs/lessons/2026-07-02-gate-state-file-expiry.md` + `scripts/guardrail-gate-armament.mjs`:

1. **Regex/deterministic only — no LLM in any gate.** Pure text/JSON checks on hook input.
2. **Armament, not registration:** every gate ships a hermetic fixture test that proves it can
   DENY *and* that it stays SILENT on the legitimate neighbor case (over-block guard).
3. **One `_hev` JSONL line per decision** (ALLOW/DENY/DEGRADED) to
   `.claude/logs/harness-events.jsonl` — blind-open streaks visible in data (reuse
   `guard-bash.sh` `_hev`).
4. **Any release/override state carries its own embedded expiry**, compared fail-closed against
   wall-clock in the hook — never a cleanup step.
5. **Over-block ⇒ narrow the matching rule; NEVER unregister the gate.**
6. **Graceful degradation:** fail-open ONLY on infrastructure failure (no python3/node, unreadable
   input) and LOG `degraded` — a silent fail-open is the #47 failure mode. Never block the
   operator's own prompt text; gates deny only agent tool calls (Agent-dispatch / Bash).

## Enforcement tier honesty (what is mechanically deniable vs measurable)

| rule | tier | mechanism |
|---|---|---|
| explicit `model:` on every Agent call | **DENY** | B1 hook, field check |
| no Fable for doer-lanes | **DENY** | B1 hook, allowlist + human-only expiring override |
| Explore (not general-purpose) for read-only lanes | **DENY** | B1 hook, self-declared `LANE-CLASS` stamp + consistency check |
| router one-liner + step budget embedded in every dispatch | **DENY** | B1 hook, stamp grep |
| raw JSON >1KB into a prompt without a `route.mjs` verdict | **DENY** | B1 hook, payload scan + `[route:*]` stamp |
| no NEW lanes past session hard-cap | **DENY** | B1 hook, live CTX from transcript |
| 300K session recycle | **DIRECTIVE** (injected, unskippable) | B2 hook (UserPromptSubmit can't block without blocking the human) |
| 80K lane recycle | **DENY if probe passes, else AUDIT** | B2 probe → B1 check; fallback B4 |
| distill noisy command output | **WARN → ratchet to DENY** | B3 PostToolUse, size threshold; promote after false-positive data |
| batch independent calls; distilled returns; deterministic-before-LLM (blind-orch) | **AUDIT** | B4 transcript audit — honest: not judgeable by regex pre-hoc without over-blocking |

## B0 · Baseline before enforcing (S)

- **Scope:** run the B4 audit script (built first, read-only) over the last N session transcripts
  to measure CURRENT violation rates per rule — the deny thresholds and over-block guards are
  tuned on real data, not assumptions (VSA rule 0: measure, don't assume).
- **DoD:** one table appended to `docs/research/token-economy-comparison-2026-07-05.md`:
  violations/day per rule, projected token save, expected deny rate.
- **Gate:** the audit script itself (deterministic, re-runnable, idempotent on same input).

## B1 · `agent-dispatch-gate.sh` — the ONE dispatch gate (M)

- **Hook:** NEW `.claude/hooks/agent-dispatch-gate.sh`, `PreToolUse`, matcher `"Task|Agent"`
  (both names asserted — `scripts/guardrail-hook-matchers.mjs` extended so the matcher can't
  drift when the harness renames the tool).
- **Checks (all on `tool_input` JSON — description/prompt/subagent_type/model):**
  1. `model` absent → **DENY** ("MODEL ROUTING: explicit model: required").
  2. `model` matches `fable` → **DENY** unless `.claude/state/fable-override` contains a
     non-expired line (`<slug>|<unix-expiry>`, expiry embedded, wall-clock compared in-hook,
     fail-closed). `guard-bash.sh` `OVERRIDES` regex extended to include `fable-override` →
     human-only, agent cannot write its own bypass. Honors D4-style per-arc operator overrides
     without reopening the gate permanently.
  3. Prompt lacks a `LANE-CLASS: read-only|write|reasoning` stamp → **DENY** (the lead must
     classify every lane; classification becomes machine-checkable instead of aspirational).
  4. `LANE-CLASS: read-only` AND `subagent_type` ≠ `Explore` → **DENY** (−18.8K/lane, the #2
     ranked sink). `reasoning` lanes require `model: opus` (quality floor, cost never overrides).
  5. Prompt lacks the router stamp (`Token router ON`) or a `step budget` line → **DENY** —
     this is how "distilled returns + cheapest-adequate route" reaches every lane (AGENTS.md
     ops note: subagents don't reliably load AGENTS.md; the stamp IS the delivery mechanism).
  6. Inline JSON payload in prompt > 1KB (fenced block or brace-run heuristic) without a
     `[route: raw|frame|viz <tok>]` stamp → **DENY** with the fix command
     (`node tools/vsa/route.mjs <file>` or `dispatch.mjs --task … files…`). Raw below the
     crossover is CORRECT — the gate enforces that `route.mjs` *measured*, not that framing won.
     This makes VSA codec/dispatch the default composer without ever forcing a net-loss encode.
  7. Live session context (same transcript-parse snippet as `context-budget-guard.sh`) >
     `LANE_HARD_CAP` (default 320K) → **DENY new lane dispatches** ("finish current step,
     persist h_t, handoff") — the mechanical teeth behind the 300K directive, without blocking
     the human (only new fan-out is refused).
- **Fails closed** on its target conditions (missing field/stamp = deny); fails open + logs
  `degraded` only when JSON parsing is impossible.
- **Armament test:** NEW `scripts/guardrail-token-gates.mjs` (same hermetic pattern as
  `guardrail-gate-armament.mjs`: fixture `CLAUDE_PROJECT_DIR` outside the repo, JSON piped in):
  - DENY cases: no model · `model:"fable"` no override · expired `fable-override` line ·
    read-only+general-purpose · missing router stamp · 2KB raw JSON unstamped · fixture 350K
    transcript + Agent call.
  - ALLOW (over-block guards): haiku+Explore+read-only+stamps · opus+reasoning ·
    800-byte inline JSON unstamped (below threshold) · 2KB payload WITH `[route: raw 512]`
    stamp · prose that merely *mentions* "json" (no payload) · non-Agent tools untouched.
  - Asserts one `_hev` line per decision exists.
  **Red→green proof recorded in the ledger row (mandatory, D5).**
- **Effort:** M · **Red-line:** no (harness) · **OP:** no (post-unlock; settings.json edit is
  agent-side, commit-gated).

## B2 · Context-budget guard goes LIVE — 300K session / 80K lane (S)

- **Scope:** the phantom becomes real: `cp docs/operating-model/proposed-hooks/context-budget-guard.sh
  .claude/hooks/` (already red→green proven on a real 507K transcript per its header + ledger row
  81) + register under `UserPromptSubmit` with env tuned to the standing thresholds:
  `CONTEXT_WINDOW=1000000 CONTEXT_BUDGET_PCT=30` (⇒ fires at 300K). Two-tier with B1-check-7:
  300K = injected mandatory wrap-up directive; 320K = new-lane DENY.
- **Lane 80K:** one PROBE first — capture a real hook-input JSON from inside a running lane and
  check whether `transcript_path` resolves to the lane's own sidechain. If YES: add a lane-budget
  branch (deny further Agent/expensive calls inside a >80K lane, directing checkpoint-return).
  If NO (hooks only see the lead transcript): 80K enforcement stays **post-hoc** — B4 flags every
  >80K lane as a `bad` signal into THE EYE's tally, and B1's mandatory `step budget ≤25 calls`
  stamp remains the in-lane brake. The plan does not pretend a hook can see what it can't.
- **Armament:** re-run the guard's existing proof fixtures post-install (fires at threshold,
  silent at window=10M, silent on missing transcript) + one new fixture for the 320K lane-deny.
- **Effort:** S · **Red-line:** no · **OP:** no.

## B3 · Distill-by-default on noisy output (S)

- **Hook:** NEW `.claude/hooks/distill-nudge.sh`, `PostToolUse`, matcher `Bash`. If
  `tool_response` length > 8K chars AND the command didn't already pipe through
  `repowise distill` / `--reporter=list` / an explicit `tail|head` cap → inject a one-line
  directive naming the exact wrapper (`repowise distill '<cmd>'`) + `_hev` WARN line.
- **Why WARN not DENY (degradation by design):** a PreToolUse deny needs to predict noisiness
  from command text — measured false-positive risk is exactly the guard-bash over-block failure
  (an over-broad gate converts to NO gate). PostToolUse measures ACTUAL size — zero false
  positives, but it can only warn after the spend. **Ratchet clause:** after ≥1 week of `_hev`
  data, IF one command-shape repeats un-distilled ≥3× (data, not assumption), that literal shape
  is promoted into a PreToolUse deny-list (narrow rule, per lesson).
- **Armament:** fixture with a 20K-char tool_response → directive emitted; 2K response → silent;
  distilled command with big output → silent (over-block guard).
- **Effort:** S · **Red-line:** no.

## B4 · `scripts/audit-token-router.mjs` — the deterministic post-hoc auditor (M)

- **Scope:** a $0 script (no LLM) over session/lane transcript JSONL that counts, per session:
  (a) Agent dispatches that would have failed B1 (pre-B1 backfill + drift check that B1 stays
  armed); (b) lanes whose cumulative usage crossed 80K; (c) ≥3 consecutive single-tool assistant
  turns whose calls were independent (batching miss heuristic — flagged, human-judged);
  (d) raw >1KB JSON blobs that entered prompts; (e) any `fable` usage. Emits a JSONL report +
  non-zero exit on any (a)/(e) hit.
- **Wiring:** weekly curation pass + THE EYE (`proposed-eye/eye-guard.sh` counts these as `bad`
  signals once applied) + `vsa report` FCE line. This is where the honest-AUDIT-tier rules
  (batching, distilled returns, blind-orch-first) get *measured teeth* — trend must go to zero,
  and a regression is a visible red number, not a vibe.
- **Red proof:** fixture transcript containing one unstamped GP read-only dispatch → exit 1.
- **Effort:** M · **Red-line:** no.

## B5 · Ratchets: registration, circuits, ledger (S)

- Extend `scripts/guardrail-hook-matchers.mjs`: `agent-dispatch-gate.sh` (Task|Agent),
  `context-budget-guard.sh` (UserPromptSubmit), `distill-nudge.sh` (Bash PostToolUse) MUST stay
  registered in `settings.json` — deregistration = red in pre-commit (the #47 anti-pattern:
  removal is the easiest over-block "fix"; this makes it fail loudly).
- Add `guardrail-token-gates.mjs` to pre-commit + the staged CI proposal
  (`proposed-sovereign-core-ci/APPLY.md` — the `.github` wiring itself stays OP).
- KNOWLEDGE-AS-CIRCUITS promotion (mandatory per AGENTS.md): registry entries so
  `run-circuits.mjs` fails on a committed script that dispatches agents without `model:`.
- One REGRESSION-LEDGER row for the whole B-arc with the red→green proofs pasted.
- **Effort:** S · **Red-line:** no.

**Part B exit gate:** `guardrail-token-gates.mjs` green (all DENY + ALLOW fixtures) ·
`guardrail-hook-matchers.mjs` green · B0-vs-post-B1 violation delta measured and appended to the
token report · `_hev` shows real ALLOW/DENY traffic (armed, not just registered).

---

# PART A — Structure upgrade: current → modular Sovereign Core (strangler, byte-identical)

## Current map (verified 2026-07-06)

| unit | where | role today | integrity status |
|---|---|---|---|
| `dowiz-core` | `rebuild/crates/domain` (lib `domain`) | pure kernel: `decide/fold/replay`, money `Lek(i64)`, 10-status machine, codec/request_hash, policy+idempotency | purity mechanically proven (wasm32 + clippy bans); corridors NOT yet composed behind `decide` (0b-3); pricing NOT in (0b-1) |
| Rust shell | `rebuild/crates/api` | axum routes, pg interpreter, auth, ws, jobs, media | monolithic `routes/`; `orders/` = 4,976 lines across 7 files; calls corridor fns individually (until 0b-5) |
| Hub | `rebuild/crates/api/src/routes/orders/channel.rs` (140 lines) | ONE primitive: write-only channel attribution, 13-value allowlist | doctrine-clean (never read by decisions); not yet a named module |
| Legacy shell | `apps/api` (Node) + `apps/web`, `apps/worker` | strangled origin; surfaces already on Rust w/ proofs | must shrink forward-only, never grow |
| Token stack | `tools/vsa/`, `.claude/hooks/`, `scripts/guardrail-*` | built + measured | enforcement = Part B |

## Target (Manifesto §1.2 "Modular Integrity", D2 seams)

Pure `dowiz-core` (no side-effects, no platform vocabulary) ← thin shell adapters (HTTP/pg/ws) ←
hub modules, each ATOMIC with a **manifest/contract**, communicating via **events**
(`Envelope<Event>` through the SyncPort — 0b-2/1.3), so a module fault cannot break the core.
Compiler-enforced direction: core depends on nothing; shell depends on core; modules depend on
core + ports, NEVER on each other's internals.

**Non-goal (deferred as reorg-for-its-own-sake — see the defer table):** moving `rebuild/` to
repo root, one-crate-per-module, any message broker. Modular integrity here = enforced
boundaries + manifests + events, not maximal file motion.

## Moves (each: scope / files / DoD / gate with red proof / red-line / effort)

### A0 · Module manifest contract (S)
- **Scope:** define ONE small manifest schema and stamp the three existing units. `module.toml`:
  `name`, `kind` (`core|shell-adapter|hub-module`), `depends` (explicit allowlist), `events_in`,
  `events_out`, `contract` (doc pointer), `red_line` (bool). Manifests for: `crates/domain`
  (core, depends=[]), `crates/api` (shell-adapter, depends=[domain]), and the channel primitive
  (stamped in A2).
- **Files:** `rebuild/crates/domain/module.toml`, `rebuild/crates/api/module.toml`,
  schema section appended to this doc's folder (`MODULE-CONTRACT` § inside this file or
  GRAND-PLAN — no new doc unless it grows).
- **DoD:** manifests exist, parse, and match reality (declared deps == `cargo metadata` deps).
- **Gate (D5):** `scripts/module-integrity.mjs --schema` (A1's script, schema mode) reds on a
  missing/malformed manifest or a declared-vs-actual dep mismatch. **RED proof:** declare
  `depends=[]` for `api` → gate red (actual dep on `domain` undeclared) → fix → green.
- **Red-line:** no · **Effort:** S
- **STATUS: ✅ DONE 2026-07-06.** `rebuild/crates/{domain,api}/module.toml` stamped + green;
  gated by `scripts/module-integrity.mjs` (below). RED proof recorded live (api `depends=[]` →
  `declared≠actual` red → restore → green). The channel primitive is stamped in A2, not here.

#### MODULE-CONTRACT — the `module.toml` schema (A0)

One flat TOML file per unit, all keys required, machine-checked against reality by
`scripts/module-integrity.mjs`:

| key | type | meaning | checked against |
|---|---|---|---|
| `name` | string | for `core`/`shell-adapter`: the cargo PACKAGE name; for `hub-module`: the module id | must be a real workspace package (crate-level) |
| `kind` | `core \| shell-adapter \| hub-module` | topology role | enum |
| `depends` | string[] | crate-level: workspace PACKAGE names it deps on; hub-module: other module ids it may import | crate-level ⇒ MUST equal `cargo metadata` workspace-internal deps; hub-module ⇒ superset of its `use crate::modules::*` imports |
| `events_in` / `events_out` | string[] | `Envelope<Event>` variants consumed / emitted (empty until 0b-2/1.3) | reserved (schema-only for now) |
| `contract` | string | repo-relative doc pointer describing the behavioural contract | file MUST exist (dangling = rot) |
| `red_line` | bool | changes here need council / invariant-guardian | schema-only signal |

Enforcement tiers: (a) schema + contract-existence + hub-module cross-import → always (cargo-free,
runs in `--self-test`); (b) crate-level `depends`==actual + core ban-list → needs `cargo metadata`
(degrades to a logged SKIP when cargo is absent, like sovereign-gate's cargo-deny). The core ban-list
(`tokio,sqlx,axum,reqwest,rand,chrono,time`) is scoped to PRODUCTION deps only — dev-deps (proptest→rand)
are exempt, mirroring the wasm32 gate's `--lib` scope so the two gates cannot drift apart.

### A1 · `scripts/module-integrity.mjs` — the boundary gate (S)
- **Scope:** deterministic (no LLM) script: (1) parses all `module.toml`; (2) `cargo metadata`
  asserts dependency DIRECTION (`domain` has zero workspace deps; `api → domain` only);
  (3) scans `use` statements across `crates/api/src/modules/*` — any cross-MODULE import not
  declared in the importer's manifest = red (modules talk via events/ports, not internals);
  (4) asserts `domain` deps exclude the 0b-6 ban list (`tokio,sqlx,axum,reqwest,rand,chrono,time`)
  so the structural gate and cargo-deny can't drift apart.
- **Files:** `scripts/module-integrity.mjs` (new); wired into pre-commit + the staged CI job.
- **DoD:** green on current tree; red on injected violations; in pre-commit.
- **Gate (D5):** itself. **RED proofs:** (a) add `use crate::modules::x` across modules
  undeclared → red; (b) add `chrono` to `domain/Cargo.toml` on a throwaway branch → red.
- **Red-line:** no · **Effort:** S
- **STATUS: ✅ DONE 2026-07-06.** Built as a pure core (`checkIntegrity`) + real-tree driver +
  hermetic `--self-test` (14 DENY/over-block assertions, no cargo/fs — the armament caught two real
  parser bugs: inline-comment stripping). Both RED proofs proven: (a) covered by the self-test
  `hub-module imports another module undeclared` case; (b) covered by the self-test `core carries a
  banned production dep (chrono)` case (via synthetic metadata — the real `domain/Cargo.toml` is
  concurrent-staged and MUST NOT be touched). Wired: `--self-test` into `run-armaments.sh` (pre-commit
  1.4d, cargo-free) + real-tree into `.husky/pre-commit` 1.4e (cargo-guarded). CI wiring = staged proposal
  (`.github` operator-gated). NOTE: implemented as one default gate (schema folded in) + `--self-test`,
  not the planned `--schema`/`--self-test` split — the schema check runs unconditionally in default mode.

### A2 · Pilot module: channel attribution (S)
- **Scope:** prove the module pattern on the safest unit: `git mv
  rebuild/crates/api/src/routes/orders/channel.rs → rebuild/crates/api/src/modules/channel_attribution/mod.rs`
  + a one-line `pub use` shim at the old path (strangler — call sites untouched) +
  `modules/channel_attribution/module.toml` (`kind=hub-module`, `depends=[]`, `events_out=[]` —
  it is a pure normalizer; write-only doctrine restated in `contract`).
- **DoD:** byte-identical behavior; shim in place; manifest green; the module dir is the landing
  zone GRAND-PLAN 1.1/1.5 extend (registry becomes `modules/channels_registry/`, a SECOND module,
  when 1.1 executes — with its own council per that step).
- **Gate (D5):** `git diff --find-renames` shows ≥99% rename similarity (mechanical-move proof);
  `cargo test` + `bash rebuild/scripts/sovereign-gate.sh` + `module-integrity` green;
  grep-proof: zero call-site diffs outside the shim. **RED proof:** delete the shim → `cargo
  check` red at every call site → restore.
- **Red-line:** no (attribution is write-only, never read by pricing/state/authz — doctrine
  verified in `channel.rs` header) · **Effort:** S
- **STATUS: ✅ DONE 2026-07-06.** `git mv` → `crates/api/src/modules/channel_attribution/mod.rs`;
  old path `routes/orders/channel.rs` is now an 8-line `pub use crate::modules::channel_attribution::*`
  shim (the single call site `routes/orders/mod.rs:330` byte-identical). `mod modules;` added to
  `main.rs`; `modules/mod.rs` + `channel_attribution/module.toml` (hub-module, depends=[]) stamped.
  **Proofs:** moved `mod.rs` byte-identical to pre-move `channel.rs` (empty diff — the mechanical-move
  proof, stronger than git's rename % which the surviving shim hides); 9 moved unit tests pass; call
  site diff empty; `cargo check -p api` + `module-integrity` green; **RED proof** — blanking the shim
  re-export → `error[E0425]: cannot find function channel_from_header in module channel` at the exact
  call site (line 330) → restore → green. Ledger #87. This dir is the landing zone for GRAND-PLAN
  1.1/1.5 (each its own module + council).

### A3 · Orders module boundary — AFTER 0b-5 + 1.3 land (M)
- **Scope:** once the shell flip (0b-5: handlers pass through the ONE `decide` door) and the
  SyncPort (1.3) exist, split `routes/orders/` along the seam they create:
  `modules/order_intake/` (HTTP + DTO + validation + the channel header fold) vs the pg
  interpreter behind `ports/` (`pg.rs` stays the SQL adapter — SQLSTATE handling untouched).
  Mechanical file motion + shims ONLY; zero logic edits (any logic change belongs to 0b-5 itself
  and its council).
- **Files:** `rebuild/crates/api/src/routes/orders/*` → `modules/order_intake/*` + shims;
  `ports/sync.rs` (1.3's file); manifests.
- **DoD:** grep-proof (0b-5's own gate) still holds — no direct corridor calls anywhere;
  module-integrity green; `/reliability-gate` GO on staging with `x-dowiz-cutover` asserted
  (per F1: ride the existing cutover harness, do not invent a new flip mechanism).
- **Gate (D5):** the three above + rename-similarity proof. **RED proof:** on a proof branch,
  reintroduce one direct `assert_transition` call in `order_intake` → 0b-5 grep-gate red.
- **Red-line:** YES (order-lifecycle + money-path files) — invariant-guardian pass + doubt-pass
  mandatory; council ONLY if the diff is not a pure rename (any behavior delta escalates).
- **Effort:** M · **Hard dependency:** 0b-5 + 1.3 merged. Do not start earlier.

### A4 · Legacy `apps/api` strangler ratchet (S)
- **Scope:** freeze the Node shell forward-only: snapshot the current route-registration count
  into `scripts/legacy-api-baseline.json`; new guardrail `scripts/guardrail-legacy-freeze.mjs`
  reds when the count INCREASES (decrease auto-rebaselines). No file moves, no deletions — the
  rebuild cutover plan owns retirement; this only prevents regression into the strangled shell.
- **Gate (D5):** the guardrail in pre-commit. **RED proof:** add a dummy route registration to
  `apps/api` on a throwaway branch → red.
- **Red-line:** no · **Effort:** S
- **STATUS: ✅ DONE 2026-07-06.** `scripts/guardrail-legacy-freeze.mjs` — counts Fastify route
  registrations in `apps/api/src` (method call + leading-`/` string path; excludes tests/scripts) and
  reds on INCREASE. Baseline `scripts/legacy-api-baseline.json` = **237** across 68 files (the JS
  match-count; the shell's real leaf-route surface). Decrease → `--update` ratchets the floor down.
  Hermetic `--self-test` (regex counts routes not `Map/params.get`; grew→red / shrank / frozen). RED
  proof: a dummy `apps/api` route → `237 → 238 +1` red (names the file) → remove → green. Wired:
  `--self-test` in `run-armaments.sh` + real check in `.husky/pre-commit` 1.4f. Ledger #88.

### A5 · Placement rule for all Phase-1/2 hub work (paper, S)
- **Scope:** one binding convention added to GRAND-PLAN's conventions block: every new hub
  feature (1.1 registry, 1.5 channels surface, 2.1 distribution, 2.4 aggregator-view trait)
  lands as `crates/api/src/modules/<name>/` WITH a manifest, consuming/emitting only
  `Envelope<Event>` via ports — never importing another module's internals. The GRAND-PLAN
  steps keep their own scopes, gates, and councils (1.1/1.2/2.2/2.3 stay council-gated as
  written there); this move only pins WHERE they land, and A1's gate enforces it mechanically.
- **Gate:** `module-integrity.mjs` (already armed by A1). **Red-line:** no · **Effort:** S
- **STATUS: ✅ DONE 2026-07-06.** Binding convention added to `GRAND-PLAN.md` §Conventions ("Module
  placement (BINDING — STRUCTURE-UPGRADE A5)"): every new hub feature lands as
  `rebuild/crates/api/src/modules/<name>/` with a manifest, `Envelope<Event>` via ports only, no
  cross-module internals — `module-integrity.mjs` enforces it. `modules/channel_attribution/` (A2) is
  the reference shape. Paper move; the gate is A1's (already armed).

**PART A EXIT (2026-07-06):** A0 · A1 · A2 · A4 · A5 all ✅ DONE + gated. A3 (orders split) stays
BLOCKED on GRAND-PLAN 0b-5 + 1.3 (hard dependency — do not start early; it sits on the #1 failure root).
→ Next: the red-line **money boundary** (GRAND-PLAN 0b-1 — extract `pricing.rs` with the f64-boundary
split, decorrelated hand-derived oracle) in a FRESH session.

## Deferred (YAGNI — each with its unlock trigger, D6 discipline)

| deferred move | why deferred | unlock trigger |
|---|---|---|
| Hub modules as separate CRATES (`crates/hub-*`) | dirs + manifest + gate give the same boundary at ~0 cost; crates add Cargo ceremony per module | module-integrity shows a recurring cycle, OR a module needs an independent wasm/release artifact |
| Move `rebuild/` to repo root / retire the prefix | pure churn; breaks every doc pointer, gate path, CI glob for zero coupling reduction | Node `apps/api` fully retired (cutover complete) |
| In-process event BUS abstraction (beyond SyncPort) | one port with 2 impls (1.3) IS the seam; a bus with 1 producer/1 consumer is theater | ≥2 real independent event consumers exist (e.g. Canvas subscriber, Phase 3) |
| Per-module process isolation / broker | manifesto's "fault can't break core" is satisfied by purity + dependency direction at MVP scale | measured fault blast-radius incident that purity + refusal semantics didn't contain |
| `domain` crate split (money/kernel/codec crates) | one pure crate is the RIGHT atom now; splitting multiplies gate surface | compile time or ownership pain measured, not predicted |

---

# Sequencing

```
B0 → B1 ─┬→ B2 → B3 → B4 → B5   (Part B first — mandatory token stack; ~2-3 lane-days)
         │
         └→ (Part B exit gate green) → A0 → A1 → A2 → A4 → A5   (S-moves, parallel with GRAND-PLAN 0b)
                                              A3 ⟵ blocked on GRAND-PLAN 0b-5 + 1.3
GRAND-PLAN 1.1/1.5/2.x land INTO the A5 placement rule under A1's gate.
```

Every step: feature branch, ship discipline, red→green proof pasted, ledger row. F5 applies:
execute in fresh sessions per arc, resuming from this doc + DECISIONS + GRAND-PLAN — B2's guard
makes that self-enforcing, which is why B runs first.

# Self-critique (how this plan could break)

- **B1 is one gate with 7 checks — the highest over-block risk in the plan.** If its payload
  heuristic (check 6) or stamp greps misfire on legitimate dispatches, the historical failure
  mode is wholesale unregistration (#47). Mitigations are structural (ALLOW fixtures in the
  armament suite, `_hev` data, narrow-never-remove rule, B0 baseline tuning) — but the first
  week needs active watching of the DENY log.
- **Stamps can be cargo-culted.** `LANE-CLASS`/router stamps prove classification HAPPENED, not
  that it's CORRECT — a lead can stamp `write` on everything and route around Explore. B4's
  audit catches drift post-hoc; only culture + THE EYE catch it live. Honest limit, stated.
- **The 80K lane cap may be un-enforceable at hook level** (transcript visibility probe may
  fail) — then "mandatory" quietly means "audited," which is weaker than the operator asked.
  The plan says so explicitly rather than promising a hook that can't see.
- **A-moves are byte-identical by design, but A3 sits on the project's #1 failure root** (live
  ≠ edited model): a rename that subtly changes module init order or router registration IS a
  behavior change invisible to `cargo test`. That's why A3 rides `/reliability-gate` + the
  existing cutover harness (F1) and is blocked on 0b-5 — resisting the urge to move files early
  is the single most important sequencing rule in Part A.
- **Manifest drift = new paper-gate risk.** `module.toml` files are exactly the kind of artifact
  that rots into decoration; the ONLY defense is that A1's gate diffs them against `cargo
  metadata`/`use` reality in pre-commit — if that gate is ever soft-skipped, the manifests are
  worse than nothing (false confidence).
- **Big-bang temptation is displaced, not eliminated:** the deferred table (hub crates, event
  bus, root move) will resurface each Phase-1 step as "while we're here." Every one must lose
  in review until its written trigger fires.
- **Part-B-first has a real cost:** 2-3 lane-days of harness work before any core/product motion,
  on a startup whose #1 risk is over-engineering. Justified only because the same enforcement
  pays for every subsequent lane (measured −20…−55% + −18.8K/lane) — if B stretches past ~3
  days, cut B3/B4 to WARN/backlog and ship B1+B2+B5 only.
