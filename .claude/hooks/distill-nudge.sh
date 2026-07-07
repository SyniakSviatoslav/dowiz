#!/usr/bin/env bash
# distill-nudge.sh — PostToolUse gate on Bash: nudge (never block) when a command dumps a large
# output that was NOT distilled. STRUCTURE-UPGRADE.md Part B, step B3.
#
# WHY WARN not DENY (degradation by design): a PreToolUse deny would need to PREDICT noisiness from
# the command text — that's the guard-bash over-block failure mode (an over-broad gate becomes NO
# gate). PostToolUse measures the ACTUAL output size → zero false positives on size, but it can only
# warn after the spend. RATCHET (B3 clause): after ≥1 week of _hev data, if one command-shape
# repeats un-distilled ≥3×, promote that literal shape into a PreToolUse deny-list (narrow rule).
#
# Non-blocking: emits the nudge via hookSpecificOutput.additionalContext + one _hev WARN line.
# Fail-open on any parse error.
set -uo pipefail

INPUT="$(cat)"
[ -z "$INPUT" ] && exit 0

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
THRESHOLD="${DISTILL_NUDGE_BYTES:-8000}"

HEV_LOG="$ROOT/.claude/logs/harness-events.jsonl"
_hev() {
  mkdir -p "$(dirname "$HEV_LOG")" 2>/dev/null || true
  printf '{"ts":"%s","hook":"%s","event":"%s","target":"%s","detail":"%s"}\n' \
    "$(date -Iseconds)" "$1" "$2" \
    "$(printf '%s' "${3:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    "$(printf '%s' "${4:-}" | tr '"\\' '..' | tr '\n' ' ' | cut -c1-200)" \
    >>"$HEV_LOG" 2>/dev/null || true
}

# Compute (tool_name, output_size, already_distilled 0/1, cmd_snippet) — the size + distill-marker
# are computed in-parser so a MULTI-LINE command can never break field splitting (4 clean lines).
# already_distilled: command already piped through a distiller (repowise distill / --reporter=list /
# a tail|head cap) — respect the effort, stay silent.
DISTILL_RE='repowise[[:space:]]+distill|--reporter[= ]list|\|[[:space:]]*(tail|head)\b'
parse() {
  if command -v python3 >/dev/null 2>&1; then
    printf '%s' "$INPUT" | THRESHOLD="$THRESHOLD" python3 -c '
import sys, json, re, os
try: d = json.load(sys.stdin)
except Exception: sys.exit(0)
tn = d.get("tool_name","") or ""
cmd = (d.get("tool_input",{}) or {}).get("command","") or ""
tr = d.get("tool_response")
if isinstance(tr, dict): size = len(str(tr.get("stdout") or "")) + len(str(tr.get("stderr") or ""))
elif isinstance(tr, str): size = len(tr)
else: size = 0
dist = 1 if re.search(r"repowise\s+distill|--reporter[= ]list|\|\s*(tail|head)\b", cmd) else 0
snip = re.sub(r"\s+"," ",cmd).strip()[:80]
print("\n".join([tn, str(size), str(dist), snip]))
' 2>/dev/null
  elif command -v node >/dev/null 2>&1; then
    printf '%s' "$INPUT" | node -e '
let s="";process.stdin.on("data",c=>s+=c);process.stdin.on("end",()=>{try{
const d=JSON.parse(s);const tn=d.tool_name||"";const cmd=(d.tool_input||{}).command||"";
const tr=d.tool_response;let size=0;
if(tr&&typeof tr==="object")size=String(tr.stdout||"").length+String(tr.stderr||"").length;
else if(typeof tr==="string")size=tr.length;
const dist=/repowise\s+distill|--reporter[= ]list|\|\s*(tail|head)\b/.test(cmd)?1:0;
const snip=cmd.replace(/\s+/g," ").trim().slice(0,80);
process.stdout.write([tn,String(size),String(dist),snip].join("\n"));}catch(e){}});
' 2>/dev/null
  fi
}

TOOL=""; SIZE=""; DIST=""; SNIP=""
{ IFS= read -r TOOL; IFS= read -r SIZE; IFS= read -r DIST; IFS= read -r SNIP; } < <(parse) || true

case "$SIZE" in ''|*[!0-9]*) exit 0 ;; esac   # unparseable → fail-open silent
[ "$TOOL" = "Bash" ] || exit 0                # exact — PostToolUse Bash only
[ "$SIZE" -le "$THRESHOLD" ] && exit 0        # small output → silent
[ "$DIST" = "1" ] && exit 0                   # already distilled → respect it, silent

_hev distill-nudge warn "$SNIP" "output=${SIZE}B > ${THRESHOLD}B, undistilled"
DIR="📉 DISTILL NUDGE: that Bash command returned ~${SIZE} chars (> ${THRESHOLD}) undistilled — every big blob re-enters context on the next call (quadratic cache cost). Next time wrap it: 'repowise distill \"<cmd>\"', or add '--reporter=list' / a '| tail -N' cap. [advisory — non-blocking]"
if command -v jq >/dev/null 2>&1; then
  jq -nc --arg d "$DIR" '{hookSpecificOutput:{hookEventName:"PostToolUse",additionalContext:$d}}'
elif command -v python3 >/dev/null 2>&1; then
  python3 -c 'import json,sys;print(json.dumps({"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":sys.argv[1]}}))' "$DIR"
else
  printf '%s\n' "$DIR" >&2
fi
exit 0
