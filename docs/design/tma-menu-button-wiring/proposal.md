# Design Proposal: Telegram Mini App (TMA) — bot-side menu-button wiring

**Status:** DRAFT (design-time; жодного продакшн-коду в цьому PR)
**Flag:** `TMA_ENABLED` (`z.enum(['true','false']).default('false')`) — default OFF
**Companion ADR:** `docs/adr/ADR-tma-menu-button-wiring.md` (draft)
**Extends (не суперседить):** `ADR-TELEGRAM-NOTIFICATIONS-ACTIONS` (той самий owner-side bot, той самий `callTelegramApi`-helper, той самий best-effort-off-critical-path принцип, ті самі dark-flag-прецеденти `TG_CATEGORY_GATING`/`TG_STOREFRONT_ACTION`)
**Concept anchors:** boring&proven > новизна · «схема багата, рантайм мінімальний» · failure-first · integration-addition (нуль нової поверхні авторизації) · ідемпотентність через natural-overwrite (не через dedup)

---

## 1. Проблема + non-goals

**Проблема.** Owner-side Telegram-бот сьогодні лише сповіщає (`owner_notification_targets`) і приймає owner-дії (order confirm/reject, store open/close, prefs). Ми хочемо, щоб після успішного connect-flow (`/start <token>`) чат власника отримував **menu-button типу `web_app`**, який відкриває вже наявний публічний storefront `/s/:slug` у Telegram WebView (Mini App). Це zero-схемна, flag-gated інтеграційна добавка.

**Що це НЕ (non-goals):**
- НЕ Telegram Payments / НЕ окремий checkout. Mini App = **той самий** `/s/:slug` у WebView; оплата лишається звичайним web-checkout. Money-path не торкається.
- НЕ нова таблиця/колонка/міграція. Нуль DDL.
- НЕ нова авторизація. Ця гілка лише **додає** один outbound-виклик після вже-перевіреного connect; не читає/не пише нових прав.
- НЕ customer-side бот. Кнопка ставиться на **чат власника** (owner-side). Аудиторія-питання винесене в §10 як OPEN-ризик (owner: Product) — воно НЕ блокує технічну безпеку зміни.
- НЕ Telegram `initData`-підпис / НЕ BotFather domain-allowlist. Storefront не читає Telegram-контекст; це звичайна web-сторінка у WebView.

---

## 2. Back-of-envelope

**Обсяг коду:** новий чистий модуль `apps/api/src/notifications/telegram-mini-app.ts` (~15 рядків, дві pure-функції, нуль I/O) + ~10 рядків inline у `telegram-webhook.ts` `/start`-гілці + 1 рядок у `packages/config` EnvSchema. Разом < 30 рядків.

**Частота події.** Тригер = успішний `/start <token>` connect. Це **раз на link власника↔локація** (не per-order, не per-message). Порядок величини: одиниці-десятки подій на локацію **за весь час життя** акаунта (initial connect + зрідка reconnect/re-scan). Навіть 500 локацій × кілька reconnect = сотні подій сукупно, не потік.

**Конект-бюджет.** +0 нових DB-конектів. SELECT slug виконується на **вже-взятому** `client` у `handleMessage` (operational pool). Загальний конект-бюджет (API operational 8 + session 3 + notif-worker 2, ~13 сукупно — див. sibling-ADR §Consequences) не змінюється. Supavisor тримає сотні.

**Outbound-бюджет (коли ON).** +1 HTTPS-виклик `setChatMenuButton` до **вже-інтегрованого** `api.telegram.org/bot<token>/<method>` — той самий endpoint-паттерн, той самий helper `callTelegramApi`, що вже обслуговує sendMessage/editMessageText/answerCallbackQuery. Виклик у серії ПІСЛЯ `sendMessage('start.connected')`. Нуль нового egress-хоста, нуль нового domain-trust.

**Blast radius.**
- **Flag OFF (default):** `process.env.TMA_ENABLED !== 'true'` → гілка не входить → байт-в-байт як зараз. Новий модуль імпортується, але його функції ніхто не викликає (tree-inert). Storefront, webhook, money — незмінні.
- **Flag ON:** додатковий один PK-SELECT (`locations` за id, індекс) + один outbound-виклик, тільки на `/start`. Webhook і так завжди повертає 200.

**Failure modes (усі проковтуються, нуль каскаду):** `setChatMenuButton` 400/401/429/5xx/timeout/network → inner try/catch → лишається без menu-button для цього чату; власник вже побачив `start.connected`; наступний `/start` повторить. Детально §7.

---

## 3. Опції (≥2) з tradeoffs

### Опція A — inline best-effort виклик у `/start`-гілці (**обрана**)
**Концепт:** integration-addition на happy-tail наявного flow; best-effort off-critical-path (той самий патерн, що order.reject_choose/store.close вже роблять inline через `callTelegramApi`).
- **(+)** Найпростіше, що тримає back-of-envelope. Нуль нової топології, нуль нового QUEUE_NAME (registry — governance-protected файл), нуль нового воркера/route/FE.
- **(+)** Прямо повторює наявні dark-flag-прецеденти в ЦЬОМУ Ж файлі → нульова когнітивна/операційна новизна.
- **(+)** Ідемпотентність безкоштовна: setChatMenuButton перезаписує; повторний /start = той самий стан.
- **(−)** +1 послідовний outbound-виклик у webhook-обробнику. Мітигація: подія рідкісна (раз на link), 200 повертається незалежно, inner try/catch + опційний timeout (§7).

### Опція B — async job через pg-boss (`QUEUE_NAMES.NOTIFY_TELEGRAM_SEND`-патерн)
**Концепт:** transactional-enqueue → воркер робить outbound; ізолює webhook-latency від Telegram API latency.
- **(+)** Webhook-response не чекає Telegram; вбудовані retries.
- **(−)** **Over-engineering для рідкісної best-effort-події.** Потребує: новий QUEUE_NAME у **governance-protected** `registry.ts`, новий worker-handler, retry/backoff-семантику, dead-letter-думання — усе це для виклику, який **природно ідемпотентний і природно ретраїться наступним /start**. Вмикає рантайм (черга) там, де вимога його не потребує → порушує «рантайм мінімальний». Зайва durable-latency (enqueue→poll→run) без користувацької вигоди (owner і так не чекає на кнопку синхронно).
- **Вердикт:** відхилено — диспропорційна складність.

### Опція C — owner-triggered admin endpoint (кнопка в `/admin` «встановити menu-button»)
**Концепт:** явний owner-контроль замість авто-тригера на /start.
- **(+)** Owner явно керує; можна кастомити text/slug; re-set on demand.
- **(−)** **Найбільший blast radius** для того самого ефекту: новий owner-route (нова auth-поверхня + Zod-контракт + rate-limit), новий FE-елемент, нові i18n (sq/en), новий E2E. Суперечить «дуже маленька зміна». Менш «магічно», але ціна непропорційна.
- **Вердикт:** відхилено зараз; **defer-flag** як можливий майбутній UX-апгрейд (owner-контроль над text/re-set), якщо з'явиться попит.

---

## 4. Рішення + обґрунтування (ADR-формат)

**Обрано Опцію A** з трьома несучими уточненнями поверх буквального ТЗ (кожне — коректність/безпека, не роздування):

1. **Inner try/catch — ОБОВ'ЯЗКОВИЙ, не косметика.** Без власного try/catch throw від `setChatMenuButton` (напр. 429) підніметься у **зовнішній** catch `handleMessage` (`telegram-webhook.ts:713`), який шле користувачу `botT(locale,'msg.error')`. Результат: власник побачить `start.connected` **і одразу** помилку — UX-регресія та хибний сигнал «connect не спрацював». Inner try/catch ізолює best-effort від success-signal. Це виправлення коректності, а не «nice-to-have».
2. **Guard на порожній slug.** `locations.slug` може бути NULL → `buildMiniAppUrl` дав би `/s/null`. Best-effort-обгортка **пропускає** виклик, якщо slug відсутній (`if (!slug) return`). Нуль зіпсованих кнопок.
3. **`appBaseUrl` — з валідованого config, не hardcoded fly.dev-літерал.** `APP_BASE_URL` вже `required url` у `packages/config` EnvSchema і **вже коректний per-environment** (staging→staging-хост, prod→prod-хост). Джерело з config → staging-власник відкриває staging-storefront, prod-власник — prod-storefront, автоматично. Hardcoded `'https://dowiz.fly.dev'`-fallback ризикує повести prod-власника на fly.dev-хост, якщо APP_BASE_URL коли-небудь порожній (а він required — тож fallback і так недосяжний → зайвий host-літерал у коді). Build-lane ЗОБОВ'ЯЗАНИЙ звірити (TMA-VALIDATION), що резолвлений host дійсно віддає `/s/:slug` SSR.

Обґрунтування вибору A: найдешевше рішення, що тримає back-of-envelope; повторює доведені in-file патерни; нуль нової топології/поверхні; ідемпотентність і retry — безкоштовні через природу API. B/C додають рантайм/поверхню, яких вимога не потребує (анти-over-engineering).

---

## 5. Дані / міграції

**N/A — і це навмисно.** Нуль DDL, нуль нових таблиць/колонок, нуль forward-міграцій.
- `location_id` вже відомий з `tokenRes.rows[0]` того самого запиту, що upsert'ить target.
- `slug` читається наявною колонкою `locations.slug` через звичайний SELECT.
- Жодного стану не персиститься на нашому боці: menu-button — стан **на боці Telegram** (per-chat), керований через API-виклик, не через нашу БД. Нема чого мігрувати, нема down-міграції, схема лишається інертною при OFF.
- integer-гроші / grошові шляхи — не торкнуто (money non-goal).

---

## 6. Узгодженість + ідемпотентність

- **`setChatMenuButton` природно ідемпотентний:** він **перезаписує** поточну menu-button чату. Повторний `/start` (reconnect/re-scan) просто ставить той самий стан. **Не потрібні** dedup/nonce/idempotency-key.
- Нема розподіленої транзакції: upsert target + mark-token-used вже завершені й закомічені (їхня консистентність — забота наявного connect-flow); menu-button — окремий незалежний best-effort side-effect ПІСЛЯ них. Якщо він не спрацює — target усе одно активний, сповіщення працюють.
- CAP-нота: це AP-side-effect на зовнішній системі (Telegram). Ми не блокуємо власний success на його консистентності. «Eventually»-встановиться на наступному /start, якщо цей раз не вдався.

---

## 7. Відмови + деградація (кожен зовнішній виклик: timeout + fallback, нуль каскаду)

Єдиний новий зовнішній виклик — `callTelegramApi('setChatMenuButton', …)`:

| Відмова | Поведінка | Каскад? |
|---|---|---|
| 400 (bad URL / non-HTTPS у dev) | inner catch → лог warn, кнопки нема | ні |
| 401 (bad bot token) | inner catch → лог warn | ні — це той самий токен, що вже слав `start.connected`; якщо 401 тут, `sendMessage` вже теж впав би раніше |
| 429 (rate limit) | inner catch → кнопки нема цей раз; наступний /start повторить | ні |
| 5xx / network / timeout (hung conn) | inner catch → кнопки нема | ні |
| slug = NULL/порожній | guard → виклик пропущено | ні |

**Деградація:** без menu-button власник **і далі** отримує звичайні notification-повідомлення й усі owner-дії — нічого не зламано; storefront доступний за прямим `/s/:slug` як завжди. Функція суто адитивна.

**Timeout-нота (accept-risk, репо-рівень):** наявний `callTelegramApi` НЕ має AbortController-timeout (на відміну від BR-10 sibling-ADR, який мандатує 10s на Telegram-fetch — ще не застосовано до цього helper). Новий виклик **не вводить новий клас** hang — він додає один послідовний no-timeout-fetch у гілку, яка вже awaited-ить no-timeout `sendMessage`. Опційне дешеве hardening без зміни спільного helper: обгорнути новий виклик у `Promise.race([call, timeout(~5s)])` всередині best-effort-обгортки (нуль впливу на інших callers `callTelegramApi`). Рекомендовано, але не обов'язково для OFF-дефолту. Ширший фікс (timeout у самому helper) — окремий репо-рівневий тикет (owner: Backend), не роздуваємо цю мікро-зміну.

---

## 8. Безпека + tenant-ізоляція

- **chatId вже автентифікований** наявним one-time token-flow: `telegram_connect_tokens` з `FOR UPDATE`-lock, `expires_at > now()`, `used_at IS NULL`. Нова гілка НЕ вводить нову authority-перевірку — вона будована на вже-перевіреному `location_id` з ТОГО САМОГО запиту, що upsert'ить target.
- **Нуль cross-tenant:** button ставиться на chatId, який щойно довів право на `location_id`; slug належить тому самому `location_id`. Кнопка веде на **власний** storefront власника.
- **SELECT slug під RLS — ПЕРЕВІРЕНО коректний.** `locations` має FORCE RLS, але з політикою `public_select ON locations FOR SELECT USING (true)` (міграція `1780338909301_public-locations-rls`, без `TO role` → чинна для operational-ролі `deliveryos_api_user`). Тож `SELECT slug FROM locations WHERE id=$1` **повертає рядок без GUC** — типовий клас «FORCE RLS → 0 rows без app.user_id» тут НЕ виникає. (Це був би прихований функціональний баг, якби `locations` мала лише tenant-policy; `public_select` його усуває — і ця перевірка є частиною DoD.)
- **Slug — публічний, не PII.** Вже видимий у `/s/:slug` URL усім відвідувачам storefront. Передача його в Telegram API — не витік. `?ch=telegram-tma` — пасивний attribution-тег, нуль PII, storefront ігнорує невідомі query-параметри.
- **Нуль cookies / нуль секретів у git / JWT не торкнуто.** Bot-token читається з env (як зараз).
- **Abuse-межа:** актор із валідним connect-токеном лінкує СВІЙ чат і отримує кнопку на **публічний** storefront тієї локації — сторінку, яку він і так може відкрити за `/s/:slug`. Нуль нової authority.

---

## 9. Операбельність

- **Kill-switch:** `TMA_ENABLED` (env, без деплою флип як інші `*_ENABLED`). Default OFF → deploy dark, флип після staging-валідації, миттєвий revert флипом.
- **Health degraded-vs-down:** ця зміна не має власного health-сигналу і **не може** перевести webhook у down (200 завжди; best-effort проковтнуто). Degraded-стан ззовні непомітний — це прийнятно, бо функція чисто-адитивна (відсутність кнопки ≠ інцидент).
- **Observability (<1 хв):** inner catch логує `warn` з методом і статусом (за взірцем наявних `console.warn('[TelegramWebhook] Failed to …')`). Оператор бачить частоту фейлів у логах; сплеск 429 = Telegram rate-limit, не наш баг.
- **Rollback:** флип `TMA_ENABLED=false` (миттєво) або revert PR. Нуль down-міграції (нуль DDL).
- **Enable-gate / scaling-gate:** ПЕРЕД `TMA_ENABLED=on` — operator manual test (BotFather → /start → menu-button з'являється → відкриває правильний живий `/s/:slug`), задокументовано у `docs/design/channel-hub/TMA-VALIDATION.md` (пише build-lane). Валідація мусить підтвердити: (1) резолвлений `APP_BASE_URL` host дійсно віддає storefront SSR; (2) кнопка відкриває саме цю локацію; (3) OFF → кнопки нема / байт-в-байт.

---

## 10. Відкриті / прийняті ризики

| # | Клас | Ризик | Рішення | Власник |
|---|---|---|---|---|
| R1 | **OPEN** | Аудиторія: кнопка ставиться на **owner-side** чат і відкриває **власний публічний storefront** власника. Цінність = preview/self-order/demo. Якщо справжня мета — **customer-facing** TMA, це окремий customer-bot flow (deep-links зі storefront, окремий бот). Не блокує технічну безпеку. | Product підтверджує намір аудиторії ПЕРЕД `TMA_ENABLED=on`. Fail-safe: при OFF нічого не відбувається. | Product |
| R2 | **accept-risk** | `callTelegramApi` без AbortController-timeout (репо-рівень, pre-existing). Новий виклик не вводить новий клас hang (гілка вже awaited-ить no-timeout `sendMessage`). | Опційне inline hardening `Promise.race([call, timeout(~5s)])`; ширший фікс — окремий тикет. | Backend |
| R3 | **accept-risk** | `APP_BASE_URL` мусить резолвитись на host, що віддає `/s/:slug` SSR (є два домени в системі: admin `app.dowiz.org` та storefront `dowiz.fly.dev`). Невірний host → кнопка відкриє не той сайт. | Джерело з config (§4.3) + явна перевірка в TMA-VALIDATION перед флипом. | Backend |
| R4 | **precondition (fix-in-PR)** | **`TMA_ENABLED` НЕ присутній у `packages/config` EnvSchema** (grep = 0 збігів), попри формулювання ТЗ «вже додано». | Build-lane ДОДАЄ рядок `TMA_ENABLED: z.enum(['true','false']).default('false')` за взірцем `MEDIA_RICH_ENABLED`/`GOOGLE_OAUTH_ENABLED` у тому самому PR. | Backend |

**Прийняті інваріанти (мають лишитись цілими):** нуль схеми/міграцій · нуль auth-змін · нуль money-path · нуль нових залежностей · default OFF · best-effort (фейл setChatMenuButton не ламає `start.connected`).

---

## Verification (DoD — design-time)
- **OFF-parity:** з `TMA_ENABLED` unset/false — `/start <token>` flow байт-в-байт як зараз (жоден новий виклик, жоден новий SELECT); assert у тесті webhook.
- **ON-happy:** `/start <token>` з валідним токеном → target upsert'нуто → `start.connected` надіслано → `setChatMenuButton` викликано з тілом `{chat_id, menu_button:{type:'web_app',text,web_app:{url:`${APP_BASE_URL}/s/${slug}?ch=telegram-tma`}}}`.
- **ON-degraded:** мок `callTelegramApi` кидає 429 → `start.connected` усе одно надіслано, зовнішній catch НЕ спрацював (користувач НЕ отримав `msg.error`), webhook повернув 200.
- **slug-guard:** slug=NULL → виклик пропущено, нуль throw.
- **RLS:** `SELECT slug FROM locations WHERE id=$1` на operational-ролі під FORCE повертає рядок (доводить, що `public_select` покриває цей read).
- **pure-функції:** `buildMiniAppUrl`/`buildSetChatMenuButtonRequest` — unit-тести на форму URL/тіла, нуль I/O.
