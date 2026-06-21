# Реєстр суб-процесорів
> Тригер: новий зовнішній виклик/інтеграція. Кожен, хто торкається даних, тут. Несе PII → потрібен DPA.
> Останній review: [____]

| Процесор | Дані | Мета | Локація | Env | PII? | DPA/статус |
|---|---|---|---|---|---|---|
| Supabase (Postgres) | всі | основне сховище | [EU ____] | DATABASE_URL_* | так | [____] |
| Fly.io | хостинг | компʼют | EU (fra) | FLY_MACHINE_ID | так | [____] |
| Upstash Redis | кеш маршрутів (+payload доки не claim-check) | pub/sub+кеш | [____] | REDIS_URL | [після P0-3: ні] | [____] |
| Cloudflare R2 | зображення, фото підʼїзду, аватари, theme | обʼєктне сховище | [____] | R2_* | так | [____] |
| Resend | email lead (1 поле) | алерт оператору | [____] | RESEND_API_KEY | мін | [____] |
| Sentry | помилки (scrubbed) | моніторинг | [____] | SENTRY_DSN | ні* | [____] |
| Telegram | алерт власнику (імʼя/тел/адреса) | сповіщення | [UAE ____] | ***REDACTED*** | так [HIGH] | [____] |
| OpenRouteService | lat/lng (не адреса) | ETA | [____] | ROUTING_* | так | [haversine-fallback ____] |
| Groq/OpenAI | текст меню + контакт власника | OCR/онбординг | [____] | GROQ_/OPENAI_* | ні (нуль клієнт-PII) | [____] |
| Web Push (VAPID) | order id/сума | push | — | VAPID_* | ні | — |

* Sentry scrub regex-based — перевіряй покриття.
WhatsApp/Baileys — ВИДАЛЕНО (P0-2).
