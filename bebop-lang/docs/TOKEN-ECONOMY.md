Status: 2026-09-07 CURRENT (operator priority: "економити нещадно", mechanical enforcement, exact accounting)

# Token economy -- mechanical rules and the measured ledger

Every rule is enforced by a hook or a script, not by prose. `python3 tools/token_ledger.py [session.jsonl]` prints the exact
numbers for a session (turns, cached context per turn, alert-turn cost, tool-result bulk, Agent prompt bulk).

## Mechanisms (all live)
| # | mechanism | enforced by | what it removes |
|---|---|---|---|
| M1 | no box Monitor task; boxguard alone guards the box | session practice (Monitor stopped 2026-09-07) | every alert wakes a full-context turn |
| M2 | no raw dumps into context (`cat`, `git diff/show`, `objdump`, `sed -n a,bp` must be squeezed by head/tail/grep/cut/wc) | `~/.claude/hookify.block-raw-dump.local.md` (PreToolUse Bash, block) | tool results that then ride in the context for every later turn |
| M3 | worker prompts = docs/WORKER-CARD.md reference + task; `cap:` step ceiling; no opus; < 6000 chars | `~/.claude/hooks/agent-gate.sh` (PreToolUse Agent, deny) | 1.5-2k output tokens per launch, unstable cache prefixes |
| M4 | hard per-agent step ceiling (default 150 tool calls, `TOOL_CAP` env), then STATE.md + stop | `~/.claude/hooks/tool-cap.sh` (PreToolUse all tools, deny past the cap) | runaway reflection loops in workers |
| M5 | routine tasks routed to a free model | `tools/llm_route.sh [model] < prompt` (OpenRouter free tier; key in ~/.config/openrouter/key, never in the tree) | one full-context frontier turn per routed task |
| M6 | structured hand-offs: VERDICT block + STATE.md, resumes point at the snapshot | docs/WORKER-CARD.md, agent-gate | narrated history between agents |

## Baseline ledger (session 22, 2026-09-07, measured from the transcript)
- 540 model turns; context per turn 513-542k tokens; cache_read 160.2M, cache_create 2.93M, uncached input 13.6k, output 643k.
- 15 Monitor/alert turns = 10.19M context tokens (M1 removes: exact).
- 197 tool results = 86.7k tokens; 49 of them > 2k chars = 63.6k tokens, each re-read on every later turn (M2; at the mid-session average of ~260 remaining turns that is ~16.5M cached tokens, projected).
- 8 Agent launches = 14.1k prompt tokens (avg 1.76k) + 8 resumes = 2.6k; with the card a launch is ~200 tokens (M3: ~12.5k output tokens per 8 launches, exact).
- 7 workers = 2.59M subagent tokens; the largest (csel) 831k over 367 tool calls -- a 150-call cap with a STATE.md hand-off would have cut ~490k (M4, projected).
- one routed routine task saves one frontier turn = ~0.52M cached tokens at this context size (M5, per task).

Projected per comparable session: ~27M+ of 163M context-read tokens (~17%) plus most of the output-token cost of prompts; the next session's ledger is the measurement, not this projection.
