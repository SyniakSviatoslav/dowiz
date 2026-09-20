-- The waiting list: a venue that left its address on dowiz.org.
--
-- The landing page has no price on it, so the only thing a visitor can do is
-- say "talk to me". That is this row. The address is the key: a second submit
-- from the same mailbox updates the note and the time rather than making a
-- second row, so the list an administrator reads has one line per venue.
--
-- No location_id: this is the PLATFORM's table, written before a venue exists.
CREATE TABLE IF NOT EXISTS waitlist (
  email         TEXT PRIMARY KEY,           -- lower-cased, trimmed
  venue         TEXT,                       -- what they typed as the venue's name, if anything
  lang          TEXT NOT NULL DEFAULT 'uk', -- the language the page was read in
  source        TEXT,                       -- the Host the form was posted from
  at_ms         INTEGER NOT NULL,           -- first submit
  updated_ms    INTEGER NOT NULL,           -- last submit
  notified_ms   INTEGER                     -- when the mail to the operator went out; NULL = not sent
);
CREATE INDEX IF NOT EXISTS waitlist_recent ON waitlist (updated_ms DESC);
