# RoPA — Реєстр діяльності з обробки (Art 30 GDPR / Закон 124/2024)
> Тригер: нова/змінена обробка або PII-поле. Тримати поточним; віддається Commissioner на запит.
> Controller: [SHPK ____] · DPO: [не призначено / ____] · Останній review: [____]

Один рядок = одна діяльність з обробки.

| # | Діяльність | Роль | Субʼєкт | Категорії даних | Мета | Підстава | Отримувачі/суб-процесори | Ретенція | Тех.заходи |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Приймання й виконання замовлення | Processor (owner=ctrl) | Клієнт | імʼя, тел, адреса, гео | доставка | договір | Supabase, Telegram, ORS | retention_days (def 365)→анонім | RLS, шифр-at-rest |
| 2 | Облік власника/курʼєра + автентифікація | Controller | Власник/курʼєр | email, telegram_id, тел, hash паролю/TOTP | доступ до платформи | договір | Supabase, Google/Telegram | строк акаунту | шифр, hash |
| 3 | Трекінг GPS курʼєра | Controller | Курʼєр | lat/lng | live-ETA | [договір+LI ____] | Supabase, ORS | 24г | guard+24г+обмеж.доступ; DPIA |
| 4 | Анти-фрод (IP/velocity/OTP) | Controller (phone=owner) | Клієнт/D | ip_hash, phone_hash | запобігання зловживанню | LI | Supabase | retention_days | hash |
| 5 | Аналітика/телеметрія | Controller | D (anon) | anon_id, ip_hash | продуктова аналітика | LI | Supabase | [____] | без ідентичності |
| 6 | Lead-capture доступу | Controller | особа | email, ip_hash | реєстрація інтересу | consent | Resend | 12 міс auto-erase | — |
| ... | [решта з data-map] |  |  |  |  |  |  |  |  |
