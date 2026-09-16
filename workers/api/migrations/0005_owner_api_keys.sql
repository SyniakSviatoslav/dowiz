-- Long-lived credentials for an owner's own tools and MCP clients.
--
-- SEPARATE FROM SESSIONS on purpose. A session is a browser that will come back
-- tomorrow; a key is a script that runs unattended for a year. Mixing them
-- means either sessions that never expire or keys that log somebody out, and
-- revoking one kind would take the other with it.
--
-- The SECRET IS HASHED. The row is enough to recognise a key and never enough
-- to use one, so a copied database does not hand over a venue's API.
CREATE TABLE IF NOT EXISTS owner_api_keys (
  id            TEXT PRIMARY KEY,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  owner_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  label         TEXT NOT NULL,
  key_hash      TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  expires_at_ms INTEGER NOT NULL,
  last_used_ms  INTEGER,
  revoked_at_ms INTEGER
);
CREATE INDEX IF NOT EXISTS idx_owner_api_keys_loc ON owner_api_keys (location_id);
