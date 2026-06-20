---
description: Серверна convergence-петля — звести хендлери/Zod/міграції у відповідність контракту й інваріантам (RLS FORCE, integer, idempotency, RS256). Сервер тут — ціль (на відміну від error-fix, де read-only).
argument-hint: <опц. скоуп: домен/ендпоінт>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю backend-contract-convergence (loops/backend-contract-convergence.yaml). Скоуп: «$ARGUMENTS».
Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; дотримуй гейти (STOP-CONTRACT-MAP, STOP-MIGRATION) й iron principles картки. 🔴 forward-only міграції; RLS ENABLE+FORCE; integer-гроші; idempotency у PG. Зміна контракту, що ламає клієнтів → /council. Training-mode на гейтах. Онови loops/memory/backend-contract-convergence.md.
