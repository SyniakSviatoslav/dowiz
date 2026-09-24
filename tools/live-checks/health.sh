#!/usr/bin/env bash
# PRODUCTION HEALTH, READ-ONLY. What `.github/workflows/health-cron.yml` runs every 15 minutes,
# and what anyone can run from a laptop:
#
#   bash tools/live-checks/health.sh
#   HEALTH_VENUES="sushi-durres dubin-sushi" PLATFORM=https://dowiz.org bash tools/live-checks/health.sh
#
# WHAT IT NEVER DOES: log in, place an order, or send a body to a route that writes. Every probe is
# a GET, except the ETA quote, which is a POST that computes and stores nothing
# (`workers/api/src/eta.rs::quote`). No secret is needed or read.
#
# WHY NOT `workers/api/scripts/smoke.sh`: that is the post-deploy smoke, and its step 5 POSTs a
# CANCELLED body at the two deleted legacy write routes to prove they stay deleted. Once, after a
# deploy, that is a proof; 96 times a day it would be a standing write attempt against a real venue.
#
# A 200 PROVES LITTLE, so each probe checks what the answer says (status-code-is-not-working):
#   healthz   the Worker booted and routed                       200 and body "ok"
#   store     the storefront document, with its CSP header        200, text/html, content-security-policy
#   menu      the venue's catalogue image was read               200 and more than 0 dishes
#   quote     the kernel priced a wait from the venue's profile  200 and a "range" field
#   privacy   an order id alone is not a key                     401 or 404, never 200
#   apps      console, courier and room shells are served        200 each
#   manifest  the PWA manifest                                   200
# and on the platform host: the landing page and /healthz.
#
# EXIT: 0 when every probe passed; 1 when any failed, with the failing rows marked FAIL.
set -u
VENUES="${HEALTH_VENUES:-sushi-durres dubin-sushi}"
DOMAIN="${HEALTH_DOMAIN:-dowiz.org}"
PLATFORM="${PLATFORM:-https://$DOMAIN}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
fail=0
rows=0

row() { # row <ok|FAIL> <what> <detail>
  rows=$((rows + 1))
  printf '%-4s %-44s %s\n' "$1" "$2" "$3"
  [ "$1" = ok ] || fail=1
}
get() { # get <url> <outfile> -> prints the status code ("000" when unreachable)
  curl -sS -o "$2" -D "$2.h" -w '%{http_code}' --max-time 20 --retry 2 --retry-delay 5 \
    -H 'user-agent: dowiz-health-cron' "$1" 2>"$2.err" || true
}

for slug in $VENUES; do
  base="https://$slug.$DOMAIN"
  f="$TMP/$slug"

  c=$(get "$base/healthz" "$f.hz")
  if [ "$c" = 200 ] && [ "$(cat "$f.hz")" = ok ]; then row ok "$slug healthz" "200 ok"
  else row FAIL "$slug healthz" "$c $(head -c 80 "$f.hz" 2>/dev/null) $(cat "$f.hz.err" 2>/dev/null)"; fi

  c=$(get "$base/" "$f.root")
  if [ "$c" = 200 ] && grep -qi '^content-type: text/html' "$f.root.h" \
     && grep -qi '^content-security-policy:' "$f.root.h"; then row ok "$slug storefront" "200 html, CSP present"
  else row FAIL "$slug storefront" "$c (needs 200, text/html and a CSP header)"; fi

  c=$(get "$base/api/public/locations/$slug/menu" "$f.menu")
  n=$(python3 -c 'import json,sys
d=json.load(open(sys.argv[1]))
print(sum(len(c.get("products") or []) for c in (d.get("categories") or [])))' "$f.menu" 2>/dev/null || echo 0)
  if [ "$c" = 200 ] && [ "${n:-0}" -gt 0 ]; then row ok "$slug menu" "200, $n dishes"
  else row FAIL "$slug menu" "$c, dishes=${n:-0}"; fi

  c=$(curl -sS -o "$f.eta" -w '%{http_code}' --max-time 20 --retry 2 --retry-delay 5 \
    -H 'user-agent: dowiz-health-cron' -H 'content-type: application/json' -X POST \
    -d '{"items":[{"quantity":1}],"pickup":true}' \
    "$base/api/public/locations/$slug/eta" 2>/dev/null || true)
  if [ "$c" = 200 ] && grep -q '"range"' "$f.eta"; then row ok "$slug quote (kernel ETA)" "200 $(grep -o '"range":"[^"]*"' "$f.eta")"
  else row FAIL "$slug quote (kernel ETA)" "$c $(head -c 80 "$f.eta" 2>/dev/null)"; fi

  c=$(get "$base/api/order/ord_health_probe" "$f.ord")
  case "$c" in
    401|404) row ok "$slug order id alone refused" "$c" ;;
    *) row FAIL "$slug order id alone refused" "$c -- an id must not be a key" ;;
  esac

  for app in admin courier room; do
    c=$(get "$base/$app/" "$f.$app")
    if [ "$c" = 200 ]; then row ok "$slug /$app/" "200"; else row FAIL "$slug /$app/" "$c"; fi
  done

  c=$(get "$base/manifest.webmanifest" "$f.mf")
  if [ "$c" = 200 ]; then row ok "$slug manifest" "200"; else row FAIL "$slug manifest" "$c"; fi
done

c=$(get "$PLATFORM/" "$TMP/platform")
if [ "$c" = 200 ]; then row ok "platform landing" "200"; else row FAIL "platform landing" "$c"; fi
c=$(get "$PLATFORM/healthz" "$TMP/platform.hz")
if [ "$c" = 200 ]; then row ok "platform healthz" "200"; else row FAIL "platform healthz" "$c"; fi

echo
if [ $fail = 0 ]; then echo "HEALTH: all $rows probes passed ($(date -u +%FT%TZ))"
else echo "HEALTH: FAILURES above ($(date -u +%FT%TZ))"; fi
exit $fail
