---
description: REUSE-вхід канонічної error-fix-петлі (UI↔сервер convergence на живому Playwright). Фаза A (матриця+харнес) → Фаза B (петля до повної зелені 3× поспіль). Сервер read-only, нуль fake-green.
argument-hint: <опц. скоуп: роль/слаг/брейкпоінти; інакше — повна матриця>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---

Запусти error-fix-convergence-петлю. Скоуп: «$ARGUMENTS» (порожньо → повна матриця).
ПРОЧИТАЙ і виконуй промпт `DeliveryOS-Convergence-Playwright-Loop-Prompt.md` (це джерело істини механіки) + прикріплений інвентар `deliveryos_v2_pages_components.html` і контракти.
ФАЗА A: побудуй MATRIX.md (усі RED) + Playwright-харнес (headed, 390/768/1280, trace/video/screenshot, retries:0) + seed детермінованого стану. STOP-чекпойнт A.
ФАЗА B (петля): RUN → DIAGNOSE (корінь по trace/network/console/DOM) → FIX (мінімальна коректна зміна фронту / рефактор до єдиного джерела) → RE-VERIFY → REPEAT, доки red/flaky=0.
🔴 Сервер read-only (контрактна прогалина → MISSING/BLOCKED-contract); тест=специфікація, не дзеркало коду; нуль fake-green; нуль flaky.
ВИХІД: MATRIX 100% GREEN × 3 поспіль + X1–X11 + артефакти. У training-mode — пауза на STOP-A і STOP-B. Онови loops/memory/error-fix-convergence.md уроками прогону.
