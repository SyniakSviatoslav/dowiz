# codegraph-rust teardown — license-first reverse-engineering dossier

**Scope:** does `Jakedismo/codegraph-rust` (1) reduce token consumption for agentic code work, and (2)
integrate into dowiz — especially `rebuild/crates/` (Rust)? Clone kept out-of-tree at
`/tmp/.../scratchpad/codegraph-rust-teardown/` per the skill-adoption guardrail; nothing landed under
`/root/dowiz` except this doc.

**Verdict: LEARN, DO NOT INTEGRATE (yet).** License is under-specified (no LICENSE file — see below),
which alone blocks vendoring/depending. Architecturally additive to repowise in one dimension
(Rust-native structural precision, richer edge types, offline embeddings) but the "agentic tools"
layer that is its main selling point makes its own LLM calls per query — the opposite of what we
need for token reduction. Operational cost (separate SurrealDB process, per-language LSP servers,
LLM API keys) is heavy for a 56-file/26k-LOC Rust tree. Re-evaluate once (a) upstream fixes the
license, and (b) `rebuild/crates/` is 10x its current size.

---

## 1. Repo identification

Searched GitHub for "codegraph-rust" and close variants. Six "codegraph"-named projects exist; the
literal, exact-name match and best fit for "Rust tool that builds a code graph for agent context" is:

**Picked: [`Jakedismo/codegraph-rust`](https://github.com/Jakedismo/codegraph-rust)**
- 824 stars, 75 forks, 34 open issues, no releases/tags cut
- Created 2025-09-12, last push **2025-12-20** — no commits in ~6.5 months as of today (2026-07-04).
  Active-looking history (59+ PRs merged) but currently dormant, not "actively maintained."
- 100% Rust, workspace of 14 crates. Description (verbatim): "100% Rust implementation of code
  graphRAG with blazing fast AST+FastML parsing, surrealDB backend and advanced agentic code
  analysis tools through MCP for efficient code agent context management."
- What it actually does, per its own README: transforms a codebase into a knowledge graph (AST +
  type-aware enrichment via LSP + embeddings) stored in SurrealDB, exposed to AI coding agents
  (Claude Code, Codex, Cursor, Gemini CLI) via 4 MCP "agentic" tools that each run their own
  reasoning loop over the graph.

**Runner-ups (one line each, GitHub metadata as of 2026-07-04):**
| Repo | Lang | License (GH-detected) | Stars | Note |
|---|---|---|---|---|
| `suatkocar/codegraph` | Rust | MIT (proper LICENSE file) | 10 | Smaller/newer; 32 langs, 44 tools; actually has a license |
| `anvanster/codegraph` | C | Apache-2.0 | 15 | Graph DB for code relationships, not Rust-primary despite name |
| `colbymchenry/codegraph` | TypeScript | MIT | 57,517★ / 3,539 forks | Huge traction (created Jan 2026) but wrong language for this ask — not Rust |
| `codegraph-ai/CodeGraph` | C | Apache-2.0 | 34 | 42 MCP tools, VS Code extension, 38 languages |
| `websines/codegraph-mcp` | Rust | **none detected** | 13 | Rust, but tiny and also license-less |
| `CodeGraphContext/CodeGraphContext` | Python | MIT | 3,871 | Popular, but Python, not Rust |

`Jakedismo/codegraph-rust` remains the correct pick: it is the exact name asked for, is Rust-native
(not just Rust-adjacent), and is the most architecturally complete of the Rust-primary options — but
note `suatkocar/codegraph` is a credible, actually-licensed Rust alternative worth a 15-minute look if
this space gets revisited later.

---

## 2. License — FIRST, and the loudest flag in this dossier

**No LICENSE file exists anywhere in the repository.** Verified two ways:
- GitHub's own license detector returns `licenseInfo: null` for the repo (`gh api repos/Jakedismo/codegraph-rust --jq .licenseInfo` → `null`).
- A full recursive tree listing (465 files, non-truncated) contains zero files matching
  `licen*`/`copying*`/`notice*` in any casing, anywhere, including inside `crates/*`.

What *is* present is inconsistent and non-binding:
- `README.md:516-518` — a bare `## License` heading followed by the single word `MIT`. No copyright
  holder, no year, no full MIT text, no SPDX block.
- Root `Cargo.toml:29` — `license = "MIT OR Apache-2.0"` (workspace default).
- But five of the fourteen crates **override** the workspace default to `Apache-2.0`-only:
  `codegraph-mcp-autoagents`, `codegraph-mcp-daemon`, `codegraph-mcp-rig`, `codegraph-mcp-server`,
  `codegraph-mcp-tools` — i.e. exactly the crates implementing the MCP server / agentic-tools layer
  that would be the integration surface. The README's "License: MIT" is thus not even accurate for
  the part of the codebase we'd care about.

**Verdict by use:**
- **(a) Reading/learning from the source** — fine. Reading code to learn architecture/patterns is not
  a licensing act; nothing in copyright law requires a license to *read* public code. Proceed freely
  (this dossier already does).
- **(b) Vendoring / depending on it (as a crate dep, git submodule, or copied code)** — **blocked**.
  A `## License: MIT` one-liner in a README, with no LICENSE file and an internally inconsistent
  `Cargo.toml` license field across crates, is not a legally reliable grant. `cargo` and crates.io
  registries treat the `license` field as metadata, not as the license text itself — there is no
  actual instrument here a downstream user could point to. Given dowiz's own open-source posture
  (ADR-020, AGPLv3 target) and its history of licensing/secrets caution (see
  `docs/research/oss-teardown-research.md` precedent of license-first dossiers), do not add this as
  a dependency, do not copy code from it, until upstream either (i) adds a real `LICENSE`/`LICENSE-MIT`
  +`LICENSE-APACHE` file matching what `Cargo.toml` claims, or (ii) the five Apache-only crates get
  reconciled with the dual-license claim.
- **(c) Running its binary locally against our repo (as an external dev tool, not vendored)** —
  **acceptable, with the scan below satisfied first.** Running a third-party CLI/MCP server against
  your own machine (analogous to running any other open-ish tool) does not require a perfected
  license the way redistribution or embedding does; the author's clear (if sloppily documented)
  intent is permissive. This is the "learn" lane, not the "integrate" lane — no code from
  codegraph-rust would ship inside dowiz.

**This is flagged loudly per the task's instruction**, not because the license is restrictive
(GPL/AGPL/proprietary) but because it is **absent** — which is a worse position for due diligence
than a known-restrictive license, since there is nothing formal to rely on at all.

---

## 3. Pre-run scan (scan before running, per skill-adoption guardrail)

Clone location: `/tmp/claude-0/-root-dowiz/d1b102b0-0624-452a-9e2d-72865dd2e9d8/scratchpad/codegraph-rust-teardown/`
(shallow clone, read-only, nothing executed against `/root/dowiz`).

**Verdict: SAFE TO RUN LOCALLY as a dev tool (not vendored). No blockers.**

- **Install scripts (6, all read not executed):** none do `curl|bash` of remote code. The four
  `install-codegraph*.sh` variants just `brew install surrealdb/tap/surreal` + `cargo install --path
  crates/codegraph-mcp-server` (local compile, no sudo). `setup-build-optimization.sh` installs
  `sccache` from crates.io and **appends to `~/.zshrc`/`~/.bashrc`** and writes `$HOME/.cargo/config.toml`
  (backs up existing) — writes outside the repo to `$HOME`/shell rc files, but content is benign
  build-cache env vars, no exfiltration. `verify-setup.sh` is read-only health checks against
  `127.0.0.1:3004`.
- **Credentials/telemetry:** `.env.example` expects only self-hosted/provider keys
  (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `JINA_API_KEY`, `XAI_API_KEY`) plus local SurrealDB/Ollama/
  LM Studio URLs. Zero telemetry/analytics/phone-home code found (grep clean for posthog/segment/
  sentry/mixpanel across docs/ and crates/). Every hardcoded URL in `crates/` classifies as either an
  expected LLM/embedding provider (`api.anthropic.com`, `api.openai.com`, `api.x.ai`, `api.jina.ai` —
  each in the relevant provider adapter file) or `localhost` (Ollama/LM Studio). One inert placeholder
  string (`crates/codegraph-core/src/config.rs:361`, `github.com/your-repo/codegraph-rust`) written
  into a generated local README — not a network call. No unexplained/suspicious domains.
- **Filesystem scope — one non-blocking footgun:** per-project index metadata is written to
  `<project_root>/.codegraph/` (good — scoped), but the actual SurrealDB backing store defaults to a
  **global, shared** path (`$HOME/.codegraph/surreal.db`) with a static default namespace/database
  (`"ouroboros"/"codegraph"` in `crates/codegraph-graph/src/surrealdb_storage.rs:46`, or `"main"/
  "codegraph"` per `.env.example:33-34`) that is **not derived from project path/ID**. Indexing more
  than one repo without manually changing `CODEGRAPH_SURREALDB_NAMESPACE`/`DATABASE` would commingle
  graphs from different projects in one local DB. Not malicious, just a footgun to configure around.
- **Build-time execution:** none — no `build.rs` anywhere in `crates/`, no `[build-dependencies]`,
  no in-repo proc-macro crates.
- **Dependency red flags:** `vendor/rmcp` and `vendor/semchunk-rs` are excluded from the workspace but
  actually resolve from crates.io in `Cargo.lock` (no patch redirect to an unofficial fork). One git
  dependency outside crates.io — `autoagents`/`autoagents-derive` from
  `github.com/liquidos-ai/AutoAgents`, pinned to a fixed commit in `Cargo.lock`, optional, gated
  behind the `autoagents-lats` feature only pulled in by the "full features" install script. Pinned,
  not floating — moderate, not blocking.
- **`.dockerignore.security`:** a genuine security-hardened dockerignore (excludes `*.key`/`*.pem`/
  `id_rsa*`/`.ssh`/`.env*`/secrets) sitting alongside a normal `.dockerignore` — suggests security
  awareness, but since Docker only honors the default-named file, this stricter one may not actually
  be enforced unless renamed. Hygiene note, not a vulnerability.

---

## 4. Architecture (reverse-engineered)

14-crate Rust workspace. Pipeline: `source → tree-sitter AST → (optional) LSP enrichment → graph +
embeddings in SurrealDB → 4 MCP "agentic" tools`.

- **Graph model.** Nodes are `CodeNode` (`crates/codegraph-core/src/node.rs`) keyed by
  `NodeType` = `Function, Struct, Enum, Trait, Module, Directory, Variable, Import, Class, Interface,
  Type, Other` (`types.rs:60-73`) — i.e. it models **symbols and modules, not raw files**. Edges
  (`EdgeType`, `types.rs:76-87`) = `Calls, Defines, Uses, Imports, Extends, Implements, Contains,
  References, Other`. The live SurrealDB schema stores `edge_type` as a free string and its functions
  reference a richer vocabulary than the Rust enum — including Rust-local dataflow edges
  (`defines/uses/flows_to/returns/mutates`) and `depends_on/exports/reexports`
  (`schema/codegraph.surql:26`). Each node optionally carries an `embedding: Vec<f32>` and a
  `complexity` score.
- **Parsing.** Pure **tree-sitter** — no `syn`, no rust-analyzer-as-library. 11 grammars wired in
  `crates/codegraph-parser/src/language.rs::LanguageRegistry::new()`: rust, typescript, javascript,
  python, go, java, cpp/c, swift, c#, ruby, php. Kotlin and Dart are present in marketing but
  **commented out** in code (tree-sitter version conflicts, `language.rs:84,117`). Separately,
  `crates/codegraph-mcp/src/analyzers/lsp.rs` is a real **LSP client** that spawns `rust-analyzer`/
  `typescript-language-server`/`pyright`/`gopls`/`jdtls`/`clangd` as child processes to enrich
  symbols/definitions — this runs only on the `balanced`/`full` indexing tiers, not the default
  `fast` tier.
- **Storage.** SurrealDB, but **a separate server process, not embedded by default**. Default
  connection is `ws://localhost:3004` (`crates/codegraph-graph/src/surrealdb_storage.rs:43-44`); the
  install flow requires you to `surreal start … file://$HOME/.codegraph/surreal.db` first. Only the
  `kv-mem` SurrealDB feature is compiled in (no `kv-rocksdb`/`kv-surrealkv`), so persistence is
  delegated entirely to the externally-launched `surreal` process. Vector search uses SurrealDB's
  **native HNSW** index (per embedding dimension 384–4096) plus BM25 full-text — **not** FAISS. Note:
  `crates/codegraph-vector`'s `PersistentVectorStore` is described as FAISS-backed but is a
  brute-force stub with placeholder vectors, referenced only by its own tests (dead code); the live
  path is SurrealDB HNSW.
- **Query interface.** 4 consolidated MCP tools (`crates/codegraph-mcp-server/src/official_server.rs`):
  `agentic_context` (search/build/answer), `agentic_impact` (dependency + call-chain), `agentic_architecture`
  (structure + api-surface), `agentic_quality` (complexity/coupling/hotspots) — each with a `focus`
  sub-param, returning **JSON** (summary, analysis, file:line highlights, risks, next_steps,
  confidence). Underneath sits a non-MCP-exposed structural toolset (`get_dependencies, detect_cycles,
  call_chain, coupling_metrics, hub_nodes, semantic_search, complexity_hotspots`) that the agent
  executor drives internally. **There is no ad-hoc CLI query command** — the `codegraph` binary only
  does `index / estimate / start / daemon / config / status`; you cannot ask "who calls X" from the
  shell, only through the MCP+LLM agent path.
- **LLM/context-sizing.** The "context window awareness" is real (`crates/codegraph-mcp-core/src/
  context_aware_limits.rs`): a hard `MCP_MAX_OUTPUT_TOKENS = 52_000` cap and a 4-tier system
  (Small <50K / Medium / Large / Massive) that scales retrieval limits (10/25/50/100 results) and
  prompt verbosity to the configured LLM context window. **But the flagship agentic tools are LLM
  callers, not pure retrievers** — `execute_agentic_workflow` builds an LLM provider (Anthropic/
  OpenAI/Ollama/LM Studio) and runs a ReAct/LATS loop that calls out to that model to synthesize the
  JSON answer. This is **feature-gated behind `ai-enhanced`, which is NOT in the default feature set**:
  without it, all 4 tools return `"Agentic tools require the ai-enhanced feature to be enabled"`
  (`official_server.rs:1104-1116`). Out of the box the headline capability is inert until the operator
  compiles with `ai-enhanced` and supplies LLM API keys.
- **Multi-language reality (checked, not assumed).** Genuinely multi-language *input* — 11
  tree-sitter grammars with per-language extractor modules, and an integration test exercising
  extension→language detection for all 11. But conversion-depth is uneven: full AST→CodeNode
  conversion tests exist only for **Rust, TypeScript, Python**; Go/Java/C++/Swift/C#/Ruby/PHP
  extractors have zero `#[test]`s. So for dowiz's two languages (TS + Rust), TS and Rust are among the
  best-tested paths — a point in its favor.
- **Maturity.** Active architectural churn (default agent backend just switched AutoAgents→Rig; the
  8→4 tool consolidation is a breaking change in "Unreleased"). Multiple stubs on non-core paths
  (cross-encoder reranker is keyword-matching sim + dead code; LATS executor is "skeleton with
  placeholder logic"; TS semantic analysis is a TODO; daemonization "requires fork", not done; TLS
  "detected but not implemented"). E2E coverage of the flagship tools is thin *by construction* —
  tests mostly assert the tools are correctly **disabled** without `ai-enhanced`, and graph-storage
  tests soft-skip when no live SurrealDB is present. No releases/tags cut; 34 open issues; dormant
  since 2025-12-20.

---

## 5. Token-reduction analysis (priority goal)

**The core tension: codegraph-rust's flagship layer SPENDS tokens, it doesn't save them.** Its 4
headline tools (`agentic_*`) each run a server-side ReAct/LATS agent that calls an external LLM to
plan graph queries and synthesize a prose answer. That is a *second* model invocation stacked under
your primary agent — the opposite of the "small structured result instead of reading files" lever we
want. The part that would actually reduce tokens is the **lower-level structural graph queries**
(`get_dependencies`, `call_chain`, `coupling_metrics`, `hub_nodes`, `semantic_search`) which return
compact JSON. But those are **not exposed as MCP tools** by default — they are internal to the agent
executor. And if you compile *without* `ai-enhanced` to avoid the extra LLM cost, the only exposed
MCP tools return `"requires ai-enhanced"` errors. So the structural, token-cheap path exists in the
code but is not reachable as a standalone query surface without patching the tool router.

**Where a graph query genuinely beats grep+read (the real prize):**
| Workflow question | Subagent-with-grep cost | Graph-query cost |
|---|---|---|
| "all callers of `settle_order`" | grep the token repo-wide, open each hit to confirm it's a call not a comment/string, read surrounding fn | one reverse-`Calls`-edge query → list of `file:line` symbols |
| "impact radius if I change `Money`'s repr" | trace usages transitively by hand across files | transitive `Uses`/`depends_on` closure in one call |
| "what does `routes/orders` depend on" | read the module + follow imports | one out-edge query |

For a large tree this is a real multi-thousand-token saving per question. **But dowiz already has this
capability via the repowise MCP** — and repowise is served, always-on, and free of a second LLM call:
- `get_context(targets, include=["callers","callees"])` → import+call rollup (structural, no LLM).
- `get_risk(targets, changed_files=[...])` → blast radius / co-changes / "will_break" directive —
  precisely the impact-radius query above.
- `search_codebase` / `get_answer` → concept→file, with a `grep_hint` when identifier-shaped.

**Additive vs redundant — honest verdict: mostly REDUNDANT with repowise for the token-reduction
goal, with one narrow additive edge.**
- **Redundant:** the "who-calls / what-depends / impact-radius returned as a small structured result"
  value proposition is already delivered by `get_context(callers)` + `get_risk`, which cost us zero
  extra infra and zero extra LLM calls. codegraph-rust's `agentic_*` tools would actually *increase*
  token/compute cost per query (they invoke an LLM), and its structural tools aren't exposed
  standalone. Adopting it to answer these questions would be paying more to duplicate what we have.
- **The one additive edge — and it's a real gap:** **repowise's index currently has ZERO coverage of
  the Rust rebuild tree.** Verified empirically this session: `get_context` on
  `rebuild/crates/api`, `rebuild/crates/domain`, and `rebuild/crates/api/src/routes` all returned
  *"Target not found"*, and `get_overview` lists only TS/JS layers, TS entry points, and TS churn
  modules — the repowise parser pipeline here is TypeScript/JS-native (its `state.json` phase timings
  show `tsconfig`, `graph.ts_index`, `graph.heritage` — all TS-shaped) and does not index `.rs`. So
  for the Rust rebuild specifically, codegraph-rust's tree-sitter-Rust + rust-analyzer path would give
  *precision repowise cannot currently provide at all*. That is the only place it is genuinely
  additive rather than redundant.
- Caveat even there: at today's size (`rebuild/crates/` = 2 crates, 56 `.rs` files, ~26k LOC incl.
  tests), the whole Rust tree fits in a couple of `get_context`/Read passes. A code graph earns its
  keep on trees too big to hold in context; the rebuild tree is not there yet. The token-reduction
  win from indexing 26k LOC of Rust is small in absolute terms today.

---

## 6. Applicability + integration path

**(a) Against `rebuild/crates/` (Rust) — the only place it fits.** It would index our axum/sqlx tree:
tree-sitter-rust for AST, `rust-analyzer` for symbol/definition resolution (LSP tier), producing
call/impl/import/dataflow edges. This is real capability repowise does not currently offer for Rust.
But the current tree is tiny (56 files) — the graph would be built and re-built faster than it saves.
The applicability grows *only if* the rebuild tree grows large (the strangler-by-surface plan implies
it will, over many surfaces). Best treated as a "revisit when `rebuild/crates/` crosses ~50k–100k LOC
across many crates" option, not a now-tool.

**(b) Against the TS `apps/` + `packages/` (the bulk of dowiz).** codegraph-rust *does* support TS/JS
input (tree-sitter-typescript + typescript-language-server enrichment, and TS is one of its 3
best-tested conversion paths). So technically it could index our ~1,300 TS/TSX files. **But here it is
purely redundant with repowise**, which already indexes exactly this surface and serves it always-on.
No reason to stand up a second, self-hosted, LLM-calling code-graph for TS when repowise covers it.

**(c) Integration shape (if ever adopted — for the Rust tree only).** Three options, cheapest first:
  1. **Standalone offline structural query, no LLM** — the token-honest shape. Would require a small
     upstream patch to expose the internal structural tools (`call_chain`, `get_dependencies`, etc.)
     as MCP tools *without* `ai-enhanced`, or to add a CLI query subcommand (which doesn't exist
     today). A subagent calls `codegraph <query>` and gets compact JSON. This is the only shape that
     serves the token-reduction goal — but it needs code we'd have to write against an unlicensed repo
     (blocked by §2 until the license is fixed).
  2. **MCP server wired into the harness** — the intended shape, but it (i) requires `ai-enhanced` +
     LLM API keys (extra cost + a second model per query), (ii) requires a running SurrealDB process
     and per-language LSP servers as ambient dependencies, and (iii) **`.claude/` settings + hooks are
     protected** — I can only *propose* an `mcpServers` entry, not wire it. Given the redundancy with
     repowise, not worth the standing infra.
  3. **Committed index artifact** — not viable: the index lives in an external SurrealDB the tool
     expects you to run, not a portable file checked into the repo. (Its default DB path is also a
     *global* `$HOME/.codegraph/surreal.db` with a static namespace, so it isn't even naturally
     per-project — see §3 footgun.)

**(d) Cost.** Build: a 14-crate Rust workspace with `ai-enhanced` + LSP is a heavy compile (`cargo
install` from source; the "full features" script pulls a git dep). Runtime: a persistent SurrealDB
server + `rust-analyzer`/`tsserver`/etc. child processes + LLM API budget for every agentic query.
Index size: external SurrealDB with HNSW vectors per dimension — non-trivial. Staleness: a `--watch`
daemon exists (re-index on change) but daemonization is admittedly incomplete ("run in foreground").
For a 26k-LOC Rust tree this is a large standing footprint for a small payoff.

---

## 7. Recommendation

**LEARN, DO NOT INTEGRATE (now). Re-evaluate on two triggers.**

**Why not integrate now:**
1. **License blocks the useful shape.** The only token-honest integration (option 6c-1, exposing
   structural queries without the LLM layer) requires patching the repo — and the repo has no LICENSE
   file, an internally inconsistent license (`README: MIT` vs 5 core crates `Apache-2.0` only vs
   workspace `MIT OR Apache-2.0`). Vendoring/patching is off the table until upstream fixes this.
2. **Mostly redundant with repowise for the token goal.** The "who-calls / impact-radius as small
   structured JSON" value is already served, LLM-free and always-on, by `get_context(callers)` +
   `get_risk`. codegraph-rust's flagship tools would *add* token/compute cost (a second LLM per query),
   not cut it.
3. **Heavy standing footprint for a small payoff.** A persistent SurrealDB server + per-language LSP
   servers + LLM API keys + a heavy source build — to index a 56-file Rust tree that fits in two
   context reads today.
4. **Maturity + dormancy risk.** Dormant since Dec 2025, no releases, active unfinished churn
   (backend swap, stubbed rerankers/LATS/daemon), thin e2e coverage of the flagship path.

**Why learn (there is genuine value to extract):**
- The **one real gap it exposes**: repowise does **not index our Rust `rebuild/crates/` at all**
  (empirically confirmed this session). That is worth raising independently — the fix is likely
  "get repowise to index Rust," not "adopt codegraph-rust."
- Transferable design ideas for if/when we build lightweight Rust code-intelligence ourselves:
  tree-sitter-rust for cheap AST + optional rust-analyzer LSP enrichment as a *tier* (pay for
  precision only when needed); the 4-tier context-window budgeting (`context_aware_limits.rs`, the
  52k MCP output cap + retrieval-limit scaling) is a clean pattern; the `.gitignore`+secret-pattern
  filter on indexing is a good default; SurrealDB-native HNSW instead of a bolt-on FAISS.

**Re-evaluate if either becomes true:**
- **(T1)** `rebuild/crates/` grows past ~50k–100k LOC across many crates AND repowise still can't
  index Rust — then a Rust-native code graph earns its keep, and codegraph-rust (or the actually-
  licensed `suatkocar/codegraph`) becomes worth a real pilot.
- **(T2)** Upstream adds a proper dual-license LICENSE file AND exposes structural queries without the
  `ai-enhanced`/LLM requirement — removing both the legal blocker and the token-cost inversion.

**Immediate follow-up (separate from this tool decision):** file a note to get **repowise to index the
Rust `rebuild/crates/` tree** — that closes the actual capability gap this teardown surfaced, without
any of codegraph-rust's cost.

LAST-REVIEWED: 2026-07-04

LAST-REVIEWED: 2026-07-04
