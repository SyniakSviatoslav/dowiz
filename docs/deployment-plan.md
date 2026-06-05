# План розгортання DeliveryOS (Go-Live)

Цей документ описує покроковий процес розгортання проєкту DeliveryOS на цільовій інфраструктурі (Supabase Free, Fly.io, Cloudflare) відповідно до архітектурних рішень Фази 5 (v4.5).

## 1. Підготовка Бази Даних (Supabase Free)

Оскільки проєкт розрахований на запуск N=1 з використанням Supabase Free, вкрай важливо правильно налаштувати Connection Pooling та бекапи.

**Кроки:**
1. Створіть новий проєкт на Supabase.
2. Перейдіть до налаштувань Database та увімкніть Connection Pooling (Supavisor).
3. Отримайте три варіанти рядків підключення (Connection Strings):
   - **Transaction Pooler (порт 6543):** Для `***REDACTED***`. Максимум 8 з'єднань (operational budget).
   - **Session Pooler (порт 5432):** Для `***REDACTED***`. Максимум 3 з'єднання.
   - **Direct Connection:** Для `***REDACTED***`. Використовується лише під час CI/CD або ручного запуску міграцій.
4. **Безпека:** Створіть окрему роль у PostgreSQL для операційного пулу (щоб не використовувати `postgres` superuser, який обходить RLS). RLS (Row Level Security) базується на змінних сесії (наприклад, `SET LOCAL app.current_tenant`).
5. Запустіть міграції для ініціалізації схеми:
   ```bash
   pnpm run migrate:up
   ```

## 2. Налаштування Storage та Бекапів (Cloudflare R2)

Supabase Free не має керованих бекапів (PITR), тому R2 є єдиною страхувальною сіткою (recovery-net), яку підтримує Worker.

**Кроки:**
1. У Cloudflare створіть новий R2 Bucket (наприклад, `deliveryos-backups`).
2. Згенеруйте Access Keys (надайте права на читання та запис для цього бакета).
3. Збережіть `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` та `R2_ENDPOINT`.

## 3. Розгортання Backend та Worker (Fly.io)

Проєкт налаштовано як monorepo, де backend (`web`) та фонові задачі (`worker`) компілюються в окремі файли, але живуть в одному Docker-контейнері (`Dockerfile` та `fly.toml`).

**Кроки:**
1. Встановіть `flyctl` та авторизуйтесь (`fly auth login`).
2. Ініціалізуйте додаток у регіоні `fra` (Франкфурт):
   ```bash
   fly launch --no-deploy
   ```
3. Встановіть необхідні секрети:
   ```bash
   fly secrets set \
     ***REDACTED***="<string>" \
     ***REDACTED***="<string>" \
     ***REDACTED***="<string>" \
     ***REDACTED***="<secret_key_rs256_or_hs256>" \
     JWT_KID="v1" \
     ***REDACTED***="<google_oauth_client_id>" \
     ***REDACTED***="<google_oauth_secret>" \
     APP_BASE_URL="https://api.dowiz.org" \
     R2_ACCESS_KEY_ID="<key>" \
     R2_SECRET_ACCESS_KEY="<secret>" \
     R2_ENDPOINT="<cloudflare_r2_endpoint>" \
     R2_BUCKET="deliveryos-backups"
   ```
4. Виконайте деплой:
   ```bash
   fly deploy
   ```
   *Fly.io запустить 2 процеси згідно з `fly.toml`: `web` (Fastify 5) та `worker` (pg-boss).*

## 4. Розгортання Frontend PWA

Клієнтський та адмінський інтерфейси (React 18 + Vite 6) розгортаються як статичні сайти.

**Кроки:**
1. Підключіть репозиторій до Cloudflare Pages (або Vercel).
2. Налаштуйте build-команду:
   - Build Command: `pnpm --filter @deliveryos/web run build` (перевірте актуальну команду в `apps/web/package.json`).
   - Output Directory: `apps/web/dist`.
3. Встановіть змінні оточення (наприклад, `VITE_API_BASE_URL=https://api.dowiz.org`).
4. Налаштуйте DNS-записи для домену (наприклад, `dowiz.org`, `slug.dowiz.org`) у Cloudflare та увімкніть SSL.

## 5. Post-Deploy та Go-Live Gate

Згідно з інструкціями Фази 5 (Етапи 30-35), перед повноцінним запуском необхідно виконати аудит:
1. **Worker Liveness:** Перевірте логи Fly.io, чи воркер коректно стартував і регулярно подає сигнали (observability).
2. **Backup Test:** Запустіть скрипт тестування відновлення, щоб переконатися, що R2 бекапи працюють:
   ```bash
   pnpm run backup:drill
   ```
3. **Smoke Test:** Виконайте "перше реальне платне замовлення" від тестового клієнта, щоб перевірити весь ланцюжок: UI → POST /orders → Worker Timeout → WS-сповіщення → Telegram.
4. **Моніторинг лімітів:** Встановіть алерти у Supabase на досягнення 80% використання бази даних (Free tier) та пулу з'єднань.
