# agentmemory — License-First Reverse-Engineering Dossier

**Date:** 2026-07-04
**Requested by:** operator, for the dowiz agent harness (token-consumption reduction)
**Scope discipline:** cloned only into `/tmp/claude-0/-root-dowiz/d1b102b0-0624-452a-9e2d-72865dd2e9d8/scratchpad/agentmemory-teardown/` (main-repo, classic-chromadb-mirror, markdown-variant); nothing vendored into the product tree. This doc is the only artifact under `/root/dowiz`.

---

## 0. TL;DR

- **Canonical pick:** [`rohitg00/agentmemory`](https://github.com/rohitg00/agentmemory) — **Apache-2.0**, 24,550 stars, actively maintained (releases weekly, last push 2026-06-29), 390-commit primary author + real contributor base. Clean to read AND to vendor, but **don't vendor** — argued below.
- **Scan result:** clean. No install-time code execution, no phone-home telemetry, no credential access. All "network calls" in the source are to its own local daemon (`localhost:3111` by default); OTEL metrics are local-only with a no-op default exporter.
- **The prior "classic" `lalalune/agentmemory`** (ChromaDB-powered, the one named in the task brief) is **gone from GitHub** — both `lalalune/agentmemory` and its PyPI-declared homepage `AutonomousResearchGroup/agentmemory` 404. The PyPI package (`agentmemory`, last release 0.4.8, MIT) still exists but points at a dead repo. Best surviving mirror: `Josephrp/agentmemory` (3 stars, dead since 2023, MIT) — used here only to confirm the classic architecture, not as the primary source.
- **Recommendation: LEARN, not integrate.** Apply the patterns natively in our own bash/Node hooks and markdown corpus. Do not add `@agentmemory/agentmemory` (or its pinned `iii-sdk` engine) as a dependency — full reasoning in §7.

---

## 1. Repo identification

GitHub search for "agentmemory" (`in:name`, sorted by stars) returned 170 hits. The relevant ones:

| Repo | Stars | Lang | License | Last push | State |
|---|---|---|---|---|---|
| **rohitg00/agentmemory** | **24,550** | TypeScript | **Apache-2.0** | 2026-06-29 | Active, weekly releases (v0.9.27), 390+10+8+5+4... commits across a real contributor list, dedicated site (agent-memory.dev), Trendshift-featured |
| lalalune/agentmemory | — | — | — | — | **404 — deleted/renamed.** This is the repo the task brief named ("Easy-to-use agent memory, powered by chromadb"). |
| AutonomousResearchGroup/agentmemory | — | — | — | — | **404** — the PyPI package's declared homepage; also gone. |
| Josephrp/agentmemory | 3 | Python | MIT | 2023-07-11 | Dead. Best surviving copy of the classic ChromaDB design (same author "Moon"/shawmakesmagic@gmail.com as the PyPI package) — used here as an architecture reference only. |
| jayzeng/agentmemory | 9 | TypeScript | MIT | 2026-06-20 | Small, alive, philosophically closest to OUR shape: plain markdown files + optional external semantic-search tool (qmd), no database at all. |
| JordanMcCann/agentmemory | 40 | Python | MIT | 2026-03-26 | "#1 on LongMemEval" claim, solo project, low adoption — not investigated further, self-reported benchmark only. |
| kishan0725/AgentMemory | 1 | TypeScript | Apache-2.0 | 2026-06-29 | Fork, trivial star count — not investigated. |
| Dooders/AgentMemory | 2 | Python | MIT | 2025-05-31 | Simulation-agent memory (not coding-agent), unrelated domain — not investigated. |

**Why `rohitg00/agentmemory` is the pick, not the literally-named `lalalune/agentmemory`:** the named repo no longer exists to reverse-engineer directly, and the field has moved on. `rohitg00/agentmemory` is the dominant, actively-maintained, purpose-built successor in this exact space — its own README explicitly benchmarks itself against `mem0`, `Letta`, `Khoj`, `supermemory`, and Claude Code's built-in `CLAUDE.md`, and its headline claims (**95.2% retrieval R@5, 92% fewer tokens**) are a direct hit on the operator's stated goal (reduce token consumption of an agentic coding system). Its 24.5k stars in ~4 months is unusual but not fabricated — it's explained by a linked viral gist (1.3k stars / 182 forks) and a Trendshift feature; contributor and release-cadence data look organic, not bot-inflated.

---

## 2. License — FIRST, before anything else

**`rohitg00/agentmemory`: Apache License 2.0.** Confirmed by `LICENSE` file text and `package.json`'s `"license": "Apache-2.0"`, copyright Rohit Ghumare.

- **(a) Reading / learning from it:** unrestricted. Apache-2.0 places zero conditions on reading, studying, or being inspired by the code. Full green light.
- **(b) Vendoring / integrating it:** permitted under Apache-2.0 (redistribute + modify, must preserve LICENSE/NOTICE and mark changed files, includes an express patent grant — the most redistribution-friendly of the major permissive licenses). **No legal blocker.** The reason we still recommend against integration is architectural/operational, not legal — see §7.

**One secondary flag, not about agentmemory's own license:** agentmemory hard-pins a runtime engine dependency, `iii-sdk` (npm package declares Apache-2.0), which is the client SDK for `iii-hq/iii`. That upstream repo's GitHub metadata shows **no detected license file** (`license: null` via the GitHub API) despite 18.4k stars. This doesn't taint agentmemory's own Apache-2.0 grant, but it means a literal `npm install @agentmemory/agentmemory` pulls in a transitive engine whose own repo-level licensing posture is murkier than the leaf package's `package.json` claims. Treat as a caution for the "integrate" path, not a verdict on agentmemory itself.

**Runners-up, for completeness:** `Josephrp/agentmemory` mirror — MIT (classic lalalune design). `jayzeng/agentmemory` — MIT. `JordanMcCann/agentmemory` — MIT. All permissive; none change the overall verdict since we aren't vendoring any of them.

**Verdict: no "learning-only" restriction applies here — the license is as permissive as it gets. The recommendation to not vendor is purely an architecture-fit judgment (§7), which the operator asked us to argue honestly rather than default to "found a permissive license, therefore add the dependency."**

---

## 3. Scan result (skill-adoption guardrail, before reverse-engineering)

Cloned all three repos into the scratchpad only. Checked for anything that executes on import, calls home, or touches credentials, per the standing skill-adoption discipline:

- **`package.json` scripts:** no `postinstall`/`preinstall`/`prepare` hooks anywhere in `main-repo` (verified via repo-wide grep). Nothing runs at `npm install` time.
- **Outbound network calls:** every `fetch(...)` in `src/` targets `${REST_URL}` / `getBaseUrl()`, which default to `http://localhost:3111` (`process.env["AGENTMEMORY_URL"] || "http://localhost:3111"`). These are calls from the CLI/hooks to agentmemory's *own* local daemon, not third-party egress. The one federation feature (`src/functions/mesh.ts`) fans out to `peer.url` — but that's an explicit, user-configured multi-instance mesh feature, not default-on phone-home.
- **Telemetry:** `src/telemetry/setup.ts` wires an OTEL `Meter` interface with a **no-op default** (`NOOP_COUNTER`/`NOOP_HISTOGRAM`) — metrics only flow if the host app supplies a real meter/exporter. No default external OTEL collector endpoint is configured anywhere in source or `.env.example`.
- **Credential/secret surface:** `.env.example` documents opt-in LLM/embedding provider keys (Anthropic/OpenAI/Gemini/OpenRouter/MiniMax) — every one is commented out by default, and the README states plainly: "no LLM key, no embedding key, and no API auth" required to run. There's also a dedicated `src/functions/privacy.ts` that regex-strips `<private>` tags and ~14 secret-shaped patterns (API keys, bearer tokens, JWTs, GitHub/Slack/AWS/GCP/npm/GitLab/DigitalOcean token formats) from every observation *before* it's stored — i.e., the tool actively scrubs secrets rather than being a secret-exfiltration risk.
- **`jayzeng/agentmemory`'s `scripts/postinstall.cjs`** (the one repo of the three that does have a postinstall hook): read in full. It only (1) checks for a local `qmd` binary via `spawnSync` (no network), (2) prints install instructions if missing, and (3) sets a git hook path — but only when `packageRoot` is a dev checkout of agentmemory's own repo (gated on `.githooks` existing AND not being under `node_modules`), explicitly to avoid ever touching a consumer's repo. No credential access, no network calls.
- **`Josephrp/agentmemory` (classic mirror) `setup.py`:** plain setuptools boilerplate (reads `README.md` for the long description, declares `chromadb` as the only dependency). No exec-on-install logic.

**Verdict: clean across all three. No import-time execution risk, no network-call-home, no credential access found in any of the three clones.**

---

## 4. Architecture (the core deliverable)

### 4.1 Memory model

Not a flat vector store — a **4-tier consolidation hierarchy**, explicitly modeled on human memory consolidation:

| Tier | What | Analogy |
|---|---|---|
| Working | Raw observations from tool use | Short-term memory |
| Episodic | Compressed session summaries | "What happened" |
| Semantic | Extracted facts and patterns | "What I know" |
| Procedural | Workflows and decision patterns ("skills") | "How to do it" |

On top of the tiers sit two higher-order artifacts: **Crystals** (`src/functions/crystallize.ts`) — an LLM digest of a *chain* of completed actions (narrative + key outcomes + files + lessons), which auto-emits `mem::lesson-save` calls; and **Skills** (`src/functions/skill-extract.ts`) — an XML-tagged `<trigger>/<steps>/<expected_outcome>` procedural document extracted from a session, with an explicit `<no-skill/>` escape hatch when the session doesn't show "a clear multi-step procedure that succeeded."

### 4.2 Storage backend

**Zero external databases.** Per the README: "**0 external DBs**" and "External deps: None (SQLite + iii-engine)." Concretely: KV state + an in-memory vector index, both provided by the underlying `iii` runtime (`iii-state` worker), not Postgres/ChromaDB/Redis. `iii worker add iii-database` is offered as an *opt-in* upgrade path to a SQL-backed adapter once you outgrow the in-memory defaults — the classic lalalune/ChromaDB design (confirmed via the `Josephrp` mirror: `pip install agentmemory`, `chromadb` as the sole runtime dependency, category-scoped Chroma collections) has been fully replaced. This is a real architectural lineage change worth noting: the field moved from "always require a vector DB" to "in-memory index + optional upgrade."

### 4.3 Retrieval

**Triple-stream hybrid search fused with Reciprocal Rank Fusion** (`src/state/hybrid-search.ts`, `RRF_K = 60`):
1. **BM25** keyword search (always on, stemmed, multi-script tokenization for Greek/Cyrillic/Hebrew/Arabic/accented Latin; CJK needs an optional segmenter).
2. **Vector** cosine similarity (only if an embedding provider + non-empty index; default local `all-MiniLM-L6-v2` via `@xenova/transformers`, or a paid provider).
3. **Graph** — knowledge-graph traversal via entity matching, expanded from the top-5 vector hits (`graphRetrieval.expandFromChunks`).

Each stream's rank feeds `1 / (RRF_K + rank)`, weighted (`0.4` BM25 / `0.6` vector / `0.3` graph by default, renormalized to whichever streams actually produced results), then **diversified** — `diversifyBySession` caps results at 3 per source session so one noisy session can't dominate the top-K — and optionally reranked. Query expansion (`searchWithExpansion`) issues the same triple-stream search across paraphrases + temporal concretizations and merges by best score per document.

### 4.4 Keeping context SMALL — the part that matters most here

This is where the "92% fewer tokens" claim is actually implemented, and it's not one trick but a stack of five:

1. **Token-budget greedy packing** (`src/functions/context.ts`, the `mem::context` function). Every candidate — pinned slots, project profile, ranked lessons, session summaries, top observations — is turned into a `{content, tokens, recency}` block. Blocks are sorted by recency, then packed greedily into a fixed `TOKEN_BUDGET` (default 2000, `estimateTokens = ceil(chars/3)`): `if (usedTokens + block.tokens > budget) continue;` — **it skips a whole block rather than truncating it mid-sentence.** Whatever doesn't fit this session is simply not injected.
2. **Zero-LLM compression by default** (`src/functions/compress-synthetic.ts`). Every observation is compressed with pure heuristics — infer a type from the tool name, extract file paths, truncate the narrative to 400 chars / title to 80 / subtitle to 120 — with **no LLM call and no token spend** unless the user opts into `AGENTMEMORY_AUTO_COMPRESS=true`. LLM summarization is the exception path, not the default.
3. **SHA-256 dedup with a 5-minute TTL window** (`src/functions/dedup.ts`). Hash `sessionId:toolName:input[:500]`; if seen in the last 5 minutes, don't even store it. A tight, cheap, in-memory `Map` with its own cleanup timer.
4. **Scheduled decay / eviction / auto-forget**, run as background sweeps, not per-read filtering: `mem::auto-forget` expires TTL'd memories, detects near-duplicate **contradictions** via Jaccard token-overlap over `concepts` (>0.9 similarity marks the older memory `isLatest: false`), and prunes observations older than 180 days with `importance <= 2`. `mem::evict` separately handles stale sessions (30 days, no summary), low-importance-old observations (90 days, importance < 3), and a **hard per-project cap** (10,000 observations, importance-ordered eviction beyond the cap). Consolidation itself applies an Ebbinghaus-style exponential decay to memory `strength` based on days-since-last-access.
5. **Small, fixed top-K everywhere** — search always returns a capped, session-diversified top-20 merged set, never "everything matching."

### 4.5 API surface

Function-oriented (`mem::<verb>`), not REST-CRUD-first, though a REST proxy and 53 MCP tools expose the same functions: `mem::remember`, `mem::search` / `mem::smart-search`, `mem::context` (the token-budgeted injector above), `mem::observe` (raw capture), `mem::compress` / `mem::compress-file` (LLM path) vs. the synthetic path above, `mem::consolidate-pipeline` (tier-by-tier promotion working→episodic→semantic→procedural), `mem::crystallize` / `mem::crystal-list`, `mem::skill-extract`, `mem::auto-forget`, `mem::evict`, `mem::profile` (per-project top concepts/files/conventions/errors, 1-hour cache), `mem::lesson-save` / lesson recall, plus governance (`mem::audit`, snapshots, team namespacing, export/import). All wired through 12 Claude Code lifecycle hooks (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, PreCompact, SubagentStart/Stop, Stop, SessionEnd) — this is a hook-driven architecture, the same shape as dowiz's own `pre-edit-lessons` PreToolUse hook, just far more elaborated.

### 4.6 Eval / benchmark claims

- **LongMemEval-S** (ICLR 2025, 500 questions, public benchmark): agentmemory hybrid scores R@5 95.2% / R@10 98.6% / MRR 88.2%, vs. its own BM25-only fallback at 86.2%/94.6%/71.5% — this is the one number in the README the maintainers flag as independently reproducible from their own repo (`benchmark/COMPARISON.md`).
- **coding-agent-life-v1** (their own in-house 15-session corpus): explicitly caveated in the README as "small + gold-sparse" and near the P@5 math ceiling for that corpus — a rare instance of a project honestly downgrading its own benchmark.
- **Token/cost table:** "paste full context" 19.5M+ tokens/yr (exceeds context window) vs. LLM-summarized ~650K tokens/yr (~$500) vs. agentmemory ~170K tokens/yr (~$10, or $0 with local embeddings).
- **vs-competitors table** explicitly marks which numbers are self-measured vs. vendor-self-reported (mem0/Letta figures are their own published LoCoMo numbers on a *different* dataset — not head-to-head). This transparency about benchmark provenance is itself worth noting as a project-quality signal.

---

## 5. Transferable patterns, ranked by expected token savings for OUR shape

Our shape: `/root/.claude/projects/-root-dowiz/memory/MEMORY.md` (one-fact-per-file markdown + an index file loaded in full **every session**), `docs/lessons/` (trigger-keyed, injected by the `pre-edit-lessons` PreToolUse hook), `docs/reflections/` (INBOX/ARCHIVE/RETRO), and subagent prompts that currently inline large context blocks.

1. **Token-budget greedy-pack assembly (context.ts pattern) — biggest lever.** Right now MEMORY.md is loaded in full every session regardless of size; it will keep growing (it's already ~100 index lines). Wrap the "assemble what goes into context" step in a packer: tag each MEMORY.md entry / lesson / reflection with an estimated token cost and a priority (recency, or a manually-set weight), sort, and greedily add whole entries until a fixed budget is hit — **drop the rest of the list entirely rather than truncate mid-entry.** This is a ~30-line script, no dependency needed.
2. **Priority-tiered per-section budgets with graceful truncation (jayzeng markdown-variant pattern) — second biggest, and the most literally analogous.** Their context injector caps each *category* independently (scratchpad 2K chars / topics 2K / today's log 3K tail / search hits 2.5K / MEMORY.md 4K middle-truncated / yesterday's log 3K, trimmed first, under one 16K total cap) rather than one global budget. Maps almost 1:1 onto our stack: give MEMORY.md itself a hard char cap once it grows past N entries, give `docs/lessons/` a separate cap, give `docs/reflections/INBOX/` a separate (smaller, since it's transient) cap, and always trim the *lowest-priority* tier first (their choice: yesterday's log dies before today's).
3. **Content-hash dedup at write time (dedup.ts pattern) — prevents corpus bloat before it happens.** Hash a normalized form of (trigger + causal WHY + touched files) before writing a new lesson/reflection row; skip the write if an equivalent hash was recorded recently. Cheaper than pruning after the fact, and directly bounds the steady-state size of the artifacts that get loaded every session.
4. **Scheduled decay/eviction sweep, not per-read filtering (auto-forget.ts + evict.ts pattern).** Give the `librarian` agent's "prune the store so it never grows" step (already mandated in CLAUDE.md §"Self-improvement loop") a concrete, borrowed shape: TTL-style staleness (a lesson not reinforced/cited in N months), near-duplicate contradiction detection (two lessons whose trigger+content token-overlap exceeds a threshold — keep the newer, mark the older superseded), and a **hard cap** on `docs/lessons/` count with importance-ordered eviction beyond it. Directly targets the standing risk that the trigger-keyed lesson store (and MEMORY.md, which indexes it) monotonically grows and inflates the per-edit hook's injected payload over time.
5. **Trigger/keyword-filtered retrieval instead of full-corpus load.** dowiz already does this *partially* — the `pre-edit-lessons` hook injects by TRIGGER keyword rather than dumping all lessons. agentmemory generalizes the same idea to session-start: inject a compact *profile* (top concepts/files/conventions) plus hybrid-search hits seeded by recent activity, not a full dump of everything ever learned. The gap in our current design is that **MEMORY.md itself has no such filter** — it's an unconditional full load every session. Consider splitting it into an always-loaded one-line-per-entry index (already true) whose *entries themselves* stay index-sized, with the full detail living in per-topic files retrieved on demand (grep/BM25) rather than growing the index file's own prose.
6. **Whole-block selection, never mid-block truncation** (same code path as #1, called out separately because it's a distinct principle) — when assembling a subagent's prompt from multiple lessons/files/reflections, prefer selecting N complete, self-contained items over concatenating truncated excerpts of more items. A half-a-fact is worse than a dropped fact.
7. **Small fixed top-K + source diversification** (`diversifyBySession`, cap 3 per session) — when a subagent's context assembly pulls "all reflections matching X," cap it (e.g., top 10 by a relevance/recency score) and diversify across categories/files so one chatty source doesn't crowd out the rest.
8. **Reserve LLM calls for the rare high-value step; default to free/deterministic for the common path** (compress-synthetic.ts default + local-embeddings-by-default). Every *write* (an edit, a tool call) should cost zero extra tokens — heuristic classification/truncation only. Reserve an actual model call for the periodic, low-frequency promotion step (our librarian's "distill → challenge → promote" already does this in spirit; the transferable refinement is making the *default* path explicitly zero-LLM, with LLM as an opt-in escalation, mirroring `AGENTMEMORY_AUTO_COMPRESS`).
9. **Strict extraction gate before promoting to a reusable artifact** (skill-extract.ts's `<no-skill/>` escape hatch: "if the session is exploratory with no clear procedure, output no-skill"). Our librarian should have the same explicit right-to-refuse: don't promote a reflection into a lesson (and thus into every future session's injected payload) unless the causal WHY is crisp and the steps are concrete — bloat prevention starts at the gate, not just at the prune step.
10. **Contradiction detection via cheap token-overlap, not embeddings** (auto-forget.ts's Jaccard similarity over `concepts` sets, no vector math required) — a fast, dependency-free way to catch "this new lesson supersedes an old one" without needing an embedding model at all, useful if we want #4/#9 without adding any ML dependency.

---

## 6. Applicability map

### (a) dowiz agent harness

The single highest-value move is **#1/#2 above** applied to `MEMORY.md`: it is currently unconditionally loaded in full, every session, with no budget and no tiering — exactly the failure mode agentmemory's own comparison table calls out for `CLAUDE.md` ("Loads everything into context," "22K+ tokens at 240 observations"). A small (~50-line) bash/Node script that (i) estimates each MEMORY.md entry's token cost, (ii) enforces a budget once the file crosses a size threshold, and (iii) defers overflow entries to a searchable secondary file, would directly target the harness's own stated goal without touching `docs/lessons/`'s existing (already-partially-correct) trigger-keyed injection. `docs/lessons/` and `docs/reflections/` benefit most from **#3/#4/#9** — dedup-at-write plus a librarian-side decay/eviction sweep plus a stricter promotion gate, which harden the *existing* "distill → challenge → promote → prune" pipeline described in CLAUDE.md rather than replacing it. Subagent prompts that inline large context are the target of **#6/#7**: cap and diversify what gets inlined, and prefer dropping whole items over truncating them.

### (b) dowiz product's `mem0ai` integration

Grepped `apps/api` and `packages/config` for `mem0`. Findings: `apps/api/src/lib/memory.ts` wraps `mem0ai/oss`'s `Memory` class, configured with **Ollama** for both LLM (`llama3.1:8b`) and embeddings (`nomic-embed-text`), an **in-memory vector store** (`vectorStore.provider: 'memory'`), and `historyDbPath: ':memory:'` — i.e., **nothing persists across a server restart.** It's decorated onto Fastify as `fastify.memory` and passed into `NotificationWorker` (`apps/api/src/notifications/workers/index.ts`), which calls exactly one method: `memory?.recordWorkerAction('notification', 'Dispatched ...', ...)`. A repo-wide grep for any `.search(`/`.getWorkerContext(`/`.getUserContext(` call site against this service found **none** — `getWorkerContext` and `getUserContext` are fully implemented but never invoked anywhere. This matches the memory-corpus note that flagged it "low-value": it is a **write-only, ephemeral, unqueried** integration that requires a live Ollama sidecar in production for zero read-side product value today.

This is a **separate decision from the harness question**, and honestly they don't compose: agentmemory's patterns are about shrinking an *actively-used, growing* context injection; mem0ai here isn't serving a feature at all, so there's nothing to "shrink" — the fix is either (i) wire it up for real (persist the vector store, add the missing read call-sites for actual worker/user personalization) or (ii) retire it (drop the `mem0ai` dependency and the Ollama operational requirement). Worth flagging on its own: if `recordUserInteraction` (also unused today, but present in the wrapper) is ever wired up, it would write arbitrary user-interaction text into a store with **no TTL/decay/eviction of its own** — the exact gap agentmemory's `auto-forget`/`evict` functions exist to close. Recommend raising this as its own small ticket (retire-or-wire-up + add retention if wired up) rather than folding it into the token-reduction work.

---

## 7. Integrate vs. learn — recommendation

**Learn, don't vendor.** Argued honestly, not defaulted:

- **The license doesn't block it** (§2) — Apache-2.0 permits vendoring outright. This is not a legal "no."
- **The runtime shape is the mismatch.** agentmemory is built around an **always-running local daemon** (the `iii` engine — three primitives: worker/function/trigger — replacing Express/SQLite/Socket.io/pm2/Prometheus per its own README table), a pinned engine version (`iii-sdk` 0.11.2, "won't attach to a different version"), an MCP server exposing 53 tools, and a REST API on `:3111` with a companion viewer on `:3113`. Every hook in the repo is a `fetch()` to that daemon. Adopting the package literally means running and operating a new always-on service, not importing a library function.
- **The task-brief's own framing (Node/bash harness vs. a Python dependency) turned out to be based on stale information** — worth correcting explicitly: the *classic* `lalalune`/ChromaDB `agentmemory` was Python, but it's gone from GitHub; the actively-maintained successor we're actually reverse-engineering is **TypeScript/Node**, same runtime family as our harness. That removes the cross-language objection — but doesn't remove the architectural one. A literal dependency would still mean: a new long-running process, a pinned third-party engine whose own upstream repo shows no detected license file at the repo level (§2), and re-plumbing our existing hooks (which today just grep markdown files) to instead speak REST/MCP to a daemon.
- **Scale mismatch in the other direction too.** agentmemory is engineered for an unbounded stream of raw tool-call observations across arbitrary session counts (10,000-observation-per-project caps, 4-tier consolidation, knowledge graphs, mesh federation). Our actual corpus is a few dozen markdown files. The 10 patterns in §5 are individually small (each is tens of lines of bash/Node): a token-budget packer, a dedup hash-check, a decay/eviction sweep in the librarian's existing prune step, a stricter promotion gate. Standing up an entire engine to get those five ideas is disproportionate to the problem size.
- **What we lose by not vendoring, honestly:** we don't get their BM25/vector/graph triple-stream search, their MCP tool surface, their real-time viewer, or their LongMemEval-benchmarked retrieval quality out of the box. If the harness's memory corpus ever grows to the point where trigger-keyword matching in `docs/lessons/` genuinely stops finding the right entry (i.e., we outgrow simple grep), that's the point to revisit a real hybrid-search *library* (not necessarily this whole engine) — not before.

**Concrete next step (not part of this dossier's scope, flagged for a separate task):** implement patterns #1–#4 and #9 from §5 as small, native additions — a token-budget packer for MEMORY.md, a content-hash dedup check before writing a lesson/reflection, and a decay/eviction pass added to the librarian's existing prune step — with no new dependency and no new running process.

---

## Sources

- [rohitg00/agentmemory](https://github.com/rohitg00/agentmemory) — canonical pick, Apache-2.0
- [jayzeng/agentmemory](https://github.com/jayzeng/agentmemory) — markdown-first variant, MIT
- [Josephrp/agentmemory](https://github.com/Josephrp/agentmemory) — surviving mirror of the classic ChromaDB design, MIT
- [JordanMcCann/agentmemory](https://github.com/JordanMcCann/agentmemory) — runner-up, not investigated in depth
- PyPI `agentmemory` package metadata (points to the now-404 `AutonomousResearchGroup/agentmemory`)
- `iii-hq/iii` GitHub repo metadata (engine dependency behind the canonical pick; no repo-level license detected)
- dowiz repo: `apps/api/src/lib/memory.ts`, `apps/api/src/server.ts`, `apps/api/src/notifications/workers/index.ts`, `apps/api/src/bootstrap/notifications.ts`
