#!/usr/bin/env bash
# hermes-verify-selfcompile-bridge.sh — verify self-compilation bridge works
set -euo pipefail
cd /root/dowiz/bebop-lang
BEBOPC="/lib/ld-linux-aarch64.so.1 ./native/build/bebopc"
BP="../selfhost/expr_compile.bp"

echo "=== 1. make test ==="
make -C native test 2>&1 | tail -3

echo "=== 2. check ==="
"$BEBOPC" check "$BP" 2>&1 | tail -3

echo "=== 3. strict ==="
"$BEBOPC" strict "$BP" 2>&1 | tail -3

echo "=== 4. self_check via run ==="
"$BEBOPC" run "$BP" self_check 2>&1

echo "=== 5. simple selfcompile ==="
echo 'fn main() { 1 }' > /tmp/tiny.bp
"$BEBOPC" selfcompile /tmp/tiny.bp 2>&1 || true

echo "=== VERIFICATION COMPLETE ==="
