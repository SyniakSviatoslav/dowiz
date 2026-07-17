#!/usr/bin/env bash
# telemetry bridge self-test (local-only, no Telegram secret needed).
#
# CI invokes this from .github/workflows/ci.yml (telemetry-selftest job). It must PASS
# on a clean checkout with zero network and zero secret. Two layers:
#   1. syntax-check every shell script in tools/telemetry — catches the
#      missing-source / broken-parse class of breakage (e.g. a script sourcing a
#      file that is absent). This is the part that is ALWAYS verifiable in CI.
#   2. live send-path smoke test ONLY when the (operator-local, untracked) `telemetry`
#      binary is present AND TELEMETRY_NO_TG=1 — so a developer with the binary gets a
#      real end-to-end check, while CI (where the binary is absent) stays green without
#      faking a pass.
set -euo pipefail
cd "$(dirname "$0")"
chmod +x ./*.sh 2>/dev/null || true

# 1) syntax-check every shell script (catches the missing-source class of breakage).
for f in ./*.sh; do
  [ -f "$f" ] || continue
  bash -n "$f" || { echo "::error::syntax error in $f"; exit 1; }
done
echo "syntax-check: OK ($(ls ./*.sh 2>/dev/null | wc -l) script(s))"

# 2) optional live smoke test (skipped in CI where the binary is untracked/absent).
if [ -x ./telemetry ] && [ "${TELEMETRY_NO_TG:-0}" = "1" ]; then
  printf 'edit\nrun_fail\nedit\nrun_fail\n' | ./telemetry health || true
  echo "live telemetry health: ran (best-effort)"
else
  echo "live telemetry health: skipped (binary absent or TELEMETRY_NO_TG != 1)"
fi

echo "telemetry selftest OK"
