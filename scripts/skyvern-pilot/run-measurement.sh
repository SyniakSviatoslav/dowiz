#!/usr/bin/env bash
# G4 — Skyvern failing-tail recovery measurement (OUT-OF-BAND, one-shot). NOT wired into the app, NOT
# in CI, NOT a fallback. Skyvern is AGPL and lives OUT OF TREE (its own container); we reach it ONLY
# over HTTP via $SKYVERN_BASE_URL. Records recovery-rate + cost-per-URL over the URLs the current
# Playwright + brand-extractor pipeline fails on. See docs/research/skyvern-pilot.md.
#
# Controls (ADR-tooling-integration-eval G4 / Breaker H4/RA-7):
#   - no-credential attestation on the sidecar env (machine-checked here);
#   - network egress allowlist {target URLs, local-LLM endpoint} is the LOAD-BEARING control — applied
#     at the sidecar's container/network layer (docker network + firewall), NOT in this script;
#   - SKYVERN_TELEMETRY=false + SKYVERN_LLM=<local model> on the sidecar;
#   - a sample of sent/received content gets a HUMAN third-party-PII review (recorded in the artifact);
#   - named expiry + owner: after expiry, `docker stop` the sidecar and freeze the measurement.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

: "${SKYVERN_BASE_URL:?set SKYVERN_BASE_URL (out-of-tree sidecar HTTP endpoint)}"
: "${SKYVERN_ENV_FILE:?set SKYVERN_ENV_FILE (the sidecar env file, for the no-credential attestation)}"
URLS_FILE="${1:?usage: run-measurement.sh <failing-urls.txt>}"

echo "── G4 no-credential attestation ──"
node "$HERE/no-credential-attest.mjs" "$SKYVERN_ENV_FILE"

echo "── reminder: the egress allowlist is the LOAD-BEARING control; it must already be applied at the"
echo "   sidecar network layer (only {target URLs, local-LLM} reachable). A host-string check is NOT proof."

ART="docs/research/skyvern-measurement.md"
{
  echo "# Skyvern measurement run"
  echo ""
  echo "| url | current-pipeline | skyvern-items | cost-usd |"
  echo "|---|---|---|---|"
} > "$ART"

count_items() { node "$HERE/count-items.mjs"; }

while IFS= read -r url || [ -n "$url" ]; do
  [ -z "$url" ] && continue
  payload="$(printf '{"url":"%s","goal":"extract the menu items and prices"}' "$url")"
  resp="$(curl -sS --max-time 180 -X POST "$SKYVERN_BASE_URL/api/v1/run" -H 'content-type: application/json' -d "$payload" || printf '{"error":"call_failed"}')"
  items="$(printf '%s' "$resp" | count_items)"
  printf '| %s | FAIL | %s | TODO |\n' "$url" "$items" >> "$ART"
  echo "measured: $url -> items=$items"
done < "$URLS_FILE"

echo "── wrote $ART. Now do the HUMAN third-party-PII review of a sample and record it in"
echo "   docs/research/skyvern-pilot.md, then docker stop the sidecar at expiry. ──"
