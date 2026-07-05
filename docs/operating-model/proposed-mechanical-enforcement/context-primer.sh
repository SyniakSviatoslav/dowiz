#!/usr/bin/env bash
# context-primer.sh — SessionStart hook. MECHANICALLY surfaces the durable context every session must
# read (memory index + codebase graph + the query-don't-embed rule), so project/memory/graph reading
# is never forgotten. Read-only, fail-open. Emits additionalContext (injected into the session).
set -uo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
# Auto-memory lives OUTSIDE the repo at ~/.claude/projects/<sanitized-cwd>/memory/ (default), not in
# the repo's .claude/. Derive the sanitized project dir (/ -> -) and try the standard candidates.
SAN="$(printf '%s' "$ROOT" | sed 's#[/]#-#g')"
MEM=""
for cand in "$HOME/.claude/projects/$SAN/memory/MEMORY.md" \
            "${CLAUDE_AUTO_MEMORY_DIR:-}/MEMORY.md" \
            "$ROOT/.claude/projects/$SAN/memory/MEMORY.md"; do
  [ -n "$cand" ] && [ -f "$cand" ] && MEM="$cand" && break
done

lines=""
[ -n "$MEM" ] && lines="$(grep -E '^- \[' "$MEM" 2>/dev/null | head -40 | sed 's/^/  /')"

CTX="MECHANICAL CONTEXT PRIMER (SessionStart — do not skip):
1. MAPPING: for ANY structure question, query the graph FIRST (codebase-memory MCP project 'root-dowiz'
   query_graph/search_graph, or repowise get_context/get_overview). NEVER embed or hand-copy standing
   architecture maps into prompts — query per step.
2. MEMORY: the durable memory index is below; recall the relevant file before acting, verify any named
   file/flag still exists (memories reflect write-time truth).
3. ROUTING + REDUCTION: MODEL ROUTING v3 (haiku doer / opus reasoning / never Fable, explicit model on
   every Agent call); TOKEN ROUTER (deterministic-first, distill noisy output, VSA codec/viz for state);
   token circuits (unit 80K / session 300K). Save remotely (push) before any session end.
${lines:+
Memory index:
$lines}"

if command -v jq >/dev/null 2>&1; then
  jq -cn --arg c "$CTX" '{hookSpecificOutput:{hookEventName:"SessionStart", additionalContext:$c}}'
else
  python3 -c 'import sys,json;print(json.dumps({"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":sys.argv[1]}}))' "$CTX"
fi
exit 0
