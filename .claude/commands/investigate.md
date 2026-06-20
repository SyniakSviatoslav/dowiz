---
description: Діагностична петля для невідомого баґу — repro-first, бісекція гіпотез, корінь (не симптом), фікс + регрес-тест. Не лагодить «заодно».
argument-hint: <опис симптому / звіт>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю investigation-triage (loops/investigation-triage.yaml) для: «$ARGUMENTS».
🔴 Спершу детермінований репро (STOP-REPRO) — без падаючого репро НЕ лагодь. Тоді доведи корінь доказом (STOP-ROOT), не здогадом. Фікс мінімальний; ОБОВ'ЯЗКОВО регрес-тест, що RED на старому коді й GREEN на новому. Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; training-mode на гейтах; онови loops/memory/investigation-triage.md.
