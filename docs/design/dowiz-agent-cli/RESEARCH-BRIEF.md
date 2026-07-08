# Research Brief — "Own Agent CLI" for dowiz (a la Claude Code / Hermes)

> Author: Hermes Agent (operator directive 2026-07-08: "researxch" first). Status: RESEARCH ONLY.
> No code written. Goal: ground the feature, surface the decision fork, hand back a go/no-go.
> Governing rule: ground truth over proxy — every claim below cites a real file or a verified source.

## 0. The question being researched
Build an agent CLI "just like you or Claude Code" for the dowiz project setup — one that runs the
dowiz Operating System (ethics charter, red-lines/invariants, Verified-by-Math, token router, model
routing, knowledge-as-circuits, loops) as NATIVE behavior, not as a prompt someone has to remember.

## 1. What already exists in THIS repo (do not rebuild it)
- **OpenCode is already installed + configured here.** `.opencode/opencode.json` loads two plugins;
  `.opencode/plugin/karpathy-guards.ts` is a *native* model-agnostic guard that enforces
  **P1 plan-gate** (plan.jsonc required before file-mutation tools) and **P3 scope-block**
  (edits outside `scope.jsonc` are rejected). `.opencode/scope.jsonc` already scopes the protected
  surface (migrations, contracts, routes, websocket, ui, web). This is the OS already running as
  hooks — the hard part is mostly done.
- **Claude-Code operating core** (`.claude/CLAUDE.md`, `AGENTS.md`): the full rule set. Plus 8
  enforcement hooks in `.claude/hooks/` (protect-paths, red-line-doubt-gate, guard-bash,
  agent-dispatch-gate, post-edit-gates, distill-nudge, subagent-return-guard, context-budget/require-classification).
- **Loop harness** (`tools/loop-harness/`, `docs/operating-model/living-loop-system-v3.md`): the
  only-sane way a loop runs — breaker, telemetry, §5 report, lossless storage. ~30 loop specs in `loops/`.
- **Token economy stack**: `tools/vsa/` (VSA1 frames, route.mjs crossover-aware, match.mjs recall),
  `repowise` (code-intel MCP, structural graph of the whole repo), `spikes/living-knowledge/`
  (deterministic recall@5=1.0 retriever — the §0·GP engine), `codebase-memory-mcp`.
- **Cross-agent mesh** (`scripts/agents-mesh.sh`, `scripts/hermes-fallback.sh`): ordered fallthrough
  Hermes→OpenCode→Goose→Aider→OpenHands + Hermes-fallback. Shared credential pool at `~/.hermes/.env`.
- **Prior agentic plan**: `docs/agents/INTEGRATION-PLAN.md` — 8-phase plan to stand up a live
  agentic system (loop core, triad, council, hooks) on a `feat/agentic-system` branch.
- **Tooling registry**: `TOOLING-REGISTRY.md` — OpenRouter (LLM) + local Ollama embeddings
  (qwen3-embedding:0.6b, 1024d) + Repowise + mempalace, all on one 7.6 GiB box.
- **Rust core**: `rebuild/crates/domain/src/kernel/` already has `decide`, `validate`,
  `idempotency`, `pricing` — the "one door" pattern. An agent CLI that routes intents to
  `kernel::decide` mirrors the product's own architecture (doors carry Commands, never invent money).

## 2. The decision fork (the only thing that changes the build)
| Option | What it is | Build cost | Risk | Fit to "own + OS-native" |
|---|---|---|---|---|
| A | **From scratch**: our own binary owning the tool loop, model routing, hooks, baking the OS in. | High (months). Re-implement REPL, streaming, tool protocol, subagents, context mgmt. | Re-building solved problems; bugs in the loop = agent goes feral. | Max ownership, max surface to get wrong. |
| B | **Adopt + harden OpenCode** (already configured): bake OS in as plugins/hooks/skills. Extend, don't reinvent. | Low–Med. Plugins already exist; add circuit-gate + token-router + VbM hooks; port `.claude` hooks to OpenCode equivalents. | Locked to OpenCode's model/tool abstractions; must track upstream. | OS-native via plugins; OpenCode already the chosen substrate. |
| C | **Thin `dowiz-agent` wrapper**: our own CLI entry-point that orchestrates an underlying agent (OpenCode/Claude) but injects OS + tokens + memory + loops as a native layer. | Med. Owns the *shell + policy*, delegates the *loop* to OpenCode via SDK/CLI. | Two layers to keep in sync; wrapper drift. | Owns brand/entry/policy; loop borrowed. Good middle. |
| D | **Research only** (this doc). | — | — | n/a |

## 3. External landscape (verified 2026-07-08)
- Open-source coding agents, self-hostable: **OpenCode** (terminal-first, plugin+hook system in
  TS, provider-agnostic), **Aider** (diff-based, git-native, legacy model), **Goose** (Block,
  tool-ecosystem), **Cline** (VS Code ext), **Plandex**, **Gemini CLI / Codex CLI** (vendor-first).
  OpenCode leads the dedicated-agent space and is already in this repo.
- The *internals* of any such agent are the same skeleton: **(1) system prompt + rules, (2) tool
  definitions (read/edit/run/grep), (3) an LLM-call loop that appends tool results and re-calls,
  (4) subagent fan-out, (5) hooks on tool events, (6) context/compaction management.** Building A
  means re-implementing all six; Options B/C reuse them.
- The differentiator the operator already wants is NOT the loop — it's the **OS baked in**: red-line
  gates that *deny*, VbM proofs that *fail red*, token router that *picks the cheapest adequate model*,
  circuits that *mechanically block* known-bad patterns. Those are **hooks + guards**, which every
  agent above (and OpenCode specifically) already supports. So the novel work is the *guard library*,
  not the *agent*.

## 4. Recommendation (for the operator to confirm — not executed)
**Option B (harden OpenCode) as the substrate, with a thin Option-C `dowiz-agent` shim for entry +
policy if a branded binary is wanted.** Rationale, all grounded:
- The OS is ALREADY running as OpenCode plugins (`karpathy-guards.ts`). Re-implementing the loop
  (Option A) throws that away and re-buys solved bugs.
- The valuable, un-duplicated work is the **guard/circuit library** (red-line deny, VbM self-test,
  token-router model pin, scope-block, subagent-return-guard) — portable across B and C, and mostly
  already present in `.claude/hooks/` + `docs/operating-model/circuits/registry.json`.
- Concretely, the build = port `.claude/hooks/*` + circuits into `.opencode` plugins, add a
  `dowiz-agent` wrapper that sets `ANTHROPIC_BASE_URL`/router env, loads `AGENTS.md`+`HERMES.md`, and
  wires `tools/vsa` + `spikes/living-knowledge` into the tool-pre-step. Same pattern as
  `scripts/agents-mesh.sh` already uses.

## 5. Open questions for the operator (per ask-don't-guess)
1. Fork: A / B / C / D? (Recommend B+C-shim.)
2. If B/C: is OpenCode the permanent substrate, or should the shim also support driving Claude Code?
3. Branding: do you want a literal `dowiz-agent` binary on PATH, or is "OpenCode + our plugins" enough?
4. Scope gate: which globs are red-line-protected for the agent (mirror `scope.jsonc` + `INVARIANTS.md`)?
5. Model routing: is the Haiku-default / opus-red-line policy (AGENTS.md MODEL ROUTING v3.4) the
   enforced default for the agent too?

## 6. Verification plan (when code starts — VbM)
- **Red-line deny RED case**: agent attempts to edit `packages/db/migrations/*` without gate → blocked.
- **VbM self-test RED case**: a planted all-green guardrail → `guardrail-falsifiable-proof.mjs` fails.
- **Token router proof**: opus call on a non-red-line step → assert model!=opus (grep session ledger).
- **Scope-block RED case**: edit outside `scope.jsonc` → hook throws.
- Each ships GREEN + RED per `docs/operating-model/verified-by-math.md`.

## 7. Sources
- Repo: `.opencode/opencode.json`, `.opencode/plugin/karpathy-guards.ts`, `.opencode/scope.jsonc`,
  `AGENTS.md`, `.claude/CLAUDE.md`, `.claude/hooks/*`, `docs/operating-model/*`, `docs/agents/INTEGRATION-PLAN.md`,
  `TOOLING-REGISTRY.md`, `tools/vsa/*`, `spikes/living-knowledge/*`, `rebuild/crates/domain/src/kernel/*`.
- Web (2026-07-08): OpenCode docs (plugins/hooks/agents); awesome-cli-coding-agents; coding-agent
  internals deep-dives (Raschka; cefboud OpenCode deep-dive); open-source coding-agent comparisons.
