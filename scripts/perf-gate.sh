#!/usr/bin/env bash
# Live-latency gate against the DEPLOYED service.
#
# Measures 7 endpoints over one keep-alive connection, 3 interleaved rounds of
# 5 samples each, reports the MEDIAN and IQR of server time (round trip minus
# the same run's /healthz median), and compares against the committed
# scripts/perf-baseline.json.
#
# It fails ONLY on a regression it can defend:
#   median > baseline Q3 + max(60 ms, 25 % of baseline median)
#   AND a majority of the rounds are above that same threshold.
# The spread that justifies those numbers is written down at the top of
# tools/live-checks/bench.py, with the measurements it came from. The table
# it prints says, per endpoint, the smallest regression it can actually see.
#
# Exit 0: no defensible regression.   Exit 1: regression, reproduced.
# Exit 2: the service could not be measured (login failed, 5xx burst, no
#         baseline) -- reported loudly, never counted as a pass.
#
# Re-baseline after an INTENDED change in server work:
#   python3 tools/live-checks/bench.py --record     # pools into the baseline
# (Run it at least twice, minutes apart; a baseline from one minute measures
# that minute.)
set -uo pipefail
cd "$(dirname "$0")/.."
echo "— perf-gate: $(date -u +%FT%TZ) → https://dowiz-api.sviatoslavsyniak.workers.dev —"
python3 tools/live-checks/bench.py "$@"
rc=$?
case $rc in
  0) echo "perf-gate: PASS — жодної відтвореної регресії проти scripts/perf-baseline.json" ;;
  1) echo "perf-gate: FAIL — регресія відтворилася у більшості раундів (рядки з ✗ вище)" ;;
  *) echo "perf-gate: НЕ ВИМІРЯНО (exit $rc) — вердикту немає; це не PASS" ;;
esac
exit $rc
