#!/usr/bin/env bash
# loop-detector.sh — PostToolUse/Stop hook (Doubt model: mandatory loop escalation)
# Counts CONSECUTIVE failures on the same target/error-signature. At N (default 3) it
# emits a STRONG escalation directive: stop retrying the same path → fresh context /
# specialist / stronger model. Deterministic (not at the agent's discretion).
# Zero-noise on routine success: a success on a signature resets its counter and is silent.
# Fail-open on any parse/IO error. Never blocks (exit 0) — friction via additionalContext.
set -uo pipefail

N="${DOUBT_LOOP_N:-3}"
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-$PWD}")"
STATE_DIR="$ROOT/.claude/.loop-state"
mkdir -p "$STATE_DIR" 2>/dev/null || true

INPUT="$(cat)"

# --- extract (tool_name, file_path, success-bit, error-text) via jq→python3→python→node ---
# success-bit: "1" if the tool clearly failed, "0" if it clearly succeeded, "" if unknown.
parse() {
  if command -v jq >/dev/null 2>&1; then
    printf '%s' "$INPUT" | jq -r '
      [ (.tool_name // ""),
        (.tool_input.file_path // .tool_input.path // .tool_input.command // ""),
        ( if   (.tool_response.error // .tool_response.stderr // "") != "" then "1"
          elif (.tool_response.is_error == true) then "1"
          elif (.tool_response.success == false) then "1"
          elif (.tool_response | type) == "object" then "0"
          else "" end ),
        ((.tool_response.error // .tool_response.stderr // "") | tostring | .[0:200])
      ] | @tsv' 2>/dev/null
  elif command -v python3 >/dev/null 2>&1; then
    printf '%s' "$INPUT" | python3 -c '
import sys,json
try: d=json.load(sys.stdin)
except Exception: sys.exit(0)
tn=d.get("tool_name","") or ""
ti=d.get("tool_input",{}) or {}
tgt=ti.get("file_path") or ti.get("path") or ti.get("command") or ""
tr=d.get("tool_response",{})
fail=""
err=""
if isinstance(tr,dict):
    err=str(tr.get("error") or tr.get("stderr") or "")
    if err or tr.get("is_error") is True or tr.get("success") is False: fail="1"
    else: fail="0"
print("\t".join([tn,tgt,fail,err[:200]]))
' 2>/dev/null
  elif command -v python >/dev/null 2>&1; then
    printf '%s' "$INPUT" | python -c '
import sys,json
try: d=json.load(sys.stdin)
except Exception: sys.exit(0)
tn=d.get("tool_name","") or ""
ti=d.get("tool_input",{}) or {}
tgt=ti.get("file_path") or ti.get("path") or ti.get("command") or ""
tr=d.get("tool_response",{})
fail=""; err=""
if isinstance(tr,dict):
    err=str(tr.get("error") or tr.get("stderr") or "")
    if err or tr.get("is_error") is True or tr.get("success") is False: fail="1"
    else: fail="0"
print("\t".join([tn,tgt,fail,err[:200]]))
' 2>/dev/null
  elif command -v node >/dev/null 2>&1; then
    printf '%s' "$INPUT" | node -e '
let s="";process.stdin.on("data",c=>s+=c);process.stdin.on("end",()=>{
try{const d=JSON.parse(s);const ti=d.tool_input||{};const tr=d.tool_response||{};
let err=String((tr&&(tr.error||tr.stderr))||"");
let fail="";if(typeof tr==="object"&&tr){if(err||tr.is_error===true||tr.success===false)fail="1";else fail="0";}
const tgt=ti.file_path||ti.path||ti.command||"";
process.stdout.write([d.tool_name||"",tgt,fail,err.slice(0,200)].join("\t"));}catch(e){}});
' 2>/dev/null
  fi
}

LINE="$(parse)" || exit 0
[ -z "$LINE" ] && exit 0

IFS=$'\t' read -r TOOL TARGET FAILBIT ERR <<<"$LINE"

# Unknown outcome (no tool_response shape we recognise) → stay silent, fail-open.
[ -z "$FAILBIT" ] && exit 0

# --- signature = tool + target + normalized error class (digits/hex/paths squashed) ---
ERRSIG="$(printf '%s' "$ERR" \
  | tr 'A-Z' 'a-z' \
  | sed -E 's/[0-9a-f]{7,}/H/g; s/[0-9]+/N/g; s#/[^ ]+#/P#g' \
  | tr -cd 'a-z _/.-' \
  | cut -c1-80)"
RAWSIG="${TOOL}|${TARGET}|${ERRSIG}"
# stable, filesystem-safe key
KEY="$(printf '%s' "$RAWSIG" | cksum | tr -d ' \t' )"
CF="$STATE_DIR/$KEY"

if [ "$FAILBIT" = "0" ]; then
  # success on this signature → reset its counter, emit nothing (zero-noise on routine success)
  rm -f "$CF" 2>/dev/null || true
  exit 0
fi

# failure → increment
count=0
[ -f "$CF" ] && count="$(cat "$CF" 2>/dev/null || echo 0)"
case "$count" in ''|*[!0-9]*) count=0 ;; esac
count=$((count+1))
printf '%s' "$count" > "$CF" 2>/dev/null || true

# below threshold → stay quiet (don't nag on the 1st/2nd stumble)
[ "$count" -lt "$N" ] && exit 0

# --- AT/OVER threshold: MANDATORY escalation directive ---
rm -f "$CF" 2>/dev/null || true   # reset so the directive isn't re-spammed every subsequent failure
DIR="🔴 LOOP DETECTED — ${count} consecutive failures on the same signature [${TOOL} · ${TARGET:-<no-target>}${ERRSIG:+ · $ERRSIG}].
This is a MANDATORY escalation (Doubt model, N=${N}) — do NOT attempt the same path a ${count}+1-th time the same way.
Climb the ladder (skill: doubt-escalation):
  1) self-divergence — enumerate 2–3 DIFFERENT approaches; if one dominates on evidence, take it;
  2) specialist/research subagent in a FRESH/isolated context (security-sentinel · invariant-guardian · systematic-debugging · cause-critic) — get evidence (file:line), then resume;
  3) escalate THIS sub-decision to a stronger model (/model or OpenRouter);
  4) if systemic → /council.
Change at least ONE variable (approach, context, model, or assumption). Counter has been reset."

if command -v jq >/dev/null 2>&1; then
  jq -nc --arg d "$DIR" '{hookSpecificOutput:{hookEventName:"PostToolUse",additionalContext:$d}}'
elif command -v python3 >/dev/null 2>&1; then
  python3 -c 'import json,sys;print(json.dumps({"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":sys.argv[1]}}))' "$DIR"
else
  printf '%s\n' "$DIR" >&2
fi
exit 0
