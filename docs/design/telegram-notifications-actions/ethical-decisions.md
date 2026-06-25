# Ethical Decisions — telegram-notifications-actions

> Записані людські рішення на STOP-гейтах Триадної Ради. Counsel не перекриває свідому людину; рішення тут — авторитетні.

---

## STOP-ETHICS-1 · класифікація `order.pending_aging`

- **Дата:** 2026-06-22
- **Власник рішення:** SyniakSviatoslav (Product+Arch)
- **Контекст:** Інваріант дизайну — «категорія = зворотність наслідку, не гучність сповіщення». `order.pending_aging` (замовлення застаряло, ще НЕ скасоване) — єдиний пограничний рядок: історично per-event toggle-able, новий статус transactional відбирає toggle. Counsel (Раунд 1–3) підняв це як умовний ETHICAL-STOP: held-доставка перетворила б нічний алерт на «некролог уранці».

- **РІШЕННЯ: `transactional` (пробиває тишу).**
- **Обґрунтування:** Зберігає поточну гарантію (`quietHours:'always'` у `event-registry.ts`), що сьогодні рятує замовлення. Aging — предвісник незворотного `timeout_cancelled`; оператор має бачити старіюче замовлення вночі, поки ще може діяти. Подвійний нічний дзвінок (aging + окремий timeout) прийнятний проти ризику тихої втрати замовлення. Узгоджується з fail-safe (усе незворотне-у-вікні-тиші лишається transactional).
- **Наслідок для impl:** `order.pending_aging` лишається non-mutable transactional (поза `prefs[category]`), не читає toggle, пробиває quiet-hours. Backfill BR-9 не застосовується до нього.

---

## STOP-DESIGN-B · code gate

- **Дата:** 2026-06-22
- **Власник:** SyniakSviatoslav
- **РІШЕННЯ: GO** — блок коду знято для `telegram-notifications-actions`.
- **Умови (перенесені в DoD / enable-gate, НЕ блокують старт кодування):**
  1. Staging-проба BYPASSRLS-доступності на не-superuser Supabase-ролі = ПЕРШИЙ деплой-крок (визначає role/policy-гілку; R-BR24).
  2. Enable-gate ordering: web `app.user_id`-setter live ДО FORCE-міграції; retention-cron live ДО `TG_CATEGORY_GATING=on`.
  3. DoD-тести (FORCE-під-worker обидві гілки; store.close deny-vs-noop; concurrent prefs; strict-secret; guard-on-rowcount) = умова вмикання прапорців `TG_CATEGORY_GATING` / `TG_STOREFRONT_ACTION` (default off).
- **Нагадування:** після відвантаження зміни — очистити маркер (`: > .claude/state/serious-cleared`), щоб озброїти serious-gate знову.
