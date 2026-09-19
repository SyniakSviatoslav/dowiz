-- Messages that arrive from, and go out to, a customer on a chat channel.
--
-- WhatsApp (Meta Cloud API) and Instagram (Messenger platform) deliver a
-- customer's message to one webhook; the owner reads and answers it in the
-- console. The row is the whole record: the platform keeps nothing else, and
-- a reply is a row too, so a thread is one query in either direction.
--
-- EVERY ROW CARRIES location_id. The webhook resolves the venue from the Host
-- it was called on, so two venues' inboxes never share a row.
CREATE TABLE IF NOT EXISTS channel_messages (
  id            TEXT PRIMARY KEY,           -- "<channel>:<external id>", so a redelivery is ignored
  location_id   TEXT NOT NULL,
  channel       TEXT NOT NULL CHECK (channel IN ('whatsapp','instagram')),
  direction     TEXT NOT NULL CHECK (direction IN ('in','out')),
  peer          TEXT NOT NULL,              -- the customer's WhatsApp number or Instagram-scoped id
  peer_name     TEXT,                       -- the profile name the platform sent, when it did
  text          TEXT NOT NULL,
  external_id   TEXT,                       -- the platform's own message id
  at_ms         INTEGER NOT NULL,
  read_ms       INTEGER                     -- when the owner opened the thread; NULL = unread
);
CREATE INDEX IF NOT EXISTS channel_messages_thread
  ON channel_messages (location_id, channel, peer, at_ms);
CREATE INDEX IF NOT EXISTS channel_messages_recent
  ON channel_messages (location_id, at_ms DESC);
