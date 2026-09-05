# Claude tooling for this loop — research 2026-09-06 (session 13)

Status: 2026-09-06 CURRENT. Installed: ralph-loop, hookify, claude-md-management (user scope, marketplace claude-plugins-official), jq, ~/.claude/karpathy.md (rules 1+4), four hookify rules in ~/.claude/hookify.*.local.md, /lint-docs (.claude/commands), AGENTS L20. Pending by hand (auto mode blocks ~/.claude/settings.json and CLAUDE.md edits): the SessionStart/compact handoff hook and the @karpathy.md include — see the end of this file.

# Claude tooling survey for bebop-lang — 2026-09-05 (research only, nothing installed)

Context read: ~/.claude/CLAUDE.md (RTK + graphify), settings.json (RTK PreToolUse on Bash; ponytail+mempalace
enabled; headroom 0.37 installed but NOT enabled), AGENTS.md v2 (Occam ladder, laws), docs/SESSION-HANDOFF.md
(rewritten every close), docs/exp.journal (epoch H:/DID:/GOT:/VERDICT:), docs/TOKEN-ECONOMY.md (tier routing),
tools/hooks/pre-push. Box: python3.14 yes, node yes, **jq missing** (matters for ralph-loop).

## 1. What Karpathy actually publishes (no dotfiles / Claude config exist publicly)

| artefact | what it concretely is | relevance here |
|---|---|---|
| X post 2015883857489522876 (Jan 2026) | "80% agent coding"; failure list: assumes without checking, hides confusion, no clarifying Qs, no tradeoffs; several Claude windows + IDE for review | already encoded in AGENTS.md T0 + ponytail |
| gist karpathy/442a6bf555914893e9891c11519de94f "llm-wiki" (Apr 2026) | pattern, not code: raw/ (immutable) -> wiki/ (LLM-written md) + CLAUDE.md schema; ops ingest/query/**lint** (contradictions, stale claims, orphans); index.md + append-only log.md | docs/ + exp.journal + graphify already are this; the missing op is **lint** |
| github.com/karpathy/autoresearch (Mar 2026) program.md | agent loop: edit train.py -> commit -> run 5 min -> read metric -> results.tsv (hash, metric, mem, keep/discard/crash, note) -> keep or `git reset`; may not touch harness; "do NOT pause to ask the human" | exp.journal == results.tsv; chain.sh == harness. Missing: the explicit keep/revert rule + a stop-hook loop |
| multica-ai/andrej-karpathy-skills (was forrestchang/…) | third-party 65-line CLAUDE.md, NOT by Karpathy: Think-before-coding / Simplicity-first / Surgical-changes / Goal-driven-execution ("[step] -> verify: [check]"). No hooks, no network, MIT | rules 2-3 duplicate ponytail; rules 1 and 4 add value |

## 2. Candidates

Fit 0-3 = value for a self-hosted compiler gated by shell battery on a phone. Risk = injection / network / footprint.

| name | URL | what | install | fit | risk |
|---|---|---|---|---|---|
| karpathy CLAUDE.md | github.com/multica-ai/andrej-karpathy-skills | 4 behavioural rules, 65 lines | `curl` raw CLAUDE.md -> ~/.claude/karpathy.md, `@karpathy.md` | 2 | none (plain md) |
| autoresearch pattern | github.com/karpathy/autoresearch/blob/master/program.md | keep/revert experiment loop | copy 6 lines into AGENTS.md | 3 | none |
| ralph-loop (official) | claude-plugins-official/plugins/ralph-loop | Stop hook re-feeds prompt until `<promise>DONE</promise>` or max_iterations; state in `.claude/ralph-loop.local.md` | `/plugin install ralph-loop@claude-plugins-official` (+ `pkg install jq`) | 3 | low; runaway tokens capped by --max-iterations; hook uses jq (absent) |
| hookify (official) | claude-plugins-official/plugins/hookify | markdown+regex rules -> deterministic PreToolUse/PostToolUse/Stop/UserPromptSubmit hooks, python3, local only | `/plugin install hookify@claude-plugins-official`; rules in `.claude/hookify.*.local.md` | 3 | low; 4 python3 spawns per tool call (~0.1 s each on A78) |
| SessionStart context hook (DIY) | code.claude.com/docs/en/hooks | on startup/compact inject head of SESSION-HANDOFF + last journal lines (what superpowers' session-start does, minus the sermon) | 8 lines in settings.json | 3 | none |
| claude-md-management (official) | claude-plugins-official/plugins/claude-md-management | `/revise-claude-md` captures session learnings; claude-md-improver audits CLAUDE.md vs codebase | `/plugin install claude-md-management@claude-plugins-official` | 2 | none (skill+command, no hooks) |
| claude-code-setup (official) | claude-plugins-official/plugins/claude-code-setup | read-only recommender of hooks/skills for THIS repo | `/plugin install claude-code-setup@claude-plugins-official`, run once, uninstall | 2 | none |
| llm-wiki lint | Karpathy gist | periodic contradiction/stale/orphan pass over docs/ + exp.journal | a /lint-docs command file, no install | 2 | none |
| REMvisual/claude-handoff | github.com/REMvisual/claude-handoff | /handoff writes plans/handoffs/HANDOFF_*.md; optional PreCompact hook | git clone -> ~/.claude/skills/handoff | 1 | none; duplicates SESSION-HANDOFF.md |
| mattpocock/skills | github.com/mattpocock/skills | 25 skills; useful: diagnosing-bugs, grill-me, handoff, tdd | `npx skills add mattpocock/skills` (network) or clone 2 folders | 1 | install-time network; tracker/CONTEXT.md oriented, big |
| superpowers | github.com/obra/superpowers | 14 skills (brainstorm, plans, TDD, systematic-debugging, subagents); SessionStart hook injects ~900 words "EXTREMELY_IMPORTANT… YOU DO NOT HAVE A CHOICE" every startup/compact | `/plugin install superpowers@claude-plugins-official` | 1 | **injection**: competes with AGENTS.md laws + ponytail; ~1.2k tokens per session/compact; optional telemetry (SUPERPOWERS_DISABLE_TELEMETRY) |
| caveman | github.com/juliusbrussee/caveman | terse-output skill (-65% output tokens) + BSL proxy | `npx skills add JuliusBrussee/caveman` | 1 | CLI telemetry ON by default (DO_NOT_TRACK=1); overlaps RTK + ponytail |
| headroom (installed, disabled) | github.com/chopratejas/headroom | python proxy compressing tool output (20-60%); HF model | already present | 1 | telemetry ON by default (HEADROOM_BEACON=off); daemon; overlaps RTK — enable with beacon off or `/plugin uninstall` |
| tdd-guard | github.com/nizos/tdd-guard | PreToolUse blocks edits without failing test; node22; reporters for vitest/pytest/go/rust… | marketplace | 0 | model call per edit; no reporter for a shell battery |
| claude-mem | github.com/thedotmack/claude-mem | 5 hooks + bun worker + sqlite + chroma; sends transcripts to a provider for compression | `npx claude-mem install` | 0 | network + daemon; mempalace already fills this slot |
| beads (bd) | github.com/gastownhall/beads | agent issue tracker, now Dolt-backed | npm/go | 0 | heavy; TASKS.md already |
| GSD | github.com/glittercowboy/get-shit-done (archived -> open-gsd/gsd-core) | spec-driven phases, fresh-context subagents | npx installer | 1 | app-dev flavoured; unverified after move |
| code-review / commit-commands / pr-review-toolkit (official) | claude-plugins-official | GitHub-PR review agents; /commit | marketplace | 1 | needs gh + PRs; commit msgs must carry gate evidence anyway |
| context7, LSPs, security-guidance, anthropics/skills (docx/pdf/pptx) | official | third-party docs / web security / office files | — | 0 | irrelevant to own language |
| Anthropic "Effective context engineering for AI agents" | anthropic.com/engineering/effective-context-engineering-for-ai-agents | essay: compaction, JIT retrieval, note-taking | read once | 2 | none |

## 3. Ranked top 5

**1. Stop-hook experiment loop = ralph-loop + autoresearch keep/revert rule (fit 3).**
```
pkg install jq                                   # stop-hook.sh needs jq
/plugin install ralph-loop@claude-plugins-official
/ralph-loop "run tools/fuzzd.sh batch; triage each DIVERGE per AGENTS.md; journal every one" --max-iterations 20 --completion-promise "BATTERY GREEN"
```
AGENTS.md addition: `L20 experiment loop: one hypothesis per iteration; keep only if chain.sh+battery green and metric not worse, else git checkout -- . ; every iteration = one exp.journal line; do not ask to continue, stop only on the promise or max_iterations.`
Daily loop change: overnight fuzz triage / T-step grinding runs unattended in ONE session (respects the 32-proc cap) instead of you re-prompting; journal lines are the results.tsv.

**2. hookify: AGENTS.md laws as deterministic hooks (fit 3).**
```
/plugin install hookify@claude-plugins-official
/hookify Block Bash commands that write bebop.bin or $BEBOP_BIN outside tools/chain.sh
/hookify Block git commit when the message lacks "gen3" or "battery"     # gate evidence rule
/hookify Warn when rm -rf touches $BEBOP_TMP or docs/
```
CLAUDE.md addition: `Hard laws (bin writes, gate evidence in commits, journal before commit) are enforced by hookify rules in .claude/hookify.*.local.md; edit rules there, not in prose.`
Daily loop change: the laws that cost "dozens of avoidable cycles" become blocks instead of reminders; zero tokens (python regex).

**3. DIY SessionStart/compact handoff injection (fit 3, no install).** settings.json:
```json
"SessionStart":[{"matcher":"startup|compact","hooks":[{"type":"command","command":"cd /root/dowiz/bebop-lang && { head -30 docs/SESSION-HANDOFF.md; git log --oneline -3; tail -3 docs/exp.journal | cut -c1-200; } | jq -Rs '{additionalContext:.}'"}]}
```
(without jq: `python3 -c 'import json,sys;print(json.dumps({"additionalContext":sys.stdin.read()}))'`).
CLAUDE.md addition: `Session state is injected at start and after every compact from docs/SESSION-HANDOFF.md; do not re-read HISTORY/ROADMAP unless the handoff points there.`
Daily loop change: post-compaction amnesia (the "50 first dates" problem) disappears; SESSION-HANDOFF.md stays the single source, nothing new to maintain.

**4. Karpathy rules 1+4 only (fit 2, no install).** `curl -o ~/.claude/karpathy.md https://raw.githubusercontent.com/multica-ai/andrej-karpathy-skills/main/CLAUDE.md`, delete sections 2-3 (ponytail covers them), add `@karpathy.md` to ~/.claude/CLAUDE.md.
CLAUDE.md addition: `@karpathy.md — state assumptions and tradeoffs before editing; every multi-step task lists "[step] -> verify: [check]" where check is a chain.sh/battery.sh/journal line.`
Daily loop change: plans come pre-wired to the gate; fewer silent assumption bugs (the class AGENTS.md T0 documents).

**5. claude-md-management + one llm-wiki "lint" pass (fit 2).**
```
/plugin install claude-md-management@claude-plugins-official     # /revise-claude-md at session close
```
plus a `/lint-docs` command (`.claude/commands/lint-docs.md`, ~10 lines): "compare ROADMAP/TASKS/SESSION-HANDOFF/exp.journal; list contradictions, stale claims, orphan tasks; propose edits, change nothing".
CLAUDE.md addition: `Session close: /revise-claude-md (learnings -> AGENTS.md/CLAUDE.md), then /lint-docs; SESSION-HANDOFF.md rewrite stays last.`
Daily loop change: AGENTS.md grows from scars automatically instead of by audit sessions; roadmap drift caught weekly (the 09-04 audit/critique docs become a command).

Skip: superpowers (injects a mandatory-skill sermon over AGENTS.md; TDD shape is unit-test frameworks, not a shell battery), claude-mem/beads/tdd-guard (daemons, DBs, per-edit model calls — wrong for a phone), caveman/headroom (RTK+ponytail already own token economy; if headroom stays, set HEADROOM_BEACON=off or uninstall the dormant plugin).

## VERDICT
Top 5: (1) ralph-loop + autoresearch keep/revert rule for unattended fuzz/T-step loops; (2) hookify to turn L-laws into blocking hooks; (3) 8-line SessionStart/compact hook injecting SESSION-HANDOFF; (4) Karpathy rules 1+4 as @karpathy.md; (5) claude-md-management + /lint-docs. Items 3-4 need no install; 1 needs `pkg install jq`; all five are local, no network, no daemons, no injected "must" prose. Everything with a background process or per-edit model call is a no for this box.
Not verified: Karpathy has no public Claude config/dotfiles (only the X post, the llm-wiki gist, autoresearch; the "Karpathy CLAUDE.md" is multica-ai's distillation); GSD post-archive state; tdd-guard's exact per-edit cost; node version here; hook ordering of RTK vs hookify on PreToolUse (both should run, untested); ralph-loop's stop hook was read locally but not executed.
