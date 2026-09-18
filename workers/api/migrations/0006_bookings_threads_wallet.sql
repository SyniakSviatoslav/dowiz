-- Reservations, message threads and the wallet journal.
--
-- These three domains had screens in the product and nothing behind them. The
-- LAW for each lives in the kernel and not here:
--   * `dowiz_kernel::reservation`     — which transitions are legal at all
--   * `dowiz_kernel::thread`          — ordering, idempotency, read marks
--   * `dowiz_kernel::ledger_account`  — every transaction nets to exactly zero
--   * `dowiz_kernel::pass`            — minting and checking an entry pass
-- SQLite stores bytes. It decides nothing.
--
-- EVERY TABLE CARRIES location_id. `hub_image` did not, and new venues adopted
-- the first venue's data until that was found. A tenant boundary that is not in
-- the schema is a tenant boundary that leaks.
--
-- EVENTS, NOT STATE. A reservation's status is `fold(events)`, the same way an
-- order's is. `reservations.status` below is a CACHE of that fold, written in
-- the same statement as the event and never read as the authority — it exists
-- so a list query does not replay every booking in the venue.

-- ─────────────────────────────────────────────────────────────────────────────
-- Reservations
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS reservations (
  id            TEXT PRIMARY KEY,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  -- Who booked. NULL for a guest booking taken over the phone by the venue.
  user_id       TEXT REFERENCES users(id) ON DELETE SET NULL,
  party         INTEGER NOT NULL,
  -- Minutes since the Unix epoch. The same unit the kernel uses; storing
  -- milliseconds here and minutes there is how two clocks start disagreeing.
  slot_min      INTEGER NOT NULL,
  occasion      TEXT NOT NULL DEFAULT '',
  -- Contact for THIS booking. Kept beside the reservation rather than joined
  -- from `users`, because a table booked for somebody else is a real thing.
  contact_name  TEXT NOT NULL DEFAULT '',
  contact_phone TEXT NOT NULL DEFAULT '',
  -- The cached fold. Authority is `reservation_events`.
  status        TEXT NOT NULL DEFAULT 'REQUESTED',
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_reservations_loc_slot
  ON reservations (location_id, slot_min);
CREATE INDEX IF NOT EXISTS idx_reservations_user
  ON reservations (user_id, slot_min);

-- The authority. Append-only: a row is never updated and never deleted.
CREATE TABLE IF NOT EXISTS reservation_events (
  id             TEXT PRIMARY KEY,
  reservation_id TEXT NOT NULL REFERENCES reservations(id) ON DELETE CASCADE,
  location_id    TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  -- The status this event moved the booking INTO. Replaying these in `seq`
  -- order through `reservation::fold_transitions` reproduces `status` exactly.
  to_status      TEXT NOT NULL,
  -- Monotonic per reservation. Two events can never share one.
  seq            INTEGER NOT NULL,
  -- Who caused it: CUSTOMER | VENUE | SYSTEM. Recorded, never ranked.
  actor          TEXT NOT NULL,
  reason         TEXT NOT NULL DEFAULT '',
  at_ms          INTEGER NOT NULL,
  UNIQUE (reservation_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_reservation_events_res
  ON reservation_events (reservation_id, seq);

-- The venue's pass key. Symmetric ON PURPOSE: the venue's hub mints an entry
-- pass and that same venue's scanner checks it, so there is no third party who
-- needs to verify without being able to mint. See `dowiz_kernel::pass` for why a
-- post-quantum signature cannot be used here — 3309 bytes does not fit in a QR.
CREATE TABLE IF NOT EXISTS venue_pass_keys (
  location_id   TEXT PRIMARY KEY REFERENCES locations(id) ON DELETE CASCADE,
  -- 32 bytes, base64. A venue secret living in that venue's own database.
  key_b64       TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  rotated_at_ms INTEGER
);

-- ─────────────────────────────────────────────────────────────────────────────
-- Message threads
-- ─────────────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS threads (
  id            TEXT PRIMARY KEY,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  -- What the conversation is about. Exactly one is set.
  order_id      TEXT,
  reservation_id TEXT REFERENCES reservations(id) ON DELETE CASCADE,
  created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_threads_loc ON threads (location_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_threads_order ON threads (order_id);

CREATE TABLE IF NOT EXISTS thread_messages (
  -- The kernel's idempotency key. A resend under the same id is a no-op.
  id            TEXT PRIMARY KEY,
  thread_id     TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  -- CUSTOMER | VENUE | COURIER | SYSTEM
  from_party    TEXT NOT NULL,
  -- Per-sender and monotonic. This ORDERS the thread; `sent_at_ms` does not.
  seq           INTEGER NOT NULL,
  -- TEXT | READ
  kind          TEXT NOT NULL,
  body          TEXT NOT NULL DEFAULT '',
  -- For a READ mark: how far the sender has read.
  read_through  INTEGER,
  -- Display only. Never orders anything: two phones disagree about the time.
  sent_at_ms    INTEGER NOT NULL,
  UNIQUE (thread_id, from_party, seq)
);
CREATE INDEX IF NOT EXISTS idx_thread_messages_thread
  ON thread_messages (thread_id, seq);

-- ─────────────────────────────────────────────────────────────────────────────
-- The wallet journal
-- ─────────────────────────────────────────────────────────────────────────────

-- A transaction is a set of postings that sums to EXACTLY zero. The sum is
-- checked by `dowiz_kernel::ledger_account::validate` before a row is written,
-- because SQLite cannot express "these children must sum to zero" and a CHECK
-- that only looks at one row would be a guarantee in name only.
CREATE TABLE IF NOT EXISTS ledger_tx (
  id            TEXT PRIMARY KEY,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  -- TOP_UP | SPEND | REFUND | PAYOUT | FEE
  kind          TEXT NOT NULL,
  -- For a REFUND: the transaction it exactly negates. At most one per target,
  -- which the unique index below enforces in the schema as well as the kernel.
  reverses      TEXT REFERENCES ledger_tx(id),
  -- An order id, a reservation id — whatever caused the money to move.
  memo          TEXT NOT NULL DEFAULT '',
  at_ms         INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_ledger_tx_one_refund_per_target
  ON ledger_tx (reverses) WHERE reverses IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_ledger_tx_loc ON ledger_tx (location_id, at_ms);

CREATE TABLE IF NOT EXISTS ledger_postings (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  tx_id         TEXT NOT NULL REFERENCES ledger_tx(id) ON DELETE CASCADE,
  location_id   TEXT NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
  -- wallet:<user> | venue:<location> | platform | external
  account       TEXT NOT NULL,
  -- INTEGER MINOR UNITS. There is no REAL column anywhere in this file and
  -- there must never be one: `kernel/src/money.rs` is zero floats, ever.
  minor         INTEGER NOT NULL,
  currency      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ledger_postings_account
  ON ledger_postings (location_id, account);
CREATE INDEX IF NOT EXISTS idx_ledger_postings_tx
  ON ledger_postings (tx_id);
