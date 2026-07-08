# dowiz / DeliveryOS — Project View (operating-system orientation)

> Written 2026-07-08 by Hermes Agent after reading the operating core, living memory,
> loops, agents, skills, hooks, circuits, and the sovereign-core plan. Terminal-friendly
> digest of *how this repo actually runs its agents*. (Full detail lives in the files cited.)

## 1. What this repo is
A multi-tenant delivery/ordering platform. Stack: Astro/React frontend (`apps/web`),
Node API (`apps/api`), worker (`apps/worker`), Postgres/Supabase, plus a Rust/WASM
**Sovereign Core** (`dowiz-core`) being built under `rebuild/crates/*` and
`docs/design/sovereign-core-mvp/`. Main product surface is an owner data-hub / checkout /
order-lifecycle engine. Branch in play: `feat/sovereign-core-phase-zero`.

## 2. The "DOVS" — its operating system (there is no file literally named DOVS)
You asked for the decision/operating/value system. It exists as a *layered stack of
enforced machinery*, not a single doc. In priority order (lower = overrides higher):

  ETHICS CHARTER  (AGENTS.md / .claude/CLAUDE.md) — non-negotiable, overrides all.
    └─ No AI for warfare; peace; AI as a commons.
  PRODUCT RED-LINES / INVARIANTS  (docs/agent-rules/INVARIANTS.md, ADR-0012)
    └─ 12 gated invariants: integer money, FORCE-RLS, forward-only migrations,
       no raw SQL, no PII egress, RS256 JWT, require-auth-hook, error-envelope, etc.
       Each row = a real executable gate, not prose.
  VERIFIED-BY-MATH (VbM, 2026-07-07)  — universal validation rule
    └─ works? proven-with-math? falsifiable? Ship the RED case with the GREEN.
  §0·GP GROUND-TRUTH over PROXY (model-agnostic-playbook.md)  — governs everything
    └─ deterministic check WINS over any proxy (no standing council/critic anymore).
  AGENT OPERATING MODEL  — two speeds, one boundary (agent-operating-model.md)
    └─ recon(spike/challenge, relaxed) vs execution(build/audit, full discipline);
       red lines hold in BOTH; honest FAIL = success.
  TOKEN ROUTER + MODEL ROUTING v3.4  — cheapest ADEQUATE route; Haiku default doer,
       opus only for red-line reasoning; Fable OFF everywhere; explicit model: per lane.
  KNOWLEDGE-AS-CIRCUITS + THE EYE  — lessons become mechanical gates, not advice.
  Ponytail  — lazy-senior YAGNI mode (deletion over addition).

This stack IS the DOVS. The "value system" = ground truth over proxy, honest red over
green, integer money, tenant isolation, human authority on deliver. The "decision system"
= the gate/hook/harness chain below. The "operating system" = the loop/agent/skill mesh.

## 3. Layers (what's actually wired, not aspirational)

A. ENFORCEMENT HOOKS (`.claude/hooks/`, live in settings.json)
   - protect-paths.sh     (PreTool Edit/Write) — blocks edits to red-line globs
   - red-line-doubt-gate.sh (PreTool Edit/Write) — prompts on money/auth/RLS/migration edits
   - guard-bash.sh        (PreTool Bash) — sandbox/safety gate
   - agent-dispatch-gate.sh (PreTool Agent/Task) — DENIES model-less dispatch; opus red-line rail
   - post-edit-gates.sh   (PostTool Edit/Write) — runs the 12 INVARIANTS red_lines() checks
   - distill-nudge.sh     (PostTool Bash) — nudges repowise distill on noisy output
   - subagent-return-guard.sh (PostTool Agent/Task + SubagentStop) — catches 0-tool-use degenerate lanes
   - context-budget-guard.sh / require-classification.sh — token + classification gates
   These fire on Claude Code. For Hermes, the same rules are encoded in HERMES.md + the
   cross-agent mesh + guardrail scripts under scripts/.

B. GUARDRAIL ARMAMENTS (scripts/guardrail-*.mjs) — run in pre-commit via run-armaments.sh
   falsifiable-proof, token-gates, no-set-cookie, owner-active-membership, ledger-integrity,
   loop-registry-parity, license, legacy-freeze, subagent-return-guard, etc.
   Each is itself falsifiable (--self-test proves it flags an all-green proof).

C. CIRCUITS (docs/operating-model/circuits/registry.json) — run by scripts/run-circuits.mjs
   Machine-readable error-patterns/lessons. Seeded: money-no-float-in-core (red-line, Rust core),
   no-raw-any-ts (warn), no-process-exit-ts (warn), rls-force-on-enable (red-line),
   no-removed-machinery-loops/skills (red-line — bans references to deleted proxy gates:
   council/invariant-guardian/security-sentinel/serious-gate/design-council).
   RED-LINE → exit 2; warn → exit 1. Promotion mandatory: a qualified lesson MUST become a circuit.

D. LIVING-KNOWLEDGE RETRIEVER (spikes/living-knowledge/) — the §0·GP engine
   `node search.mjs "<q>"` → deterministic file lookup over the harness corpus.
   recall@5 = 1.000 on a 29-query oracle (vs 0.621 pure-vector). Any model consults it
   BEFORE acting — same files for every model = model-agnostic ground truth.
   `eval.mjs` (offline, RED/GREEN) + `selftest.mjs` (sabotage → proves checks redden).

E. VSA TOKEN ECONOMY (tools/vsa/) — data-compression layer (34.3% aggregate)
   -1 inversion-of-control (don't send state to LLM if code can decide); route.mjs picks
   frame/raw/crossver; match.mjs for recall-before-LLM; viz.mjs for state→image decision-support.
   Telemetry ledger in tools/vsa/telemetry/usage.jsonl.

F. LOOP SYSTEM (docs/operating-model/living-loop-system-v3.md + tools/loop-harness/)
   Every loop (audit-gate, autoupgrade, triage, demo-builder, skill-evolution, …) runs
   ONLY through the harness. Contract: goal/iterate/progressMetric/reflect/isTerminal.
   Breaker (no-progress K-trip), telemetry (tokens/eco/code), §5 LOOP REPORT always printed
   to terminal, permanent lossless storage (loops/runs/). ~30 loop specs in loops/*.yaml.
   Cross-agent mesh: scripts/agents-mesh.sh (Hermes→OpenCode→Goose→Aider→OpenHands ordered
   fallthrough) + scripts/hermes-fallback.sh (Claude-outage → Hermes via HERMES.md).

G. AGENTS (`.claude/agents/`): loop-architect, playwright-test-{planner,generator,healer}.
   COMMANDS (`.claude/commands/`, 14): audit-gate, build-stage, converge-loop, incident,
   investigate, loop-orchestrator, opsx/*, perf, refactor-converge, regression-hunt, exit-audit.
   SKILLS (`.claude/skills/`, 67): supabase, playwright-cli, tdd, systematic-debugging,
   subagent-driven-development, stop-slop, doubt-escalation, reliability-gate,
   openspec-*, frontend-design/*, vercel-react-best-practices, etc.
   (Hermes-side skills live in ~/.hermes/skills/. I load them per AGENTS.md instinct.)

H. MEMORY (living, outside repo): /root/.claude/projects/-root-dowiz/memory/
   MEMORY.md index + per-arc files (ATTIC for closed topics). Mirrored into HERMES.md by
   scripts/sync-memory-to-hermes.mjs so Hermes sees the same operating rules + memory digest.

## 4. Current state (from session handoff + PROGRESS.md)
- Reliability Gate L0–L11: PASS (5 parallel audits); 2 critical bugs fixed (courier channel,
  today's-counts filter). Staging deployed (v266, health 200). Prod merge DEFERRED — MVP ~40%,
  500+ git conflicts, red-line phases (persistent event log 1.2, checkout 2.2) not started.
- Sovereign Core 0b-1/0b-2/0b-3 DONE+PUSHED (money boundary, event vocab/Envelope,
  `decide` composes machine→actor-gate→cc1→pricing, core invents NO money number).
  NEXT = 0b-4 (Hard Truth L1–2) then keystone 0b-5 (shell flips to `kernel::decide`, red-line).
- Branch: 12 commits ahead of origin/feat/sovereign-core-phase-zero. typecheck green.
  Unit 1217/1300 (1 pre-existing unrelated fail).

## 5. How I (Hermes) will operate here
1. Read-first: read the file/graph bytes before editing; existing files win.
2. Honor red-lines: auth / money / RLS / packages/db/migrations/ / bulk-edit = STOP and ask,
   do not bypass. Any edit to those globs needs an explicit human gate per change.
3. Verified-by-Math on every change: ship the RED case with the GREEN; falsifiable proof.
4. Token router: deterministic code before any LLM; graph/skeleton-first; distilled returns;
   explicit model routing (haiku doer / opus only on red-line reasoning).
5. Ground truth over proxy: prefer a re-read of real bytes / failing test over any opinion.
6. Ship discipline: feature branch → staging deploy → tests/Playwright proof → prod only on
   explicit approval. Commit messages contextual; pre-commit gates must pass.
7. Honest red = success: surface blockers/red findings with evidence, never hide to go green.

## 6. Fast-start commands (real, from the repo)
  pnpm lint | pnpm typecheck | pnpm build | pnpm format
  pnpm verify:rls | verify:migrations | verify:secrets | verify:privacy | verify:error-contract
  node spikes/living-knowledge/search.mjs "<q>"
  node tools/vsa/cli.mjs encode|match|pe <file>
  npx tsx tools/loop-harness/src/cli.ts finalize --record run.json --base loops/runs ...
  bash scripts/agents-mesh.sh --dry-run "<task>"
  bash scripts/deploy-staging.sh
