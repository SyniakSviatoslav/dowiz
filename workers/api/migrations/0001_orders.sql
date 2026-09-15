-- The order log. `order_json` is the kernel's own serialization and is the
-- SINGLE source of truth; `status` and the timestamps are projections kept
-- alongside it so the owner surface can query without parsing every row.
-- Nothing here re-derives money: totals live inside order_json, recomputed by
-- the kernel from items on every transition.
CREATE TABLE IF NOT EXISTS orders (
  id            TEXT    PRIMARY KEY,
  status        TEXT    NOT NULL,
  order_json    TEXT    NOT NULL,
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

-- The two queries the owner surface actually runs: "today's orders, newest
-- first" and "everything still awaiting me".
CREATE INDEX IF NOT EXISTS orders_created_at_idx ON orders (created_at_ms DESC);
CREATE INDEX IF NOT EXISTS orders_status_idx     ON orders (status, created_at_ms DESC);
