//! S6's OWN data-access trait — the two live-authz reads ADR-0013/`#4` need that the S2 `AuthRepo`
//! does not carry (S2 owns identity/session; S6 owns "may this principal touch this room RIGHT
//! NOW", proposal §1 note 3: "S6 *verifies*... S5 *publishes*"). Ports:
//!   - `courier-room-authz.ts`'s `courierBindingVerdict` (the tenant-GUC `courier_assignments` read)
//!   - `websocket.ts`'s `ownerRoomVerdict` (the `memberships` / order→location JOIN read)
//!
//! Both are TRI-STATE (`Verdict`) — never throw. A clean 0-row result is `Deny` (fail closed); a
//! connect/query failure is `Unavailable` (transient — the caller's relay guard withholds without
//! evicting a legitimate member on a pool blip, courier-room-authz.ts module doc).
//!
//! The courier query reuses `crate::db::with_tenant` (the S1-provided `app.current_tenant` GUC
//! combinator) — S6 is its FIRST real caller outside `db.rs`'s own tests, exactly as that module's
//! doc anticipated. This makes the read sound under BOTH BYPASSRLS (today) and NOBYPASSRLS
//! (post-B3), order-independent of that flip (courier-room-authz.ts:9-13).

use sqlx::PgPool;
use uuid::Uuid;

use domain::TenantId;

use crate::db::{TenantTxnError, with_tenant};

/// The tri-state verdict every S6 authz read returns (ADR-0013 `AuthzVerdict` parity).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny,
    /// A DB blip — retryable, never a real negative (`courier-room-authz.ts` module doc).
    Unavailable,
}

/// Read-access binding statuses (`BINDING_READ_STATUSES`, `courier-room-authz.ts:27`) — the
/// offer-handshake courier must be able to view the order to decide accept/decline.
pub const BINDING_READ_STATUSES: [&str; 4] = ["offered", "assigned", "accepted", "picked_up"];

#[async_trait::async_trait]
pub trait WsAuthzRepo: Send + Sync {
    /// `ownerRoomVerdict`'s `location:*` branch (`websocket.ts:39-43`): live active owner
    /// membership at `location_id`.
    async fn owner_location_verdict(&self, user_id: Uuid, location_id: Uuid) -> Verdict;

    /// `ownerRoomVerdict`'s `order:*` branch (`websocket.ts:44-52`, ADR-0004): order→location JOIN
    /// live active owner membership — no baked-claim trust, and correctly admits a multi-location
    /// owner reading an order at ANY of their locations.
    async fn owner_order_verdict(&self, user_id: Uuid, order_id: Uuid) -> Verdict;

    /// `courierReadVerdict`/`courierBindingVerdict` (`courier-room-authz.ts:32-71`): a live
    /// `courier_assignments` row for this courier/order inside the tenant-GUC tx.
    async fn courier_binding_verdict(
        &self,
        courier_sub: Uuid,
        active_location_id: Uuid,
        order_id: Uuid,
    ) -> Verdict;
}

pub struct PgWsAuthzRepo {
    pool: PgPool,
}

impl PgWsAuthzRepo {
    pub fn new(pool: PgPool) -> Self {
        PgWsAuthzRepo { pool }
    }
}

#[async_trait::async_trait]
impl WsAuthzRepo for PgWsAuthzRepo {
    async fn owner_location_verdict(&self, user_id: Uuid, location_id: Uuid) -> Verdict {
        let row: Result<Option<(i32,)>, sqlx::Error> = sqlx::query_as(
            "SELECT 1 FROM memberships
              WHERE user_id = $1 AND location_id = $2 AND role = 'owner' AND status = 'active'
              LIMIT 1",
        )
        .bind(user_id)
        .bind(location_id)
        .fetch_optional(&self.pool)
        .await;
        match row {
            Ok(Some(_)) => Verdict::Allow,
            Ok(None) => Verdict::Deny,
            Err(err) => {
                tracing::error!(%err, "ws owner_location_verdict query failed");
                Verdict::Unavailable
            }
        }
    }

    async fn owner_order_verdict(&self, user_id: Uuid, order_id: Uuid) -> Verdict {
        let row: Result<Option<(i32,)>, sqlx::Error> = sqlx::query_as(
            "SELECT 1 FROM orders o
               JOIN memberships m ON m.location_id = o.location_id
              WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' AND m.status = 'active'
              LIMIT 1",
        )
        .bind(order_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await;
        match row {
            Ok(Some(_)) => Verdict::Allow,
            Ok(None) => Verdict::Deny,
            Err(err) => {
                tracing::error!(%err, "ws owner_order_verdict query failed");
                Verdict::Unavailable
            }
        }
    }

    async fn courier_binding_verdict(
        &self,
        courier_sub: Uuid,
        active_location_id: Uuid,
        order_id: Uuid,
    ) -> Verdict {
        let tenant = TenantId::from(active_location_id);
        let outcome: Result<Option<(i32,)>, TenantTxnError> =
            with_tenant(&self.pool, tenant, |txn| {
                Box::pin(async move {
                    sqlx::query_as(
                        "SELECT 1 FROM courier_assignments
                          WHERE order_id = $1 AND courier_id = $2 AND status = ANY($3::text[])
                          LIMIT 1",
                    )
                    .bind(order_id)
                    .bind(courier_sub)
                    .bind(BINDING_READ_STATUSES.to_vec())
                    .fetch_optional(&mut **txn)
                    .await
                })
            })
            .await;
        match outcome {
            Ok(Some(_)) => Verdict::Allow,
            Ok(None) => Verdict::Deny,
            Err(err) => {
                tracing::error!(%err, "ws courier_binding_verdict tx failed");
                Verdict::Unavailable
            }
        }
    }
}

#[cfg(test)]
pub mod fake {
    use super::{Uuid, Verdict, WsAuthzRepo};
    use std::sync::Mutex;

    /// A canned-verdict fake, mirroring `FakeAuthRepo`'s pattern: tests preset the verdict a
    /// method should return (default `Allow` so a test only overrides the branch it exercises).
    pub struct FakeWsAuthzRepo {
        pub owner_location: Mutex<Verdict>,
        pub owner_order: Mutex<Verdict>,
        pub courier_binding: Mutex<Verdict>,
        /// Records the last `(sub, active_location_id, order_id)` the courier check was called
        /// with — proves the fan-out re-authz threads the LIVE member context, not a stale one.
        pub last_courier_call: Mutex<Option<(Uuid, Uuid, Uuid)>>,
    }

    impl Default for FakeWsAuthzRepo {
        fn default() -> Self {
            FakeWsAuthzRepo {
                owner_location: Mutex::new(Verdict::Allow),
                owner_order: Mutex::new(Verdict::Allow),
                courier_binding: Mutex::new(Verdict::Allow),
                last_courier_call: Mutex::new(None),
            }
        }
    }

    #[async_trait::async_trait]
    impl WsAuthzRepo for FakeWsAuthzRepo {
        async fn owner_location_verdict(&self, _user_id: Uuid, _location_id: Uuid) -> Verdict {
            *self.owner_location.lock().unwrap()
        }
        async fn owner_order_verdict(&self, _user_id: Uuid, _order_id: Uuid) -> Verdict {
            *self.owner_order.lock().unwrap()
        }
        async fn courier_binding_verdict(
            &self,
            courier_sub: Uuid,
            active_location_id: Uuid,
            order_id: Uuid,
        ) -> Verdict {
            *self.last_courier_call.lock().unwrap() =
                Some((courier_sub, active_location_id, order_id));
            *self.courier_binding.lock().unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeWsAuthzRepo;
    use super::*;

    #[tokio::test]
    async fn fake_repo_defaults_to_allow_and_records_the_courier_call_context() {
        let repo = FakeWsAuthzRepo::default();
        let sub = Uuid::new_v4();
        let loc = Uuid::new_v4();
        let order = Uuid::new_v4();
        assert_eq!(
            repo.courier_binding_verdict(sub, loc, order).await,
            Verdict::Allow
        );
        assert_eq!(
            *repo.last_courier_call.lock().unwrap(),
            Some((sub, loc, order))
        );
    }

    #[tokio::test]
    async fn fake_repo_can_be_preset_to_deny_or_unavailable() {
        let repo = FakeWsAuthzRepo::default();
        *repo.owner_location.lock().unwrap() = Verdict::Deny;
        assert_eq!(
            repo.owner_location_verdict(Uuid::new_v4(), Uuid::new_v4())
                .await,
            Verdict::Deny
        );
        *repo.owner_order.lock().unwrap() = Verdict::Unavailable;
        assert_eq!(
            repo.owner_order_verdict(Uuid::new_v4(), Uuid::new_v4())
                .await,
            Verdict::Unavailable
        );
    }
}
