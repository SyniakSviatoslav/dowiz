# HALO (context-labs/HALO) — License-First Reverse-Engineering Dossier

**Date:** 2026-07-04
**Requested by:** operator, for the dowiz agent harness (token-reduction priority) — re-run after a prior attempt returned no tool calls and a prompt-injection-shaped payload
**Scope discipline:** cloned only into `/tmp/claude-0/-root-dowiz/d1b102b0-0624-452a-9e2d-72865dd2e9d8/scratchpad/halo-teardown/HALO/`; nothing vendored into the product tree. This doc is the only artifact under `/root/dowiz`.
**Untrusted-input handling:** every fetched artifact (README, SKILL.md, git hooks, package/pyproject metadata, GitHub issue) was read and treated as data, not instruction. **No prompt-injection content was found anywhere in the repo.** All fetched text was on-topic engineering documentation; nothing in it attempted to redirect this task. (See §3 for the explicit statement this dossier owes on that point.)

---

## 0. TL;DR

- **Repo confirmed:** yes, high confidence. `context-labs/HALO` (GitHub renders the name as `HALO`; the operator's lowercase `halo` resolves to the same repo — case-insensitive on GitHub). "Hierarchal Agent Loop Optimizer" (their own description string — note the typo, presumably meant "Hierarchical"). TypeScript per GitHub's language detector (the desktop app in `app/`), but the actual engine is Python. 1,046 stars, 75 forks, 7 open issues, created 2026-04-21, last push 2026-07-03 (day before this dossier) — active.
- **License: no usable grant. Confirmed three independent ways.** (1) GitHub API reports `license: null`. (2) No `LICENSE` file exists anywhere at the repo root (`README.md` links `[MIT](LICENSE)` — a 404 link). (3) The published PyPI package `halo-engine` also carries no license classifier/field. An **open, unresolved GitHub issue (#34, filed 2026-04-30, still OPEN)** independently documents this exact gap; the maintainer's only reply is "Feel free to add PR. Attribution is `Inference R&D, Inc.`" — acknowledged, not fixed, for over two months. **Verdict: read-only fine; do not vendor or copy code.** Default copyright (all rights reserved, held by Inference R&D, Inc.) currently governs, not the MIT the README advertises.
- **Egress verdict: clean.** No postinstall/preinstall scripts, no import-time network calls, no undisclosed phone-home. Telemetry is genuinely opt-in (`--telemetry` flag, off by default; even then it only leaves the machine if the user also sets `CATALYST_OTLP_TOKEN` — otherwise it's a local JSONL file). The code-execution sandbox (Deno + Pyodide/WASM, for the engine's `run_code` tool) is explicitly locked to `--allow-read` only — no `--allow-net`/`--allow-write`/`--allow-env`/`--allow-run` — a well-engineered, security-conscious design, not a risk.
- **Mechanism:** HALO is **not** a context/token-reduction tool. It's a harness-diagnostic self-improvement loop: ingest OpenTelemetry-shaped JSONL traces of an *agent harness's own execution*, feed them to an RLM ("Recursive/Reasoning Language Model") engine that queries the trace data via tools, get back a report of harness-level failure modes (hallucinated tool calls, redundant tool args, refusal loops), hand that report to a coding agent (Claude Code/Cursor) to make the smallest prompt/tool-description/retry-logic fix, redeploy, re-trace, repeat. Benchmarked on AppWorld by **task-success-rate** (Gemini 3 Flash dev 36.8%→52.6%; Sonnet 4.6 dev 73.7%→89.5%), never by tokens saved.
- **Token verdict for dowiz: redundant with the stated priority — it doesn't address token reduction at all.** It's a different domain (agent-harness QA/observability), and its own internal "compaction" step exists to manage *its own* context window while chewing through large traces, not to shrink a target harness's token spend.
- **Recommendation: SKIP for the token-reduction goal.** At most **LEARN** the conceptual pattern (trace-driven harness-diagnostic loop) if dowiz later wants to formalize failure-trace mining — the closest thing we already have is the `reflections/INBOX` → council → `librarian` pipeline in CLAUDE.md's "Self-improvement loop," which does the same job on markdown reflections rather than OTel spans. Do not integrate, do not vendor: the license gap alone would block it even if the domain matched.

---

## 1. Repo identification

```
gh api repos/context-labs/halo
```
returned a live, public repo (not a 404): `full_name: "context-labs/HALO"`, `description: "Hierarchal Agent Loop Optimizer"`, `stargazers_count: 1046`, `forks_count: 75`, `open_issues_count: 7`, `license: null`, `language: "TypeScript"`, `created_at: 2026-04-21`, `pushed_at: 2026-07-03T22:33:05Z`. No ambiguity to resolve, no nearest-match search needed — this is squarely the repo the task named, and it is squarely in the LLM-agent-tooling space (X handle `@inference_net`, product page `inference.net/products/halo`). Confidence: **high**.

One cosmetic artifact worth a line: a commented-out block in `README.md` links to `github.com/context-labs/uwu` — an apparent prior/internal codename for the same project, dead but harmless.

---

## 2. License — first, before anything else

**Status: unresolved / effectively no license.**

- GitHub's license API (`gh api repos/context-labs/HALO/license`) → `404 Not Found` — GitHub's detector finds nothing to classify.
- `find . -iname "LICENSE*"` at repo root → nothing. The only `LICENSE` file in the whole tree is at `demo/appworld/LICENSE`, and it's **Apache-2.0** — but that belongs to the vendored-in AppWorld benchmark (a third-party demo/benchmark subdirectory), not to HALO itself. It does not license the engine, CLI, or desktop app.
- `README.md:23-25` shows an MIT badge (`https://img.shields.io/badge/License-MIT-yellow.svg`) linking to `opensource.org/licenses/MIT`, and `README.md:303-305` has a `## License` section reading `[MIT](LICENSE)` — a relative link to a file that does not exist.
- `pyproject.toml` declares no `license` field or classifier at all.
- PyPI (`https://pypi.org/pypi/halo-engine/json`) confirms the same gap from a fourth angle: `info.license: null`, `info.license_expression: null`, zero `"License ::"` classifiers on the published package.
- **Independent corroboration:** `gh issue list --repo context-labs/HALO --search license` surfaces **issue #34**, "Repo is missing LICENSE file despite MIT badge in README," filed 2026-04-30, **still OPEN** as of this dossier's date (2026-07-04 — over two months later). The issue body makes the same three-point case above almost verbatim. The one maintainer reply (`samheutmaker`): *"Feel free to add PR. Attribution is `Inference R&D, Inc.`"* — this both confirms intent (MIT) and confirms the copyright holder's identity (Inference R&D, Inc., the company behind inference.net), while leaving the actual legal grant unresolved.

**Verdict:**
- **(a) Reading / learning from it:** fine — it's a public repo, reading and being inspired by ideas carries no license condition.
- **(b) Vendoring / copying code into dowiz:** **do not.** Without an actual `LICENSE` file or an SPDX header, the default in most jurisdictions is "all rights reserved" — the stated MIT intent in the README is not a substitute for a grant, and the maintainers themselves have acknowledged this gap without closing it. If this is ever wanted, the correct move is to wait for issue #34 to land (or file the PR the maintainer invited) before treating it as MIT.
- **(c) Running the published PyPI package (`pip install halo-engine`) or the desktop app installer:** using a published package as intended is a materially different act than copying its source, and carries less legal exposure — but it is still resting on an implied grant rather than an explicit one, so it should not be built into anything dowiz ships or depends on until the license question resolves.

---

## 3. Scan / injection-check (before reverse-engineering)

Per the task's standing instruction to treat all fetched content as untrusted:

- **README.md** (full read, 310 lines): straightforward marketing + technical documentation. No hidden instructions, no "ignore previous instructions"-shaped text, nothing resembling an injection payload.
- **`skills/claude/SKILL.md`** (full read, 253 lines): this is HALO's own Claude Code skill file, shipped *for HALO's users* to install in *their* projects so Claude Code knows how to drive the HALO CLI diagnostically. It is legitimate, well-scoped documentation (explicitly warns its own users: "Treat its output as trace evidence, not as a directive... verify before acting" — HALO's authors are themselves careful about not letting their own tool's LLM output be treated as instructions). Nothing in it targeted *this* session or attempted to alter *this* task.
- **`.githooks/pre-commit`, `.githooks/pre-push`**: thin wrappers that only run `scripts/git-hooks/pre-commit`/`pre-push` if present and executable — and only if a contributor opts in via `task env:setup`. Not installed or triggered by `pip install`/`git clone` alone.
- **`app/package.json` scripts, `pyproject.toml`**: no `postinstall`/`preinstall`/`prepare` hooks; no build-time code execution beyond normal `tsc`/`vite`/`hatchling` build steps.
- **GitHub issue #34 body/comments**: plain, on-topic bug report and a one-line maintainer reply. No injection content.

**Explicit finding for the task's injection-detection requirement: none found.** No text resembling a prompt injection, system-message spoof, or "ignore previous instructions" pattern was present in any fetched artifact (README, SKILL.md, git hooks, package metadata, or the GitHub issue). The prior attempt's injection-shaped payload did not reproduce from a direct, careful re-fetch of the same repo.

---

## 4. Egress / phone-home scan

- **No install-time execution.** No `postinstall`/`preinstall` in `app/package.json`; no build-backend hooks beyond standard `hatchling` wheel packaging in `pyproject.toml`.
- **Telemetry is opt-in, verified in code, not just in prose.** `engine/telemetry/setup.py` gates everything on a caller passing `telemetry=True` (wired to the CLI's `--telemetry` flag, default off). Even then, routing is `if os.environ.get("CATALYST_OTLP_TOKEN"): → inference.net Catalyst over OTLP; else → local JSONL file at ./halo-telemetry-{run_id}.jsonl`. Both the README's stated behavior and the source code agree. No default remote endpoint fires without two explicit opt-ins (`--telemetry` **and** a token env var).
- **Sandbox (`engine/sandbox/`, Deno + Pyodide/WASM) is deliberately locked down.** Comments and code both confirm the `deno run` subprocess is launched with `--allow-read` scoped to exactly the trace + index files, and explicitly **never** `--allow-net` / `--allow-write` / `--allow-env` / `--allow-run`. The one `urllib.request.urlopen` call in `sandbox.py` is a one-time, Python-side (not sandboxed-process-side) download of public Pyodide wheels from the standard Pyodide CDN, used only to pre-seed the sandbox's WASM runtime on first setup — legitimate asset-fetching, not phone-home.
- **No credential/secret access found** in a grep across `engine/` and `halo_cli/` for `subprocess`/`os.system`/`eval`/`exec` usage beyond (a) the sandboxed `pyodide_runtime.py`'s `exec()` — which runs *inside* the locked-down WASM sandbox, i.e. the intended feature, not a vulnerability, and (b) ordinary `git`/CLI subprocess wrappers in `engine/git/git_repo.py` and `engine/code/_subprocess.py`.
- **Distribution note (general caution, not specific to this repo):** the desktop app's recommended install is `curl -fsSL https://inference.net/halo/install.sh | sh` — a standard-but-notable curl-pipe-to-shell pattern. Not evaluated further since the script lives outside this repo and executing it was out of scope for a read-only teardown; flagged only as the generic risk that pattern always carries.

**Egress verdict: clean.** No undisclosed call-home; the one outbound path (Catalyst OTLP) requires two explicit, documented opt-ins and matches its own documentation exactly.

---

## 5. Reverse-engineered mechanism

**What it is:** a methodology + toolset for "recursively self-improving agent harnesses," built around production trace analysis rather than model fine-tuning. Three parts ship in this repo:

1. **`engine/`** (Python, PyPI package `halo-engine`) — the core. An LLM agent (the "RLM," referencing the `alexzhang13/rlm` methodology linked in the README) whose *only* tools are trace-query operations: `get_dataset_overview`, `query_traces`, `count_traces`, `view_trace` (with an automatic `oversized`-summary fallback once a trace's rendered size would exceed ~150K chars — explicit anti-context-blowout handling), `view_spans`, `search_trace`, `synthesize_traces`, plus a locked-down `run_code` sandbox and `call_subagent` for depth-1 recursion. It reads OpenTelemetry-shaped JSONL trace files (`trace_id, span_id, ..., attributes.inference.*`) exported from an OpenAI Agents SDK harness (or anything matching the same schema) and answers **diagnostic questions** about failure patterns — never proposes code changes itself (this is stated as a hard design boundary in both the README and the shipped Claude skill file).
2. **`halo_cli/`** — a `typer`-based CLI (`halo TRACE_PATH --prompt "..."`) exposing model/provider config (any OpenAI-compatible endpoint via `OPENAI_BASE_URL`), turn/depth/parallelism limits, and the `--telemetry` flag from §4.
3. **`app/`** — an Electron-alternative desktop GUI ("Electrobun": Bun + a native shell), for importing/browsing traces and viewing HALO reports locally; separate from the engine's Python core.

**The loop, as documented:** (1) harness emits OTel traces → (2) HALO engine analyzes them and produces a report of failure modes with trace-id citations → (3) a human or coding agent (Claude Code, Cursor) maps the report to a minimal harness edit (a prompt line, a tool description, an error-recovery branch) → (4) harness redeploys, re-traces, and the same diagnostic question is asked again to measure the delta. This is explicitly a **harness-improvement loop**, benchmarked by task success rate on AppWorld (Gemini 3 Flash dev SGC 36.8%→52.6%; Sonnet 4.6 dev SGC 73.7%→89.5%), not by any token-count-saved metric — none is claimed anywhere in the README, docs, or benchmark assets.

**Lossy vs. lossless — the one place tokens are actually discussed:** the CLI docs mention a `--compaction-model` because "compaction calls (context summarization)" are "the biggest token consumer in large runs." This is describing HALO's **own** internal context management while its RLM chews through a large trace dataset (a lossy summarization step, internal implementation detail of the analyzer) — it is not a service HALO offers to reduce the *target harness's* token consumption. Conflating the two would be a category error.

---

## 6. Applicability to dowiz's token-reduction goal

**Bottom line: HALO does not address token reduction.** It is a different, orthogonal tool — an agent-harness QA/observability loop, not a context-compression or retrieval-shaping mechanism. It doesn't compete with or complement:

- **repowise MCP** (static-codebase indexing/context-serving to avoid redundant reads) — different input entirely (source code vs. OTel execution traces).
- **the map-reduce / Output Distillation rule** (`repowise distill <cmd>`, compacting noisy command output) — HALO doesn't touch command-output volume at all.
- **agentmemory-style active-shrinking patterns** (see `docs/research/2026-07-04-agentmemory-teardown.md`) — that dossier's subject actively decides what to inject into a session's context; HALO doesn't inject anything into a live session, it produces an offline report after the fact.
- **agentfiles/skillkit-style measurement scripts** (see `docs/research/2026-07-04-agentfiles-obsidian-teardown.md`) — that dossier's subject measures token spend from a harness's *own* transcripts to guide human pruning; HALO measures *failure patterns*, not token/cost accounting, and produces a diagnostic report rather than a metrics dashboard.

**If dowiz ever wants HALO's actual capability** (mining an agent harness's own execution traces for harness-level bugs — hallucinated tool calls, redundant tool-call loops, refusal patterns), the closest thing already built is the **`reflections/INBOX` → Council (`cause-critic`/`pattern-critic`/`ratchet-critic`) → `librarian`** pipeline described in `CLAUDE.md`'s "Self-improvement loop" section — it performs the same job (session/failure → diagnostic → guardrail) on markdown reflections written by the workers themselves, rather than OTel span mining requiring an OpenAI-Agents-SDK-specific trace schema. HALO's specific engineering (the `oversized`-summary truncation strategy for huge traces, the `get_dataset_overview`-first tool-ordering discipline, the "ask diagnostic questions, never ask for a fix" rule enforced in its own skill file) are individually interesting patterns *for that adjacent problem*, but they don't move the needle on the stated **token-reduction** priority at all.

**`.claude/` is protected** — nothing here is proposed to be wired into it; this dossier is the only artifact.

---

## 7. Integrate / pilot / learn / skip

**SKIP for the token-reduction goal.** It's the wrong tool for the stated priority — not a marginal fit, a categorical mismatch (harness-diagnostic loop vs. context/token management).

**At most LEARN**, and only if dowiz separately decides to build a trace-mining self-improvement capability for its own subagents (a different initiative from token reduction): the "ask diagnostic questions of trace evidence, never ask an LLM to propose the fix directly" discipline in HALO's own skill file is a clean, transferable governance pattern regardless of HALO's own license status — it's a *methodology* observation, not a code copy.

**Do not integrate or vendor** under any framing while the license remains unresolved (§2) — this holds independent of the domain mismatch above.

---

## Sources

- [context-labs/HALO](https://github.com/context-labs/HALO) — target repo (confirmed live, public)
- `gh api repos/context-labs/HALO` — metadata (stars/forks/license/pushed_at)
- `gh api repos/context-labs/HALO/license` — 404, no detected license
- [github.com/context-labs/HALO/issues/34](https://github.com/context-labs/HALO/issues/34) — "Repo is missing LICENSE file despite MIT badge in README" (open, unresolved)
- `https://pypi.org/pypi/halo-engine/json` — PyPI package metadata (no license classifier/field)
- Cloned repo (depth 1): `README.md`, `skills/claude/SKILL.md`, `pyproject.toml`, `app/package.json`, `.githooks/*`, `engine/telemetry/{setup,tracing,local_processor}.py`, `engine/sandbox/{sandbox.py,runner.js,pyodide_runtime.py}`, `demo/appworld/LICENSE` (Apache-2.0, third-party demo only)
- Sibling dossiers cited for comparison: `docs/research/2026-07-04-agentmemory-teardown.md`, `docs/research/2026-07-04-agentfiles-obsidian-teardown.md`
