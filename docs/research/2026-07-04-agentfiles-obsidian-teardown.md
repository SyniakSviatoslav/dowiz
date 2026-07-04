# agentfiles (Obsidian) — License-First Reverse-Engineering Dossier

**Date:** 2026-07-04
**Requested by:** operator, for the dowiz agent harness (token-consumption reduction — TOP priority)
**Emphasis axes:** (1) metrics collection, (2) context-health, (3) token-spending
**Scope discipline:** cloned only into `/tmp/claude-0/-root-dowiz/d1b102b0-0624-452a-9e2d-72865dd2e9d8/scratchpad/agentfiles-teardown/`. Nothing vendored into the product tree. This doc is the only artifact under `/root/dowiz`. No memory file, lesson, reflection, or ledger row was edited or deleted — every proposal below is a candidate for the operator to apply, not an applied change.
**Grounding:** read `/root/.claude/projects/-root-dowiz/memory/MEMORY.md` (122-file index) + two sample files (`audit-remediation-orchestration-2026-07-03.md`, `memory-corpus-meta-patterns-2026-07-02.md`) before starting, and cross-referenced the sibling dossiers already in this directory — `2026-07-04-agentmemory-teardown.md` (patterns for the memory *engine* shape) and `2026-07-04-harness-token-audit.md` (the exact, MEASURED byte/token numbers for our current corpus). This dossier does not re-derive those numbers; it cites them and shows where agentfiles/skillkit's mechanisms attach to them.

---

## 0. TL;DR

- **Canonical pick:** [`Railly/agentfiles`](https://github.com/Railly/agentfiles) — **MIT**, 669 stars, 41 forks, active (last push 2026-05-17, `updated_at` 2026-07-04 same day as this dossier), single-author (Railly Hugo) TypeScript Obsidian plugin. It is exactly what the name suggests: an **Obsidian plugin that manages AI agent skill/command/agent files** (Claude Code, Cursor, Codex, Windsurf, Copilot, and 9 more), from inside a vault. No ambiguity — one clear candidate, high confidence.
- **The real analytics engine is a separate MIT repo it shells out to**, not bundled here: `@crafter/skillkit` ([crafter-station/skill-kit](https://github.com/crafter-station/skill-kit) on GitHub, same author/org). agentfiles' dashboard is a thin UI (`src/views/dashboard.ts`) over `skillkit`'s CLI JSON output (`src/skillkit.ts`, `execSync(...) --json`). The metrics/context-health/token-spend **schemas** (the actual deliverable of axes 1–3) live in agentfiles' own TypeScript interfaces, which mirror skillkit's JSON contract byte-for-byte — extracted below without needing to clone the second repo.
- **Scan result: clean.** No install-time code execution (no postinstall/preinstall/prepare hooks anywhere in the tree). Outbound network calls exist but are all **user-initiated** (marketplace search/install/update against `skills.sh` + GitHub raw/API, triggered by clicking a button) — nothing fires automatically on load. `skillkit` itself is explicitly local-only, no telemetry, SQLite at `~/.skillkit/analytics.db`.
- **Direct structural match to our harness:** agentfiles' `SkillType` enum already includes `"memory"` as a first-class scanned kind, and its Conversation Explorer feature (`src/conversations/parser.ts`) parses **exactly our directory shape** — `~/.claude/projects/<encoded-path>/*.jsonl`, the same JSONL session-transcript format `/root/.claude/projects/-root-dowiz/` uses. This is not a loose analogy; it is the same file format.
- **Recommendation: apply patterns, don't integrate.** It's an Obsidian plugin; we have no Obsidian vault in the loop, and the "install skillkit" path is a new global npm CLI + SQLite DB most useful for an interactive human browsing session, not our subagent-dispatch harness. The three mechanisms worth lifting are: (a) mine *already-existing* logs for usage counts instead of adding new instrumentation, (b) a `health`-style JSON contract (staleness / budget-% / oversized / duplicate-conflict warnings) over our own doc-stores, and (c) an `always_loaded` token-tax breakdown with a hard budget, refreshed as a script rather than a one-off audit.

---

## 1. Repo identification

GitHub search for "agentfiles" + Obsidian returned one dominant, unambiguous hit plus several unrelated near-namesakes (a different "hide AGENTS.md from the file tree" plugin, an unrelated ACP bridge, a knowledge-ingestion pipeline). None of those compete for the name; `Railly/agentfiles` is the only repo that is *literally* "agentfiles" and is Obsidian-native.

| Field | Value |
|---|---|
| Repo | [`Railly/agentfiles`](https://github.com/Railly/agentfiles) |
| Description (GitHub) | "Browse, create, and edit AI agent files across Claude Code, Cursor, Codex, and 13+ tools — from Obsidian." |
| Stars / forks / watchers | 669 / 41 / 669 |
| Language | TypeScript |
| Created | 2026-03-28 |
| Last push | 2026-05-17 (`5857a50e`, "chore(deps): bump next to 16.2.6") |
| Open issues | 3 |
| License (GitHub API) | **MIT** |
| Homepage | https://agentfiles.crafter.run |
| Topics | agent-skills, ai-agents, ai-tools, claude-code, codex, coding-agents, copilot, cursor, developer-tools, dotfiles, obsidian, obsidian-plugin, skills, windsurf |

**Confidence: high.** README read in full (`README.md`, 66 lines) confirms the GitHub metadata verbatim: "AI skills manager for Obsidian. Browse, create, and manage skills across Claude Code, Cursor, Codex, Windsurf, and 10+ coding agents." Screenshots in the README show exactly the dashboard described below ("Dashboard with burn rate, context tax, and health metrics"). No confirmation gap — this is not a "best guess," it's a direct hit.

**One companion repo worth naming, not a competing candidate:** [`crafter-station/skill-kit`](https://github.com/crafter-station/skill-kit) (npm `@crafter/skillkit`, also MIT, same author org "Crafter Station"). agentfiles' own README says the dashboard "requires skillkit" — it's an optional CLI dependency, not part of the agentfiles repo, and not itself named "agentfiles." Flagged here because its JSON output contract is where axes 1–3's actual schemas live.

---

## 2. License — FIRST, before anything else

**`Railly/agentfiles`: MIT License**, copyright (c) 2026 Railly Hugo. Confirmed by `LICENSE` file text (standard MIT boilerplate) and GitHub API's `license.spdx_id: "MIT"`.

- **Reading / learning:** unrestricted.
- **Vendoring / integrating:** permitted (must retain the copyright/permission notice; no other condition). **No legal blocker** — this repo, in isolation, is the most permissive tier.
- **`@crafter/skillkit`** (the companion analytics CLI): also **MIT** per its npm registry metadata (`"license": "MIT"`, repo `crafter-station/skill-kit`). Same verdict.

**Verdict: apply-patterns or vendor, either is legally clean.** The recommendation below to *not* vendor either package is an architecture-fit judgment (§7), not a licensing one — same honest framing as the sibling `agentmemory` dossier.

---

## 3. Scan result (skill-adoption guardrail, before reverse-engineering)

Cloned `Railly/agentfiles` (depth 1) into the scratchpad only. Checked for import-time execution, network egress, and credential/fs access per the standing discipline (this is a *lower*-egress-risk shape than a proxy tool — an editor-side Obsidian plugin, not a network service — but scanned in full regardless):

- **`package.json` scripts:** `dev`/`build`/`lint` only (`node esbuild.config.mjs [production]`, `eslint src/`). **No `postinstall`/`preinstall`/`prepare` hook anywhere in the tree** (grepped repo-wide). Nothing runs at `npm install` / plugin-install time beyond Obsidian loading the built `main.js` when the user enables the plugin — which is the same trust boundary as any other Obsidian community plugin (Obsidian itself gates plugin installs behind an explicit community-plugin toggle + a "desktop only, reads files outside your vault" README warning the user must accept).
- **Outbound network calls** (`grep -rn requestUrl`, `src/marketplace.ts` only): `https://skills.sh/api/search` (marketplace search, fired only when the user types ≥2 chars in the marketplace search box), `https://api.github.com/repos/{source}` + `.../git/trees/{branch}` (resolving a marketplace skill's GitHub source when the user clicks "install"), `https://raw.githubusercontent.com/{source}/{branch}/{path}` (fetching that skill's `SKILL.md` content). All three are **explicit, user-initiated actions** (search box input / install button), not automatic on load or on a timer. No calls fire from `onload()` (read in full — see §4, `src/main.ts`): it only scans local disk (`this.refreshStore()`) and starts an `fs.watch` file watcher.
- **Shell execution:** `child_process.execSync`/`exec` appears in two files — `src/skillkit.ts` (shells out to a locally-installed `skillkit` binary it discovers via a PATH search across every known package-manager bin dir: npm/pnpm/yarn/bun/volta/asdf/mise/nvm/fnm/proto — read in full, no network, no eval of untrusted strings, just `execSync("<found-path> <fixed-subcommand> --json")`) and `src/marketplace.ts` (shells out to `npx`/`bunx skills add|remove|update`, the `skills.sh` CLI, again a fixed command template with a user-supplied skill-source string interpolated — no `eval`, no `new Function`, argument goes into a shell string but is drawn from GitHub-repo-shaped marketplace results, not arbitrary user text). No `eval(`, no `new Function(` anywhere in `src/`.
- **Credential/secret surface:** grepped for `api[_-]?key|secret|token|password|credential` — every hit is either UI/type labels for **our own** "auth"-tagged conversation-tagger keyword list (`src/conversations/tagger.ts`, tags conversations that *mention* auth/login/jwt — doesn't read or transmit any) or the `tokens`/token-*count* fields in the metrics schema itself (§4). **No API keys, no credential files, no `.env` parsing found anywhere in `src/`.**
- **Filesystem access outside the vault:** by design and disclosed in the README ("Desktop only... reads files outside your vault") — it scans `~/.claude/`, `~/.cursor/`, `~/.codex/`, etc. for skill/command/agent files, and separately `~/.claude/projects/*/*.jsonl` for the Conversation Explorer. This is read-only scanning (`readFileSync`/`readdirSync`/`statSync`) except for its own edit/create/delete actions on skill files that the user explicitly triggers, and its own settings file (`this.saveData(this.settings)`, Obsidian's own plugin-data API, sandboxed to the plugin's own JSON blob).

**Verdict: clean.** No import-time execution risk, no automatic phone-home, no credential handling. Lower egress risk than a network-facing tool by construction (it's a local editor plugin); the marketplace network surface is opt-in-per-click and points only at a public skills registry + GitHub, never at agentfiles' own maintainers.

---

## 4. Architecture — the three emphasis axes

agentfiles itself is a thin presentation layer; the actual metrics/health/token engine is `skillkit`, invoked as `execSync("<bin> <subcommand> --json")` and parsed into agentfiles' own TypeScript interfaces (`src/views/dashboard.ts`, `src/types.ts`, `src/skillkit.ts`). Those interfaces are a verbatim, verified JSON contract — reading them **is** reading skillkit's data model, without needing skillkit's own source. Confirmed independently against the npm package description and skillkit's README (fetched, not cloned, since it's a companion package rather than the target repo): CLI commands are `scan`, `list`, `stats`, `health`, `prune [--yes]`, `burn`, `conflicts [--dry-run]`, `coverage <skill-path>`, `trace <prompt>` / `trace --list --skill <name> --limit N`.

### 4a. Metrics collection

**What's measured, concretely (verbatim field names from `src/views/dashboard.ts` + `src/skillkit.ts`):**

```ts
// skillkit stats --json
interface StatsJson {
  period: { days: number };
  total_invocations: number;
  unique_skills: number;
  most_active_day: string;
  streak?: { current: number; longest: number };
  velocity?: { this_week: number; last_week: number; change_pct: number };
  top_skills: { name: string; total: number; daily: { date: string; count: number }[] }[];
}

// skillkit trace --list --skill <name> --limit 5 --json  (per-invocation record)
{ trace_id, timestamp, tokens_total, cost_estimate, duration_ms, model }
```

**How it's collected without burning tokens — the central mechanism, and the one most worth borrowing:** skillkit does **not** instrument anything inline. It is a periodic, offline, read-only parser over transcripts that already exist: Claude Code's own session JSONL files (the same `tool_use` blocks in `~/.claude/projects/*/*.jsonl` that agentfiles' *own* Conversation Explorer parses, confirmed identical format — see `src/conversations/parser.ts`, which reads `entry.type === "assistant"` messages and extracts `tool_use` blocks by `b.type === "tool_use" && b.name`) plus, for other tools, their native session stores (OpenCode's SQLite `part` table with `tool: skill` rows). **Collection cost = zero added tokens, because it's a batch job over data the agent already produced for other reasons** (its own transcript), run on-demand (`skillkit scan`) rather than during the session. The dashboard itself caches results to `~/.skillkit/dashboard-cache.json` (`src/views/dashboard.ts:99-121`) so re-opening the panel doesn't even re-invoke the CLI.

**Filtering discipline:** both agentfiles and skillkit maintain an identical hardcoded `BUILTIN_TOOL_NAMES` exclusion set (Read/Write/Edit/Bash/Glob/Grep/Task/Agent/MCP-prefixed names, etc. — `src/skillkit.ts:6-17` and `src/views/dashboard.ts:7-18`, byte-identical lists in two files) so that generic tool calls never pollute "skill" usage counts — only genuinely named skills/commands/agents are counted.

### 4b. Context-health

```ts
// skillkit health --json
interface HealthJson {
  installed: number;
  agents: string[];
  db: { exists: boolean; events: number };
  usage: { used_30d: number; unused_30d: number; never_used: string[] };
  metadata: { total_chars: number; budget: number; pct: number };  // e.g. "12.5K / 16.0K" → pct
  content: { total_chars: number };
  warnings: {
    oversized: { name: string; lines: number }[];
    long_descriptions: { name: string; chars: number }[];
  };
}
// skillkit conflicts --dry-run --json  (semantic/trigger collision, not embedding-confirmed — "black box" per skillkit's own docs)
{ pairs: { skill_a: string; skill_b: string; similarity: number }[] }
```

Health is a composite of **four distinct signals**, each independently actionable:
1. **Staleness / orphans** — `usage.never_used` (zero invocations ever) vs. `usage.unused_30d` (a rolling 30-day window) — two different thresholds, not one. The dashboard renders `never_used` as a scrollable list (capped at 20 + "N more"), and offers a one-click `prune --yes` action gated behind a confirm modal (`src/views/confirm-modal.ts`) — **destructive action is always human-confirmed, never automatic**, even though the underlying detection is fully automated.
2. **Metadata budget overrun** — `metadata.total_chars / metadata.budget` as a percentage, rendered as a horizontal bar that turns red past 80% (`fill.addClass("as-budget-over")` at `pct > 80`, `src/views/dashboard.ts:343`). This is a **fixed byte/char budget for the always-loaded metadata layer** (frontmatter name+description across all installed skills), separate from full file content (`content.total_chars` is tracked but not budgeted the same way).
3. **Oversized / verbose files** — `warnings.oversized` (line-count threshold on the full skill body) and `warnings.long_descriptions` (char-count threshold specifically on the frontmatter `description:` field) are tracked as **two separate warning classes**, because a long body and a long always-loaded description have different cost profiles (body only loads when the skill fires; description loads every time the skill catalog is enumerated).
4. **Duplication / conflict** — `conflicts --dry-run` returns pairwise `similarity` scores between skill descriptions ("trigger collision testing" per skillkit's own docs — two skills whose descriptions would both plausibly fire on the same prompt, wasting either a double-load or an ambiguous routing decision). Mechanism internals aren't published (skillkit's own README calls it "a black box" beyond the example), but the contract (pairs + a similarity float) is enough to reproduce independently with a much cheaper method (see §5).

Action model: **flag → human decides → prune is one explicit confirmed click**, never silent. This matches our own "never weaken a gate," "no false-green," and "don't delete without a human" postures almost exactly.

### 4c. Token-spending

```ts
// skillkit context --json — the "Context Tax" panel
interface ContextJson {
  always_loaded: { total_tokens: number; claude_md_tokens: number; skill_metadata_tokens: number; memory_tokens: number };
  cost_per_call: { first_call_cache_write: number; subsequent_cache_read: number };
  session_estimate: { with_cache: number; without_cache: number; savings_pct: number };
  sources: { name: string; tokens: number }[];
}
// skillkit burn --json — cost attribution
interface BurnAgent {
  agent: string;
  cost: { total: number };
  period: { days: number; sessions: number; api_calls: number };
  by_day: { date: string; costUsd: number }[];
  by_model: { model: string; apiCalls: number; costUsd: number }[];
}
```

**`always_loaded` is the single most transferable idea in this whole dossier**: it explicitly decomposes "everything that loads into context regardless of what the task needs" into three named buckets — `claude_md_tokens`, `skill_metadata_tokens`, `memory_tokens` — rendered as a stacked bar with a legend (`src/views/dashboard.ts:385-414`), so the always-paid floor is visible as a *number per source*, not one opaque total. `cost_per_call` separately models prompt-caching economics (first-call cache-write cost vs. every-subsequent-call cache-read cost) and `session_estimate.savings_pct` quantifies exactly how much caching saves — i.e., it doesn't just measure raw tokens, it measures **dollar cost under the actual caching regime**, the same distinction our own token audit already drew when it labeled some figures MEASURED vs. estimated.

**Token counting method — confirmed cheap and non-circular:** `src/views/detail.ts:14-16`:
```ts
function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}
```
A plain chars÷4 heuristic, computed client-side with zero LLM calls, used both for the per-skill "~N tokens" badge in the detail view and (per skillkit's own README) for the health/context budget math. **This is the exact same heuristic `docs/research/2026-07-04-harness-token-audit.md` already uses and validates** ("All 'est. tokens' figures use chars ÷ 4 ... Caveat: Cyrillic ... 1.3–1.8× higher"). Independent convergence on the identical cheap heuristic from two unrelated projects is a meaningful corroboration that chars/4 is the right zero-cost default, with the same caveat we already flagged.

**Reduction mechanism, honestly assessed:** neither agentfiles nor skillkit does *active* token reduction (no top-k retrieval, no summarization tiers, no dynamic loading) — they are **measurement and human-actioned pruning only**. The actual reduction lever exposed to the user is exactly one: `skillkit prune --yes` deletes skills unused in 30 days, which stops their metadata from being in the always-loaded catalog going forward. Everything else (stats/health/burn/context) is visibility, not enforcement. This is a meaningful finding on its own: **the "token-spending" axis in this project is entirely an accounting/attribution layer, not a retrieval-optimization engine** — contrast with the `agentmemory` dossier's token-budget greedy-packer, which *actively* decides what gets injected. The two dossiers are complementary, not overlapping: agentmemory shows *how to actively shrink* what loads; agentfiles/skillkit shows *how to cheaply and continuously measure* what's loading and flag it for a human to prune.

---

## 5. Apply to us — concrete, mergeable with the running token audit

Grounding numbers below are **cited from `docs/research/2026-07-04-harness-token-audit.md`**, not re-derived: CLAUDE.md 19,016 B ≈ 4,754 tok; MEMORY.md 15,194 B ≈ 3,799 tok (guaranteed per-session floor ≈ 8,553 tok, MEASURED); memory corpus 122 files / 541,655 B total; `docs/regressions/REGRESSION-LEDGER.md` 113,407 B ≈ 28,352 tok (loaded wholesale by `librarian.md`/`pattern-critic.md`, append-only, un-shrinkable); `docs/lessons/` 10 files, 26,083 B, hook fires 464 times/2.5 days (`.claude/logs/harness-events.jsonl`, MEASURED); duplication-candidate clusters already identified (redteam 8 files/15,014 B; polish/QA 8 files/33,189 B; tooling-eval 6 files/25,149 B; staging-audit 4 files/13,003 B; meta-loop-governance 5 files/~52,000 B — ≈146,000 B / 27% of the corpus total).

### (a) Memory corpus (`memory/*.md` + `MEMORY.md`) — the primary target

**5a-1. Metrics to add (frontmatter proposal — NOT applied, no memory file touched):**

```yaml
---
name: <slug>
description: "..."
metadata:
  node_type: memory
  type: project | reference
  originSessionId: <uuid>
  # PROPOSED additions (computed by a script, never hand-authored):
  created: 2026-07-04            # from filename date or git log --diff-filter=A
  est_tokens: 3799                 # chars/4 heuristic, computed at write/scan time — same formula as §4c
  last_cited: 2026-07-03           # most recent session JSONL transcript that mentions this file's slug or [[wikilink]]
  cite_count_30d: 4                # count of the above, rolling 30d window — direct analog of health.usage
  inbound_links: 3                 # count of other memory/*.md files or MEMORY.md lines linking [[this-file]]
---
```

**5a-2. Metrics collection mechanism (the "how, without burning tokens" answer):** mine two **already-existing** logs, exactly mirroring skillkit's "parse existing session data, add zero new instrumentation" principle:
- `.claude/logs/harness-events.jsonl` — already logs every `pre-edit-lessons` fire per lesson (464 in 2.5 days, per the token audit's Finding 2/3); a script can already compute per-lesson invocation counts and staleness from data that exists *today*, with zero new logging code.
- `~/.claude/projects/-root-dowiz/*.jsonl` (Claude Code's own session transcripts — the exact same file format agentfiles' `src/conversations/parser.ts` reads) — grep each transcript's assistant-role text for mentions of a memory file's slug or `[[wikilink]]` target. A session that cites `[[deploy-topology]]` counts as a "recall" of that memory the same way skillkit counts a `Skill` tool-use as an invocation of that skill.
- **Both are batch/offline jobs run on demand**, never inline during a session — same zero-token-cost collection model as §4a.

**5a-3. Context-health checks to run** (candidates only — propose, don't prune):
- **Duplication/conflict pass, skillkit-`conflicts`-style but embedding-free:** run a cheap Jaccard token-overlap over each memory file's `description:` field + title (the same trick `agentmemory`'s `auto-forget.ts` uses for contradiction detection, no ML dependency needed) across all 121 files. Expected top hits, *predicted from the existing byte-cluster analysis rather than re-scanned here*: the 5 clusters the token audit's Finding 5 already named by filename-grouping (redteam/polish-QA/tooling-eval/staging-audit/meta-loop-governance) should also surface as the top-`similarity` pairs — this would be the first real test of whether a content-level similarity check agrees with the filename-heuristic grouping already done manually.
- **Oversized-file / long-description warnings, skillkit's exact two-class split:** flag files over a line/byte threshold (candidates from the token audit: `rebuild-decision-rust-astro-2026-07-04.md` 24,971 B, `audit-remediation-orchestration-2026-07-03.md` 20,780 B, `error-contract-council-2026-06-26.md` 15,029 B) as "oversized body," and separately flag any `description:` frontmatter field over, say, 400 chars as "long description" — our memory files already carry a `description:` field structurally identical to a `SKILL.md`'s, so this check is a direct, zero-adaptation port of skillkit's own logic.
- **Orphan detection** (an Obsidian-native concept skillkit doesn't even need to invent — Obsidian itself ships "orphan notes" as a core feature): any `memory/*.md` file with **zero inbound links** from `MEMORY.md` or any other memory file. Given MEMORY.md is described in the corpus's own meta-patterns doc as "already lean... 1 line/file," a true orphan would mean a memory file that isn't indexed at all — worth a one-time sanity check that every file on disk has a corresponding MEMORY.md line (a simple set-difference between `ls memory/*.md` and the wikilink targets parsed out of MEMORY.md).
- **Staleness, `never_used`/`unused_30d` two-tier, mapped onto the 4 "staging audit" files** the token audit flagged as "closed, one-time... low ongoing recall value" — these are exactly skillkit's `never_used` candidates (topic closed, likely zero citations in any transcript since), pending confirmation via 5a-2's citation-mining script before any merge/archive decision.

**5a-4. Token-spend accounting to add:** a repeatable `always_loaded`-style breakdown, refreshed on demand instead of hand-computed once (as the token audit currently is):
```
CLAUDE.md tokens:            4,754  (MEASURED)
MEMORY.md tokens:             3,799  (MEASURED)
——— guaranteed floor:         8,553
Memory corpus (if fully recalled):  ~135,400  (541,655 B / 4, NOT normally paid — cited for scale only)
Duplication-cluster overhead:  ~36,500  (146,000 B / 4 — paid only when clusters are jointly recalled)
```
This is the same shape as `ContextJson.always_loaded` (named buckets, not one opaque total) plus the `sources: {name, tokens}[]` array for anything beyond the guaranteed floor — a direct, low-effort port.

### (b) `docs/lessons/`, `docs/reflections/`, `docs/regressions/`

- **`docs/lessons/`** — the harness-events.jsonl mining (5a-2) applies identically and is the single cheapest win here: cross-reference the 464 fires against `docs/lessons/INDEX.md`'s trigger table to get skillkit's exact `stats.top_skills` (per-lesson fire count) for free, from a log that already exists. Two lessons with the broad `docs/**` and `packages/db/migrations/**` triggers almost certainly dominate the count (matching the token audit's own flag) — this is the `health.warnings`-equivalent finding: an over-broad "always matches" pattern, the lesson-store analog of an oversized/over-loaded skill.
- **`docs/reflections/{INBOX,ARCHIVE,RETRO}`** — a `health`-style check specific to this store: INBOX is *designed* to stay near-empty (triaged quickly per CLAUDE.md's self-improvement loop). A one-line script reporting `{inbox_count, oldest_inbox_file_age_days}` is the direct analog of skillkit's `usage.never_used` list — except here a *non-zero, non-fresh* INBOX is the unhealthy signal (the reverse polarity of "skill never fired": here, "reflection never triaged").
- **`docs/regressions/REGRESSION-LEDGER.md`** — this is our closest real-world analog to skillkit's `metadata.pct` "over 80% budget" warning: 28,352 tok today, append-only, growing forever (the ratchet rule forbids shrinking it), read wholesale by `librarian.md`/`pattern-critic.md`. Propose an explicit **token budget threshold** (e.g., "INDEX table must stay under 5,000 tok; full ledger only reachable via grep/expand") and track `current_tok / budget` as a first-class percentage each time a row is appended — mirroring skillkit's exact `metadata.total_chars / metadata.budget → pct` computation, red-lined past 80%. The token audit already proposed the INDEX-table fix (Finding row 7); this dossier adds the *ongoing percentage tracking* skillkit does that a one-off audit doesn't.

### (c) Script vs. manual

- **Should be scripts (`scripts/`, new files, non-`.claude`, safe to build):**
  1. `scripts/memory-metrics.mjs` — mines `.claude/logs/harness-events.jsonl` (lesson fires) + `~/.claude/projects/-root-dowiz/*.jsonl` (memory-file citation counts) → per-file/per-lesson invocation counts + staleness, exactly §5a-2. Read-only, deterministic, zero new tokens burned to run it.
  2. `scripts/context-tax.mjs` — computes the `always_loaded`-style breakdown (§5a-4) using the already-validated chars/4 heuristic, appends a dated row to a small trend log instead of a one-off report, so token-cost drift is visible over time rather than re-audited from scratch each time.
  3. `scripts/memory-health.mjs` — the duplication/oversized/orphan/staleness checks (§5a-3), Jaccard-based (no embedding dependency, matching agentmemory's own zero-ML contradiction detector), output-only (never deletes or merges — that decision stays human).
- **Inherently manual (propose, don't wire):** any actual merge/archive/prune of a memory file, lesson, or ledger row. This mirrors both skillkit's own design (detection is automatic, `prune` requires an explicit `--yes` + a confirm-modal in the UI) and our own standing rule (never weaken a gate, never delete without a human, "no files were deleted" discipline already followed in the token audit). `.claude/` (hooks, settings) is protected — none of the three scripts above need to touch it; they are read-only analyzers that a human (or the `librarian` agent, at its existing trigger points) runs and interprets, not new hooks.

---

## 6. Integrate vs. apply-patterns — recommendation

**Apply patterns, don't integrate.** Argued honestly:

- **License doesn't block it** (§2) — MIT permits vendoring outright, for both agentfiles and skillkit. Not a legal "no."
- **The runtime shape is the mismatch, same category of reason as the `agentmemory` dossier's verdict.** agentfiles is an *Obsidian plugin* — its entire distribution mechanism (`main.js`/`manifest.json`/`styles.css` into `<vault>/.obsidian/plugins/`) presupposes a running Obsidian vault, which is not part of our harness. `skillkit` is closer to directly usable (a standalone global CLI + local SQLite DB, no vault needed) but its value proposition — a human browsing a dashboard for burn-rate/streaks/sparklines — doesn't fit a mostly-autonomous multi-lane subagent harness where no one is watching a UI panel in real time.
- **What we'd gain by installing `@crafter/skillkit` directly (cheap, worth flagging as a *separate*, smaller option):** it already parses Claude Code's exact JSONL format and already computes stats/health/burn/context for **skills** — and since our `.claude/skills/`, `.claude/commands/`, `.claude/agents/` directories are real Claude Code skill/agent files in the same shape skillkit expects, `npm i -g @crafter/skillkit && skillkit scan` would work **today, unmodified**, against our actual skill/agent catalog (not memory — skillkit doesn't know about our bespoke `memory/*.md`/MEMORY.md shape, only about `SKILL.md`/agent files). This is a legitimate low-cost trial, distinct from "integrate agentfiles" — flagged for the operator to consider separately, outside this dossier's scope (installing a new global npm tool is itself a decision, not something to silently do).
- **What we lose by not adopting either wholesale:** we don't get a real UI dashboard, sparklines, or a maintained SQLite analytics store. The three scripts proposed in §5c are each tens of lines and purpose-built for our two-tier corpus (memory files + doc-stores) rather than skillkit's skill-file-only model — proportionate to problem size, same judgment call as the `agentmemory` dossier reached for its own five patterns.
- **Complementary, not redundant, with the `agentmemory` dossier already in this directory:** that dossier's patterns (token-budget greedy-packer, dedup-at-write, decay/eviction sweep) are about *actively deciding what to load*; this dossier's patterns (mine existing logs for usage, health-check contract, always_loaded token-tax with a budget) are about *cheaply and continuously measuring what's loading and flagging it for a human*. Implementing both is not double work — agentmemory's packer needs exactly the per-entry token/recency data this dossier's `context-tax.mjs` and `memory-metrics.mjs` would produce as inputs.

**Concrete next step (not part of this dossier's scope, flagged for a separate task):** build `scripts/memory-metrics.mjs` and `scripts/context-tax.mjs` first (both are pure log-mining, zero risk, zero new dependency) — they immediately validate or correct the token audit's Finding 5 duplication clusters with real citation data instead of filename-heuristic grouping, before any merge decision is made.

---

## Sources

- [Railly/agentfiles](https://github.com/Railly/agentfiles) — canonical pick, MIT
- [Railly/agentfiles README](https://github.com/Railly/agentfiles/blob/main/README.md) — read in full, local clone
- `src/main.ts`, `src/scanner.ts`, `src/skillkit.ts`, `src/marketplace.ts`, `src/watcher.ts`, `src/types.ts`, `src/views/dashboard.ts`, `src/views/detail.ts`, `src/conversations/parser.ts` — read in full from the scratchpad clone
- [crafter-station/skill-kit](https://github.com/crafter-station/skill-kit) (npm `@crafter/skillkit`) — companion analytics engine, MIT; README fetched (not cloned) for CLI command confirmation
- [Skill Kit: Local Analytics for AI Agent Skills — railly.dev blog](https://www.railly.dev/blog/skill-kit) — architecture description, fetched
- npm registry metadata for `@crafter/skillkit@0.10.6`
- `/root/.claude/projects/-root-dowiz/memory/MEMORY.md` and two sample memory files (grounding, read-only, no edits)
- `docs/research/2026-07-04-agentmemory-teardown.md` (sibling dossier — cited for complementary patterns, not re-derived)
- `docs/research/2026-07-04-harness-token-audit.md` (sibling audit — all byte/token figures cited from here, not re-measured)
