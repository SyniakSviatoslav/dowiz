# Reverse-engineering the integrated tools → meta-loop upgrade insights (2026-06-29)

Studied the integrated harness tools' *mechanisms* (not adopted as deps — out-of-tree) to extract
transferable patterns for the autoupgrade / Living-Loop meta-loop (`loops/autoupgrade.yaml`,
`tools/loop-harness/src/autoupgrade.ts`). Each insight is grounded against the loop's own
`build_status.deferred` + run-#5 `watch`.

| Tool | Mechanism (reverse-engineered) | Insight → meta-loop upgrade |
|---|---|---|
| **EvoMap** (GEP) | Capabilities are versioned, auditable, *validated-before-inherit* reusable assets ("Genes/Capsules") shared across agents | **Proven-upgrade asset registry.** A kept Class-A upgrade should become a versioned "gene": `{patch, oracle before/after metric, revert record, provenance}` in a durable, append-only ledger — so a validated speedup is auditable + replayable, not re-discovered each run. (Today: kept upgrades aren't persisted as reusable assets.) |
| **STORM** | Perspective-guided question-asking + simulated multi-expert conversation → multi-angle coverage | **Multi-perspective oracle/CLASSIFY.** Evaluate each candidate through decorrelated lenses (perf · security · reversibility) before KEEP — mirrors the council's cause/pattern/ratchet critics. Hardens the §2 oracle against single-lens blind spots. |
| **DeerFlow** | Sandbox-aware execution + sub-agent spawn + message gateway + memory for long-horizon tasks | **Worktree-sandboxed APPLY-VERIFY + concurrency.** The deferred "worktree concurrency upgrade" + "runtime sandbox": run candidate apply/benchmark in an isolated git worktree, parallelize independent candidates. (build_status: teamConcurrency=1 today.) |
| **Scrapling** | Adaptive, resilient fetch (single request → full crawl) | **Contained web-research fetch.** The deferred RESEARCH phase ("contained web-research") gets a resilient fetch layer **behind the scraping-conduct gate** (`scripts/scrape-pilot/scraping-conduct-attest.mjs`) — web is untrusted *data*, never executed (iron principle already present). |
| **Decepticon** | Generates an RoE/ConOps/OPPLAN engagement package + deconfliction BEFORE any action; sandbox-net isolated | **Pre-run engagement contract.** Each loop run emits, up front, its boundary declaration + revert plan + scope as an artifact (formalize FIRM-BOUNDARY-STOP / ORACLE-GATE like an OPPLAN) — making "act only within declared rules" a checked artifact, not just runtime code. |

## Cross-cutting
The strongest convergent insight (EvoMap + Decepticon + the loop's own design): **a self-evolving loop is
trustworthy only when every change is a validated, auditable, reversible asset bounded by a declared
engagement contract.** dowiz's loop already has the bones (oracle KEEP-or-rollback, firm-boundary, Class-B
human queue, §5 report). The upgrades above *deepen* that spine — they do NOT loosen any firm boundary.

## Constraints for the upgrade (non-negotiable)
- Never auto-mutate auth/RLS/secrets/payments/PII/schema/migrations/architecture (firm boundary).
- One-change-at-a-time; proven-speedup-or-rollback; no-fake-green; report-only widens only after clean runs.
- The upgrade itself goes through **loop-architect** (M1–M11 + anti-cheat dry-run) and stays CERTIFIED.

## Reverse-engineering the PROJECT's own integrated tools (2026-06-29 round 2)
Studied the in-repo harness to map insight → where it ALREADY lives vs where to strengthen.

| Project tool | Mechanism | Insight convergence |
|---|---|---|
| `tools/loop-harness` (autoupgrade/Living-Loop) | MAP→CLASSIFY→oracle KEEP-or-rollback→§5 report; firm boundary; Class-B human queue | The spine. v0.2 added the EvoMap gene-ledger + STORM lenses — the genuinely-new external insights, now applied. |
| `tools/ccc` | AST symbol index → "where is X" at a fraction of grep tokens; secret-safe walker | **Token-efficiency** (the user's standing goal). Already built; UNDER-used by agents → propagate the *habit* (ccc/repowise skeleton before grep/full-read), don't rebuild. |
| `tools/skillspector` | Security scanner for agent skills (detect vuln/malicious before install) | = **scan-before-install** (EvoMap "validate-before-inherit" / Decepticon pre-engagement). Already the gate; ensure the skill-adoption flow routes through it. |
| `tools/eslint-plugin-local` (21 rules) + `scripts/guardrail-*` (7) | Mechanical red→green gates | = auditable, deterministic "engagement contract" enforcement. All 7 guardrails now wired into verify:all (orphan fixed this session). |
| council critics (cause/pattern/ratchet) + review agents (security-sentinel, invariant-guardian) | Decorrelated, perspective-specific review | = STORM multi-perspective, ALREADY embodied. The v0.2 lens pattern is the same idea, now also in the autoupgrade oracle. |
| `docs/{regressions,lessons,reflections}` | Ledger + lessons + reflections self-improvement stores | = EvoMap auditable-reversible-asset evolution, ALREADY the project's spine. The gene-ledger is its machine-loop sibling. |

**Convergence finding:** the harness is already well-architected around exactly these patterns — the external
tools mostly **validate** the design rather than reveal gaps. The one genuinely-new mechanism (proven-upgrade
gene-ledger + decorrelated-lens oracle) is now applied (v0.2). So project-wide propagation is **targeted
reinforcement, not a rewrite** (rewriting working skills/agents = bloat, against the ponytail/anti-slop rule).

## Propagation map (targeted — what actually changes)
- **Docs (this round):** this doc records the analysis; the convergence + the gene-ledger/lens pattern are
  now the documented reference for future loop work.
- **`.claude/` agents + skills are protect-paths** → concrete enhancement proposals are STAGED for operator,
  not edited here: `docs/operating-model/harness-insight-propagation-PROPOSALS.md`.
- **No mass edit** of the ~80 skills/agents or the product code — only the specific, high-value targets in
  the proposals doc, each justified.

## Recommended first upgrade (highest leverage, smallest blast radius)
**EvoMap-insight A — the proven-upgrade asset registry** + **STORM-insight B — multi-perspective oracle**:
both strengthen the *trust* spine (auditability + decorrelated validation) without touching firm
boundaries or enabling broader auto-apply. The worktree/sandbox/web-research insights (C/D) are larger and
should follow only after several clean report-only runs (per the loop's own widen-after-clean rule).
