-- An invite is addressed to a PHONE, not an email.
--
-- The table was written when a courier signed in with an address; they sign in
-- with a number, and `couriers` already has `phone_hash` as the unique key the
-- login path uses. Storing a phone in `invited_email_hash` would make the
-- column a lie for every row written from here on, and the first person to read
-- the schema would trust the name.
ALTER TABLE courier_invites ADD COLUMN invited_phone_hash TEXT;
ALTER TABLE courier_invites ADD COLUMN invited_name TEXT;
CREATE INDEX IF NOT EXISTS idx_courier_invites_phone
  ON courier_invites (invited_phone_hash) WHERE invited_phone_hash IS NOT NULL;
