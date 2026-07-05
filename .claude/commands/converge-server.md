---
description: Серверна convergence-петля — звести хендлери/Zod/міграції у відповідність контракту й інваріантам (RLS FORCE, integer, idempotency, RS256). Сервер тут — ціль (на відміну від error-fix, де read-only).
argument-hint: <опц. скоуп: домен/ендпоінт>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю backend-contract-convergence (loops/backend-contract-convergence.yaml). Скоуп: «$ARGUMENTS».
Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; дотримуй гейти (STOP-CONTRACT-MAP, STOP-MIGRATION) й iron principles картки. 🔴 forward-only міграції; RLS ENABLE+FORCE; integer-гроші; idempotency у PG. Зміна контракту, що ламає клієнтів → /council. Training-mode на гейтах. Онови loops/memory/backend-contract-convergence.md.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
