# Карта даних (PII inventory) — джерело істини
> Тригер: будь-яка нова PII-колонка/поле або зміна потоку. Регенеруй сканом коду, не вручну.
> Останній скан: 2026-06-21 · коміт: feat/golive-remediation @ P0-privacy-hardening
> Метод: code-grounded (migrations + Zod + routes + packages/platform seams + localStorage).

Subject: **C**=клієнт · **Cr**=курʼєр · **O**=власник · **D**=пристрій/anon.
Підстава: договір / згода (consent) / легітимний інтерес (LI). [HIGH-RISK]/[special] позначено.

| # | Елемент (table.field / система) | Субʼєкт | Збір + підстава | Зберігання + суб-процесори | Ретенція | Controller + чому |
|---|---|---|---|---|---|---|
| 1 | customers.phone, .name | C | POST /api/orders; договір | customers (Supabase); → Telegram | retention_days (def 365)→анонім | **Owner** / DeliveryOS=processor — клієнт власника |
| 2 | customers.no_show_count, completed_count, last_no_show_at | C | derived; LI | customers | з клієнтом | Owner |
| 3 | customers.marketing_opt_in, loyalty_points | C | order flow; **consent** | customers | з клієнтом | Owner (маркетинг власника) |
| 4 | orders.delivery_address, delivery_lat/lng, delivery_instructions [HIGH-RISK] | C | POST /api/orders; договір | orders; → Telegram (адреса), → ORS (lat/lng) | retention_days → anonymized_at NULLs | Owner / DeliveryOS=processor + ORS sub-proc |
| 5 | orders.delivery_photo_key → R2 [HIGH-RISK, grey] | C/сторонні | POST /api/orders; договір/LI | R2 (Cloudflare) | з замовленням | Owner — DPIA-кандидат (фото підʼїзду) |
| 6 | orders.cash_pay_with | C | order; договір | orders | з замовленням | Owner (cash-only → нема карткового PII) |
| 7 | orders.client_ip_hash | C/D | order (anti-fake-seam); LI | orders (NULLed on anonymize) | retention_days | **DeliveryOS** — платформне анти-фрод рішення |
| 8 | order_ratings.feedback, rating, customer_id, courier_id | C→Cr | POST /…/rating; договір/LI | order_ratings | з замовленням | Owner / DeliveryOS=processor |
| 9 | phone_otp.phone+code_hash, customer_otp_sessions.phone_hash, velocity_events.phone_hash+client_ip_hash | C | /otp/send|verify; LI/договір | відповідні таблиці (hashed) | OTP ~хв; velocity 24г | **Mixed**: phone=Owner; ip_hash+механізм=DeliveryOS |
| 10 | customer_signals.* (PII-free evidence) | C | derived; LI | customer_signals | з клієнтом | DeliveryOS (trust/safety), advisory-only |
| 11 | customer_devices.token_encrypted, fingerprint, push_subscription, vapid_endpoint, keys_* | C/D | push opt-in; **consent** | customer_devices; → web-push | до відписки | Owner (канал власника) / processor |
| 12 | customer_track_grants.token_hash | C | order mint; договір | customer_track_grants | expires_at purge | Owner / DeliveryOS=processor |
| 13 | gdpr_erasure_requests.subject_phone, customer_id, requested_by_owner_id | C+O | POST /…/gdpr-requests; **юр.обовʼязок** | gdpr_erasure_requests | audit | Owner (його erasure-обовʼязок); механізм=DeliveryOS |
| 14 | MessageBus payload (P0-3: лише статус/ids — БЕЗ name/phone/address/item-names) | C | runtime; договір | pg NOTIFY / Upstash (claim-check) | transient | Owner / DeliveryOS — **PII прибрано (P0-3)** |
| 15 | users.email, google_sub, telegram_user_id, display_name, phone, password_hash, totp_secret_enc | O | auth.ts (Google/Telegram/local); договір | users; → Google/Telegram | строк акаунту | **DeliveryOS** — платформний акаунт |
| 16 | owner_notification_targets.address (telegram/push) | O | owner setup; договір | таблиця; → Telegram | строк акаунту | DeliveryOS (платформна фіча), субʼєкт=власник |
| 17 | couriers.*_encrypted (name/email/phone), password_hash, last_login_at, deactivated_* [grey] | Cr | /courier/auth redeem/login; договір | couriers (шифр-at-rest) | строк акаунту | **Joint/DeliveryOS-leaning** — платформний акаунт, власник наймає курʼєра |
| 18 | courier_positions.lat/lng/accuracy [HIGH-RISK] | Cr | courier app → /shifts/ping; договір/LI | courier_positions; → ORS | **24г** (P0-1 named const) | **DeliveryOS** — платформний трекінг; **DPIA** |
| 19 | courier_sessions.ip_hash/user_agent_hash; courier_audit_log.ip_hash/user_agent_hash | Cr | auth; LI | відповідні (hashed) | — | DeliveryOS (платформна безпека) |
| 20 | analytics_events / analytics_cwv / analytics_abuse_log — ip_hash, anon_id, session_id | D | POST /api/telemetry; LI | analytics (no RLS) | [____] | DeliveryOS — платформна аналітика |
| 21 | access_requests.email, ip_hash, consent_at, privacy_version | особа | POST /api/access-requests; **consent** | access_requests; → Resend | **12 міс** auto-erase | DeliveryOS — платформний lead-capture |
| 22 | telegram_login_tokens / telegram_connect_tokens (chat_id) | O | auth; договір | таблиці | short-TTL | DeliveryOS |
| 23 | upload_audit.uploaded_by, file_name (free-text) | O | upload; LI | upload_audit | audit | DeliveryOS |
| 25 | locations.phone, public_phone, address (business contact, not personal PII) | O | onboarding; договір | locations; → storefront/Telegram | строк акаунту | Owner — власний бізнес-лістинг закладу |
| 24 | localStorage dos_checkout_draft_* (phone/name/messenger), dos_last_delivery_* (lat/lng/address) [HIGH-RISK] | C | браузер (CheckoutPage.tsx); consent/договір | пристрій (нема cookies) | до очистки | Owner (storefront власника), дані на пристрої |

## Прогалини / неоднозначності (потрібен юрист)
- **Курʼєр (17,18,19)**: joint vs DeliveryOS vs owner — курʼєр доставляє для закладу, але онбординг/auth/GPS/логи робить платформа. Ймовірний joint-controllership → закріпити контрактом.
- **Анти-фрод split (9)**: phone_hash клієнта (owner) vs ip_hash/механізм (платформа) в одних таблицях під різними контролерами.
- **MessageBus (14)**: внутрішнє, після P0-3 PII прибрано — задокументоване рішення.

## Корективи до seed-тверджень (код ≠ seed)
- **Geocode.maps.co НЕ в коді** — лише OpenRouteService (ROUTING_*), отримує lat/lng (не адресу).
- **Courier GPS ретенція = 24г hard-coded** (не 1р).
- **google_sub + email досі є** на users поряд із telegram_user_id.
- **couriers.photo / .status (як у seed) НЕ існують** — є full_name/email/phone _encrypted + deactivated_at.
