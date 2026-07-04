# Token-Reduction Synthesis — the reduce over 7 teardowns + the harness audit

> **Priority: #1 (operator, 2026-07-04) — reduce agent token consumption.** This is the REDUCE over
> the map phase (7 external-repo teardowns + a quantified harness audit), following the just-codified
> agentic map-reduce rule. Numbers from `2026-07-04-harness-token-audit.md` (chars/4 heuristic; two
> directly-measured anchors). Each teardown is its own dossier in this dir.

## 1. Where the tokens actually go (audit, ranked)

| Rank | Cost line | Measured / est. | Lever | Est. saving | Path |
|---|---|---|---|---|---|
| 1 | **Subagent dispatch floor** ~42K/lane (only ~8.5K is CLAUDE.md+MEMORY.md; rest = tool-schema/MCP overhead from broad grants) | MEASURED; ~714K of 2.57M session (~28%) | Narrow-tool agents (not `general-purpose`); trim unused global MCP connectors | ~20-30K/lane on read-only lanes | 🔒 protected (agent defs + how lead dispatches) |
| 2 | **`pre-edit-lessons` hook** 464 injections / 2.5d | est. | Narrow the two broad triggers (`docs/**`, `packages/db/migrations/**`) | ~40-70K / window | 🔒 `.claude/hooks` |
| 3 | **`route-request.sh` nudge** 383 fires / 2.5d (194 serious + 189 repeat) | counted | Context-aware fire (the harness's own P3 — still unshipped) | ~15-25K/session | 🔒 `.claude/hooks` |
| 4 | **Memory-corpus duplication** ~146KB / ~36.5K tok in 6 clusters | est. | Merge superseded files (information-preserving) | ~36.5K off recall path | ✅ non-protected (memory/ not in git) |
| 5 | **REGRESSION-LEDGER.md** 28,352 tok/full read, read wholesale by librarian + pattern-critic; append-only (grows) | MEASURED size | Compact index (grep instead of full-Read) | ~25K per critic invocation | 🔒 governance doc |

## 2. External teardowns — verdicts (the map results)

| Repo | License | Egress | Verdict for token goal | Action |
|---|---|---|---|---|
| **agentmemory** (rohitg00) | Apache-2.0 ✅ | clean | Value = 5 patterns; vendoring means running a daemon | **LEARN** — apply patterns natively |
| **agentfiles** (Railly) + skillkit | MIT ✅ | clean | Measurement layer (metrics/health/token-spend) from logs we already have | **APPLY** — 3 zero-dep scripts (in flight) |
| **pxpipe** (teamchong) | MIT ✅ | SAFE (local proxy; no exfil) | Exact Fable-5 fit (`PXPIPE_MODELS`), renders bulk context as images — but **LOSSY on exact strings** (hashes/IDs); one documented silent failure | **PILOT dark, gated** to non-red-line lanes only |
| **codegraph-rust** (Jakedismo) | ❌ no LICENSE (conflicting) | safe to run local | Flagship tools SPEND tokens (2nd LLM); cheap structural layer not exposed; redundant w/ repowise except Rust-tree indexing | **BAKE-OFF pending** (prove-or-reject, task #18) |
| **halo/HALO** (context-labs) | ❌ unresolved (issue #34) | clean | Harness QA/observability — **wrong domain**, doesn't address tokens | **SKIP** (learn the trace-diagnostic governance pattern only) |
| **future-agi** | pending | pending | pending | **PENDING** (task #17) |

Cross-cutting: agentmemory (active context-shrinking) + agentfiles (cheap continuous measurement) are
**complementary, not redundant** — measure first, then shrink. codegraph & halo both blocked by absent
licenses regardless of merit. pxpipe is the only tool that could cut Fable-5 tokens directly, but its
lossiness collides with our Mandatory-Proof / red-line invariants — hence gated-pilot, never default.

## 3. APPLIED this pass (non-protected, via ethics/quality check)

1. **Memory-corpus dedup** — librarian lane merging the 6 confirmed clusters, information-preserving,
   red-line memories untouched (secrets-incident / dev-login-backdoor / money-RLS-PII stay individual).
   [outcome folded in on lane return]
2. **3 measurement scripts** (`scripts/memory-metrics.mjs`, `context-tax.mjs`, `memory-health.mjs`) —
   zero-dep, read-only, mine existing logs/transcripts. Makes future token decisions data-driven and
   re-runnable (catches drift), instead of one-off audits. [committed on lane return]
3. **agentmemory patterns** adopted into the map-reduce rule already (distilled returns = the
   context-shrink pattern; narrow grants). Write-time dedup + decay-sweep = future librarian upgrades.

## 4. PROPOSED (protected — operator applies; before→after)

1. **Narrow-tool agent grants** (win #1, ~28% of session tokens). Default read-only/search lanes to
   `Explore` or a scoped tool set; reserve `general-purpose` ("All tools") for write+build+test lanes.
   Also audit global MCP connectors — every connector's schema loads into every lane's floor.
   Before: ~42K/lane. After: ~12-20K/lane on read-only lanes.
2. **`pre-edit-lessons` trigger-narrowing** (win #2). Scope the `docs/**` + `migrations/**` triggers to
   the specific paths a lesson actually guards. Before: 464 fires/2.5d. After: target ≤ ~100.
3. **`route-request.sh` context-aware nudge** (win #3 = the harness's own unshipped P3). Fire only on
   real red-line/repeat. Before: 383 fires/2.5d. After: ~cut 250 low-signal.
4. **CLAUDE.md + AGENTS.md map-reduce insert** — `proposed-claude-md/agentic-map-reduce-rule.md`.
   Makes the distilled-return + narrow-grant discipline universal (compounds #1).
5. **REGRESSION-LEDGER compact index** (win #5) — a generated one-line-per-row index the critics grep,
   leaving the 28K full ledger read only when a specific row is needed.
6. **pxpipe dark pilot** — external local sidecar, `PXPIPE_MODELS=claude-fable-5`, wired via
   `ANTHROPIC_BASE_URL` for non-red-line reasoning sessions ONLY. Never for money/auth/RLS/migrations/
   bulk-edit or any session needing verbatim hash/ID recall. Measure real savings before trusting.

## 5. Pending fold-ins
- **codegraph bake-off** (task #18) → settles the Rust-indexing item; the cheaper alternative is
  "get repowise to index `rebuild/crates/`" (repowise confirmed blind to the Rust tree).
- **future-agi** (task #17) → additive-vs-redundant vs the agentfiles measurement layer.

## 6. Note on this pass's own cost
The map phase (7 teardowns) itself spent tokens — the honest ledger: the reduce's applied wins
(#4 memory dedup + measurement) plus the protected proposals (#1-#3, the big ones) must clear that
outlay over subsequent sessions. #1 alone (28% of every session) does so quickly if applied.
