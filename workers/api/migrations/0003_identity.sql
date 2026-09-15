-- Accounts, sessions and tenancy. Ported from the old platform's auth
-- (origin/backup-wip-2026-07-08), keeping what was deliberate and dropping what
-- was accidental.
--
-- KEPT, because the old code earned these the hard way:
--   * Owner authority is NEVER trusted from a token. It is re-derived from
--     `memberships` on every request, so a removed owner is denied on their very
--     next call even with an unexpired JWT.
--   * A courier's session is a ROW, not just a JWT. Revoking it, or removing the
--     courier from a location, kills access immediately rather than at token exp.
--   * Courier PII is encrypted at rest with a separate sha256 lookup hash, so the
--     table can be queried without holding the plaintext.
--   * Refresh tokens rotate within a FAMILY; replaying a spent one revokes the
--     whole family.
--
-- DROPPED, deliberately:
--   * Postgres RLS and the two rival tenant mechanisms (`app.user_id` +
--     app_member_location_ids() for some tables, `app.current_tenant` for
--     others). D1 has no session GUCs and no RLS. Two non-unified scoping schemes
--     in one codebase is a hazard, not a feature; tenancy is now enforced in one
--     place -- the guard -- and every query is explicitly scoped.
--   * `membership_role = 'admin'`. No code path ever created one.
--
-- Platform-admin authority stays OUT of every token by design: it is a row here,
-- point-read per request, never a forgeable claim.

CREATE TABLE IF NOT EXISTS users (
  id               TEXT PRIMARY KEY,
  email            TEXT UNIQUE,            -- lowercased on write (no citext in SQLite)
  google_sub       TEXT UNIQUE,
  telegram_user_id TEXT UNIQUE,
  display_name     TEXT,
  phone            TEXT,
  password_hash    TEXT,                   -- NULL => this account logs in another way
  created_at_ms    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS organizations (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  -- Nullable on purpose: a "shadow" org exists before anyone claims it.
  owner_id      TEXT REFERENCES users(id) ON DELETE SET NULL,
  created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS memberships (
  id            TEXT PRIMARY KEY,
  user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  role          TEXT NOT NULL CHECK (role IN ('owner','courier')),
  status        TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','suspended','removed')),
  created_at_ms INTEGER NOT NULL,
  UNIQUE (user_id, location_id, role)
);
CREATE INDEX IF NOT EXISTS memberships_lookup_idx ON memberships (location_id, user_id, role, status);

-- Never a JWT claim. A point-read on every /admin request, failing CLOSED.
CREATE TABLE IF NOT EXISTS platform_admins (
  user_id       TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  created_at_ms INTEGER NOT NULL
);

-- Owner refresh family. Only the sha256 of the token is stored.
CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
  id            TEXT PRIMARY KEY,
  user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  family_id     TEXT NOT NULL,
  token_hash    TEXT NOT NULL,
  used          INTEGER NOT NULL DEFAULT 0 CHECK (used IN (0,1)),
  expires_at_ms INTEGER NOT NULL,
  created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS auth_refresh_token_hash_idx ON auth_refresh_tokens (token_hash);
CREATE INDEX IF NOT EXISTS auth_refresh_family_idx     ON auth_refresh_tokens (family_id, created_at_ms);

CREATE TABLE IF NOT EXISTS couriers (
  id                  TEXT PRIMARY KEY,
  -- PII encrypted; the *_hash columns are what queries match on.
  email_encrypted     TEXT NOT NULL,
  email_hash          TEXT NOT NULL UNIQUE,
  phone_encrypted     TEXT,
  phone_hash          TEXT UNIQUE,
  full_name_encrypted TEXT NOT NULL,
  password_hash       TEXT NOT NULL,
  status              TEXT NOT NULL DEFAULT 'active'
                      CHECK (status IN ('active','deactivated','suspended')),
  messenger_kind      TEXT,
  messenger_handle    TEXT,
  created_at_ms       INTEGER NOT NULL,
  last_login_at_ms    INTEGER,
  deactivated_at_ms   INTEGER
);
-- The old schema reused ONE hash column space for both email and phone lookups.
-- Here they are separate columns, so a phone can never match an email row.

CREATE TABLE IF NOT EXISTS courier_locations (
  courier_id  TEXT NOT NULL REFERENCES couriers(id) ON DELETE CASCADE,
  location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  role        TEXT NOT NULL DEFAULT 'courier' CHECK (role IN ('courier','dispatcher')),
  added_at_ms INTEGER NOT NULL,
  PRIMARY KEY (courier_id, location_id)
);

-- The courier's real credential. The JWT's `jti` points here and the guard
-- re-reads this row on EVERY request.
CREATE TABLE IF NOT EXISTS courier_sessions (
  id                 TEXT PRIMARY KEY,
  courier_id         TEXT NOT NULL REFERENCES couriers(id) ON DELETE CASCADE,
  family_id          TEXT NOT NULL,
  token_hash         TEXT NOT NULL,
  active_location_id TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  issued_at_ms       INTEGER NOT NULL,
  expires_at_ms      INTEGER NOT NULL,
  revoked_at_ms      INTEGER,
  replaced_by        TEXT REFERENCES courier_sessions(id) ON DELETE SET NULL,
  last_used_at_ms    INTEGER,
  user_agent_hash    TEXT,
  ip_hash            TEXT
);
CREATE INDEX IF NOT EXISTS courier_sessions_courier_idx ON courier_sessions (courier_id, revoked_at_ms);
CREATE INDEX IF NOT EXISTS courier_sessions_family_idx  ON courier_sessions (family_id);

CREATE TABLE IF NOT EXISTS courier_invites (
  id                  TEXT PRIMARY KEY,
  location_id         TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  created_by_owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  -- Hard allow-list, at the schema level too: an invite must never mint an owner.
  role                TEXT NOT NULL DEFAULT 'courier' CHECK (role IN ('courier','dispatcher')),
  invited_email_hash  TEXT NOT NULL,
  code_hash           TEXT NOT NULL,
  expires_at_ms       INTEGER NOT NULL,
  used_at_ms          INTEGER,
  used_by_courier_id  TEXT REFERENCES couriers(id) ON DELETE SET NULL,
  revoked_at_ms       INTEGER,
  created_at_ms       INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS courier_invites_location_idx ON courier_invites (location_id, expires_at_ms);

CREATE TABLE IF NOT EXISTS courier_audit_log (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  courier_id      TEXT REFERENCES couriers(id) ON DELETE SET NULL,
  location_id     TEXT REFERENCES locations(id) ON DELETE SET NULL,
  action          TEXT NOT NULL,
  actor_kind      TEXT CHECK (actor_kind IN ('owner','courier','system')),
  actor_id        TEXT,
  metadata        TEXT NOT NULL DEFAULT '{}',
  ip_hash         TEXT,
  user_agent_hash TEXT,
  created_at_ms   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS courier_audit_courier_idx ON courier_audit_log (courier_id, created_at_ms DESC);

-- The customer is not a login. A row appears when an order does.
CREATE TABLE IF NOT EXISTS customers (
  id            TEXT PRIMARY KEY,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  phone_hash    TEXT NOT NULL,
  phone_encrypted TEXT,
  name          TEXT,
  created_at_ms INTEGER NOT NULL,
  UNIQUE (location_id, phone_hash)
);
