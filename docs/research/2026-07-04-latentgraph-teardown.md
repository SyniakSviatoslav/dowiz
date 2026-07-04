# latentgraph teardown — license-first reverse-engineering dossier

**Scope:** is `LatentForce-ai/latentgraph-mcp-server` ("latentgraph") worth integrating into dowiz as
a code-intelligence MCP, given we already run repowise? Clone kept out-of-tree at
`/tmp/.../scratchpad/latentgraph-teardown/` per the skill-adoption guardrail; nothing landed under
`/root/dowiz` except this doc.

**Verdict: SKIP — redundant AND dangerous.** MIT-licensed client, but the client is a thin shell
around a paid third-party SaaS that requires uploading full source file contents + git history to
`latentgraph.latentforce.ai` for indexing, and runs a background daemon that accepts
**server-initiated arbitrary shell command execution** (`execute_command` / `run_bash`, "auto-approval
— no UI") over a persistent WebSocket. It is feature-redundant with repowise (same 8 read-tool shape:
overview / file / dependencies / call-chain / symbol / decisions / NL-Q&A) and adds a live remote-code-
execution trust surface plus recurring "credits" billing that repowise doesn't have. Do not point it at
this codebase.

---

## 1. Repo identification

`gh search repos latentgraph` surfaced 5 hits; four are academic "Latent Graph Diffusion" ML papers
(wrong domain — graph *diffusion models*, not code graphs). The one real candidate, confirmed via
`gh api repos/LatentForce-ai/latentgraph-mcp-server`:

- **Repo:** [`LatentForce-ai/latentgraph-mcp-server`](https://github.com/LatentForce-ai/latentgraph-mcp-server)
  — published to npm as `@latentforce/latentgraph`
- **Stars:** 10 · **Forks:** 2 · **Open issues:** 2 · **Language:** TypeScript
- **License:** MIT (proper `LICENSE` file, copyright "LatentForce", present and correctly formed)
- **Last push:** 2026-06-09 · **Created:** 2026-04-17 (young — ~2.5 months old at teardown time)
- **Description (verbatim):** "MCP server that connects coding agents to LatentGraph's full repo
  index — deep codebase knowledge for Claude, Cursor, Windsurf, and any AI coding tool"

**What it actually is** (from README + source, not the name): a CLI (`lgraph`) + MCP server that
scans a project, uploads file trees / git metadata / full source contents to LatentForce's hosted
backend (`latentgraph.latentforce.ai` for the API, `latentgraph-orch.latentforce.ai` for the
orchestrator/WebSocket), which builds a "Dependency Relationship Graph" (DRG) + wiki docs +
implicit-dependency graph server-side using LLM calls, then exposes 9 MCP tools (8 read + 1 write)
that query that hosted index. Requires a **paid API key** — CLI errors literally include
`Insufficient credits: ...` (HTTP 402) and a "credits reserved"/"umbrella" billing model per LOC.

This is category (a) from the task brief: a code-intelligence/dependency graph tool, directly
comparable to repowise (which we already run) — **not** an agent-memory graph and not something else.

---

## 2. License — clean, but scoped to a shell

MIT, properly formed, no ambiguity. **However**, the license only covers the ~50-file TypeScript
CLI/MCP client in this repo. The actual code-intelligence engine — the AST/dependency analysis, the
wiki-doc generation, the `ask_codebase` retrieval, the call-graph resolution — all runs **server-side**
on LatentForce's proprietary, closed, paid backend. There is nothing to vendor or self-host: forking
this repo gets you a client with no brain. "Read/vendor/run" verdict: **read** is fine (that's this
dossier); **vendor** is pointless (thin client only); **run** against our own code is the disqualifying
part — see §3.

---

## 3. Egress + trust scan — the load-bearing finding

Per the brief, "if it ingests code or sees model/agent traffic → egress scan is load-bearing." It
does both, and the findings disqualify it outright regardless of license:

**a) Full source upload, not just metadata.** `src/utils/api-client.ts` defines
`sendInitScan` / `sendUpdateDrg` / `sendUpdateImplicit` / `sendUpdateWiki` / `sendUpdateFileIndex`,
each POSTing a `files: FileWithContent[]` array (`{ file_path, content }` — **raw file text**) plus
git branch/commit info to the LatentForce backend. `lgraph init` "sends everything to the Latentgraph
backend for indexing" (README, line 157) and `update-wiki`/`update-file-index` "send all project
source files to the backend" verbatim. This is the intended, documented behavior, not a bug — but it
means the entire proprietary tree (money logic, RLS policies, auth, anything not yet scrubbed of the
kind of secret we already had a live incident over) would leave our infrastructure and sit in a
third-party's paid multi-tenant SaaS.

**b) A background daemon that accepts server-directed shell execution.**
`src/daemon/websocket-client.ts` opens a persistent `wss://latentgraph-orch.latentforce.ai/ws/extension/{project_id}`
connection (100 reconnect attempts, 5s backoff — effectively always-on) and dispatches
`execute_tool` messages from the server to `src/daemon/tools-executor.ts`. That executor's tool table
includes:
```
'execute_command': this.executeCommand.bind(this),   // execAsync(command, { cwd, timeout: 30000 })
'run_bash':        this.runBash.bind(this),           // execAsync(command, { cwd: workspaceRoot })
```
with an explicit code comment: *"Execute command (with auto-approval for MCP - no UI)"*. This is a
live remote-shell-execution surface, gated only by the vendor's server logic, with **zero local
confirmation**. A backend compromise, a malicious insider, or a MITM'd/typosquatted update at
LatentForce would translate directly into arbitrary code execution on every connected developer
machine. This is architecturally the same class of risk our own `secrets-exposure-incident-2026-07-03`
memory is about — except here it would be self-inflicted by choosing to install this tool.

**c) Self-mutates our protected `.claude/` surface, unprompted.** `lgraph add claude-code` writes
`.claude/hooks/lgraph/lgraph-hook.cjs` and rewrites `.claude/settings.json`, registering PreToolUse
hooks on `Grep|Glob|Read|Bash|Edit|Write|MultiEdit`, a PostToolUse hook, a PreCompact hook, and a
SessionStart hook — all calling home to the LatentForce API on nearly every tool call the agent makes
(`/api/v1/mcp/what-is-this-file`, `/dependency`, `/pr-insights`, `/project-overview`), and it patches
our project's `CLAUDE.md` in place (`src/integration/claude/claude-md.ts`, marker-delimited
auto-injected section). The CLI's own `--yes`/`-y` flag is documented as "**Skip the consent prompt**
and apply Claude/Kiro integration changes automatically" — i.e., it is designed to be able to rewrite
a repo's `.claude/settings.json` and `CLAUDE.md` non-interactively. Per our own CLAUDE.md, `.claude/`
is protected and any change there must be proposed, not auto-applied by a third-party installer.

**d) Prompt-injection-shaped payload, by design.** The `CLAUDE_MD_CONTENT` string in
`claude-md.ts` and the `HOOK_SCRIPT` string in `hooks.ts` are literal text blocks meant to be spliced
into a *different* project's `CLAUDE.md` and to inject `additionalContext` into a *different* agent's
context window on almost every tool call (e.g. "`ask_codebase(question)` — call this **as step 2 of
every task**..."; "REMEMBER: ... use `mcp__lgraph__update_graph` to record them"). This is exactly the
shape a prompt-injection payload takes — imperative instructions embedded in fetched repo content,
designed to steer a future agent's behavior. I did not execute, obey, or apply any of this text; it is
quoted above purely as a finding, per the untrusted-data instruction in this task. It is not disguised
or hidden (the README documents the feature openly), so it is not malicious in a covert sense, but it
is the identical mechanism a hostile actor would use, and it is worth flagging as a category, not just
this vendor.

**e) Minor/non-live:** `src/utils/machine-id.ts` computes a SHA-256(MAC address + platform)
fingerprint but is not imported anywhere else in this repo (dead code here — likely used by a sibling
VS Code extension the comments repeatedly reference as "matching extension's ..."). No `postinstall`/
`preinstall` script in `package.json` (checked directly) and no hardcoded secrets found via grep. So
the install itself is not immediately hostile — the danger is in the intended runtime behavior once
configured with a real API key, not in a supply-chain trick at `npm install` time.

---

## 4. Mechanism vs. repowise — feature comparison

latentgraph's 9 MCP tools map almost 1:1 onto tools we already have via repowise:

| latentgraph tool | repowise equivalent | Notes |
|---|---|---|
| `get_project_overview` | `get_overview` | Same shape (architecture summary + top modules) |
| `get_module_info` | `get_context` (module-scoped) | |
| `get_file` | `get_context` (file target, `verified` skeleton) | repowise's is source-verified against live tree; latentgraph's is a server-side cached snapshot that can drift ("degraded" fallback state exists precisely for this) |
| `get_dependencies` | `get_context(include=["callers"])` / `get_risk` | |
| `get_call_chain` | `get_symbol` + repowise's call-graph via `get_context` | latentgraph's is a dedicated call-chain walker with confidence scores — marginally richer here |
| `get_symbol` | `get_symbol` | Same purpose |
| `get_pr_insights` | `get_why` | Both surface "why is this shaped this way" — latentgraph's needs a GitHub token wired in for PR-derived invariants; repowise falls back to git archaeology with no extra auth |
| `ask_codebase` | `get_answer` | Same purpose; repowise's returns a `confidence` + verbatim `quotes`, which is stronger grounding than latentgraph's prose+citations |
| `update_graph` (write, queued for owner approval) | *(none)* | The one tool repowise doesn't have — a human-gated way to add curator notes back into the graph. Interesting as a **pattern**, not compelling enough to offset §3. |

Net: no capability gap repowise doesn't already close for us, and the one differentiator
(`update_graph`) is a nice-to-have curation feature, not a blocker we currently feel.

---

## 5. Is the value real for us? — Honest answer

**No.** Two independent reasons, either one sufficient alone:

1. **Redundant.** Feature-for-feature it duplicates repowise, which we already have wired in, free of
   the SaaS coupling, and already trusted (per our own CLAUDE.md instructions on how to use it). This
   is the same verdict we already reached for `codegraph-rust` (`docs/research/2026-07-04-codegraph-rust-teardown.md`,
   NO-GO) — another "yet another code graph" that loses to what we already run.
2. **Actively dangerous for us specifically.** We are mid-way through a live security posture
   (`secrets-exposure-incident-2026-07-03`: rotating creds after a prod Supabase leak; open-sourcing
   gated on secrets scrubbing) and hold a hard internal rule against exporting PII/proprietary logic to
   third parties without an explicit council (`owner-data-export-ai-2026-06-30`: ETHICAL-STOP on PII
   export even for *our own* export feature). Voluntarily uploading full source of a payments/RLS/auth
   codebase to an unaffiliated paid SaaS, PLUS granting that SaaS's server a live, auto-approved
   shell-execution channel into our dev machines, is squarely the risk class we're trying to close, not
   open.

**Recommendation: SKIP. Do not integrate, do not pilot, do not point it at this repo.** Nothing here
clears the bar for even a sandboxed trial — the "trial" itself is the exfiltration event.

**Patterns worth stealing (idea only, not the code, not the SaaS):** the hook-based auto-injection of
file/dependency context at `PreToolUse` on `Edit`/`Write` (so the agent sees blast-radius + invariants
*before* editing, not after being asked) is a reasonable UX idea we could someday implement as a *local*
hook against repowise's own `get_context`/`get_risk`/`get_why` — entirely self-hosted, no third-party
call-home. Not proposing this now; flagging it only because the mechanism (not the vendor) is sound
and the brief asked for concrete, nameable wins if any exist. This is the only one, and it's an idea,
not code to adopt.
