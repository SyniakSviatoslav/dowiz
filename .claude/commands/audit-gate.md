---
description: Петля гейту якості (Frontend-Audit-Polish-Gate) — секції A–F наживо в браузері, PASS лише з артефактом; косметику фіксиш inline, логіку/контракти/безпеку — flag-only.
argument-hint: <опц. скоуп: екран/роль/секція>
allowed-tools: Read, Write, Edit, Glob, Grep, Bash
---
Запусти петлю audit-gate (loops/audit-gate.yaml). Скоуп: «$ARGUMENTS». Слідуй `DeliveryOS-Frontend-Audit-Polish-Gate.md`.
🔴 PASS лише з браузерним артефактом (скріншот/запис) — «з читання коду» не зараховується. Кожен екран пройди наживо (console+network): A токени (grep 0 hex у packages/ui) · B уніфікація станів/кнопок/форм · C інтеграції наживо (Zod-парс, WS, idempotency, матриця помилкових кодів) · D polish/адаптив 390/768/1280 · F рідкісні стани відтворені. Косметику/стани/токени — фіксиш inline (перезнімай до/після); логіку/контракт/безпеку — НЕ чіпай, винеси flag-only. STOP-VERDICT: усі секції PASS з артефактами. Онови loops/memory/audit-gate.md.
