#!/usr/bin/env bash
# wasm_check.sh — B3-5 release-gate slice: regenerate the parity corpus with
# `bebopc compile`, then validate AND execute each module in a real wasm
# runtime (node/V8) via wasm_check.mjs. Division cases are included since
# codegen.c now emits i64.div_s (0x7f); they trap on /0, so all divisors are
# nonzero constants.
set -u
cd "$(dirname "$0")/.."
BIN=build/bebopc
OUT=build/wasm
mkdir -p "$OUT"
pass=0; fail=0
while IFS='|' read -r expr want; do
  [ -z "${expr:-}" ] && continue
  f="$OUT/case$pass.wasm"
  if ! "$BIN" compile "$expr" "$f" >/dev/null 2>&1; then
    echo "FAIL(emission): $expr"; fail=$((fail+1)); continue
  fi
  if node wasm_check.mjs "$f" "$want" >/dev/null 2>&1; then
    pass=$((pass+1))
  else
    echo "FAIL(exec): $expr (want $want)"; fail=$((fail+1))
  fi
done <<'CASES'
1 + 2 * 3|7
(3 + 4) * 2|14
100 - 50|50
7 * 6|42
42|42
0|0
300|300
70000|70000
1 - 2 - 3|-4
(1 + 2) * (3 + 4)|21
((7))|7
2 * 3 + 4 * 5|26
100 - (20 + 30)|50
123456 * 654321|80779853376
1 + 1 + 1 + 1 + 1 + 1|6
(5)|5
8 * 1000000|8000000
9 - 8 + 7 - 6 + 5|7
2 * (30000 + 777)|61554
500 - 250 * 2|0
84 / 12 - 6|1
(200 / 8) * 3|75
CASES
echo "wasm-check: $pass executed OK, $fail failed (runtime: node/$(node --version))"
[ "$fail" -eq 0 ]
