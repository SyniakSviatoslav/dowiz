-- Catalog: the storefront's read model. Ported from the old platform's contract
-- (packages/shared-types/src/contracts/public/menu.ts on
-- origin/backup-wip-2026-07-08) and checked against the LIVE payload that
-- dowiz-staging.fly.dev still serves at /public/locations/demo/menu.
--
-- Postgres -> SQLite translation, applied consistently:
--   uuid          -> TEXT            (the ids stay the old uuids so data ports 1:1)
--   boolean       -> INTEGER 0/1     (SQLite has no bool)
--   timestamptz   -> INTEGER         (unix ms, the same unit the order log uses)
--   enum          -> TEXT + CHECK    (the constraint survives; the type does not)
--   text[]/jsonb  -> TEXT            (JSON, read with json_extract when needed)
--
-- Money is INTEGER minor units everywhere, never REAL. SQLite would happily
-- store a float here and that is exactly how money law gets lost.

CREATE TABLE IF NOT EXISTS locations (
  id                TEXT PRIMARY KEY,
  slug              TEXT NOT NULL UNIQUE,
  name              TEXT NOT NULL,
  phone             TEXT NOT NULL,
  address           TEXT,
  -- open/closed/busy drives the storefront gate. `busy` is not cosmetic: the old
  -- service doubled the confirmation timeout in that state.
  status            TEXT NOT NULL DEFAULT 'closed'
                    CHECK (status IN ('open','closed','busy')),
  closes_at         TEXT,
  timezone          TEXT NOT NULL DEFAULT 'Europe/Tirane',
  delivery_eta      TEXT NOT NULL DEFAULT '30-45',
  delivery_fee      INTEGER NOT NULL DEFAULT 0,
  min_order         INTEGER NOT NULL DEFAULT 0,
  currency_code     TEXT NOT NULL DEFAULT 'ALL',
  -- Bumped on every menu write so a client can tell a stale cart from a fresh one.
  menu_version      INTEGER NOT NULL DEFAULT 1,
  hero_image_url    TEXT,
  logo_url          TEXT,
  supported_locales TEXT NOT NULL DEFAULT '["sq","en","uk"]',
  default_locale    TEXT NOT NULL DEFAULT 'sq',
  -- Micro-degrees, integer -- same rule as the order aggregate: no floats in
  -- anything the kernel folds or replays.
  lat_udeg          INTEGER,
  lon_udeg          INTEGER,
  delivery_paused   INTEGER NOT NULL DEFAULT 0 CHECK (delivery_paused IN (0,1)),
  created_at_ms     INTEGER NOT NULL,
  updated_at_ms     INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS categories (
  id            TEXT PRIMARY KEY,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  name          TEXT NOT NULL,
  sort_order    INTEGER NOT NULL DEFAULT 0,
  created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS categories_location_idx ON categories (location_id, sort_order);

CREATE TABLE IF NOT EXISTS products (
  id               TEXT PRIMARY KEY,
  location_id      TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  category_id      TEXT REFERENCES categories(id) ON DELETE SET NULL,
  name             TEXT NOT NULL,
  description      TEXT,
  -- Minor units. The trusted price: an order line NEVER takes its price from the
  -- client, the kernel re-derives it from here (place_order_priced).
  price            INTEGER NOT NULL CHECK (price >= 0),
  available        INTEGER NOT NULL DEFAULT 1 CHECK (available IN (0,1)),
  -- The stop-list reason, shown greyed-out rather than hidden: the old service
  -- deliberately kept sold-out dishes visible with a reason.
  unavailable_note TEXT,
  image_url        TEXT,
  primary_media_id TEXT,
  allergens        TEXT NOT NULL DEFAULT '[]',
  calories         INTEGER,
  prep_time_minutes INTEGER,
  attributes       TEXT NOT NULL DEFAULT '{}',
  sort_order       INTEGER NOT NULL DEFAULT 0,
  created_at_ms    INTEGER NOT NULL,
  updated_at_ms    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS products_location_idx ON products (location_id, category_id, sort_order);
CREATE INDEX IF NOT EXISTS products_available_idx ON products (location_id, available);

CREATE TABLE IF NOT EXISTS modifier_groups (
  id           TEXT PRIMARY KEY,
  product_id   TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,
  min_select   INTEGER NOT NULL DEFAULT 0,
  max_select   INTEGER NOT NULL DEFAULT 1,
  -- NULL means "infer from max_select" -- the contract's documented fallback.
  display_type TEXT CHECK (display_type IN ('radio','checkbox','select','quantity')),
  sort_order   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS modifier_groups_product_idx ON modifier_groups (product_id, sort_order);

CREATE TABLE IF NOT EXISTS modifiers (
  id         TEXT PRIMARY KEY,
  group_id   TEXT NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
  name       TEXT NOT NULL,
  -- Surcharge in minor units; matches PriceableLeaf.modifiers in the kernel catalog.
  price      INTEGER NOT NULL DEFAULT 0,
  available  INTEGER NOT NULL DEFAULT 1 CHECK (available IN (0,1)),
  sort_order INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS modifiers_group_idx ON modifiers (group_id, sort_order);

-- Translations live beside the row rather than in per-locale columns, because the
-- old service kept adding locales (sq/en/uk today, and the standalone PWA shipped
-- Arabic). `field` is 'name' or 'description'.
CREATE TABLE IF NOT EXISTS content_i18n (
  entity_type TEXT NOT NULL CHECK (entity_type IN ('product','category','modifier_group','modifier','location')),
  entity_id   TEXT NOT NULL,
  locale      TEXT NOT NULL,
  field       TEXT NOT NULL,
  value       TEXT NOT NULL,
  PRIMARY KEY (entity_type, entity_id, locale, field)
);
