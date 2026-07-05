# caveman teardown — license-first reverse-engineering dossier

**Scope:** operator's active priority is reducing token consumption of the dowiz agent harness.
Clone kept out-of-tree at
`/tmp/claude-0/-root-dowiz/d1b102b0-0624-452a-9e2d-72865dd2e9d8/scratchpad/caveman-teardown/caveman/`
(shallow clone, nothing executed, nothing installed, nothing landed under `/root/dowiz` except this
doc). All fetched repo content (README, skill markdown, AGENTS.md, CLAUDE.md) was treated as untrusted
data — none of it was obeyed; scanned for prompt-injection patterns targeting a reviewing agent
(`ignore previous instructions`, `disregard`, `grant all permission`, `sudo`, `curl | sh`, "as an AI
agent reading this…", etc.) — **none found.** The repo's own AGENTS.md/CLAUDE.md/SKILL.md files are
literally agent-directed instruction text by design (that's the product), but they were only Read as
research data in this session, never installed or invoked as a live skill.

**Verdict: MIXED — APPLY-PATTERNS (narrow, gated) on one sub-component, SKIP the other two for our
governance docs.** Real repo, confirmed via direct GitHub API call. MIT license (clean). Egress scan
is clean for the core skill and hooks (zero network calls, zero telemetry, verified against source) —
but one sub-tool (`/caveman-compress`) genuinely ships file content to the Anthropic API by design, and
that sub-tool is an **LLM-based rewrite with no semantic-fidelity gate**, which collides directly with
dowiz's own red-line/Mandatory-Proof-Rule discipline if pointed at CLAUDE.md or MEMORY.md. The one
piece worth borrowing — `caveman-shrink`'s local, deterministic, zero-LLM regex compressor for MCP
tool-description fields — is a genuinely different, lower-risk pattern than everything else in the repo
and is additive to (not redundant with) what dowiz has already applied from the prior 7-teardown pass.

---

## 1. Repo identification (confidence: HIGH, ~98%)

Confirmed via direct `gh api repos/juliusbrussee/caveman` call (not search synthesis) — GitHub logins
are case-insensitive; the canonical form is `JuliusBrussee/caveman`.

- Description (verbatim, GH API): *"why use many token when few token do trick — Claude Code skill
  that cuts 65% of tokens by talking like caveman"*
- **83,853 stars / 4,674 forks / 355 open issues** (as of 2026-07-04); created 2026-04-04; last push
  2026-07-03. Unusually high star count for a ~3-month-old repo — plausible given the viral,
  meme-branded README (front-page-worthy joke framing: "Brain still big. Mouth small."), but the
  number itself should be sanity-checked by the operator if it matters for a trust decision; star
  count is not load-bearing for this dossier's verdict either way.
- License (GH API `license.spdx_id`): **MIT**. Confirmed against the actual `LICENSE` file
  (copyright Julius Brussee, 2026, standard MIT text).
- Topics: `ai, anthropic, caveman, claude, claude-code, llm, meme, prompt-engineering, skill, tokens`
  — self-tagged as a meme, not hiding that framing.
- What it actually is, confirmed by reading `README.md` and the source tree (not inferred from the
  name): a **prompt/skill package with three genuinely distinct mechanisms**, not one thing:
  1. `caveman` skill — a system-prompt instruction telling the agent itself to write terser replies
     (compresses **output** tokens, at generation time).
  2. `caveman-shrink` — a local MCP stdio proxy that regex-compresses `description` fields in
     `tools/list`/`prompts/list`/`resources/list` responses from a wrapped MCP server (compresses
     **tool-schema metadata**, deterministically, no LLM).
  3. `/caveman-compress <file>` — an LLM-based rewrite tool that sends a whole markdown file's prose
     to the Anthropic API and overwrites it with a "caveman-speak" version (compresses **memory-file
     input tokens**, once, persistently, every session after).

No ambiguity on identity — this is the only repo named `caveman` matching the description, and its
own README/CLAUDE.md corroborate the "token reduction skill" framing independently of the joke name.

---

## 2. License — read FIRST

**MIT**, confirmed via `gh api repos/juliusbrussee/caveman/license` and the raw `LICENSE` file text.

**Verdict by use:**
- **Reading/learning** — unrestricted.
- **Vendoring / copying patterns** (e.g. the `compress.js` regex compressor, or the validation-gate
  pattern in `validate.py`) — **permitted**. MIT requires only preserving copyright + license text; no
  copyleft, no field-of-use restriction, compatible with dowiz's AGPLv3 open-source target (ADR-020).
- **Running it locally** (installing the skill/plugin/MCP wrapper as-is) — **permitted**, same as
  above.

No loud flag on licensing. Moving to the load-bearing egress/safety check.

---

## 3. Egress / safety scan

Per the skill-adoption guardrail (scan-before-run): no `npm install`/plugin-install was performed —
static source review only, across every `.js`/`.ts`/`.py`/`.sh`/`.ps1` file in the tree.

**Findings by component:**

| Component | Network calls | Verdict |
|---|---|---|
| `skills/caveman/SKILL.md` (core output-compression skill) | None — it's a markdown prompt, no code | Clean |
| `src/hooks/*.js` (mode-tracker, `/caveman-stats`, statusline) | None found (`grep`'d for `fetch\|http\|https\.\|request(` — zero hits in hook source; the one `https://` string is a code **comment** pointing at Anthropic's public pricing page, not a call) | Clean — local file I/O only (session flag file, JSONL session log, statusline savings counter) |
| `src/mcp-servers/caveman-shrink/index.js` + `compress.js` (the MCP proxy) | None of its own. It `spawn()`s the upstream MCP server the *user* configures as a local child process, line-buffers its stdout, regex-transforms `description` fields in place, passes `tools/call` payloads and all client→server traffic through **unmodified**. Any network activity belongs to whatever server it wraps, not to caveman-shrink itself. | Clean — read the full 126-line proxy + 133-line compressor; confirmed no `fetch`/`http`/`net` anywhere |
| `skills/caveman-compress/scripts/compress.py` (`/caveman-compress`) | **Yes** — calls `anthropic.Anthropic().messages.create()` directly if `ANTHROPIC_API_KEY` is set, else shells out to `claude --print` (fixed arg list, no `shell=True`, file content passed via stdin not as a shell arg). This is the one real egress path in the repo. | Disclosed, gated, opt-in only (fires solely on explicit `/caveman-compress <file>` invocation) |
| `install.sh` / `bin/install.js` | `curl -fsSL raw.githubusercontent.com/.../install.sh \| bash` (documented, standard curl-pipe-bash pattern); installer shells out to per-agent CLIs (`claude plugin install`, `gemini extensions install`, `npx skills add`) which hit their own registries (GitHub/npm) | Install-time only; no `postinstall` script in `package.json` — nothing runs merely from cloning/npm-installing |

**Sensitive-file denylist (a real, load-bearing safety control):** `compress.py` refuses to compress
any file whose name matches a hard-coded pattern for credentials/secrets/keys (`.env*`, `.netrc`,
`credentials*`, `secrets*`, `passwords*`, `id_rsa`/`id_ed25519` etc., `*.pem/.key/.p12/.crt/...`) or
whose path contains `.ssh/.aws/.gnupg/.kube/.docker`, or whose (normalized) basename contains
`secret|credential|password|apikey|accesskey|token|privatekey` — **before the file is ever read**,
with an explicit comment: *"Compressing them ships raw bytes to the Anthropic API — a third-party data
boundary... this is a hard refuse before read."* A 500KB size cap applies too. This is good defensive
engineering and matches the repo's own honest `SECURITY.md` disclosure (self-reported Snyk "High Risk"
rating for in-place file rewriting + subprocess use, with a plain-language explanation of exactly what
triggers it and why it's not exploitable as scored).

**Telemetry claim ("Caveman has no telemetry. Zero.") — verified, not just asserted:** grepped the
whole tree for `telemetry|analytics|posthog|segment|sentry|mixpanel|amplitude`; the only hits are the
tool's own **local** JSONL session log and statusline file. No SDK, no beacon, no external endpoint
found anywhere outside the install-time GitHub/npm fetches and the disclosed `/caveman-compress` →
Anthropic API path.

**Egress verdict: SAFE, not disqualifying.** The core skill and hooks are genuinely zero-network. The
one real egress path (`/caveman-compress` → Anthropic API) is disclosed in the repo's own `SECURITY.md`
and `SKILL.md`, denylist-gated against obviously-sensitive filenames, and fires only on explicit user
invocation — not the "call-home you didn't ask for" pattern that would be automatically disqualifying.

---

## 4. Mechanism (reverse-engineered, three distinct sub-mechanisms — do not conflate them)

**(a) `caveman` skill — output-side behavioral compression.** Pure system-prompt instruction (no code):
tells the agent to drop articles/filler/hedges/pleasantries, use fragments, keep code/paths/errors
verbatim, and — notably — an explicit **"Auto-Clarity" exception list** that reverts to normal prose
for security warnings, irreversible-action confirmations, and any case where omitted conjunctions would
create ambiguous ordering. This is not a compression algorithm; it's a policy applied at generation
time, functionally identical in kind to dowiz's own "map-reduce distilled-return rule" (agents are
already told to return distilled, non-verbose output to the parent). Six intensity levels (`lite` →
`ultra` → `wenyan-*`), switchable per-session via `/caveman <level>`.

**(b) `caveman-shrink` — deterministic local MCP-proxy compression.** A ~126-line Node process that
sits between an MCP client and an upstream MCP server (`spawn()`'d as a child process), and regex-
compresses only the `description` string fields of `tools/list`/`prompts/list`/`resources/list`
responses. The underlying transform (`compress.js`) is a **pure lexical filter**: drop articles,
filler words, pleasantries, hedges, leading "I'll/let me/you can", collapse whitespace — with a
sentinel-substitution pass that protects fenced/inline code, URLs, filesystem paths, `CONST_CASE`
tokens, dotted method calls, function-call syntax, and semver strings from being touched at all
(matched out via regex before the prose transform runs, spliced back in after). No LLM in this loop —
fully deterministic, same input always yields the same output. `tools/call` payloads and all
client→server request traffic pass through completely unmodified in v1 (documented as a deliberate
scope limit — "high risk of breaking downstream parsing").

**(c) `/caveman-compress <file>` — LLM-based file rewrite.** A Python CLI (`compress.py`) that: strips
YAML frontmatter off locally via regex (preserved verbatim, re-prepended after — because the authors
found Claude "has a habit of stripping or rewriting these despite preserve-structure rules in the
prompt"), sends the remaining body to Claude with a prompt instructing it to compress natural language
while preserving code blocks/URLs/paths/commands exactly, then runs a **deterministic post-hoc
validator** (`validate.py`) checking: heading count+text match, code-block exact match, URL-set exact
match, inline-code multiset exact match (paths and bullet-count-within-15% are warnings only, not hard
failures) — with up to 2 retry/targeted-fix rounds and a full rollback to the original file (backup
written to an out-of-tree, platform-appropriate data dir, verified by readback before the primary file
is ever touched) if validation still fails after retries. This is the most sophisticated piece of
engineering in the repo — a genuine red→green structural gate around an inherently non-deterministic
LLM call — but the gate only checks **markdown-structural** fidelity, never semantic/normative fidelity
(see §6).

---

## 5. Measured vs. asserted savings

- **Output compression (a):** "65% output token reduction, average" is measured from a **committed
  benchmark suite** (`benchmarks/`, `evals/` — 10 prompts, real Claude API token counts, range 22–87%),
  not just claimed. The repo is unusually honest about the caveat: `docs/HONEST-NUMBERS.md` explicitly
  states the skill **only** shrinks output tokens — input/reasoning tokens are untouched, and the skill
  instructions themselves **add ~1–1.5k input tokens per turn** — and documents that already-terse
  workloads can go **net-negative** once that overhead is counted. This kind of self-undermining
  disclosure is a positive signal for trustworthiness of the rest of the numbers.
- **Memory-file compression (c):** "~46% input-token reduction," measured on 5 example memory files in
  a receipts table (706→285, 1145→535, 1122→636, 627→388, 888→560 chars). These are the authors' own
  test files, not independently audited, but the methodology (before/after char counts on committed
  sample files) is reproducible and transparent.
- **Tool-description compression (b):** **no published benchmark** found anywhere in the repo for
  `caveman-shrink` specifically — plausible because MCP tool descriptions are typically short already,
  so the absolute savings per call are likely small; this would need to be measured on our own traffic
  before trusting any number, since none exists to trust in the first place.

---

## 6. Lossy vs. lossless — CRITICAL, and the load-bearing question for correctness

**All three mechanisms are lossy by design; the degree and where the loss lands differs sharply:**

- **(a) and (b)** are **word/phrase-level** lossy: dropping articles/filler/pleasantries is generally
  meaning-preserving, but the `HEDGES` list in both the skill prompt and `compress.js` explicitly
  targets modal words — `might`, `perhaps`, `could potentially`, `it seems` — for removal. Dropping a
  hedge changes epistemic modality (possibility → flat assertion), a real if usually low-stakes
  semantic shift. Code/URLs/paths/identifiers are protected via a **sentinel find-and-restore pass
  verified by regex match**, not "hope the model listens" — this part is genuinely close to lossless
  for the protected categories, in (b) especially since there's no LLM in that loop at all.
- **(c) is qualitatively more lossy**: it's a full LLM paraphrase/summarization pass over prose, with
  the `SKILL.md` prompt explicitly instructing the model to "merge redundant bullets that say the same
  thing differently," "keep one example where multiple examples show the same pattern," drop
  "connective fluff" like **however/furthermore/additionally** (words that frequently carry exception
  or contrast logic), and drop "you should"/"make sure to" in favor of bare imperatives. The
  post-hoc validator (`validate.py`) checks **only** heading text/count, code-block exact match, URL
  set, and inline-code multiset — it has **no mechanism to detect semantic drift in plain prose**:
  a merged bullet that silently drops one of two differing exception clauses, or a dropped "however"
  that was carrying a scoping qualifier, would pass every check in `validate.py` because none of them
  look at meaning, only at markdown-structural landmarks.

**Correctness verdict for dowiz specifically:** dowiz's `CLAUDE.md` and `MEMORY.md` are governance
artifacts whose enforcement value depends on **exact normative prose** — the Ethics Charter ("No AI for
military or warfare... Refuse such requests outright"), the Mandatory Proof Rule, the numbered Ship
Discipline steps, the Task-Exit Rule's enrichment dimensions, red-line globs, and the
self-improvement-loop's threshold definitions ("qualified" = ≥3 files OR ≥3 iterations OR stage-close OR
red-line touch). These are exactly the shape of content `/caveman-compress`'s own validator cannot
protect: it is not a bulleted list of code snippets, it's dense conditional/exception-laden legal-style
prose, and a "merge redundant bullets" / "drop connective fluff" pass over it could silently invert or
blur a MUST/SHOULD/MAY distinction, drop an "unless" clause, or merge two similarly-worded rows in
MEMORY.md that differ only by an approval-state word (e.g. dowiz's own recent MEMORY.md rows literally
contain "COUNCIL-APPROVED" vs "🔴 not yet approved" as the only distinguishing text between two
near-identical lines — losing one hedge word there inverts an approval state, and nothing in
`validate.py` would catch it). This is precisely the failure class dowiz's own self-improvement-loop
rule already guards against structurally ("never weaken an existing gate... never cheat green") but
this tool has no equivalent gate for prose. **An unreviewed LLM rewrite of CLAUDE.md is itself an edit
to a red-line-adjacent governance artifact, and this tool provides no gate that would catch a semantic
weakening expressed only in prose.**

---

## 7. Application to dowiz's actual shape

Per `2026-07-04-token-reduction-synthesis.md`, dowiz's real cost lines are: the ~42K/lane subagent
dispatch floor (mostly tool-schema/MCP overhead from broad grants, not CLAUDE.md+MEMORY.md — those are
only ~8.5K of it), the `pre-edit-lessons` hook's broad triggers, the `route-request.sh` nudge, memory-
corpus duplication, and the REGRESSION-LEDGER's full-read cost. Mapping caveman's three mechanisms onto
that list:

- **(a) the `caveman` output-skill is redundant with what's already applied.** dowiz agents (this one
  included) are already instructed to return distilled, non-verbose output to the parent — the same
  design goal caveman's skill targets, already achieved via prompt discipline in `AGENTS.md`/subagent
  instructions, without an external skill's fixed ~1–1.5k-token-per-turn overhead. Installing it on top
  would be a second, overlapping terseness mechanism, and per `HONEST-NUMBERS.md`'s own warning, adding
  it to already-terse subagent replies (which dowiz's distilled-return rule already produces) is close
  to the exact "net-negative on already-terse workloads" scenario the authors flag.
- **(b) `caveman-shrink`'s pattern is additive, not redundant, and lower-risk than the other two.**
  None of the prior 7 teardowns (agentmemory, agentfiles, pxpipe, codegraph, halo, future-agi) touch
  MCP tool-*description*-metadata payload size specifically — this session's own deferred-tool list
  alone spans 150+ MCP tool names (Notion/Gmail/Google Drive/Playwright/browser-use/Sentry/etc.), and
  every one of those schemas loads into the harness's tool-call budget. The mechanism itself (local,
  zero-network, zero-LLM, sentinel-protected regex substitution, applied only to `description` fields,
  never to `tools/call` payloads) is a genuinely different and safer risk profile than either of the
  other two components. **Value is unmeasured, not proven** — no benchmark exists for it even in the
  source repo — so this is a "worth measuring, then maybe building our own," not an assumed win.
- **(c) `/caveman-compress` should not be pointed at any dowiz governance doc.** MEMORY.md is already
  written in exactly the compressed style this tool produces (fragment-heavy, article-dropping, short
  synonyms) — it would likely trip the tool's own "output identical to input" abort guard or yield
  near-zero savings; the 46% receipts number was measured on verbose prose, not an already-terse
  ledger. CLAUDE.md, by contrast, *is* the verbose normative prose shape where this tool would show real
  percentage savings — which is exactly the document it must not touch, per §6.

---

## 8. Recommendation: APPLY-PATTERNS (narrow) on (b); SKIP (a) and (c)

**Do:**
- Treat `caveman-shrink`'s design — local, zero-network, sentinel-protected lexical compression of MCP
  `tools/list` `description` fields only — as a pattern worth measuring against dowiz's own MCP tool
  surface (the 150+-tool deferred list) before building anything. If description-field bytes turn out
  to be a real fraction of the ~42K/lane floor, a small in-house equivalent (same sentinel-protect →
  strip-filler-words → restore shape, MIT-compatible to borrow directly if wanted) is low-risk to pilot
  dark, since it never touches `tools/call` payloads or model output.
- Borrow the **validation-gate pattern** (`validate.py`'s structural diff + retry + rollback-on-failure)
  as a general template for any future compress-in-place tooling dowiz builds for genuinely
  non-normative prose (e.g. a verbose research-doc backlog, not CLAUDE.md/MEMORY.md) — the
  backup-before-write + readback-verification + abort-on-identical/empty-output discipline in
  `compress.py` is solid defensive engineering worth copying regardless of whether the LLM-rewrite
  mechanism itself is used.

**Do not:**
- Install the `caveman` output-compression skill — redundant with the existing distilled-return
  discipline, adds fixed per-turn overhead the authors themselves warn can go net-negative.
- Run `/caveman-compress` on `CLAUDE.md`, `MEMORY.md`, or any file under `docs/regressions/` or
  `docs/adr/` — the validator has no semantic-fidelity check, and these are exactly the red-line-
  adjacent governance documents where a silently dropped "unless"/hedge/exception clause is
  indistinguishable, to the validator, from a clean compression.
- Wire anything under `.claude/` directly — protected; any of the above would need a proposal + the
  same doubt-escalation/council gate already in flight for other harness-touching changes
  ("🔴 per-surface councils remain" per current memory), not an agent applying it unilaterally.

**Why APPLY-PATTERNS and not PILOT/INTEGRATE:** unlike pxpipe (clean license + egress + a proven,
code-verified per-model scoping fit for an exact stated goal), caveman's most relevant-looking
component for our shape (memory-file compression) is precisely the one with no defense against the
specific failure mode our governance docs are most exposed to — so a gated pilot of it would be piloting
the wrong thing. The one component that IS safely pilotable (`caveman-shrink`) has no existing
benchmark to justify skipping straight to "pilot" over "measure our own traffic first, then maybe
build our own minimal version" — hence apply-patterns rather than a direct dependency.

**Why not full SKIP:** the repo is honestly engineered (self-disclosed Snyk flag with a real
explanation, an explicit net-negative-scenario writeup, a hard sensitive-filename denylist before any
network call, a validator gate on the one LLM-rewrite path) and one sub-component's pattern (local,
zero-LLM, zero-network MCP description compression) is a genuinely novel angle relative to everything
dowiz has already evaluated in the prior 7-teardown pass. Dismissing the whole repo because two of its
three mechanisms are wrong for our red-line docs would throw away the one piece that isn't.

LAST-REVIEWED: 2026-07-04
