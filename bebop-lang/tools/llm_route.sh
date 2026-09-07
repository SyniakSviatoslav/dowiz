#!/usr/bin/env bash
# llm_route.sh (operator token-economy priority 2026-09-07): route a ROUTINE task to a FREE model with fallback.
# Usage: tools/llm_route.sh [provider[:model]] < prompt.txt      e.g. tools/llm_route.sh kilo < p.txt ; LLM_MAX=800
# Order without an argument: groq (keyed, 1000 RPD, fast) -> openrouter -> kilo -> ovh -> llm7 (keyless, often overloaded) -> mistral -> gemini -> nim
# Keys live in ~/.config/llm/<provider>.key (chmod 600; openrouter's is ~/.config/openrouter/key). Never echo a key.
# Source of the provider table: github.com/mnfst/awesome-free-llm-apis (fetched 2026-09-07).
set -u
P=$(python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))')
MAX=${LLM_MAX:-1200}
keyf() { case "$1" in openrouter) echo "$HOME/.config/openrouter/key";; *) echo "$HOME/.config/llm/$1.key";; esac; }
spec() {  # provider -> "base|model|needs_key"
  case "$1" in
    kilo)       echo "https://api.kilo.ai/api/gateway|nvidia/nemotron-3-ultra-550b-a55b:free|0";;
    ovh)        echo "https://oai.endpoints.kepler.ai.cloud.ovh.net/v1|gpt-oss-120b|0";;
    llm7)       echo "https://api.llm7.io/v1|gpt-oss:20b|0";;
    openrouter) echo "https://openrouter.ai/api/v1|nvidia/nemotron-3.5-lightning:free|1";;
    groq)       echo "https://api.groq.com/openai/v1|openai/gpt-oss-120b|1";;
    mistral)    echo "https://api.mistral.ai/v1|mistral-small-latest|1";;
    gemini)     echo "https://generativelanguage.googleapis.com/v1beta/openai|gemini-3.6-flash|1";;
    nim)        echo "https://integrate.api.nvidia.com/v1|nvidia/nemotron-3-super-120b-a12b|1";;
    *) return 1;;
  esac
}
one() {  # one provider [model] -> prints content, rc 0 on success
  local pv=$1 m=${2:-} s base model needs k auth=()
  s=$(spec "$pv") || { echo "llm_route: unknown provider $pv" >&2; return 1; }
  base=${s%%|*}; s=${s#*|}; model=${m:-${s%%|*}}; needs=${s##*|}
  if [ "$needs" = 1 ]; then k=$(keyf "$pv"); [ -s "$k" ] || return 3; auth=(-H "Authorization: Bearer $(cat "$k")"); fi
  curl -s --max-time 120 "$base/chat/completions" -H "Content-Type: application/json" "${auth[@]}" \
    -d "{\"model\":\"$model\",\"max_tokens\":$MAX,\"messages\":[{\"role\":\"user\",\"content\":$P}]}" \
    | python3 -c "import sys,json
try: j=json.load(sys.stdin)
except Exception: sys.exit(2)
if not isinstance(j, dict): print('llm_route['+'$pv'+'] error: '+json.dumps(j)[:200], file=sys.stderr); sys.exit(2)
c=j.get('choices')
if not c or not c[0].get('message',{}).get('content'): print('llm_route['+'$pv'+'] error: '+json.dumps(j.get('error') or j)[:200], file=sys.stderr); sys.exit(2)
print(c[0]['message']['content'])"
}
if [ -n "${1:-}" ]; then one "${1%%:*}" "$([ "${1#*:}" = "$1" ] && echo "" || echo "${1#*:}")"; exit $?; fi
for pv in groq openrouter kilo ovh llm7 mistral gemini nim; do
  if one "$pv"; then exit 0; fi
done
echo "llm_route: every provider failed (no key or error)" >&2; exit 1
