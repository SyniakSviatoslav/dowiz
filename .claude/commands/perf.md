---
description: Перф-петля — виміряти базлайн проти бюджету, знайти вузьке місце, точково оптимізувати, додати perf-гейт; нуль функціональної регресії, нуль передчасної оптимізації.
argument-hint: <метрика / перф-симптом>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю performance (loops/performance.yaml) для: «$ARGUMENTS».
🔴 STOP-BASELINE: визнач бюджет і виміряй базлайн ПЕРШ НІЖ щось міняти (вимір до оптимізації). Знайди вузьке місце доказом, не здогадом. STOP-BUDGET: метрика ≤ бюджет, відтворювано (медіана N), функціональні флоу зелені. Додай perf-бюджет/тест як охорону. Архітектурна зміна заради швидкості → /council. Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; онови loops/memory/performance.md.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
