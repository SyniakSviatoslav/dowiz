---
description: REUSE-вхід канонічної error-fix-петлі (UI↔сервер convergence на живому Playwright). Фаза A (матриця+харнес) → Фаза B (петля до повної зелені 3× поспіль). Сервер read-only, нуль fake-green.
argument-hint: <опц. скоуп: роль/слаг/брейкпоінти; інакше — повна матриця>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---

Запусти error-fix-convergence-петлю. Скоуп: «$ARGUMENTS» (порожньо → повна матриця).
ПРОЧИТАЙ і виконуй картку петлі `loops/error-fix-convergence.yaml` (джерело істини механіки) + `loops/memory/error-fix-convergence.md` (уроки минулих прогонів) і контракти.
ФАЗА A: побудуй MATRIX.md (усі RED) + Playwright-харнес (headed, 390/768/1280, trace/video/screenshot, retries:0) + seed детермінованого стану. STOP-чекпойнт A.
ФАЗА B (петля): RUN → DIAGNOSE (корінь по trace/network/console/DOM) → FIX (мінімальна коректна зміна фронту / рефактор до єдиного джерела) → RE-VERIFY → REPEAT, доки red/flaky=0.
🔴 Сервер read-only (контрактна прогалина → MISSING/BLOCKED-contract); тест=специфікація, не дзеркало коду; нуль fake-green; нуль flaky.
ВИХІД: MATRIX 100% GREEN × 3 поспіль + X1–X11 + артефакти. У training-mode — пауза на STOP-A і STOP-B. Онови loops/memory/error-fix-convergence.md уроками прогону.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
