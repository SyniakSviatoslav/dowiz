# Agentic Tooling Registry (dev plane — Hetzner /root/dowiz)

> Single config node + service registry for the agentic toolchain.
> Companion to the build plan (`DeliveryOS-Tooling-Integration-Build-Plan`, 2026-06-17).
> Dev/agentic plane only. Product plane (PostHog on Fly/Supabase) is tracked separately.
> Box: 4 vCPU (AMD EPYC-Genoa) · 7.6 GiB RAM (no swap) · ~68 GB free · ≈CPX31-class.

## Provider injection (the two seams — lay once, all tools pull from here)

| Seam | Provider | Source / endpoint | Status |
|------|----------|-------------------|--------|
| **LLM** (chat/completions only) | OpenRouter | `***REDACTED***` in `.env`; rotation chain Nemotron→Qwen→DeepSeek→Gemma→Mistral via `scripts/openrouter-implement.ts` | ✅ smoke-tested on Linux 2026-06-17 (nemotron-3-super-120b:free responded) |
| **Embeddings** (LOCAL only — never OpenRouter) | local Ollama | `http://127.0.0.1:11434` (`/api/embed`, OpenAI-compat `/v1/embeddings`) | ✅ live 2026-06-17, dim 1024 verified |

**Lock-in rule:** vectors are bound to the embedding model. Changing the model = full re-index of every corpus (Repowise / Airweave / Mem0 separately). Pin the tag + dimension below; never mix models across the pipe. A "hybrid" (provider for index, local for query) does NOT work — incompatible vectors.

### Pinned embedding model
- **Model:** Qwen3-Embedding 0.6B (model-hedge: same model exists local + on OpenRouter, so future choice is "where to run it", not "which vendor")
- **Ollama tag (PINNED):** `qwen3-embedding:0.6b` (digest id `ac6da0dfba84`, 639 MB, 32K ctx) — official Ollama library; verified live before pull, not guessed
- **Dimensions:** `1024` (native; MRL-truncatable 32–1024) — verified empirically; keep identical across all vector stores

## Service registry (persistent processes on this one box — budget them)

| Service | Form | Status | Notes |
|---------|------|--------|-------|
| Ollama (embeddings endpoint) | systemd service (`ollama.service`, 127.0.0.1:11434, CPU-only) | ✅ Step 2 done | model 639 MB; watch 8 GB RAM / no swap during bulk index |
| Repowise (code intelligence) | `uv tool` (v0.20.0) + MCP stdio | ✅ Step 3 (structural) | structural index done (1263 files); 10 MCP tools live; LLM→OpenRouter, embeddings→local-ollama. Full LLM wiki-gen DEFERRED (free-tier 429) |
| mempalace (session diary) | existing | ✅ in place | transit/diary memory layer; see note below |

## Notes / resolved open questions

- **mempalace.yaml / MEMORY-MAP.md (open-Q #3):** `mempalace.yaml` configures **mempalace**, an existing session-diary / code-memory tool (`mempalace_search`, `mempalace_diary_read`) — the *transit* layer, NOT canonical. `MEMORY-MAP.md` is the canonical-store routing index. **Implication:** mempalace already owns the session-diary niche → if Mem0/OpenMemory is ever added, it must not duplicate it.
- **Box specs (open-Q #4):** confirmed 4 vCPU / 7.6 GiB / 68 GB free (not the CPX22 the handoff guessed). Bulk index tolerable; RAM with zero swap is the watch item.
- **OpenRouter ↔ Linux (open-Q #5):** ✅ `scripts/openrouter-implement.ts` works on Linux (was Windows). No code change needed.

## Step 3 — Repowise outcome (2026-06-17)

- **Structural index complete** (`--index-only`, 54.9s): 1263 files, 3358 symbols, 5939 graph nodes / 9571 edges, 326 dead-code findings, 870-file git history / 96 hotspots, 2 decisions. `repowise doctor` clean; `Coordinator drift: SQL=14 / Vector=14 / 0.0%` (vector store consistent with pages).
- **MCP server verified end-to-end** over stdio JSON-RPC: 10 tools live (`get_answer, get_context, get_dead_code, get_health, get_overview, list_repos, get_risk, search_codebase, get_symbol, get_why`). `get_overview` + `search_codebase` return real data.
- **Embeddings seam proven LOCAL & consistent:** index embedding = 14 local `/api/embed` calls to Ollama; query embedding (CLI semantic + MCP `search_codebase`) = local calls; **zero OpenRouter embed calls**. Semantic search returns ranked hits.
- **Embedder wiring (the critical seam):**
  - `repowise` reads **`.repowise/.env`** (NOT the app `.env`) → holds `***REDACTED***` + `OLLAMA_EMBEDDING_MODEL=qwen3-embedding:0.6b` + `OLLAMA_BASE_URL` + `REPOWISE_EMBEDDER=ollama`. Gitignored.
  - **Auto-detect would silently pick OpenRouter for embeddings** (key present) → a locality violation. Forced via `--embedder ollama` (CLI) and `REPOWISE_EMBEDDER=ollama` (env). MCP server reads `REPOWISE_EMBEDDER` from `os.environ` (`_server.py:50`) → injected via **`env` block in project `.mcp.json`**.
  - ollama embedder default model is `embeddinggemma` (not pulled) → unset = silent **MockEmbedder** fallback. Pinned to `qwen3-embedding:0.6b` everywhere.
  - repowise default LLM is paid `claude-sonnet-4.6` → overridden to free model.

### Deferred / follow-ups
- **Full LLM wiki-gen** (rich `get_answer` synthesis, full doc pages) — blocked by free-tier **429 rate-limits** (single free models can't rotate like `openrouter-implement.ts`). This is the **"Headroom / paid LLM lane"** park-with-trigger: backfill via `repowise init --resume` once a non-rate-limited LLM lane exists (paid OpenRouter / BYOK / off-peak). Until then the corpus is 14 pages (symbol spotlights + 2 ADRs); structural tools are fully usable.
- **`~/.claude/settings.json` (global, protected zone — needs manual approval):** `init` auto-added a 2nd `repowise` MCP entry (lacks the embedder `env` block) + a global `PostToolUse: repowise-augment` hook. Decide: keep / align env / remove. Project `.mcp.json` is the canonical, correctly-wired registration.

## Browse/extract (dev/ops research seam) — browser-use chosen over Hyperbrowser
- **Decision:** browser-use (MIT) is the browse/extract tool; **Hyperbrowser dropped from the queue** (the duplicate — pick one). browser-use's actual consumer (Open Deep Research / ODR) is **not in this repo yet**, so the ODR-MCP wiring + a live ODR research-run are **deferred until ODR lands**.
- **Form:** **on-demand MCP server, NOT self-hosted** — registered in project `.mcp.json` as `browser-use` via `uvx browser-use[cli] --mcp` (pulled + run only when an MCP client connects, then exits; nothing persistent, nothing always-on — I4). **Self-host lib, never the cloud plan** (cloud trains on input without opt-out). Telemetry forced off.
- **LLM (BYOK):** LLM-backed tools (`browser_extract_content`, `retry_with_browser_use_agent`) need OpenRouter at launch: `OPENAI_API_KEY=$***REDACTED***` + `OPENAI_BASE_URL=https://openrouter.ai/api/v1` (not committed — supplied in the launch env). Pure browse tools (`browser_navigate`, `browser_get_html`, `browser_get_state`, …) need no LLM.
- **Verified:** MCP server enumerated 16 browse/extract tools over stdio JSON-RPC (`browser_navigate, browser_click, browser_type, browser_get_state, browser_extract_content, browser_get_html, browser_screenshot, browser_scroll, …, retry_with_browser_use_agent`). A live browse run was intentionally **not hosted** (no persistent install).

## Subagents (dev) — 3 hand-picked from agency-agents (MIT)
- **Source:** `msitarzewski/agency-agents` (MIT — license gate I3 ✓). Installed by copying into **`~/.claude/agents/`** (user-global, outside this repo — not committed). The other ~46 agents were **not** installed (took 2–3, not all — DoD D).
- **Installed:** `engineering-backend-architect` (Backend Architect) · `security-appsec-engineer` (Application Security Engineer) · `testing-api-tester` (API Tester — fits the API-heavy stack + proof-by-test rule).
- **Governance non-conflict (verified):** the agent files carry no `tools:` restriction → they inherit the full toolset, so the session hooks fire for them too. `settings.json` untouched; `protect-paths` re-tested post-install — blocks a protected path (exit 2), allows a normal path (exit 0). `require-classification` (Stop) + `post-edit-gates` (PostToolUse) unchanged → governance holds.

## Research agent — Open Deep Research (ODR), Stage A done
- **Location:** `/root/open_deep_research` (cloned, **outside the product repo** — dev/ops plane, not product N=1/PII). LangGraph app; Python 3.11 venv via `uv`. MIT.
- **Models:** all roles (summarization/research/compression/final_report) via **OpenRouter** — redirect is env-only: `OPENAI_BASE_URL=https://openrouter.ai/api/v1` + `OPENAI_API_KEY=<OpenRouter key>` (ODR `.env`, gitignored, key never printed). Capability footgun cleared: **`nvidia/nemotron-3-super-120b-a12b:free`** passes structured-output + tool-calling (gpt-oss-120b failed structured; qwen3-next/llama-3.3 were 429).
- **Search (no Tavily, OSS rule):** ODR's `search_api` enum has **no searxng** value and `mcp_config` is a **single Streamable-HTTP server** (not the multi-server block the plan assumed). So: `search_api=none` + `mcp_config.url=http://127.0.0.1:8765` → **`oss_search_mcp.py`** (FastMCP + `ddgs` DuckDuckGo, both MIT) — Docker-free, keyless, on-demand. Replaces Tavily.
- **Tracing:** LangSmith OFF (empty `LANGSMITH_*`).
- **DoD A proven:** one dowiz query (PaddleOCR vs Tesseract for menu OCR) → 10.2k-char cited report (9 citations); 22 OSS-search MCP tool hits; **zero Tavily**; zero structured-output/tool-calling/429 errors. Report is ADVISORY (G3) — single-model output has likely-fabricated specifics; the factcheck + **decorrelated** adversarial verifier (different OpenRouter model, G4) are Stage B/C + agent-system §6 step 4.
- **Stage B/C deferred:** grounding on Repowise/Airweave needs an HTTP MCP gateway (ODR takes ONE http url; Repowise/browser-use are stdio). Notes/signals (Mem0 + pg-boss→Telegram) later.

## Parked (with triggers — see build plan §4)
Headroom · Mem0/OpenMemory · Airweave · Octogent (`hesamsheikh/octogent`, MIT) · Pake.

## Privacy gate (§2.2 — build into architecture before any owner-data tool)
Any text → vector must pass ONE ingest contract: (a) strip/pseudonymize PII, (b) tag tenant, (c) then embed locally. Tenant-isolate vectors (extend `verify:rls` to vector tables). Support erasure. Applies to Airweave / Mem0 / product AI features — NOT to Repowise (code corpus: no PII risk, only proprietary-code risk, closed by locality).
