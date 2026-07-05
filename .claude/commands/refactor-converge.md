---
description: Рефактор-петля — звести дублі/прямі fetch/WS/хардкод до єдиного джерела, БЕЗ зміни поведінки; повний прогін флоу має лишатися зеленим.
argument-hint: <опц. скоуп: модуль/компонент>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю refactor-convergence (loops/refactor-convergence.yaml). Скоуп: «$ARGUMENTS».
🔴 Поведінка незмінна (ті самі виходи). STOP-MAP: карта дублів + план зведення. Після рефактора — ПОВНИЙ прогін флоу (не лише зачеплені), STOP-FULLRUN. grep-чистота: 0 hex у packages/ui, 0 прямих fetch/new WebSocket повз shared, 0 локальних перерахунків ціни. Нуль нових фіч/контрактних змін заодно. Скелет SENSE→DIAGNOSE→ACT→VERIFY→REPEAT; онови loops/memory/refactor-convergence.md.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
