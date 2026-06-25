# Breaker Findings — Telegram Notifications + Actions

**Breaker:** System Breaker DeliveryOS
**Date:** 2026-06-22
**Target:** `docs/design/telegram-notifications-actions/proposal.md` + `docs/adr/ADR-TELEGRAM-NOTIFICATIONS-ACTIONS.md`
**Метод:** READ-ONLY перевірка реального коду/міграцій. Жодних фіксів — лише де ламається + порушений інваріант.

Усі anchors звірені з робочим деревом (не з proposal-таблицею).

---

## CRITICAL

### BR-1 · B-SEC / B-FAIL — `service-role`-припущення хибне: FORCE RLS на audit/targets може повністю зламати диспетчер
**Вектор:** B-SEC (RLS), B-FAIL (каскад на весь notification-flow).
**Знахідка.** Proposal §5.1/§8/§5.2 фундаментально спирається на твердження: *"Воркер пише під service-role (поза RLS) — FORCE не ламає запис"*. Це **неправда за фактом коду**:
- Operational pool конектиться як `deliveryos_api_user` (`.env:4` → `DATABASE_URL_OPERATIONAL=...deliveryos_api_user...`), а НЕ `service_role`, НЕ `postgres`. Ніде в коді нема `SET ROLE service_role`.
- `handleTelegramSend`/`handleDispatch` (`apps/api/src/notifications/workers/index.ts`) пишуть audit і SELECT-ять `owner_notification_targets` **без** `set_config('app.current_tenant')` і без membership-context (на відміну від order-actions, що роблять `set_config`). Зараз це працює ЛИШЕ тому, що targets має `ENABLE` без `FORCE` → policy не застосовується до власника таблиці.
- Єдине, що рятує — `ALTER ROLE deliveryos_api_user BYPASSRLS` у `1780691681296_ops-location-alerts-policy.ts:8`. Але воно загорнуте в `DO … EXCEPTION WHEN OTHERS THEN -- Ignore if not allowed`. На Supabase pooler `ALTER ROLE … BYPASSRLS` потребує суперюзера; якщо платформа його не дала — **виняток мовчки проковтнувся, і роль НЕ має BYPASSRLS.**

**Сценарій поломки.** Якщо BYPASSRLS-grant не пройшов у проді (а ми цього не знаємо — exception swallowed), то після `...048`/`...049` додають `FORCE ROW LEVEL SECURITY`:
- кожен `INSERT INTO notification_outbox_audit` від воркера → 0 рядків відповідають policy → INSERT кидає / drop.
- кожен `SELECT … FROM owner_notification_targets WHERE location_id=$1` від воркера → 0 рядків → `no_target` для КОЖНОЇ події → **жодне Telegram-сповіщення не доставляється взагалі.**

**Порушений інваріант.** "Нуль тихих дропів" + "Telegram off critical-path, не каскад": FORCE без доведеного BYPASSRLS перетворює фікс BUG-B на повний outage notification-pipeline.

---

### BR-2 · B-CONSIST — BUG-A фікс не покриває `'sending'` ON-CONFLICT-no-op, і статус-нейм розсинхрон (`pref_off` vs `prefs_disabled`)
**Вектор:** B-CONSIST, B-OPS.
**Знахідка.** Дві діри у фіксі BUG-A: (1) CHECK додає `'pref_off'` замість `'prefs_disabled'` що реально пише код; (2) `AuditStatus`-union не містить нових статусів.
**Порушений інваріант.** "Нуль тихих дропів".

---

## HIGH

### BR-3 · B-SEC / B-CONSIST — non-order authority резолвер бере `rows[0]` (АРБІТРАРНА локація)
**Вектор:** B-SEC, B-CONSIST. Storefront-toggle через non-order гілку (`telegram-webhook.ts:158-173`) бере `rows[0]` без location-scope; chatId може бути active для N локацій. Закриває не ту локацію.

### BR-4 · B-CONSIST — гонка web-PUT prefs ↔ telegram /settings: read-merge-write (lost update)
**Вектор:** B-CONSIST. `notifications.ts:135-140` — read-merge-write на цілому jsonb. Concurrent toggle різних категорій від одного snapshot → lost update.

### BR-5 · B-SCALE — per-chat Telegram ліміт + in-memory rate-limiter не виживає мульти-інстанс.

### BR-6 · B-FAIL / B-DATA — held-черга: max-1-rehold + wraparound + відсутня `locations.timezone`.

---

## MEDIUM

### BR-7 · B-CONSIST — toggle-close не блокує order-insert; вікно = весь TTL відкритого меню.
### BR-8 · B-SEC — lenient webhook secret + replay = auth-bypass на мутацію.
### BR-9 · B-CONSIST — prefs backfill per-event→per-category губить кастомізацію/consent.
### BR-10 · B-FAIL — outbound fetch без timeout → каскад на pool.

## LOW
### BR-11 · B-DATA — retention DELETE без партицій на 52k/день.
### BR-12 · B-OPS — мертвий `ON CONFLICT` → дублі audit-метрик.

---
---

# РАУНД 2 — Регресія по resolution.md + нова атака

**Метод раунду 2:** кожен fix верифіковано проти робочого дерева. Несуча знахідка: **воркер і order-action шлях НЕ використовують `BEGIN`/`COMMIT`** — кожен `client.query()` на pooled-конекті виконується в autocommit (implicit single-statement transaction). Це підтверджено live:
- `workers/index.ts:92,187,310` — `this.db.connect()` → серія голих `client.query(...)` → `client.release()`. Жодного `BEGIN` у файлі (grep `BEGIN|COMMIT` → 0).
- `telegram-webhook.ts:227` — `set_config('app.current_tenant',$1,true)` потім окремим statement `updateOrderStatus(client,...)`; `orderStatusService.ts:31-83` — голі `client.query`, без `BEGIN` (grep → 0).
- `audit.ts:26-44` — `writeAudit` робить один `client.query(INSERT)`, без транзакції.

Цей факт — корінь регресії BR-1-fix і нової BR-13.

---

## Регресія BR-1…BR-12

### BR-1 fix — РЕГРЕСУВАВ (нова CRITICAL, див. BR-13). Механізм worker-escape GUC нереальний на autocommit-конекті.
Resolution каже: воркер виконує `SELECT set_config('app.notif_worker','on', true)` (transaction-local), потім SELECT targets / writeAudit у "тій же транзакції". **Транзакції немає.** На autocommit-конекті `set_config(...,true)` скоупиться до ЦЬОГО ОДНОГО statement і скидається одразу після його коміту. Наступний `SELECT … FROM owner_notification_targets` бачить `app.notif_worker` = порожньо → policy worker-escape-гілка = `current_setting('app.notif_worker',true)='on'` → false → під FORCE повертає 0 рядків. **Точно той самий outage, що BR-1 в раунді 1, лише тепер замаскований "механізмом", який мовчки не діє.** Доказ-патерн у коді: order-action на `telegram-webhook.ts:227` робить рівно те саме (`set_config(...,true)` окремим statement перед `updateOrderStatus`) і покладається на autocommit — тобто команда не доводить, що GUC переживає до наступного statement; навпаки, демонструє, що репо НЕ обгортає ці шляхи в транзакцію. → **BR-1 НЕ закритий.**

### BR-2 fix — ЗАКРИТИЙ (умовно). Канон `prefs_disabled` у CHECK + перелік 15 статусів.
Звірка: перелік статусів у resolution §BR-2 (`queued·sending·delivered·failed·archived·prefs_disabled·quiet_hours·held·no_target·dedup·circuit_open·rate_limited·target_inactive·order_not_found·unknown_event`) ⊇ множина, що реально пише код (`audit.ts:3-15` union + worker writes). CHECK тепер містить `'prefs_disabled'`. `AuditStatus`-union розширення позначено impl-кроком. Залишковий ризик НИЗЬКИЙ: union у `audit.ts` ще НЕ містить `held/queued/archived/unknown_event` (live: `audit.ts:3-15` без них) — якщо impl-крок забудуть, `writeAudit({status:'held'})` не скомпілюється; це compile-time, не runtime-дроп → не повертає BUG-A. Закрито з verification-gate на union-розширення.

### BR-3 fix — ЗАКРИТИЙ (з одним залишком, див. BR-15). `store.*:<locationId>` + явний scope.
Резолвер тепер `WHERE address=$chatId AND location_id=$locationIdFromCallback` (resolution §BR-3) ∩ membership. Підробка чужого `locationId` провалює target-lookup. Крос-tenant закрито. АЛЕ: `locationId` з callback_data все ще ВИКОРИСТОВУЄТЬСЯ для targeting у межах того самого чату — див. BR-15 (in-chat lateral).

### BR-4 fix — ЗАКРИТИЙ (механізм реальний). Atomic per-cell `jsonb_set` без SELECT.
`UPDATE … SET prefs=jsonb_set(prefs,'{operational}',…) WHERE id=$1` — кожен statement бере row-lock і читає свіжий committed image (read-committed). Concurrent toggle різних клітинок серіалізуються на row-lock, не втрачаються. Web PUT мігрує на цей шлях. Lost-update класу елімінований. Deadlock-ризик відсутній: обидва шляхи лочать ОДИН рядок (target id) у фіксованому порядку → серіалізація, не deadlock. **Закрито.** Застереження: цей шлях МАЄ бути в одній транзакції з prefs-audit INSERT (resolution §BR-4 це каже) — а патерн репо транзакцій не використовує (див. BR-13). Якщо impl напише обидва UPDATE+INSERT голими autocommit-statements, атомарність jsonb_set збережеться (per-statement row-lock), але atomicity "prefs+audit разом" зламається → див. BR-16.

### BR-5 — defer-flag, БЕЗ регресії. Документований `NOTIF_WORKER_SINGLETON` + scaling-gate.

### BR-6 fix — ЗАКРИТИЙ (схема+математика), з backfill-дірою (див. BR-14). `locations.timezone` колонка + wraparound.
Live: `grep timezone packages/db/migrations` → 0 (колонки ще нема — очікувано, design-time). Wraparound-математика специфікована коректно (`from<=to` денне vs `nowLocalMin>=from OR nowLocalMin<to` нічне). Закрито на рівні дизайну. Нова діра — backfill NULL timezone (BR-14).

### BR-7 fix — ЧАСТКОВО закритий. Soft-409 знімає UI-брехню, гонка лишається (accepted).
Resolution: order-create при `delivery_paused=true` → `409 store_paused`, один шлях у `orders.ts`. Live: `orders.ts` НЕ читає `delivery_paused` (grep → 0; лише `NOT_PUBLISHED` 409 на :135). Фікс реальний ЯКЩО impl додасть read+409. АЛЕ це read-then-insert БЕЗ транзакції/lock → гонка все ще є (BR-17). Hard-gate accepted-risk. UI-частина закрита, race-частина — нова MEDIUM.

### BR-8 fix — ЗАКРИТИЙ для мутацій (з лазівкою, див. BR-18). Strict secret на mutating callback.
Класифікація mutating (`order.`/`store.`/`settings.`) → header обов'язковий і має збігатись; absent∨mismatch → нуль мутації. Live-anchor `telegram-webhook.ts:34-47` зараз lenient — фікс ще не в коді (design-time). Механізм валідний. Лазівка — non-mutating lenient гілка + класифікація по data-prefix (BR-18).

### BR-9 fix — ЗАКРИТИЙ. AND-reduction backfill + `source='migration'` слід + старі ключі не видаляються.

### BR-10 fix — ЗАКРИТИЙ. AbortController 10s на кожен outbound fetch, у скоупі MVP (не optional).

### BR-11 — accept-risk з тригером, БЕЗ регресії.

### BR-12 fix — ЗАКРИТИЙ. Прибрано мертвий `ON CONFLICT`; метрика `DISTINCT ON (event,target_id)` останній статус.

---

## НОВІ знахідки BR-13+

### BR-13 · CRITICAL · B-SEC / B-FAIL — worker-escape GUC `app.notif_worker='on'` неробочий на autocommit; а якщо impl додасть `BEGIN` — будь-яка роль на пулі обходить tenant-RLS
**Вектор:** B-SEC (RLS-bypass / tenant-isolation), B-FAIL (outage), B-CONSIST.
**Знахідка (дві гілки, обидві ламаються).**

(A) **Як специфіковано (autocommit) — НЕ діє → outage.** Воркер: `connect()` → `set_config('app.notif_worker','on',true)` (statement 1, autocommit) → `SELECT targets` (statement 2). `true`-scope = transaction-local; на autocommit транзакція = один statement → GUC скинуто перед statement 2. Під FORCE policy `OR current_setting('app.notif_worker',true)='on'` бачить порожнє → 0 рядків. **= повний BR-1 outage, замаскований.** Доказ live: `workers/index.ts` не має `BEGIN`; патерн `telegram-webhook.ts:227` (set_config окремо від наступного query) це й демонструє.

(B) **Якщо impl "виправить" обгорнувши в транзакцію АБО використає `false`-scope (session-local) — RLS-bypass.** Щоб GUC дожив до SELECT targets, треба або `BEGIN; set_config(...,true); SELECT…; COMMIT`, або `set_config('app.notif_worker','on',false)` (session-scoped). Session-scoped на **спільному pg-pool** (`packages/db/src/index.ts:20` max=8; pg-boss на тому ж operational-конекшені per server.ts:283-285) → GUC ЛИШАЄТЬСЯ на конекті після `release()` → наступний орендар конекта (інший job, інший tenant-context) бачить `app.notif_worker='on'` → читає/пише ВСІ tenant-рядки повз RLS. Гірше: **операційний pool ділиться між API-роутами і pg-boss**? Перевірено — `server.ts:283-285` pg-boss бере окремий конект із session-URL (port 5432), API operational pool — інший. Але `ResetSession`/`DISCARD ALL` на release у коді НЕ знайдено (grep `RESET|DISCARD` → 0 у notif/webhook/pool). Тобто будь-який session-scoped GUC leak'ить між job-ами на boss-конекті. А якщо web-роут колись виконає `set_config('app.notif_worker','on')` (помилково чи зловмисно через інший injection-вектор) на operational-пулі — він обійде RLS на 3 нових таблицях.

**Сценарій поломки.** Гілка (A): деплой FORCE → нуль доставлених сповіщень (як BR-1). Гілка (B) з session-scope: job#1 (tenant A) ставить GUC, не очищає; конект повертається в пул; job#2 (tenant B) орендує ТОЙ САМИЙ конект, `app.notif_worker` ще `'on'` → job#2 пише/читає під escape-гілкою без перевірки membership; якщо в job#2 location_id приходить з ненадійного джерела — крос-tenant запис у audit/prefs-audit.
**Порушений інваріант.** RLS FORCE має ізолювати tenant. GUC-escape на пулі без гарантованого RESET = або outage (autocommit), або RLS-bypass (session-leak). Транзакційна ізоляція GUC передбачає транзакцію, якої в цих шляхах НЕМАЄ. **BR-1 НЕ закритий — регрес.**

### BR-14 · HIGH · B-DATA / B-FAIL — `locations.timezone` backfill: існуючі рядки беруть DEFAULT 'Europe/Tirane', але реальні не-албанські локації → quiet-hours зсунуті; NULL-семантики нема, бо NOT NULL DEFAULT затирає намір
**Вектор:** B-DATA (backfill intent), B-FAIL (quiet-вікно не той час).
**Знахідка.** `...049` додає `timezone text NOT NULL DEFAULT 'Europe/Tirane'`. На ADD COLUMN з NOT NULL DEFAULT **всі існуючі рядки отримують 'Europe/Tirane'** мовчки — немає way відрізнити "оператор справді в Тирані" від "ще не налаштовано". Якщо план рік-1 = multi-tenant ≤200 локацій (proposal §2), будь-яка локація поза UTC+1/+2 (Косово=UTC+1 ок, але майбутні EU-локації, або тестова з US-даними) отримує тихий зсув quiet-вікна на 1-N годин. Гірше: немає `NOT NULL`-нюансу — quiet-gating НЕ має fallback-гілки "timezone невідома → не тримати" (resolution §BR-6 math припускає валідну TZ). Якщо хтось вставить невалідний TZ-рядок (CHECK на валідність TZ відсутній — лише `text`), `now() AT TIME ZONE 'BadZone'` кидає → весь quiet-розрахунок у воркері падає → catch проковтує → подія або шлеться завжди, або дропається.
**Сценарій.** Оператор у Косові (UTC+1, але взимку Тирана теж UTC+1 — ок). Майбутня локація-партнер у Чорногорії влітку очікує 22:00 локально; DEFAULT 'Europe/Tirane' = той самий offset — пощастило. Але оператор вписує `timezone='Europe/Kyiv'` (UTC+2/+3) у власних налаштуваннях через майбутній UI → math коректна. Реальна діра: **немає валідації TZ-рядка** і **немає fallback при invalid/NULL** → один зіпсований рядок робить quiet-gating exception для всієї локації.
**Порушений інваріант.** Backfill зберігає намір (BR-9-клас). Тут DEFAULT нав'язує намір ('я в Тирані') усім існуючим рядкам без аудиту, і немає guard на invalid TZ.

### BR-15 · MEDIUM · B-SEC — `store.close:<locationId>` довіряє locationId з callback для in-chat lateral: оператор зі спільним чатом для лок A+B закриває B, маючи намір A — детермінізм відновлено, але targeting усе ще керується нетрастед-полем
**Вектор:** B-SEC (in-tenant lateral), B-CONSIST.
**Знахідка.** BR-3 fix робить authority детермінованою: `(chatId, locationId)` має бути active target ∩ membership(user, location). Це закриває крос-tenant. АЛЕ: якщо менеджер дійсно має membership на A І B (франшиза, спільний чат), `locationId` з callback_data проходить ВСІ перевірки для B. Кнопка несе `locationId`, але Telegram callback_data — це клієнтський payload, який атакувальник із доступом до чату (інший менеджер, скомпрометований пристрій) може підмінити `store.close:<B>` навіть тиснучи кнопку, надіслану для A. Перевіряється authority (має право на B), не intent (хотів A). Результат: закрито B замість A — той самий клас "мовчазна неправильна мутація власного бізнес-стану", що BR-3 раунд-1, але звужений до випадку спільного membership A∩B.
**Сценарій.** Менеджер у чаті, де бот шле алерти і для A, і для B. Підроблений/застарілий callback `store.close:<B>` проходить (membership на B є) → закриває не ту точку. Confirm-friction (Counsel 3-2) показує "Точно закрити приймання?" але НЕ показує ЯКУ локацію → менеджер підтверджує, закриває B.
**Порушений інваріант.** Targeting не має керуватись нетрастед callback-полем без echo-back ідентичності локації у confirm-крок. (Severity MEDIUM: вимагає спільного A∩B membership, не крос-tenant.)

### BR-16 · MEDIUM · B-CONSIST — prefs-зміна + prefs-audit "в одній транзакції", але шляхи репо не використовують BEGIN → consent-INSERT може закомітитись окремо від UPDATE (або навпаки)
**Вектор:** B-CONSIST (GDPR-слід розсинхрон).
**Знахідка.** Resolution §BR-4 і proposal §7 ("consent-INSERT падає → rollback всієї зміни") припускають, що prefs-UPDATE і prefs-audit-INSERT — в ОДНІЙ транзакції. Реальний патерн (`notifications.ts:135-141`, `workers`, `telegram-webhook`) — голі autocommit `client.query`, БЕЗ `BEGIN`/`COMMIT` (grep → 0). Якщо impl напише `UPDATE prefs; INSERT prefs_audit;` двома autocommit-statements — UPDATE комітиться НЕЗАЛЕЖНО. Якщо INSERT падає (CHECK на `source`/`category`, FK на target_id ON DELETE SET NULL race) → prefs змінено БЕЗ audit-сліду. Прямо порушує задекларований інваріант "no silent pref-change without consent record".
**Сценарій.** Telegram /settings toggle: `UPDATE prefs SET operational=false` коммітиться; `INSERT prefs_audit` кидає (напр. actor_user_id NULL на NOT-NULL-шляху, або CHECK-violation на category). Prefs змінено, audit нема → GDPR-слід втрачено мовчки.
**Порушений інваріант.** Atomicity prefs+audit. Залежить від транзакції, якої патерн репо не забезпечує.

### BR-17 · MEDIUM · B-CONSIST — soft-409 `store_paused` на order-insert: read-`delivery_paused`-then-insert без lock → гонка все ще пропускає замовлення в момент close
**Вектор:** B-CONSIST (TOCTOU).
**Знахідка.** BR-7 fix: order-create читає `delivery_paused` і повертає 409 якщо true. Це read-then-insert. Між `SELECT delivery_paused` і `INSERT INTO orders` оператор може закрити storefront (`UPDATE delivery_paused=true` з Telegram). Замовлення, що пройшло SELECT (false) до close, все одно вставиться після close. Вікно мале (мс), але існує — і важливіше: BR-7 ОРИГІНАЛ був про вікно = TTL відкритого меню (хвилини). Soft-409 переносить чек на момент submit, що ЗВУЖУЄ вікно до TOCTOU-мс — це покращення, не повне закриття. Resolution чесно маркує hard-gate як accept-risk R2'. Фіксую як ВІДКРИТУ MEDIUM (звужена, не закрита): дія "екстрено стоп" усе ще не authoritative на момент INSERT.
**Сценарій.** Обідній наплив: оператор тисне close о 20:00:00.000; клієнт сабмітить о 20:00:00.050; order-route прочитав `delivery_paused=false` о 20:00:00.040 (до коміту close) → вставляє замовлення о 20:00:00.060 попри close. Рідко, але "екстрено стоп" не дає гарантії.
**Порушений інваріант.** Accepted (R2'), severity коректна MEDIUM. Зазначаю, що навіть soft-409 не атомарний без `WHERE NOT delivery_paused` у самому INSERT.

### BR-18 · MEDIUM · B-SEC — strict-secret класифікація по `data`-prefix: non-mutating lenient гілка + ескалація мутації через нібито-read callback
**Вектор:** B-SEC (auth-bypass через класифікацію).
**Знахідка.** BR-8 fix: mutating = callback `data` починається `order.`/`store.`/`settings.`; решта (read-only/link-flow) лишається lenient за flag. Класифікація по STRING-PREFIX callback-payload, який контролює відправник. Дві діри:
1. **Будь-яка майбутня мутувальна дія з іншим префіксом** (напр. `shift.close`, `ack.`, `menu.`) випадає з allow-list → обробляється lenient-гілкою → auth-bypass. Allow-list = denylist-style (перелік mutating), а не "все мутувальне крім явного read" — новий mutating-handler без оновлення переліку проходить без secret.
2. **Link-flow lenient** (`/start <token>`) лишається без secret за flag. Якщо link-flow має будь-який побічний state-write (створення/активація target — а саме він і лінкує chat↔location), атакувальник без secret може зробити inbound `message` з `text:'/start <leaked_token>'` → лінкує СВІЙ chatId до чужої локації (якщо token leaked/guessable) → потім отримує всі сповіщення цієї локації. Lenient на link-flow = auth-bypass на сам акт лінкування, не лише на read.
**Сценарій.** Атакувальник дізнається webhook-URL + connect-token (короткий, з логів/підбору). Шле POST без secret-header, `message.text='/start <token>'`. Lenient link-гілка пропускає → його chatId стає active target локації → сповіщення з PII-мінімумом течуть йому, і він може тиснути mutating-кнопки (бо тепер chatId↔membership резолвиться).
**Порушений інваріант.** Strict secret має покривати кожен state-write inbound, не лише prefix-matched мутації. Класифікація по контрольованому payload — denylist, дірява за дизайном.

### BR-19 · LOW · B-CONSIST — confirm-friction на `store.close` обходиться повторним callback; підтвердження не має nonce/state
**Вектор:** B-CONSIST (friction-bypass), B-OPS.
**Знахідка.** Counsel 3-2: `store.close` → inline-confirm крок ("Точно закрити?"). Реалізація inline-confirm у Telegram = бот шле друге повідомлення з кнопкою `store.close.confirm:<loc>`. Callback_data не має TTL/nonce (proposal §8 R-секція визнає). Атакувальник/скрипт може надіслати `store.close.confirm:<loc>` НАПРЯМУ, минаючи перший крок — friction-bypass. Або replay старого confirm-callback закриває storefront знову. Friction захищає від "фантомного тапу в кишені" (Counsel 3-2 ціль), але НЕ від навмисного/реплейного обходу.
**Сценарій.** Скрипт із доступом до чату шле `callback_query{data:'store.close.confirm:<loc>'}` без проходження confirm-prompt → закрито без друкарського захисту. (Secret-strict BR-8 покриває auth, але confirm-friction як ОКРЕМИЙ захист нівелюється, бо confirm-крок stateless.)
**Порушений інваріант.** Confirm-friction має бути stateful (одноразовий nonce, прив'язаний до конкретного prompt), інакше це не friction, а лише зайвий крок для чесного користувача.

---

## Підсумок раунду 2

**Реально ЗАКРИТІ (механізм верифіковано):** BR-2, BR-3 (крос-tenant), BR-4, BR-6 (math/схема), BR-9, BR-10, BR-12. Defer-flag без регресії: BR-5, BR-11. Частково: BR-7 (UI-брехня закрита, гонка → BR-17).

**ВІДКРИТІ / РЕГРЕСУВАЛИ:**
- **BR-1 — НЕ закритий (РЕГРЕС → BR-13 CRITICAL).** Worker-escape GUC нереальний на autocommit-патерні репо; "виправлення" транзакцією/session-scope відкриває RLS-bypass на спільному пулі без RESET.
- BR-8 — закритий для prefix-мутацій, але класифікація denylist-дірява → BR-18.

**Нові BR-13+:**
- **BR-13 · CRITICAL** — GUC worker-escape: autocommit → outage, АБО session-leak на пулі без RESET → cross-tenant RLS-bypass.
- **BR-14 · HIGH** — `locations.timezone` NOT NULL DEFAULT затирає намір усім рядкам; нема валідації TZ → invalid-zone exception ламає quiet-gating локації.
- **BR-15 · MEDIUM** — `store.close:<locationId>` довіряє locationId з callback для in-chat lateral (спільний A∩B membership) + confirm не echo'ить локацію.
- **BR-16 · MEDIUM** — prefs-UPDATE + prefs-audit-INSERT не атомарні (патерн репо без BEGIN) → GDPR-слід може розсинхронитись.
- **BR-17 · MEDIUM** — soft-409 store_paused = TOCTOU read-then-insert без lock; "екстрено стоп" не authoritative на момент INSERT.
- **BR-18 · MEDIUM** — strict-secret класифікація по контрольованому `data`-prefix = denylist; новий mutating-handler або lenient link-flow → auth-bypass на state-write/лінкування.
- **BR-19 · LOW** — confirm-friction stateless → обходиться прямим `store.close.confirm` callback / replay.

---
---

# РАУНД 3 — РЕГРЕСІЯ на фікси Раунду 2 (BR-13…BR-19 + R-PRE)

**Date:** 2026-06-22 · **Метод:** READ-ONLY верифікація механізмів проти робочого дерева. Round-2 фікси ще design-only (нуль міграцій `notif_worker`/`telegram_action_nonces`/`pg_timezone_names`, нуль коду в репо — verified `grep notif_worker apps/ packages/` → 0). Тому регресую МЕХАНІЗМ у `proposal.md §5` + `resolution.md РАУНД 2`, не commited-код.

## Несучі live-факти (звірені)
- Notif-worker отримує **operational pool** (`server.ts:249,346` — `new NotificationWorker(pool,…)`, `pool=createOperationalPool()`), той самий що web-routes. `telegram-webhook` теж operational (`server.ts:632`).
- Operational pool = **Supabase transaction-pooler `:6543`** (`.env:4`, `config/src/index.ts:7`). pgbouncer **transaction-mode**.
- `owner_notification_targets` сьогодні **лише `ENABLE` RLS, НЕ `FORCE`** (`1780348982032:20`; жодного FORCE-рядка для неї в усіх міграціях — verified grep). Policy `USING` гейтить на `current_setting('request.jwt.claim.sub',true)`.
- **Жоден route не ставить `request.jwt.claim.sub`** (verified grep → 0 setters). Реальні setters ставлять `app.user_id` (`notifications/workers/index.ts`, `customer/push.ts`, `mock-auth.ts`).
- `owner/notifications.ts` web prefs-шлях: `db.connect()` → прямий SQL по `owner_notification_targets` (`:21,133,168`) **БЕЗ set_config / BEGIN / withTenant**. Працює сьогодні лише бо table не FORCE'd (owner-bypass).
- `INSERT INTO orders` — **рівно один шлях** (`orders.ts:597`, VALUES-форма). catch робить `ROLLBACK().catch(()=>{})` ПЕРЕД `release()` (`:776→785`). Це канонічний репо-патерн.
- `telegram-webhook.ts`: `set_config('app.current_tenant',$1,true)` ×3 на `:227/294/497`, `grep BEGIN → 0`. Pre-existing підтверджено.

---

## Регресія по пунктах завдання

### BR-13(1) — leak при throw/release без ROLLBACK → **ЗАКРИТО** (за умови operational-pool=txn-mode)
node-pg: помилка в транзакції → backend у `aborted`-state. `release()` без ROLLBACK повертає конект у pgbouncer; pgbouncer transaction-mode на `:6543` сам видає `ROLLBACK`/`DISCARD ALL` на server-конекті при поверненні в свій пул на межі транзакції → transaction-local (`true`) GUC гарантовано зникає. Навіть на direct-PG: `set_config(...,true)` прив'язаний до (під)транзакції, ABORT його прибирає до видачі наступному орендарю. Канонічний репо-патерн (`orders.ts:776`) усе одно робить explicit `ROLLBACK` у catch. Тому leak-через-release **не відтворюється**. Резолюція права. **ЗАКРИТО.**
> Застереження (не severity-bump): резолюція має ЯВНО написати, що нові 3 notif-шляхи копіюють патерн `orders.ts` (ROLLBACK у catch ПЕРЕД release) — `handleTelegramSend` сьогодні `finally{client.release()}` БЕЗ ROLLBACK у catch (`workers/index.ts:461-485`). На txn-pooler це безпечно, але DoD-тест нуль-leak має ганятися саме single-conn проти `:6543`, не проти direct-PG (інакше тест зелений, прод інший). Записано як impl-DoD-уточнення, не відкритий BR.

### BR-13(2) — web-конект легально піднімає `app.notif_worker='on'` у своїй транзакції → читає чужі tenant-и → **ВІДКРИТО-РЕГРЕС → BR-20 (HIGH)**
Escape-гілка `OR current_setting('app.notif_worker',true)='on'` — це **роль-сліпий backdoor на спільному пулі**. Будь-який web-код-шлях на operational-пулі (той самий `deliveryos_api_user`), що зробить `BEGIN; set_config('app.notif_worker','on',true); SELECT * FROM owner_notification_targets` — легально прочитає **всі локації всіх тенантів**. Немає role/grant-розрізнення web↔worker (резолюція сама визнала: опція (c) відкинута бо «той самий user»). FORCE захищає від чужого `request.jwt.claim.sub`, але НЕ від власного `app.notif_worker`. Один багнутий/зловмисний/скопійований-з-воркера web-шлях = повний крос-tenant read усіх owner-таргетів (PII: chatId, address). **Інваріант: tenant-ізоляція не має покладатися на дисципліну «web не ставить worker-GUC» — escape без role-розрізнення = self-grantable bypass.**

### BR-13(3) / FORCE+GUC-неузгодженість → **ВІДКРИТО-РЕГРЕС → BR-21 (CRITICAL)** — web prefs/read ламається ПІСЛЯ FORCE
Proposal §5.2 додає `ALTER TABLE owner_notification_targets FORCE ROW LEVEL SECURITY` + policy membership-гілка `current_setting('request.jwt.claim.sub',true)`. Live-факт: **(a) жоден route не ставить `request.jwt.claim.sub`** (verified 0 setters), **(b) web prefs-шлях `owner/notifications.ts` не ставить ЖОДНОГО GUC** і покладається на owner-bypass. Як тільки FORCE вмикається:
- membership-гілка: `request.jwt.claim.sub`=NULL → `WHERE user_id=NULL::uuid` → порожньо → **deny**;
- escape-гілка: `app.notif_worker` не виставлений у web → **deny**.
→ Web GET targets (`:21`) повертає **0 rows** (UI «у вас нема Telegram»), web prefs UPDATE (`:168`) → **0 rows affected** (тихо не зберігає, ще й BR-16 audit-INSERT під тим самим FORCE → теж 0/збій). **Read-after-write на prefs ламається повністю.** Це не leak — це outage web-площини, яку FORCE мав «захистити». Та сама причина, що BR-1 раунд-1 (хибний GUC), повторена в новій формі: policy читає GUC (`request.jwt.claim.sub`), який продакшн-код **не ставить**. **Інваріант: FORCE-policy мусить читати ТОЙ САМИЙ GUC, що його ставить реальний web-шлях (тут — `app.user_id` через `app_member_location_ids()`), а web-шлях має бути мігрований на встановлення цього GUC ПЕРЕД FORCE. Жодного з двох у дизайні нема.** DoD-тест §5.1 («SELECT з фейковим чужим `app.user_id`→0») перевіряє НЕ той GUC, що в policy → тест зелений, прод мертвий.
> Чому CRITICAL: повний outage owner-notifications web-керування + тихий data-loss на prefs-save при вмиканні FORCE-міграції; вмикається міграцією (forward-only), важко відкотити швидко.

### R-PRE — покриття всіх інстансів + lint-guard → **частково ЗАКРИТО, 1 ВІДКРИТО → BR-22 (HIGH)**
Три інстанси `:227/294/497` — verified повний перелік (`grep set_config telegram-webhook.ts` → рівно ці 3; жоден не пропущений). Обгортка покриє всі три. **АЛЕ:** резолюція R-PRE каже обгорнути в `BEGIN; set_config('app.user_id',<member>,true); set_config('app.current_tenant',$loc,true)`. Проблема: handler ставить `app.current_tenant`, але `updateOrderStatus`→orders-policy читає `app.user_id` (резолюція сама це визнала, anchor 2). Дизайн каже «ставити обидва» — ОК. Але **`<resolvedMember>` для `app.user_id` у webhook-контексті недовизначений**: callback приходить від Telegram chatId, не від JWT-user. Резолюція не специфікує, ЯКИЙ user_id ставити (target.user_id? будь-який member локації?) — а від цього залежить, чи orders-FORCE-policy пропустить `updateOrderStatus`. Якщо поставити `target.user_id`, а він не member цієї локації (disconnected target) → policy deny → status-update тихо 0 rows → бот каже «підтверджено», замовлення PENDING. **Інваріант: webhook-resolved `app.user_id` мусить бути доведеним active-member локації, інакше guarded order-transition fail-silent.** lint-guard «`set_config(...,true)` потребує BEGIN у блоці» — реальний і ловить клас, АЛЕ regex по тексту не доведе, що ставиться ПРАВИЛЬНИЙ GUC (`app.user_id` для orders-policy) — ловить «поза BEGIN», не «не той GUC».

### BR-17 — атомарний INSERT…SELECT WHERE delivery_paused=false → **ЗАКРИТО**
Один insert-шлях (`orders.ts:597`, verified — нема other `INSERT INTO orders`). Конверсія VALUES→`INSERT…SELECT FROM locations WHERE id=$loc AND delivery_paused=false` атомарна, read-committed бачить свіжий committed `delivery_paused`, 0 rows→409. Без extra lock, без extra конекта. TOCTOU реально елімінується (не звужується). Узгодженість шляхів ОК — шлях один. **ЗАКРИТО.**
> Impl-нота (не BR): зараз `location` читається раніше для розрахунку fee/tax (`orders.ts` use `location.currency_code` etc). INSERT…SELECT мусить тягнути ТІ САМІ поля з того ж рядка, інакше другий read = новий TOCTOU на іншому полі. Тривіально, але має бути в DoD.

### BR-18 — allowlist default-deny + `/start <token>` secret → **ЗАКРИТО (design), 1 застереження**
Інверсія denylist→allowlist read-only enum + link-flow завжди secret — закриває «новий префікс fail-open» і «лінкування без secret». Механізм реальний (Set-membership, default-deny). **ЗАКРИТО.**
> Застереження (impl-DoD, не BR): allowlist мусить матчити по **нормалізованому exact-action** (split `data` по `:` → беремо `store.close`, не весь payload), інакше атакувальник додає суфікс `noop:<mutating-payload>` щоб потрапити в lenient. Резолюція каже «exact callback action OR command» — добре, аби impl справді нормалізував до action-кореня, а не `startsWith`.

### BR-14/15/19 — швидка перевірка реальності → **ЗАКРИТО**
- **BR-14:** `CHECK (timezone IN (SELECT name FROM pg_timezone_names))` — `pg_timezone_names` реальний PG view; CHECK з підзапитом до immutable-набору валідний; NULLable+code-fallback+`quiet_tz_fallback`-audit реальні; незворотне не залежить від TZ. Механізм реальний. **ЗАКРИТО.**
- **BR-15:** echo `location.name` + `location_id` з nonce-store (server-side, не з callback) — реальний intent+targeting hardening. **ЗАКРИТО.**
- **BR-19:** `DELETE FROM telegram_action_nonces WHERE nonce=$1 AND expires_at>now() RETURNING location_id` — атомарний one-shot consume; прямий/replay/expired → 0 rows → reject. Механізм реальний. **ЗАКРИТО.**
> Застереження BR-19 (наслідок BR-21): `telegram_action_nonces` proposal §5.4 теж FORCE+`request.jwt.claim.sub`+`app.notif_worker`-escape. Той самий BR-21-дефект: webhook consume-DELETE біжить у webhook-handler, який не ставить `request.jwt.claim.sub`. Consume покладається на escape-гілку `app.notif_worker='on'` → значить webhook MUSIT ставити worker-GUC щоб видалити nonce → що повертає BR-20 (web/webhook легально піднімає escape). Кільце: nonce-consume не працює без escape, escape = self-grantable bypass. Покрито BR-20+BR-21.

---

## Таблиця Раунду 3

| BR | Severity | Статус | Суть |
|---|---|---|---|
| BR-13(1) leak-on-release | — | **ЗАКРИТО** | txn-pooler + repo ROLLBACK-у-catch патерн → transaction-local GUC не leak'ить |
| BR-13(2) self-grant escape | **HIGH** | **ВІДКРИТО → BR-20** | web на спільному пулі легально ставить `app.notif_worker='on'` → крос-tenant read; нема role-розрізнення |
| BR-13(3) FORCE+GUC mismatch | **CRITICAL** | **ВІДКРИТО → BR-21** | policy читає `request.jwt.claim.sub` (0 setters); web prefs-шлях не ставить GUC → FORCE→0 rows→outage+тихий prefs data-loss |
| R-PRE coverage | — | **ЗАКРИТО** (3/3 інстанси) | усі `:227/294/497` покриті обгорткою |
| R-PRE correctness | **HIGH** | **ВІДКРИТО → BR-22** | webhook `app.user_id`=`<resolvedMember>` недовизначений → orders-policy може fail-silent на order.confirm |
| BR-17 | — | **ЗАКРИТО** | один insert-шлях, атомарний INSERT…SELECT |
| BR-18 | — | **ЗАКРИТО** | allowlist default-deny + link-flow secret |
| BR-14/15/19 | — | **ЗАКРИТО** | TZ-CHECK / echo / one-shot nonce реальні |

## Нові знахідки Раунду 3

**BR-20 · HIGH · B-SEC** — Worker-escape GUC `app.notif_worker='on'` = self-grantable крос-tenant backdoor · Сценарій: web-шлях на operational-пулі (той самий `deliveryos_api_user`) робить `BEGIN; set_config('app.notif_worker','on',true); SELECT * FROM owner_notification_targets` → читає owner-таргети (chatId/address PII) ВСІХ тенантів; FORCE+`request.jwt.claim.sub` його не зупиняє · Порушений інваріант: tenant-ізоляція не сміє покладатися на «web не ставить worker-GUC» — escape без role/grant-розрізнення = self-grantable bypass на спільному пулі.

**BR-21 · CRITICAL · B-SEC/B-CONSIST/B-OPS** — FORCE-міграція вмикає policy на GUC, який жоден web-шлях не ставить → outage + тихий data-loss · Сценарій: `...049` робить `owner_notification_targets` FORCE; policy membership-гілка читає `current_setting('request.jwt.claim.sub')` (0 production setters, verified) і web `owner/notifications.ts` не ставить жодного GUC → після міграції GET targets=0 rows (UI «нема Telegram»), PUT prefs=0 rows affected (тихо не зберігає), prefs-audit INSERT під FORCE=deny · Число: 100% web owner-notification операцій ламаються в момент forward-only міграції · Порушений інваріант (read-after-write + RLS-GUC-узгодженість): FORCE-policy мусить читати ТОЙ САМИЙ GUC, що його ставить реальний web-шлях; web-шлях має бути мігрований на встановлення цього GUC ПЕРЕД FORCE. Жодного з двох у дизайні. DoD-тест §5.1 перевіряє `app.user_id`, а policy читає `request.jwt.claim.sub` → тест зелений, прод мертвий.

**BR-22 · HIGH · B-CONSIST** — webhook-resolved `app.user_id` для order-action недовизначений → guarded transition fail-silent · Сценарій: R-PRE-обгортка ставить `set_config('app.user_id',<resolvedMember>,true)`, але дизайн не специфікує джерело `<resolvedMember>`; якщо це `target.user_id` disconnected/non-member → orders-FORCE-policy deny → `updateOrderStatus` 0 rows, бот `answerCallbackQuery('confirmed')`, замовлення лишається PENDING до auto-cancel · Порушений інваріант: webhook `app.user_id` мусить бути доведеним active-member локації перед guarded order-transition, інакше rowcount=0 трактується як успіх (guard-on-rowcount порушено мовчки).

---
---

# РАУНД 4 — ФІНАЛЬНА РЕГРЕСІЯ (вузька) на структурне рішення role/pool (BR-20/21/22)

**Date:** 2026-06-22 · **Метод:** READ-ONLY верифікація проти робочого дерева. Регресую структурне рішення Раунду 3 (`resolution.md` РАУНД 3 + `proposal.md` §0/§2/§5/§8/§10). Code ще design-only (verified `grep deliveryos_notif_worker apps/ packages/` → 0, `grep createNotifWorkerPool` → 0) → регресую МЕХАНІЗМ.

## Несучі live-факти (звірені цього раунду)
- `locations` — **сьогодні `ENABLE`+`FORCE` RLS** (`core-identity.ts:84-85`), policy `tenant_isolation USING (id IN (SELECT app_member_location_ids()))` (`:86-87`). `app_member_location_ids()` → `app_current_user()` → `current_setting('app.user_id',true)` (`:70-79`). **`locations` FORCE — НЕ нова, вже у проді.**
- store-toggle `setAcceptingOrders` (proposal §C2 `:148-158`) = `UPDATE locations SET delivery_paused=$new WHERE id=$loc AND delivery_paused IS DISTINCT FROM $new RETURNING …`, на **operational-пулі** (web-роль `deliveryos_api_user`), викликається з webhook non-order гілки (`telegram-webhook.ts:158-194`).
- proposal `:154` + `:324`: **`rowCount===0` ⇒ "вже в цільовому стані ⇒ no-op (подвійний тап безпечний)"**, audit `store.toggle.noop`, бот рапортує успіх.
- BR-22 `GUARDED_NOOP`-fix (`resolution.md:500-509`, proposal `:374`) застосований **ЛИШЕ до `updateOrderStatus`** (order.confirm/reject). Store-toggle 0-rows у дизайні трактується протилежно — як успішний idempotent-noop.
- webhook non-order гілка (`:160-172`) резолвить `targetUserId` + membership (`:185-194`), АЛЕ для store-toggle дизайн НЕ специфікує `set_config('app.user_id', targetUserId)` перед `setAcceptingOrders` (BR-22 джерело визначене лише для order-handler; §C2 `:156` каже патерн `set_config('app.current_tenant')`, НЕ `app.user_id`).

---

## Регресія по пунктах завдання

### BR-20 (web структурно не може прочитати чужі owner-таргети) → **ЗАКРИТО**
Escape перенесений з self-settable GUC на атрибут окремої ролі `deliveryos_notif_worker` (verified BYPASSRLS, FATAL-on-missing boot-assert `resolution.md:457-462`); `app.notif_worker` ВИДАЛЕНО з усіх policy → web `set_config('app.notif_worker','on')` = no-op для RLS. Web-роль `deliveryos_api_user` не має BYPASSRLS (guardrail `db/index.ts:32`), не має `SET ROLE` (DoD-grep §5.1 п.4), не має escape-гілки в policy. Підробка `app.user_id` на web-конекті: web сам ставить GUC, АЛЕ зі свого автентикованого `request.user.sub` (`resolution.md:417`, патерн `customer/push.ts:35`) — value НЕ з input/callback; навіть якщо web поставить чужий `app.user_id`, `app_member_location_ids()` поверне ЛИШЕ локації, де той user реально active-member (`memberships`-таблиця, SECURITY DEFINER) → щоб прочитати чужого тенанта, треба вже мати чужу membership = не leak. Self-grantable bypass усунутий структурно. **ЗАКРИТО.**

### BR-21 (FORCE+GUC mismatch outage) → **ЗАКРИТО (з 1 ordering-застереженням, не BR)**
Policy 4 нових таблиць = канон `app.user_id` через `app_member_location_ids()` (рівно як `orders`/`locations`, `proposal.md:245`); `request.jwt.claim.sub` ВИДАЛЕНО. Web prefs-шлях `owner/notifications.ts` мігрує на `BEGIN; set_config('app.user_id', request.user.sub, true)` ПЕРЕД FORCE (`:247`). DoD §5.1 виправлений — тестує `app.user_id` (той GUC, що policy реально читає), п.4 grep `request.jwt.claim.sub`→0 (`resolution.md:425-431`). Forward-only порядок: enable-gate «web setter live ДО FORCE `...049`» + «DoD п.1+3 (web живий під FORCE) блокує деплой FORCE» (`resolution.md:559`). **ЗАКРИТО.**
> Застереження (impl-ordering, не BR): «code-deploy ДО міграції» — операційна обіцянка, не CI-гарантія. Якщо release_command (auto-migrate на push до main, per MEMORY «prod auto-migrates via release_command») запускає FORCE-міграцію В ТОМУ Ж деплої, що несе web-setter-код — міграція біжить ПЕРЕД тим, як новий образ став live (release_command виконується до swap інстансів). Вікно: міграція FORCE застосована → старий образ (без setter) ще обслуговує трафік до health-swap → web owner-notification 0-rows на це вікно (секунди-хвилини). Дизайн каже «live ДО», але механізм forward-only-в-одному-деплої цього НЕ гарантує без розщеплення на 2 деплої (setter-деплой N, FORCE-деплой N+1). **DoD §5.1 п.1+3 ловить це на staging ЯКЩО staging теж 2-фазний; якщо staging одно-деплойний — зелений staging, вікно в проді.** Записую як ordering-DoD-уточнення (власник Backend), не окремий BR — severity нижче HIGH (короткочасне, self-healing після swap, лише owner-notification-площина, не клієнт/гроші). Рекомендація: 2-деплойне розщеплення або explicit «FORCE-міграція в окремому release ПІСЛЯ підтвердження setter live».
> Rollback setter-коду ПІСЛЯ FORCE: якщо web-setter відкочується (revert PR) при ВЖЕ застосованому FORCE → web знову робить голий SQL без `app.user_id` → 0-rows → той самий BR-21 outage. Forward-only міграції не відкочуються, але КОД відкочується. → enable-gate має включати «FORCE-таблиці не можна NO FORCE назад швидко; rollback web-setter заборонений поки FORCE live». Це деплой-інваріант, не дизайн-діра — записую для Ops-runbook.

### BR-22 (order-action guarded transition) → **ЗАКРИТО для order-handler**
`<resolvedMember> := targetUserId` = вже-верифікований active-member (`telegram-webhook.ts:185-194` membership-перевірка, verified live); NULL-user order-mutation → reject; 0-rows `updateOrderStatus` → `GUARDED_NOOP` throw → бот рапортує помилку, не «confirmed» (`resolution.md:500-517`). Для accept/reject шляхів (вже-працюючих) узгоджено: вони йдуть через `updateOrderStatus`, який тепер кидає на 0-rows. **ЗАКРИТО для order-площини.**

---

## BR-23 · HIGH · B-CONSIST / B-SEC — store-toggle 0-rows ДВОЗНАЧНІСТЬ: idempotent-noop vs RLS-permission-deny НЕРОЗРІЗНЕННІ; BR-22 guard НЕ застосований до `setAcceptingOrders`

**Вектор:** B-CONSIST (guard-on-rowcount порушено на новій store-toggle дії), B-SEC (RLS-deny маскується під успіх).

**Знахідка (verified, нова двозначність, яку структурне рішення Раунду 3 ВВЕЛО, не закрило).**

`locations` — **FORCE RLS вже сьогодні** (`core-identity.ts:84-87`, policy `id IN (SELECT app_member_location_ids())` на `app.user_id`). Store-toggle `setAcceptingOrders` = `UPDATE locations … WHERE id=$loc AND delivery_paused IS DISTINCT FROM $new` — цей UPDATE проходить через FORCE RLS на `locations`. **rowCount=0 має ДВА структурно нерозрізненні джерела:**
1. **Legit idempotent-noop** — `delivery_paused` вже == `$new` (подвійний тап, replay store.open коли вже open). `WHERE … IS DISTINCT FROM $new` не матчить → 0 rows. Дизайн (`:154`, `:324`) трактує це як **успіх** (`store.toggle.noop`, бот каже «ОК»).
2. **RLS-permission-deny** — `app.user_id` не виставлений (або виставлений не-член) на webhook store-toggle конекті → policy `id IN (SELECT app_member_location_ids())` не матчить рядок → `UPDATE` повертає **0 rows БЕЗ throw** (RLS-deny не кидає — точно як BR-22 для orders). Стан НЕ змінений (storefront НЕ закрито), але бот рапортує **успіх** (бо 0-rows == «вже в цільовому стані»).

**Обидва = rowCount=0. Дизайн store-toggle трактує ОБИДВА як успіх.** BR-22-fix (`GUARDED_NOOP` throw на 0-rows) застосований ЛИШЕ до `updateOrderStatus` (order-handler) — `resolution.md:500-509` і proposal `:374` явно про order-action. `setAcceptingOrders` (§C2) **навмисно** трактує 0-rows як idempotent-success (`:154` «no-op, подвійний тап безпечний»). Дизайн НЕ може застосувати той самий `GUARDED_NOOP`-throw до store-toggle, бо там 0-rows ЛЕГІТИМНО означає double-tap — інакше кожен подвійний тап = помилка. Тобто два guarded-UPDATE шляхи трактують 0-rows **протилежно й несумісно**: order → 0-rows=fail; store → 0-rows=success. На store-toggle це робить RLS-deny **невидимим**.

**Гірше — store-toggle НЕ ставить `app.user_id`:** §C2 `:156` каже патерн `set_config('app.current_tenant')` (НЕ `app.user_id`); BR-22 джерело `app.user_id` визначене ЛИШЕ для order-handler (`:374`). `locations`-policy читає `app.user_id` (`core-identity.ts:72,76-79`), НЕ `app.current_tenant`. Тобто store-toggle на webhook-конекті, що ставить лише `app.current_tenant` (або нічого), → `app.user_id` порожній → `app_member_location_ids()` порожній → policy deny → **0 rows на КОЖНОМУ store.close/store.open** → бот завжди каже «ОК», storefront НІКОЛИ не закривається/відкривається. Це той самий клас, що BR-22 для orders, але на store-toggle він **тихий за дизайном** (0-rows=success), не ловиться guard.

**Сценарій поломки (демонстровний):**
- Пожежа/отруєння. Оператор тисне `store.close` у Telegram → confirm → `setAcceptingOrders(delivery_paused=true)`. Webhook store-toggle конект НЕ ставить `app.user_id` (дизайн його не вимагає для store-шляху). `locations` FORCE-policy `id IN app_member_location_ids()` з порожнім `app.user_id` → 0 rows. Бот: `store.toggle.noop` → показує «приймання на паузі ✅». **Storefront ЗАЛИШАЄТЬСЯ ВІДКРИТИМ.** Клієнти продовжують сабмітити (BR-17 INSERT…SELECT WHERE delivery_paused=false → delivery_paused все ще false → замовлення приймаються). Оператор переконаний, що закрив. «Екстрено стоп» тихо не спрацював, з позитивним підтвердженням боту.
- Навіть ЯКЩО store-toggle ставить `app.user_id=targetUserId` (як order-handler): non-member/disconnected/legacy-NULL target → 0 rows → бот «noop=ОК» → той самий тихий неуспіх. order-handler це ловить (`GUARDED_NOOP` throw); store-toggle НЕ може (0-rows=legit double-tap), тому НЕ ловить.

**Чому двозначність нерозв'язна тим самим механізмом:** щоб відрізнити idempotent-noop від permission-deny на одному `UPDATE … WHERE IS DISTINCT FROM`, треба ДОДАТКОВИЙ сигнал (напр. окремий `SELECT 1 FROM locations WHERE id=$loc` під тією ж policy ПЕРЕД/ПІСЛЯ: 0 rows = deny [рядок невидимий під RLS], 1 row = idempotent [рядок видимий, але вже в цільовому стані]). Дизайн §C2 цього НЕ робить — він колапсує обидва в «no-op success». Поки store-toggle на FORCE-таблиці `locations` без явного розрізнення «рядок видимий але не змінений» vs «рядок невидимий під RLS» — RLS-deny на store-close невидимий, а store-close = обіцянка безпеки оператора (пожежа).

**Порушений інваріант.** Guard-on-rowcount: 0-rows на guarded-UPDATE НЕ можна трактувати як успіх, поки не доведено, що рядок ВИДИМИЙ під RLS (тобто 0-rows = справді idempotent, а не permission-deny). Store-toggle на FORCE-таблиці колапсує deny і idempotent у один «success» → «екстрено стоп» fail-silent з позитивним підтвердженням. Це новий клас, симетричний BR-22, але на шляху, де BR-22-фікс структурно непридатний.

**Severity HIGH (не CRITICAL):** вимагає, щоб store-toggle конект не мав валідного `app.user_id`-члена (дизайн-пропуск §C2, який ще не специфікує setter для store-шляху) АБО non-member/legacy target. Не крос-tenant leak, не гроші. Але «екстрено стоп» тихо не спрацьовує з позитивним ботом-підтвердженням — пряма обіцянка безпеки оператора, тому не MEDIUM.

---

## BR-24 · MEDIUM · B-OPS / B-FAIL — BYPASSRLS на не-superuser Supabase-ролі НЕ доведений на staging; fallback `policy TO role` має той самий «спільний конект»-розрив, що дизайн не закрив

**Вектор:** B-OPS (scaling/infra-gate), B-ANTIPATTERN (accept як даність).

**Знахідка.** Дизайн ЧЕСНО позначив, що BYPASSRLS на не-superuser-ролі треба перевірити на staging (STOP-DESIGN-B `resolution.md:546-551`, R-ROLE `:557`) — це **закрито** на рівні чесності (не accept як даність; FATAL-on-missing boot-assert + FATAL-міграція замість swallowed-EXCEPTION). АЛЕ fallback **не доведений робочим**:
- Fallback при недоступному BYPASSRLS = `policy TO deliveryos_notif_worker` (role-based, `resolution.md:441,547`). Це вимагає, щоб notif-worker КОНЕКТИВСЯ роллю `deliveryos_notif_worker` — що дизайн і робить через окремий пул/credential. ОК для worker.
- АЛЕ дизайн каже (b) «технічно еквівалентний» — НЕ перевірено, що `policy TO role` на Supabase Supavisor (pgbouncer transaction-mode :6543/session :5432) поважає роль конекту. На pgbouncer роль визначається connection-string'ом пулу — окремий пул під окремою credential дає окрему backend-роль, тому `current_user`='deliveryos_notif_worker' має триматися. Це ПРАВДОПОДІБНО, але дизайн позначив (a) FATAL-on-missing assert, а для (b) fallback **немає еквівалентного boot-assert** «policy TO role реально пропускає мене» — лише припущення «еквівалентний». Якщо Supabase не дасть BYPASSRLS І `policy TO role` має edge (напр. роль не успадковує grants коректно, або Supavisor мультиплексує конекти ролей) → notif-worker під FORCE = 0-rows = той самий BR-1 outage, лише через fallback-гілку.
- **Спільний-конект розрив (нагадування):** дизайн каже окремий пул/credential для worker — добре. АЛЕ якщо fallback (b) колись поєднають з role-membership на СПІЛЬНОМУ конекті (`SET ROLE` на operational-пулі замість окремого пулу — дешевший shortcut під тиском) → escape знову self-grantable (web робить `SET ROLE deliveryos_notif_worker`). Дизайн НЕ заборонив явно `SET ROLE` на operational-пулі для fallback-гілки (DoD §5.1 п.4 grep `SET ROLE`→0 стосується escape, але fallback-(b) спокуса — окремий ризик).

**Сценарій.** Backend на staging виявляє: Supabase НЕ дає BYPASSRLS не-superuser-ролі (типово для managed Postgres). Падає на fallback (b) `policy TO role`. Boot-assert (a) `rolbypassrls=true` тепер FATAL-fail'иться (бо роль НЕ має BYPASSRLS у fallback-режимі) → треба окремий boot-assert для (b)-гілки, якого дизайн не специфікував. Якщо assert не адаптований → worker або не стартує (a-assert FATAL на b-режимі), або стартує без доказу, що `policy TO role` реально пропускає → ризик тихого 0-rows під FORCE.

**Порушений інваріант.** Verified-grant gate має покривати ОБИДВІ гілки (BYPASSRLS-роль І fallback `policy TO role`) — інакше fallback приймається як даність без доказу, що повторює BR-1-клас (escape припущений, не доведений). Дизайн позначив staging-перевірку для (a), але boot-assert/DoD для (b) — порожній.

**Severity MEDIUM:** дизайн чесно виніс на STOP-DESIGN-B (людський акт перед prod), FATAL-on-missing для (a) реальний; діра лише в тому, що (b)-fallback не має власного verified-gate. Ловиться на staging ПЕРШИМ кроком (як дизайн і вимагає) — тому MEDIUM, не HIGH. Записую щоб (b)-гілка не пройшла як «еквівалентна» без власного doдоказу.

---

## Таблиця Раунду 4

| BR | Severity | Статус | Суть |
|---|---|---|---|
| BR-20 self-grant escape | — | **ЗАКРИТО** | escape = атрибут окремої ролі; web `app.notif_worker`=no-op; web `app.user_id` з автентикованого sub, не з input; чужа membership потрібна = не leak |
| BR-21 FORCE+GUC mismatch | — | **ЗАКРИТО** | policy=канон `app.user_id`; web prefs setter ПЕРЕД FORCE; DoD тестує правильний GUC; +ordering/rollback-застереження для Ops-runbook (не BR) |
| BR-22 order-action guard | — | **ЗАКРИТО** (order-площина) | resolvedMember=верифікований targetUserId; NULL→reject; 0-rows→GUARDED_NOOP throw |
| **BR-23 store-toggle 0-rows** | **HIGH** | **ВІДКРИТО** | idempotent-noop і RLS-deny обидва=0-rows; дизайн store-toggle трактує 0-rows як success (`:154,:324`); BR-22 GUARDED_NOOP застосований ЛИШЕ до orders; store-toggle не ставить `app.user_id` (§C2 `:156` лише `app.current_tenant`); «екстрено стоп» fail-silent з позитивним ботом |
| **BR-24 fallback verified-gate** | **MEDIUM** | **ВІДКРИТО** | BYPASSRLS на не-superuser чесно на STOP-DESIGN-B, але fallback `policy TO role` не має власного boot-assert/DoD «реально пропускає» — приймається «еквівалентним» без доказу |

## Нові знахідки Раунду 4

**BR-23 · HIGH · B-CONSIST/B-SEC** — Store-toggle `setAcceptingOrders` 0-rows нерозрізненна двозначність (idempotent-noop vs RLS-permission-deny) · Сценарій: `locations` FORCE RLS (вже сьогодні, `core-identity.ts:84-87`, policy на `app.user_id`); store-toggle webhook-конект не ставить `app.user_id` (§C2 `:156` лише `app.current_tenant`; BR-22 setter лише для order-handler) → `UPDATE locations … WHERE IS DISTINCT FROM` під FORCE → 0 rows від RLS-deny → дизайн (`:154,:324`) трактує 0-rows як idempotent-success → бот «приймання на паузі ✅», storefront ЗАЛИШАЄТЬСЯ відкритим у пожежу; BR-22 `GUARDED_NOOP`-фікс непридатний (там 0-rows=legit double-tap) · Порушений інваріант: guard-on-rowcount — 0-rows на guarded-UPDATE не можна трактувати як успіх без доказу, що рядок ВИДИМИЙ під RLS (deny vs idempotent колапсовані в один success на FORCE-таблиці); store-close = обіцянка безпеки оператора, fail-silent неприйнятний.

**BR-24 · MEDIUM · B-OPS/B-ANTIPATTERN** — Fallback `policy TO deliveryos_notif_worker` (якщо Supabase не дає BYPASSRLS не-superuser-ролі) не має власного verified-gate · Сценарій: STOP-DESIGN-B чесно вимагає staging-перевірку (a)-гілки з FATAL-on-missing assert `rolbypassrls=true`; але (b)-fallback позначений лише «технічно еквівалентний» без власного boot-assert «policy TO role реально пропускає мене під FORCE» → якщо Supabase відмовляє в BYPASSRLS (типово для managed PG) і (b)-гілка має edge (role-grant inheritance / Supavisor multiplexing) → notif-worker 0-rows під FORCE = BR-1-клас outage через непротестований fallback · Порушений інваріант: verified-grant gate має покривати ОБИДВІ гілки; fallback не сміє прийматися «еквівалентним» без власного доказу. Severity MEDIUM — ловиться staging-першим-кроком як дизайн і вимагає; (a) має реальний FATAL-assert.

---

## Підсумок раунду 4

**Структурне рішення Раунду 3 ВИТРИМАЛО регресію по своїх трьох цілях:** BR-20 (self-grant escape) ЗАКРИТО структурно (роль, не GUC), BR-21 (FORCE+GUC mismatch) ЗАКРИТО (канон `app.user_id` + setter-перед-FORCE + виправлений DoD), BR-22 (order-action guard) ЗАКРИТО для order-площини. Корінь, який Ламач довів за 3 раунди, дизайн усунув структурно, не GUC-трюком.

**АЛЕ структурне рішення ВВЕЛО нову HIGH-двозначність на сусідній (store-toggle) площині, яку BR-22-фікс не покриває:**
- **BR-23 · HIGH · ВІДКРИТО** — store-toggle 0-rows колапсує idempotent-noop і RLS-permission-deny в один «success»; `GUARDED_NOOP`-патерн з BR-22 структурно непридатний до compare-set (там 0-rows=legit double-tap); store-toggle ще й не специфікує `app.user_id`-setter (§C2 лише `app.current_tenant`, а `locations`-policy читає `app.user_id`) → «екстрено стоп» fail-silent з позитивним підтвердженням боту. **Це нова відкрита HIGH — ціль 0 НЕ досягнута.**
- **BR-24 · MEDIUM · ВІДКРИТО** — fallback `policy TO role` без власного verified-gate.

**Відкритих CRITICAL = 0. Відкритих HIGH = 1 (BR-23). Відкритих MEDIUM = 1 (BR-24).** Структурне рішення витримало по своїх трьох цілях, але регресія знайшла, що `GUARDED_NOOP`-фікс BR-22 НЕ узгоджений з store-toggle-площиною — та сама 0-rows-семантика трактується двома guarded-шляхами протилежно й несумісно, і на store-close (обіцянка безпеки) deny невидимий. Потрібен ще один раунд на BR-23 ПЕРЕД нулем відкритих HIGH.

---
---

# РАУНД 5 — ФІНАЛЬНА ВУЗЬКА РЕГРЕСІЯ на BR-23 / BR-24 (resolution.md РАУНД 4 + proposal §C2/§5.1/§5.5/§8)

**Date:** 2026-06-22 · **Метод:** READ-ONLY верифікація + **ЕМПІРИЧНА перевірка SQL-семантики на живому PostgreSQL 16.14** (throwaway-кластер, точна репліка `core-identity.ts` RLS-ланцюга: FORCE на `locations`, policy `id IN (SELECT app_member_location_ids())` на `app.user_id`, не-власник роль). Механізм BR-23/24 — не «правдоподібний на віру», а **прогнаний**. Код ще design-only → регресую механізм + його SQL-семантику.

## Несучі live-факти (звірені цього раунду)
- `core-identity.ts:70-87` — `locations` **FORCE RLS вже сьогодні**, `app_current_user()`=`current_setting('app.user_id',true)` (`:72`), policy `id IN (SELECT app_member_location_ids())` (`:86-87`). **Підтверджено: store-toggle UPDATE проходить FORCE на `app.user_id`.**
- `proposal.md §C2 (:148-176)` — §C2 ПЕРЕПИСАНО на `cur FOR UPDATE`-CTE + канон `app.user_id`=targetUserId (NULL→reject). 3-way таблиця (`:166-170`) + доведення інваріантів (`:172-173`).
- `proposal.md §5.1 DoD (:228-234)` — додано п.7 (a/b/c/d) store-toggle deny-vs-noop; п.(c) deny=0-rows=fail блокує `TG_STOREFRONT_ACTION`.
- `proposal.md §5.5 (:338-339)` — fallback `policy TO role` отримав **functional** boot-assert (не атрибутивний); boot-логіка обирає assert за режимом; обидва-false→FATAL; `SET ROLE`-заборона поширена на fallback.
- `proposal.md §8 (:399-400)` — store-toggle authority секція + RLS FORCE на 4 таблицях узгоджені з BR-23/BR-20/BR-21.

---

## BR-23 — РЕГРЕСІЯ ПО СУТІ (механізм прогнаний на живому PG, не на віру)

### Sub-1: `FOR UPDATE` у CTE під FORCE на permission-deny → 0 rows, НЕ помилка/блокування? → **ЗАКРИТО (empirically)**
Прогнав на PG 16.14, не-власник роль, FORCE на `locations`:
- `app.user_id`=non-member → `WITH cur AS (SELECT … FOR UPDATE)` → `cur` **порожній**, UPDATE 0 rows, **БЕЗ помилки, БЕЗ блокування**. RLS USING-qual застосовується до `SELECT … FOR UPDATE` як до звичайного SELECT: невидимий рядок просто не потрапляє в результат, `FOR UPDATE` не лочить те, чого не бачить. Підтверджено: Ламач підозрював, що `FOR UPDATE` на невидимому рядку може кинути/заблокувати — **не кидає, не блокує, тихо 0 rows.** Резолюція права.
- `app.user_id`=порожній (не виставлений) → теж 0 rows (той самий шлях). Покриває «store-toggle конект не поставив `app.user_id`» сценарій BR-23 раунду-4 — тепер це 0-rows=deny=FAIL, не success.

### Sub-2: `UPDATE … FROM cur` коли `cur` порожній → rowCount=0 (а не unconditional update без джерела)? → **ЗАКРИТО (empirically)**
`UPDATE locations l SET delivery_paused=$new FROM cur WHERE l.id=$loc` — коли `cur` порожній, **0 rows** (join проти порожнього `cur` = порожній результат). Прогнано: non-member close → `(0 rows)`. **НЕ оновлює рядок без джерела** — `FROM cur` робить його conditional на наявності рядка в `cur`. Це й несе всю вагу: видимість `cur` (RLS-gated) = умова мутації. Ламач підозрював, що безумовний `SET` без `IS DISTINCT FROM` міг би оновити рядок навіть з порожнім `cur` — **ні: порожній `cur` = порожній join = 0 rows.** Резолюція права.

### Sub-3: `RETURNING cur.was` віддає СТАРЕ значення (cur матеріалізований ДО UPDATE)? → **ЗАКРИТО (empirically)**
Прогнано: member, `delivery_paused`=false, close → `was=f, now=t` (1 row). `cur` обчислений з pre-UPDATE row-image (CTE-snapshot перед мутацією тієї ж таблиці в тому ж statement), `l.delivery_paused` у RETURNING — post-UPDATE. **`was`=старе, `now`=нове, в одному рядку.** Це й дає 3-way: `was≠now`=changed, `was=now`=idempotent. Idempotent-прогон (вже true, close again) → `was=t, now=t` (1 row, НЕ 0 rows) — підтверджено, що idempotent НЕ колапсує в deny. Резолюція права на критичній деталі, яку сама ж і виправила (раунд-4 `:625` спершу мала `IS DISTINCT FROM` у UPDATE-WHERE, що колапсувало noop→0-rows; виправлена форма `:627-634` прибрала його — фінальна форма в proposal `:154-164` коректна).

### Sub-4: гонка FOR UPDATE-lock ↔ паралельний web PUT на той самий рядок (два канали) — серіалізуються, не deadlock? → **ЗАКРИТО (empirically)**
Прогнано 2 конкурентні транзакції на той самий рядок: S1 бере `FOR UPDATE`-lock, тримає 2с; S2 на тому ж рядку **блокується 1.63с**, потім бачить committed-значення S1 (`s2_was=t`), застосовує свій toggle. **Серіалізація на row-lock, нуль deadlock, last-writer-wins з консистентним RETURNING.** Web PUT мігрує на ТОЙ САМИЙ `setAcceptingOrders`-сервіс (proposal `:176`) → web↔telegram = той самий CTE на тому ж lock-порядку (один рядок) → нема lock-ordering deadlock-вектора (лочиться рівно один рядок `locations`).

### Sub-5: store-toggle ставить `app.user_id` у ТІЙ САМІЙ транзакції що CTE (інакше FOR UPDATE бачить 0→кожен toggle deny)? Узгоджено з BR-13 обгорткою? → **ЗАКРИТО (узгоджено)**
proposal `:176`: обгортка = `BEGIN; set_config('app.user_id', targetUserId, true); <CTE-UPDATE>; <publish>; COMMIT` — `set_config(...,true)` транзакційно-локальний, ставиться ПЕРШИМ statement у тій же транзакції що CTE → CTE бачить `app.user_id` member → `cur` непорожній для свого тенанта. Узгоджено з BR-13/Раунд-2 транзакційним патерном (web/courier `BEGIN`-перед-GUC; `customer/push.ts:35`, `courier/shifts.ts:23-24`). targetUserId = вже-верифікований active-member (`telegram-webhook.ts:160-194`, той самий резолвер що BR-22). **Прогнано непрямо:** scenario A/B (member у транзакції з GUC) → cur непорожній → changed/noop; scenario D (GUC не виставлений) → cur порожній → deny. Підтверджує: GUC у тій же транзакції обов'язковий, і дизайн його ставить.

### Sub-6: несуперечність канону — store-toggle на `app.user_id`, а не `app.current_tenant` (як openShift)? → **ЗАКРИТО (несуперечливо)**
Ламач (раунд-4) і завдання правильно вказали: `openShift`/courier-шлях ставить `app.current_tenant` (courier/settlement-таблиці читають його прямо). АЛЕ `locations`-policy читає `app.user_id` (`core-identity.ts:72`), НЕ `app.current_tenant` — два РІЗНІ канони, не взаємозамінні (proposal `:26` це фіксує). Тому store-toggle на `locations` МУСИТЬ ставити `app.user_id` — і резолюція раунду-4 саме це й робить (`:584-591`, proposal `:151`,`:399`). Раунд-1 §C2 «як openShift → `app.current_tenant`» був ХИБНИЙ для `locations` (читав не той GUC) — це й був корінь BR-23 п.1, **виправлено**. Несуперечність: courier-шлях лишається на `app.current_tenant` (його таблиці), store-шлях на `app.user_id` (`locations`-таблиця). Жодного конфлікту — кожна площина ставить GUC, який ЇЇ policy реально читає. **Узгоджено.**

**BR-23 ВЕРДИКТ: ЗАКРИТО.** Усі 6 sub-перевірок пройдені, 4 з них — емпірично на живому PG (FOR UPDATE-deny=0-rows-no-throw, empty-cur-UPDATE=0-rows, RETURNING cur.was=старе, race=serialize-no-deadlock). Канон `app.user_id` несуперечливий з courier-`app.current_tenant`. Guard-on-rowcount тепер консистентний на обох guarded-площинах: рядок невидимий (0-rows) = FAIL, ніколи success; idempotent-noop винесений в окремий сигнал (1 row + was=now).

---

## BR-24 — РЕГРЕСІЯ ПО СУТІ (fallback functional-assert прогнаний на живому PG)

### Sub-1: functional boot-assert «SELECT під FORCE без GUC» доводить, що worker пропускає? → **ЗАКРИТО (empirically)**
Прогнано на PG 16.14: роль `notif_polerole` БЕЗ BYPASSRLS (`rolbypassrls=f`), escape через `CREATE POLICY … TO notif_polerole USING(true)` → `SELECT count(*) FROM owner_notification_targets WHERE location_id=$seed` під FORCE, БЕЗ жодного GUC → **1 row** (бачить targets). Functional-assert (`SELECT 1 → рядок є, інакше FATAL`) реально доводить, що `policy TO role`-гілка пропускає роль під FORCE — не атрибутивно (`rolbypassrls=f` легітимно), а функціонально. Резолюція права: (b)-assert перевіряє РЕЗУЛЬТАТ, не атрибут.

### Sub-2: на BYPASSRLS-гілці той самий functional-assert НЕ дає false-negative? → **ЗАКРИТО (empirically)**
Прогнано: роль `notif_bypass` з BYPASSRLS (`rolbypassrls=t`) → той самий `SELECT count(*)` під FORCE без GUC → **1 row**. Functional-assert (рядок є) проходить і на (a)-гілці. Тобто навіть якщо boot-логіка помилково прогнала б (b)-functional-assert на BYPASSRLS-ролі — **не false-fail'нув би** (BYPASSRLS теж бачить рядок). Boot-логіка дизайну (`rolbypassrls=true`→(a); else→(b); both-false→FATAL, proposal `:339`) коректна на обох гілках; навіть деградована до «завжди functional-assert» вона лишилась би коректною (обидві ролі бачать рядок). Нема false-negative-вектора.

### Sub-3: role-bound escape — web-роль не дотягується до `TO role`-гілки? → **ЗАКРИТО (empirically + контроль)**
Контроль: web-роль (не той `current_user`, без membership, без GUC) під FORCE → **0 rows / permission-denied**, escape-policy `TO notif_polerole` для неї не застосовується (policy `TO role` прив'язана до `current_user` конекту). `policy TO role` поважає `current_user`, який на окремому пулі = окрема backend-роль (proposal `:339` `SET ROLE`-заборона тримає це: окремий пул/credential, не `SET ROLE` shortcut на operational). Self-grantable escape через fallback структурно неможливий — той самий клас гарантії, що BR-20 для BYPASSRLS-гілки.

**BR-24 ВЕРДИКТ: ЗАКРИТО.** Fallback `policy TO role` має власний **прогнаний** functional-assert (доводить escape під FORCE без GUC); той самий assert не false-fail'ить на BYPASSRLS-гілці; escape role-bound (web не дотягується). Обидві гілки покриті verified-gate + integration-тестом; жодна не приймається «еквівалентною» на віру.

---

## Таблиця Раунду 5

| BR | Severity | Статус | Підстава |
|---|---|---|---|
| BR-23 sub-1 FOR UPDATE-deny | — | **ЗАКРИТО** | empirically: невидимий рядок → cur порожній → 0 rows, БЕЗ throw/блокування |
| BR-23 sub-2 empty-cur UPDATE | — | **ЗАКРИТО** | empirically: `UPDATE … FROM cur` порожній cur → 0 rows (не unconditional update) |
| BR-23 sub-3 RETURNING cur.was | — | **ЗАКРИТО** | empirically: was=старе (pre-UPDATE), now=нове; idempotent=1row(was=now), не 0-rows |
| BR-23 sub-4 race two-channel | — | **ЗАКРИТО** | empirically: S2 блокується 1.63с на S1-lock, серіалізація, нуль deadlock, один рядок |
| BR-23 sub-5 GUC у тій же txn | — | **ЗАКРИТО** | proposal `:176` BEGIN+set_config(app.user_id,true)+CTE+COMMIT; узгоджено з BR-13 |
| BR-23 sub-6 канон несуперечність | — | **ЗАКРИТО** | locations-policy=app.user_id, courier=app.current_tenant; два канони, кожна площина ставить свій |
| BR-24 sub-1 fallback functional-assert | — | **ЗАКРИТО** | empirically: rolbypassrls=f + policy TO role → SELECT під FORCE без GUC → рядок є |
| BR-24 sub-2 no false-negative на bypass | — | **ЗАКРИТО** | empirically: rolbypassrls=t → той самий functional-assert теж бачить рядок |
| BR-24 sub-3 escape role-bound | — | **ЗАКРИТО** | empirically: web-роль (інший current_user) → 0/deny; policy TO role поважає current_user |

## Нові знахідки Раунду 5

**Нуль нових BR-25+.** Жодного нового вектора не введено фіксами BR-23/BR-24. Регресія була вузька (2 фікси) — кожен sub-механізм прогнаний на живому PostgreSQL 16.14 проти точної репліки prod RLS-ланцюга, не прийнятий на віру. SQL-семантика, яку Ламач підозрював (FOR UPDATE-deny кидає/блокує; empty-cur UPDATE оновлює без джерела; RETURNING cur.was віддає нове; race deadlock), **спростована емпірично** — поводиться рівно як резолюція стверджує.

## Підсумок раунду 5 (конспект)

- **Відкритих CRITICAL = 0. Відкритих HIGH = 0. Відкритих MEDIUM = 0. Відкритих LOW = 0.** Ціль (0 відкритих CRITICAL/HIGH) **досягнута**.
- **Нові BR-25+: НЕМАЄ.** Вузька регресія на BR-23/BR-24 не відкрила нових векторів.
- **BR-23 (раунд-4 HIGH) — ЗАКРИТО:** 6/6 sub-перевірок, 4 емпіричні. `cur FOR UPDATE`-CTE розрізняє deny(0-rows=FAIL+web-fallback) / idempotent-noop(1 row, was=now, не помилка) / changed(1 row, was≠now, «✅ HH:MM»). Канон `app.user_id` несуперечливий з courier-`app.current_tenant`. Дві guarded-площини консистентні: 0-rows=рядок невидимий=fail.
- **BR-24 (раунд-4 MEDIUM) — ЗАКРИТО:** fallback `policy TO role` має власний прогнаний functional boot-assert; не false-fail'ить на BYPASSRLS-гілці; escape role-bound (web не дотягується). Обидві гілки під verified-gate.
- **Регресія по ревізії:** усі попередньо-закриті (BR-1…BR-22) НЕ перевідкривались фіксами раунду-4 — фікси локальні (store-toggle SQL-форма + fallback-assert), нуль blast-radius на раніше-закриті механізми.

### ФІНАЛЬНИЙ ВЕРДИКТ (5 раундів)

**Дизайн ВИТРИМАВ 5 раундів змагальної атаки.** Траєкторія: раунд-1 знайшов хибне service-role-припущення (BR-1 CRITICAL) → раунд-2 довів autocommit-GUC нереальність + session-leak (BR-13) → раунд-3 довів self-grantable GUC-backdoor + FORCE/GUC-mismatch-outage (BR-20/21 — структурний корінь) → раунд-4 знайшов, що структурне рішення ВВЕЛО store-toggle 0-rows-двозначність на сусідній площині (BR-23) → раунд-5 емпірично підтвердив, що фікс BR-23/24 механічно коректний на живому PG.

Корінь (один пул/одна роль, два різні tenant-GUC `app.user_id`≠`app.current_tenant`, FORCE-семантика 0-rows) дизайн усунув **структурно** (окрема роль/пул) і **семантично** (CTE-visibility-розрізнення), не GUC-трюком — і остання форма прогнана на PostgreSQL, не на віру. Лишаються СВІДОМІ людські акти (НЕ дизайн-діри): **STOP-ETHICS-1** (підпис під `order.pending_aging` категоризацією) і **STOP-DESIGN-B** (staging-проба BYPASSRLS-доступності = ПЕРШИЙ деплой-крок, визначає (a)/(b)-гілку). Обидва — записані очікування підпису, з fail-safe за замовчуванням, не приховані пропуски.

**Готовність:** дизайн чистий для impl за умови виконання записаних enable-gate (web-setter live ДО FORCE-міграції; retention-cron ДО gating; STOP-DESIGN-B staging-проба; STOP-ETHICS-1 підпис) і DoD-гейтів (§5.1 п.1-7). Ціль 0 відкритих CRITICAL/HIGH/MEDIUM — досягнута.
