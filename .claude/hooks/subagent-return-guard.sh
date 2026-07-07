#!/usr/bin/env bash
# subagent-return-guard.sh — SubagentStop (+ belt PostToolUse Agent|Task) gate.
#
# ROOT CAUSE (fable-audit-findings-2026-07-07, §ROOT-CAUSE): ~2.5% of subagent returns are
# 0-tool-use DEGENERATE — the subagent's first decode *continues the harness-injected trailing
# metadata* (deferred-tools delta / skill listing / system-reminder) instead of executing the task,
# or it surfaces an API-error string ("API Error: 529 Overloaded", "You've hit your session limit",
# "temporarily limiting requests") AS its result. Signatures observed in real transcripts:
#   ^_[a-z_]+:        metadata continuation   (agent-ae2504…: "_context_relevance:\nSkills marked…")
#   ^_id: … The system is Claude Code   fabricated system-prompt echo (agent-a79d88…)
#   <br> / <system-reminder> / API Error: / You've hit your session limit / …temporarily limiting
# The parent already has ground truth (task-notification carries <tool_uses>0</tool_uses>), but
# nothing ACTS on it. This gate does.
#
# MECHANISM: on SubagentStop, locate the stopped agent-*.jsonl, count tool_use blocks + assistant
# turns + classify the final assistant text. RED (block + continue-reason) when
#   tool_uses == 0  &&  assistant_turns == 1  &&  final_text matches an echo/API-error signature.
# WARN (log only, non-blocking) on any other 0-tool-use return. Silent otherwise.
# stop_hook_active guard: if we already blocked once this stop-cycle, downgrade RED→WARN so a truly
# stuck subagent cannot loop forever. Fail OPEN on any parse/locate failure (never a silent hang).
#
# The DETECTION+DECISION logic is proven hermetically by scripts/guardrail-subagent-return-guard.mjs
# using SUBAGENT_TRANSCRIPT to point at committed fixtures (the 2 real degenerate transcripts + a
# good control) — so correctness does not depend on the location heuristic. Registration is pinned
# in scripts/guardrail-hook-matchers.mjs MUST_COVER (#47: the easy "fix" for a gate is to unregister
# it — that now fails loudly in pre-commit).
set -uo pipefail

INPUT=$(cat)
[ -z "$INPUT" ] && exit 0

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"

# ── telemetry (ASCII-only values → no cut -c UTF-8 corruption, finding #10) ──
HEV_LOG="$ROOT/.claude/logs/harness-events.jsonl"
_hev() {
  mkdir -p "$(dirname "$HEV_LOG")" 2>/dev/null || true
  printf '{"ts":"%s","hook":"subagent-return-guard","event":"%s","target":"%s","detail":"%s"}\n' \
    "$(date -Iseconds)" "$1" "${2:-}" "${3:-}" >>"$HEV_LOG" 2>/dev/null || true
}

# ── parse the hook payload (jq → python3 → node; jq absent on this host, python3 present) ──
# Emits four lines: hook_event_name, tool_name, transcript_path, stop_hook_active.
_parse() {
  if command -v jq &>/dev/null; then
    printf '%s' "$INPUT" | jq -r '[(.hook_event_name // ""),(.tool_name // ""),(.transcript_path // ""),(.stop_hook_active // false)] | .[]' 2>/dev/null
  elif command -v python3 &>/dev/null; then
    printf '%s' "$INPUT" | python3 -c '
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print("\n".join([str(d.get("hook_event_name") or ""), str(d.get("tool_name") or ""),
                     str(d.get("transcript_path") or ""), str(bool(d.get("stop_hook_active")))]))
except Exception:
    pass
' 2>/dev/null || true
  elif command -v node &>/dev/null; then
    printf '%s' "$INPUT" | node -e '
let d="";process.stdin.on("data",c=>d+=c);process.stdin.on("end",()=>{try{const o=JSON.parse(d);
process.stdout.write([o.hook_event_name||"",o.tool_name||"",o.transcript_path||"",String(!!o.stop_hook_active)].join("\n"));}catch(e){}});' 2>/dev/null || true
  fi
}

EVENT=""; TOOL_NAME=""; TRANSCRIPT=""; STOP_ACTIVE="False"
{ IFS= read -r EVENT; IFS= read -r TOOL_NAME; IFS= read -r TRANSCRIPT; IFS= read -r STOP_ACTIVE; } < <(_parse) || true

# ── locate the stopped subagent transcript ──
# Test override wins. Otherwise: if transcript_path already IS an agent-*.jsonl use it; else derive
# the session's subagents/ dir from transcript_path (parent = <project>/<session>.jsonl; subagents =
# <project>/<session>/subagents/agent-*.jsonl) and take the most-recently-modified one.
locate_transcript() {
  if [ -n "${SUBAGENT_TRANSCRIPT:-}" ]; then printf '%s' "$SUBAGENT_TRANSCRIPT"; return; fi
  local tp="$TRANSCRIPT" base sess sub
  base="$(basename "$tp" 2>/dev/null)"
  case "$base" in
    agent-*.jsonl) [ -f "$tp" ] && { printf '%s' "$tp"; return; } ;;
  esac
  sess="${tp%.jsonl}"                      # <project>/<session>
  for sub in "$sess/subagents" "$(dirname "$tp")/subagents"; do
    if [ -d "$sub" ]; then
      ls -t "$sub"/agent-*.jsonl 2>/dev/null | head -1
      return
    fi
  done
}
TRANSCRIPT_FILE="$(locate_transcript)"
if [ -z "$TRANSCRIPT_FILE" ] || [ ! -f "$TRANSCRIPT_FILE" ]; then
  _hev degraded "" "no-subagent-transcript-located; failing open"
  exit 0
fi

# ── analyze: tool_uses, assistant_turns, sig(1 if final assistant text matches a degenerate signature) ──
analyze() {
  local f="$1"
  if command -v python3 &>/dev/null; then
    python3 - "$f" <<'PY' 2>/dev/null || true
import sys, json, re
SIG = re.compile(r"^(_[a-z_]+|<br>|<system-reminder>|API Error:|You[’']ve hit your session limit)"
                 r"|temporarily limiting|The system is Claude Code")
tool_uses = 0; asst = 0; final = ""
try:
    for ln in open(sys.argv[1], encoding="utf-8", errors="replace"):
        ln = ln.strip()
        if not ln: continue
        try: o = json.loads(ln)
        except Exception: continue
        if o.get("type") == "assistant":
            asst += 1
            for b in ((o.get("message") or {}).get("content") or []):
                if isinstance(b, dict):
                    if b.get("type") == "tool_use": tool_uses += 1
                    if b.get("type") == "text": final = b.get("text", "") or final
except Exception:
    print("ERR"); sys.exit(0)
sig = 1 if SIG.search(final.lstrip()) else 0
print(f"{tool_uses}\n{asst}\n{sig}")
PY
  else
    printf 'ERR'
  fi
}

OUT="$(analyze "$TRANSCRIPT_FILE")"
TOOL_USES=""; ASST=""; SIG=""
{ IFS= read -r TOOL_USES; IFS= read -r ASST; IFS= read -r SIG; } <<< "$OUT" || true
case "$TOOL_USES" in ''|*[!0-9]*) _hev degraded "$(basename "$TRANSCRIPT_FILE")" "unparseable-transcript; failing open"; exit 0 ;; esac
: "${ASST:=0}"; : "${SIG:=0}"

BN="$(basename "$TRANSCRIPT_FILE")"

# ── decision ──
if [ "$TOOL_USES" -eq 0 ] && [ "$ASST" -le 1 ] && [ "$SIG" -eq 1 ]; then
  # RED — degenerate echo / surfaced API error.
  if [ "$STOP_ACTIVE" = "True" ] || [ "$STOP_ACTIVE" = "true" ]; then
    _hev warn "$BN" "degenerate-return-but-stop-active; not re-blocking (loop guard)"
    exit 0
  fi
  REASON="subagent-return-guard: this subagent returned with ZERO tool_uses in a single turn and its output looks like injected-context echo / a surfaced API error, not real work (fable-audit ROOT-CAUSE). Do NOT stop — actually execute the task: use your tools, then return the distilled result. If the task genuinely needs no tools, state that explicitly in prose."
  if [ "$EVENT" = "PostToolUse" ] || [ -n "$TOOL_NAME" ]; then
    # belt: on the parent side we cannot re-drive the child; surface a non-blocking nudge instead.
    _hev warn "$BN" "degenerate-subagent-return-detected (post-tool belt)"
    printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"%s Consider re-dispatching this subagent."}}\n' "$REASON"
    exit 0
  fi
  _hev block "$BN" "degenerate-0-tool-use-return (signature match); blocking stop to force real execution"
  printf '{"decision":"block","reason":"%s"}\n' "$REASON"
  exit 0
fi

if [ "$TOOL_USES" -eq 0 ]; then
  _hev warn "$BN" "0-tool-use-return without degenerate signature (possibly legit no-op)"
fi
exit 0
