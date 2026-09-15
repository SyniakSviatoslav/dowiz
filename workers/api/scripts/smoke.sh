#!/usr/bin/env bash
# Post-deploy smoke test. Proves the deployed Worker actually reaches the kernel
# and D1 -- not that it merely answers 200.
#
#   bash scripts/smoke.sh https://dowiz-api.<subdomain>.workers.dev
#
# What each step PROVES, because a 200 alone proves nothing:
#   1. health      -- the Worker booted inside the 1s startup budget.
#   2. place       -- the kernel priced the order (subtotal recomputed from items,
#                     never trusted from the wire) and D1 accepted the row.
#   3. read-back   -- the order SURVIVED the request. This is the whole point of
#                     D1: the native adapter's Mutex<HashMap> could not do this.
#   4. legal edge  -- PENDING -> CONFIRMED is accepted by the kernel FSM.
#   5. ILLEGAL edge-- the kernel REFUSES it with 400. If this returns 200, the
#                     Worker is deciding for itself and the whole design is void.
set -uo pipefail
BASE="${1:?usage: smoke.sh <worker-url>}"
ITEMS='[{"product_id":"item-01","modifier_ids":[],"quantity":2,"unit_price":900}]'
fail=0
note() { printf '%-42s %s\n' "$1" "$2"; }

code=$(curl -s -o /tmp/s1 -w '%{http_code}' "$BASE/healthz")
[ "$code" = 200 ] && note "1 health" "200 ok" || { note "1 health" "FAIL $code"; fail=1; }

code=$(curl -s -o /tmp/s2 -w '%{http_code}' -X POST "$BASE/api/order" \
  -H 'content-type: application/json' \
  -d "{\"items_json\":$(python3 -c 'import json,sys;print(json.dumps(sys.argv[1]))' "$ITEMS"),\"channel\":\"smoke\"}")
ID=$(python3 -c 'import json;print(json.load(open("/tmp/s2"))["id"])' 2>/dev/null || echo "")
SUB=$(python3 -c 'import json;print(json.load(open("/tmp/s2"))["subtotal"])' 2>/dev/null || echo "")
if [ "$code" = 200 ] && [ -n "$ID" ] && [ "$SUB" = 1800 ]; then
  note "2 place (subtotal 2x900=1800)" "200 id=$ID"
else
  note "2 place" "FAIL $code id=$ID subtotal=$SUB"; head -c 300 /tmp/s2; echo; fail=1
fi

code=$(curl -s -o /tmp/s3 -w '%{http_code}' "$BASE/api/order/$ID")
[ "$code" = 200 ] && note "3 read back from D1" "200 (order survived)" \
  || { note "3 read back from D1" "FAIL $code"; fail=1; }

code=$(curl -s -o /tmp/s4 -w '%{http_code}' -X POST "$BASE/api/order/$ID/advance" \
  -H 'content-type: application/json' -d '{"next_status":"CONFIRMED"}')
[ "$code" = 200 ] && note "4 legal edge PENDING->CONFIRMED" "200" \
  || { note "4 legal edge" "FAIL $code"; head -c 200 /tmp/s4; echo; fail=1; }

# The one that matters: DELIVERED is not reachable from CONFIRMED.
code=$(curl -s -o /tmp/s5 -w '%{http_code}' -X POST "$BASE/api/order/$ID/advance" \
  -H 'content-type: application/json' -d '{"next_status":"DELIVERED"}')
if [ "$code" = 400 ]; then
  note "5 ILLEGAL edge refused by kernel" "400 (correct): $(head -c 120 /tmp/s5)"
else
  note "5 ILLEGAL edge" "FAIL $code -- the kernel must refuse this"; fail=1
fi

echo; [ $fail = 0 ] && echo "SMOKE: all 5 passed" || echo "SMOKE: FAILURES above"
exit $fail
