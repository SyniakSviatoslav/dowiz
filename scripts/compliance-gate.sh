#!/bin/bash
# Compliance CI gate — P50 pre-commit check
# Скрипт перевіряє, чи немає порушень політик відповідності

set -e

PASS=true

# 1. PII columns detection in schemas
echo "[compliance] Checking PII column annotations..."
PII_FILES=$(grep -rn "pii\|PII\|phone\|email.*\|tax_id\|iban" --include="*.rs" kernel/src/ | grep -v "test\|fn.*pii\|//.*pii" | head -10)
if [ -n "$PII_FILES" ]; then
    echo "  ⚠  Potential PII fields without annotations:"
    echo "$PII_FILES"
fi

# 2. No raw secrets in source
echo "[compliance] Checking for secrets in source..."
if grep -rn "BEGIN PGP\|PRIVATE KEY\|api_key=\|password=" --include="*.rs" kernel/src/ 2>/dev/null | grep -v "test\|check_output\|PGP PRIVATE" | head -5; then
    echo "  ✗ Secrets found in source!"
    PASS=false
fi

# 3. Data retention: check for TTL/deletion logic in store modules
echo "[compliance] Checking data retention markers..."
RETENTION=$(grep -rn "retention\|ttl\|expir\|delete\|erase" --include="*.rs" kernel/src/retrieval/ kernel/src/event_log.rs 2>/dev/null | head -5)
if [ -z "$RETENTION" ]; then
    echo "  ⚠  No data retention markers found in storage modules"
fi

# 4. Consent recording check
echo "[compliance] Checking consent recording..."
CONSENT=$(grep -rn "consent\|opt.?in\|gdpr" --include="*.rs" kernel/src/ | head -3)
if [ -z "$CONSENT" ]; then
    echo "  ⚠  No consent recording found in kernel"
fi

if [ "$PASS" = true ]; then
    echo "[compliance] ✓ Gate passed"
else
    echo "[compliance] ✗ Gate FAILED"
    exit 1
fi
