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
# d99 (generated, roadmap 1b 2026-09-06): one fn of 24000 statements emits > 65536 words ->
# the planning buffer traps 83 with a message (it was a SIGSEGV = exit 82 before).
python3 -c 'print("fn main() -> i64 {\n  let s = 1;"); [print("  let s = s + %d;" % (i % 7)) for i in range(24000)]; print("  s\n}")' > "$T/d99_cap.bp"
err=$(./seed/build/seed "$BIN" compile "$T/d99_cap.bp" "$T/d99_cap.bin" 2>&1 >/dev/null); rc=$?
if [ "$rc" = 83 ] && [ "$err" = "code buffer exhausted at 65536 words (one fn or the program)" ]; then echo "PASS d99_cap exit 83: $err"; pass=$((pass+1));
else echo "FAIL d99_cap want exit 83 + message, got exit $rc: $err"; fail=$((fail+1)); fi
echo "diag: $pass pass, $fail fail"; [ "$fail" = 0 ]
