---
description: Петля відновлення інциденту — спершу стабілізувати ЗВОРОТНИМИ діями (fallback/flag/scaling-gate, нуль втрати замовлень), тоді корінь, постійний фікс, детекція, post-mortem.
argument-hint: <що падає зараз>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю incident-recovery (loops/incident-recovery.yaml) для: «$ARGUMENTS».
🔴 ФАЗА 1 — СТАБІЛІЗУЙ зворотними діями (fallback Етапу 33 / feature-flag / scaling-gate). Нуль втрати замовлень, нуль деструктиву, нуль автобану — людина авторитет. STOP-STABILIZED: кровотеча спинена, замовлення цілі. ФАЗА 2 — тоді корінь і ПОСТІЙНИЙ фікс; якщо він на серйозній поверхні — через /council ПІСЛЯ стабілізації. Додай детекцію/алерт, щоб ловилось наступного разу. Запиши post-mortem у docs/incidents/. Онови loops/memory/incident-recovery.md.

## Телеметрія (обов'язково — фініш БУДЬ-ЯКОГО результату)
На success/stall/abort виклич finalize харнесу за інструкцією `harness:` вузла картки loops/<id>.yaml
(tools/loop-harness §5 LOOP REPORT → loops/runs/metrics.jsonl). Пропуск finalize = незавершений запуск
(P2 2026-07-02: 6 телеметрія-рядків на 19 петель — прогони губились).
