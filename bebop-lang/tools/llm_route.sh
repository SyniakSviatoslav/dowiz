#!/usr/bin/env bash
# llm_route.sh (operator token-economy priority 2026-09-07): route a ROUTINE task to a free OpenRouter model.
# Usage: tools/llm_route.sh [model] < prompt.txt   (key: ~/.config/openrouter/key, chmod 600; never echo it)
# Default model = the first free model that answered on 2026-09-07; override with LLM_MODEL or $1.
set -u
K=$(cat "$HOME/.config/openrouter/key") || { echo "llm_route: no key at ~/.config/openrouter/key" >&2; exit 1; }
M=${1:-${LLM_MODEL:-nvidia/nemotron-3.5-lightning:free}}
P=$(python3 -c 'import sys,json; print(json.dumps(sys.stdin.read()))')
curl -s --max-time 120 https://openrouter.ai/api/v1/chat/completions -H "Authorization: Bearer $K" -H "Content-Type: application/json" \
  -d "{\"model\":\"$M\",\"max_tokens\":${LLM_MAX:-1200},\"messages\":[{\"role\":\"user\",\"content\":$P}]}" \
  | python3 -c "import sys,json; j=json.load(sys.stdin); c=j.get('choices'); print(c[0]['message']['content'] if c else 'llm_route error: '+json.dumps(j.get('error'))[:300])"
