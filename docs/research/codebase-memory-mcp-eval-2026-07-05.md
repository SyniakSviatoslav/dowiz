# codebase-memory-mcp vs the existing graph/memory layers — eval + decision (2026-07-05)

Operator top priority: make agents read the codebase **structurally/visually, not file-by-file**.
Researched [codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp) (MIT, DeusData),
compared to what the repo already has, integrated it, and (below) measured the real token reduction.

## What the repo ALREADY had (three graphs, none delivering the goal)

| Layer | Nodes | State | Renders? | Fatal gap for the goal |
|---|---|---|---|---|
| **graphify** (`graphify-out/`) | 10,832 | **STALE** — Windows paths `C:\Users\Dell5\…`, commit 76cb64ec, last run 2026-05-27 | No renderer | Stale + never rendered; queried as text |
| **repowise KG** (`.repowise/`) | 2,097 | index ~2026-06-14 | Served JSON, not image | Semantic/summary layer DEFERRED (429 rate-limits → empty `summary` fields); the "intelligence" half is unbuilt |
| **oh-my-mermaid** (`.omm/`) | 1 perspective | manual | **Yes** (mermaid→GitHub) | Only `api-routes`; hand-authored per view; pilot, not in CI |
| markdown vault + lessons/loops | — | manual | No | Pure file-by-file reads |
| **VSA** (`tools/vsa/`) | — | fresh | No | Compresses DATA payloads, does not map code |

The capability existed in three places but none was **fresh + whole-repo + Rust-aware + rendered +
LLM-independent** at once.

## codebase-memory-mcp — what it adds

- **Fresh, in one command:** re-indexed the WHOLE repo in **8.2s** → 32,544 nodes / 86,983 edges /
  2,647 files / 8,368 functions / 585 routes / 91 enums. `detect_changes` maps a git diff to affected
  symbols, so it stays current cheaply.
- **Rust-aware** (Hybrid LSP): 117 Rust files parsed — the rebuild's `rebuild/crates/api` is in the graph
  (graphify/repowise's Rust coverage was never established; this project is now half-Rust).
- **No LLM / no 429:** deterministic tree-sitter+LSP graph. repowise's semantic layer died on free-tier
  rate limits; this needs no model to be useful.
- **Actual visual:** `codebase-memory-mcp --ui=true --port=9749` = a 3D interactive graph explorer (the
  literal "read visually"). Plus `get_architecture` = a 9k-token holistic map of the entire codebase.
- **14 query tools** replacing file reads: `search_graph`, `trace_path` (BFS call chains), `query_graph`
  (Cypher-like), `get_code_snippet`, `get_graph_schema`, `detect_changes`, `search_code`, `get_architecture`,
  `manage_adr`, `ingest_traces`, + index mgmt. MIT, single static binary, sub-ms queries, storage in
  `~/.cache/codebase-memory-mcp/` (SQLite WAL, + optional `.codebase-memory/graph.db.zst` for team sharing).

## Decision

**ADOPT** — registered in `.mcp.json` as `codebase-memory`. It **supersedes stale graphify** for
structural navigation and fills repowise's dead semantic gap. Kept alongside: **repowise** (its
lancedb embeddings power `search_codebase` semantic search + `get_why` archaeology — a different job),
**oh-my-mermaid** (curated publishable diagrams), **VSA** (data-payload token compression). Three
complementary token-economy layers now: VSA compresses DATA, codebase-memory compresses CODE-STRUCTURE
navigation, repowise answers WHY.

## Real token-reduction probe (A/B, actual harness tokens)

Two identical Explore agents, SAME structural question — "trace the S5 `create_order` call chain and
list the (a) repo method, (b) pricing/money functions, (c) request-hash/idempotency functions with
files." Both produced an **equivalent, correct** answer (same repo method, same pricing chain
compute_order_pricing→compute_line_total / apply_tax / charged_tax / delivery_fee_for_order→resolve→
distance_km / compose_total, same request_hash + idempotency_decision).

| arm | subagent tokens | tool uses | how it answered |
|---|---|---|---|
| file-by-file (grep + Read) | **90,976** | 6 | read mod.rs, pg.rs, pricing.rs, request_hash.rs, state.rs |
| **codebase-memory graph (CLI)** | **41,047** | 19 | search_graph + trace_path + query_graph, no source reads |
| **saved** | **49,929 = 54.9%** | | equivalent answer |

**Real-harness reduction: ~55%** on this structural question. This is LOWER than the vendor's headline
"99%" because that figure is isolated *query tokens vs file bytes*; this A/B measures the WHOLE subagent
cost — agent reasoning + per-CLI-call tool overhead + results — which is the honest, apples-to-apples
number for how an agent actually works. The graph arm used 19 exploratory calls (a practiced agent that
knows the qualified names uses far fewer → the reduction grows toward the vendor number as usage
tightens). Net: a real ~55% cut today, equivalent accuracy, and it climbs with targeted queries — plus
it never touches an LLM (unlike repowise's dead semantic layer) and stays fresh in 8s.

Method note: same measuring stick both arms (Agent-tool usage meters), same question, verified-equivalent
answers — the same rigor as the VSA A/B (`tools/vsa/README.md`).

## Apply / reverse-engineered findings

1. **Convention (AGENTS.md):** for STRUCTURAL questions (what calls what, where is X, routes, call
   chains, blast radius) query `codebase-memory` FIRST; only Read files for the specific bytes a query
   points you to. Same discipline as the VSA "frames for data" rule.
2. **Freshness:** `detect_changes` after edits (cheap) or a full re-index (8s) keeps it live — unlike
   graphify which rotted. Wire a re-index into the loop-harness/CI later.
3. **Retire the stale graphify graph** from the retrieval decision tree (MEMORY-MAP.md) — it points at a
   Windows-path graph from May; codebase-memory replaces it.
4. **Optional visual:** enable `--ui=true` for the 3D explorer when a human wants to SEE the graph.
