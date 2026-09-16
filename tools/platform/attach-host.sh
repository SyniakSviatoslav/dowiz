#!/usr/bin/env bash
# Attach one client hub's hostname to the Worker.
#
# WHY THIS EXISTS RATHER THAN A WILDCARD ROUTE. `*.dowiz.org/*` is the right
# shape and needs one permission this deploy token does not have --
# `Workers Routes:Edit` on the dowiz.org zone. Worse, wrangler reconciles
# `routes` by LISTING the zone's routes first, so merely having a `routes` key
# in `wrangler.toml` fails the deploy outright and the Worker never ships.
#
# So each client host is attached as its own Workers CUSTOM DOMAIN, which the
# deploy token can write. Cloudflare creates the proxied DNS record and issues
# the certificate; it takes a minute or two before the host answers.
#
# Run it once per hub created from the main hub:
#   bash tools/platform/attach-host.sh sushi-durres
#
# When the token grows `Workers Routes:Edit`, put the wildcard back in
# `wrangler.toml` and this file becomes unnecessary.
set -euo pipefail
slug="${1:?usage: attach-host.sh <slug> [zone]}"
zone_name="${2:-dowiz.org}"
service="${SERVICE:-dowiz-api}"

. /root/.cf_deploy_token
api="https://api.cloudflare.com/client/v4"
auth=(-H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" -H "content-type: application/json")

zone_id=$(curl -s "${auth[@]}" "$api/zones?name=$zone_name" \
  | python3 -c 'import sys,json;r=json.load(sys.stdin)["result"];print(r[0]["id"] if r else "")')
[ -n "$zone_id" ] || { echo "zone $zone_name not found or not readable by this token" >&2; exit 1; }

out=$(curl -s -X PUT "${auth[@]}" "$api/accounts/$CLOUDFLARE_ACCOUNT_ID/workers/domains" \
  -d "{\"environment\":\"production\",\"hostname\":\"$slug.$zone_name\",\"service\":\"$service\",\"zone_id\":\"$zone_id\"}")

python3 - "$out" "$slug.$zone_name" <<'PY'
import sys, json
d = json.loads(sys.argv[1]); host = sys.argv[2]
if d.get("success"):
    print(f"attached {host} -> {d['result'].get('service')}")
else:
    # Already attached is not a failure; re-running this must be safe.
    msgs = [e.get("message", "") for e in d.get("errors", [])]
    if any("already" in m.lower() for m in msgs):
        print(f"{host} was already attached")
    else:
        print(f"FAILED for {host}: {msgs}", file=sys.stderr); sys.exit(1)
PY
