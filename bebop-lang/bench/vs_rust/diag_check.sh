#!/usr/bin/env bash
# diag_check.sh (T90 step 1, 2026-09-06): every bench/diag_neg/*.bp carries a hand-counted
# `// EXPECT line:col code` header; the compiler must exit with that code AND write exactly
# `line:col: <message>` on stderr. Positions are hand-counted, never copied from the output.
# env: BEBOP_BIN, BEBOP_TMP.
cd "$(dirname "$0")/../.." || exit 1
BIN=${BEBOP_BIN:-./bebop.bin}; T=${BEBOP_TMP:-/tmp/opencode}/diag; mkdir -p "$T"
[ -s "$BIN" ] || { echo "GUARD: $BIN missing or empty (L12)"; exit 1; }
pass=0; fail=0
for f in bench/diag_neg/*.bp; do
  b=$(basename "$f" .bp); read -r _ _ want code < <(head -n 1 "$f")
  err=$(./seed/build/seed "$BIN" compile "$f" "$T/$b.bin" 2>&1 >/dev/null); rc=$?
  got=$(echo "$err" | tail -n 1 | cut -d: -f1,2)
  if [ "$rc" = "$code" ] && [ "$got" = "$want" ]; then echo "PASS $b $got exit $rc: $(echo "$err" | tail -n 1 | cut -d: -f3-)"; pass=$((pass+1));
  else echo "FAIL $b want $want exit $code, got '$got' exit $rc: $err"; fail=$((fail+1)); fi
done
echo "diag: $pass pass, $fail fail"; [ "$fail" = 0 ]
