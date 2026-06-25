# RESOLVE — Telegram-сповіщення + дії

**System Architect (DeliveryOS)** · **Date:** 2026-06-22
**Inputs:** `proposal.md` (design) · `breaker-findings.md` (BR-1…BR-12) · `counsel-opinion.md` (1 умовний ETHICAL-STOP)
**Метод:** кожна знахідка верифікована проти робочого дерева перед вердиктом. Жоден fix не маркований «вирішено» без конкретного механізму (SQL / код-шлях / інваріант).

> Всі anchors нижче звірені live (не з proposal-таблиці):
> - `audit.ts:9,13-15` пише `'prefs_disabled'`, type-union без `held/pref_off/queued/archived` — **BR-2 підтверджено**.
> - `1780691681296_ops-location-alerts-policy.ts:5-12` — `ALTER ROLE … BYPASSRLS` у swallowed `EXCEPTION WHEN OTHERS` — **BR-1 підтверджено**.
> - `workers/index.ts:200-224` — SELECT targets + writeAudit **без** `set_config('app.current_tenant')` (лише `app.user_id` на іншому шляху, рядок 109) — **BR-1 підтверджено**.
> - `telegram-webhook.ts:159-172` — non-order гілка бере `rows[0]` без location-scope — **BR-3 підтверджено**.
> - `telegram-webhook.ts:43-46` — header-absent → process anyway — **BR-8 підтверджено**.
> - `owner/notifications.ts:135-140` — read-merge-write на цілому jsonb — **BR-4 підтверджено**.
> - `grep timezone` по `packages/db/migrations` → **0 файлів** — `locations.timezone` НЕ існує — **BR-6 підтверджено**.
> - `orders.ts` — `grep delivery_paused` → **0 matches** — order-insert не читає прапор — **BR-7 підтверджено**.
> - `event-registry.ts:59-79` — `order.timeout_cancelled`, `order.substitution_needs_human`, `cash.reconcile_discrepancy`, `delivery.flag_raised` усі `quietHours:'always'` — ETHICAL-STOP foundation підтверджено.
> - `workers/index.ts:387-392,415-420` — audit INSERT `ON CONFLICT DO NOTHING` без unique-constraint — **BR-12 підтверджено**.

---

## Таблиця вердиктів

| BR | Severity | Вердикт | Механізм (стислий) | Власник |
|---|---|---|---|---|
| BR-1 | CRITICAL | **fix** | Прибрати залежність від undocumented BYPASSRLS. Worker встановлює per-job `app.notif_worker='on'` GUC; кожна RLS-policy на 3 таблицях отримує `OR current_setting('app.notif_worker',true)='on'` гілку. Policy явна, верифікована, не залежить від суперюзер-grant. DoD-gate: `rolbypassrls`-перевірка більше не несуча. | Backend owner |
| BR-2 | CRITICAL | **fix** | Канонічний enum статусів зафіксовано (нижче). `audit.ts` `AuditStatus`-union розширюється тими ж рядками; CHECK і код пишуть **`'prefs_disabled'`** (не `'pref_off'`). Migration виправлено. | Backend owner |
| BR-3 | HIGH | **fix** | `store.*` callback несе `locationId`: `store.close:<locationId>`. Резолвер: target по `(chatId, locationId)` явно (не `rows[0]`) ∩ membership(user,location). Якщо немає → reject. Детермінізм відновлено. | Backend owner |
| BR-4 | HIGH | **fix** | Atomic per-cell на сервері: `UPDATE … SET prefs = jsonb_set(prefs,'{operational}',to_jsonb($v))` — окремий statement на КОЖНУ змінену категорію, без попереднього SELECT. Read-committed: кожен statement читає свіжий row-image під row-lock. Web PUT мігрує на цей же шлях (видаляє read-merge-write). | Backend owner |
| BR-5 | HIGH | **defer-flag (з фіксом-в-схему)** | MVP single-instance worker → in-memory rate-limit коректний (документований інваріант `NOTIF_WORKER_SINGLETON`). Перед horizontal scaling — `MISSING: distributed per-chat throttle` за flag `TG_WORKER_MULTI`. Scaling-gate: не масштабувати worker горизонтально поки прапор off. Burst у один чат — per-chat 1200ms + Telegram-side 429-retry (наявний retry-policy ловить). | Backend owner / Ops |
| BR-6 | HIGH | **fix** | (1) `...049` додає `locations.timezone text NOT NULL DEFAULT 'Europe/Tirane'`. (2) quiet-window math специфіковано з wraparound (from>to через північ) у local-TZ. Інваріант записано. | Backend owner |
| BR-7 | MEDIUM | **fix (UI-чесність) + accept-risk (hard-gate)** | Hard-gate на order-insert — окремий скоуп (accept-risk R2, власник Product+Arch). АЛЕ Counsel 3-1: order-create при `delivery_paused=true` повертає м'яке `409 store_paused` (не тихий прийом). Один код-шлях у `orders.ts`. UI більше не бреше. | Product+Arch / Backend |
| BR-8 | MEDIUM→**escalated** | **fix** | Strict secret-token ОБОВ'ЯЗКОВИЙ для мутувальних callback (`store.*`, order-actions, `/settings`-toggle). Header-absent ∨ mismatch на мутації → reject (answerCallbackQuery error, нуль мутації). Read-only inbound (link-flow) лишається lenient за flag до аудиту. | Backend owner |
| BR-9 | MEDIUM | **fix** | Backfill зберігає намір: per-event→category = AND-reduction (категорія OFF якщо БУДЬ-ЯКИЙ її event був явно false). Consent-log пише `migration`-source рядок про похідне значення. Rollback-сумісність: backfill НЕ видаляє старі ключі (gating під прапором off читає їх назад). | Backend owner |
| BR-10 | MEDIUM | **fix** | `AbortController` 10s на КОЖЕН outbound Telegram `fetch` (dispatcher + `callTelegramApi`/`sendMessage`). Timeout → `delivered:false, reason:'TIMEOUT'` → циркуляр-брейкер рахує. У скоупі MVP (не optional). | Backend owner |
| BR-11 | LOW | **accept-risk (з тригером)** | DELETE-retention на ~4.7M rows прийнятний @50лок; range-index на `created_at` обмежує scan. Тригер на партиціювання: `>200 лок` АБО audit-таблиця `>20M live rows` (метрика в Ops-dashboard). Власник тримає тригер. | Ops |
| BR-12 | LOW | **fix** | Прибрати мертвий `ON CONFLICT DO NOTHING` (немає unique → no-op clause бреше). Audit-рядки by design append-only — дублі при job-retry прийнятні, АЛЕ метрика `delivered`/`failed` рахує DISTINCT по `(event,target_id)` за вікно (остання спроба), не `count(*)`. Видаляє ілюзію ідемпотентності. | Backend owner |

**Підсумок:** fixed = **9** (BR-1,2,3,4,6,8,9,10,12 + BR-7 UI-частина) · accept-risk = **2** (BR-7 hard-gate, BR-11) · defer-flag = **1** (BR-5).
**Невирішених CRITICAL/HIGH = 0.** Усі CRITICAL (BR-1, BR-2) і всі HIGH (BR-3, BR-4, BR-5, BR-6) мають конкретний верифікований механізм; BR-5 свідомо defer-flag з schema-seam + scaling-gate + документованим singleton-інваріантом (не тихий пропуск).

---

## Деталі fix-механізмів (несучі)

### BR-1 — Worker-RLS без залежності від BYPASSRLS

**Проблема:** proposal стверджував «worker під service-role». Live: worker = `deliveryos_api_user`, RLS-bypass лише через swallowed `ALTER ROLE … BYPASSRLS`. FORCE без доведеного grant = повний notification outage.

**Fix (явна policy-гілка, не grant):**

`...048` (audit) і `...049` (targets), `...050` (consent) — кожна policy дістає worker-escape через GUC, який воркер виставляє per-job:

```sql
-- targets (приклад; так само audit, consent)
DROP POLICY IF EXISTS owner_notification_targets_owner_all ON owner_notification_targets;
CREATE POLICY ont_owner_all ON owner_notification_targets FOR ALL TO authenticated
  USING (
    location_id IN (SELECT location_id FROM memberships
       WHERE user_id = (current_setting('request.jwt.claim.sub', true))::uuid)
    OR current_setting('app.notif_worker', true) = 'on'   -- worker-escape
  );
```

Воркер на старті кожного job (перед SELECT targets / writeAudit) виконує:
```sql
SELECT set_config('app.notif_worker', 'on', true);   -- true = transaction-local
```
GUC `true`-scoped (transaction-local), не leak'ить між jobs/конектами. Не потребує суперюзера — будь-яка роль може `set_config` власну сесію. FORCE тепер безпечний: web → membership-гілка (tenant-ізольовано), worker → escape-гілка.

**Чому це краще за service-role конект:** не додає новий конект-пул (конект-бюджет §2 не зростає), не вимагає platform-grant, верифіковано локально (E2E: worker INSERT під FORCE проходить; web SELECT чужої локації → 0 rows).

**DoD-доказ (impl-stage):** integration-тест — підняти FORCE, виконати writeAudit з worker-GUC → рядок є; виконати той самий SELECT з фейковим `request.jwt.claim.sub` чужої локації → 0 rows. Без цього тесту gating НЕ вмикається.

### BR-2 — Канонічний enum статусів (зафіксовано)

**Єдиний канон (CHECK == `AuditStatus` union == код, що пише):**
```
queued · sending · delivered · failed · archived
prefs_disabled · quiet_hours · held · no_target · dedup
circuit_open · rate_limited · target_inactive · order_not_found · unknown_event
```
- Код **залишає** `'prefs_disabled'` (не перейменовуємо в `'pref_off'` — менший diff, нуль ризику пропустити call-site).
- `...048` CHECK містить **`'prefs_disabled'`** (виправлено з `'pref_off'`).
- `audit.ts` `AuditStatus`-union розширюється `+'held' +'queued' +'archived' +'unknown_event'` (impl-крок, явно в скоупі).

### BR-3 — store.* authority детермінований

`callback_data` для storefront = **`store.close:<locationId>`** / `store.open:<locationId>`. Резолвер:
```sql
SELECT ont.id, ont.user_id FROM owner_notification_targets ont
 WHERE ont.address=$chatId AND ont.channel='telegram' AND ont.status='active'
   AND ont.location_id=$locationIdFromCallback;   -- явний scope, НЕ rows[0]
-- 0 rows → 'Account not linked to this location' → reject
-- потім membership(user_id, locationId, active) → інакше reject
```
Authority = `(chatId↔target) ∩ (locationId присутній у callback) ∩ (membership user↔location)`. Кнопка, яку бот САМ надіслав у конкретний чат конкретної локації, несе свою `locationId` — підробка `locationId` чужої локації провалює target-lookup (немає `(chatId, чужий loc)` active-рядка). Нуль крос-tenant, нуль недетермінованого `rows[0]`.

### BR-4 — atomic per-cell prefs

Сервер (web PUT і Telegram /settings) пише КОЖНУ змінену категорію окремим statement, **без попереднього SELECT**:
```sql
UPDATE owner_notification_targets
   SET prefs = jsonb_set(prefs, '{operational}', to_jsonb($newVal::boolean))
 WHERE id=$1 AND location_id=$2;
```
Read-committed: кожен `UPDATE` бере row-lock і читає НАЙСВІЖІШИЙ committed image (не stale snapshot). Concurrent web(operational←false) ∥ telegram(quality←true) → два UPDATE різних клітинок серіалізуються на row-lock, кожен застосовується до результату попереднього → фінал `{operational:false, quality:true}`. Lost-update класу більше не існує. Web `notifications.ts:135-141` read-merge-write **видаляється**, замінюється циклом per-changed-category UPDATE у транзакції разом з consent-INSERT.

### BR-6 — timezone-колонка + wraparound (специфіковано)

`...049`:
```sql
ALTER TABLE locations ADD COLUMN IF NOT EXISTS timezone text NOT NULL DEFAULT 'Europe/Tirane';
```
Quiet-gating (інваріант, у воркері):
```
nowLocalMin = minutes-of-day у locations.timezone (через AT TIME ZONE)
isQuiet = (from <= to)  ?  (from <= nowLocalMin AND nowLocalMin < to)         -- денне вікно
                        :  (nowLocalMin >= from OR nowLocalMin < to)          -- wraparound через північ (22:00→08:00)
```
window-open для re-enqueue = найближчий момент, коли `nowLocalMin == to` у локальному TZ → конверт назад у UTC `startAfter`. Це знімає BR-6 п.1 (wraparound) і п.2 (TZ-зсув). Hardcoded `getUTCHours()` (`index.ts:219`) видаляється.

### BR-8 — strict secret для мутацій

Класифікація inbound:
- **Mutating** (`callback_query` з `data` що починається `order.`/`store.`/`settings.`; будь-яка inbound-команда `/open`,`/close`,`/settings`-toggle): **header обов'язковий і має збігатись**. Absent ∨ mismatch → НЕ виконувати мутацію (best-effort 200 до Telegram, але `answerCallbackQuery({text:'auth'})` + нуль state-change).
- **Read-only / link-flow**: lenient-гілка лишається за flag `TG_WEBHOOK_STRICT` (defer повного strict до аудиту всіх `setWebhook`).

Це знімає auth-bypass на `store.close` (BR-8 + BR-3 combo). Replay-DoS на toggle обмежений: ідемпотентний compare-set (close→close = no-op), а перший підроблений close блокується відсутністю валідного secret-token.

### BR-9 — backfill зберігає намір

```sql
-- operational OFF якщо БУДЬ-ЯКИЙ operational-event був явно false; інакше default true
UPDATE owner_notification_targets SET prefs = jsonb_build_object(
  'operational', NOT (prefs ?| ARRAY['order.pending_aging'] AND prefs->>'order.pending_aging' = 'false'),
  'quality', COALESCE((prefs->>'rating.low_received')::boolean, false)
) WHERE NOT (prefs ? 'operational');
```
(точна мапа event→category фіксується в impl разом з EVENT_REGISTRY `category`-полем; принцип — AND-reduction зберігає явне OFF). Старі ключі НЕ видаляються (rollback під flag off читає їх назад). Consent-log дістає `source='migration'` рядок про похідне значення (видимий слід зміни ефективних налаштувань — знімає «мовчазне скасування»).

### BR-10 — AbortController на outbound

```
const ac = new AbortController();
const t = setTimeout(() => ac.abort(), 10_000);
try { const r = await fetch(url, { signal: ac.signal, ... }); }
catch (e) { return { delivered:false, reason:'TIMEOUT' }; }   // circuit-breaker рахує
finally { clearTimeout(t); }
```
Network-blackhole більше не тримає job-slot/конект до OS-default. У скоупі MVP (не optional fix-list).

### BR-12 — прибрати мертвий ON CONFLICT, метрика DISTINCT

`ON CONFLICT DO NOTHING` видаляється (немає unique → бреше про ідемпотентність). Audit append-only; дублі при job-retry прийнятні. Метрика §9 рахує per `(event,target_id)` останній термінальний статус за вікно (window-function), не `count(*)` — алерт `failed-rate` не шумить на retry.

---

## ETHICAL-STOP — резолюція

**Counsel (умовний):** перекласифікація точок-неповернення у held = тихо забрати видимість незворотної втрати.

**Резолюція = design-time інваріант (знімає умову STOP на рівні дизайну) + один записаний людський акт перед prod-enable.**

Вписую в ADR і proposal явне речення-інваріант:

> **Інваріант категоризації:** категорія = **зворотність наслідку**, не гучність сповіщення. Будь-яка подія, наслідок якої незворотний у вікні тиші, КЛАСИФІКУЄТЬСЯ `transactional` (пробиває тишу, ніколи не held, не в `prefs`), незалежно від «операційного» тону.

**Канонічна мапа non-mutable `transactional` (лишаються `quietHours:'always'`, пробивають тишу, НЕ в prefs):**

| Event | Підстава незворотності |
|---|---|
| `order.created` | вікно дії на прийом/відмову закінчиться (auto-cancel) |
| `order.confirmed` / `order.rejected` | стан-перехід наслідку |
| `order.timeout_cancelled` | замовлення вже мертве — held доставив би некролог |
| `order.substitution_needs_human` | блокує fulfillment до рішення людини |
| `order.dwell_escalation` | вікно на дію закривається |
| `order.pending_aging` | aging → timeout-cancel якщо не діяти **(reclassify-perевірка: ЯКЩО власник свідомо хоче held — записати в ADR людиною)** |
| `order.ready_for_pickup` | вікно на pickup |
| `cash.reconcile_discrepancy` | грошова розбіжність — фінансова незворотність |
| `delivery.flag_raised` | скарга на безпеку доставки |
| `courier.assigned` | координація доставки в реальному часі |
| `ops.worker_liveness` / `ops.backup_failed` / `ops.degradation_changed` | системний збій — пробиває завжди |
| `shift.*` | operational-критичні для координації зміни |

**`operational` (default ON, можна held у тиші — наслідок зворотний):** — наразі **порожня** для order-flow; кандидати лише ті, де ранкова дія ще можлива (напр. майбутні info-only події). `order.pending_aging` НЕ йде сюди без записаного людського рішення.

**`quality` (default OFF, held у тиші):** `rating.low_received` (зворотний — відповісти на відгук можна вранці), `shift.close_reminder`.

**Чи лишається STOP для людини?** — **ТАК, один записаний акт (не блокер дизайну).** Перед `TG_CATEGORY_GATING=on` у проді Product+Arch письмово підтверджують цю мапу в ADR (особливо рядок `order.pending_aging` — єдиний реальний пограничний). Дизайн-інваріант знімає УМОВУ STOP'а (категорія≠гучність зафіксована); людський підпис на фінальній мапі лишається як `STOP-ETHICS-1` (одне речення в ADR, не переробка). Інваріант + порожній operational-order-flow означає: за замовчуванням НІЩО незворотне не held — навіть якщо людський акт забудуть, fail-safe = все transactional.

---

## Нон-блокінг поради Counsel — вердикти

| Порада | Вердикт | Дія |
|---|---|---|
| 3-1 UI не бреше «зачинено» | **fix** | BR-7: order-create при `delivery_paused` → `409 store_paused` (м'яко), не тихий прийом. У скоупі. |
| 3-2 Асиметричне тертя close (підтвердження) vs friction-free open | **fix** | `store.close:<loc>` → inline-confirm крок («Точно закрити приймання?»); `store.open:<loc>` — один тап. Пропорційність незворотності. |
| 3-3 `consent_log` → `prefs_audit` (over-claim) | **fix** | Перейменувати таблицю `notification_consent_log` → `notification_prefs_audit`; `/compliance` рядок «operator-action audit, не data-subject consent». |
| 3-4 Узгодити алфавіт категорій | **fix (доку)** | Слово = канон коду (`transactional/operational/quality`); 🔴🟠🟡 = дериватив для UI в одному місці. |
| 3-5 Retention-cron ДО gating-enable | **fix (ordering-gate)** | 90d cron має бути live ДО `TG_CATEGORY_GATING=on` (інакше BUG-A-фікс почне писати 52k/день у hotspot). Записано як enable-gate. |
| 3-6 R3 timezone не дати випасти | **fix** | Знято в BR-6 — `locations.timezone` у скоупі `...049`, не defer. |
| Steel-man B1 / ранковий шквал | **accept-risk (з тригером)** | MVP: голий re-enqueue без digest. Тригер на digest: якщо held-backlog ранковий burst >20/локацію за 1 годину (метрика) → переглянути digest. Власник Product. Інваріант категоризації вже відсікає незворотне → шквал = лише зворотні quality-події. |
| §5 право передати лінію (відкрите питання) | **defer (людське)** | Поза скоупом MVP; записано як відкрите продуктове питання `OPEN-Q-HANDOVER` (система сповіщень однієї людини vs передача чергування) — власник Product. Не блокує цей дизайн. |

---

## Оновлені accept-risk (іменовані власники)

| # | Ризик | Обґрунтування | Власник |
|---|---|---|---|
| R2' | `delivery_paused` не **hard**-блокує order-insert | Soft `409 store_paused` (BR-7 fix) усуває тихий прийом; hard-gate (stock/capacity-aware) = окремий скоуп | Product+Arch |
| R5/BR-11 | DELETE-retention без партицій | Прийнятно @50лок з range-index; тригер партиціювання `>200 лок` ∨ `>20M rows` | Ops |
| R-BR5 | In-memory rate-limit не мульти-інстанс | Документований singleton-інваріант `NOTIF_WORKER_SINGLETON`; scaling-gate блокує horizontal worker-scale поки `TG_WORKER_MULTI` off | Backend/Ops |

---
---

# РАУНД 2 — RESOLVE (BR-13…BR-19) + передумовний фікс

**Date:** 2026-06-22 · **Inputs:** `breaker-findings.md` (РАУНД 2: BR-13…BR-19 + регресія BR-1) · `counsel-opinion.md` (Раунд 2)
**Метод:** кожна знахідка верифікована проти робочого дерева ПЕРЕД вердиктом. Ключові live-anchors нижче — це факти, не цитати з resolution-таблиці.

## Live-верифікація несучих фактів (BR-13 ядро)

Розкопав повний RLS-ланцюг та транзакційний патерн. Висновки **переписують** BR-1-fix:

1. **Існує ДВА різних GUC, не один.** Канонічна tenant-RLS на `orders`/`locations`/`couriers`/`order_items` гейтиться через `app.user_id`, НЕ `app.current_tenant`:
   - `orders` policy (`1780310074262_orders.ts:82-84`): `FORCE` + `USING (location_id IN (SELECT app_member_location_ids()))`.
   - `app_member_location_ids()` (`core-identity.ts:76-79`, `SECURITY DEFINER`) → `WHERE user_id = app_current_user()`.
   - `app_current_user()` (`core-identity.ts:70-72`) → `current_setting('app.user_id', true)`.
   - `app.current_tenant` читається **прямо** лише courier/settlement-таблицями (`courier-shifts:18`, `couriers:32`, `settlement-audit:26` тощо).
   → **Два GUC — НЕ взаємозамінні.** `orders`-шлях залежить від `app.user_id`; courier-шлях від `app.current_tenant`.

2. **Транзакційний патерн РОЗДІЛЯЄТЬСЯ — і це реабілітує більшість репо, але викриває один вузький pre-existing баг:**
   - Web/courier guarded-шляхи **роблять `BEGIN`** перед transaction-local GUC: `courier/shifts.ts:23→24` (`BEGIN` потім `set_config(...,true)`), `spa-proxy.ts:420→421`, `orders.ts:105`, `mock-auth.ts:90→91`, усі courier-cron/dispatch/settlement. → у них `set_config(...,true)` КОРЕКТНИЙ (живе всю транзакцію).
   - **`telegram-webhook.ts` — ЄДИНИЙ доведено-зламаний інстанс.** Handler `handleCallbackQuery` бере `client = db.connect()` (рядок 101), і виконує `set_config('app.current_tenant',$1,true)` на рядках 227/294/497 **БЕЗ жодного `BEGIN`** (grep `BEGIN` у файлі → 0). Гірше: він ставить `app.current_tenant`, але `updateOrderStatus()` (`orderStatusService` → `orders` policy) читає `app.user_id`, який тут **взагалі не ставиться**.
   - Воркер (`workers/index.ts`) — autocommit, без `BEGIN` (verified рядки 92,187,310); ставить лише `app.user_id` (рядок 109) на customer-push шляху, не на targets/audit.

**Наслідок для дизайну (BR-13):** механізм worker-escape GUC `app.notif_worker` з `set_config(...,true)` на autocommit-воркері **дійсно нереальний** — Ламач правий. Але корінь глибший за «треба BEGIN»: репо вже має латентний баг класу «transaction-local GUC поза транзакцією» у `telegram-webhook.ts`. Обернути нові шляхи в `BEGIN` — половина рішення; інша половина — не вводити leak-вектор на спільному пулі.

---

## BR-13 · CRITICAL — РІШЕННЯ: явна транзакція-обгортка + `BEGIN;…;COMMIT` на воркер/dispatch-шляхах, GUC `set_config(...,true)` транзакційно-локальний, нуль session-leak

**Опції, які я зважив (verified проти коду):**

- **(a) Явна транзакція `BEGIN; set_config('app.notif_worker','on',true); …; COMMIT`** на 3 notif-шляхах (`handleTelegramSend`, `handleDispatch`, prefs-write, store-toggle). GUC `true`-scoped живе рівно до `COMMIT`, далі автоматично зникає при поверненні конекта в autocommit — **нуль leak без потреби в RESET**. Blast-radius: малий і ізольований (нові notif-job-и + `telegram-webhook` order-handler, який МАЄ бути обгорнутий усе одно для свого ж pre-existing багу). **ОБРАНО.**
- (b) Dedicated пул/роль з BYPASSRLS — **відкинуто:** додає конект-бюджет (порушує §2 «0 нових конектів»), вимагає platform-grant (той самий swallowed-EXCEPTION ризик, що BR-1 раунд-1), і фізично відокремити boss-пул від web — більша зміна, ніж того варта проблема.
- (c) Policy без GUC (по ролі/праву) — **відкинуто:** воркер = той самий `deliveryos_api_user`, що web; роль-based policy не розрізнила б їх без окремої ролі (=опція b). Немає дешевого role-розрізнення.
- (d) `DISCARD ALL`/`RESET ALL` на release-хуку — **відкинуто як основний механізм** (тримаю як defence-in-depth, нижче): покладатися на release-RESET означає, що ОДИН пропущений release (throw до `finally`, або майбутній код-шлях) leak'ить cross-tenant. Це fail-OPEN. Транзакційний scope fail-CLOSED: при будь-якому throw транзакція ABORT'иться і GUC зникає з конектом самим Postgres, без нашого хуку.

**Чому (a) не дає НІ outage, НІ leak (доказ):**
- **Не outage:** GUC ставиться першим statement ПІСЛЯ `BEGIN`, у тій самій транзакції що SELECT targets / writeAudit. На транзакційному конекті `true`-scope живе до `COMMIT` → policy-гілка `current_setting('app.notif_worker',true)='on'` бачить `'on'` для ВСІХ statements транзакції → FORCE пропускає worker-запис/читання. (На відміну від autocommit, де GUC жив один statement → це й була BR-13(A).)
- **Не leak:** при `COMMIT` (або ABORT при throw) транзакційно-локальний GUC скидається самим Postgres ДО повернення конекта в пул. Наступний орендар того ж конекта бачить `app.notif_worker` порожнім. Жодного `RESET`/`DISCARD` не потрібно — це властивість `set_config(...,true)`, а не нашого коду. (На відміну від `false`-scope (session), що й була BR-13(B) leak-вектором — ми його НЕ використовуємо.)

**DoD-gate (impl):** інтеграційний тест — (1) під FORCE, у транзакції з worker-GUC: writeAudit/SELECT targets проходить; (2) **після COMMIT, на ТОМУ САМОМУ фізичному конекті** (forced single-conn pool, max=1): новий `SELECT … FROM owner_notification_targets` БЕЗ нової транзакції/GUC → **0 rows** (доказ, що GUC не leak'нув). Без (2) gating НЕ вмикається. Це прямо ловить BR-13(B).

**Defence-in-depth (не основний механізм):** додати `client.query('DISCARD ALL')` у release-обгортку notif-конектів НЕ потрібно при транзакційному scope, але ми додаємо **boot-time assert**: при старті воркера один раз `SHOW app.notif_worker` на свіжому конекті має бути порожнім — sanity, що жоден шлях не ставить session-scoped GUC помилково.

## BR-13' · ПЕРЕДУМОВНИЙ ФІКС (ширший, pre-existing) — `telegram-webhook` order-handler ставить transaction-local GUC поза транзакцією

**Знахідка (verified, поза цим дизайном, але блокує його коректність):** `telegram-webhook.ts:101` бере конект, `:227/:294/:497` ставлять `set_config('app.current_tenant',$1,true)` **без `BEGIN`**. На транзакційному (autocommit) конекті GUC живе один statement → наступний `updateOrderStatus`/`openShift` біжить без нього. Додатково handler НЕ ставить `app.user_id`, від якого реально залежить `orders` FORCE-policy. Тобто **наявний Telegram order.confirm/reject шлях уже працює на хибному припущенні транзакційності** — рівно той патерн, що BR-13 в новому коді.

**Вердикт:** **defer-flag → передумовний фікс, ВЛАСНИК Backend, окремий PR ПЕРЕД вмиканням `TG_STOREFRONT_ACTION`.** Цей дизайн вводить нову мутацію (`store.close`) у ТОЙ САМИЙ handler — він МУСИТЬ бути обгорнутий у `BEGIN; set_config('app.user_id', <resolvedMember>, true); set_config('app.current_tenant', $loc, true); …; COMMIT`. Оскільки фікс нових дій вимагає виправити обгортку handler'а, pre-existing order-action баг закривається тим самим рухом. **Позначено як ризик R-PRE (нижче) з власником і явним gate.** Чому defer-як-передумова, а не «тихо полагодити»: це поза скоупом notif-дизайну (це order-action RLS), але дизайн на нього спирається → робимо явним передумовним кроком, не приховуємо.

> **Чи це ширший repo-wide баг?** Перевірено: courier/web/cron-шляхи РОБЛЯТЬ `BEGIN` (verified `courier/shifts.ts:23`, `spa-proxy.ts:420`, всі cron). Доведено-зламаний інстанс ОДИН — `telegram-webhook.ts`. Тому це **вузький pre-existing баг, не repo-wide катастрофа**. Але клас («transaction-local GUC без транзакції») реальний → додаю lint-guard як accept-risk-тригер (нижче R-PRE).

## BR-14 · HIGH — РІШЕННЯ: TZ-валідація через `pg_timezone_names` FK-стиль CHECK + NULLable з code-fallback+audit, НЕ NOT NULL DEFAULT

**Fix:** замінити `timezone text NOT NULL DEFAULT 'Europe/Tirane'` на:
```sql
ALTER TABLE locations ADD COLUMN IF NOT EXISTS timezone text;   -- NULLable: NULL = "не налаштовано", не мовчазний намір
-- валідація: лише назви, що Postgres реально знає (інакше AT TIME ZONE кидає)
ALTER TABLE locations ADD CONSTRAINT locations_timezone_valid
  CHECK (timezone IS NULL OR timezone IN (SELECT name FROM pg_timezone_names));
```
- **NULLable, не DEFAULT:** ADD COLUMN з NOT NULL DEFAULT мовчки нав'язує «я в Тирані» УСІМ існуючим рядкам без сліду. NULL чесно каже «невідомо». Backfill — окремим явним `UPDATE … SET timezone='Europe/Tirane'` ТІЛЬКИ для пілотних албанських локацій (з `notification_prefs_audit source='migration'` слідом), не німим DEFAULT.
- **Code-fallback (quiet-gating):** `tz = location.timezone ?? 'Europe/Tirane'` у воркері, з `audit status='quiet_tz_fallback'` коли fallback спрацював → видимий слід «локація без TZ рахується по дефолту». Незворотне (transactional) НЕ залежить від TZ узагалі (пробиває завжди) → invalid/NULL TZ НЕ може заглушити 🔴.
- **CHECK проти exception:** `pg_timezone_names`-CHECK гарантує, що `now() AT TIME ZONE locations.timezone` ніколи не кине на committed-рядку → quiet-розрахунок не падає. (`pg_timezone_names` — view, не FK-таблиця; CHECK з підзапитом валідний у Postgres для immutable-набору зон.)

> Застереження impl: CHECK з підзапитом до `pg_timezone_names` оцінюється при INSERT/UPDATE; набір зон стабільний у межах major-версії PG. Прийнятно. Альтернатива (тригер) — over-engineering для цього.

**Вердикт BR-14: fix.** NULLable + CHECK + code-fallback-з-аудитом. DEFAULT прибрано (був мовчазний намір).

## BR-15 · MEDIUM — РІШЕННЯ: confirm-крок echo'ить ім'я локації + nonce (закривається разом з BR-19)

**Fix:** inline-confirm на `store.close` показує **`Закрити приймання для «{location.name}»? Клієнти бачитимуть пауза.`** — echo конкретної локації з `locationId`, який пройшов authority-резолвер. Менеджер зі спільним A∩B membership бачить, ЯКУ точку закриває → intent-перевірка людиною, не лише authority-перевірка машиною. Targeting усе ще несе `locationId` з callback (BR-3 закрив крос-tenant), але echo додає human-in-loop intent-confirm для in-tenant lateral. **Вердикт: fix** (echo) + механічний replay-захист у BR-19.

## BR-16 · MEDIUM — ЗАКРИВАЄТЬСЯ BR-13-fix-ом (одна транзакція на prefs+audit)

**Узгодження:** BR-13-fix вводить `BEGIN;…;COMMIT` на prefs-write шляху (web PUT і Telegram /settings). prefs-`jsonb_set`-UPDATE і `notification_prefs_audit`-INSERT ідуть у ТІЙ САМІЙ транзакції → атомарні: INSERT падає → ROLLBACK всього UPDATE → «no silent pref-change without audit record» тримається реально, не на припущенні. **Вердикт: fix (успадкований від BR-13).** Явно: impl-DoD — тест, що CHECK-violation на `notification_prefs_audit.category` відкочує prefs-UPDATE (prefs незмінені після збою INSERT).

## BR-17 · MEDIUM — РІШЕННЯ: атомарний guard у самому INSERT (`WHERE NOT delivery_paused`), не accept-risk

**Fix (підсилення BR-7):** замість read-`delivery_paused`-then-insert (TOCTOU вікно), переносимо чек у сам INSERT як conditional-write:
```sql
INSERT INTO orders (...)
SELECT ... FROM locations l
 WHERE l.id = $loc AND l.delivery_paused = false;
-- 0 rows inserted → 409 store_paused (атомарно, без вікна)
```
Read-committed: INSERT…SELECT бере свіжий committed image `delivery_paused` у момент вставки під той самий lock-порядок, що `UPDATE delivery_paused=true` від Telegram-close. Якщо close закомітився ДО INSERT → `SELECT` бачить true → 0 рядків → 409. Вікно TOCTOU **елімінується** (не звужується). «Екстрено стоп» тепер authoritative на момент INSERT: жодне замовлення не проскочить після committed-close. **Вердикт: fix** (було accept-risk R2', тепер закрито атомарним INSERT-guard). Hard capacity-gate (stock-aware) лишається окремим скоупом — це інша річ, не race.

> Чому не accept-risk: «екстрено стоп» — обіцянка безпеки оператора (пожежа/отруєння). «INSERT проскочить у вікні мс» неприйнятно для цієї обіцянки. Атомарний INSERT…SELECT коштує нуль додаткових конектів і закриває вікно повністю → fix, не accept.

## BR-18 · MEDIUM — РІШЕННЯ: allowlist (default-deny на ВСЕ мутувальне/лінкувальне), lenient лише для явного read-only enum

**Fix (інверсія класифікації):** замість denylist-префіксів mutating, робимо **allowlist read-only**:
```
INBOUND_LENIENT = Set<exact callback action OR command>{  // явний, вичерпний перелік read-only
  'noop', 'help', 'menu.view', 'status.view'   // приклади; розширюється ЯВНО
}
isMutatingOrLinking(inbound) = NOT INBOUND_LENIENT.has(action)   // default: вимагає secret
```
- Будь-який inbound, що НЕ в явному read-only allowlist → **secret обов'язковий і має збігатись** (включно з невідомими/майбутніми префіксами `shift.`, `ack.`, новими mutating-handler'ами — вони fail-CLOSED).
- **Link-flow (`/start <token>`) — ЗАВЖДИ потребує secret** (це state-write: лінкує chat↔location). Прибрано з lenient. Атакувальник без secret не може зробити `/start <token>` inbound → не залінкує свій chatId.
- Lenient-flag `TG_WEBHOOK_STRICT` керує лише поведінкою для allowlist-read-only гілки (backward-compat для legacy read commands), НІКОЛИ для mutating/linking.

**Вердикт BR-18: fix.** Default-deny allowlist. Новий mutating-handler без оновлення allowlist → fail-closed (вимагає secret), не bypass. Знімає обидві діри (новий префікс + link-flow).

## BR-19 · LOW — РІШЕННЯ: stateful nonce з TTL на confirm-крок

**Fix:** `store.close` (крок 1) генерує одноразовий `nonce = gen_random_uuid()`, пише `(nonce, location_id, action='store.close', expires_at=now()+'2 min')` у короткоживучу таблицю `telegram_action_nonces` (або pg-boss-backed; найдешевше — нова дрібна таблиця з TTL-cleanup на тому ж 90d-cron). Confirm-кнопка несе `store.close.confirm:<nonce>`. Крок 2:
```sql
DELETE FROM telegram_action_nonces
 WHERE nonce=$1 AND action='store.close' AND expires_at > now()
 RETURNING location_id;   -- 0 rows → expired/replayed/forged → reject, нуль мутації
```
Атомарний `DELETE … RETURNING` = одноразовість (consume): прямий `store.close.confirm:<nonce>` без проходження кроку 1 → nonce не існує → reject; replay того ж nonce → вже видалений → reject; протермінований → `expires_at` фільтрує. **Вердикт: fix.** Confirm стає stateful one-shot, не зайвий-крок-для-чесного. `location_id` повертається з nonce-таблиці (server-side), не з callback → ще й підсилює BR-15 (targeting з trusted-store, не з callback-payload).

> Конект-бюджет: nonce-таблиця — наявний пул, transient рядки (~6/лок/день × 50 = 300 живих максимум, TTL 2хв → одиниці в будь-який момент). Нуль нових конектів. Schema-seam дешевий.

---

## Таблиця вердиктів — Раунд 2

| BR | Severity | Вердикт | Механізм (1 рядок) |
|---|---|---|---|
| BR-13 | CRITICAL | **fix** | Явна `BEGIN; set_config('app.notif_worker','on',true); …; COMMIT` на notif/dispatch/prefs/toggle-шляхах. GUC живе всю транзакцію (не outage), скидається при COMMIT/ABORT самим PG (не leak). НЕ session-scope, НЕ покладаємось на release-RESET. DoD: single-conn тест доводить нуль leak. |
| BR-13' | CRITICAL→**передумова** | **defer-flag (передумовний фікс, gate перед prod)** | Pre-existing: `telegram-webhook.ts:227/294/497` ставить transaction-local GUC без BEGIN + не ставить `app.user_id` (від якого залежить orders-policy). Цей дизайн вводить нову мутацію в той самий handler → МУСИТЬ обгорнути в BEGIN+обидва GUC. Закривається тим самим PR. Власник Backend, gate перед `TG_STOREFRONT_ACTION`. |
| BR-14 | HIGH | **fix** | `timezone` NULLable (не NOT NULL DEFAULT) + CHECK проти `pg_timezone_names` + code-fallback `?? 'Europe/Tirane'` з `quiet_tz_fallback`-audit. Незворотне не залежить від TZ. |
| BR-15 | MEDIUM | **fix** | Confirm echo'ить `location.name`; `location_id` береться з nonce-store (BR-19), не з callback → intent-confirm людиною на in-tenant lateral. |
| BR-16 | MEDIUM | **fix (успадк. BR-13)** | prefs-UPDATE + prefs-audit-INSERT у тій самій BEGIN-транзакції що BR-13 → атомарні; INSERT-збій → ROLLBACK UPDATE. DoD-тест на CHECK-violation rollback. |
| BR-17 | MEDIUM | **fix** | Атомарний `INSERT … SELECT … WHERE delivery_paused=false` → 0 rows=409. TOCTOU-вікно елімінується (не звужується). «Екстрено стоп» authoritative на момент INSERT. |
| BR-18 | MEDIUM | **fix** | Allowlist default-deny: secret обов'язковий для ВСЬОГО, крім явного read-only enum; link-flow завжди потребує secret. Новий mutating-handler fail-closed. |
| BR-19 | LOW | **fix** | One-shot nonce `(nonce,action,expires_at)` + `DELETE…RETURNING` consume. Прямий/replay/expired confirm → reject. |

**Підсумок раунду 2:** fixed = **7** (BR-13, BR-14, BR-15, BR-16, BR-17, BR-18, BR-19) · передумовний-фікс-з-gate = **1** (BR-13', закривається разом з BR-13-PR) · accept-risk нових = **0**.

**Відкритих CRITICAL = 0. Відкритих HIGH = 0.** BR-13 має конкретний транзакційний механізм з доведеним нуль-outage + нуль-leak (single-conn DoD-тест). BR-13' (pre-existing) явно ескальований як передумовний фікс з власником і enable-gate — не прихований, не тихо обійдений.

**Ширший передумовний фікс — ТАК, з'явився (вузький, не repo-wide):** клас «transaction-local GUC поза транзакцією» доведено існує в `telegram-webhook.ts` order-handler (наявний баг). НЕ repo-wide: courier/web/cron-шляхи верифіковано РОБЛЯТЬ `BEGIN`. Закривається тим самим PR, що нова store-toggle-дія (handler усе одно треба обгорнути). Додаю lint-guard-тригер (R-PRE) проти регресії класу.

## ETHICAL — STOP-ETHICS-1 (без змін, лишається людським актом)

Counsel Раунд 2 підтвердив: STOP-ETHICS-1 — **справжній людський акт**, не знімається дизайном. Вузько: чи `order.pending_aging` пробиває тишу як передвісник, чи чекає ранку (поки лише його незворотний наслідок `timeout_cancelled` будить). Fail-safe незмінний: якщо підпис не поставлено → `pending_aging` лишається `transactional` (пробиває), бо operational-order-flow порожня. **Дизайн НЕ намагається зняти STOP — фіксує його в ADR як очікування підпису Product+Arch.**

## Нові / оновлені accept-risk + передумова (Раунд 2)

| # | Ризик | Рішення | Власник |
|---|---|---|---|
| R-PRE | Pre-existing: `telegram-webhook` order-handler ставить transaction-local GUC поза транзакцією + не ставить `app.user_id` | **Передумовний фікс** (обгорнути handler у BEGIN+обидва GUC); закривається PR-ом нової store-дії; lint-guard: ESLint/grep-CI правило «`set_config(...,true)` має мати `BEGIN` у тому ж блоці» проти регресії класу. Gate ПЕРЕД `TG_STOREFRONT_ACTION=on`. | Backend |
| R2' | ~~delivery_paused не hard-блокує~~ | **ЗАКРИТО (BR-17)** — атомарний INSERT…SELECT guard; вікно елімінується. Hard capacity-gate (stock-aware) — окремий скоуп, інша річ. | Product+Arch |

---
---

# РАУНД 3 — RESOLVE (BR-20 / BR-21 / BR-22) + СТРУКТУРНЕ рішення role/pool

**Date:** 2026-06-22 · **Inputs:** `breaker-findings.md` (РАУНД 3) · `counsel-opinion.md` (Раунд 3 — етично готовий, 3 нон-блокінг ноти)
**Метод:** кожна знахідка верифікована проти робочого дерева ПЕРЕД вердиктом. Корінь Ламача визнаю прямо.

## Визнання кореня (Ламач довів за 3 раунди — приймаю дослівно)

Раунд-2 «worker-escape GUC» був **структурно хибним**, і я це визнаю без захисту:
- **Один пул, одна роль.** Notif-worker отримує **operational pool** (`server.ts:346` — `new NotificationWorker(pool,…)`, `pool=createOperationalPool()`), той самий `deliveryos_api_user` що web-routes і `telegram-webhook` (`server.ts:632`). Verified.
- **Self-grantable backdoor (BR-20).** Escape-гілка `OR current_setting('app.notif_worker',true)='on'` — **роль-сліпа**. Будь-який web-шлях на тому ж пулі легально робить `BEGIN; set_config('app.notif_worker','on',true); SELECT * FROM owner_notification_targets` → читає owner-таргети (chatId/address PII) ВСІХ тенантів. Дисципліною «web не ставить worker-GUC» tenant-ізоляцію будувати НЕ можна.
- **Policy читає неіснуючий GUC (BR-21).** Proposal §5 policy гейтить на `current_setting('request.jwt.claim.sub')`. Live-факт: **0 production-setters цього GUC** (verified grep). Канонічні owner-таблиці (`orders`/`locations`/`memberships`/`customers`) гейтяться через **`app.user_id`** → `app_member_location_ids()` → `app_current_user()` → `current_setting('app.user_id',true)` (`core-identity.ts:70-79`, verified). Web prefs-шлях `owner/notifications.ts:21,23` робить голий `SELECT … WHERE location_id=$1` **БЕЗ жодного set_config / BEGIN** — працює сьогодні ЛИШЕ бо table `ENABLE`-not-`FORCE` (owner-bypass). Forward-only FORCE-міграція з `request.jwt.claim.sub`-policy → 100% web owner-notification операцій мертві.

**Жоден GUC-трюк це не лагодить.** Потрібне структурне role/pool-розрізнення + узгодження policy-GUC з каноном. Нижче — обидва.

---

## СТРУКТУРНЕ РІШЕННЯ (одне речення кожне)

> **BR-21 (policy-GUC):** усі 4 нові таблиці гейтяться на **канон `app.user_id`** (через `app_member_location_ids()`, рівно як `orders`/`locations`), `request.jwt.claim.sub` ВИДАЛЕНО з кожної policy; web prefs-шлях `owner/notifications.ts` мігрує на `BEGIN; set_config('app.user_id', <jwt.sub>, true)` ПЕРЕД увімкненням FORCE (інакше read-after-write на prefs мертвий).
>
> **BR-20 (worker-escape):** escape прив'язується до **ролі, не до runtime-GUC** — notif-worker (і лише він) конектиться окремим **dedicated restricted-пулом під роллю `deliveryos_notif_worker`** з **верифікованим (не swallowed) BYPASSRLS**, фізично відокремленим від web operational-пулу; runtime-GUC `app.notif_worker` ВИДАЛЕНО з усіх policy. Web-роль `deliveryos_api_user` тоді не може обійти RLS жодним шляхом, бо обхід — властивість ролі, а не self-settable значення.

**Інфра дозволяє (verified, нуль нової платформенної залежності):** репо **вже має 2-пуловий патерн** — `createSessionPool()` (`db/index.ts:45`, port 5432, max=3, `***REDACTED***`) існує паралельно `createOperationalPool()` (port 6543, max=8). `messageBus` уже бере окремий session-пул (`server.ts:262-263`). Тобто «другий фізичний пул для воркера» — **наявний архітектурний шов, не нова інфра**. Бракує лише **окремої restricted-ролі** (нова DB-роль + grant) — це міграційна вартість, винесена на STOP-DESIGN-B (нижче).

---

## BR-21 · CRITICAL — РІШЕННЯ: policy-GUC = канон `app.user_id`, web prefs-шлях мігрує ПЕРЕД FORCE

**Опції зважені (verified проти коду):**
- **(a) Привести до канону `app.user_id`.** Policy всіх 4 нових таблиць читає `location_id IN (SELECT app_member_location_ids())` — той самий вираз, що `orders`-policy (`orders.ts:83-84`), що резолвиться через `current_setting('app.user_id',true)`. Web prefs-handler мігрує на канонічний патерн репо: `BEGIN; SELECT set_config('app.user_id', $jwtSub, true); …; COMMIT` (рівно як `customer/push.ts:35`, `courier/shifts.ts:23-24`, `spa-proxy.ts:420`). **ОБРАНО.**
- (b) Лишити targets на `ENABLE`-not-`FORCE` (owner-bypass прийнятний) — **відкинуто:** прямо порушує червону лінію `ENABLE+FORCE` на кожній tenant-таблиці; owner-bypass на pooled-ролі = той самий клас, що BUG-B, який цей дизайн і закриває. Жодного accept-risk-аргументу, що переважив би червону лінію, тут нема.

**Чому (a) — це не GUC-трюк, а вирівнювання з каноном:** проблема BR-21 була НЕ «FORCE» і НЕ «GUC», а **policy читає GUC, який прод не ставить**. Канон `app.user_id` прод **реально ставить** на JWT-шляхах (verified setters: `customer/push.ts:35`, `mock-auth.ts:92`; функції `app_current_user`/`app_member_location_ids` — `core-identity.ts:70-79`). Web prefs-шлях `owner/notifications.ts` його НЕ ставив, бо table не була FORCE'd — owner-bypass маскував пропуск. Вирівнюючи policy на канон І додаючи setter у web-шлях, read-after-write на prefs працює під FORCE, як працює для `orders`.

**SQL (фінальна форма policy — заміна §5.1/§5.2/§5.3/§5.4):**
```sql
CREATE POLICY <name> ON <table> FOR ALL TO authenticated
  USING ( location_id IN (SELECT app_member_location_ids()) );   -- канон, рівно як orders/locations
-- БЕЗ request.jwt.claim.sub, БЕЗ current_setting('app.notif_worker')
```
Worker-escape більше НЕ в policy (перенесено в роль, BR-20). `audit`/`prefs_audit`/`nonces` — `location_id IN (SELECT app_member_location_ids())`; `targets` — те саме (вона має `location_id`).

**Web prefs-шлях — обов'язкова міграція ПЕРЕД FORCE (forward-only ordering):**
```ts
// owner/notifications.ts — КОЖЕН handler (GET targets, PUT prefs, status) обгортається:
const client = await db.connect();
try {
  await client.query('BEGIN');
  await client.query("SELECT set_config('app.user_id', $1, true)", [request.user.sub]);
  // … SELECT/UPDATE owner_notification_targets … (+ prefs-audit INSERT у тій же транзакції, BR-16)
  await client.query('COMMIT');
} catch (e) { await client.query('ROLLBACK').catch(()=>{}); throw e; }
finally { client.release(); }
```
**Міграційний ordering (forward-only, критичний):** `owner/notifications.ts`-міграція на `set_config('app.user_id')` деплоїться у **тому самому або попередньому деплої**, що FORCE-міграція `...049`. Якщо FORCE вмикається раніше за код-міграцію → BR-21 outage. → **enable-gate: code-deploy (web prefs setter) live ДО запуску FORCE-міграції.** Записано нижче.

**ВИПРАВЛЕНИЙ DoD-тест §5.1 (Ламач: тест перевіряв `app.user_id`, policy читала `request.jwt.claim.sub` → зелений тест/мертвий прод):**
> DoD тепер тестує **ТОЙ САМИЙ GUC, що policy реально читає** — `app.user_id`:
> 1. Під FORCE, `set_config('app.user_id', <member-of-loc-A>, true)` → `SELECT … FROM owner_notification_targets WHERE location_id=<A>` → **повертає рядки** (доказ: web read-after-write живий).
> 2. Під FORCE, `set_config('app.user_id', <non-member>, true)` → той самий SELECT → **0 rows** (доказ: tenant-ізоляція).
> 3. Під FORCE, `PUT prefs` web-шляхом (з `app.user_id` member) → `jsonb_set`-UPDATE → **rowCount=1** + prefs-audit INSERT успішний (доказ: prefs save живий під FORCE, BR-16 атомарність).
> 4. **Регрес-guard:** `grep "request.jwt.claim.sub"` по нових міграціях → **0** (доказ: policy не читає GUC без setter'а).
> Без 1+3 (web живий під FORCE) — FORCE-міграція НЕ деплоїться.

**Вердикт BR-21: fix.** Policy-GUC = канон `app.user_id`; web prefs-шлях мігрований на setter ПЕРЕД FORCE; DoD перевіряє той GUC, що policy читає.

---

## BR-20 · HIGH — РІШЕННЯ: dedicated restricted-роль `deliveryos_notif_worker` + окремий пул; escape прив'язаний до РОЛІ, не до GUC

**Опції зважені (verified проти інфри):**
- **(a) Окрема DB-роль `deliveryos_notif_worker` з реальним верифікованим BYPASSRLS + окремий конект-пул лише для notif-воркера.** Escape-policy НЕ потрібна взагалі — worker-роль обходить RLS легітимно (атрибут ролі), web-роль `deliveryos_api_user` ніколи не може (нема self-settable GUC). **ОБРАНО — це варіант (b) із завдання у його структурно-чистій формі: право прив'язане до ролі.**
- (b) Worker-escape `policy TO deliveryos_notif_worker` (role-based policy замість BYPASSRLS) — **технічно еквівалентний за безпекою** (не self-grantable), але вимагає писати окрему `TO`-гілку в КОЖНІЙ policy 4 таблиць. BYPASSRLS-роль чистіша: воркер легітимно поза RLS на ВСІХ tenant-таблицях, до яких йому треба (audit/targets/nonces/prefs_audit), без per-table policy-розгалуження. **Беремо (a) як основу; (b) — fallback якщо Supabase не дасть BYPASSRLS-атрибут (нижче).**

**Чому BYPASSRLS-роль тепер РЕАЛЬНА (не swallowed-EXCEPTION як BR-1 раунд-1):** ключова різниця — у раунді-1 grant робився `ALTER ROLE deliveryos_api_user BYPASSRLS` у `DO … EXCEPTION WHEN OTHERS` (мовчки ковтав провал). Тут:
1. **Окрема роль** `deliveryos_notif_worker` — створюється явно, grant `BYPASSRLS` **БЕЗ swallowed-EXCEPTION**; якщо платформа не дасть атрибут → міграція **FATAL-fail'иться** (не ковтає), і ми дізнаємось на staging, не в проді.
2. **Boot-time assert (verified-grant gate):** воркер на старті виконує `SELECT rolbypassrls FROM pg_roles WHERE rolname=current_user` — якщо `false` → FATAL-exit (worker не стартує). Це робить grant **доведеним**, не припущеним. Дзеркалить наявний guardrail `db/index.ts:32` («operational pool НЕ під postgres»), лише в інший бік: «notif-pool МУСИТЬ мати BYPASSRLS».

**Інфра — наявний шов (verified, нуль нової платформенної залежності):**
```ts
// db/index.ts — НОВИЙ пул за взірцем createSessionPool (вже існує, port 5432, max=3)
export function createNotifWorkerPool(): pg.Pool {
  const pool = new Pool({
    connectionString: env.DATABASE_URL_NOTIF_WORKER,   // нова env: роль deliveryos_notif_worker
    max: 2, idleTimeoutMillis: 30000, connectionTimeoutMillis: 5000, ssl: …
  });
  pool.on('connect', async (client) => {
    await client.query("SET statement_timeout = '30s'");
    const res = await client.query('SELECT current_user, rolbypassrls FROM pg_roles WHERE rolname = current_user');
    if (res.rows[0].current_user !== 'deliveryos_notif_worker')
      throw new Error('FAULT: notif pool not connected as deliveryos_notif_worker');
    if (res.rows[0].rolbypassrls !== true)
      throw new Error('FATAL: deliveryos_notif_worker lacks BYPASSRLS — RLS-escape grant did not apply');
  });
  return pool;
}
```
`server.ts:346` — `new NotificationWorker(notifPool, …)` де `notifPool = createNotifWorkerPool()`, **НЕ** `pool` (operational). Webhook order-handler лишається на operational (web-роль) — він НЕ обходить RLS, він ставить `app.user_id` як член (BR-22).

**Конект-бюджет (§2 перерахунок):** +2 конекти (notif-worker max=2). §2 раунду-1 заявляв «0 нових конектів» — це більше НЕ так. Перерахунок: operational max=8 + session max=3 (messageBus) + notif-worker max=2 + migrations transient = **сукупно 13 постійних + transient**. Supabase pooler (Supavisor) на 6543/5432 тримає сотні — 13 комфортно. Але **це нова залежність (роль+пул+env)** → на STOP-DESIGN-B (нижче).

**Чому web НЕ може прочитати чужі owner-таргети ЖОДНИМ шляхом (доказ-замикання):**
- Web/webhook конектяться роллю `deliveryos_api_user` (operational/session пули). Ця роль **не має BYPASSRLS** (verified guardrail `db/index.ts:32` гарантує навіть не-postgres). FORCE-policy на `targets` = `location_id IN (SELECT app_member_location_ids())` → web бачить ЛИШЕ локації свого `app.user_id`-члена.
- Немає `OR current_setting('app.notif_worker')` гілки в policy (ВИДАЛЕНО) → web не може self-escape. `set_config('app.notif_worker','on')` тепер **no-op** для RLS (жодна policy його не читає).
- BYPASSRLS має ЛИШЕ `deliveryos_notif_worker`, яким web-код НЕ конектиться (окремий пул, окрема env-credential). Web-код не має способу «стати» цією роллю на operational-пулі (нема `SET ROLE` у коді — verified grep потрібен в impl-DoD).
→ Крос-tenant read owner-таргетів web-роллю **структурно неможливий**, не «дисципліною». Self-grantable backdoor усунутий.

**DoD-gate (impl):**
1. Boot-assert: notif-pool під `deliveryos_notif_worker` з `rolbypassrls=true` (інакше FATAL).
2. Negative-тест: на operational-пулі (`deliveryos_api_user`), `set_config('app.notif_worker','on',true); SELECT * FROM owner_notification_targets WHERE location_id=<чужа>` → **0 rows** (доказ: GUC-escape мертвий, web не може).
3. Positive-тест: на notif-пулі, той самий SELECT БЕЗ будь-якого GUC → **всі рядки** (доказ: BYPASSRLS-роль легітимно поза RLS).
4. `grep "SET ROLE\|set_config('app.notif_worker'" apps/ packages/` у policy/web → **0** (доказ: escape не в коді й не в policy).

**Вердикт BR-20: fix.** Escape прив'язаний до ролі `deliveryos_notif_worker` (verified BYPASSRLS, FATAL-on-missing), runtime-GUC `app.notif_worker` ВИДАЛЕНО з усіх policy. Self-grantable bypass усунутий структурно.

> **Наслідок для BR-13 (раунд-2 транзакційна обгортка):** worker-escape більше НЕ через `set_config('app.notif_worker',true)` у транзакції — він через роль. Тому BR-13-обгортка `BEGIN; set_config('app.notif_worker'…)` **спрощується**: notif-worker конектиться BYPASSRLS-роллю, йому НЕ потрібен GUC-escape узагалі; транзакція потрібна ЛИШЕ де треба атомарність (prefs+audit, BR-16), не для RLS-escape. BR-13(B) leak-вектор зникає разом з GUC. BR-13(A) outage зникає (роль обходить FORCE без GUC). **BR-13 закривається чистіше структурним рішенням, ніж транзакційним трюком.**

---

## BR-22 · HIGH — РІШЕННЯ: `<resolvedMember>` = вже-доведений active-member з webhook-резолвера; 0-rows на guarded-UPDATE = explicit failure

**Знахідка Ламача (verified):** R-PRE-обгортка ставить `set_config('app.user_id', <resolvedMember>, true)`, але джерело `<resolvedMember>` недовизначене. Якщо `target.user_id` non-member/disconnected → orders-FORCE-policy deny → `updateOrderStatus` 0 rows → бот каже «confirmed», замовлення лишається PENDING.

**Fix п.1 — джерело `<resolvedMember>` точно визначене (resolver ВЖЕ існує, verified):** webhook order-handler **уже** резолвить і перевіряє active-member ПЕРЕД мутацією:
- `telegram-webhook.ts:147-157` — order-гілка: `targetUserId` = `owner_notification_targets.user_id` для `(chatId, channel='telegram', status='active', location_id)`.
- `telegram-webhook.ts:185-194` — **membership-перевірка**: `SELECT 1 FROM memberships WHERE user_id=$targetUserId AND location_id=$loc AND status='active'`; rowCount=0 → `'Unauthorized: not a member'` → return (нуль мутації).
→ Тобто на момент `updateOrderStatus` `targetUserId` **уже доведений active-member цієї локації**. **`<resolvedMember> := targetUserId`** (той самий, що пройшов `:185-194`). Це НЕ «будь-який member» і НЕ «disconnected target» — це рівно той user, чию membership webhook щойно верифікував. orders-FORCE-policy `app_member_location_ids()` для цього user поверне `location_id` → guarded UPDATE проходить.

> **Залишковий edge (закривається п.2):** legacy-рядки з `targetUserId=NULL` (`:185` `if (targetUserId)` пропускає membership-перевірку). Для НИХ `app.user_id` не можна поставити надійним членом. Рішення: **якщо `targetUserId IS NULL` для order-mutating дії → reject** (`'Account link incomplete — reconnect Telegram'`), НЕ ставити порожній/довільний `app.user_id`. Legacy-bypass «user_id NULL = довірений токен-flow» прийнятний для READ, але НЕ для guarded order-transition під FORCE (інакше app.user_id порожній → policy deny → fail-silent). Записано в impl-DoD.

**Fix п.2 — 0-rows на guarded-UPDATE = explicit failure (не «confirmed»):** `updateOrderStatus` після FORCE-обгортки МУСИТЬ розрізняти «status-transition застосувався» від «0 rows». Поточний `orderStatusService` кидає 404/409 (handler це ловить, `:232-254`) — АЛЕ під FORCE з НЕвірним `app.user_id` UPDATE поверне **0 rows БЕЗ throw** (RLS-deny не кидає, просто не матчить рядок).
```ts
// orderStatusService / webhook order-handler:
const upd = await client.query(
  `UPDATE orders SET status=$3 WHERE id=$1 AND location_id=$2 RETURNING id`, [entityId, locationId, newStatus]);
if (upd.rowCount === 0) {
  // RLS-deny АБО order-not-found АБО concurrent-transition — НЕ трактувати як успіх
  throw new OrderTransitionError('GUARDED_NOOP', 'order transition affected 0 rows under RLS');
}
```
Handler: `GUARDED_NOOP` → `resultText = '⚠️ Помилка: не вдалось підтвердити (перевірте підключення)'`, бот рапортує **помилку**, НЕ `'✅ ЗАМОВЛЕННЯ ПІДТВЕРДЖЕНО'`. `answerCallbackQuery` показує помилку, не «confirmed». Замовлення лишається PENDING — і оператор це **бачить** (не тихий неуспіх). guard-on-rowcount відновлений.

**DoD-gate (impl):**
1. Order-confirm з `targetUserId`=active-member → UPDATE rowCount=1 → бот «confirmed», order CONFIRMED.
2. Order-confirm з `targetUserId`=non-member (підроблений/disconnected scenario) → membership-перевірка `:185-194` reject ПЕРШОЮ (нуль мутації) — guard #1.
3. Симуляція FORCE-deny (app.user_id порожній) → UPDATE rowCount=0 → `GUARDED_NOOP` throw → бот «помилка», НЕ «confirmed», order лишається PENDING — guard #2 (defence-in-depth, на випадок якщо resolver-guard колись пропустить).
4. `targetUserId IS NULL` + order-mutating → reject до UPDATE.

**Вердикт BR-22: fix.** `<resolvedMember>` = вже-верифікований `targetUserId` (webhook membership-resolver `:185-194`); NULL-user order-mutation → reject; 0-rows guarded-UPDATE → explicit `GUARDED_NOOP` failure, бот рапортує помилку, не «confirmed».

---

## Нон-блокінг Counsel (Раунд 3) — адресовано

| Нота | Дія |
|---|---|
| 409 `store_paused` клієнт-копірайт каже правду | **fix (UX-копірайт):** frontend toast/error-state для `store_paused` 409 = «Заклад щойно поставив паузу на приймання — спробуйте трохи згодом», НЕ сирий код. Власник: клієнтський checkout-UI (frontend). Записано в §7/§10. |
| nonce-INSERT-fail → бот fallback-pointer на web | **fix:** якщо `telegram_action_nonces`-INSERT падає (БД glitch) → бот шле «Не вдалось підготувати підтвердження — закрийте приймання через застосунок», явний fallback на web-канал (швидкий, без nonce), НЕ тиха відсутність confirm-кнопки. §8. |
| R-PRE окремий security-changelog рядок | **fix (governance):** R-PRE отримує власний changelog-рядок «зміцнено tenant-ізоляцію наявного Telegram order-action шляху (app.user_id під FORCE)» — окрема security-перемога, не схована в feature-PR. §10. |

## ETHICAL — STOP-ETHICS-1 (БЕЗ ЗМІН)

Лишається людський підпис під `order.pending_aging` (передвісник пробиває тишу vs чекає ранку). Раунд-3 (role/pool/policy-GUC) не торкнувся жодного `quietHours`-значення чи category-присвоєння. Counsel Раунд-3: етично готовий, нова червона лінія НЕ з'явилась. **Не чіпаю.** Fail-safe незмінний.

---

## Таблиця вердиктів — Раунд 3

| BR | Severity | Вердикт | Механізм (1 рядок) |
|---|---|---|---|
| BR-20 | HIGH | **fix (структурний)** | Dedicated роль `deliveryos_notif_worker` + окремий пул `createNotifWorkerPool` з verified BYPASSRLS (FATAL-on-missing, не swallowed); runtime-GUC `app.notif_worker` ВИДАЛЕНО з усіх policy. Escape — атрибут ролі, не self-settable. Web-роль структурно не може обійти RLS. |
| BR-21 | CRITICAL | **fix (структурний)** | Policy-GUC = канон `app.user_id` (`app_member_location_ids()`, рівно як orders); `request.jwt.claim.sub` ВИДАЛЕНО. Web prefs-шлях `owner/notifications.ts` мігрує на `set_config('app.user_id')` ПЕРЕД FORCE (enable-gate ordering). DoD §5.1 виправлено — тестує `app.user_id` (той GUC, що policy читає). |
| BR-22 | HIGH | **fix** | `<resolvedMember>` = вже-верифікований `targetUserId` (webhook membership-resolver `:185-194`); NULL-user order-mutation → reject; 0-rows guarded-UPDATE → `GUARDED_NOOP` throw, бот рапортує помилку НЕ «confirmed». |

**Підсумок раунду 3:** fixed = **3** (BR-20, BR-21, BR-22) · нові accept-risk = **0** · defer = **0**.
**Відкритих CRITICAL = 0. Відкритих HIGH = 0.**

**Нова залежність / міграційна вартість на STOP-DESIGN-B:** **ТАК — виношу людині.** Структурне рішення BR-20 вводить:
1. **Нову DB-роль** `deliveryos_notif_worker` з BYPASSRLS-grant (Supabase: чи платформа дає BYPASSRLS-атрибут не-superuser-ролі? Якщо НІ — fallback на опцію (b) role-based `policy TO deliveryos_notif_worker`, яка BYPASSRLS не потребує). **Власник: Backend/Infra перевіряє на staging ПЕРШИМ кроком.**
2. **Нову env-credential** `DATABASE_URL_NOTIF_WORKER` (secret-менеджмент, нуль у git).
3. **+2 постійних конекти** (§2 «0 нових конектів» більше не вірний — перераховано на 13 сукупно, у бюджеті).
4. **Enable-gate ordering:** web prefs `set_config('app.user_id')`-міграція live ДО FORCE-міграції `...049` (інакше BR-21 outage).
→ Це **структурна зміна топології конектів** (нова роль+пул), не локальний fix. Рекомендую STOP-DESIGN-B: людина (Backend/Infra+Arch) підтверджує (a) Supabase дає BYPASSRLS не-superuser-ролі (інакше fallback b), (b) приймає +1 роль/пул/env у топологію.

## Нові accept-risk / залежності (Раунд 3)

| # | Ризик/залежність | Рішення | Власник |
|---|---|---|---|
| R-ROLE | Нова DB-роль `deliveryos_notif_worker` + BYPASSRLS-grant — чи Supabase дає атрибут не-superuser-ролі | **STOP-DESIGN-B** — Backend перевіряє на staging ПЕРШИМ; якщо НІ → fallback role-based `policy TO deliveryos_notif_worker` (BYPASSRLS не потрібен). FATAL-on-missing boot-assert. | Backend/Infra+Arch |
| R-POOL | +2 конекти (notif-worker пул); §2 «0 нових конектів» застарів | **Accept** — перераховано 13 сукупно, Supavisor тримає сотні. Записано в §2. | Backend |
| R-ORDER | FORCE-міграція раніше за web-setter-код → BR-21 outage | **Enable-gate** — web prefs `set_config('app.user_id')` deploy live ДО FORCE-міграції `...049`. DoD §5.1 п.1+3 (web живий під FORCE) блокує деплой FORCE. | Backend |
| R-PRE | (раунд-2) — БЕЗ ЗМІН, + окремий security-changelog рядок (Counsel R3) | Передумовний фікс, gate перед `TG_STOREFRONT_ACTION`; тепер `set_config('app.user_id', targetUserId, true)` (BR-22 джерело визначене). | Backend |

---
---

# РАУНД 4 — RESOLVE (BR-23 / BR-24) — вузький, до жорсткого виходу

**Date:** 2026-06-22 · **Inputs:** `breaker-findings.md` (РАУНД 4: BR-23 HIGH, BR-24 MED) · структурне рішення Раунду 3
**Метод:** кожен корінь верифіковано проти робочого дерева ПЕРЕД вердиктом.

## Live-верифікація несучих фактів (BR-23/24 ядро)

- `1780310071220_core-identity.ts:72,76,85-87` — `locations` **FORCE RLS вже сьогодні**, policy `tenant_isolation USING (id IN (SELECT app_member_location_ids()))`, `app_member_location_ids()`→`app_current_user()`→`current_setting('app.user_id',true)` (`:72`). **Підтверджено: store-toggle UPDATE на `locations` проходить через FORCE на `app.user_id`, НЕ `app.current_tenant`.**
- `telegram-webhook.ts:147-194` — non-order гілка (store-toggle) **резолвить `targetUserId`** (`:160-172`) і верифікує `memberships` (`:185-194`) ЛИШЕ якщо `targetUserId` non-NULL. → джерело `app.user_id` для store-шляху **доступне** (той самий `targetUserId`-патерн, що BR-22).
- proposal §C2 (`:154,156,324`) — store-toggle ставить `set_config('app.current_tenant')` (хибний GUC для `locations`-policy) і трактує `rowCount=0` як idempotent-success. **Обидва — корінь BR-23, підтверджено.**

---

## BR-23 · HIGH — РІШЕННЯ: канон `app.user_id` на store-шляху + 3-way CTE-розрізнення deny vs idempotent-noop vs changed

**Визнаю корінь дослівно (Ламач правий — структурне рішення Раунду 3 ВВЕЛО цю двозначність на сусідній площині):** дві склеєні проблеми.

### Проблема 1 — неправильний GUC (тривіальний, узгоджую)

`locations`-policy під FORCE читає `app.user_id` (`core-identity.ts:72`), а §C2 (`:156`) ставить `app.current_tenant` → RLS-deny на КОЖНОМУ store.close/open. **Fix:** store-toggle обгортка ставить **канон `app.user_id`** (як order-шлях BR-22, як `orders`/`locations`):
```sql
BEGIN;
SELECT set_config('app.user_id', $targetUserId, true);   -- канон; targetUserId = вже-верифікований active-member (webhook-resolver :160-194)
-- … compare-set нижче …
COMMIT;
```
`$targetUserId` — той самий, що пройшов membership-перевірку `:185-194` (для order-шляху BR-22 джерело вже визначене; для store-шляху воно ТЕ САМЕ — non-order гілка резолвить `targetUserId` на `:160-172`). NULL-user store-mutating дія → reject (як BR-22 п.2, інакше app.user_id порожній → deny→fail-silent). `app.current_tenant` лишається опційно для courier-таблиць, але `locations`-policy його НЕ читає — тому додаємо `app.user_id` обов'язково.

### Проблема 2 (суть) — 0-rows двозначність: deny vs idempotent-noop НЕРОЗРІЗНЕННІ

Навіть з правильним `app.user_id`, `UPDATE … WHERE delivery_paused IS DISTINCT FROM $new` під FORCE дає `rowCount=0` з ДВОХ джерел: (a) legit idempotent-noop (значення вже $new, рядок видимий); (b) RLS-deny (рядок невидимий цьому app.user_id — non-member/legacy-NULL/disconnected target). Дизайн трактує 0-rows як success → на deny бот каже «✅ пауза», storefront ВІДКРИТИЙ.

`GUARDED_NOOP`-throw з BR-22 тут **структурно непридатний**: для store-toggle 0-rows ЛЕГІТИМНО означає double-tap. Потрібен ДОДАТКОВИЙ сигнал «рядок видимий під RLS?».

**Опції зважені:**
- **(CTE з `FOR UPDATE`)** — один атомарний statement розрізняє ВСІ три випадки. **ОБРАНО.**
- (двокроковий authorize-SELECT → unconditional compare-set) — теж розрізняє, але два statements + потенційна мікро-гонка між SELECT-видимості й UPDATE (хоч обидва під row-lock у транзакції). CTE-варіант робить це одним атомарним statement під одним lock — чистіший, нуль вікна.

**ОБРАНА SQL (один statement, FOR UPDATE під FORCE):**
```sql
WITH cur AS (
  SELECT delivery_paused AS was
  FROM locations
  WHERE id = $loc
  FOR UPDATE                         -- бере row-lock ЛИШЕ якщо рядок ВИДИМИЙ під RLS (app.user_id member)
)
UPDATE locations l
   SET delivery_paused = $new
  FROM cur
 WHERE l.id = $loc
   AND l.delivery_paused IS DISTINCT FROM $new
RETURNING cur.was;                   -- 1 row → cur знайдено (видимий); was=стан ДО
```
**Розрізнення (3-way рапорт боту):**
| Результат | Означає | Бот рапортує | Audit |
|---|---|---|---|
| **0 rows** (CTE `cur` порожній під RLS) | **deny / not-found** — рядок невидимий цьому app.user_id АБО не існує | **ПОМИЛКА** «не вдалось — перевірте підключення» + **fallback на web** | `store.toggle.denied` |
| **1 row, `was = $new`** | **idempotent-noop** — рядок видимий, але вже в цільовому стані (double-tap/replay) | «вже на паузі» / «вже відкрито» (м'яко, НЕ помилка) | `store.toggle.noop` |
| **1 row, `was ≠ $new`** | **changed** — рядок видимий, значення перемкнуто | «✅ приймання на паузі о HH:MM» + realtime publish | `store.toggle.changed` |

**Чому FOR UPDATE-CTE під FORCE бачить рядок ЛИШЕ якщо authorized (доказ):** RLS USING-quals застосовуються до `SELECT … FOR UPDATE` так само, як до звичайного SELECT — `FOR UPDATE` блокує (lock) лише рядки, що пройшли policy `id IN (SELECT app_member_location_ids())`. Невидимий рядок не потрапляє в `cur` → `UPDATE … FROM cur` не має джерела → 0 rows. Видимий-але-вже-$new рядок потрапляє в `cur` (1 row), але `UPDATE`-WHERE `IS DISTINCT FROM $new` не матчить → **АЛЕ `RETURNING cur.was` все одно повертає рядок з `cur`?** — НІ: `UPDATE … FROM cur WHERE l.delivery_paused IS DISTINCT FROM $new` не оновлює жодного рядка `l`, тому `RETURNING` порожній → це знову колапсує noop у 0-rows. **Виправлення форми (критичне):** прибираємо `IS DISTINCT FROM` з UPDATE-WHERE і робимо compare у RETURNING-розрізненні на боці коду:
```sql
WITH cur AS (
  SELECT delivery_paused AS was FROM locations WHERE id = $loc FOR UPDATE
)
UPDATE locations l
   SET delivery_paused = $new
  FROM cur
 WHERE l.id = $loc            -- безумовний set; cur гарантує видимість
RETURNING cur.was, l.delivery_paused AS now;
```
- **0 rows** → `cur` порожній → рядок невидимий під RLS (deny) АБО id не існує → **FAIL** (бот помилка + web-fallback).
- **1 row & `was = now` (== $new, бо set безумовний)** → рядок був уже в $new → idempotent-noop → бот «вже …».
- **1 row & `was ≠ now`** → реально перемкнуто → бот «✅ о HH:MM» + publish.

Безумовний `SET delivery_paused=$new` ідемпотентний (write $new коли вже $new = no-op-write, той самий стан) → жодного побічного ефекту на double-tap, але `RETURNING cur.was` дає сигнал «що було», а наявність рядка дає сигнал «видимий під RLS». **Це розрізняє всі три, нуль колапсу.**

**Доведення інваріантів (вимога завдання):**
1. **Deny НІКОЛИ не рапортується як успіх:** deny → рядок невидимий → `cur` порожній → 0 rows → бот ПОМИЛКА + web-fallback. Неможливо отримати 0-rows-успіх (на відміну від `:154,324`, де 0-rows=success).
2. **Idempotent double-tap НЕ рапортується як помилка:** double-tap → рядок видимий (member) → `cur` має рядок → 1 row, `was=now` → бот «вже на паузі» (м'яко). Не помилка.
3. **Changed → точний рапорт:** 1 row, `was≠now` → «✅ о HH:MM».

**Узгодження двох площин (вимога завдання — чому КОНСИСТЕНТНІ):**
- **Order-площина (BR-22):** `updateOrderStatus` 0-rows = **fail** (`GUARDED_NOOP` throw). Order не має легітимного same-value-noop (confirm на вже-confirmed = реальна no-op-помилка стану, рідкісна, прийнятно як fail).
- **Store-площина (BR-23):** 0-rows = **fail** ТАКОЖ (рядок невидимий = deny → бот помилка). Відмінність ЛИШЕ: store має легітимний same-value-noop (double-tap close), якого order не має → тому store розрізняє «рядок видимий але вже $new» (1 row, noop) від «рядок невидимий» (0 rows, deny) через `cur FOR UPDATE`-сигнал.
- **Спільний інваріант обох:** **«рядок невидимий під RLS (0 rows на guarded-statement) = FAIL, ніколи не success».** Обидві площини тепер тримають guard-on-rowcount однаково: 0-rows ≠ success. Розбіжність трактування з Раунду 4 (order→fail, store→success) **усунута** — store теж 0-rows=fail; idempotent-noop винесений у ОКРЕМИЙ сигнал (1 row + `was=now`), а не сплутаний з 0-rows.

**Вердикт BR-23: fix.** (1) store-toggle ставить канон `app.user_id` (targetUserId, NULL→reject); (2) `setAcceptingOrders` переходить на `cur FOR UPDATE`-CTE → 3-way розрізнення; 0-rows=deny=FAIL+web-fallback (ніколи success), 1-row-same-value=idempotent-noop (не помилка), 1-row-changed=success. Guard-on-rowcount тепер консистентний з order-площиною (обидві: рядок невидимий=fail).

## BR-24 · MEDIUM — РІШЕННЯ: fallback `policy TO role` отримує ВЛАСНИЙ boot-assert + integration-тест на ОБОХ гілках; staging перевіряє BYPASSRLS-доступність ПЕРШИМ

**Визнаю діру (Ламач правий):** STOP-DESIGN-B чесно вимагає staging-перевірку (a)-гілки з FATAL-on-missing `rolbypassrls=true` assert. Але (b)-fallback (`policy TO deliveryos_notif_worker`) позначений лише «технічно еквівалентний» **без власного verified-gate** — якщо Supabase не дає BYPASSRLS (типово для managed PG) і (b) має edge (role-grant inheritance / Supavisor multiplexing) → notif-worker 0-rows під FORCE = BR-1-клас outage через непротестований fallback.

**Fix — обидві гілки отримують власний verified boot-assert + DoD-тест:**

| Гілка | Boot-assert (FATAL якщо false) | Сенс |
|---|---|---|
| **(a) BYPASSRLS** | `SELECT rolbypassrls FROM pg_roles WHERE rolname=current_user` → `true` (наявний, `resolution.md:457-462`) | роль реально має атрибут |
| **(b) `policy TO role`** | `SELECT count(*) FROM owner_notification_targets` під FORCE на notif-конекті БЕЗ GUC → **`> 0` для відомого seed-рядка** (АБО точніше: `SELECT 1 FROM owner_notification_targets WHERE location_id=$seedLoc` повертає рядок) → інакше FATAL | **роль реально SELECT-ить targets під FORCE через `TO role`-гілку** — доводить escape ФУНКЦІОНАЛЬНО, не атрибутивно |

**Ключовий зсув:** (b)-assert НЕ перевіряє атрибут (його нема в fallback-режимі — `rolbypassrls=false` легітимно), а перевіряє **функціональний результат** — «я як `deliveryos_notif_worker` реально бачу tenant-рядки під FORCE без GUC». Це той самий клас доказу, що (a)-assert (escape доведений, не припущений), адаптований під role-policy-механізм. Boot-логіка обирає assert за режимом: якщо `rolbypassrls=true` → (a)-assert; інакше → (b)-functional-assert (а якщо ОБИДВА false → FATAL «escape не працює жодним механізмом»).

**Integration-тест на ОБОХ гілках (DoD §5.1 розширення):** CI/staging проганяє notif-worker SELECT-targets-під-FORCE сценарій у двох конфігураціях — (a) роль з BYPASSRLS, (b) роль без BYPASSRLS + `policy TO deliveryos_notif_worker`-гілка в кожній policy. Обидві МУСЯТЬ повернути всі tenant-рядки під FORCE. Зелений лише той режим, який staging реально дає; але тест ІСНУЄ для обох → fallback не приймається «еквівалентним» без коду-доказу.

**Явна заборона `SET ROLE` на operational-пулі (BR-24 spillover):** fallback-(b) спокуса — `SET ROLE deliveryos_notif_worker` на спільному operational-пулі замість окремого пулу (дешевший shortcut). Це повертає self-grantable escape (web робить `SET ROLE`). **DoD §5.1 п.4 grep `SET ROLE` → 0 ПОШИРЮЄТЬСЯ на fallback-(b):** навіть у fallback notif-worker конектиться ОКРЕМИМ пулом під окремою credential `deliveryos_notif_worker`, НЕ `SET ROLE` на operational. `policy TO role` поважає `current_user` конекту (окремий пул = окрема backend-роль) — це й перевіряє (b)-functional-assert.

**STOP-DESIGN-B — що перевірити на staging ПЕРШИМ (визначає яка гілка жива):**
1. **ПЕРШИЙ крок:** `CREATE ROLE deliveryos_notif_worker … BYPASSRLS` на staging-DB → grant пройшов? `SELECT rolbypassrls` → true?
   - **ТАК** → гілка (a) жива; (a)-boot-assert активний; деплой під BYPASSRLS-режимом.
   - **НІ** (Supabase відмовляє не-superuser-ролі) → гілка (b): додати `TO deliveryos_notif_worker`-escape в КОЖНУ з 4 policy; (b)-functional-assert активний; деплой під role-policy-режимом.
2. **BYPASSRLS-доступність визначає яку гілку деплоїти** — це не runtime-fallback, це **deploy-time вибір за результатом staging-проби**. Обидві гілки мають код+тест; staging-проба обирає живу.

**Вердикт BR-24: fix.** Fallback-(b) отримує власний functional boot-assert «реально SELECT-ить під FORCE» (не «еквівалентний» на віру) + integration-тест на обох гілках + явну заборону `SET ROLE` на operational + STOP-DESIGN-B чітко вказує: staging-проба BYPASSRLS-доступності = ПЕРШИЙ крок, визначає deploy-time гілку.

## Таблиця вердиктів — Раунд 4

| BR | Severity | Вердикт | Механізм (1 рядок) |
|---|---|---|---|
| BR-23 | HIGH | **fix** | store-toggle ставить канон `app.user_id`=targetUserId (NULL→reject); `cur FOR UPDATE`-CTE розрізняє 0-rows=deny=FAIL+web-fallback (ніколи success) vs 1-row-`was=now`=idempotent-noop (не помилка) vs 1-row-`was≠now`=changed=«✅ HH:MM». Guard-on-rowcount консистентний з order-площиною: обидві «рядок невидимий=fail». |
| BR-24 | MEDIUM | **fix** | Fallback `policy TO role` отримує власний **functional** boot-assert (SELECT targets під FORCE без GUC → рядки, інакше FATAL) + integration-тест на ОБОХ гілках + заборона `SET ROLE` на operational; STOP-DESIGN-B: staging-проба BYPASSRLS = ПЕРШИЙ крок, визначає deploy-time гілку. |

**Підсумок раунду 4:** fixed = **2** (BR-23, BR-24) · нові accept-risk = **0** · defer = **0**.

**Відкритих CRITICAL = 0. Відкритих HIGH = 0. Відкритих MEDIUM = 0.** BR-23 (HIGH) закрито 3-way CTE-розрізненням + каноном `app.user_id`: deny НІКОЛИ не success, idempotent НІКОЛИ не помилка, дві guarded-площини тепер консистентні на «0-rows=рядок невидимий=fail». BR-24 (MED) закрито власним functional-assert на fallback-гілці + staging-першим визначенням живої гілки. **Ціль 0 відкритих CRITICAL/HIGH досягнута.**

## Нові accept-risk / залежності (Раунд 4)

| # | Ризик/залежність | Рішення | Власник |
|---|---|---|---|
| R-BR23 | store-toggle 0-rows колапсує deny vs idempotent | **FIXED** — канон `app.user_id` + `cur FOR UPDATE`-CTE 3-way; deny=fail+web-fallback, noop≠помилка, changed=точний рапорт | Backend |
| R-BR24 | fallback `policy TO role` без власного verified-gate | **FIXED** — functional boot-assert на (b)-гілці + integration-тест на обох + `SET ROLE`-заборона; STOP-DESIGN-B: staging-проба ПЕРША | Backend/Infra+Arch |
