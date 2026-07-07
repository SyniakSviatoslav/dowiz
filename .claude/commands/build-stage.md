---
description: Білд-петля одного етапу роадмапу — реалізувати рівно скоуп Stage N до його ✅ GATE-чекпойнта. Дисципліна скоупу, forward-only міграції, RLS FORCE, нуль feature-creep.
argument-hint: <номер етапу N (+ опц. посилання на build-prompt)>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю build-stage (loops/build-stage.yaml) для Stage «$ARGUMENTS».
🔴 STOP-SCOPE: підтверди скоуп етапу + deps ПЕРЕД кодом (читай build-prompt етапу + Execution-Runbook + Context-Handoff). Реалізуй РІВНО скоуп — «схема багата, рантайм мінімальний», нуль feature-creep, відкладене не вмикай. forward-only міграції, RLS ENABLE+FORCE, integer-гроші. STOP-CHECKPOINT (GATE): кожен пункт ✅ Чекпойнта зелений; migrate:up/verify:db/verify:rls зелені; крос-tenant=0.
ПРИМІТКА про gate require-classification: етап роадмапу = вже затверджений дизайн (build-prompt + ADR — план-істина), тож на STOP-SCOPE людина знімає блок require-classification, дописавши CHANGE-MANIFEST/reflection. НОВА серйозна архітектурна потреба, якої нема в спеці етапу → спершу ескалюй людині. Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; training-mode на гейтах; онови loops/memory/build-stage.md.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
