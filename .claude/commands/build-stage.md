---
description: Білд-петля одного етапу роадмапу — реалізувати рівно скоуп Stage N до його ✅ GATE-чекпойнта. Дисципліна скоупу, forward-only міграції, RLS FORCE, нуль feature-creep.
argument-hint: <номер етапу N (+ опц. посилання на build-prompt)>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю build-stage (loops/build-stage.yaml) для Stage «$ARGUMENTS».
🔴 STOP-SCOPE: підтверди скоуп етапу + deps ПЕРЕД кодом (читай build-prompt етапу + Execution-Runbook + Context-Handoff). Реалізуй РІВНО скоуп — «схема багата, рантайм мінімальний», нуль feature-creep, відкладене не вмикай. forward-only міграції, RLS ENABLE+FORCE, integer-гроші. STOP-CHECKPOINT (GATE): кожен пункт ✅ Чекпойнта зелений; migrate:up/verify:db/verify:rls зелені; крос-tenant=0.
ПРИМІТКА про gate require-classification: етап роадмапу = вже затверджений дизайн (build-prompt + ADR — план-істина), тож на STOP-SCOPE людина може зняти блок (`echo "stage-N" >> .claude/state/serious-cleared`). НОВА серйозна архітектурна потреба, якої нема в спеці етапу → спершу /council. Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; training-mode на гейтах; онови loops/memory/build-stage.md.
