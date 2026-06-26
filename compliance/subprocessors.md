# Реєстр суб-процесорів
> Тригер: новий зовнішній виклик/інтеграція. Кожен, хто торкається даних, тут. Несе PII → потрібен DPA.
> Останній review: 2026-06-21

| Процесор | Дані | Мета | Локація | Env | PII? | DPA/статус |
|---|---|---|---|---|---|---|
| Supabase (Postgres) | всі | основне сховище | [EU ____] | DATABASE_URL_* | так | [____] |
| Fly.io | хостинг | компʼют | EU (fra) | FLY_MACHINE_ID | так | [____] |
| Upstash Redis | кеш маршрутів; pub/sub payload = лише ID/статус (claim-check ВЖЕ shipped, P0-3) | pub/sub+кеш | [____] | REDIS_URL | ні (PII прибрано P0-3) | [____] |
| Cloudflare R2 | зображення, фото підʼїзду, аватари, theme | обʼєктне сховище | [____] | R2_* | так | [____] |
| Resend | email lead (1 поле) | алерт оператору | [____] | RESEND_API_KEY | мін | [____] |
| Sentry | помилки (scrubbed) | моніторинг | [____] | SENTRY_DSN | ні* | [____] |
| Telegram | алерт власнику (імʼя/тел/адреса) | сповіщення | [UAE ____] | TELEGRAM_BOT_TOKEN | так [HIGH] | [____] |
| OpenRouteService | lat/lng (не адреса) | ETA | [____] | ROUTING_* | так | [haversine-fallback ____] |
| Groq/OpenAI | текст меню + контакт власника | OCR/онбординг | [____] | GROQ_/OPENAI_* | ні (нуль клієнт-PII) | [____] |
| OpenCode Zen | текст меню (raw, un-redacted) + контакт власника | OCR-структуризація (preferred) | [____] | OPENCODE_ZEN_API_KEY | можливо [owner-uploaded doc може містити 3-ю-сторонню PII; notice на upload] | [____] |
| OpenRouter | текст меню (raw) + контакт власника; UI-скріншоти SEED/synthetic стану (vision QA) — без production-PII | OCR-структуризація (fallback); UI screenshot review (vision QA) — SEED/synthetic state only, never production data | per OpenRouter | OPENROUTER_API_KEY | можливо [як вище; vision QA = seed-only, нуль prod-PII, масковано перед egress] | [seed-only, no prod data ____] |
| Web Push (VAPID) | order id/сума | push | — | VAPID_* | ні | — |

* Sentry scrub regex-based — перевіряй покриття.
WhatsApp/Baileys — ВИДАЛЕНО (P0-2).

**Vision/axe QA hard rule (OpenRouter vision):** the Non-Pixel Verification Net's vision review
(and any agent-as-eye screenshot review) runs ONLY against SEED/synthetic state — never production
data. Screenshots are masked for any seed-PII (customer name/phone/address/contact) before egress to
the OpenRouter-hosted vision model (see `e2e/visual/harness.ts` → `maskPII` / `[data-pii]`). No
production screenshot may ever be sent to a vision subprocessor.
