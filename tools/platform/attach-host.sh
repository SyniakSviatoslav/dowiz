#!/usr/bin/env bash
# Attach one client hub's hostname to the Worker -- or, with `--cdn`, create
# the published-storefront bucket and put `cdn.dowiz.org` in front of it.
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
# THE CDN (BN2, workers/api/src/hubdo/publish.rs). The storefront reads each
# venue's published menu from an R2 bucket behind a custom domain, so the
# Worker is not in the read path. Three writes, all through the account API
# the deploy token can reach (Workers R2 Storage:Edit, 2026-09-26), each safe
# to repeat:
#   bash tools/platform/attach-host.sh --cdn [bucket=dowiz-cdn] [host=cdn.dowiz.org] [zone=dowiz.org]
#     1. the bucket (already exists: fine);
#     2. the custom domain on the bucket -- Cloudflare writes the DNS record
#        and the certificate; the first GET answers a minute or two later;
#     3. the bucket's CORS: the storefront is `https://<slug>.dowiz.org` and the
#        bucket is another origin, so without a CORS rule every fetch from
#        store/shell.js fails and the shell reads through the Worker forever.
#        GET/HEAD from `https://*.dowiz.org` and `https://dowiz.org`; a venue
#        on its own domain reads through the Worker until a rule names it.
# Then uncomment the `CDN` binding in workers/api/wrangler.toml and deploy.
#
# When the token grows `Workers Routes:Edit`, put the wildcard back in
# `wrangler.toml` and the first half of this file becomes unnecessary.
set -euo pipefail

. /root/.cf_deploy_token
api="https://api.cloudflare.com/client/v4"
auth=(-H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" -H "content-type: application/json")

zone_id_of() {
  curl -s "${auth[@]}" "$api/zones?name=$1" \
    | python3 -c 'import sys,json;r=json.load(sys.stdin)["result"];print(r[0]["id"] if r else "")'
}

# `say <label> <json>`: print the outcome; an "already exists" answer is not a failure.
say() {
  python3 - "$1" "$2" <<'PY'
import sys, json
label, raw = sys.argv[1], sys.argv[2]
try:
    d = json.loads(raw)
except json.JSONDecodeError:
    print(f"FAILED {label}: not JSON: {raw[:200]}", file=sys.stderr); sys.exit(1)
if d.get("success"):
    print(f"ok      {label}")
else:
    msgs = [f'{e.get("code")}: {e.get("message", "")}' for e in d.get("errors", [])]
    if any("already" in m.lower() or "exists" in m.lower() for m in msgs):
        print(f"ok      {label} (already there)")
    else:
        print(f"FAILED  {label}: {msgs}", file=sys.stderr); sys.exit(1)
PY
}

if [ "${1:-}" = "--cdn" ]; then
  bucket="${2:-dowiz-cdn}"
  host="${3:-cdn.dowiz.org}"
  zone_name="${4:-dowiz.org}"
  zone_id=$(zone_id_of "$zone_name")
  [ -n "$zone_id" ] || { echo "zone $zone_name not found or not readable by this token" >&2; exit 1; }
  acct="$api/accounts/$CLOUDFLARE_ACCOUNT_ID/r2/buckets"
  say "bucket $bucket" "$(curl -s -X POST "${auth[@]}" "$acct" -d "{\"name\":\"$bucket\"}")"
  say "domain $host -> $bucket" "$(curl -s -X POST "${auth[@]}" "$acct/$bucket/domains/custom" \
    -d "{\"domain\":\"$host\",\"zoneId\":\"$zone_id\",\"enabled\":true,\"minTLS\":\"1.2\"}")"
  say "cors on $bucket" "$(curl -s -X PUT "${auth[@]}" "$acct/$bucket/cors" -d '{"rules":[{
    "allowed":{"origins":["https://*.'"$zone_name"'","https://'"$zone_name"'"],"methods":["GET","HEAD"],"headers":["*"]},
    "exposeHeaders":["etag","content-length"],"maxAgeSeconds":86400}]}')"
  echo "read back: curl -sI https://$host/v/<slug>/manifest.json   (404 until a venue publishes; 200 with cache-control: public, max-age=30 after)"
  exit 0
fi

slug="${1:?usage: attach-host.sh <slug> [zone]   |   attach-host.sh --cdn [bucket] [host] [zone]}"
zone_name="${2:-dowiz.org}"
service="${SERVICE:-dowiz-api}"

zone_id=$(zone_id_of "$zone_name")
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
