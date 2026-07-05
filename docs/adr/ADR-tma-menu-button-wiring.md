# ADR: Telegram Mini App (TMA) — bot-side menu-button wiring

**Date:** 2026-07-04
**Status:** DRAFT (design-time; behind flag `TMA_ENABLED` — `z.enum(['true','false']).default('false')`, default OFF)
**Design:** `docs/design/tma-menu-button-wiring/proposal.md`
**Extends (не суперседить):** `ADR-TELEGRAM-NOTIFICATIONS-ACTIONS` (той самий owner-side bot / той самий `callTelegramApi` / той самий best-effort-off-critical-path принцип / ті самі dark-flag-прецеденти `TG_CATEGORY_GATING`, `TG_STOREFRONT_ACTION`)
**Пов'язані ADR (не суперечить):** ADR 0001 (queue-in-Postgres — свідомо НЕ використовуємо чергу тут, §Alternatives B), ADR 0002 («схема багата, рантайм мінімальний» — тут навіть без схеми: нуль DDL).

---

## Context

Owner-side Telegram-бот обробляє connect-flow `/start <token>` (upsert `owner_notification_targets` по `(location_id, channel='telegram', address=chatId)` під one-time токеном `telegram_connect_tokens`: `FOR UPDATE`, `expires_at`, `used_at`) та owner-дії (order/store/prefs). Ми хочемо, щоб після успішного connect чат власника отримав **menu-button типу `web_app`**, що відкриває наявний публічний storefront `/s/:slug` у Telegram WebView. Це zero-схемна, flag-gated, дуже маленька інтеграційна добавка. Money-path, авторизація, схема — поза скоупом.

## Decision

1. **Новий чистий модуль `apps/api/src/notifications/telegram-mini-app.ts`** — дві pure-функції, нуль I/O:
   - `buildMiniAppUrl({appBaseUrl, slug}): string` → `${appBaseUrl}/s/${slug}?ch=telegram-tma`.
   - `buildSetChatMenuButtonRequest(chatId, {appBaseUrl, slug, text?})` → тіло Telegram Bot API `setChatMenuButton`: `{chat_id, menu_button:{type:'web_app', text, web_app:{url}}}`.
2. **Флаг `TMA_ENABLED`** у `packages/config` EnvSchema (взірець `MEDIA_RICH_ENABLED`/`GOOGLE_OAUTH_ENABLED`), default `'false'`. **ПЕРЕДУМОВА:** наразі відсутній у config (grep=0) попри формулювання ТЗ — додається build-lane'ом у тому самому PR.
3. **Одне місце виклику** — `telegram-webhook.ts` `handleMessage`, гілка `/start <token>` (НЕ `login_`), ПІСЛЯ upsert target і ПІСЛЯ `sendMessage(chatId, botT(locale,'start.connected'))`. Якщо `process.env.TMA_ENABLED === 'true'`: best-effort (inner try/catch) → `SELECT slug FROM locations WHERE id=$1` (location_id з `tokenRes`) → `callTelegramApi('setChatMenuButton', buildSetChatMenuButtonRequest(chatId, {appBaseUrl, slug}))`.
4. **Три несучі уточнення поверх буквального ТЗ (коректність/безпека, не роздування):**
   - **(a) Inner try/catch ОБОВ'ЯЗКОВИЙ.** Без нього throw (429/тощо) підніметься у зовнішній catch `handleMessage:713` → користувач отримає `msg.error` ПІСЛЯ `start.connected` (хибний сигнал провалу connect). Inner catch ізолює best-effort від success-signal.
   - **(b) Guard на порожній slug** — `if (!slug)` → пропуск (нуль `/s/null`-кнопок).
   - **(c) `appBaseUrl` з валідованого config** (`APP_BASE_URL`, вже `required url`, per-environment коректний), НЕ hardcoded `dowiz.fly.dev`-літерал → staging/prod самі резолвляться правильно.
5. **Ідемпотентність — безкоштовна:** `setChatMenuButton` перезаписує menu-button чату; повторний `/start` = той самий стан. Нуль dedup/nonce.

## Consequences

- **(+)** < 30 рядків, нуль DDL, нуль нової топології/route/воркера/залежності, нуль нового QUEUE_NAME (registry — governance-protected). +0 DB-конектів (SELECT на вже-взятому client), +1 outbound на вже-інтегрований Telegram endpoint лише на рідкісний `/start`.
- **(+)** OFF (default) → байт-в-байт як зараз (tree-inert модуль). ON → чисто-адитивно; фейл проковтнуто, `start.connected` не страждає.
- **(+)** SELECT slug коректний під FORCE RLS завдяки `public_select ON locations USING(true)` (міграція `1780338909301`) — типовий «0 rows без GUC» клас тут НЕ виникає (перевірено, у DoD).
- **(−)** +1 послідовний outbound-виклик у webhook-обробнику (мітигація: рідкісний, 200 незалежно, inner catch, опційний `Promise.race`-timeout).
- **(neutral)** Menu-button — стан на боці Telegram (per-chat), не в нашій БД; нема чого мігрувати/бекапити.

## Alternatives rejected

- **B — pg-boss async job (`NOTIFY_TELEGRAM_SEND`-патерн).** Over-engineering для рідкісної, природно-ідемпотентної, природно-ретраюваної best-effort-події: новий QUEUE_NAME у governance-protected registry + worker + retry/DLQ-семантика без користувацької вигоди. Вмикає рантайм там, де вимога його не потребує.
- **C — owner-triggered `/admin`-endpoint.** Найбільший blast radius (нова auth-поверхня + Zod + FE + i18n + E2E) для того самого ефекту. **Defer-flag** як майбутній UX-апгрейд (owner-контроль над text/re-set), не зараз.

## ⚖️ Ethical / scope note

Зміна суто-адитивна, реверсивна флипом, нуль незворотних наслідків, нуль PII (slug публічний), нуль money. Жодного ETHICAL-STOP. Єдине product-питання (не етичне, не блокуюче): аудиторія menu-button (owner-preview vs customer-facing) — винесено як OPEN-ризик R1 (owner: Product) у proposal §10.

## Verification (DoD — design-time)

- OFF-parity: `/start` flow байт-в-байт (нуль нового виклику/SELECT) — assert у webhook-тесті.
- ON-happy: target upsert → `start.connected` → `setChatMenuButton` з коректним тілом.
- ON-degraded: `callTelegramApi` кидає 429 → `start.connected` надіслано, зовнішній catch НЕ спрацював (нуль `msg.error`), webhook = 200.
- slug=NULL → виклик пропущено.
- RLS: `SELECT slug FROM locations WHERE id=$1` на operational-ролі під FORCE → рядок є.
- pure-функції: unit-тести форми URL/тіла, нуль I/O.
- Enable-gate: operator manual test у `docs/design/channel-hub/TMA-VALIDATION.md` (BotFather → /start → кнопка → відкриває правильний живий `/s/:slug`) + звірка резолвленого `APP_BASE_URL` host ПЕРЕД `TMA_ENABLED=on`; Product підтверджує аудиторію (R1).

## Open / accepted risks

- **R1 (OPEN, Product):** аудиторія owner-side vs customer-facing — підтвердити перед флипом.
- **R2 (accept, Backend):** `callTelegramApi` без timeout (репо-рівень, pre-existing) — не новий клас hang; опційне `Promise.race`-hardening.
- **R3 (accept, Backend):** `APP_BASE_URL` мусить резолвитись на storefront-host (`/s/:slug` SSR) — звірка в TMA-VALIDATION.
- **R4 (precondition fix-in-PR, Backend):** додати `TMA_ENABLED` у EnvSchema (наразі відсутній).
