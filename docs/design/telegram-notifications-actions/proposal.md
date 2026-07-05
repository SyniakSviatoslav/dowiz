# Design Proposal — DeliveryOS: Розширені Telegram-сповіщення + дії

**Status:** DRAFT (design-time, no production code) · **РАУНД 3 структурне рішення вписане (role/pool + policy-GUC канон)**
**Author:** System Architect (DeliveryOS)
**Date:** 2026-06-22
**Предтеча (must align):** `docs/adr/ADR-NOTIFICATION-CONSOLIDATION.md`
**ADR draft:** `docs/adr/ADR-TELEGRAM-NOTIFICATIONS-ACTIONS.md`

---

## 0. Розвідка реального коду (anchors — verified, не вигадано)

| Концепт | Реальний файл / факт |
|---|---|
| Модель targets | `owner_notification_targets` — мігр. `packages/db/migrations/1780348982032_owner_notification_targets.ts`. Колонки: `id, location_id, channel('telegram'\|'push'), address, status('pending'\|'active'\|'disabled'\|'disconnected'), prefs jsonb DEFAULT '{"order.created":true,"order.pending_aging":true}', created_at, last_error, disabled_at`. RLS **ENABLE** (не FORCE) + policy `owner_notification_targets_owner_all`. `user_id` додано пізніше (`...004`), `locale` (`...005`). Унікальний індекс `(location_id, channel, address)`. |
| Telegram inbound webhook | `apps/api/src/routes/telegram-webhook.ts`. Шлях `POST /webhook/telegram/:secret`. Secret валідується через header `x-telegram-bot-api-secret-token` (з backward-compat lenient-режимом якщо header відсутній). Завжди відповідає `200` (best-effort, off critical-path). |
| Authority (callback) | Для `order.*`: order→`location_id`→`owner_notification_targets(address=chatId, channel='telegram', status='active', location_id)`→якщо `user_id` set → перевірка `memberships(user_id, location_id, status='active')`. **Authority береться з chat_id↔membership, НЕ з callback_data** — це вже інваріант у коді. |
| Accept/reject кнопки | `order.confirm`, `order.reject_choose`, `order.reject_reason_*` — через `updateOrderStatus(client, …)` (canonical guarded path) під `set_config('app.current_tenant', …)`. **Контракт НЕ чіпаємо.** |
| Inbound shift action (взірець для нової дії) | `/open` команда → `openShift(client, user_id, locationId, {messageBus})` під `set_config('app.current_tenant')`. Це готовий взірець guarded-UPDATE + realtime fan-out через messageBus. |
| Storefront online/offline | Колонка `locations.delivery_paused boolean NOT NULL DEFAULT false` (мігр. `...023`). Інверсія: `delivery_paused=false` ⇒ "приймаємо замовлення". Web write: `PUT /api/owner/settings` (`apps/api/src/routes/spa-proxy.ts:646`, guarded `getLocationId(request)` + `COALESCE($13, delivery_paused)`). Public read: `apps/api/src/routes/public/menu.ts:63` `isOpen = !delivery_paused`. Web UI toggle: `apps/web/src/pages/admin/SettingsPage.tsx:619`. |
| Dispatcher | `apps/api/src/notifications/workers/index.ts` — `NotificationWorker.handleTelegramSend()` (fan-out на всі active targets) + `handleDispatch()` (single-target). Уже робить: `prefs[event]===false`→audit `prefs_disabled`; quiet-hours через `isEventAllowedDuringQuietHours()`; per-chat rate-limit + circuit breaker; in-memory dedup. |
| Quiet-hours політика | `apps/api/src/notifications/event-registry.ts` — `EVENT_REGISTRY[event].quietHours: 'always'\|'during_business'\|'never'`. **Hardcoded UTC 22:00–08:00** у воркері (`index.ts:219`), не per-location. |
| Audit | Таблиця `notification_outbox_audit` (мігр. `...007`). **CHECK status IN ('queued','sending','delivered','failed','archived')**. `writeAudit()` (`apps/api/src/notifications/audit.ts`) пише статуси `'quiet_hours'`, `'prefs_disabled'`, `'no_target'`, `'dedup'`, `'circuit_open'`, `'rate_limited'`, `'target_inactive'`, `'order_not_found'`. **Жоден з них не в CHECK** — `writeAudit` падає на CHECK-violation. Таблиця **без RLS**. |
| Web preference-центр | `apps/web/src/pages/admin/SettingsPage.tsx` вже читає targets (`/owner/locations/:id/notifications/targets`) і PUT prefs (рядок 238). Backend `apps/api/src/routes/owner/notifications.ts` `PUT …/targets/:targetId` приймає `prefs: z.record(z.boolean())` (per-event boolean). |
| Consent-log | **Немає** виділеної таблиці GDPR-consent. Найближче — `access_requests` (`...041`). |
| **RLS-ланцюг (РАУНД 2 verified)** | **ДВА різних GUC, НЕ взаємозамінні.** Канонічна tenant-RLS на `orders`/`locations`/`couriers`/`order_items` гейтиться через **`app.user_id`**: `orders` policy (`1780310074262_orders.ts:82-84`, FORCE) → `app_member_location_ids()` (`core-identity.ts:76`, SECURITY DEFINER) → `app_current_user()` (`:70`) → `current_setting('app.user_id', true)`. `app.current_tenant` читається прямо ЛИШЕ courier/settlement-таблицями. → **РАУНД 3 рішення:** 4 НОВІ таблиці гейтяться на ТОЙ САМИЙ канон `app.user_id` (через `app_member_location_ids()`), НЕ на окремий GUC і НЕ на `request.jwt.claim.sub` (0 setters). Worker-escape = роль `deliveryos_notif_worker` BYPASSRLS, не self-settable GUC (BR-20/21). Раунд-1/2 «окремий worker-GUC `app.notif_worker`» ВІДКИНУТО як self-grantable backdoor. |
| **Транзакційний патерн (РАУНД 2 verified)** | Web/courier/cron guarded-шляхи РОБЛЯТЬ `BEGIN` перед transaction-local GUC (`courier/shifts.ts:23→24`, `spa-proxy.ts:420→421`, `orders.ts:105`, всі cron). **Виняток — `telegram-webhook.ts` order-handler** (`:101` connect, `:227/294/497` `set_config(...,true)` БЕЗ `BEGIN`, + не ставить `app.user_id`) → pre-existing латентний баг (R-PRE, §10). Воркер (`workers/index.ts`) — autocommit. → новий код МУСИТЬ обгортати в `BEGIN…COMMIT` (BR-13). |

### Два пре-існуючі баги, які цей дизайн ОБОВ'ЯЗКОВО лагодить (інакше "нуль тихих дропів" — фікція)

- **BUG-A (тихий drop аудиту):** `notification_outbox_audit` CHECK не містить gating-статусів. Кожен `writeAudit({status:'prefs_disabled'|'quiet_hours'|…})` кидає `23514 check_violation`. У `handleTelegramSend` він обгорнутий per-target try/catch → проковтується. Тобто **зараз кожен pref-off / quiet-hours no-op губиться без сліду**. Прямо суперечить вимозі "нуль тихих дропів". → міграція розширює CHECK.
- **BUG-B (RLS gap):** `owner_notification_targets` має `ENABLE` без `FORCE`; `notification_outbox_audit` без RLS взагалі. Червона лінія DeliveryOS = `ENABLE + FORCE` на кожній tenant-таблиці. → міграція додає FORCE + audit RLS.

---

## 1. Проблема + Non-goals

**Проблема.** Сповіщення керуються per-event boolean у `prefs`, розкиданих по двох воркер-шляхах; quiet-hours hardcoded глобально UTC; gating-no-op'и не аудитяться (BUG-A); немає per-category UX ні в web, ні в Telegram; немає inbound-дії "відкрити/закрити storefront"; consent на зміну prefs не логується. Оператору неможливо відповісти "чому сповіщення не прийшло".

**Мета.** (1) 3 категорії (🔴 транзакційні non-mutable · 🟠 операційні default ON · 🟡 якість default OFF) з одним source-of-truth `prefs[category]` + `quiet_hours{from,to}`, керовані з web preference-центру і Telegram `/settings`. (2) Нова inbound-дія storefront open/close тим самим guarded-UPDATE шляхом що web. (3) Категорійний gating у диспетчері з повним audit-слідом (нуль тихих дропів).

**Non-goals (поза скоупом, явно).** Per-channel вибір на категорію; зміна кур'єра через Telegram; тренд-аналітика / складний дайджест; промо-категорія; push/email канали (модель готова тим самим рядком, рантайм НЕ вмикаємо — "схема багата, рантайм мінімальний").

---

## 2. Back-of-envelope

**Параметри (go-live горизонт, з MEMORY: single-tenant pilot → ранній multi-tenant).**

- Локацій: 1 → 50 (план рік-1: ≤ 200).
- Membership/targets на локацію: 1–3 Telegram-чатів (власник + 1–2 менеджери). → 50 лок × 3 = **150 active targets**.
- Замовлень/день на активну локацію: 30–150 (пік 200). Беремо 100 avg.
- Telegram-події на замовлення: `order.created` (1) + `confirmed/rejected` (1) + lifecycle (`ready_for_pickup`, `delivered`, можливо `dwell_escalation`) ≈ 2 → **~4 події/замовлення**.

**Об'єм Telegram outbound API:**
- 50 лок × 100 ord × 4 ev × ~2 targets fan-out = **40 000 sendMessage/день** ≈ 0.46 msg/s avg, пік ~5 msg/s.
- Telegram ліміт: 30 msg/s глобально на бота, ~1 msg/s на чат. Per-chat rate-limit (1200ms) у воркері вже покриває. Глобально — комфортно (5 ≪ 30). **Не потребує шардингу бота** на цьому горизонті.

**Inbound callback (дії):**
- confirm/reject ≈ 1–2 тапи/замовлення + storefront toggle ≈ 2–6 тапів/локацію/день + `/settings` toggles ≈ рідко.
- 50 × 100 × 1.5 + 50×6 = **~7 800 inbound/день** ≈ 0.09 req/s. Тривіально.

**Audit-лог:**
- Кожна подія лишає ≥1 audit-рядок (delivered/failed/prefs_off/quiet_hours/dedup). Fan-out × no-op'и ≈ 1.3× outbound. ~40k×1.3 = **~52 000 audit-рядків/день**.
- Рядок ~120 B → **~6 MB/день**, ~190 MB/міс, ~2.3 GB/рік на 50 лок. → потрібен **retention (90 днів) + partial index**, інакше таблиця стане hotspot для "чому не прийшло"-запитів. Партиціювання НЕ потрібне на цьому масштабі (схема готова: `created_at` index → range-drop).

**Dispatcher навантаження.**
- pg-boss воркери, ~0.5 job/s avg. Кожен job: 1 SELECT targets + N×(prefs-check у пам'яті + 1 dispatch + 1–2 audit INSERT). I/O-bound на Telegram fetch, не CPU. Монoліт-first (ADR 001) тримає — **жодного розпилу.**

**Quiet-hours черга (вартість притримання).**
- Якщо тримати відкладені 🟠/🟡 повідомлення: за нічне вікно (22:00–08:00, 10 год) на локацію ~ 🟠-подій = операційні non-critical. При 100 ord/день більшість 🔴 (транзакційні, шлються завжди). 🟠 нічних ≈ 5–15/локацію/ніч. 50 лок × 15 = **~750 held-рядків/ніч максимум**. → черга крихітна; pg-boss `startAfter` (delayed job) тримає її в БД без окремого сховища. **Не потрібен Redis/окрема черга.**

**Бюджет конектів (API + worker + analytics + migrations сукупно) — РАУНД 3 перерахунок.** Раунд-1 заявляв «0 нових конектів» — **це більше НЕ так після структурного BR-20 fix.** Notif-worker переноситься на **окремий dedicated пул** під роллю `deliveryos_notif_worker` (BR-20 — escape прив'язаний до ролі, не до self-settable GUC). Перерахунок постійних: operational `max=8` (`deliveryos_api_user`, web/webhook) + session `max=3` (messageBus LISTEN/NOTIFY, `server.ts:262`) + **notif-worker `max=2`** (НОВИЙ, `deliveryos_notif_worker`) + migrations transient = **~13 постійних + transient**. Supabase Supavisor (6543/5432) тримає сотні → 13 комфортно. **Нова залежність (роль+пул+env `DATABASE_URL_NOTIF_WORKER`) — на STOP-DESIGN-B** (§10 R-ROLE/R-POOL). Інфра-шов наявний: `createSessionPool()` (`db/index.ts:45`) вже доводить 2-пуловий патерн.

---

## 3. Архітектурні розвилки (≥2 опції кожна, named concept, tradeoffs, вибір)

### Розвилка A — Де живе категорія→event мапа й gating

**A1 — Inline у кожному call-site (Scattered guard).** Кожен producer перевіряє категорію/prefs перед enqueue.
- (+) нуль зайвої роботи у воркері.
- (−) дублювання логіки по ~10 producer-ах; розсинхрон неминучий; audit-слід розкиданий; саме так BUG-A і виник. **Anti-pattern (premature distribution of policy).**

**A2 — Централізований dispatcher-шар (Policy Gateway).** Єдина серверна мапа `EVENT→CATEGORY` поряд з наявним `EVENT_REGISTRY`; весь gating (category-pref, quiet-hours, hold) у `handleTelegramSend`/`handleDispatch` — там, де gating вже частково живе.
- (+) один source-of-truth; кожен no-op аудититься в одній точці; рантаж-мінімальний — розширюємо наявний воркер, не плодимо шари.
- (−) воркер трохи товщає (прийнятно: ~30 рядків).

**ВИБІР: A2 (Policy Gateway), розширення наявного `EVENT_REGISTRY`.** Концепт: **single-writer policy gateway**. Додаємо поле `category: 'transactional'|'operational'|'quality'` у кожен `EVENT_REGISTRY` запис (категорія→event = інверсія цієї мапи, обчислюється на сервері). Gating-порядок у воркері:
1. target.status≠active → audit `target_inactive`.
2. `category==='transactional'` → шлемо ЗАВЖДИ (ігнор toggle+quiet).
3. `prefs[category]===false` (operational default true, quality default false) → audit `prefs_disabled` (BR-2 канон — НЕ `pref_off`), skip.
4. quiet-hours активні (per-location `quiet_hours{from,to}`) ∧ non-transactional → **hold** (delayed job) + audit `quiet_hours`.
5. інакше → dispatch → audit `delivered`/`failed`.

> Узгодження з предтечею: ADR-NOTIFICATION-CONSOLIDATION закріпила **єдиний canonical status-change path** і **єдиний dwell-producer**. A2 продовжує цю лінію — єдиний gating-вузол, а не розсип.

#### ⚖️ ETHICAL-інваріант категоризації (RESOLVE Counsel ETHICAL-STOP — умовний)

> **Інваріант (записаний у дизайні, знімає УМОВУ STOP'а):** категорія = **зворотність наслідку**, НЕ гучність сповіщення. Будь-яка подія, наслідок якої незворотний у вікні тиші (втрата замовлення, грошова розбіжність, скарга на безпеку доставки), КЛАСИФІКУЄТЬСЯ `transactional` — пробиває тишу, ніколи не held, НЕ в `prefs` — незалежно від «операційного» тону.

**Канонічна мапа non-mutable `transactional`** (live-verified: усі наразі `quietHours:'always'` у `event-registry.ts:17-149`, тобто інваріант = ЗБЕРЕЖЕННЯ поточної гарантії, не нова поведінка):

| transactional (пробиває тишу, не в prefs) | підстава незворотності |
|---|---|
| `order.created`, `order.confirmed`, `order.rejected`, `order.delivered` | стан-перехід наслідку / вікно прийому |
| `order.timeout_cancelled` | замовлення вже мертве — held доставив би некролог |
| `order.substitution_needs_human` | блокує fulfillment до людини |
| `order.dwell_escalation`, `order.pending_aging`, `order.ready_for_pickup` | вікно на дію закривається (aging→auto-cancel) |
| `cash.reconcile_discrepancy` | фінансова незворотність |
| `delivery.flag_raised` | скарга на безпеку доставки |
| `courier.assigned` | координація доставки real-time |
| `ops.worker_liveness`, `ops.backup_failed`, `ops.degradation_changed` | системний збій |
| `shift.started`, `shift.closed` | координація зміни |

**`quality` (default OFF, held у тиші — наслідок зворотний):** `rating.low_received`, `shift.close_reminder`.
**`operational` (default ON, held-дозволено):** наразі **порожня** для order-flow — fail-safe: за замовчуванням НІЩО незворотне не held.

**STOP-ETHICS-1 (записаний людський акт перед prod):** Product+Arch письмово підтверджують цю мапу в ADR перед `TG_CATEGORY_GATING=on` — особливо `order.pending_aging` (єдиний реальний пограничний: aging→timeout). Якщо людина свідомо вирішить «pending_aging — operational/held», це її право, але **записане**, не випале мовчки. Дизайн-інваріант + порожній operational-order-flow = навіть якщо акт забудуть, fail-safe = все transactional.

### Розвилка B — Quiet-hours притримання: drop vs delayed-queue

**B1 — Drop + audit (Silent-suppress).** non-critical у quiet-вікно просто не шлеться, лишається audit `quiet_hours`.
- (+) нуль інфраструктури; дешево.
- (−) оператор втрачає 🟠-контекст назавжди; "тихо пропустити" суперечить духу "оператор має бачити, що сталось вночі".

**B2 — Delayed-queue через pg-boss `startAfter` (Deferred-deliver).** non-critical у quiet-вікно → re-enqueue з `startAfter = next_window_open`. Прокидається після quiet-end. Опційно — згорнути в один дайджест-меседж.
- (+) нуль drop; повідомлення долітає вранці; черга живе в БД (pg-boss), back-of-envelope ~750 рядків/ніч — дешево; **узгоджено з транзакційним outbox-патерном репо.**
- (−) трохи більше job-traffic; треба guard від re-hold нескінченного (cap attempts).

**ВИБІР: B2 (Deferred-deliver) для MVP — простий re-enqueue, БЕЗ digest-згортки (digest = non-goal).** Концепт: **transactional deferred delivery**. Кожен held — audit `held` (видимий слід) + re-enqueue з `startAfter`. 🔴 ніколи не тримаються. Hold-storm guard: max 1 re-hold; якщо й після цього quiet — deliver anyway. Тримати held у pg-boss, не в окремому сховищі (YAGNI — ponytail).

> **RESOLVE BR-6 (HIGH) — quiet-window math (специфіковано, з wraparound + local-TZ):** `quiet_from_min/to_min` — local minutes-of-day у `locations.timezone` (колонка додана §5.2). `getUTCHours()`-хардкод (`index.ts:219`) **видаляється**.
> ```
> tz = locations.timezone ?? 'Europe/Tirane'   -- BR-14 code-fallback; NULL → audit 'quiet_tz_fallback'
> nowLocalMin = minutes-of-day(now AT TIME ZONE tz)
> isQuiet = (from <= to) ? (from <= nowLocalMin AND nowLocalMin < to)        -- денне вікно
>                        : (nowLocalMin >= from OR nowLocalMin < to)          -- wraparound 22:00→08:00
> ```
> CHECK проти `pg_timezone_names` (BR-14) гарантує, що `AT TIME ZONE tz` ніколи не кине на committed-рядку.
> `startAfter` (window-open) = найближчий локальний момент `nowLocalMin == to`, конвертований назад у UTC. Це знімає BR-6 п.1 (наївне `from<to` робило 22:00–08:00 порожньою множиною) і п.2 (TZ-зсув). Edge на межі вікна (BR-6 п.3): max-1-rehold + «deliver anyway» застосовується ЛИШЕ до зворотних подій — незворотні вже `transactional` (ETHICAL-інваріант нижче) і ніколи не held, тож «deliver anyway вночі» не порушує «не турбувати незворотним некрологом».

### Розвилка C — Storefront-toggle authority + idempotency

**C1 — Read-then-write (Check-and-set).** `SELECT delivery_paused` → if differs → `UPDATE`.
- (−) гонка між read і write (web↔telegram одночасно); подвійний тап між SELECT і UPDATE = два UPDATE; **не атомарно.**

**C2 — Conditional guarded UPDATE (Idempotent compare-set + RLS-visibility розрізнення).**

> **RESOLVE BR-23 (HIGH, РАУНД 4) — дві склеєні проблеми store-toggle на FORCE-таблиці `locations`:**
> **(1) Неправильний GUC:** `locations` — FORCE RLS **вже сьогодні** (`core-identity.ts:85-87`), policy `id IN (SELECT app_member_location_ids())` читає **`app.user_id`** (`:72`), НЕ `app.current_tenant`. Раунд-1 §C2 ставив `app.current_tenant` → RLS-deny на КОЖНОМУ store.close/open. **Fix: store-toggle обгортка ставить канон `app.user_id` = `targetUserId`** (вже-верифікований active-member з webhook non-order resolver `telegram-webhook.ts:160-194`, той самий патерн що BR-22 order-шлях). NULL-user store-mutating → **reject** (інакше `app.user_id` порожній → policy deny → fail-silent).
> **(2) 0-rows двозначність (суть):** навіть з правильним `app.user_id`, `rowCount=0` має ДВА нерозрізненні джерела: (a) legit idempotent-noop (значення вже `$new`, рядок видимий) і (b) RLS-permission-deny (рядок невидимий цьому `app.user_id` — non-member/legacy/disconnected). Раунд-1 трактував 0-rows як success → на deny бот каже «✅ пауза», storefront лишається ВІДКРИТИМ (fail-silent на обіцянці «екстрено стоп»). `GUARDED_NOOP`-throw з BR-22 тут непридатний (для store 0-rows ЛЕГІТИМНО = double-tap). **Fix: `cur FOR UPDATE`-CTE → окремий RLS-visibility сигнал.**

**ОБРАНА SQL (один атомарний statement, FOR UPDATE під FORCE):**
```sql
WITH cur AS (
  SELECT delivery_paused AS was FROM locations WHERE id = $loc FOR UPDATE
)                                  -- FOR UPDATE лочить рядок ЛИШЕ якщо видимий під RLS (app.user_id member)
UPDATE locations l
   SET delivery_paused = $new      -- безумовний set; ідемпотентний (write $new коли вже $new = той самий стан)
  FROM cur
 WHERE l.id = $loc
RETURNING cur.was, l.delivery_paused AS now;
```
**3-way розрізнення + рапорт боту (deny ≠ noop ≠ changed):**
| Результат | Означає | Бот рапортує | Audit |
|---|---|---|---|
| **0 rows** (`cur` порожній під RLS) | **deny / not-found** — рядок невидимий цьому `app.user_id` | **ПОМИЛКА** «не вдалось — закрийте через застосунок» + **fallback на web** | `store.toggle.denied` |
| **1 row, `was = now`** | **idempotent-noop** — рядок видимий, але вже в цільовому стані (double-tap/replay) | «вже на паузі» / «вже відкрито» (м'яко, **НЕ помилка**) | `store.toggle.noop` |
| **1 row, `was ≠ now`** | **changed** — рядок видимий, стан перемкнуто | «✅ приймання на паузі о HH:MM» + realtime publish | `store.toggle.changed` |

- **Чому FOR UPDATE-CTE під FORCE бачить рядок ЛИШЕ якщо authorized:** RLS USING-quals застосовуються до `SELECT … FOR UPDATE` так само, як до звичайного SELECT — невидимий рядок не потрапляє в `cur` → `UPDATE … FROM cur` без джерела → 0 rows. Видимий рядок (member) завжди в `cur` (1 row), `RETURNING cur.was` дає «що було».
- **Доведення інваріантів:** (1) **deny НІКОЛИ не success** — невидимий→`cur` порожній→0 rows→бот помилка+web-fallback (на відміну від раунду-1, де 0-rows=success); (2) **idempotent double-tap НЕ помилка** — видимий→1 row, `was=now`→бот «вже …»; (3) **changed** — 1 row, `was≠now`→«✅ HH:MM».
- (+) атомарно одним statement під одним row-lock (нуль вікна); ідемпотентно; гонка web↔telegram serialізується на `FOR UPDATE`-lock — останній writer виграє, обидва бачать консистентний RETURNING.

**ВИБІР: C2 (compare-set + RLS-visibility розрізнення).** Концепт: **compare-and-set + visibility-aware guard + transactional realtime publish**. Реалізуємо через **спільний сервіс** `setAcceptingOrders(client, locationId, accepting, {messageBus})` за взірцем `openShift` — щоб web (`PUT /owner/settings`) і Telegram inbound викликали ОДИН код (нуль паралельної мутації). Обгортка: `BEGIN; set_config('app.user_id', targetUserId, true); <CTE-UPDATE>; <publish>; COMMIT`. Web PUT мігрує на цей сервіс. Realtime publish — у тій самій транзакції (outbox-стиль). **Узгодження двох guarded-площин:** order (BR-22) і store (BR-23) тепер обидві тримають «0-rows=рядок невидимий=FAIL, ніколи success»; відмінність лише: store має легітимний same-value-noop (винесений в ОКРЕМИЙ сигнал `1 row + was=now`), order не має.

---

## 4. Рішення (ADR-формат) — див. `docs/adr/ADR-TELEGRAM-NOTIFICATIONS-ACTIONS.md`

Стисло: **A2 Policy Gateway + B2 Deferred-deliver + C2 Idempotent compare-set**, плюс фікс BUG-A (audit CHECK) і BUG-B (RLS FORCE). Категорії — нова мапа поверх наявного `EVENT_REGISTRY`; `prefs` мігрує з per-event на per-category (backfill зберігає поведінку); `quiet_hours{from,to}` — нова per-target колонка. Нова дія `store.open`/`store.close` через `setAcceptingOrders`. `/settings` inline-команда у наявному `handleMessage`.

---

## 5. Дані / Міграції (forward-only, атомарні, RLS FORCE, integer)

Усі нові — `packages/db/migrations/1790000000048…050_*.ts` (наступні після `...047`). Forward-only, idempotent (`IF NOT EXISTS`).

### 5.1 `...048_notification-audit-statuses-and-rls.ts` (фікс BUG-A + BUG-B)

> **RESOLVE BR-2 (CRITICAL):** канонічний enum статусів = той, що код **реально пише** — `'prefs_disabled'` (НЕ `'pref_off'`). Live-anchor: `audit.ts:9` і `workers/index.ts:213,338` пишуть `'prefs_disabled'`. CHECK і `AuditStatus`-union узгоджені нижче. Канон: `queued · sending · delivered · failed · archived · prefs_disabled · quiet_hours · held · no_target · dedup · circuit_open · rate_limited · target_inactive · order_not_found · unknown_event`.

> **RESOLVE BR-20/BR-21 (РАУНД 3 — структурне, замінює worker-escape-GUC раундів 1–2):** припущення «worker під service-role» було **ХИБНЕ**, а worker-escape через self-settable GUC `app.notif_worker` — **self-grantable крос-tenant backdoor на спільному пулі** (BR-20 HIGH). Policy на `request.jwt.claim.sub` — **GUC без жодного production-setter** → FORCE→0 rows→outage+тихий prefs data-loss (BR-21 CRITICAL). **СТРУКТУРНЕ рішення:**
> - **policy-GUC = канон `app.user_id`** (`app_member_location_ids()`, рівно як `orders`/`locations` — `core-identity.ts:70-79`). `request.jwt.claim.sub` ВИДАЛЕНО. `app.notif_worker` ВИДАЛЕНО з усіх policy.
> - **worker-escape прив'язаний до РОЛІ:** notif-worker конектиться окремим пулом під `deliveryos_notif_worker` з **verified BYPASSRLS** (FATAL-on-missing, не swallowed-EXCEPTION). Web-роль `deliveryos_api_user` структурно не може обійти RLS (нема self-settable значення).
> - **web prefs-шлях `owner/notifications.ts` мігрує на `set_config('app.user_id', request.user.sub, true)` у `BEGIN…COMMIT` ПЕРЕД FORCE** (інакше read-after-write на prefs мертвий — BR-21).

```sql
-- BUG-A (BR-2): CHECK = рівно ті статуси, що пише код. 'prefs_disabled', НЕ 'pref_off'.
ALTER TABLE notification_outbox_audit DROP CONSTRAINT IF EXISTS notification_outbox_audit_status_check;
ALTER TABLE notification_outbox_audit ADD CONSTRAINT notification_outbox_audit_status_check
  CHECK (status IN (
    'queued','sending','delivered','failed','archived',
    'prefs_disabled','quiet_hours','no_target','dedup','circuit_open',
    'rate_limited','target_inactive','order_not_found','unknown_event','held','quiet_tz_fallback'));
-- BUG-B: RLS на audit. Policy-GUC = КАНОН app.user_id (BR-21), НЕ request.jwt.claim.sub.
ALTER TABLE notification_outbox_audit ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification_outbox_audit FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS noa_tenant ON notification_outbox_audit;
CREATE POLICY noa_tenant ON notification_outbox_audit FOR ALL TO authenticated
  USING ( location_id IN (SELECT app_member_location_ids()) );   -- канон; БЕЗ notif_worker-GUC (BR-20: escape = роль)
-- retention helper index (90d retention via cron DELETE)
CREATE INDEX IF NOT EXISTS noa_created_at_idx ON notification_outbox_audit(created_at);
```
> **Worker-escape механізм (РАУНД 3 — РОЛЬ, не GUC):** notif-worker отримує **окремий пул** `createNotifWorkerPool()` (`db/index.ts`, взірець наявного `createSessionPool` port 5432) під роллю `deliveryos_notif_worker` з **verified BYPASSRLS**. Воркер легітимно поза RLS (атрибут ролі) → FORCE пропускає його SELECT targets / writeAudit БЕЗ жодного GUC. **Чому це усуває BR-20:** `app.notif_worker` більше не в policy → web `set_config('app.notif_worker','on')` = no-op для RLS; BYPASSRLS має ЛИШЕ окрема роль, якою web-код не конектиться (окремий пул, окрема credential). Self-grantable bypass усунутий **структурно**, не дисципліною. **Чому це усуває BR-13 (раунд-2 транзакційний трюк) чистіше:** немає GUC-escape → немає ні autocommit-outage (роль обходить FORCE без GUC), ні session-leak (немає GUC, що міг би leak'нути). Транзакція потрібна ЛИШЕ для атомарності prefs+audit (BR-16), не для RLS.
> **Verified-grant gate (не swallowed-EXCEPTION як BR-1 раунд-1):** міграція `CREATE ROLE deliveryos_notif_worker … BYPASSRLS` БЕЗ `EXCEPTION WHEN OTHERS` → провал grant = FATAL-міграція (дізнаємось на staging). Pool boot-assert: `SELECT rolbypassrls FROM pg_roles WHERE rolname=current_user` — `false` → FATAL-exit (worker не стартує). Grant **доведений**, не припущений. Якщо Supabase НЕ дає BYPASSRLS не-superuser-ролі → fallback `policy TO deliveryos_notif_worker` (role-based, BYPASSRLS не потрібен) — на STOP-DESIGN-B (R-ROLE).
>
> **`AuditStatus`-union (audit.ts) розширюється** (impl-крок, у скоупі): `+'held' +'queued' +'archived' +'unknown_event' +'quiet_tz_fallback'`.
>
> **ВИПРАВЛЕНИЙ DoD-gate §5.1 (BR-21 — тестує ТОЙ САМИЙ GUC, що policy реально читає, `app.user_id`):**
> 1. Під FORCE, `set_config('app.user_id', <member-of-A>, true)` → `SELECT … targets WHERE location_id=<A>` → **рядки є** (web read-after-write живий).
> 2. Під FORCE, `set_config('app.user_id', <non-member>, true)` → той самий SELECT → **0 rows** (tenant-ізоляція).
> 3. Під FORCE, web `PUT prefs` (member app.user_id) → `jsonb_set` UPDATE → **rowCount=1** + prefs-audit INSERT ok (prefs save живий, BR-16).
> 4. На operational-пулі (web-роль), `set_config('app.notif_worker','on',true); SELECT … targets WHERE location_id=<чужа>` → **0 rows** (BR-20: GUC-escape мертвий, web не може).
> 5. На notif-пулі (`deliveryos_notif_worker`), той самий SELECT БЕЗ GUC → **всі рядки** (BR-20: BYPASSRLS-роль легітимно поза RLS).
> 6. Регрес-guard: `grep "request.jwt.claim.sub"` по нових міграціях → **0**.
> 7. **(BR-23 store-toggle deny-vs-noop) Під FORCE на `locations`, `setAcceptingOrders` через `cur FOR UPDATE`-CTE:**
>    - (a) `app.user_id`=member, `delivery_paused`=false, close → **1 row, `was=false`≠`now=true`** → changed (бот «✅», storefront реально закрито).
>    - (b) той самий close ПОВТОРНО (вже paused) → **1 row, `was=true`=`now=true`** → idempotent-noop (бот «вже на паузі», **НЕ помилка**).
>    - (c) `app.user_id`=**non-member** (або порожній — RLS-deny), close → **`cur` порожній → 0 rows** → `store.toggle.denied` → бот **ПОМИЛКА + web-fallback**, storefront НЕ змінено. **Доказ: deny НІКОЛИ не рапортується як success** (на відміну від раунду-1).
>    - (d) store-mutating з `targetUserId IS NULL` → reject ДО CTE (не ставити порожній `app.user_id`).
>    Без (c) (deny=fail, не success) — `TG_STOREFRONT_ACTION` НЕ вмикається.
> Без 1+3 (web живий під FORCE) — FORCE-міграція НЕ деплоїться. Без 4+5 — gating не вмикається.
>
> **`AuditStatus`-union (audit.ts) розширюється** (impl-крок, у скоупі): `+'held' +'queued' +'archived' +'unknown_event' +'quiet_tz_fallback'` (останній — BR-14 fallback-слід) — інакше нові статуси не компілюються через `writeAudit()` (BR-2 п.2).

### 5.2 `...049_notification-targets-categories.ts` (prefs → category + quiet_hours + timezone + RLS FORCE)

> **RESOLVE BR-6 (HIGH):** `locations.timezone` **НЕ існує** (grep по migrations → 0). Без неї quiet `minutes-of-day` порівнюється з UTC-now → вікно зсунуте на TZ-offset (Албанія UTC+1/+2). **Fix:** додаємо колонку тут + специфікуємо wraparound (нижче §6). R3 → з accepted у **fix**.
> **RESOLVE BR-9 (MEDIUM):** backfill `SET prefs={operational:true}` **затирав** свідоме per-event OFF (напр. вимкнений `pending_aging`-спам). **Fix:** AND-reduction — категорія OFF якщо БУДЬ-ЯКИЙ її event був явно false; старі ключі НЕ видаляються (rollback під flag off читає назад); consent/prefs-audit пише `source='migration'` рядок про похідне значення.

> **RESOLVE BR-14 (HIGH, РАУНД 2):** `timezone text NOT NULL DEFAULT 'Europe/Tirane'` мовчки нав'язує намір «я в Тирані» УСІМ існуючим рядкам без сліду + немає TZ-валідації (`now() AT TIME ZONE 'BadZone'` кидає → ламає quiet-gating локації). **Fix:** колонка **NULLable** (NULL = «не налаштовано», чесно) + **CHECK проти `pg_timezone_names`** (лише зони, що PG знає) + **code-fallback `?? 'Europe/Tirane'` з `quiet_tz_fallback`-audit** (видимий слід). Незворотне (transactional) НЕ залежить від TZ (пробиває завжди) → invalid/NULL TZ не може заглушити 🔴. Backfill пілотних албанських локацій — окремим явним `UPDATE` з `source='migration'` слідом, НЕ німим DEFAULT.

```sql
-- BR-6 + BR-14: per-location timezone. NULLable (не NOT NULL DEFAULT — той мовчки нав'язував намір).
ALTER TABLE locations ADD COLUMN IF NOT EXISTS timezone text;
ALTER TABLE locations DROP CONSTRAINT IF EXISTS locations_timezone_valid;
ALTER TABLE locations ADD CONSTRAINT locations_timezone_valid
  CHECK (timezone IS NULL OR timezone IN (SELECT name FROM pg_timezone_names));
-- BR-14 backfill: ЯВНИЙ, лише пілотні албанські локації; решта лишаються NULL → code-fallback з аудитом.
--   (виконується свідомо, з notification_prefs_audit source='migration' слідом — НЕ німий DEFAULT)
-- per-target quiet hours (local HH:MM як integer minutes-of-day, 0..1439; null=disabled; from>to = wraparound через північ)
ALTER TABLE owner_notification_targets
  ADD COLUMN IF NOT EXISTS quiet_from_min integer CHECK (quiet_from_min BETWEEN 0 AND 1439),
  ADD COLUMN IF NOT EXISTS quiet_to_min   integer CHECK (quiet_to_min   BETWEEN 0 AND 1439);
-- prefs: новий канон = категорії. BR-9 intent-preserving backfill (AND-reduction).
-- operational OFF якщо власник явно вимкнув будь-який operational-event; quality з наявного rating-pref.
UPDATE owner_notification_targets SET prefs = jsonb_build_object(
    'operational', NOT (prefs ? 'order.pending_aging' AND prefs->>'order.pending_aging' = 'false'),
    'quality',     COALESCE((prefs->>'rating.low_received')::boolean, false)
  )
  WHERE NOT (prefs ? 'operational');   -- forward-only, idempotent; старі ключі НЕ видаляються (rollback-safe)
ALTER TABLE owner_notification_targets ALTER COLUMN prefs SET DEFAULT '{"operational":true,"quality":false}'::jsonb;
-- BUG-B: FORCE RLS. Policy-GUC = КАНОН app.user_id (BR-21), escape = роль (BR-20), та сама форма що §5.1
ALTER TABLE owner_notification_targets FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS owner_notification_targets_owner_all ON owner_notification_targets;
CREATE POLICY ont_owner_all ON owner_notification_targets FOR ALL TO authenticated
  USING ( location_id IN (SELECT app_member_location_ids()) );   -- канон; web prefs-шлях МУСИТЬ set_config('app.user_id') ПЕРЕД FORCE
```
> **BR-21 enable-gate ordering (forward-only, критичний):** web prefs-handler `owner/notifications.ts` (GET targets `:21`, PUT prefs, status) сьогодні робить голий SQL **БЕЗ set_config** (працює лише бо table не FORCE'd). Перед цією FORCE-міграцією handler МУСИТЬ бути мігрований на `BEGIN; set_config('app.user_id', request.user.sub, true); …; COMMIT` (канонічний патерн `customer/push.ts:35`, `courier/shifts.ts:23-24`). **Code-deploy (web setter) live ДО запуску FORCE `...049`** — інакше 100% web owner-notification операцій мертві (BR-21 outage + тихий prefs data-loss).
> Точна мапа event→category (operational/quality) фіксується разом з `EVENT_REGISTRY.category` (§3 A2) і ETHICAL-інваріантом (§ нижче): незворотні-у-вікні-тиші = `transactional`, не в prefs.
**Форма `prefs` (новий канон):** `{"operational": <bool>, "quality": <bool>}`. `transactional` — НЕ в prefs (за визначенням non-mutable; gating-крок 2 його не читає). Старі ключі (`order.created` тощо) лишаються в рядку безпечно — gating їх більше не читає (читає лише `category`). Опційно прибрати їх у тій самій міграції `prefs - 'order.created' - 'order.pending_aging'` для чистоти. **Гроші не торкаються — integer-інваріант не порушений.**

### 5.3 `...050_notification-prefs-audit.ts` (operator-action audit на зміну prefs)

> **RESOLVE Counsel 3-3:** перейменовано `notification_consent_log` → **`notification_prefs_audit`**. Це НЕ data-subject consent (це власник над власними prefs) — назва `consent` over-claims і створить хибне відчуття суб'єкта даних у RoPA. `/compliance` дістає рядок «operator-action audit, не data-subject consent». Додано `source='migration'` для BR-9 backfill-сліду.

```sql
CREATE TABLE IF NOT EXISTS notification_prefs_audit (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  target_id uuid REFERENCES owner_notification_targets(id) ON DELETE SET NULL,
  actor_user_id uuid,                       -- хто змінив (membership); null для source='migration'
  source text NOT NULL CHECK (source IN ('web','telegram','migration')),
  category text NOT NULL CHECK (category IN ('operational','quality')),
  new_value boolean NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
ALTER TABLE notification_prefs_audit ENABLE ROW LEVEL SECURITY;
ALTER TABLE notification_prefs_audit FORCE ROW LEVEL SECURITY;
CREATE POLICY npa_tenant ON notification_prefs_audit FOR ALL TO authenticated
  USING ( location_id IN (SELECT app_member_location_ids()) );   -- канон app.user_id (BR-21); worker/migration пише під BYPASSRLS-роллю (BR-20)
CREATE INDEX IF NOT EXISTS npa_loc_created_idx ON notification_prefs_audit(location_id, created_at);
```
**Storefront-toggle — НЕ нова колонка** (`delivery_paused` уже є). DDL на `locations` лише `timezone` (BR-14, §5.2).

### 5.4 `...051_telegram-action-nonces.ts` (BR-19, РАУНД 2 — stateful confirm)

> **RESOLVE BR-19 (LOW, РАУНД 2):** confirm-friction раунд-1 був stateless → прямий `store.close.confirm:<loc>` / replay обходив його. **Fix:** one-shot nonce з TTL; consume через `DELETE…RETURNING`.

```sql
CREATE TABLE IF NOT EXISTS telegram_action_nonces (
  nonce uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id uuid NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  action text NOT NULL CHECK (action IN ('store.close')),   -- лише незворотні-намір дії
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz NOT NULL
);
ALTER TABLE telegram_action_nonces ENABLE ROW LEVEL SECURITY;
ALTER TABLE telegram_action_nonces FORCE ROW LEVEL SECURITY;
CREATE POLICY tan_tenant ON telegram_action_nonces FOR ALL TO authenticated
  USING ( location_id IN (SELECT app_member_location_ids()) );   -- канон app.user_id (BR-21). ⚠ webhook consume-DELETE біжить web-роллю → МУСИТЬ set_config('app.user_id', targetUserId) (BR-22)
CREATE INDEX IF NOT EXISTS tan_expires_idx ON telegram_action_nonces(expires_at);  -- TTL-cleanup
```
TTL-cleanup на тому ж 90d-cron (`DELETE … WHERE expires_at < now()`). Об'єм: ~6/лок/день, TTL 2хв → одиниці живих рядків.

### 5.5 `...052_notif-worker-role.ts` (BR-20, РАУНД 3 — dedicated restricted role з verified BYPASSRLS)

> **RESOLVE BR-20 (HIGH, РАУНД 3):** worker-escape перенесено з self-settable GUC у **роль**. Notif-worker (і ЛИШЕ він) конектиться окремим пулом під `deliveryos_notif_worker`. **Verified-grant (не swallowed):** провал BYPASSRLS = FATAL-міграція + FATAL boot-assert.

```sql
-- НОВА restricted роль для notif-worker. Verified BYPASSRLS — БЕЗ swallowed EXCEPTION (на відміну від BR-1 раунд-1).
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='deliveryos_notif_worker') THEN
    CREATE ROLE deliveryos_notif_worker LOGIN PASSWORD :'notif_worker_pw' BYPASSRLS;
  ELSE
    ALTER ROLE deliveryos_notif_worker BYPASSRLS;
  END IF;
END $$;   -- НЕ EXCEPTION WHEN OTHERS: якщо платформа не дає BYPASSRLS → міграція FATAL-fail'иться (дізнаємось на staging)
-- мінімальні grants: лише notif-таблиці (least-privilege; BYPASSRLS дає обхід RLS, не доступ до чужих схем)
GRANT SELECT, INSERT, UPDATE, DELETE ON owner_notification_targets, notification_outbox_audit,
  notification_prefs_audit, telegram_action_nonces TO deliveryos_notif_worker;
GRANT SELECT ON locations, memberships, orders TO deliveryos_notif_worker;   -- read для fan-out/quiet-gating
GRANT USAGE ON SCHEMA public TO deliveryos_notif_worker;
```
> **Pool + boot-assert (`db/index.ts`, взірець наявного `createSessionPool`):** `createNotifWorkerPool()` під `DATABASE_URL_NOTIF_WORKER` (нова env, роль `deliveryos_notif_worker`, max=2). `pool.on('connect')` assert: `SELECT current_user, rolbypassrls FROM pg_roles WHERE rolname=current_user` → якщо current_user≠`deliveryos_notif_worker` ∨ `rolbypassrls≠true` → **throw FATAL** (worker не стартує). Дзеркало наявного guardrail `db/index.ts:32` («operational НЕ під postgres»), у інший бік. `server.ts:346` — `new NotificationWorker(notifPool,…)` де `notifPool=createNotifWorkerPool()`, НЕ operational `pool`.
> **Fallback (R-ROLE/R-BR24, STOP-DESIGN-B):** якщо Supabase НЕ дає BYPASSRLS не-superuser-ролі → `policy TO deliveryos_notif_worker` (role-based escape-гілка в кожній policy, BYPASSRLS не потрібен) — еквівалентна безпека (не self-grantable), per-table вартість. **RESOLVE BR-24 (MEDIUM, РАУНД 4) — fallback НЕ приймається «еквівалентним» на віру:** (b)-гілка отримує ВЛАСНИЙ **functional** boot-assert (не атрибутивний `rolbypassrls`, бо в fallback він легітимно false) — `SELECT 1 FROM owner_notification_targets WHERE location_id=$seedLoc` під FORCE на notif-конекті БЕЗ GUC → **рядок є, інакше FATAL-exit**. Це доводить «роль реально SELECT-ить targets під FORCE через `TO role`-гілку» функціонально, не припущено. Boot-логіка обирає assert за режимом: `rolbypassrls=true`→(a)-assert; інакше→(b)-functional-assert; обидва false→FATAL «escape не працює жодним механізмом». **Integration-тест на ОБОХ гілках** (CI/staging проганяє SELECT-targets-під-FORCE у конфігах a/b). **`SET ROLE` на operational заборонений** (DoD §5.1 п.4 grep `SET ROLE`→0 поширюється і на fallback — окремий пул/credential, не `SET ROLE` shortcut). Backend перевіряє BYPASSRLS-доступність на staging ПЕРШИМ кроком — вона визначає deploy-time гілку.

### Retention
Audit + consent: cron `DELETE … WHERE created_at < now()-interval '90 days'` (consent — GDPR-мін, можна довше; узгодити з `/compliance` RoPA). Range-index уже додано.

---

## 6. Узгодженість + Ідемпотентність

- **Storefront подвійний-тап (RESOLVE BR-23 HIGH, РАУНД 4):** C2 `cur FOR UPDATE`-CTE розрізняє три випадки замість колапсу в «0-rows=success». 2-й тап (рядок видимий під RLS, вже `$new`) → 1 row, `was=now` → `store.toggle.noop`, бот «вже на паузі» (м'яко). **RLS-deny (рядок невидимий)** → 0 rows → `store.toggle.denied`, бот **помилка + web-fallback** (НЕ тихий «✅»). Store-toggle ставить канон `app.user_id`=targetUserId (NULL→reject), бо `locations`-policy читає `app.user_id`, НЕ `app.current_tenant`. Guard-on-rowcount консистентний з order-площиною (обидві: 0-rows=рядок невидимий=fail). ✅
- **Гонка web↔telegram на тих самих prefs (RESOLVE BR-4 HIGH):** proposal раніше стверджував «одночасні зміни різних категорій не б'ються» — **хибно для одного рядка** (два UPDATE від одного snapshot серіалізуються на row-lock, останній перезаписує клітинку іншого). Live-anchor: `notifications.ts:135-141` = read-merge-write на цілому jsonb. **Fix:** atomic per-cell **без попереднього SELECT** — кожна змінена категорія = окремий statement:
  ```sql
  UPDATE owner_notification_targets
     SET prefs = jsonb_set(prefs, '{operational}', to_jsonb($newVal::boolean))
   WHERE id=$1 AND location_id=$2;
  ```
  Read-committed: кожен `UPDATE` бере row-lock і читає НАЙСВІЖІШИЙ committed image (не stale snapshot). Concurrent web(operational←false) ∥ telegram(quality←true) → серіалізуються на row-lock, кожен застосовується до результату попереднього → фінал `{operational:false, quality:true}`. Lost-update класу більше не існує (елімінований, не залатаний). **Web PUT `notifications.ts` видаляє read-merge-write**, замінюється циклом per-changed-category UPDATE у тій же транзакції що prefs-audit INSERT. R4 «Accepted» → **Fixed** (попереднє обґрунтування спиралось на хибне розуміння jsonb_set-атомарності).
- **Гонка toggle↔вхідне замовлення (RESOLVE BR-7 + BR-17 MEDIUM, РАУНД 2):** live-verified — `orders.ts` НЕ читає `delivery_paused` (grep → 0). Раунд-1 fix (read-`delivery_paused`-then-insert + 409) лишав TOCTOU-вікно (BR-17): close комітиться між SELECT і INSERT → замовлення проскакує. Для обіцянки «екстрено стоп» (пожежа/отруєння) це неприйнятно. **Fix (BR-17, атомарний, замінює soft-409 read-then-insert):** чек переноситься в САМ INSERT як conditional-write:
  ```sql
  INSERT INTO orders (...) SELECT ... FROM locations l
   WHERE l.id = $loc AND l.delivery_paused = false;
  -- 0 rows inserted → 409 store_paused (атомарно, нуль вікна)
  ```
  Read-committed: INSERT…SELECT читає свіжий committed `delivery_paused` у момент вставки. Close закомітився ДО INSERT → 0 рядків → 409. **TOCTOU-вікно елімінується** (не звужується). «Екстрено стоп» authoritative на момент INSERT. R2' (accept-risk раунд-1) → **Fixed**. Hard capacity-gate (stock-aware) — окремий скоуп, інша річ (не race).
- **Inbound callback dedup:** Telegram ретраїть webhook при не-200. Webhook завжди 200 → дублі рідкісні, але можливі (мережа). Дія ідемпотентна by design (C2 compare-set; order-actions уже 409-safe). Подвійний callback на toggle → 2-й no-op.
- **Held re-enqueue ідемпотентність:** dedup-ключ `held:{event}:{entity}:{loc}` + max-1-rehold guard унеможливлює дублікат-доставку після quiet.

---

## 7. Відмови + Деградація (кожен зовнішній виклик: timeout + fallback, нуль каскаду)

| Сценарій | Поведінка |
|---|---|
| **Telegram API down (outbound)** | Наявний per-chat circuit-breaker (5 fail → 60s cooldown) + retry-policy (max 10, backoff). Held→archive по max-retries (audit `archived`). Нуль каскаду на order-flow (off critical-path). |
| **Telegram network-blackhole (RESOLVE BR-10 MEDIUM)** | Live: `callTelegramApi`/`sendMessage` (`telegram-webhook.ts:536,557`) + dispatcher fetch — БЕЗ AbortController; Node `fetch` без app-timeout висить ~OS-default → тримає pg-boss job-slot + конект → burst вичерпує boss-pool (каскад навіть на 🔴). Circuit-breaker не спрацьовує (рахує лише ПІСЛЯ повернення fetch). **Fix (у скоупі MVP, НЕ optional):** `AbortController` 10s на КОЖЕН outbound fetch → timeout → `{delivered:false, reason:'TIMEOUT'}` → circuit-breaker рахує, job acked, конект звільнений. `finally clearTimeout`. |
| **Multi-instance worker scale (RESOLVE BR-5 HIGH)** | In-memory `lastSendPerChat`/`circuitState` Map (`index.ts:47,361`) не координується між інстансами → split-brain throttle/breaker. **MVP-вердикт: defer-flag** — документований інваріант `NOTIF_WORKER_SINGLETON` (один worker-інстанс); scaling-gate блокує horizontal worker-scale поки `TG_WORKER_MULTI` off. Burst у один чат: per-chat 1200ms + Telegram-side 429-retry (наявний retry-policy ловить). `MISSING: distributed per-chat throttle` (Redis/pg-advisory) перед `TG_WORKER_MULTI`. |
| **Webhook retry / duplicate (inbound)** | Дія ідемпотентна (C2). Webhook завжди 200 → Telegram не ретраїть на бізнес-помилку. |
| **Dispatcher падає посеред fan-out** | pg-boss job не acked → re-delivery. Per-target audit-INSERT + in-process dedup-cache робить повтор частково ідемпотентним; для toggle/order — compare-set/409 ловить. Audit `sending` без `delivered` = видимий "stuck" сигнал. |
| **Quiet-черга росте** | back-of-envelope ~750/ніч — не росте необмежено (cap 1 re-hold; deliver-on-window-open). Алерт якщо held-backlog > 5000 (10× норми). |
| **`setAcceptingOrders` під час messageBus-down** | Compare-set commit'иться (БД авторитет); realtime publish — best-effort у тій же транзакції (outbox). Якщо publish впав — in-app дізнається на наступному pol/ WS-reconnect. Стан у БД коректний (нуль розсинхрону authority). |
| **prefs-audit INSERT падає (RESOLVE BR-16, РАУНД 2)** | prefs-`jsonb_set`-UPDATE і `notification_prefs_audit`-INSERT — у ТІЙ САМІЙ явній `BEGIN…COMMIT` транзакції (та сама обгортка, що web prefs-setter BR-21). Якщо audit-INSERT падає (CHECK на category/source) → `ROLLBACK` всієї зміни (no silent pref-change without audit record). DoD-тест: CHECK-violation на category → prefs незмінені. |
| **`store_paused` 409 — клієнтський копірайт (RESOLVE Counsel R3 нон-блокінг)** | BR-17 атомарний guard повертає `409 store_paused` коли оператор поставив паузу між open-menu й place-order. Frontend checkout-UI МУСИТЬ показати **«Заклад щойно поставив паузу на приймання — спробуйте трохи згодом»**, НЕ сирий код/generic «помилка». `delivery_paused` мінливий саме у вікні наповнення кошика (на відміну від статичного `NOT_PUBLISHED`) → ця відмова частіше ловить чесного клієнта; backend каже правду (409) — копірайт має теж. **Власник: клієнтський checkout-UI (frontend).** |

---

## 8. Безпека + Tenant-ізоляція

- **Storefront authority (RESOLVE BR-3 HIGH) — locationId У callback_data + явний scope:** live-anchor: non-order гілка `telegram-webhook.ts:159-172` бере `rows[0]` БЕЗ location-scope → недетермінована локація при N>1 targets на chat (франшиза, спільний чат). `callback_data` для storefront = **`store.close:<locationId>`** / `store.open:<locationId>`. Резолвер скоупить явно:
  ```sql
  SELECT ont.id, ont.user_id FROM owner_notification_targets ont
   WHERE ont.address=$chatId AND channel='telegram' AND status='active'
     AND ont.location_id=$locationIdFromCallback;   -- явний scope, НЕ rows[0]
  -- 0 rows → 'Account not linked to this location' → reject; далі membership(user,location)
  ```
  Authority = `(chatId↔target) ∩ (locationId з callback) ∩ membership(user↔location)`. Кнопка, яку бот САМ надіслав у чат конкретної локації, несе свою `locationId`; підробка чужого `locationId` провалює target-lookup (немає active `(chatId, чужий loc)` рядка) → нуль крос-tenant, нуль недетермінованого `rows[0]`. (Order-actions вже мають свій scope через order→location — не чіпаємо.)
- **Secret-token (RESOLVE BR-8 + BR-18 MEDIUM, РАУНД 2) — allowlist default-deny, НЕ denylist-prefix:** live-anchor `telegram-webhook.ts:43-46` — header-absent → process anyway. Раунд-1 fix класифікував mutating по `data`-prefix (`order.`/`store.`/`settings.`) — але це **denylist**: новий mutating-handler з іншим префіксом (`shift.`, `ack.`) або lenient link-flow → bypass (BR-18). **Fix (інверсія):** **allowlist read-only** — явний вичерпний `INBOUND_LENIENT` enum (`noop`,`help`,`menu.view`,`status.view`…); `isMutatingOrLinking = NOT INBOUND_LENIENT.has(action)` → **secret обов'язковий і має збігатись для ВСЬОГО, крім явного read-only** (невідомі/майбутні префікси fail-CLOSED). **Link-flow (`/start <token>`) — ЗАВЖДИ потребує secret** (це state-write: лінкує chat↔location; прибрано з lenient → атакувальник без secret не залінкує свій chatId). Flag `TG_WEBHOOK_STRICT` керує лише allowlist-read-only гілкою, НІКОЛИ mutating/linking. Absent ∨ mismatch на mutating → нуль state-change (200 + `answerCallbackQuery error`).
- **Storefront `close` — confirm з echo локації + nonce (RESOLVE BR-15+BR-19, Counsel 3-2):** `store.close:<loc>` (крок 1) → генерує one-shot `nonce`, пише `(nonce, location_id, action, expires_at=now()+'2 min')` у `telegram_action_nonces`; шле confirm-кнопку **`Закрити приймання для «{location.name}»? Клієнти бачитимуть пауза.`** (BR-15: echo конкретної локації → intent-перевірка людиною на in-tenant A∩B lateral). Confirm несе `store.close.confirm:<nonce>`. Крок 2: `DELETE FROM telegram_action_nonces WHERE nonce=$1 AND action='store.close' AND expires_at>now() RETURNING location_id` — атомарний consume (0 rows → expired/replayed/прямий-bypass → reject, нуль мутації). `location_id` береться з nonce-store (trusted), НЕ з callback (BR-15 підсилення). `store.open:<loc>` — один тап (відкриття зворотне, нешкідливе). Compare-set захищає від *повтору*, confirm-nonce — від *першого помилкового наміру* + *прямого/replay confirm-callback* (BR-19). **Nonce-INSERT-fail → явний fallback-pointer (Counsel R3 нон-блокінг):** якщо `telegram_action_nonces`-INSERT падає (БД glitch) → бот шле «Не вдалось підготувати підтвердження — закрийте приймання через застосунок», явний fallback на web-канал (швидкий, без nonce), НЕ тиха відсутність confirm-кнопки.
- **Replay протермінованого callback (RESOLVE BR-19):** Telegram callback_data не має TTL. Mitigation: confirm-крок тепер stateful one-shot nonce (вище) — прямий `store.close.confirm:<nonce>` без кроку 1 → nonce не існує → reject; replay → вже consumed → reject; протермінований → `expires_at` фільтрує. Дія ідемпотентна (replay store.open коли вже open → no-op); order-actions 409 ловить; allowlist-secret блокує підроблений replay на мутацію.
- **`/settings` inline у Telegram:** кнопки toggle несуть лише `category` у callback_data; авторитет (який target/location) — з chatId-резолвера, не з кнопки. Toggle пише `jsonb_set` під tenant-context + consent-log.
- **Webhook order-action authority (RESOLVE BR-22 HIGH, РАУНД 3) — `app.user_id` = доведений active-member + guard-on-rowcount:** R-PRE-обгортка `telegram-webhook` order-handler ставить `set_config('app.user_id', <resolvedMember>, true)`. **`<resolvedMember> := targetUserId`** — той самий user, що webhook УЖЕ верифікував як active-member ПЕРЕД мутацією (`telegram-webhook.ts:147-157` резолвить `targetUserId` з `owner_notification_targets.user_id`; `:185-194` перевіряє `memberships(targetUserId, locationId, status='active')`, rowCount=0 → reject). orders-FORCE-policy `app_member_location_ids()` для цього user поверне `location_id` → guarded UPDATE проходить. **Legacy NULL-user (`:185` пропускає membership-перевірку): order-mutating дія з `targetUserId IS NULL` → reject** («Account link incomplete — reconnect»), НЕ ставити порожній app.user_id (інакше FORCE-deny→fail-silent). **Guard-on-rowcount:** `updateOrderStatus` під FORCE з невірним app.user_id поверне **0 rows БЕЗ throw** (RLS-deny не кидає). Fix: `if (upd.rowCount===0) throw GUARDED_NOOP` → бот рапортує **помилку** («не вдалось підтвердити»), НЕ `'✅ confirmed'`; замовлення лишається PENDING і оператор це **бачить**. guard-on-rowcount відновлений — нуль тихого «confirmed»/мертвий-PENDING.
- **Webhook store-toggle authority (RESOLVE BR-23 HIGH, РАУНД 4) — канон `app.user_id` + RLS-visibility розрізнення на FORCE-таблиці `locations`:** `locations` — FORCE RLS **вже сьогодні** (`core-identity.ts:85-87`), policy `id IN (SELECT app_member_location_ids())` читає **`app.user_id`** (`:72`). Раунд-1 §C2 ставив `app.current_tenant` (хибний GUC) → deny на КОЖНОМУ store.close. **Fix п.1:** store-toggle обгортка ставить `set_config('app.user_id', targetUserId, true)` — `targetUserId` = вже-верифікований active-member (webhook non-order resolver `telegram-webhook.ts:160-194`, той самий патерн що BR-22). NULL-user store-mutating → reject. **Fix п.2 (суть):** `setAcceptingOrders` 0-rows під FORCE мав ДВА нерозрізненні джерела — idempotent-noop (рядок видимий, вже `$new`) і RLS-deny (рядок невидимий). Раунд-1 трактував обидва як success → на deny бот «✅ пауза», storefront ВІДКРИТИЙ (fail-silent на «екстрено стоп»). `cur FOR UPDATE`-CTE (§C2) дає окремий visibility-сигнал: **0 rows = deny → бот ПОМИЛКА + web-fallback (ніколи success); 1 row & `was=now` = idempotent-noop (бот «вже …», не помилка); 1 row & `was≠now` = changed (бот «✅ HH:MM» + publish)**. `GUARDED_NOOP`-throw з BR-22 непридатний (store 0-rows ЛЕГІТИМНО = double-tap), тому розрізнення — через CTE, не throw. **Дві guarded-площини консистентні:** order і store обидві «рядок невидимий (0-rows)=fail»; store додатково має легітимний same-value-noop у ОКРЕМОМУ сигналі.
- **RLS FORCE** на всіх чотирьох таблицях (targets, audit, prefs_audit, nonces) — закриває BUG-B. Policy-GUC = **канон `app.user_id`** (BR-21, рівно як `orders`/`locations`); web/webhook під `deliveryos_api_user` читають tenant-ізольовано через `app_member_location_ids()`. **Notif-worker** обходить RLS легітимно через **роль `deliveryos_notif_worker` (verified BYPASSRLS)** — окремий пул, не self-settable GUC (BR-20). Web-роль структурно не може обійти RLS жодним шляхом.
- **Нуль PII:** audit/consent зберігають event/category/target_id/status — нуль customer-PII (узгоджено з P0-privacy). Telegram body вже керується `telegram_alert_detail` (`...044`).
- **GDPR consent-log** на кожну зміну prefs (хто/коли/звідки/категорія/значення) — у тій же транзакції що зміна.

---

## 9. Операбельність

- **Метрики (observability < 1 хв):** з `notification_outbox_audit` агрегувати per-location/per-status за 5-хв вікно: `delivered`, `failed`, `prefs_disabled`, `quiet_hours`, `held`, `circuit_open`. Expose у наявному health/metrics шляху.
- **RESOLVE BR-12 (LOW) — точна метрика, не `count(*)`:** live-anchor `index.ts:387-392,415-420` пишуть audit з `ON CONFLICT DO NOTHING` БЕЗ unique-constraint (PK лише на `id`) → no-op clause мертвий, job-retry дублює `sending`/`delivered` рядки. **Fix:** прибрати мертвий `ON CONFLICT` (бреше про ідемпотентність — audit append-only by design); метрика `delivered`/`failed`-rate рахує **останній термінальний статус per `(event,target_id)`** за вікно (window-function `DISTINCT ON`), не `count(*)` → алерт `failed/(delivered+failed)>30%` не шумить на retry.
- **Enable-gate (Counsel 3-5):** 90d retention-cron має бути **live ДО** `TG_CATEGORY_GATING=on`. BUG-A фікс почне реально писати 52k рядків/день — без живого retention «нуль тихих дропів» оплачується «один гучний hotspot» саме на observability-таблиці.
- **Алерти:** `failed/(delivered+failed) > 30%` за 15 хв → degraded; held-backlog > 5000 → warn; будь-який `target_inactive`-сплеск → можливий масовий /stop або blocked-bot.
- **Дебаг "чому сповіщення не прийшло":** один запит `SELECT status, count(*) FROM notification_outbox_audit WHERE location_id=$1 AND event=$2 AND created_at>… GROUP BY status` дає вердикт: `pref_off`(категорія off), `quiet_hours`(held до ранку), `no_target`(не підключено), `target_inactive`(/stop або blocked), `failed`(Telegram). **Після фіксу BUG-A ці рядки нарешті реально пишуться** — без фіксу дебаг неможливий.
- **Health degraded-vs-down:** Telegram outbound — degraded (off critical-path), НЕ down. Order-flow незалежний.
- **Rollback:** міграції forward-only; код за flag `TG_CATEGORY_GATING` (default off → стара per-event поведінка; prefs backfill сумісний з обома, бо старі ключі лишаються). `store.open/close` дія за flag `TG_STOREFRONT_ACTION`. Schema-first ("шви в схему"), рантайм вмикаємо окремим явним актом.
- **Scaling-gate:** при >30 msg/s глобально (≈ 6× поточного піку) — переглянути bot-sharding; зараз НЕ робимо (premature).

---

## 10. Відкриті / Прийняті ризики

> Після RESOLVE-раунду 1: усі CRITICAL/HIGH — fixed (BR-1,2,3,4,6) або defer-flag (BR-5).
> Після RESOLVE-раунду 2: BR-13 (CRITICAL), BR-14 (HIGH), BR-15…19 — fixed.
> Після RESOLVE-раунду 3 (СТРУКТУРНЕ): BR-20 (HIGH) — fixed (роль `deliveryos_notif_worker` + verified BYPASSRLS); BR-21 (CRITICAL) — fixed (policy-GUC = канон `app.user_id`, web prefs-шлях мігрує ПЕРЕД FORCE); BR-22 (HIGH) — fixed (resolvedMember = верифікований `targetUserId`, 0-rows = explicit failure).
> **Після RESOLVE-раунду 4 (вузький):** BR-23 (HIGH) — **fixed** (store-toggle канон `app.user_id`=targetUserId + `cur FOR UPDATE`-CTE 3-way: deny=fail+web-fallback, idempotent-noop≠помилка, changed=«✅ HH:MM»; дві guarded-площини консистентні на «0-rows=рядок невидимий=fail»); BR-24 (MEDIUM) — **fixed** (fallback `policy TO role` отримує власний **functional** boot-assert + integration-тест на обох гілках + `SET ROLE`-заборона; STOP-DESIGN-B: staging-проба BYPASSRLS = ПЕРШИЙ крок, визначає deploy-time гілку). **Відкритих CRITICAL = 0, відкритих HIGH = 0, відкритих MEDIUM = 0.** Нижче — accepted-risk + defer + передумова + STOP-DESIGN-B.

| # | Ризик | Рішення | Власник |
|---|---|---|---|
| R-BR1 | FORCE RLS + хибне service-role → можливий outage | **FIXED** (BR-1): worker-escape GUC `app.notif_worker` у policy; не залежить від BYPASSRLS; DoD integration-test gate | Backend owner |
| R-BR2 | Статус-розсинхрон `pref_off`↔`prefs_disabled` → BUG-A не фіксований | **FIXED** (BR-2): канон = `prefs_disabled`; CHECK+union узгоджені | Backend owner |
| R-BR3 | non-order authority `rows[0]` → дія на чужу локацію | **FIXED** (BR-3): `store.*:<locationId>` callback + явний scope | Backend owner |
| R-BR4 | web↔telegram lost-update prefs | **FIXED** (BR-4): atomic per-cell `jsonb_set` без SELECT; web PUT мігрує | Backend owner |
| R-BR6 | quiet-hours TZ-зсув + wraparound | **FIXED** (BR-6): `locations.timezone` колонка + специфікований wraparound | Backend owner |
| R-BR13 | FORCE RLS на нових таблицях + autocommit-патерн → outage АБО session-leak cross-tenant | **FIXED — СУПЕРСЕДЕД РАУНДОМ 3:** worker-escape-GUC `app.notif_worker` ВИДАЛЕНО; escape тепер через роль `deliveryos_notif_worker` BYPASSRLS (R-BR20). Немає GUC → немає ні autocommit-outage (роль обходить FORCE без GUC), ні session-leak (немає GUC, що міг би leak'нути). Транзакція потрібна ЛИШЕ для атомарності prefs+audit (BR-16), не для RLS-escape. | Backend owner |
| R-BR20 | Worker-escape self-grantable GUC на спільному пулі → крос-tenant read | **FIXED** (BR-20, раунд 3): окрема роль `deliveryos_notif_worker` + verified BYPASSRLS (FATAL-on-missing); `app.notif_worker` ВИДАЛЕНО з усіх policy; escape = атрибут ролі, не self-settable | Backend |
| R-BR21 | FORCE-policy на `request.jwt.claim.sub` (0 setters) → outage + тихий prefs data-loss | **FIXED** (BR-21, раунд 3): policy-GUC = канон `app.user_id`; web prefs-шлях мігрує на setter ПЕРЕД FORCE (enable-gate); DoD тестує той GUC що policy читає | Backend |
| R-BR22 | webhook resolved `app.user_id` недовизначений → guarded transition fail-silent «confirmed» | **FIXED** (BR-22, раунд 3): resolvedMember = верифікований `targetUserId` (membership-resolver `:185-194`); NULL-user→reject; 0-rows UPDATE→`GUARDED_NOOP` throw, бот рапортує помилку | Backend |
| R-ROLE | **STOP-DESIGN-B:** нова DB-роль `deliveryos_notif_worker` + BYPASSRLS-grant — чи Supabase дає атрибут не-superuser-ролі? | **Винесено людині (STOP-DESIGN-B)** — Backend/Infra перевіряє на staging ПЕРШИМ; якщо НІ → fallback `policy TO deliveryos_notif_worker` (role-based, BYPASSRLS не потрібен). FATAL-on-missing boot-assert. | Backend/Infra+Arch |
| R-BR23 | store-toggle 0-rows колапсує RLS-deny vs idempotent-noop → «екстрено стоп» fail-silent | **FIXED** (BR-23, раунд 4): канон `app.user_id`=targetUserId (NULL→reject) + `cur FOR UPDATE`-CTE → 0-rows=deny=FAIL+web-fallback (ніколи success), 1-row-`was=now`=noop (не помилка), 1-row-`was≠now`=changed; guard консистентний з order-площиною | Backend |
| R-BR24 | Fallback `policy TO role` без власного verified-gate → BR-1-клас outage якщо BYPASSRLS недоступний і (b) має edge | **FIXED** (BR-24, раунд 4): (b)-гілка отримує **functional** boot-assert «SELECT targets під FORCE без GUC → рядки, інакше FATAL» (доводить escape функціонально, не атрибутивно) + integration-тест на ОБОХ гілках (a/b) + заборона `SET ROLE` на operational-пулі. STOP-DESIGN-B: staging-проба BYPASSRLS-доступності = **ПЕРШИЙ крок**, визначає яку гілку (a/b) деплоїти (deploy-time вибір, не runtime-fallback). | Backend/Infra+Arch |
| R-POOL | +2 конекти (notif-worker пул); §2 «0 нових конектів» застарів | **Accept** — перераховано 13 сукупно (operational 8 + session 3 + notif 2); Supavisor тримає сотні | Backend |
| R-ORDER | FORCE-міграція раніше за web-setter-код → BR-21 outage | **Enable-gate ordering** — web prefs `set_config('app.user_id')` deploy live ДО FORCE `...049`; DoD §5.1 п.1+3 блокує деплой FORCE якщо web мертвий | Backend |
| R-PRE | **Pre-existing (вузький):** `telegram-webhook` order-handler ставить `set_config('app.current_tenant',true)` поза транзакцією + не ставить `app.user_id` (від якого реально залежить orders-policy) → клас «transaction-local GUC без транзакції» | **Передумовний фікс** — обгорнути handler у `BEGIN; set_config('app.user_id',targetUserId,true); set_config('app.current_tenant',$loc,true); …; COMMIT` (BR-22: targetUserId = верифікований член); закривається ТИМ САМИМ PR, що нова `store.close`. lint-guard CI: «`set_config(...,true)` потребує `BEGIN`». **Gate: ПЕРЕД `TG_STOREFRONT_ACTION=on`.** **Окремий security-changelog рядок** (Counsel R3): «зміцнено tenant-ізоляцію наявного Telegram order-action шляху (app.user_id під FORCE)» — самостійна security-перемога. НЕ repo-wide. | Backend |
| R2' | `delivery_paused` не блокує order-insert | **FIXED** (BR-17, раунд 2): атомарний `INSERT…SELECT WHERE delivery_paused=false` — TOCTOU-вікно елімінується; «екстрено стоп» authoritative на момент INSERT. Hard capacity-gate (stock-aware) — окремий скоуп, інша річ | Product+Arch |
| R-BR5 | In-memory rate-limit не мульти-інстанс | **Defer-flag** `TG_WORKER_MULTI`; інваріант `NOTIF_WORKER_SINGLETON`; scaling-gate блокує horizontal scale; `MISSING: distributed throttle` | Backend/Ops |
| R-BR11 | DELETE-retention без партицій @52k/день | **Accept-risk** — range-index тримає @50лок; тригер партиціювання `>200 лок` ∨ `>20M rows` | Ops |
| OPEN-Q-HANDOVER | Право оператора *передати* лінію (відпустка/зміна), не лише вимкнути (Counsel §5) | **Defer (людське)** — продуктове питання «сповіщення однієї людини vs передача чергування»; поза скоупом MVP | Product |
| STOP-ETHICS-1 | Фінальна мапа transactional перед prod-enable | **Записаний людський акт** — Product+Arch підпис у ADR (особливо `pending_aging`); fail-safe = все transactional якщо забудуть. **БЕЗ ЗМІН раунд 3.** | Product+Arch |
| STOP-DESIGN-B | Нова топологічна залежність: DB-роль `deliveryos_notif_worker` + BYPASSRLS + окремий пул + env | **Записаний людський акт (design-gate)** — Backend/Infra+Arch підтверджують: (1) приймають +1 роль/пул/env у топологію конектів. **ПЕРШИЙ крок impl на staging (визначає deploy-time гілку, BR-24):** `CREATE ROLE deliveryos_notif_worker … BYPASSRLS` → `SELECT rolbypassrls`=true? **ТАК** → гілка (a) BYPASSRLS, (a)-boot-assert активний. **НІ** (Supabase відмовляє не-superuser-ролі) → гілка (b) `policy TO deliveryos_notif_worker` у всіх 4 policy, (b)-**functional**-boot-assert «SELECT targets під FORCE без GUC → рядки, інакше FATAL». Обидві гілки мають код+integration-тест; staging-проба обирає живу. `SET ROLE` на operational заборонений у ОБОХ гілках (окремий пул/credential). | Backend/Infra+Arch |

---

## Конспект розвилок

- **A: Policy Gateway (централізований gating у воркері)** > scattered inline — один source-of-truth, один audit-вузол, узгоджено з NOTIFICATION-CONSOLIDATION.
- **B: Deferred-deliver (pg-boss `startAfter`)** > silent-drop — нуль drop, held долітає вранці, черга в БД ~750/ніч.
- **C: Idempotent compare-set (`IS DISTINCT FROM` + shared `setAcceptingOrders`)** > read-then-write — атомарно, ідемпотентно, нуль паралельної мутації з web.
