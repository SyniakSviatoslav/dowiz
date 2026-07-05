---
description: Змагальна петля exit-аудиту фази — довести завершеність ЦІЛОЇ фази перед переходом. Не лагодить (діри → назад у build-stage/error-fix), лише доводить/спростовує.
argument-hint: <номер/назва фази>
allowed-tools: Read, Write, Glob, Grep, Bash
---
Запусти петлю exit-audit (loops/exit-audit.yaml) для фази «$ARGUMENTS». Слідуй відповідному фазовому exit-audit-промпту.
🔴 Змагальна постановка: припускай, що фаза НЕ готова — спробуй зламати твердження про завершення. STOP-COVERAGE: усі етапи фази покрито аудитом. Доведи ДОКАЗОМ (не заявою): кожен чекпойнт етапу зелений; наскрізні червоні лінії фази тримаються (напр. Фаза 2 — модифікатори/i18n/гроші-розбивка; Фаза 5 — anonymizer/observability/backup-restore/fallback/hardening/go-live + реальний платний заказ); критичний шлях трьох ролей E2E зелений; крос-tenant=0; нуль секретів. 🔴 НЕ лагодь тут — діри віддай у build-stage/error-fix. STOP-VERDICT: PASS/FAIL з доказами. Онови loops/memory/exit-audit.md.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
