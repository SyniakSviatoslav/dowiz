#!/usr/bin/env bash
# Академія Дмитра Євдокимова — безперервний seed server.
# Працює 24/7 на цій машині, екстрактує всі source по черзі.
# Zero-trace: випадковий User-Agent, jitter, chaff.

set -euo pipefail

export HF_TOKEN="${HF_TOKEN:-}"
if [ -z "$HF_TOKEN" ]; then
    echo "❌ Потрібен HF_TOKEN"
    exit 1
fi

REPO="Delulu12/academia-matrix"
BOT="https://hf.co/datasets/${REPO}/resolve/main/bot.sh"

echo "📡 Academia Seed Server — 24/7 autonomous extraction"
echo "   HF_TOKEN: ${HF_TOKEN:0:8}..."
echo "   Repo:     ${REPO}"
echo "   Bot:      ${BOT}"

while true; do
    # Випадковий source кожні 30-60 хвилин
    SOURCES=("arxiv" "semantic" "openalex")
    SOURCE=${SOURCES[$RANDOM % ${#SOURCES[@]}]}
    
    echo ""
    echo "=== $(date) — Extracting from ${SOURCE} ==="
    
    # Jitter перед стартом
    sleep $((RANDOM % 120 + 30))
    
    # Запуск бота
    curl -sL "${BOT}" | \
        HF_TOKEN="${HF_TOKEN}" SOURCE="${SOURCE}" bash
    
    # Випадкова пауза між source (30-60 min)
    PAUSE=$((RANDOM % 1800 + 1800))
    echo "   Next run in ${PAUSE}s ($((PAUSE/60))min)..."
    sleep $PAUSE
done
