#!/bin/sh
# no-scoring's proof: a gate is triggered before it is trusted (W-MR0 2026-10-04, GATE-1 revised).
# Every case scans a SCRATCH tree (NO_SCORING_ROOT) with a scratch baseline of 0, so the product's
# baseline file is never rewritten by the ratchet. The scope is the gate header's: a guest may be
# scored on the device and in the one server scorer module, never priced by.
#   1. clean scratch                                                    -> pass
#   2. `guest_taste` in the guest's browser (public/store)              -> pass  (device scoring allowed)
#   3. the same name in a server file                                   -> refuse
#   4. the same name in the one scorer (services/customers/taste) -> pass
#   5. the one scorer naming a discount                           -> refuse (no pricing by score)
#   6. the device scorer (store/taste.js) naming a price                -> refuse
#   7. `price_sensitivity` in the browser                               -> refuse (everywhere)
#   8. `CUSTOMER_LTV` constant on the server (case-insensitive)         -> refuse
#   9. `courier_score` on the server (the original OD-8 pattern)        -> refuse
#  10. a `customer_tier` even inside the one scorer               -> refuse (a tier is a verdict)
#  11. the words only in a `//` comment on the server                   -> pass
#  12. the product tree with its own baseline                           -> pass
#  13. feedback topics folded per courier on the server (P16b)          -> refuse
#  14. feedback topics per dish on the server (P16b)                    -> pass
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
fail=0
want() { # want <rc> <label> [env...]
  rc_want=$1; label=$2; shift 2
  echo 0 > "$S/base"
  env "$@" sh "$HERE/no-scoring.sh" >"$S/out" 2>&1; rc=$?
  echo "prove: $label -> rc=$rc (want $rc_want): $(grep -m1 'FAILED' "$S/out" || tail -1 "$S/out")"
  [ "$rc" -eq "$rc_want" ] || { cat "$S/out"; fail=1; }
}
plant() { # plant <relative path> <line>
  rm -rf "$S/r"; mkdir -p "$S/r/workers/api/src" "$S/r/workers/api/public/store" "$(dirname "$S/r/$1")"
  printf '%s\n' "$2" > "$S/r/$1"
}
scan() { want "$1" "$2" NO_SCORING_ROOT="$S/r" NO_SCORING_BASELINE="$S/base"; }
RS=workers/api/src
plant $RS/menu.rs 'pub struct Menu { pub dishes: u32 }';                                    scan 0 clean
plant workers/api/public/store/taste.js 'export const guest_taste = new Map();';             scan 0 "guest_taste on the device"
plant $RS/orders.rs 'pub struct O { pub guest_taste: Vec<u32> }';                           scan 1 "guest_taste on the server"
plant $RS/services/customers/taste.rs 'pub struct P { pub guest_taste: Vec<u32> }';         scan 0 "guest_taste in the one scorer"
plant $RS/services/customers/taste.rs 'pub fn apply_discount(t: u32) -> u32 { t }';         scan 1 "the scorer naming a discount"
plant workers/api/public/store/taste.js 'export const cheaper = p => p.price - 1;';          scan 1 "the device scorer naming a price"
plant workers/api/public/store/menu.js 'const price_sensitivity = 3;';                      scan 1 "price_sensitivity in the browser"
plant $RS/x.rs 'const CUSTOMER_LTV: u64 = 0;';                                              scan 1 "CUSTOMER_LTV on the server"
plant $RS/x.rs 'fn courier_score() -> u32 { 0 }';                                           scan 1 "courier_score on the server"
plant $RS/services/customers/taste.rs 'pub enum customer_tier { Gold }';                    scan 1 "a tier inside the scorer"
plant $RS/x.rs '// never a guest_taste or a willingness_to_pay here';                       scan 0 "words only in a comment"
plant $RS/services/analytics/kitchen/topics.rs 'pub fn topics_by_courier() -> u32 { 0 }';    scan 1 "topics per courier"
plant $RS/services/analytics/kitchen/topics.rs 'pub fn dish_topics() -> u32 { 0 }';            scan 0 "topics per dish"
cp "$REPO/tools/gates/no-scoring.baseline" "$S/real-base"
want 0 "product tree" NO_SCORING_BASELINE="$S/real-base"
[ "$fail" -eq 0 ] && echo "no-scoring.prove: GREEN -- 14 of 14 cases" || echo "no-scoring.prove: RED"
exit "$fail"
