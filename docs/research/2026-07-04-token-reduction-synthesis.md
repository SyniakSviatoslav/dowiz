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
| **codegraph-rust** (Jakedismo) | ❌ no LICENSE (conflicting) | safe to run local | **BAKE-OFF PROVEN: loses 0/7 vs repowise 3/7-exact.** No query interface without `ai-enhanced` (2nd LLM/query); license blocks vendoring; its one edge (Rust indexing) absorbed by repowise which already ships the Rust stack | **NO-GO — do not integrate.** Cheaper win: point repowise at `rebuild/crates/` (13.8s, 0 LLM tokens) |
| **halo/HALO** (context-labs) | ❌ unresolved (issue #34) | clean | Harness QA/observability — **wrong domain**, doesn't address tokens | **SKIP** (learn the trace-diagnostic governance pattern only) |
| **future-agi** (future-agi) | Apache-2.0 ✅ | benign (SDK phones usage home if pip-installed; we read source only) | LLM observability/eval platform; 2 borrowable metric patterns for our own scripts | **BORROW-PATTERNS** — not a dependency |
| **caveman** (JuliusBrussee) | MIT ✅ | safe (2/3 zero-net; /compress calls Anthropic w/ denylist, disclosed) | 3 lossy compressors: (a) terse-output skill = redundant w/ our distilled-return + adds ~1-1.5k/turn; (b) MCP tool-desc regex-compressor = novel angle but redundant w/ narrow-grants + harness-deferred schemas, unmeasured; (c) LLM markdown-rewrite = DANGEROUS on normative docs (structure-only validator) | **MOSTLY REDUNDANT/SKIP** — (a) don't install, (c) NEVER on CLAUDE.md/MEMORY.md/regressions/adr; (b) = a proposed measure-first IF narrow-grants prove insufficient |
| **latentgraph** (LatentForce-ai) | MIT (thin client only; engine = paid SaaS) | **DISQUALIFYING** — uploads full source+git to latentforce.ai; daemon accepts server-initiated run_bash "auto-approval no UI"; self-patches .claude/settings.json + CLAUDE.md w/ injection-shaped text | Paid cloud code-graph, 9 tools ≈ repowise 1:1 → redundant (codegraph NO-GO precedent) AND a source-exfiltration + remote-shell risk | **SKIP OUTRIGHT** — no integrate/pilot/trial (the trial IS the exfiltration). Bank only the local PreToolUse-blast-radius UX idea for a future in-house build vs repowise |

Cross-cutting: agentmemory (active context-shrinking) + agentfiles (cheap continuous measurement) are
**complementary, not redundant** — measure first, then shrink. codegraph & halo both blocked by absent
licenses regardless of merit. pxpipe is the only tool that could cut Fable-5 tokens directly, but its
lossiness collides with our Mandatory-Proof / red-line invariants — hence gated-pilot, never default.

## 3. APPLIED this pass (non-protected, via ethics/quality check)

1. **3 measurement scripts SHIPPED + proven** (`scripts/{memory-metrics,context-tax,memory-health}.mjs`,
   commit `cbf0d088`) — zero-dep, read-only, reproduce the audit numbers exactly; re-runnable so they
   catch drift, not one-off. These are the durable win: they made the effort data-driven and
   immediately **caught a ~27× overclaim** (below).
2. **Memory-corpus dedup — DOWNGRADED after content verification.** The librarian ran real Jaccard
   (vs the audit's filename grouping): only **2 of 6 clusters genuinely redundant → ~1,330 tokens**
   reclaimable, NOT the estimated ~36,500. 4 clusters REFUTED (distinct sessions / live reference
   hubs / the in-flight orchestration doc — merging would bury reusable pointers). Fact-preserving
   merge drafts exist in scratchpad; **deferred as marginal** (1.3K tok vs the churn/wikilink-repoint
   risk on my own memory) — available if the operator wants it. Lesson: verify redundancy by content,
   not filename, before deleting memory.
3. **agentmemory patterns** adopted into the map-reduce rule already (distilled returns = the
   context-shrink pattern; narrow grants). Write-time dedup + decay-sweep = future librarian upgrades.
4. **Live finding from the scripts**: `pre-edit-lessons` fires 479/481 from the SINGLE `docs/**`
   trigger → strengthens protected proposal #2 (that one trigger narrowing is nearly the whole hook
   cost). And the biggest oversized memory file is this session's own `rebuild-decision-rust-astro`
   (6,243 tok) — consolidate at session close.

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

## 5. Settled + pending fold-ins
- **codegraph bake-off** (task #18) → **SETTLED: NO-GO** (0/7 vs repowise 3/7; no query path without a
  2nd LLM; license blocks vendoring). Actionable replacement: **point repowise at `rebuild/crates/`** —
  it already ships `tree_sitter_rust` + resolver; test indexed the 56-file tree in 13.8s / 0 LLM
  tokens. ⚠️ do NOT use `repowise init <path>` — it silently rewrote global `~/.claude/settings.json`
  in the bake-off (caught + reverted); find the non-destructive scope-add path or propose to operator.
  Two real repowise gaps logged as separate follow-ups (symbol-qualified `get_context` targets;
  cross-package caller undercounting).
- **future-agi** (task #17) → SETTLED: borrow-patterns. Two additive, non-protected `scripts/` follow-ups
  (logged, not yet applied): (1) a `token_source` provenance tag (MEASURED/estimated/fallback) on
  `scripts/exec-telemetry.mjs`'s schema — codifies the audit's prose distinction; (2) auto-emit per-lane
  `subagent_tokens` into the telemetry emitter (replaces one-off diagnostic probes like the ~42K-floor
  finding). Idea 3 (SSG registry-of-checks) deferred — SSG's own no-new-dep philosophy.

## 6. Note on this pass's own cost
The map phase (7 teardowns) itself spent tokens — the honest ledger: the reduce's applied wins
(#4 memory dedup + measurement) plus the protected proposals (#1-#3, the big ones) must clear that
outlay over subsequent sessions. #1 alone (28% of every session) does so quickly if applied.
