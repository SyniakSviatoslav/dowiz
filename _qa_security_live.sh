#!/usr/bin/env bash
# The red team's findings, re-probed against the DEPLOYED Worker.
#
# Every one of these was a live request that worked on 2026-09-21. A fix is not
# a fix until the same request is refused by the thing on the internet.
set -uo pipefail
BASE="${1:-https://sushi-durres.dowiz.org}"
OTHER="${2:-https://dubin-sushi.dowiz.org}"
fail=0
probe() { # name expected_codes curl-args...
  local name="$1" want="$2"; shift 2
  local code; code=$(curl -s -o /tmp/sec_out -w '%{http_code}' "$@")
  if [[ " $want " == *" $code "* ]]; then
    printf 'ok   %-52s %s\n' "$name" "$code"
  else
    printf 'FAIL %-52s got %s, wanted one of [%s]: %s\n' "$name" "$code" "$want" "$(head -c 120 /tmp/sec_out)"
    fail=1
  fi
}

echo "— the two deleted write routes —"
probe "POST /api/order (unauthenticated placement)" "404 405" \
  -X POST "$BASE/api/order" -H 'content-type: application/json' \
  -d '{"customer_id":"qa","items_json":"[{\"product_id\":\"item-01\",\"modifier_ids\":[],\"quantity\":1,\"unit_price\":0}]","channel":"qa"}'
probe "POST /api/order/:id/advance (anyone's order)" "404 405" \
  -X POST "$BASE/api/order/ord_qa_probe/advance" -H 'content-type: application/json' \
  -d '{"next_status":"CANCELLED"}'

echo "— the wallet, which minted ledger money —"
probe "POST wallet/topup without a token" "401 403 404" \
  -X POST "$BASE/api/public/locations/sushi-durres/wallet/topup" -H 'content-type: application/json' \
  -d '{"user":"qa","amountMinor":100000000,"currency":"ALL","providerRef":"x","requestId":"qa-probe-1"}'
probe "GET wallet balance by ?user=" "401 403 404" \
  "$BASE/api/public/locations/sushi-durres/wallet?user=qa"
probe "GET wallet statement by ?user=" "401 403 404" \
  "$BASE/api/public/locations/sushi-durres/wallet/statement?user=qa"

echo "— reservations and threads: PII read and unauthenticated write —"
probe "GET a reservation by id" "401 403 404" \
  "$BASE/api/public/locations/sushi-durres/reservations/rsv_0000000000000000"
probe "POST a reservation action" "401 403 404" \
  -X POST "$BASE/api/public/locations/sushi-durres/reservations/rsv_0000000000000000/action" \
  -H 'content-type: application/json' -d '{"to":"SEATED","actor":"qa"}'
probe "GET an entry pass" "401 403 404" \
  "$BASE/api/public/locations/sushi-durres/reservations/rsv_0000000000000000/pass"
probe "GET reservations by ?user=" "401 403 404" \
  "$BASE/api/public/locations/sushi-durres/reservations?user=qa"
probe "POST into a thread as the venue" "401 403 404" \
  -X POST "$BASE/api/public/locations/sushi-durres/threads/thr_qa/messages" \
  -H 'content-type: application/json' -d '{"from":"VENUE","body":"qa","clientId":"qa-1"}'

echo "— an order id is not a key —"
probe "GET /api/order/:id with no token" "401 404" "$BASE/api/order/ord_qa_probe"

echo "— the socket —"
probe "GET /api/live without an upgrade" "426 401 404" "$BASE/api/live"

echo "— what must still work —"
probe "GET /healthz" "200" "$BASE/healthz"
probe "GET the storefront menu" "200" "$BASE/api/public/locations/sushi-durres/menu"
probe "GET the landing" "200" "https://dowiz.org/"
probe "GET the other venue's storefront" "200" "$OTHER/"

echo
[ $fail = 0 ] && echo "LIVE SECURITY: every probe refused or served as it should" || echo "LIVE SECURITY: FAILURES above"
exit $fail
