#!/usr/bin/env bash
# G2 — DeepEval menu-parser eval orchestration (CI-only; isolated venv). Fail-closed order:
# preflight → generate outputs (real parser) → score. CI wraps this in a network egress allowlist.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Hard safety env (the in-process complement to the CI network allowlist).
export DEEPEVAL_TELEMETRY_OPT_OUT=1
export ERROR_REPORTING=0
export OTEL_SDK_DISABLED=true
export EVAL_EGRESS_ALLOWLIST="${EVAL_EGRESS_ALLOWLIST:-api.anthropic.com}"

echo "── G2 pre-flight (fail-closed) ──"
python3 "$HERE/preflight.py"

echo "── generate parser outputs (real AiOcrParser) ──"
( cd "$HERE/../.." && pnpm exec tsx eval/menu-parser/generate-outputs.ts )

echo "── DeepEval scoring (judge: Anthropic) ──"
python3 "$HERE/eval_menu_parser.py"
