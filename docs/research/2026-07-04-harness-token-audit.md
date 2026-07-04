# Harness Token-Cost Audit — dowiz / DeliveryOS (2026-07-04)

> Read-only measurement pass. Goal: a ranked, numbers-first map of where session/subagent
> tokens actually go, so token-reduction work targets the biggest lines first — not the
> most visible ones.

## Heuristic (stated once)

All "est. tokens" figures use **chars ÷ 4** on the raw UTF-8 byte/char count of the actual
injected text (not the whole source file, unless the whole file is what gets injected).
Caveat: several injected strings (router nudges, doubt-pass reminders) are **Ukrainian
Cyrillic**. Cyrillic tokenizes worse than this heuristic assumes (BPE splits multi-byte
UTF-8 runs more finely than 4 chars/token) — real cost on those specific lines is likely
**1.3–1.8× higher** than the number shown. English/code figures (CLAUDE.md, agent defs,
memory corpus) are close to the true chars/4 ratio. Two numbers in this report are
**directly measured**, not estimated (flagged `MEASURED`): the `subagent_tokens` total
Claude Code itself reported for a real dispatch, and the harness's own hook-fire counts
from `.claude/logs/harness-events.jsonl`.

---

## Ranked cost table

| # | Cost line | Est. tokens | Frequency | Reduction lever | Est. savings | Risk |
|---|-----------|-------------|-----------|------------------|---------------|------|
| 1 | **Fixed per-subagent-dispatch overhead** (tool-schemas + CLAUDE.md + MEMORY.md + skills catalog + base system prompt) | **~42,000 MEASURED** for a zero-tool-call, ~150-word dispatch (see Finding 4) | every `Agent`/Task-tool spawn | Narrow-tool custom agents instead of `general-purpose`("All tools"); trim global MCP connectors granted to this project; move CLAUDE.md sections agents don't need into an on-demand doc | ~15,000–25,000 tok/lane (est., ~40-60% of the fixed floor) | Low — pure plumbing, no behavior change |
| 2 | **CLAUDE.md full-text injection** | 19,016 B ≈ **4,754 tok** | every session + every subagent spawn (MEASURED via probe) | Split into CORE (Ethics/Ship/Task-Exit — must stay resident) + REFERENCE (Repowise tool-guide, hotspot tables, guided tour — move to `docs/`, load on demand) | ~2,000–2,500 tok/session×lane | Low if done carefully; Medium if a used rule is accidentally moved out |
| 3 | **MEMORY.md full injection** | 15,194 B ≈ **3,799 tok** | every session + every subagent spawn (MEASURED via probe) | Prune/merge the 8-file redteam cluster + 4-file staging-audit cluster + 8-file polish/QA cluster down to 1 line each (they already ARE mostly 1 line each in MEMORY.md — the win is capping total line count, e.g. archive lines for topics closed >2 weeks with no open thread) | ~1,000–1,500 tok/session if ~30 stale lines retired to an ATTIC pointer | Low — MEMORY.md lines are pointers, not authority |
| 4 | **Skills catalog + deferred-tools list** (unprompted, in every session/subagent) | est. **~3,000–4,500 tok** (skills) + **~1,000–1,500 tok** (deferred-tool names) | every session + every subagent spawn (MEASURED presence, estimated size) | Mostly platform/global, not repo-controlled; but unused global MCP connectors (Gmail/Notion/Calendar/Drive/Sentry/Figma/etc., visible in the deferred-tools list) can be disabled per-project | ~2,000–4,000 tok if global connectors trimmed | Low (repo can't disable platform skills; CAN disable unused MCP servers) |
| 5 | **`pre-edit-lessons` hook injections** | ~195 tok/injection (780 B avg ACTION+LINK text) | **464 fires MEASURED** in 2.5 days (`harness-events.jsonl`, since 07-02) — `docs/**` and `packages/db/migrations/**` triggers are broad and fire on nearly every doc/migration edit | Narrow the `docs/**` trigger (it currently fires on EVERY docs write, including this very report) to only the sub-paths the lesson is actually about; dedupe rows pointing at the same lesson file | ~40,000–70,000 tok reclaimed over the same 2.5-day window (464 × ~195 tok) if trigger set is narrowed ~50% | Low — advisory only, no gate weakened |
| 6 | **`route-request` router nudges** (UserPromptSubmit, fires on every user turn) | nudge-serious ≈ 75 tok, nudge-repeat ≈ 43 tok (chars/4; **real cost likely 1.3–1.8× higher**, Cyrillic) | **383 fires MEASURED** in 2.5 days (194 serious + 189 repeat); prior in-repo telemetry (2026-07-03 session) already logged **274 nudges in one session** | Already-identified fix not yet shipped: prior telemetry report's own P3 recommendation — "context-aware serious-gate/repeat nudges … cuts ~250 low-signal nudges" (cited verbatim below) | ~15,000–25,000 tok/session (the P3 estimate, independently corroborated by this audit's per-event math: 250 × ~90 real tok ≈ 22,500) | Low — friction-only hook, tightening the regex doesn't remove the gate |
| 7 | **`docs/regressions/REGRESSION-LEDGER.md`** full-file reads | 113,407 B ≈ **28,352 tok** whole file (78 rows, avg 1,351 B/row) | Whenever `librarian.md` or `pattern-critic.md` runs (both instruct reading the whole ledger) — append-only, **grows every qualifying fix forever** | Add a compact INDEX table (id·one-line-symptom·guardrail-path, ~100 B/row ≈ 7,800 B total) at the top; librarian/pattern-critic read the INDEX + grep only the matched row's prose instead of the whole file | ~20,000+ tok/librarian-or-pattern-critic-run, growing every week the ledger grows | Medium — must keep full prose reachable (grep/row-id), never delete ledger content (ratchet rule) |
| 8 | **`red-line-doubt-gate` advisory injections** | ~150 tok/injection (600 B doubt-pass reminder) | **59 fires MEASURED** in 2.5 days on auth/money/RLS/migration-adjacent paths | Broad regex (`price\|money\|payment\|cash\|ledger\|tax\|payout\|refund\|invoice` etc.) — narrow to files that are ACTUALLY money/auth logic, not anything with those words in a path/comment | ~3,000–5,000 tok/2.5-day window | Low |
| 9 | **`.claude/agents/*.md` system prompts** | 50,959 B / 15 agents ≈ **849 tok avg**, up to 1,872 tok (`counsel.md`) | once per dispatch of that specific agent type | Already lean; low priority | marginal | N/A |
| 10 | **`docs/lessons/*.md` full files** (not just ACTION) | 26,083 B / 10 files ≈ 2,608 B avg ≈ 652 tok/file | only if an agent Reads a lesson file directly instead of relying on the hook's ACTION/LINK extraction | None needed — the hook already does the cheap thing; flag if any agent is seen `Read`-ing lesson files wholesale | n/a | N/A |
| 11 | **`docs/reflections/{INBOX,ARCHIVE,RETRO}`** | INBOX 47,973 B (12 files) + ARCHIVE 12,243 B (6 files) + RETRO 3,347 B (1 file) ≈ **15,900 tok** if all read | `cause-critic`/`pattern-critic`/council retros read across INBOX | Archive resolved INBOX items promptly (INBOX should be near-empty between triggers per the design); low volume today, watch for growth | n/a today | Low |

---

## Finding 1 — Per-session fixed context load

| Item | Size | Est. tokens |
|---|---|---|
| `/root/dowiz/.claude/CLAUDE.md` | 19,016 B / 235 lines | 4,754 |
| `/root/.claude/projects/-root-dowiz/memory/MEMORY.md` | 15,194 B / 107 lines | 3,799 |
| `AGENTS.md` (project root) | 7,555 B | 1,889 — **NOT auto-loaded**; CLAUDE.md says agents apply it "when reading" it, i.e. on-demand only (confirmed: not part of the injected system-reminder in the probe) |
| Full memory corpus `*.md` (122 files incl. MEMORY.md) | **541,655 B total**, 122 files | ~135,400 tok if the WHOLE corpus were loaded — **it is not**; only MEMORY.md's 107 index lines are guaranteed, individual files are recalled on demand |
| Largest 10 corpus files | `rebuild-decision-rust-astro-2026-07-04.md` 24,971 B; `audit-remediation-orchestration-2026-07-03.md` 20,780 B; `error-contract-council-2026-06-26.md` 15,029 B; `merge-to-main-plan-2026-07-02.md` 12,602 B; `p6-provisioning-vertical-2026-06-28.md` 12,074 B; `memory-corpus-meta-patterns-2026-07-02.md` 11,445 B; `full-project-analysis-2026-06-27.md` 11,349 B; `pg-privilege-hardening-2026-06-29.md` 10,416 B; `menu-characteristics-model-2026-06-29.md` 9,213 B; `storefront-audit-2026-06-30.md` 8,919 B | each ≈ 2,000–6,240 tok if recalled |

**Guaranteed per-session floor from this measure alone: CLAUDE.md + MEMORY.md ≈ 8,553 tokens**, before any tool use. This was **empirically confirmed** by a diagnostic subagent probe (Finding 4) — both files appeared verbatim, unprompted, before the subagent took any action.

---

## Finding 2 — Hook-injected context

9 hooks registered in `.claude/settings.json`. Only 4 inject conversation-visible text (`additionalContext` / deny reasons); the rest are silent pass/block plumbing.

| Hook | Trigger | Injects text? | Measured fires (2.5 days, `harness-events.jsonl`) | Avg injected size |
|---|---|---|---|---|
| `route-request.sh` | UserPromptSubmit (every turn) | Yes, on regex match | 194 `nudge-serious` + 189 `nudge-repeat` | 489 B / 289 B (UTF-8) |
| `pre-edit-lessons.sh` | PreToolUse Edit/Write/MultiEdit | Yes, on TRIGGER glob/errsig match | **464** `inject` | ACTION avg 651 B across 10 lessons (367–966 B range) |
| `red-line-doubt-gate.sh` | PreToolUse Edit/Write/MultiEdit | Yes, advisory reminder | 59 `advise`, 1 `deny` | ~600 B reminder |
| `serious-gate.sh` | PreToolUse Edit/Write/MultiEdit | Only on **deny** (block reason) | 42 `allow` (silent), 0 `deny` logged in this window | ~750 B reason (when it fires) |
| `loop-detector.sh` | PostToolUse (Bash/Edit/Write/MultiEdit) | Only at N=3 failures | 0 in this window | ~700 B directive |
| `require-classification.sh` | Stop | Only on **block** | 303 `pass` (silent), **4 `block`** | ~400 B JSON reason, but each block forces the agent to author a whole new `CHANGE-MANIFEST.md` or `*.reflection.md` (hundreds–low-thousands of tokens of NEW forced content, not just the reminder text) |
| `guard-bash.sh` | PreToolUse Bash | Only on block | **41 `block`** | short stderr message + a wasted/retried tool call |
| `protect-paths.sh` / `post-edit-gates.sh` | PreToolUse/PostToolUse Edit/Write | Only on block | not separately logged | short stderr message |

`docs/lessons/INDEX.md` (10 rows, 1,631 B) is the trigger table the `pre-edit-lessons` hook parses; two triggers (`docs/**`, `packages/db/migrations/**`) are broad enough to fire on almost any doc or migration write — this is why `pre-edit-lessons` is the single most-fired injecting hook (464 in 2.5 days, more than route-request's two nudges combined).

---

## Finding 3 — Telemetry (real numbers, not adjectives)

- `.claude/logs/exec-history.jsonl` does **not exist**; `scripts/telemetry-analyze.mjs` exists but has no input file to analyze in this checkout — its P1 (unified telemetry) is still pending per the memory corpus.
- `.claude/logs/harness-events.jsonl` (286,579 B, 1,297 lines, 2026-07-02T07:35 → 2026-07-04T19:16) is the **only live telemetry** and was analyzed directly:

  | hook | event | count |
  |---|---|---|
  | pre-edit-lessons | inject | 464 |
  | require-classification | pass | 303 |
  | route-request | nudge-serious | 194 |
  | route-request | nudge-repeat | 189 |
  | red-line-gate | advise | 59 |
  | serious-gate | allow | 42 |
  | guard-bash | block | 41 |
  | require-classification | block | 4 |
  | red-line-gate | deny | 1 |

  Proxy denominator: 58 commits landed in the same window (2026-07-02→04) → **~22 hook events and ~8 lesson-injections per commit-worth of work**.

- **Prior telemetry report** (delivered in-session 2026-07-03, cited in `audit-remediation-orchestration-2026-07-03.md:168-170`, verbatim):
  > "TELEMETRY REPORT (delivered in-session): ~2.57M subagent tokens/908 tools/17 lanes/3 commits. Time bottlenecks: safe-reversal-impl 28min, awwwards 21min, design 20min; pre-commit Docker ~4-5min/commit (serial). Token: 41% on research/verify. Gaps: guard friction (#1), nudge noise, 2 agent-init failures, commit serialization."

  Same session also logged **"69 blocks + 274 nudges this session, mostly on legit work"** — directly consistent with this audit's 2.5-day totals (41 guard-bash blocks + 4 classification blocks ≈ 45 blocks; 383 nudges), meaning that one session alone accounted for the large majority of the 2.5-day nudge volume. The prior report already prescribed the fix (P3, quoted above) — **not yet shipped**: `route-request.sh`'s SERIOUS/REPEAT regexes are unchanged keyword lists, still broad.
  - 17 lanes × ~2.57M/17 ≈ **151,000 subagent tokens/lane** average that session (consistent with — and higher than — this audit's directly-measured 42K-token *zero-op* floor per lane; the gap is real task work, i.e. ~109K tok/lane of actual research/build/verify content, matching the reported "41% on research/verify").

---

## Finding 4 — Subagent dispatch overhead (directly probed)

A single diagnostic subagent (`general-purpose`, zero tool calls, ~150-word answer) was dispatched to introspect its own context. Result — **MEASURED, `subagent_tokens: 41,981`** for that no-op dispatch. It confirmed, unprompted and before taking any action:

1. Full `CLAUDE.md` content present verbatim, labeled as a `<system-reminder>` — **not** fetched via a Read call.
2. Full `MEMORY.md` content present verbatim (same mechanism).
3. A full Skills catalog (dozens of named skills + descriptions) present unprompted.
4. A deferred-tools name list present unprompted.
5. It did **not** see `docs/regressions/REGRESSION-LEDGER.md` or `docs/lessons/` content unprompted — those are hook-injected only on the matching Edit/Write, confirming Finding 2's mechanism applies identically inside subagents (every lane's own Edit/Write calls re-trigger `pre-edit-lessons`/`serious-gate`/`red-line-doubt-gate` independently — the 464-injection count in Finding 3 is a lower bound for multi-lane sessions, since each lane's edits log separately).

**Conclusion: every subagent spawn pays a ~42,000-token floor before doing any work** — CLAUDE.md (4,754) + MEMORY.md (3,799) sum to only ~8,550 of that; the remainder (~33,000 tok) is tool-definition schemas (this project has 3 MCP servers configured in `.mcp.json` — repowise, playwright-test, browser-use — plus the environment additionally exposes a long tail of user-global connectors: Gmail/Calendar/Drive/Notion/Sentry/Figma/Consensus/Harmonic/Scholar-Gateway/Synapse/Common-Room — all visible in the deferred-tools list) + the base Claude Code system prompt + the specific agent's own `.md` definition (849 tok avg, up to 1,872 for `counsel.md`).

At 17 lanes/session (the 2026-07-03 orchestration's own scale), **the fixed floor alone is ~714,000 of the reported 2.57M subagent tokens (~28%)** — before a single lane does real work. This is the single largest, most mechanical, lowest-risk lever in this audit: it is pure plumbing repeated per-lane, not signal.

---

## Finding 5 — Duplication analysis on the memory corpus

Corpus: 122 files (121 topic files + MEMORY.md), 541,655 B total. `memory-corpus-meta-patterns-2026-07-02.md` (already in-corpus, cited directly rather than re-derived) independently found the same hub structure this audit's byte-level clustering confirms:

| Cluster (by filename grouping) | Files | Total bytes | Note |
|---|---|---|---|
| Redteam tool dossiers | `redteam-pilot-tools.md` + 7× `redteam-tool-*.md` | 8 files, 15,014 B | MEMORY.md already collapses this to **1** index line ("Red-team toolset analysis + adopted tools") — good discipline already in place; the reclaim opportunity is only realized *if/when* an agent recalls all 8 raw files instead of the 1-line pointer |
| Polish/QA loop reports | `qa-loops`, `qa-sweep`, `ui-polish-loop`, `polish-debt-round`, `seam-polish-loop`, `fe-polish-batch`, `sellable-polish-phase1`, `systemic-coherence-pass` | 8 files, 33,189 B | Largest duplication-candidate cluster by bytes; each documents one polish pass with overlapping i18n/contrast/a11y content |
| Tooling evaluation | `tooling-pilot-codeburn-aislop`, `tooling-pilots-round2`, `tooling-integration-eval`, `tooling-decision-patterns`, `tooling-scaffold-and-design-shortlist`, `skill-adoption` | 6 files, 25,149 B | `tooling-decision-patterns-2026-07-02.md` is itself a 12-rule grammar meant to supersede ad-hoc reasoning in the other 5 — strong merge candidate |
| Staging audit reports | `staging-full-audit`, `staging-audit-fixes-2026-06-23b`, `staging-48h-verify`, `staging-lifecycle-validation` | 4 files, 13,003 B | All are closed, one-time "audit ran, N fixed" reports; low ongoing recall value |
| Staging access/topology | `staging-fly-access`, `staging-db-access-2026-06-30`, `staging-deploy-flags-2026-06-30`, `deploy-topology` | 4 files, 8,034 B | Per the meta-patterns doc, `deploy-topology` is the #1 wikilink hub (25 inbound) — this is reference material, not a duplication problem; keep as-is |
| Small "rule" stubs | `never-bypass-human-gates`, `cognitive-bias-rule`, `ship-discipline-rule`, `rule-loop-report-always`, `max-lanes-parallelism-rule`, `skill-self-evolution-rule`, `model-routing-policy` | 7 files, 13,446 B | Already ~1,900 B avg (small); low priority — these are exactly the standing-rule pointers the corpus is supposed to have |
| **Meta-loop/harness-governance lineage** | `meta-loop-v2-upgrade`, `meta-loop-audit-2026-07-02`, `memory-corpus-meta-patterns`, `l5-meta-controller`, `audit-remediation-orchestration` (+ rule stubs above) | 5 core files, ~52,000 B (excl. rule stubs, excl. double-counted `tooling-decision-patterns`) | `audit-remediation-orchestration-2026-07-03.md` (20,780 B, the single largest non-rebuild file) restates/supersedes findings already in `meta-loop-audit-2026-07-02.md` and `memory-corpus-meta-patterns-2026-07-02.md` — candidate to fold the superseded portions into the newest file and mark the older two ATTIC in MEMORY.md once confirmed fully subsumed |

**Total bytes inside duplication-candidate clusters (no double-count): ≈146,000 B ≈ 36,500 tok — ~27% of the 541,655 B corpus.** This is *not* a guaranteed per-session cost (only MEMORY.md's one-line pointers are); it is the cost paid **whenever these topics are recalled together** (e.g., an agent greps memory for "staging" and pulls all 4 audit files instead of 1 merged one, or "tooling" and pulls 6 files instead of 1).

**MEMORY.md itself is already lean** (107 lines / 15,194 B for 121 files ≈ 1 line/file, 142 B/line avg) — it is not carrying duplicate pointers. The lever here is **merging the underlying files** (so a future recall pulls 1 consolidated doc, not N), not editing MEMORY.md's index, which is already close to minimal. No files were deleted — this is a candidate list only, per the task's read-only constraint.

---

## Finding 6 — Doc-store injections

- `docs/lessons/`: 10 lesson files + `INDEX.md`, **26,083 B** total lesson content (2,608 B avg/file), **1,631 B** INDEX. The hook never reads a full lesson file into context — only the `ACTION:` + `LINK:` fields (651 B avg, see Finding 2) — this is already the cheap path. Of the 10 lessons, 2 have broad triggers (`docs/**`, `packages/db/migrations/**`) that dominate the 464 measured fires.
- `docs/regressions/REGRESSION-LEDGER.md`: **113,407 B / 178 lines, 78 data rows** (avg 1,351 B/row, max 3,567 B). **It IS loaded wholesale** — `librarian.md` and `pattern-critic.md` both explicitly instruct reading the full ledger file. This is an append-only, monotonically-growing file by design (the ratchet rule forbids shrinking it) — its full-read cost (currently ~28,352 tok) will keep climbing every time a fix adds a row, with no compaction lever available except adding a compact index (Finding table, row 7).

---

## Top-5 biggest wins

1. **Per-subagent fixed floor ≈ 42,000 tokens/lane (MEASURED on a zero-op dispatch)** — mostly tool-schema overhead from "All tools" grants + global MCP connectors, not CLAUDE.md/MEMORY.md (which are only ~8,550 of it). At 17 lanes/session this floor alone is **~714K of the reported 2.57M tokens (~28%)**. Fix: default to narrow-tool custom agents over `general-purpose`, and trim unused global MCP connectors from this project's session.
2. **`route-request.sh` nudge noise: 383 fires/2.5 days (274 in one prior session alone)** — the prior in-repo telemetry report already prescribed the fix (P3: "context-aware nudges … cuts ~250 low-signal nudges") and it is still unshipped. Tightening the SERIOUS/REPEAT regexes reclaims an estimated 15,000–25,000 tokens/session with zero gate weakening (it's advisory-only).
3. **`pre-edit-lessons` hook: 464 injections/2.5 days**, driven by two overly-broad triggers (`docs/**`, `packages/db/migrations/**`) — narrowing them to the sub-paths the lessons actually concern reclaims an estimated 40,000–70,000 tokens over the same window, and stops firing on every doc write (including this very report).
4. **`docs/regressions/REGRESSION-LEDGER.md` full-file reads: 28,352 tokens/read, growing every fix forever** (78 rows today, append-only, un-shrinkable by the ratchet rule). Adding a ~100-B/row compact index at the top lets `librarian`/`pattern-critic` grep instead of full-Read, capping this cost instead of letting it climb linearly with every future fix.
5. **~146,000 B (~36,500 tok, ~27% of the 541,655 B memory corpus) sits in 6 duplication-candidate clusters** (8 redteam files, 8 polish/QA files, 6 tooling-eval files, 4 staging-audit files, plus the meta-loop-governance lineage) — MEMORY.md's own index is already lean (1 line/file), so the win is merging the underlying files themselves before the next multi-file recall, not touching the index.
