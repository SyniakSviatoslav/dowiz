# RoPA — Реєстр діяльності з обробки (Art 30 GDPR / Закон 124/2024)
> Тригер: нова/змінена обробка або PII-поле. Тримати поточним; віддається Commissioner на запит.
> Controller: [SHPK ____] · DPO: [не призначено / ____] · Останній review: 2026-06-21

Один рядок = одна діяльність з обробки. Виведено з data-map.md.

| # | Діяльність | Роль | Субʼєкт | Категорії даних | Мета | Підстава | Отримувачі/суб-процесори | Ретенція | Тех.заходи |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Приймання й виконання замовлення | Processor (owner=ctrl) | Клієнт | імʼя, тел, адреса, гео, інструкції | доставка | договір | Supabase, Telegram, ORS | retention_days (def 365)→анонім | RLS FORCE, шифр-at-rest, claim-check |
| 2 | Облік власника/курʼєра + автентифікація | Controller | Власник/курʼєр | email, telegram_id, тел, hash паролю/TOTP | доступ до платформи | договір | Supabase, Google/Telegram | строк акаунту | шифр, hash |
| 3 | Трекінг GPS курʼєра | Controller | Курʼєр | lat/lng | live-ETA | [договір+LI ____] | Supabase, ORS | 24г | guard (active-delivery)+24г+обмеж.доступ; DPIA |
| 4 | Анти-фрод (IP/velocity/OTP) | Controller (phone=owner) | Клієнт/D | ip_hash, phone_hash | запобігання зловживанню | LI | Supabase | retention_days / 24г (velocity) | hash, advisory-only |
| 5 | Аналітика/телеметрія | Controller | D (anon) | anon_id, ip_hash, session_id | продуктова аналітика | LI | Supabase | [____] | без ідентичності |
| 6 | Lead-capture доступу | Controller | особа | email, ip_hash, consent_at | реєстрація інтересу | consent | Resend | 12 міс auto-erase | uniform-200, claim-check, day-one+auto erasure |
| 7 | Рейтинги/відгуки замовлення | Processor (owner=ctrl) | Клієнт→курʼєр | rating, feedback (free-text) | якість сервісу | договір/LI | Supabase | з замовленням | RLS |
| 8 | Push-сповіщення клієнту | Processor (owner=ctrl) | Клієнт/D | push token, fingerprint, vapid endpoint | статус замовлення | consent | Supabase, Web Push (VAPID) | до відписки | token шифр; payload без PII |
| 9 | Сповіщення власнику (Telegram) | Processor (owner=ctrl) | Клієнт (у тілі) | order#, сума; адреса/тел лише за auth-лінком (P0-4) | оповіщення про замовлення | договір | Telegram | транзитне | мінімізація тіла + auth deep-link |
| 10 | Трекінг-лінки замовлення | Processor (owner=ctrl) | Клієнт | token_hash | доступ клієнта до статусу | договір | Supabase | expires_at purge | single-use, hashed |
| 11 | Обробка запитів на стирання (DSAR) | Processor (owner=ctrl) | Клієнт | subject_phone, customer_id | право на стирання | юр.обовʼязок | Supabase | audit | anonymizer + audit log |
| 12 | Ретенція/анонімізація | Processor/Controller | Клієнт/курʼєр | всі PII | мінімізація/строк | юр.обовʼязок/LI | Supabase | за політикою | cron anonymizer + GPS purge + access-request sweep |
| 13 | Помилки/моніторинг | Controller | (PII scrubbed) | стек, контекст (redacted) | надійність | LI | Sentry | [____] | PII-scrub (regex) |
| 14 | Зберігання зображень (меню/аватар/фото підʼїзду) | Processor/Controller | Клієнт/власник | image, photo, avatar | сторфронт/доставка | договір/LI | Cloudflare R2 | до видалення/анонім | avatar purge on anonymize |

> Зверни увагу: рядок 9 (Telegram) лишається суб-процесором із PII у тілі лише на рівні
> order#/суми за замовчуванням (P0-4); повні дані — за автентифікованим лінком.
