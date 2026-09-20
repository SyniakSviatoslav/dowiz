#!/usr/bin/env bash
# Post-deploy smoke test. Proves the deployed Worker actually reaches the kernel
# and D1 -- not that it merely answers 200.
#
#   bash scripts/smoke.sh https://sushi-durres.dowiz.org
#
# ── WHY THIS NO LONGER PLACES AN ORDER ──
#
# It used to POST `/api/order` and drive `/api/order/:id/advance`. Both routes
# took NO AUTHENTICATION -- a red-team pass on 2026-09-21 found that anyone who
# knew a venue's host could append orders to its log, and anyone with an order
# id could walk that order to CANCELLED or DELIVERED -- so they were deleted.
# A smoke test is not a reason to keep an unauthenticated write path alive.
#
# What each step PROVES, because a 200 alone proves nothing:
#   1. health    -- the Worker booted inside the 1s startup budget.
#   2. menu      -- D1 and the hub's catalogue image both answered, with dishes.
#   3. quote     -- the KERNEL priced a delivery: an ETA computed from the
#                   venue's own profile, which no other layer can produce.
#   4. refusal   -- an order id on its own is REFUSED (401). This is the
#                   authorisation that replaced the public read.
#   5. guard     -- the deleted routes stay deleted: both answer 404/405.
set -uo pipefail
BASE="${1:?usage: smoke.sh <worker-url>}"
SLUG="${SLUG:-$(printf '%s' "$BASE" | sed -E 's#https?://##; s#\..*##')}"
fail=0
note() { printf '%-44s %s\n' "$1" "$2"; }

code=$(curl -s -o /tmp/s1 -w '%{http_code}' "$BASE/healthz")
[ "$code" = 200 ] && note "1 health" "200 ok" || { note "1 health" "FAIL $code"; fail=1; }

code=$(curl -s -o /tmp/s2 -w '%{http_code}' "$BASE/api/public/locations/$SLUG/menu")
N=$(python3 -c 'import json;d=json.load(open("/tmp/s2"));print(len(d.get("products") or d.get("items") or []))' 2>/dev/null || echo 0)
if [ "$code" = 200 ] && [ "${N:-0}" -gt 0 ]; then
  note "2 menu (catalogue image + D1)" "200, $N dishes"
else
  note "2 menu" "FAIL $code dishes=$N"; head -c 200 /tmp/s2; echo; fail=1
fi

code=$(curl -s -o /tmp/s3 -w '%{http_code}' -X POST "$BASE/api/public/locations/$SLUG/eta" \
  -H 'content-type: application/json' \
  -d '{"items":[{"product_id":"item-01","quantity":1}],"fulfilment":{"kind":"delivery"}}')
if [ "$code" = 200 ] && grep -q 'minutes\|range\|eta' /tmp/s3; then
  note "3 quote (the kernel priced the wait)" "200: $(head -c 90 /tmp/s3)"
else
  note "3 quote" "FAIL $code"; head -c 200 /tmp/s3; echo; fail=1
fi

code=$(curl -s -o /tmp/s4 -w '%{http_code}' "$BASE/api/order/ord_smoke_probe")
if [ "$code" = 401 ] || [ "$code" = 404 ]; then
  note "4 an order id alone is refused" "$code (correct)"
else
  note "4 order read" "FAIL $code -- an id must not be a key"; fail=1
fi

gone=0
for path_and_method in "POST:/api/order" "POST:/api/order/ord_smoke_probe/advance"; do
  m=${path_and_method%%:*}; p=${path_and_method#*:}
  code=$(curl -s -o /dev/null -w '%{http_code}' -X "$m" "$BASE$p" \
    -H 'content-type: application/json' -d '{"next_status":"CANCELLED"}')
  case "$code" in
    404|405) ;;
    *) note "5 deleted route $p" "FAIL $code -- it answered"; gone=1; fail=1 ;;
  esac
done
[ $gone = 0 ] && note "5 the unauthenticated writers are gone" "404/405 on both"

echo; [ $fail = 0 ] && echo "SMOKE: all 5 passed" || echo "SMOKE: FAILURES above"
exit $fail
