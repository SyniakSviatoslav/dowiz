//! SQLite store for orders + push subscriptions.
//!
//! Tables:
//!   * `orders`    — id PK, status, channel, subtotal, total, created_at_ms, payload(json)
//!   * `push_subs` — id PK, courier_id, endpoint, auth, p256dh, created_at_ms
//!
//! `rusqlite` is configured with the `bundled` feature so libsqlite3 is compiled
//! in — no system sqlite dependency, fully offline-buildable.
//!
//! `list_by_channel` does the funnel math with raw SQL (`GROUP BY channel`). The
//! kernel `ChannelLedger` is the canonical reducer; here we expose the same
//! `orders_by_channel` view directly from the persisted orders. (No courier
//! scoring/ranking anywhere.)

use dowiz_kernel::Order;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use crate::models::{StoredOrder, SubscribeRequest};

/// Thread-shared store. `Connection` is not `Sync`, so it lives behind a `Mutex`.
/// Single-writer SQLite is sufficient for this scope; the mutex serializes access.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// Open (or create) the SQLite file at `db_path`, making its parent dir if
    /// needed, then run migrations.
    pub fn open(db_path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Null,
                        Box::new(e),
                    )
                })?;
            }
        }
        let conn = Connection::open(db_path)?;
        let store = Store {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Open an in-memory database (used by tests).
    pub fn open_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Store {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS orders (
                id           TEXT PRIMARY KEY,
                status       TEXT NOT NULL,
                channel      TEXT,
                subtotal     INTEGER NOT NULL,
                total        INTEGER NOT NULL,
                created_at_ms INTEGER NOT NULL,
                payload      TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS push_subs (
                id            TEXT PRIMARY KEY,
                courier_id    TEXT NOT NULL,
                endpoint      TEXT NOT NULL,
                auth          TEXT NOT NULL,
                p256dh       TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS venues (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                claimed       INTEGER NOT NULL DEFAULT 0,
                owner_id      TEXT,
                created_at_ms INTEGER NOT NULL
            );
            "#,
        )
    }

    /// Persist a freshly-placed order.
    pub fn insert_order(&self, order: &Order) -> rusqlite::Result<()> {
        let stored = StoredOrder::from(order);
        let payload = serde_json::to_string(&stored).expect("StoredOrder is always serializable");
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO orders (id, status, channel, subtotal, total, created_at_ms, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                order.id,
                order.status.as_str(),
                order.channel,
                order.subtotal,
                order.total,
                order.created_at_ms,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Fetch an order by id, rehydrating the kernel `Order` from the stored payload.
    /// The authoritative `status` column overrides the snapshot inside `payload`
    /// (so `update_status` is reflected on read).
    pub fn get_order(&self, id: &str) -> rusqlite::Result<Option<Order>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT status, payload FROM orders WHERE id = ?1")?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let status: String = row.get(0)?;
            let payload: String = row.get(1)?;
            let mut stored: StoredOrder = serde_json::from_str(&payload).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            // Status is the live column; payload is a creation snapshot.
            stored.status = status;
            Ok(Some(Order::from(&stored)))
        } else {
            Ok(None)
        }
    }

    /// Update an order's status. Returns `true` if a row was updated.
    pub fn update_status(&self, id: &str, status: &str) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE orders SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(n > 0)
    }

    /// Distinct order count per channel (channel-attribution funnel math),
    /// descending by count then channel name. Mirrors
    /// `ChannelLedger::orders_by_channel`.
    pub fn list_by_channel(&self) -> rusqlite::Result<Vec<(String, u64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COALESCE(channel, '') AS channel, COUNT(*) AS cnt
             FROM orders
             GROUP BY channel
             ORDER BY cnt DESC, channel ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Insert a web-push subscription. `id`/`created_at_ms` are supplied by the
    /// caller (generated at the HTTP boundary).
    pub fn insert_push_sub(
        &self,
        id: &str,
        created_at_ms: i64,
        sub: &SubscribeRequest,
    ) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO push_subs (id, courier_id, endpoint, auth, p256dh, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                sub.courier_id,
                sub.endpoint,
                sub.auth,
                sub.p256dh,
                created_at_ms,
            ],
        )?;
        Ok(())
    }

    /// Count of stored push subscriptions (used by tests / diagnostics).
    pub fn push_sub_count(&self) -> rusqlite::Result<u64> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM push_subs", [], |row| row.get(0))?;
        Ok(n as u64)
    }

    /// Insert a venue (unclaimed by default).
    pub fn insert_venue(&self, id: &str, name: &str, created_at_ms: i64) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO venues (id, name, claimed, owner_id, created_at_ms)
             VALUES (?1, ?2, 0, NULL, ?3)",
            params![id, name, created_at_ms],
        )?;
        Ok(())
    }

    /// Claim a venue (idempotent). Returns true if a row was updated/inserted.
    /// `name` is used when creating a new (absent) venue; an existing venue's
    /// name is preserved on re-claim.
    pub fn claim_venue(
        &self,
        id: &str,
        owner_id: &str,
        name: &str,
        created_at_ms: i64,
    ) -> rusqlite::Result<bool> {
        let conn = self.conn.lock().unwrap();
        // Upsert: claim existing or create-claimed if absent.
        conn.execute(
            "INSERT INTO venues (id, name, claimed, owner_id, created_at_ms)
             VALUES (?1, ?3, 1, ?2, ?4)
             ON CONFLICT(id) DO UPDATE SET claimed = 1, owner_id = ?2",
            params![id, owner_id, name, created_at_ms],
        )?;
        Ok(true)
    }

    /// Fetch a venue by id.
    pub fn get_venue(&self, id: &str) -> rusqlite::Result<Option<(String, bool, Option<String>)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, claimed, owner_id FROM venues WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let claimed: i64 = row.get(1)?;
            let owner: Option<String> = row.get(2)?;
            Ok(Some((name, claimed != 0, owner)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::place_order;

    fn sample_order() -> Order {
        place_order(
            "o1".into(),
            Some("c1".into()),
            vec![dowiz_kernel::OrderItem {
                product_id: "p1".into(),
                modifier_ids: vec!["m1".into()],
                quantity: 2,
                unit_price: 500,
            }],
            1_700_000_000_000,
            Some("web".into()),
            Some("5000".into()),
        )
        .unwrap()
    }

    #[test]
    fn green_insert_then_get() {
        let store = Store::open_memory().unwrap();
        let o = sample_order();
        store.insert_order(&o).unwrap();
        let got = store.get_order("o1").unwrap().expect("order exists");
        assert_eq!(got.id, "o1");
        assert_eq!(got.status, dowiz_kernel::OrderStatus::Pending);
        assert_eq!(got.channel.as_deref(), Some("web"));
        assert_eq!(got.subtotal, 1000);
    }

    #[test]
    fn green_missing_order_is_none() {
        let store = Store::open_memory().unwrap();
        assert!(store.get_order("nope").unwrap().is_none());
    }

    #[test]
    fn green_update_status() {
        let store = Store::open_memory().unwrap();
        store.insert_order(&sample_order()).unwrap();
        assert!(store.update_status("o1", "CONFIRMED").unwrap());
        let got = store.get_order("o1").unwrap().unwrap();
        assert_eq!(got.status, dowiz_kernel::OrderStatus::Confirmed);
    }

    // ── GREEN: channel count funnel math ──
    #[test]
    fn green_list_by_channel() {
        let store = Store::open_memory().unwrap();
        let o1 = sample_order();
        let mut o2 = sample_order();
        o2.id = "o2".into();
        o2.channel = Some("tiktok".into());
        let mut o3 = sample_order();
        o3.id = "o3".into();
        o3.channel = Some("tiktok".into());
        let mut o4 = sample_order();
        o4.id = "o4".into();
        o4.channel = None;
        store.insert_order(&o1).unwrap();
        store.insert_order(&o2).unwrap();
        store.insert_order(&o3).unwrap();
        store.insert_order(&o4).unwrap();
        let by = store.list_by_channel().unwrap();
        let map: std::collections::HashMap<&str, u64> =
            by.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        assert_eq!(map["tiktok"], 2);
        assert_eq!(map["web"], 1);
        assert_eq!(map[""], 1);
    }

    #[test]
    fn green_push_sub_persists() {
        let store = Store::open_memory().unwrap();
        let sub = SubscribeRequest {
            courier_id: "courier-1".into(),
            endpoint: "https://push.example/abc".into(),
            auth: "auth-secret".into(),
            p256dh: "p256dh-key".into(),
        };
        store.insert_push_sub("sub-1", 123, &sub).unwrap();
        assert_eq!(store.push_sub_count().unwrap(), 1);
    }
}
