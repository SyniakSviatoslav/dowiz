---
trigger: always_on
description: Living-memory-graph protocol — navigate via the code graph, not grep; prune tool output; recall typed memory. Mandatory for all agents (incl. Hermes).
---

## Living-memory graph (always-on)

The living memory graph lives in `dowiz-core` (`crates/dowiz-core/src/`): `living_memory`
(typed memory), `code_graph` (navigation), `context_pruner` (token saving). Use it
instead of raw grep/file reading whenever a graph is present.

### 1. Navigate via code_graph, not grep
- For "where is X / what calls Y / how are A and B related": query `code_graph`
  (`shortest_path`, `subgraph_around`, `neighbors`, `pagerank`) — a scoped subgraph
  is far smaller than raw grep output.
- Only fall back to grep/glob when the graph does not surface enough.

### 2. Prune tool output before it enters context
- Run `context_pruner::prune_lines` (Text/Search/Diff/Log) on large tool results;
  keep only high-priority lines (error/warn/security/importance/markdown).
- This is the headroom token-saving core — always on before injecting results.

### 3. Recall typed memory, don't re-discover
- `living_memory`: episodic (what happened), semantic (what is), procedural (how),
  short-term, long-term. `recall`/`search`/`recent` before re-exploring.
- Anchor facts to code nodes (`link_to_code`) so memory is co-located with code.

### 4. Commit regularly (standing rule)
- Commit + push at every milestone/checkpoint; run only tests for changed files.
