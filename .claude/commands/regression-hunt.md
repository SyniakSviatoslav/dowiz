---
description: Петля полювання за регресією — git-bisect до одного комміту-винуватця, мінімальний фікс або чистий ревет, регрес-тест. Не рефакторить заодно.
argument-hint: <що зламалось; за можливості — last-known-good>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю regression-hunt (loops/regression-hunt.yaml) для: «$ARGUMENTS».
🔴 Спершу детермінований репро, тоді git-bisect до ОДНОГО комміту (STOP-CULPRIT). Ревет — лише якщо не ламає причину, заради якої робилась зміна (STOP-FIX). ОБОВ'ЯЗКОВО регрес-тест: RED на винуватці, GREEN після. Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; training-mode на гейтах; онови loops/memory/regression-hunt.md.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
