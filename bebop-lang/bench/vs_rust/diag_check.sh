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
# T90 step 2b (2026-09-06): `check <src>` = the same diagnostics, no output file
err=$(./seed/build/seed "$BIN" check bench/diag_neg/d01_paren.bp 2>&1 >/dev/null); rc=$?
if [ "$rc" = 95 ] && [ "$(echo "$err" | tail -n 1 | cut -d: -f1,2)" = 4:17 ]; then echo "PASS check d01 4:17 exit 95"; pass=$((pass+1));
else echo "FAIL check d01 want 4:17 exit 95, got exit $rc: $err"; fail=$((fail+1)); fi
rm -f "$T/c53.bin"; out=$(./seed/build/seed "$BIN" check bench/parity_constructs/c53_param9.bp 2>&1); rc=$?
if [ "$rc" = 0 ] && [ ! -e bench/parity_constructs/c53_param9.bin ]; then echo "PASS check c53_param9 exit 0, no .bin written"; pass=$((pass+1));
else echo "FAIL check c53_param9 want exit 0 and no .bin, got exit $rc: $out"; fail=$((fail+1)); fi
# T90 step 2c (2026-09-06): runtime traps are `brk #code`; the entry stub's SIGTRAP handler
# writes `trap NN: <text>` on stderr and exits with the code (82 = the SIGSEGV/SIGBUS handler).
printf 'fn main() -> i64 {\n  let a = zeros(40000000);\n  a[0]\n}\n' > "$T/t80.bp"
python3 -c 'lit="["+",".join(["1"]*511)+"]"; print("fn main() -> i64 {"); [print("  let a%d = %s;" % (i,lit)) for i in range(4)]; print("  a0[0] + a1[1] + a2[2] + a3[3]\n}")' > "$T/t81.bp"
printf 'fn r(n: i64) -> i64 {\n  r(n + 1)\n}\nfn main() -> i64 {\n  r(0)\n}\n' > "$T/t82.bp"
printf 'fn main() -> i64 {\n  nosuch(1)\n}\n' > "$T/t87.bp"
while read -r code text; do
  ./seed/build/seed "$BIN" compile "$T/t$code.bp" "$T/t$code.bin" >/dev/null 2>&1 || { echo "FAIL trap $code: compile failed"; fail=$((fail+1)); continue; }
  err=$(timeout 20 ./seed/build/seed "$T/t$code.bin" 2>&1 >/dev/null); rc=$?
  if [ "$rc" = "$code" ] && [ "$err" = "trap $code: $text" ]; then echo "PASS trap $code: $text"; pass=$((pass+1));
  else echo "FAIL trap $code want exit $code + 'trap $code: $text', got exit $rc: $err"; fail=$((fail+1)); fi
done <<'TRAPS'
80 arena exhausted (zeros crossed x28)
81 frame heap exhausted (array literal or enum ctor)
82 SIGSEGV/SIGBUS (stack overflow or wild access)
87 call to an unresolved function
TRAPS
echo "diag: $pass pass, $fail fail"; [ "$fail" = 0 ]
