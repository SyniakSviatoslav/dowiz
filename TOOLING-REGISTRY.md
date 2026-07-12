# Agentic Tooling Registry (dev plane — Hetzner /root/dowiz)

> Single config node + service registry for the agentic toolchain.
> Companion to the build plan (`DeliveryOS-Tooling-Integration-Build-Plan`, 2026-06-17).
> Dev/agentic plane only. Product plane (PostHog on Fly/Supabase) is tracked separately.
> Box: 4 vCPU (AMD EPYC-Genoa) · 7.6 GiB RAM (no swap) · ~68 GB free · ≈CPX31-class.

## Provider injection (the two seams — lay once, all tools pull from here)

| Seam | Provider | Source / endpoint | Status |
|------|----------|-------------------|--------|
| **LLM** (chat/completions only) | OpenRouter | `OPENROUTER_API_KEY` in `.env`; rotation chain Nemotron→Qwen→DeepSeek→Gemma→Mistral via `scripts/openrouter-implement.ts` | ✅ smoke-tested on Linux 2026-06-17 (nemotron-3-super-120b:free responded) |
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
  - `repowise` reads **`.repowise/.env`** (NOT the app `.env`) → holds `OPENROUTER_API_KEY` + `OLLAMA_EMBEDDING_MODEL=qwen3-embedding:0.6b` + `OLLAMA_BASE_URL` + `REPOWISE_EMBEDDER=ollama`. Gitignored.
  - **Auto-detect would silently pick OpenRouter for embeddings** (key present) → a locality violation. Forced via `--embedder ollama` (CLI) and `REPOWISE_EMBEDDER=ollama` (env). MCP server reads `REPOWISE_EMBEDDER` from `os.environ` (`_server.py:50`) → injected via **`env` block in project `.mcp.json`**.
  - ollama embedder default model is `embeddinggemma` (not pulled) → unset = silent **MockEmbedder** fallback. Pinned to `qwen3-embedding:0.6b` everywhere.
  - repowise default LLM is paid `claude-sonnet-4.6` → overridden to free model.

### Deferred / follow-ups
- **Full LLM wiki-gen** (rich `get_answer` synthesis, full doc pages) — blocked by free-tier **429 rate-limits** (single free models can't rotate like `openrouter-implement.ts`). This is the **"Headroom / paid LLM lane"** park-with-trigger: backfill via `repowise init --resume` once a non-rate-limited LLM lane exists (paid OpenRouter / BYOK / off-peak). Until then the corpus is 14 pages (symbol spotlights + 2 ADRs); structural tools are fully usable.
- **`~/.claude/settings.json` (global, protected zone — needs manual approval):** `init` auto-added a 2nd `repowise` MCP entry (lacks the embedder `env` block) + a global `PostToolUse: repowise-augment` hook. Decide: keep / align env / remove. Project `.mcp.json` is the canonical, correctly-wired registration.

## Browse/extract (dev/ops research seam) — browser-use chosen over Hyperbrowser
- **Decision:** browser-use (MIT) is the browse/extract tool; **Hyperbrowser dropped from the queue** (the duplicate — pick one). browser-use's actual consumer (Open Deep Research / ODR) is **not in this repo yet**, so the ODR-MCP wiring + a live ODR research-run are **deferred until ODR lands**.
- **Form:** **on-demand MCP server, NOT self-hosted** — registered in project `.mcp.json` as `browser-use` via `uvx browser-use[cli] --mcp` (pulled + run only when an MCP client connects, then exits; nothing persistent, nothing always-on — I4). **Self-host lib, never the cloud plan** (cloud trains on input without opt-out). Telemetry forced off.
- **LLM (BYOK):** LLM-backed tools (`browser_extract_content`, `retry_with_browser_use_agent`) need OpenRouter at launch: `OPENAI_API_KEY=$OPENROUTER_API_KEY` + `OPENAI_BASE_URL=https://openrouter.ai/api/v1` (not committed — supplied in the launch env). Pure browse tools (`browser_navigate`, `browser_get_html`, `browser_get_state`, …) need no LLM.
- **Verified:** MCP server enumerated 16 browse/extract tools over stdio JSON-RPC (`browser_navigate, browser_click, browser_type, browser_get_state, browser_extract_content, browser_get_html, browser_screenshot, browser_scroll, …, retry_with_browser_use_agent`). A live browse run was intentionally **not hosted** (no persistent install).

## Subagents (dev) — 3 hand-picked from agency-agents (MIT)
- **Source:** `msitarzewski/agency-agents` (MIT — license gate I3 ✓). Installed by copying into **`~/.claude/agents/`** (user-global, outside this repo — not committed). The other ~46 agents were **not** installed (took 2–3, not all — DoD D).
- **Installed:** `engineering-backend-architect` (Backend Architect) · `security-appsec-engineer` (Application Security Engineer) · `testing-api-tester` (API Tester — fits the API-heavy stack + proof-by-test rule).
- **Governance non-conflict (verified):** the agent files carry no `tools:` restriction → they inherit the full toolset, so the session hooks fire for them too. `settings.json` untouched; `protect-paths` re-tested post-install — blocks a protected path (exit 2), allows a normal path (exit 0). `require-classification` (Stop) + `post-edit-gates` (PostToolUse) unchanged → governance holds.

## Research agent — Open Deep Research (ODR), Stage A + B done
- **Location:** `/root/open_deep_research` (cloned, **outside the product repo** — dev/ops plane, not product N=1/PII). LangGraph app; Python 3.11 venv via `uv`. MIT.
- **Models:** all roles (summarization/research/compression/final_report) via **OpenRouter** — redirect is env-only: `OPENAI_BASE_URL=https://openrouter.ai/api/v1` + `OPENAI_API_KEY=<OpenRouter key>` (ODR `.env`, gitignored, key never printed). Capability footgun cleared: **`nvidia/nemotron-3-super-120b-a12b:free`** passes structured-output + tool-calling (gpt-oss-120b failed structured; qwen3-next/llama-3.3 were 429).
- **Search (no Tavily, OSS rule):** ODR's `search_api` enum has **no searxng** value and `mcp_config` is a **single Streamable-HTTP server** (not the multi-server block the plan assumed). So: `search_api=none` + `mcp_config.url=http://127.0.0.1:8765` → **`oss_search_mcp.py`** (FastMCP + `ddgs` DuckDuckGo, both MIT) — Docker-free, keyless, on-demand. Replaces Tavily.
- **Tracing:** LangSmith OFF (empty `LANGSMITH_*`).
- **DoD A proven:** one dowiz query (PaddleOCR vs Tesseract for menu OCR) → 10.2k-char cited report (9 citations); 22 OSS-search MCP tool hits; **zero Tavily**; zero structured-output/tool-calling/429 errors. Report is ADVISORY (G3) — single-model output has likely-fabricated specifics; the factcheck + **decorrelated** adversarial verifier (different OpenRouter model, G4) are Stage B/C + agent-system §6 step 4.
- **Stage B done (MCP gateway):** `odr_mcp_gateway.py` (FastMCP 3.4.2, Apache-2.0) aggregates **OSS search (HTTP) + Repowise (stdio)** behind ONE Streamable-HTTP endpoint (:8800), working around ODR's single-server `mcp_config`. ODR `mcp_config.url=http://127.0.0.1:8800`, tools=`[search_search, repowise_get_context, repowise_search_codebase, repowise_get_symbol, repowise_get_answer]`. DoD B run → 13.6k-char report that **cites the real dowiz codebase** via Repowise (`AiOcrParser`, `ai-ocr-parser.ts`, `paddle-ocr.py`) — local grounding decisively proven; web-citation side weak this run (free model fabricated a github URL). Repowise embeddings stay LOCAL (ollama). The gateway + search are **on-demand** (started for a run, stopped after — not always-on, I4).
- **All-FREE research loop:** researcher = `nemotron-3-super-120b-a12b:free`; verifier = **`odr_verify.py`** (rotates free models llama-3.3-70b → qwen3-next-80b → **gpt-oss-120b** → nemotron-ultra, all ≠ researcher per G4). Honest tradeoff (G6): the free verifier is **tool-less** (judges report text only → conservative, flags unsourced claims, defaults UNCONFIRMED). The Claude CC `research-verifier` subagent (Read+WebFetch) is the **source-confirming** path — it read repo files + fetched pricing → nuanced MIXED where the free one said UNRELIABLE. Pick per budget; both decorrelated.
- **CC subagents run on Claude, NOT OpenRouter free:** the Agent-tool `model:` field selects Claude tiers only — `invariant-guardian`/`security-sentinel` (haiku), `test-scout`/`research-verifier` (sonnet). Free-model work must go through OpenRouter (ODR + `odr_verify.py`), not CC subagents.
- **Stage C deferred:** Mem0 report-sink + pg-boss→Telegram signals. Airweave grounding pluggable into the same gateway when it lands.

## Parked (with triggers — see build plan §4)
Headroom · Mem0/OpenMemory · Airweave · Octogent (`hesamsheikh/octogent`, MIT) · Pake.

## Scaffolded — DO NOT USE (added 2026-07-02, dark; per-tool pilot doc in docs/research/)

> Added to the registry as *candidates*, per operator request. **None are wired, in CI, or a dependency.**
> All are **FORBIDDEN-DEP** (out-of-tree only). Adoption of each is a separate, explicit decision.

| Tool | Plane | Pilot doc | Gate to adopt |
|------|-------|-----------|---------------|
| **CloakBrowser** | scraping | `docs/research/cloakbrowser-pilot.md` | 🔴 ethics: stealth bot-evasion vs 3rd parties — human/operator call |
| **Firecrawl** | scraping | `docs/research/firecrawl-pilot.md` | 🔶 subprocessor/compliance gate; self-host AGPL only; pick ONE scrape lane w/ CloakBrowser |
| **Certimate** | infra/TLS | `docs/research/certimate-pilot.md` | trigger: first custom-domain white-label storefront |
| **OpenHands** | agent/dev | `docs/research/openhands-pilot.md` | 🔶 conflicts w/ Claude-Code harness; default REJECT unless it wins one scoped batch job |
| **Ubicloud** | cloud/IaaS | `docs/research/ubicloud-pilot.md` | trigger: costed scale-out OR hard data-residency requirement |
| **CF Containers (Workers+Docker)** | edge compute | `docs/research/cf-workers-docker-pilot.md` | trigger: measured edge-latency need Fly can't serve |

Common controls (all six): no dowiz DB/RLS/tenant secret in any sidecar env
(`scripts/skyvern-pilot/no-credential-attest.mjs`); scraping tools also gate on
`scripts/scrape-pilot/scraping-conduct-attest.mjs`; verify each license before adoption.

## Privacy gate (§2.2 — build into architecture before any owner-data tool)
Any text → vector must pass ONE ingest contract: (a) strip/pseudonymize PII, (b) tag tenant, (c) then embed locally. Tenant-isolate vectors (extend `verify:rls` to vector tables). Support erasure. Applies to Airweave / Mem0 / product AI features — NOT to Repowise (code corpus: no PII risk, only proprietary-code risk, closed by locality).

## Red-team / security toolset (added 2026-07-02, per `docs/security/redteam-toolset-analysis-2026-07-02.md`)

> **Governing frame:** dowiz owns **code + data, NOT infrastructure** (network/host/edge = Fly/Supabase/R2 = third parties, off-limits). Legitimate self-red-team is the **application layer** over HTTPS at owned `*.dowiz.*` hostnames only, **staging-only, modest volume**. Every finding → a red→green guardrail + `REGRESSION-LEDGER` row (same discipline as any fix).
>
> **Status semantics:** offensive tooling is **WORKSTATION ONLY** — run from a disposable Kali VM/container against staging; **never installed on the dev box, never a repo dependency** (same spirit as FORBIDDEN-DEP). `CODE-INTEGRATED` = a keyless/OSS capability wired into an in-repo script. `DATA` = an inert wordlist/pointer used only for own-target fuzzing.
>
> **🔴 Charter line:** person/social profiling is off-limits. theHarvester (email harvest) + SpiderFoot (person modules) stay scoped to owned `*.dowiz.*` assets with person-modules DISABLED. Maigret (people-profiling) is SKIP for the platform. John the Ripper never touches prod hashes (PII red-line).

### Tier 1 — ADOPT

| Tool | Plane | License | Status | Ethics | Purpose / concrete dowiz use |
|------|-------|---------|--------|--------|------------------------------|
| **Autorize** | offensive | free BApp (Burp ext) | ADOPTED — WORKSTATION ONLY (disposable Kali VM vs staging; not on dev box; not a repo dep) | ok (own assets) | Record owner-A session, replay every req as owner-B/unauth, diff responses → **the** cross-tenant/IDOR class tool (owner-revocation, ADR-0013). App-layer authz gap; RLS FORCE proves DB backstop. Start here. |
| **JWT Editor** | offensive | Apache-2.0 (Burp ext) | ADOPTED — WORKSTATION ONLY | ok (own tokens) | Attack RS256 invariants: alg-confusion (HS256 re-sign on public key), `alg:none`, tampered tenant/role claims, expired/post-revocation owner tokens (ADR-0004 24h TTL + per-req `status='active'`). |
| **SQLmap** | offensive | GPL-2.0 | ADOPTED — WORKSTATION ONLY | staging-only, modest `--risk`/`--level` | Prove injection immunity + tenant/RLS non-bypass on own API (menu search/filter, order-by-id, owner analytics, menu-import). Authenticated via `--cookie`/`--headers`. Never prod. |
| **crt.sh** | asset-recon | public CT-log service (Sectigo) | CODE-INTEGRATED → `scripts/asset-surface-scan.mjs` | ok (public data) | CT-log diff of `%.dowiz.*` to catch forgotten staging/preview subdomains. Best value-to-effort (one curl). Wired keyless into the asset-surface scan lane. |
| **theHarvester** | asset-recon | GPL-2.0 | ADOPTED — WORKSTATION ONLY | 🔶 person/email modules scoped to owned `*.dowiz.*` ONLY | Domain-scoped subdomain/email/host enum of owned assets. Latent profiling capability → keep target-restricted. |
| **SecLists** | data | MIT | DATA — vendored/pointer, own-target fuzzing only | ok (inert wordlists) | Wordlist fuel for fuzzing own staging API + DNS brute + secret-pattern repo scan. Inert data, not executed code. |
| **Kali Linux** | offensive (workstation base) | OSS distro (GPL/mixed) | ADOPTED — WORKSTATION ONLY (`kalilinux/kali-rolling` disposable VM/container; NOT 40 installs on dev box) | ok (isolation is the point) | Isolated red-team workstation that keeps offensive tooling out of the build/deploy env. Loads Burp Community + Autorize + JWT Editor. |

### Tier 2 — PILOT

| Tool | Plane | License | Status | Ethics | Purpose / trigger |
|------|-------|---------|--------|--------|-------------------|
| **Param Miner** | offensive | Apache-2.0 (Burp ext) | PILOT — WORKSTATION ONLY | staging-only | Hidden param/header + cache-poisoning recon on API + public SPA-proxy/`/s/:slug` (prior pool-starvation/caching history). |
| **Hackvertor** | offensive | free BApp (Apache-2.0) | PILOT — WORKSTATION ONLY | staging-only | Encoding-chain multiplier for menu-import parser / Zod-boundary fuzzing. |
| **John the Ripper** | offensive (credential audit) | GPL-2.0 | PILOT — WORKSTATION ONLY | 🔴 **never prod hashes** (PII red-line) | One-shot OFFLINE audit vs sample dev/staging **argon2** hashes of known-weak passwords → should NOT recover them → certifies argon2 cost params. |
| **SpiderFoot** | asset-recon | MIT (OSS stagnant since v4.0/2022) | PILOT — WORKSTATION ONLY | 🔶 person/social modules DISABLED; owned assets only | Dark, scoped, periodic wide sweep (buckets, leaked keys, breach hits) restricted to owned assets. |
| **RSSHub** | blue-team (SCOUT) | MIT | CODE-INTEGRATED → `scripts/scout-feeds.mjs` | ok | Fills plane-maintainer SCOUT hole (upstream dep releases/advisories). Start ZERO-dep GitHub `.releases.atom` polling wired into SCOUT; self-host RSSHub on Fly only once watchlist needs ≥~5 feed-less/non-GitHub sources. |
| **Bambdas** | offensive | LGPL-3.0 | PILOT — WORKSTATION ONLY | staging-only | Table-filter Bambdas triage on Community; full authz scan-checks need Burp Pro (deferred). |

> **PARK-with-trigger (not adopted):** Wireshark (raw WS-`?token=` capture artifact), THC-Hydra (auth rate-limit validation — prefer scripted E2E), Suricata (IDS — trigger: dowiz self-hosts network), ELK/Elastic (SIEM — trigger: centralized log search outgrows telemetry), Recon-ng (redundant), EnIGMA/SWE-agent self-red-team (trigger: dedicated gated pentest lane).
> **SKIP:** Nmap/RustScan/Metasploit/Aircrack-ng/Snort (target = third-party infra / no surface), Parrot OS (redundant w/ Kali), Maigret (🔴 people-profiling — operator-own-handles personal leak-check ONLY, never platform/customers).
