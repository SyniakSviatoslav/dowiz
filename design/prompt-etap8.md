# Промпт · Етап 8 — Міграція-розширення: фундамент Фази 2 (P2-0)

> Призначення: віддати кодинг-агенту **після проходження DoD-gate Фази 1** (Phase 0 exit-аудит → GO). Обсяг — **тільки Етап 8**. Самодостатній: усі DDL-контракти вшиті. Усе нижче — як єдиний промпт.

---

## РОЛЬ І КОНТЕКСТ

Ти продовжуєш **DeliveryOS**. Phase 0 (Етапи 1–7) завершена: walking skeleton зелений, міграції 001–009 застосовані, реальні шви `MessageBus`/`QueueProvider`, заглушки `NotificationProvider`/`GeocodingProvider`/`ThemeRenderer`. Phase 0 exit-аудит → GO зафіксовано.

Зараз **Етап 8: міграція-розширення Фази 2**. Це **фундамент** для всіх наступних етапів (8 → 9 → 10 → ... → 16). Помилка в схемі чи RLS виправляється міграцією на живих даних. Роби повільно й точно.

Створюєш міграції **010–017** за **точними контрактами нижче**, вмикаєш RLS **у тій самій міграції**, що створює tenant-таблицю, і доводиш ізоляцію **емпірично**. **Жодного рантайму** — лише схема, RLS і seed-розширення.

**Несуче правило RLS (те саме, що в Етапі 4):** «політики існують» ≠ «ізоляція працює». Gate проходить лише коли крос-tenant-читання **через операційний пул** реально повертає 0 рядків.

---

## ІНВАРІАНТИ (з Phase 0 — діють далі, не повторюються в DoD)
ESM; TS strict; Zod `.strict()` на кожному вході; параметризовані запити (нуль SQL-конкату); rate-limit на мутуючих; RLS (`SET LOCAL app.user_id` + `FORCE`); status-guarded переходи; N-safe (broadcast лише через MessageBus); **без cookies** (localStorage); `crypto.randomUUID()`; **гроші integer ALL** + `CHECK(>=0)`; секрети лише в `.env`/Fly; `kid` у JWT; `var(--brand-*)` замість хардкоду кольору. **Governance (нове):** нуль PII у будь-яку ШІ-модель; ШІ — лише контент меню.

**Дисципліна міграцій (з Етапу 2):** forward-only; застосовану міграцію **ніколи не редагувати**; деструктив — окремим файлом; усі нижче — `CREATE`. Мова — TS через `tsx`; для enum/RLS/policy/функцій — `pgm.sql()` (білдер їх погано покриває). Інструмент: node-pg-migrate через `***REDACTED***`.

**Пул-параметри** — підтверджені Phase 0 (з `docs/connection-budget.md`), не вгадані.

**Гроші:** integer ALL (ціни, дельти, доставка, discount, tax — `CHECK (>=0)`). Семантика `total`: `total = subtotal + delivery_fee + tax_total − discount_total`; `total >= 0`.

---

## ПОСЛІДОВНІСТЬ МІГРАЦІЙ

Нові міграції — 010+. Кожна атомарна, forward-only. Таємстамп — Unix ms, більший за найновішу існуючу (1780338981783).

| # | Ім'я | Зміст | RLS |
|---|---|---|---|
| 010 | `menu_modifiers` | `modifier_groups`, `modifiers`, `product_modifier_groups`, `order_item_modifiers` | Усі tenant-таблиці |
| 011 | `content_i18n` | `product_translations`, `category_translations` + `ALTER TABLE locations ADD COLUMN default_locale, supported_locales` | `*_translations` через location_id |
| 012 | `product_attributes_images` | `ALTER TABLE products ADD COLUMN attributes jsonb, image_key text` | — (розширення існуючої RLS) |
| 013 | `money_breakdown` | `ALTER TABLE orders ADD COLUMN delivery_fee, discount_total, tax_total` | — (розширення існуючої RLS) |
| 014 | `location_commerce` | `ALTER TABLE locations ADD ...` + `delivery_tiers` | `delivery_tiers` |
| 015 | `order_history` | `order_status_history` (append-only) | Так |
| 016 | `reservations_scaffold` | `reservations` (scaffold, без рантайму) | Так |
| 017 | `loyalty_seam` | `ALTER TABLE customers ADD COLUMN marketing_opt_in, loyalty_points` | — (розширення існуючої RLS) |

---

## ПАТЕРН RLS (КРИТИЧНО — реалізуй точно так, повторює Етап 4)

Застосунок (не PostgREST) ставить session-змінну `app.user_id` у транзакції. Політики читають її **через SECURITY DEFINER-функції** (вже створені в міграції 002 — `app_current_user()`, `app_member_location_ids()`).

Для **кожної нової tenant-таблиці**, що має `location_id`:
```sql
ALTER TABLE <t> ENABLE ROW LEVEL SECURITY;
ALTER TABLE <t> FORCE  ROW LEVEL SECURITY;   -- ОБОВ'ЯЗКОВО
CREATE POLICY tenant_isolation ON <t>
  USING ( location_id IN (SELECT app_member_location_ids()) );
```

Для `order_item_modifiers` (без прямого `location_id`):
```sql
ALTER TABLE order_item_modifiers ENABLE ROW LEVEL SECURITY;
ALTER TABLE order_item_modifiers FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON order_item_modifiers
  USING ( order_item_id IN (
    SELECT oi.id FROM order_items oi
    JOIN orders o ON o.id = oi.order_id
    WHERE o.location_id IN (SELECT app_member_location_ids())
  ));
```

Для `order_status_history` (має `location_id` прямо):
```sql
ALTER TABLE order_status_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE order_status_history FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON order_status_history
  USING ( location_id IN (SELECT app_member_location_ids()) );
```

**Передумова коректності (перевір і не обходь):** роль, якою конектиться операційний пул, **не суперюзер** і **без `BYPASSRLS`** (інакше RLS не діє взагалі). `FORCE` робить RLS чинною для власника таблиць. Це має бути підтверджено з Етапу 4; якщо емпіричний тест нижче покаже, що крос-tenant **не** блокується — **СТОП**, не йди далі: спершу полагодь привілеї ролі.

---

## DDL-КОНТРАКТИ (міграції мають дати точно це)

### 010 — `menu_modifiers`

```sql
CREATE TABLE modifier_groups (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id uuid NOT NULL REFERENCES locations(id),
  name text NOT NULL,
  min_select int NOT NULL DEFAULT 0 CHECK (min_select >= 0),
  max_select int NOT NULL DEFAULT 1 CHECK (max_select >= min_select),
  required boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE modifiers (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  group_id uuid NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
  location_id uuid NOT NULL REFERENCES locations(id),
  name text NOT NULL,
  price_delta integer NOT NULL DEFAULT 0 CHECK (price_delta >= 0),
  available boolean NOT NULL DEFAULT true,
  sort_order int NOT NULL DEFAULT 0
);

CREATE TABLE product_modifier_groups (
  product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
  group_id   uuid NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
  sort_order int NOT NULL DEFAULT 0,
  PRIMARY KEY (product_id, group_id)
);

CREATE TABLE order_item_modifiers (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  order_item_id uuid NOT NULL REFERENCES order_items(id) ON DELETE CASCADE,
  modifier_id uuid REFERENCES modifiers(id),
  name_snapshot text NOT NULL,
  price_delta_snapshot integer NOT NULL CHECK (price_delta_snapshot >= 0)
);
```

**RLS 010:** tenant_isolation на `modifier_groups`, `modifiers`, `product_modifier_groups` (усі мають `location_id` прямо або через join). `order_item_modifiers` — політика через EXISTS-join (як у паттерні вище).

### 011 — `content_i18n`

```sql
CREATE TABLE product_translations (
  product_id uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
  locale text NOT NULL,
  name text NOT NULL,
  description text,
  PRIMARY KEY (product_id, locale)
);

CREATE TABLE category_translations (
  category_id uuid NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
  locale text NOT NULL,
  name text NOT NULL,
  PRIMARY KEY (category_id, locale)
);

ALTER TABLE locations
  ADD COLUMN default_locale text NOT NULL DEFAULT 'sq',
  ADD COLUMN supported_locales text[] NOT NULL DEFAULT ARRAY['sq','en'];
```

**RLS 011:** `product_translations` — через `product_id IN (SELECT id FROM products WHERE location_id IN (SELECT app_member_location_ids()))`. `category_translations` — аналогічно через `category_id → categories → location_id`.

### 012 — `product_attributes_images`

```sql
ALTER TABLE products
  ADD COLUMN attributes jsonb NOT NULL DEFAULT '{}'::jsonb,
  ADD COLUMN image_key text;
```
RLS: не потрібна — розширює існуючу `products`, де RLS вже є з Етапу 4.

### 013 — `money_breakdown`

```sql
ALTER TABLE orders
  ADD COLUMN delivery_fee   integer NOT NULL DEFAULT 0 CHECK (delivery_fee   >= 0),
  ADD COLUMN discount_total integer NOT NULL DEFAULT 0 CHECK (discount_total >= 0),
  ADD COLUMN tax_total      integer NOT NULL DEFAULT 0 CHECK (tax_total      >= 0);
```
RLS: не потрібна — розширює існуючу `orders`.

### 014 — `location_commerce`

```sql
ALTER TABLE locations
  ADD COLUMN currency_code text NOT NULL DEFAULT 'ALL',
  ADD COLUMN currency_minor_unit int NOT NULL DEFAULT 0,
  ADD COLUMN tax_rate numeric NOT NULL DEFAULT 0,
  ADD COLUMN price_includes_tax boolean NOT NULL DEFAULT true,
  ADD COLUMN min_order_value integer,
  ADD COLUMN free_delivery_threshold integer,
  ADD COLUMN delivery_fee_flat integer,
  ADD COLUMN delivery_polygon jsonb;

CREATE TABLE delivery_tiers (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id uuid NOT NULL REFERENCES locations(id),
  max_distance_km numeric NOT NULL,
  fee integer NOT NULL CHECK (fee >= 0),
  min_order integer
);
```

**RLS 014:** `delivery_tiers` — `location_id IN (SELECT app_member_location_ids())`. Розширення `locations` — RLS вже є.

### 015 — `order_history` (append-only)

```sql
CREATE TABLE order_status_history (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  order_id uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
  location_id uuid NOT NULL,
  from_status order_status,
  to_status   order_status NOT NULL,
  actor text,
  created_at timestamptz NOT NULL DEFAULT now()
);
```

**RLS 015:** `location_id IN (SELECT app_member_location_ids())`.

### 016 — `reservations_scaffold` (нульова вага)

```sql
CREATE TABLE reservations (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  location_id uuid NOT NULL REFERENCES locations(id),
  customer_id uuid REFERENCES customers(id),
  slot_at timestamptz NOT NULL,
  party_size int NOT NULL,
  status text NOT NULL DEFAULT 'pending',
  created_at timestamptz NOT NULL DEFAULT now()
);
```

**RLS 016:** `location_id IN (SELECT app_member_location_ids())`.

### 017 — `loyalty_seam` (нульова вага)

```sql
ALTER TABLE customers
  ADD COLUMN marketing_opt_in boolean NOT NULL DEFAULT false,
  ADD COLUMN loyalty_points integer NOT NULL DEFAULT 0 CHECK (loyalty_points >= 0);
```
RLS: не потрібна — розширює існуючу `customers`.

---

## ДОКУМЕНТАЦІЯ СЕМАНТИКИ `total`

Після міграції 013, оновити `CONVENTIONS.md` — додати блок (або створити `docs/money-semantics.md`):

```
## Money Semantics (integer ALL)

Всі грошові значення — integer в мінімальній одиниці валюти
(для ALL — lek, minor_unit = 0).

### `total` розрахунок (orders)
  total = subtotal + delivery_fee + tax_total − discount_total
  total >= 0

### ПДВ
- tax_rate — дробове число (напр. 0.20 для 20%).
- price_includes_tax = true → tax_total міститься в subtotal (якщо рахувати, то tax_total = round(subtotal − subtotal / (1 + tax_rate))).
- price_includes_tax = false → tax_total = round(subtotal × tax_rate).
- Округлення: half-up (централізована утиліта).

### Доставка
- min_order_value: замовлення нижче цього → відмова з поясненням.
- free_delivery_threshold: subtotal вище цього → delivery_fee = 0.
- delivery_fee_flat: фіксована доставка (якщо не distance-tier).
- delivery_tiers: distance-зонування.
```

---

## SEED (розширення існуючого `scripts/seed.ts`)

Додати до існуючого seed-скрипту (він вже створює 1 організацію, 2 локації, owner A/B, кур'єра, 1 категорію, 2 продукти):

1. **Modifier groups + modifiers** на `demo`:
   - 1 група «Розмір» (min=1, max=1, required=true) → опції «Мала» (+0), «Велика» (+300)
   - 1 група «Додатки» (min=0, max=3, required=false) → опції «Сирок» (+200), «Гриби» (+150)
   - Зв'язати з продуктом «Margherita» через `product_modifier_groups`

2. **`order_status_history`** — один запис для існуючого demo order (якщо є в seed; якщо seed не створює orders — не додавай, це окремий скрипт)

3. **Переклади** — `product_translations` для «Margherita» (en → "Margherita", опис "Tomato sauce, mozzarella") і `category_translations` (en → "Pizzas").

4. **`delivery_tiers`** — один запис для demo (max_distance_km=5, fee=200).

5. **`reservations`** — один scaffold-запис для demo (slot_at = now() + 1 day, party_size=4).

Seed працює через session-пул (5432) і вставляє привілейованим шляхом (як адмін-операція). Після апсейту виводить усі UUID (нові сутності теж).

---

## ВЕРИФІКАЦІЯ

### `scripts/verify-rls.ts` (розширити існуючий)

Додати до існуючого verify-rls-скрипту перевірки на **кожну нову tenant-таблицю**:

```typescript
// Після існуючих перевірок для таблиць Phase 0
const TENANT_TABLES = [
  'modifier_groups', 'modifiers', 'product_modifier_groups',
  'order_item_modifiers', 'order_status_history',
  'delivery_tiers', 'reservations',
  'product_translations', 'category_translations',
];

// Для кожної таблиці:
// 1. SET app.user_id = ownerA → SELECT * → бачить лише demo
// 2. SET app.user_id = ownerB → SELECT * → 0 рядків demo
// 3. Без app.user_id → 0 рядків
```

Логіка та сама, що в оригінальному `verify-rls.ts`: скрипт падає з кодом ≠ 0, якщо будь-де ізоляція протекла.

---

## ACCEPTANCE (Чекпойнт 8 — GATE; покажи доказ кожного)

1. **Міграції застосовані:** `pnpm migrate:up` на стані-після-009 проходить 010→017; `pnpm migrate:status` показує всі; `pnpm migrate:down` на останній — чисто (forward-only, down пустий).
2. **Структура вірна:** у БД усі таблиці, FK, UNIQUE, enum (нові не потрібні), `CHECK (>=0)` на грошах — точно за контрактами.
3. **RLS працює ЕМПІРИЧНО:** `pnpm verify:rls` зелений — owner A не бачить даних owner B на жодній новій tenant-таблиці; без `app.user_id` — 0 рядків; `order_item_modifiers`-join-політика теж ізолює.
4. **Семантика `total` задокументована** в `CONVENTIONS.md` або `docs/money-semantics.md`.
5. **Seed:** `pnpm seed` проходить ідемпотентно (другий запуск не падає); додає модифікатори, переклади, delivery_tiers, reservations, order_status_history.
6. **Scaffold чистий:** `reservations` існує, код її не чіпає; `loyalty_seam` додав колонки, рантайм відсутній.
7. **Регресій немає:** `pnpm -r build`, `pnpm lint`, `pnpm lint:gates`, `pnpm verify:env`, `pnpm verify:db` — зелені.

---

## OUT OF SCOPE (НЕ роби)

- **Жодного рантайму:** без ендпоінтів, CRUD, сервісів, бізнес-логіки, валідаторів, middleware.
- **Жодної логіки `menu_version++`** — це Етап 9 (або робиться тригером/кодом).
- **Жодної логіки ціноутворення/доставки/ПДВ** — це Етап 10.
- **Жодної імпортної воронки** — це Етап 11.
- **Жодної ШІ-логіки** — це Етап 12.
- **Жодної логіки сповіщень** — це Етап 16.
- **Жодних кастомних доменів** — це пізніше (custom hostnames).
- **Жодних тригерів БД** для бізнес-правил (menu_version, статус-переходи).
- **Не «розширювати» scaffold-таблиці** (`reservations`, `delivery_polygon`, `loyalty_seam`) понад мінімум — нульова вага.

Якщо щось здається потрібним «щоб запрацювало» — мінімальний placeholder + `// TODO: Етап N`, але не реалізовуй.

---

## DEFINITION OF DONE

Вісім міграцій (010–017) створюють схему **точно за контрактами**; RLS увімкнено з `FORCE` у тій же міграції, що й tenant-таблиця, через існуючі SECURITY DEFINER-функції; `pnpm verify:rls` **емпірично** доводить крос-tenant-ізоляцію на всіх нових таблицях (0 рядків); seed розширено модифікаторами, перекладами, delivery_tiers, reservations, order_status_history; семантика `total` задокументована; scaffold присутній без рантайм-впливу; усі 7 acceptance-перевірок зелені; інваріанти Phase 0 цілі. Нічого з OUT OF SCOPE не реалізовано.
