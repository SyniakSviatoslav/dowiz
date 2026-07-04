# pxpipe teardown — license-first reverse-engineering dossier

**Scope:** operator's explicit goal is to REDUCE TOKEN CONSUMPTION of **claude-fable-5 specifically**,
using pxpipe **only** for that model. Clone kept out-of-tree at
`/tmp/claude-0/-root-dowiz/d1b102b0-0624-452a-9e2d-72865dd2e9d8/scratchpad/pxpipe-teardown/pxpipe/`
(shallow clone, nothing executed, nothing landed under `/root/dowiz` except this doc).

**Verdict: PILOT — dark, gated to non-red-line lanes only.** Real repo, confirmed via direct GitHub
API call (not just search synthesis). MIT license (clean). Egress scan is clean — the only network
destinations in source are `api.anthropic.com` / `api.openai.com` (the same upstream the harness
already talks to) plus `127.0.0.1` (local dashboard); telemetry is local-disk-only. Per-model scoping
to `claude-fable-5` is a **built-in, code-verified feature**, not a stretch fit. The blocker to full
integration is not licensing or egress — it's that the compression is **lossy by design** with a
documented "silent confabulation" failure mode on exact strings, which conflicts with this harness's
red-line invariants (money/RLS/auth/migrations/bulk-edit) and Mandatory Proof Rule. Recommend piloting
it only on non-red-line, long-context reasoning lanes, never on lanes expected to recall hashes/IDs/
secrets/exact line-numbers verbatim, and re-measuring savings on the harness's own traffic rather than
trusting the upstream README's headline numbers.

---

## 1. Repo identification (confidence: HIGH, ~95%)

Searched GitHub for `pxpipe`, `px-pipe`, and "pxpipe token reduction / prompt compression / proxy"
across three independent queries. Cross-checked the top hit with a direct `gh api
repos/teamchong/pxpipe` call (not search-engine synthesis) and a `WebFetch` of the README — both
independently corroborate the search summaries.

**Picked: [`teamchong/pxpipe`](https://github.com/teamchong/pxpipe)**
- Description (verbatim, from GH API): *"cut Fable 5 token usage by rendering text context as images"*
- 1,626 stars / 85 forks / 35 open issues (as of 2026-07-04); created 2026-05-20; **last push
  2026-07-04T02:48:40Z — commits landed the same day this dossier was written.** Actively maintained,
  effectively single-owner (`teamchong` accounts for ~all weekly commit activity per
  `gh api .../stats/participation`).
- License (GH API `licenses.mit`): **MIT**.
- What it actually does, confirmed by reading `README.md`, `FINDINGS.md`, and `src/`: it is a **local
  HTTP proxy** that sits between an Anthropic/OpenAI-API client (Claude Code, or anything pointed at
  `ANTHROPIC_BASE_URL`) and the real API. It intercepts `POST /v1/messages` (and OpenAI-compatible
  paths), rewrites the bulky, low-information-density parts of the outgoing request — the static
  system-prompt + tool-docs slab, large `tool_result` bodies, and older collapsed conversation
  history behind the live tail — into dense PNG images, and forwards the rewritten request upstream.
  It never touches the model's *response*; only the outbound request is compressed.

**Other candidates found and ruled out** (per the instruction to list every plausible match):

| Candidate | What it is | Verdict |
|---|---|---|
| `xpipe-io/xpipe` | Desktop tool for accessing server infra (SSH/shells) from a local UI | Unrelated — "xpipe" not "pxpipe"; no LLM/token angle |
| `xpipe-io/xpipe-webtop` | Containerized web desktop for XPipe | Same project family as above, unrelated |
| `aandaleon/Ad_PX_pipe` | Bioinformatics pipeline (GWAS/PrediXcan in admixed populations) | Unrelated domain entirely — "PX" here means PrediXcan, not proxy |
| `npow/kompact` | **"LLM context compression proxy — 40-70% token savings, zero code changes"** | Same problem space, different name/repo. Not "pxpipe," so out of scope for this dossier, but flagged as a same-space alternative worth a look if pxpipe's pilot doesn't pan out |

No ambiguity here: `teamchong/pxpipe` is the only repo actually named "pxpipe," and its own description/
README explicitly targets "Fable 5" (Claude) token reduction — an exact match to the operator's stated
goal, not an inferred one. Confidence is HIGH rather than absolute only because verification relied on
`gh api` + `WebFetch` rather than a human-supplied canonical URL; the operator should still spot-check
the URL themselves before any pilot lands.

---

## 2. License — read FIRST

**MIT**, confirmed two ways: GitHub's license detector (`gh api repos/teamchong/pxpipe` →
`license.spdx_id: "MIT"`) and the actual `LICENSE` file in the clone (standard MIT text, copyright
"claude-image-proxy contributors", 2026). `package.json` also declares `"license": "MIT"`.

**Verdict by use:**
- **Reading/learning** — unrestricted.
- **Vendoring / depending on it** (npm dep, or copying `transformAnthropicMessages` etc. into our own
  code) — **permitted**. MIT only requires preserving the copyright notice and license text; no
  copyleft, no field-of-use restriction, compatible with dowiz's AGPLv3 open-source target (ADR-020).
- **Running it locally as an external dev-tool proxy** (the likely pilot shape) — **permitted**,
  same as above, no additional constraint.

No loud flag here — this is the clean case. Moving to the load-bearing check.

---

## 3. Egress / safety scan (load-bearing — this tool sits in the path of ALL Fable-5 prompt+completion traffic)

Per the skill-adoption guardrail: scanned before running anything. No `pnpm install`/build/execute was
performed — static source review only.

**Network destinations found in source** (`grep -rnoE "https?://[a-zA-Z0-9.-]+"` across `src/`, `bin/`,
`scripts/`, excluding the vendored `htmx` JS blob which is inert client-side UI code):

| Destination | Where | Verdict |
|---|---|---|
| `api.anthropic.com` | `src/core/proxy.ts`, `src/node.ts`, `src/worker.ts` | **Expected** — this IS the upstream the proxy forwards to. Same destination the harness already sends to. |
| `api.openai.com` | same files | Expected — OpenAI-compatible upstream for the GPT code path (unused if we only route Fable-5/Anthropic traffic here). |
| `127.0.0.1` (various ports, default `47821`) | `src/node.ts`, `src/stats.ts` | Local-only dashboard/health endpoints. Not egress. |
| `unpkg.com` | `scripts/vendor-ui.mjs` | **Build-time only** — a one-off dev script (`pnpm run vendor:ui`) that fetches a CSS/JS vendor bundle to check in. Not part of the running proxy; never executes against live traffic. |
| `docs.anthropic.com`, `docs.claude.com`, `www.w3.org` | comment strings / an inline SVG namespace URI | Not network calls — literal string/URI text, not fetched. |

**No third-party telemetry/analytics found.** Grepped case-insensitively for
`telemetry|analytics|posthog|segment|sentry|mixpanel|amplitude|update-notifier|checkForUpdates` across
the whole tree — every "telemetry" hit refers to the tool's own **local** JSONL event log
(`~/.pxpipe/events.jsonl`), never an external SDK or beacon.

**Does it exfiltrate prompt/completion content?** No. `src/core/tracker.ts` (the only event-emission
code path) is explicitly documented in its own header comment: *"Never emits raw text; only sizes,
counts, durations, env fields, and sha256 prefixes."* Verified against the actual `TrackEvent` struct —
every field is a count, duration, boolean, model name, or `sha8` hash prefix. One nuance worth flagging
for local hygiene (not egress): on 4xx upstream errors, the tracker can persist a **gzip+base64 sample
of the already-imaged outgoing request body** — inline up to 32 KiB, or as a local sidecar file above
that — purely for local debugging. This never leaves the machine, but it means `~/.pxpipe/` should be
treated as potentially containing prompt content after error events, same as any local debug log.

**Credential access:** no reads of `~/.ssh`, `~/.aws`, `.env`, or any credential-store path found
anywhere in `src/`. The proxy forwards whatever `Authorization`/`x-api-key` header the client already
sends (standard transparent-proxy behavior — it doesn't need to read the key from disk, and doesn't).

**Execution / supply chain:**
- No `postinstall`/`preinstall` script in `package.json` — nothing runs at install time.
- `child_process` usage is limited to two benign, non-networked calls in `src/node.ts`:
  `spawnSync('git', ...)` (reads current branch/repo state for report attribution) and
  `spawnSync('open', [outDir], ...)` (best-effort opens a local report folder in Finder/Explorer,
  non-fatal on failure).
- `bin/cli.js` is a 7-line shim that just dynamically imports the bundled `dist/node.js` — no
  import-time side effects.
- `.npmrc` sets `minimum-release-age=4320` (3 days) — a deliberate pnpm supply-chain hardening measure
  against typosquat/compromise windows, with an explicit comment explaining why. This is an unusually
  security-conscious default for an npm package and a positive maintenance signal.

**Egress verdict: SAFE.** The only traffic leaving the machine is exactly the same traffic the operator
already sends to Anthropic (transformed in place, forwarded to the same destination) — no third
recipient, no phone-home, no telemetry beacon, no credential harvesting. Nothing here would have been
disqualifying under the "any call-home is disqualifying" bar.

---

## 4. Mechanism (reverse-engineered)

pxpipe is a **client-side local HTTP proxy** (runs as a Node process via `npx pxpipe-proxy`, or
deployable as a Cloudflare Worker) — not a change to the model, not a server-side Anthropic feature.
It exploits a real pricing asymmetry: an image's token cost is fixed by its pixel dimensions, not by
how much text is inside it, while dense text (code/JSON/logs) tokenizes at roughly 1 char/token. A
1928×1928 PNG costs ≈4,761 vision tokens and can hold ≈92,000 characters (≈19.3 chars/token) — so
imaging wins whenever content is denser than ~19 chars/token; Claude Code's real traffic runs ~1.91
chars/token per the maintainer's N=391 sample, comfortably inside the profitable zone.

**What it compresses** (three gated categories, each behind a profitability check):
1. Large `tool_result` bodies (file reads, command output, logs) above ~6k chars of dense content.
2. Older conversation history behind the live tail — collapsed into synthetic image "pages"; **recent
   turns always stay text.**
3. The static system-prompt + tool-docs slab (re-rendered once, cached).

**What it never touches:** the user's live/current messages, the model's output (streamed back
unmodified — "pxpipe compresses the request only, never the model's output"), sparse prose, and
anything too small to be profitable. It also preserves Anthropic prompt-cache ordering (`cache_control`
markers/static prefix kept stable) so caching discounts still apply on top of the image compression.

This is **prompt/context compression via a lossy image-encoding transform**, not KV-cache tricks,
not semantic summarization via an LLM call, and not literal caching/dedup (though it composes with
Anthropic's native prompt caching rather than replacing it). It is a request-rewriting proxy, the
simplest of the architectures the task asked to distinguish between.

A library API also exists (`transformAnthropicMessages`, `renderTextToPngs` from `src/core/index.ts`)
for embedding the transform directly into a client without running the standalone proxy.

---

## 5. Measured savings — methodology and honesty check

The maintainer's methodology is **measured, not merely asserted**, and is independently reproducible:
for every `/v1/messages` request, the proxy fires a parallel, free `count_tokens` probe against the
**original, uncompressed** body (the counterfactual baseline) while the real (compressed) request goes
out, and reads Anthropic's actually-billed `usage` block off the real response. Both land in the same
row of `~/.pxpipe/events.jsonl`, so there's no turn-count/run-to-run confound, and the formula is
documented in `src/core/baseline.ts` for anyone to re-derive from their own log.

**Headline numbers (self-reported by the maintainer, on their own production workload):**
- 59% end-to-end bill reduction on a 13,709-request snapshot ($100 → ~$41); a later 8,904-request
  trace measured ~70%. Compressed-portion-only savings run higher (~72–74%), quoted separately from
  the end-to-end headline (a common way compression tools inflate numbers — this one explicitly
  avoids that).
- A single session demo: $42.21 (plain) vs $6.06 (pxpipe) on an identical task, ending at 73.5k/1M
  context used (pxpipe) vs 96% full (plain).
- Novel-arithmetic benchmark (content the model can't have memorized): 100/100 correct on both plain
  and imaged text for `claude-fable-5`, at **−38% tokens**.
- SWE-bench Lite pilot: 10/10 task completion parity, both arms, at **−65% request size**. SWE-bench
  Pro: 14/19 (pxpipe) vs 15/19 (plain), verdicts agree 18/19, the single divergent case re-resolved
  3/3 identically on replication (attributed to run-to-run variance, not the compression) — small n,
  receipts checked into `eval/swe-bench-pro/receipts/`.

**Honesty assessment:** these are the author's own numbers on the author's own workload — not
independently audited by a third party — but the methodology is transparent, reproducible, and the
repo ships actual receipt directories (`eval/verbatim-15/`, `eval/gist-recall/`, `eval/swe-bench-pro/`,
`eval/results-opus/`) rather than just README claims. The `FINDINGS.md` changelog reads like a genuine
research log (multiple dated "Update" entries correcting earlier conclusions, including a
"reframe"/"corrected verdict" entry) rather than marketing copy. Recommend re-measuring on this
harness's own traffic via the local dashboard/JSONL log rather than trusting the headline percentages
to transfer — the maintainer says this explicitly too ("the durable number is the token cut itself...
reproduce it on your own log").

---

## 6. Fable-5 scoping path — is per-model scoping possible, or all-or-nothing?

**Per-model scoping is a built-in, code-level feature — not a stretch fit.** Confirmed by reading
`src/core/applicability.ts`:

```ts
const DEFAULT_MODEL_BASES = ['claude-fable-5', 'gpt-5.6'];
```

Resolution order: an in-memory dashboard override, else the `PXPIPE_MODELS` env var (comma-separated
model-base list), else this built-in default. Setting `PXPIPE_MODELS=claude-fable-5` restricts
imaging/compression to **only** `claude-fable-5` requests; the proxy inspects the `model` field on
every inbound `/v1/messages` body and passes every other model through **byte-identical** —
confirmed in the `isAllowed()` gate and the README's FAQ ("Models outside the allowlist pass through
entirely"). Opus and other models are intentionally excluded from the default scope already, because
the maintainer's own evals show them reading imaged content worse — which happens to line up with this
harness's model-routing policy (Fable=reasoning/main-loop, Opus=reviewer, others=doers) wanting exactly
this discrimination.

**Where this would hook in the dowiz harness:** the harness's models are invoked via one client
pointed at an Anthropic-API-compatible base URL per session/task (the operator's `/model` selection,
or the Agent tool's model override param going through the same underlying client transport). The
integration point is:

1. Run `npx pxpipe-proxy` as a local sidecar process (or a Cloudflare Worker if wanting it always-on
   off-box) — proxy listens on `127.0.0.1:47821` by default.
2. Set `PXPIPE_MODELS=claude-fable-5` in the proxy's environment so it images **only** Fable-5 traffic;
   any Opus-reviewer or doer-model call that happens to transit the same proxy passes through
   untouched.
3. Point the Fable-5-issuing client at the proxy via `ANTHROPIC_BASE_URL=http://127.0.0.1:47821`.

Because `.claude/` hooks and settings files are **PROTECTED** (propose, don't wire), step 3's actual
environment wiring is **not something an agent should apply directly** — it requires the operator to
set `ANTHROPIC_BASE_URL` in their own shell/session launch config, or an explicitly-approved change to
session-launch tooling outside `.claude/`. This dossier proposes the hook point; it does not wire it.

**Is scoping all-or-nothing at the network layer?** No — the model-based allowlist inside the proxy
does the discrimination, not network routing, so even if every model's traffic shared one
`ANTHROPIC_BASE_URL` pointed at the same proxy, `PXPIPE_MODELS=claude-fable-5` alone would confine the
lossy transform to Fable-5 calls. If the harness instead already uses distinct clients/base-URLs per
model, only the Fable-5 client needs pointing at the proxy at all (redundant with the env-var gate, but
gives defense-in-depth). Either way: **per-model scoping is real and code-verified, not all-or-nothing.**

---

## 7. Correctness risk (for a coding agent with red-line invariants)

**This is explicitly, self-documented lossy compression**, and the failure mode is the worst kind for
an agent harness: **silent confabulation**, not a loud error.

- Verbatim/exact-string recall from imaged content degrades measurably: 12-character hex-string recall
  off dense renders scored **13/15** on `claude-fable-5`, **0/15** on Opus. A documented real-world
  failure (not just a benchmark): the model recalled a person's name from imaged chat history and got
  it "confidently wrong. No error, just a plausible wrong name."
- Gist-level recall (decisions, values, paths, names, negations, with distractors, across 15k–45k
  char sessions) scored 98/98 on both plain and imaged arms — broad semantic content survives well.
  It is specifically **byte-exact tokens** (hashes, IDs, secrets, exact numbers) that are at risk.
- Mitigating factors already built in: live/recent turns and the user's current message are **never**
  imaged — only backlog history, large tool outputs, and the static prompt/tool-docs slab are. The
  maintainer explicitly recommends routing byte-exact work around the tool (e.g., via
  `CLAUDE_CODE_SUBAGENT_MODEL` override to a non-allowlisted model, or `model: sonnet` in subagent
  frontmatter) rather than trusting imaged recall for it.

**Verdict for this harness specifically:** dowiz's own Ethics Charter/CLAUDE.md defines hard red-line
globs (auth, money, RLS, `packages/db/migrations/`, bulk-edit) and a Mandatory Proof Rule requiring
exact evidence (file:line, exact test names, exact command output) for every change — precisely the
kind of byte-exact content this tool's own findings say is at risk. A silent wrong hex/ID/path in a
red-line session is not a hypothetical here; it is the documented failure mode. **Recommend gating any
pilot to non-red-line lanes only** — long-context reasoning/exploration/docs-research sessions where
gist retention matters more than byte-exact recall (a real chunk of what Fable-5 does as the harness's
reasoning model) — and hard-excluding it from any session expected to touch money/auth/RLS/migrations/
bulk-edit, or to recall a hash/UUID/secret/exact line-number verbatim later in the same session.

---

## 8. Recommendation: PILOT (gated), not full integrate, not skip

**Do:**
- Pilot pxpipe as a local, opt-in sidecar proxy for non-red-line Fable-5 sessions only (long-context
  reasoning, research, exploration — not code edits touching red-line globs).
- Set `PXPIPE_MODELS=claude-fable-5` explicitly (do not rely on the built-in default, which also
  includes `gpt-5.6` — irrelevant here but keep the scope minimal and explicit).
- Measure real savings on this harness's own traffic via `~/.pxpipe/events.jsonl` / the local dashboard
  before trusting the upstream README's headline percentages.
- Treat `~/.pxpipe/` as potentially containing prompt-content debug samples (4xx sidecar files) —
  apply the same handling discipline as any other local debug log with prompt content in it.

**Do not (yet):**
- Wire `ANTHROPIC_BASE_URL` into `.claude/settings.json` or any protected harness config directly —
  that requires explicit operator action outside this dossier, per the protected-config rule.
- Enable it for any session/subagent touching money, auth, RLS, `packages/db/migrations/`, or
  bulk-edit — the red-line globs this repo already defines.
- Rely on it for verbatim recall of hashes, secrets, exact IDs, or line numbers that get diffed or
  compared byte-for-byte later in the same session.

**Why PILOT and not INTEGRATE:** license and egress are both clean, and per-model scoping is a genuine,
code-verified fit for the stated goal — there's no reason to "learn only." But the tool is five weeks
old (created 2026-05-20), the savings numbers are self-reported on the author's own workload, and the
one documented real-world failure (confabulated name recall) is exactly the risk class this harness
cannot afford in red-line lanes. A gated pilot with the harness's own measurement, confined to
low-stakes lanes, is the right amount of trust for a tool that sits in the path of all Fable-5 prompt
and completion traffic.

**Why not SKIP:** this is a rare case where the tool actually is what its name and description claim
before any code was read — MIT-licensed, actively maintained (commits the same day as this dossier),
no exfiltration, and a built-in single-model allowlist that maps directly onto the operator's exact
ask. Skipping it without a pilot would be leaving a legitimately clean, on-target tool on the table.

LAST-REVIEWED: 2026-07-04
