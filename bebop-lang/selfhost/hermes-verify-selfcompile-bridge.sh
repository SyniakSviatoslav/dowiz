#!/usr/bin/env bash
# hermes-verify-selfcompile-bridge.sh — verify self-compilation bridge works
set -euo pipefail
BEBOPC="/root/dowiz/bebop-lang/native/build/bebopc"
BP="../selfhost/expr_compile.bp"

echo "=== 1. make test ==="
make -C /root/dowiz/bebop-lang/native test 2>&1 | tail -3

echo "=== 2. check ==="
"$BEBOPC" check "$BP" 2>&1 | tail -3

echo "=== 3. strict ==="
"$BEBOPC" strict "$BP" 2>&1 | tail -3

echo "=== 4. self_check via run ==="
"$BEBOPC" run "$BP" "self_check()" 2>&1

echo "=== 5. simple selfcompile ==="
"$BEBOPC" selfcompile /tmp/tiny.bp 2>&1 || true

echo "=== VERIFICATION COMPLETE ==="
