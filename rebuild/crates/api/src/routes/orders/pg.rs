//! S5 `PgOrdersRepo` — the real `sqlx` implementation of [`super::OrdersRepo`]. Ports the SQL of
//! `apps/api/src/routes/orders.ts`, `customer/orders.ts`, and `lib/orderStatusService.ts`, wiring
//! the pure decisions from the sibling submodules onto the DB.
//!
//! ## REV-S5-1 — the create tx is GUC-LESS BY DESIGN
//! Unlike every other Rust writer so far (S3 `with_user`, the courier/service `with_tenant`), the
//! order-create tx seats **NO** GUC. It is a bare `pool.begin()`. This is not an oversight — it is
//! the ONLY correct root: the admitting RLS policies for every table the create tx writes
//! (`orders`/`customers`/`order_items`/`order_item_modifiers`/`idempotency_keys`/`velocity_events`)
//! are `anonymous_insert … WITH CHECK (app_current_user() IS NULL)` (`mig 1780315000000`,
//! `mig 1790000000077` RC1; `app_current_user()` = `app.user_id`). Anonymous checkout passes IFF
//! `app.user_id` is UNSET — seating ANY `app.user_id`/tenant GUC here would make the WITH CHECK
//! FALSE and 0-row the whole checkout post-B3. The discriminating NOBYPASSRLS probe
//! ([`tests::anonymous_create_admitted_only_with_no_app_user_id`]) proves BOTH directions.
//!
//! ## No live DB in this sandbox
//! Every DB-touching test here is `#[ignore]` (needs `DATABASE_URL_OPERATIONAL`/`_SESSION`), the same
//! posture as `db.rs`/`owner::mod`. The pure decisions these methods wire are unit-tested in the
//! sibling submodules; these probes pin the ACTUAL SQL/RLS behavior against a real Postgres when run
//! with `--ignored`.

use uuid::Uuid;

use domain::{ErrorCode, Lek, OrderStatus};

use super::pricing::{
    self, DeliveryTier, FeeLocation, GroupInfo, ModifierInfo, PricingItem, PricingSnapshot,
    ProductInfo,
};
use super::request_hash::{CanonicalItemInput, CanonicalRequestInput, build_request_hash};
use super::state::{self, BindingState, ExistingKey, IdempotencyDecision};
use super::{
    CreateOrderCommand, CreateOutcome, CustomerCancelOutcome, OrderCreatedResponse,
    OrderReadOutcome, OrderView, OrdersRepo, StatusUpdateOutcome,
};
use crate::repo::RepoError;

/// The customer post-dispatch cancel window (`customer/orders.ts` `cancelWindowMs`) — 5 minutes.
const CANCEL_WINDOW_MS: i64 = 5 * 60 * 1000;

/// The crown-jewel order INSERT (step 10). Every money value this fn COMPUTES persists here:
/// `subtotal`/`delivery_fee`/`tax_total` (gross VAT, `orders.ts:573`)/`total`, plus the NOT-NULL
/// `request_hash` (omitting it 500'd every create — staging oracle 2026-07-05) and the metadata
/// channel. `discount_total` stays the table's 0 default (REV-S5-6 CARRY); `currency_code` its
/// 'ALL' default; item/modifier/secondary writes are the documented deferred scope. Pinned by
/// [`tests::create_order_sql_persists_every_computed_money_column`].
// `$3::order_type` — `type` is an enum column and `$3` binds as text; Postgres does NOT
// implicitly coerce a bound text param to an enum (only an unknown-type SQL literal like
// 'pickup' auto-coerces, which is why a psql literal test passed while the sqlx bind 500'd
// with `column "type" is of type order_type but expression is of type text` — staging oracle
// 2026-07-05). `status`/`payment_method` are SQL literals here so they need no bound cast.
const CREATE_ORDER_SQL: &str = "INSERT INTO orders
   (location_id, customer_id, type, status, delivery_address, delivery_lat, delivery_lng,
    subtotal, delivery_fee, tax_total, total, payment_method, cash_pay_with, request_hash,
    delivery_instructions, metadata)
 VALUES ($1,$2,$3::order_type,'PENDING',$4,$5,$6,$7::integer,$8::integer,$9::integer,$10::integer,'cash',$11::integer,$12,$13,$14)
 RETURNING id, location_id, status::text, subtotal::bigint, total::bigint,
           delivery_instructions, created_at::text";

pub struct PgOrdersRepo {
    pool: sqlx::PgPool,
}

impl PgOrdersRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        PgOrdersRepo { pool }
    }
}

/// A location's create-time config row (`orders.ts:126-133`).
#[allow(clippy::type_complexity)]
type LocationRow = (
    Option<f64>,    // lat
    Option<f64>,    // lng
    Option<String>, // published_at (text)
    Option<f64>,    // tax_rate
    bool,           // price_includes_tax
    Option<i64>,    // min_order_value
    Option<i64>,    // free_delivery_threshold
    Option<i64>,    // delivery_fee_flat
    i32,            // menu_version
    bool,           // delivery_paused
);

#[async_trait::async_trait]
impl OrdersRepo for PgOrdersRepo {
    async fn create_order(&self, cmd: CreateOrderCommand) -> Result<CreateOutcome, RepoError> {
        let input = &cmd.input;
        let location_id = input.location_id;
        let idempotency_key = input.idempotency_key.to_string();
        let is_pickup = input.order_type == super::dto::OrderType::Pickup;

        // ── GUC-LESS BY DESIGN: anonymous_insert admits on app.user_id IS NULL — seating any GUC
        //    would break it (S5 REV-S5-1). Bare begin, no set_config. ──
        let mut txn = self.pool.begin().await?;

        // Q-STMT-TIMEOUT: bound the write-hold (orders.ts:124) — the pool-wedge fuse.
        sqlx::query("SET LOCAL statement_timeout = 4500")
            .execute(&mut *txn)
            .await?;

        // 1. Location gate (orders.ts:126) — 404 if absent; NOT_PUBLISHED / delivery_paused gates.
        // `tax_rate::float8` — tax_rate is NUMERIC; sqlx rejects numeric→f64 at decode, so the
        // uncast read failed EVERY order create at its first query (staging oracle 2026-07-05;
        // same class as the ::bigint casts already on this SELECT's int4 money columns).
        let loc: Option<LocationRow> = sqlx::query_as(
            "SELECT lat, lng, published_at::text, tax_rate::float8, price_includes_tax,
                    min_order_value::bigint, free_delivery_threshold::bigint,
                    delivery_fee_flat::bigint, menu_version, delivery_paused
               FROM locations WHERE id = $1",
        )
        .bind(location_id)
        .fetch_optional(&mut *txn)
        .await?;

        let Some((
            lat,
            lng,
            published_at,
            tax_rate,
            price_includes_tax,
            min_order_value,
            free_delivery_threshold,
            delivery_fee_flat,
            menu_version,
            _delivery_paused,
        )) = loc
        else {
            txn.rollback().await.ok();
            return Ok(CreateOutcome::Rejected(
                ErrorCode::NotFound,
                "Location not found".to_string(),
            ));
        };
        if published_at.is_none() {
            txn.rollback().await.ok();
            return Ok(CreateOutcome::Rejected(
                ErrorCode::NotPublished,
                "NOT_PUBLISHED".to_string(),
            ));
        }

        // 2. Build the request hash (REV-S5-2) — customer_id = the token sub, else "anonymous".
        let customer_id = cmd
            .customer_sub
            .map_or_else(|| "anonymous".to_string(), |s| s.to_string());
        let pin = input.delivery.as_ref().map(|d| (d.pin.lat, d.pin.lng));
        let request_hash = build_request_hash(&CanonicalRequestInput {
            location_id: location_id.to_string(),
            order_type: if is_pickup { "pickup" } else { "delivery" }.to_string(),
            items: input
                .items
                .iter()
                .map(|i| CanonicalItemInput {
                    product_id: i.product_id.to_string(),
                    quantity: i.quantity,
                    modifier_ids: i.modifier_ids.iter().map(Uuid::to_string).collect(),
                })
                .collect(),
            pin,
            address_text: input.delivery.as_ref().and_then(|d| d.address_text.clone()),
            cash_pay_with: input.cash_pay_with,
            currency_code: "ALL".to_string(),
            menu_version: menu_version.to_string(),
            customer_id,
        });

        // 3. Idempotency (orders.ts:394-412) — REV-S5-5 delete-and-recreate arm.
        let existing: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT order_id, request_hash FROM idempotency_keys WHERE key = $1 AND location_id = $2",
        )
        .bind(&idempotency_key)
        .bind(location_id)
        .fetch_optional(&mut *txn)
        .await?;

        // Resolve order-presence for the decision (only when a key exists with a matching hash).
        let existing_key = match &existing {
            Some((order_id, hash)) => {
                let present: Option<(Uuid,)> =
                    sqlx::query_as("SELECT id FROM orders WHERE id = $1")
                        .bind(order_id)
                        .fetch_optional(&mut *txn)
                        .await?;
                Some(ExistingKey {
                    request_hash: hash.clone(),
                    order_present: present.is_some(),
                })
            }
            None => None,
        };

        match state::idempotency_decision(existing_key.as_ref(), &request_hash) {
            IdempotencyDecision::Reuse422 => {
                txn.rollback().await.ok();
                return Ok(CreateOutcome::Rejected(
                    ErrorCode::IdempotencyKeyReused,
                    "Idempotency key reused with different request".to_string(),
                ));
            }
            IdempotencyDecision::Replay => {
                // Replay the committed order (200).
                let order_id = existing.as_ref().map(|(id, _)| *id).unwrap_or_default();
                // Same column order as CreatedRow / CREATE_ORDER_SQL RETURNING (replay must
                // return the byte-identical create response — REV-S5-5).
                let row: Option<CreatedRow> = sqlx::query_as(
                    "SELECT id, location_id, status::text, subtotal::bigint, total::bigint,
                            delivery_instructions, created_at::text FROM orders WHERE id = $1",
                )
                .bind(order_id)
                .fetch_optional(&mut *txn)
                .await?;
                txn.commit().await.map_err(RepoError)?;
                return Ok(match row {
                    Some(r) => CreateOutcome::Replayed(order_row(r)?),
                    None => CreateOutcome::Transient, // vanished between checks — retryable
                });
            }
            IdempotencyDecision::DeleteAndRecreate => {
                // REV-S5-5: key hit, hash matches, order GONE → DELETE then fall through to persist.
                sqlx::query("DELETE FROM idempotency_keys WHERE key = $1 AND location_id = $2")
                    .bind(&idempotency_key)
                    .bind(location_id)
                    .execute(&mut *txn)
                    .await?;
            }
            IdempotencyDecision::Proceed => {}
        }

        // 4. Product snapshot (in-tx MVCC price authority, orders.ts:414 — NOT `FOR UPDATE`).
        let product_ids: Vec<Uuid> = input.items.iter().map(|i| i.product_id).collect();
        let products: Vec<(Uuid, String, i64, bool)> = sqlx::query_as(
            "SELECT id, name, price::bigint, is_available FROM products
               WHERE id = ANY($1) AND location_id = $2",
        )
        .bind(&product_ids)
        .bind(location_id)
        .fetch_all(&mut *txn)
        .await?;

        let mut product_map = std::collections::HashMap::new();
        for (id, name, price, is_available) in products {
            if !is_available {
                txn.rollback().await.ok();
                return Ok(CreateOutcome::Rejected(
                    ErrorCode::ProductUnavailable,
                    "Product unavailable".to_string(),
                ));
            }
            product_map.insert(
                id.to_string(),
                ProductInfo {
                    name,
                    price: Lek::new(price).unwrap_or(Lek::ZERO),
                },
            );
        }
        // Every cart product must exist for this location (§6 PRODUCT_NOT_FOUND).
        for item in &input.items {
            if !product_map.contains_key(&item.product_id.to_string()) {
                txn.rollback().await.ok();
                return Ok(CreateOutcome::Rejected(
                    ErrorCode::ProductNotFound,
                    "Product not found".to_string(),
                ));
            }
        }

        // 4b. Modifier + group snapshot (available modifiers only).
        let (mod_map, groups_by_product) = load_modifier_snapshot(&mut txn, &product_ids).await?;

        // 5. Pricing (order-pricing.ts) — subtotal + priced rows, or the first 422.
        let pricing_items: Vec<PricingItem> = input
            .items
            .iter()
            .map(|i| PricingItem {
                product_id: i.product_id.to_string(),
                quantity: i.quantity,
                modifier_ids: i.modifier_ids.iter().map(Uuid::to_string).collect(),
            })
            .collect();
        let snapshot = PricingSnapshot {
            product_map: &product_map,
            mod_map: &mod_map,
            groups_by_product: &groups_by_product,
        };
        let (subtotal, _priced_rows) =
            match pricing::compute_order_pricing(&pricing_items, &snapshot) {
                Ok(v) => v,
                Err(e) => {
                    txn.rollback().await.ok();
                    return Ok(CreateOutcome::Rejected(e.code, e.message));
                }
            };

        // 6. Delivery-fee ladder (needs tiers for delivery). MIN_ORDER gate inside.
        let tiers: Vec<DeliveryTier> = if is_pickup {
            Vec::new()
        } else {
            let rows: Vec<(f64, i64)> = sqlx::query_as(
                "SELECT max_distance_km::double precision, fee::bigint FROM delivery_tiers
                   WHERE location_id = $1 ORDER BY max_distance_km ASC",
            )
            .bind(location_id)
            .fetch_all(&mut *txn)
            .await?;
            rows.into_iter()
                .map(|(max_distance_km, fee)| DeliveryTier {
                    max_distance_km,
                    fee,
                })
                .collect()
        };
        let fee_location = FeeLocation {
            lat,
            lng,
            delivery_fee_flat,
            free_delivery_threshold,
            min_order_value,
        };
        let delivery_fee =
            match pricing::delivery_fee_for_order(subtotal, is_pickup, fee_location, pin, &tiers) {
                Ok(f) => f,
                Err(e) => {
                    txn.rollback().await.ok();
                    return Ok(CreateOutcome::Rejected(e.code, e.message));
                }
            };

        // 7. Tax + LC1 + total (REV-S5-4). discountTotal = 0 CARRY (REV-S5-6). A money-math error
        // (unreachable for real inputs — see pricing.rs) is a hard 5xx-class bug, never a silent
        // wrong charge, so it surfaces as an error rather than a rejection outcome.
        let tax_i64 = pricing::apply_tax(
            subtotal.minor_units(),
            tax_rate.unwrap_or(0.0),
            price_includes_tax,
        )
        .map_err(|_e| RepoError(sqlx::Error::Protocol("tax overflow".into())))?;
        let tax_total = Lek::new(tax_i64)
            .map_err(|_e| RepoError(sqlx::Error::Protocol("tax negative".into())))?;
        let charged = pricing::charged_tax(tax_total, price_includes_tax);
        let total = pricing::compose_total(subtotal, delivery_fee, charged, Lek::ZERO)
            .map_err(|_e| RepoError(sqlx::Error::Protocol("total composition".into())))?;

        // 8. Cash gate (orders.ts:537).
        if let Some(cash) = input.cash_pay_with {
            if cash < total.minor_units() {
                txn.rollback().await.ok();
                return Ok(CreateOutcome::Rejected(
                    ErrorCode::CashAmountTooLow,
                    format!("Cash amount must be at least {}", total.minor_units()),
                ));
            }
        }

        // 9. Customer upsert (if a phone was supplied) — ON CONFLICT (location_id, phone).
        let resolved_customer_id: Option<Uuid> = match input
            .customer
            .as_ref()
            .and_then(|c| c.phone.as_ref())
        {
            Some(phone) => {
                let row: (Uuid,) = sqlx::query_as(
                    "INSERT INTO customers (location_id, phone, name)
                       VALUES ($1, $2, $3)
                     ON CONFLICT (location_id, phone) DO UPDATE SET name = COALESCE(EXCLUDED.name, customers.name)
                     RETURNING id",
                )
                .bind(location_id)
                .bind(phone)
                .bind(input.customer.as_ref().and_then(|c| c.name.clone()))
                .fetch_one(&mut *txn)
                .await?;
                Some(row.0)
            }
            None => None,
        };

        // 10. Persist the order + idempotency key. (Item/modifier rows + the secondary transactional
        // writes — velocity_events / customer_track_grants / enqueues — are CARRY-deferred to keep
        // this port focused on the money/tenancy/idempotency crown-jewel path; the RLS probe below
        // covers the core `orders` insert under NOBYPASSRLS.) Money columns are int4 → `::integer`.
        //
        // request_hash + the computed breakdown (delivery_fee, tax_total) MUST persist here:
        //   • request_hash is NOT NULL with no default — omitting it 500'd EVERY create (staging
        //     oracle 2026-07-05); it's the same value the idempotency-key row already carries.
        //   • delivery_fee / tax_total are computed at steps 6-7 above — leaving them at the
        //     table's 0 default would diverge the DB breakdown from the charged `total` on any
        //     fee/tax order (a silent money-row bug, NOT the documented item/secondary scope cut).
        // discount_total stays 0 (the deliberate REV-S5-6 CARRY); currency_code takes its 'ALL'
        // default (matches Node for this build's single-currency tenants).
        let delivery_address = input.delivery.as_ref().and_then(|d| d.address_text.clone());
        let metadata = serde_json::json!({ "channel": cmd.channel });
        let order_row: Result<CreatedRow, sqlx::Error> = sqlx::query_as(CREATE_ORDER_SQL)
            .bind(location_id)
            .bind(resolved_customer_id)
            .bind(if is_pickup { "pickup" } else { "delivery" })
            .bind(delivery_address)
            .bind(pin.map(|(lat, _)| lat))
            .bind(pin.map(|(_, lng)| lng))
            .bind(subtotal.minor_units())
            .bind(delivery_fee.minor_units())
            // tax_total column = the GROSS VAT figure for records (orders.ts:528-531,573 binds
            // `taxTotal`, NOT `chargedTax`); `charged` (0 when price-inclusive) only feeds `total`.
            .bind(tax_total.minor_units())
            .bind(total.minor_units())
            .bind(input.cash_pay_with)
            .bind(&request_hash)
            .bind(input.delivery_instructions.clone())
            .bind(metadata)
            .fetch_one(&mut *txn)
            .await;

        let order_row = match order_row {
            Ok(r) => r,
            Err(e) => return Ok(classify_create_error(e, txn).await),
        };

        // idempotency key row — same tx as the order (atomic dedup token).
        if let Err(e) = sqlx::query(
            "INSERT INTO idempotency_keys (key, location_id, request_hash, order_id, response_code)
             VALUES ($1, $2, $3, $4, 201)",
        )
        .bind(&idempotency_key)
        .bind(location_id)
        .bind(&request_hash)
        .bind(order_row.0)
        .execute(&mut *txn)
        .await
        {
            return Ok(classify_create_error(e, txn).await);
        }

        match txn.commit().await {
            Ok(()) => Ok(CreateOutcome::Created(order_row_from(order_row)?)),
            Err(e) => Ok(classify_commit_error(e)),
        }
    }

    async fn owner_update_status(
        &self,
        owner_user_id: Uuid,
        order_id: Uuid,
        new_status: OrderStatus,
    ) -> Result<StatusUpdateOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, |txn| {
            Box::pin(async move {
                // 1. Membership-JOIN authz (orders.ts:891) — 0 rows → NotFound (existence-hiding).
                // `o.type::text` — order_type is an enum; sqlx cannot decode an enum into a Rust
                // String without the cast, so the uncast read 500'd EVERY owner status transition
                // (staging oracle 2026-07-05 — the read-side twin of the bound-param enum class).
                let cur: Option<(String, Uuid, String)> = sqlx::query_as(
                    "SELECT o.status::text, o.location_id, o.type::text
                       FROM orders o
                       JOIN memberships m ON m.location_id = o.location_id
                      WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' AND m.status = 'active'",
                )
                .bind(order_id)
                .bind(owner_user_id)
                .fetch_optional(&mut **txn)
                .await?;

                let Some((current_str, location_id, order_type)) = cur else {
                    return Ok(StatusUpdateOutcome::NotFound);
                };
                let Some(current) = parse_status(&current_str) else {
                    return Ok(StatusUpdateOutcome::NotFound);
                };

                // 2. Machine legality (frozen matrix) → 400/409 class.
                if let Err(e) = domain::assert_transition(current, new_status) {
                    let code = e.code();
                    return Ok(StatusUpdateOutcome::Rejected(code, e.to_string()));
                }
                // 3. Actor-gate (orderAuthz.ts) — owner may not drive SYSTEM-only cancels → 403.
                if let Err(code) = state::assert_owner_target_allowed(current, new_status) {
                    return Ok(StatusUpdateOutcome::Rejected(
                        code,
                        "Cancelling an order in preparation is not available".to_string(),
                    ));
                }
                // 4. CC-1 strand guard (orders.ts:929) for DELIVERED/PICKED_UP.
                if new_status == OrderStatus::Delivered || new_status == OrderStatus::PickedUp {
                    let binding = read_binding_state(txn, order_id).await?;
                    if let Err(code) = state::cc1_strand_guard(new_status, current, binding) {
                        let msg = if code == ErrorCode::AssignmentActive {
                            "Order has an active courier assignment — complete it via the deliver flow"
                        } else {
                            "Order is in delivery without a delivered assignment — complete it via the deliver flow"
                        };
                        return Ok(StatusUpdateOutcome::Rejected(code, msg.to_string()));
                    }
                }
                // 5. Honest dispatch (orders.ts:962) — REV-S5-9 L2 CARRY: dispatch-then-advance. The
                // dispatch ENGINE is S7; carry the ORDERING with the engine stubbed to "no courier"
                // (stay put, never advance-then-orphan).
                if state::needs_honest_dispatch(new_status, order_type == "delivery") {
                    return Ok(StatusUpdateOutcome::Dispatched {
                        status: current, // stayed put (no courier) — the F1-orphan fix
                        dispatched: false,
                        reason: Some("no_courier".to_string()),
                    });
                }
                // 6. Apply the transition + folds.
                match apply_transition(txn, order_id, location_id, current, new_status).await? {
                    true => Ok(StatusUpdateOutcome::Updated(new_status)),
                    false => Ok(StatusUpdateOutcome::Rejected(
                        ErrorCode::Conflict,
                        "Order status already changed".to_string(),
                    )),
                }
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn owner_order_action(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        order_id: Uuid,
        new_status: OrderStatus,
        reject_reason: Option<String>,
    ) -> Result<StatusUpdateOutcome, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // Membership-JOIN authz scoped to the URL location (IDOR: transitionOrder's
                // `WHERE id=$1 AND location_id=$2`, dashboard.ts:632). `o.type::text` per the
                // enum-decode class. 0 rows → NotFound.
                let cur: Option<(String, String)> = sqlx::query_as(
                    "SELECT o.status::text, o.type::text
                       FROM orders o
                       JOIN memberships m ON m.location_id = o.location_id
                      WHERE o.id = $1 AND o.location_id = $2
                        AND m.user_id = $3 AND m.role = 'owner' AND m.status = 'active'",
                )
                .bind(order_id)
                .bind(location_id)
                .bind(owner_user_id)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((current_str, order_type)) = cur else {
                    return Ok(StatusUpdateOutcome::NotFound);
                };
                let Some(current) = parse_status(&current_str) else {
                    return Ok(StatusUpdateOutcome::NotFound);
                };
                if let Err(e) = domain::assert_transition(current, new_status) {
                    return Ok(StatusUpdateOutcome::Rejected(e.code(), e.to_string()));
                }
                if let Err(code) = state::assert_owner_target_allowed(current, new_status) {
                    return Ok(StatusUpdateOutcome::Rejected(
                        code,
                        "Cancelling an order in preparation is not available".to_string(),
                    ));
                }
                if new_status == OrderStatus::Delivered || new_status == OrderStatus::PickedUp {
                    let binding = read_binding_state(txn, order_id).await?;
                    if let Err(code) = state::cc1_strand_guard(new_status, current, binding) {
                        let msg = if code == ErrorCode::AssignmentActive {
                            "Order has an active courier assignment — complete it via the deliver flow"
                        } else {
                            "Order is in delivery without a delivered assignment — complete it via the deliver flow"
                        };
                        return Ok(StatusUpdateOutcome::Rejected(code, msg.to_string()));
                    }
                }
                if state::needs_honest_dispatch(new_status, order_type == "delivery") {
                    return Ok(StatusUpdateOutcome::Dispatched {
                        status: current,
                        dispatched: false,
                        reason: Some("no_courier".to_string()),
                    });
                }
                match apply_transition(txn, order_id, location_id, current, new_status).await? {
                    true => {
                        // rejection_reason is set separately (updateOrderStatus doesn't own it —
                        // dashboard.ts:652), same tx, only on REJECTED with a reason.
                        if new_status == OrderStatus::Rejected {
                            if let Some(reason) = reject_reason {
                                sqlx::query(
                                    "UPDATE orders SET rejection_reason = $1 WHERE id = $2 AND location_id = $3",
                                )
                                .bind(reason)
                                .bind(order_id)
                                .bind(location_id)
                                .execute(&mut **txn)
                                .await?;
                            }
                        }
                        Ok(StatusUpdateOutcome::Updated(new_status))
                    }
                    false => Ok(StatusUpdateOutcome::Rejected(
                        ErrorCode::Conflict,
                        "Order status already changed".to_string(),
                    )),
                }
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn patch_order_test_metadata(
        &self,
        owner_user_id: Uuid,
        location_id: Uuid,
        order_id: Uuid,
        test_order: bool,
    ) -> Result<Option<()>, RepoError> {
        crate::db::with_user(&self.pool, owner_user_id, move |txn| {
            Box::pin(async move {
                // Membership check folded into the UPDATE's location scope (order-meta.ts:16) —
                // jsonb_set merges ONLY metadata.test_order, leaving the rest intact.
                let row: Option<(Uuid,)> = sqlx::query_as(
                    "UPDATE orders o
                        SET metadata = jsonb_set(COALESCE(o.metadata, '{}'::jsonb), '{test_order}', $1::jsonb, true)
                       FROM memberships m
                      WHERE o.id = $2 AND o.location_id = $3
                        AND m.location_id = o.location_id AND m.user_id = $4
                        AND m.role = 'owner' AND m.status = 'active'
                      RETURNING o.id",
                )
                .bind(serde_json::Value::Bool(test_order))
                .bind(order_id)
                .bind(location_id)
                .bind(owner_user_id)
                .fetch_optional(&mut **txn)
                .await?;
                Ok(row.map(|_| ()))
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn customer_cancel(
        &self,
        customer_sub: Uuid,
        order_id: Uuid,
    ) -> Result<CustomerCancelOutcome, RepoError> {
        // Resolve ownership + location + window inputs FIRST (bare read — anonymous_select admits a
        // GUC-less connection), THEN seat `app.current_tenant = location` for the mutation
        // (REV-S5-1 customer half / LC3 GUC dance, customer/orders.ts:324).
        let row: Option<(Uuid, String, Option<String>)> = sqlx::query_as(
            "SELECT location_id, status::text, picked_up_at::text
               FROM orders WHERE id = $1 AND customer_id = $2",
        )
        .bind(order_id)
        .bind(customer_sub)
        .fetch_optional(&self.pool)
        .await?;

        let Some((location_id, status_str, picked_up_at)) = row else {
            return Ok(CustomerCancelOutcome::NotFound);
        };
        if status_str != "IN_DELIVERY" {
            return Ok(CustomerCancelOutcome::NotInDelivery);
        }
        // Window: now − picked_up_at > CANCEL_WINDOW_MS → expired (customer/orders.ts:302).
        if let Some(picked_up) = picked_up_at.as_deref().and_then(parse_ts_millis) {
            let now = chrono::Utc::now().timestamp_millis();
            if now - picked_up > CANCEL_WINDOW_MS {
                return Ok(CustomerCancelOutcome::WindowExpired);
            }
        }

        let tenant = domain::TenantId::from(location_id);
        crate::db::with_tenant(&self.pool, tenant, |txn| {
            Box::pin(async move {
                // FOR UPDATE re-read under the seated GUC, still bound to (order, customer).
                let live: Option<(String,)> = sqlx::query_as(
                    "SELECT status::text FROM orders WHERE id = $1 AND customer_id = $2 FOR UPDATE",
                )
                .bind(order_id)
                .bind(customer_sub)
                .fetch_optional(&mut **txn)
                .await?;
                let Some((live_status,)) = live else {
                    return Ok(CustomerCancelOutcome::NotFound);
                };
                if live_status != "IN_DELIVERY" {
                    return Ok(CustomerCancelOutcome::NotInDelivery);
                }
                let applied = apply_transition(
                    txn,
                    order_id,
                    location_id,
                    OrderStatus::InDelivery,
                    OrderStatus::Cancelled,
                )
                .await?;
                if applied {
                    Ok(CustomerCancelOutcome::Cancelled)
                } else {
                    Ok(CustomerCancelOutcome::NotInDelivery)
                }
            })
        })
        .await
        .map_err(map_txn_err)
    }

    async fn get_order(
        &self,
        order_id: Uuid,
        owner_user_id: Option<Uuid>,
        customer_sub: Option<Uuid>,
    ) -> Result<OrderReadOutcome, RepoError> {
        // Owner: membership-JOIN (the JOIN is the tenant boundary, orders.ts:800). Customer:
        // order-scope already enforced in the handler (orderId claim == path), bind customer_id here.
        let row: Option<(Uuid, String, i64, i64, Uuid)> = if let Some(user_id) = owner_user_id {
            sqlx::query_as(
                "SELECT o.id, o.status::text, o.subtotal::bigint, o.total::bigint, o.location_id
                   FROM orders o
                   JOIN memberships m ON m.location_id = o.location_id
                  WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' AND m.status = 'active'",
            )
            .bind(order_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?
        } else if let Some(sub) = customer_sub {
            sqlx::query_as(
                "SELECT id, status::text, subtotal::bigint, total::bigint, location_id
                   FROM orders WHERE id = $1 AND customer_id = $2",
            )
            .bind(order_id)
            .bind(sub)
            .fetch_optional(&self.pool)
            .await?
        } else {
            None
        };

        Ok(match row {
            Some((id, status_str, subtotal, total, location_id)) => match parse_status(&status_str)
            {
                Some(status) => OrderReadOutcome::Found(OrderView {
                    id,
                    status,
                    subtotal: Lek::new(subtotal).unwrap_or(Lek::ZERO),
                    total: Lek::new(total).unwrap_or(Lek::ZERO),
                    location_id,
                }),
                None => OrderReadOutcome::NotFound,
            },
            None => OrderReadOutcome::NotFound,
        })
    }
}

// ─────────────────────────────── shared SQL helpers ───────────────────────────────

/// The `updateOrderStatus` mutator (`orderStatusService.ts`) — the status-guarded UPDATE (0 rows →
/// `false` = 409 CONFLICT) + the R2-3 assignment-terminalize fold + the L-A `refund_due` fold +
/// history + lifecycle bus. Uses [`state::transition_effects`] for the fold decisions. Runs inside
/// the caller's tenant-seated tx. Returns `Ok(true)` on apply, `Ok(false)` on a status-race.
///
/// `pub(crate)` (not module-private): S7 courier/dispatch (`routes::courier::assignments`) is the
/// SECOND caller of this exact mutator (`docs/design/rebuild-courier-s7-council/proposal.md` §4.3
/// — "every order-side transition funnels through S5's `updateOrderStatus`"). This function is
/// already GUC-agnostic (it takes an open `&mut Transaction`, does not itself seat any GUC) — S7
/// opens its OWN `with_tenant(activeLocationId)` transaction for the courier-side assignment/shift
/// writes, then calls this SAME function for the order-side fold, so the two surfaces never fork
/// the fold logic. Nothing about this function's body changes for the new caller.
pub(crate) async fn apply_transition(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: Uuid,
    location_id: Uuid,
    current: OrderStatus,
    new_status: OrderStatus,
) -> Result<bool, sqlx::Error> {
    let fx = state::transition_effects(current, new_status);
    let new_str = status_pg(new_status);
    let current_str = status_pg(current);

    // Status-guarded UPDATE (anti-race). The stamp column comes from the fixed allowlist (never user
    // input — safe to interpolate), matching orderStatusService.ts:114.
    let stamp = fx
        .stamp_column
        .map(|c| format!(", {c} = now()"))
        .unwrap_or_default();
    let sql = format!(
        "UPDATE orders SET status = $1::order_status, timeout_at = NULL{stamp}
           WHERE id = $2 AND status = $3::order_status RETURNING id"
    );
    let updated: Option<(Uuid,)> = sqlx::query_as(&sql)
        .bind(new_str)
        .bind(order_id)
        .bind(current_str)
        .fetch_optional(&mut **txn)
        .await?;
    if updated.is_none() {
        return Ok(false); // 0 rows → 409 CONFLICT (concurrent transition)
    }

    // R2-3 assignment-terminalize fold.
    if fx.terminalize_assignment {
        sqlx::query(
            "WITH freed AS (
               UPDATE courier_assignments SET status = 'cancelled', cancelled_at = now(),
                      cancellation_reason = 'order_' || lower($2)
                WHERE order_id = $1 AND status IN ('offered','assigned','accepted','picked_up')
               RETURNING shift_id)
             UPDATE courier_shifts SET status = 'available'
              WHERE id IN (SELECT shift_id FROM freed WHERE shift_id IS NOT NULL)",
        )
        .bind(order_id)
        .bind(new_str)
        .execute(&mut **txn)
        .await?;
    }

    // L-A refund_due fold (SAVEPOINT-wrapped, idempotent). Inert until crypto flips (no `paid` rows).
    if fx.record_refund_due {
        sqlx::query("SAVEPOINT refund_due_fold")
            .execute(&mut **txn)
            .await?;
        let fold: Result<_, sqlx::Error> = sqlx::query(
            "INSERT INTO payment_events
               (payment_id, location_id, provider, provider_payment_id, type, amount_minor, currency_code, signature_verified)
             SELECT p.id, p.location_id, p.provider, p.provider_payment_id, 'refund_due', p.amount_minor, p.currency_code, true
               FROM payments p WHERE p.order_id = $1 AND p.status = 'paid'
             ON CONFLICT DO NOTHING",
        )
        .bind(order_id)
        .execute(&mut **txn)
        .await;
        match fold {
            Ok(_) => {
                sqlx::query("RELEASE SAVEPOINT refund_due_fold")
                    .execute(&mut **txn)
                    .await?;
            }
            Err(_) => {
                // fail-closed per order (ESC-2): roll back the fold and propagate — the cancel aborts.
                sqlx::query("ROLLBACK TO SAVEPOINT refund_due_fold")
                    .execute(&mut **txn)
                    .await?;
                return Err(sqlx::Error::Protocol("refund_due fold failed".into()));
            }
        }
    }

    // order_status_history audit (SAVEPOINT best-effort — a history failure never rolls back the
    // applied status).
    sqlx::query("SAVEPOINT osh").execute(&mut **txn).await?;
    let hist: Result<_, sqlx::Error> = sqlx::query(
        // $3/$4::order_status — enum columns bound as text (same class as CREATE_ORDER_SQL's
        // type::order_type). Uncast, this INSERT threw and the SAVEPOINT silently swallowed the
        // status-history audit row (staging oracle 2026-07-05) — data loss under a green status update.
        "INSERT INTO order_status_history (order_id, location_id, from_status, to_status, actor, comment)
         VALUES ($1, $2, $3::order_status, $4::order_status, 'system:updateOrderStatus', NULL)",
    )
    .bind(order_id)
    .bind(location_id)
    .bind(current_str)
    .bind(new_str)
    .execute(&mut **txn)
    .await;
    if hist.is_err() {
        sqlx::query("ROLLBACK TO SAVEPOINT osh")
            .execute(&mut **txn)
            .await?;
    } else {
        sqlx::query("RELEASE SAVEPOINT osh")
            .execute(&mut **txn)
            .await?;
    }

    // REV-S5-9 L1: the ORDER_CONFIRMED/REJECTED bus folds are published post-apply. The WS bus
    // transport is S6 — CARRY the fold DECISION (fx.lifecycle_event) here; the publish is a no-op
    // seam until S6 wires the bus (documented, not silently dropped).
    let _lifecycle = fx.lifecycle_event;

    Ok(true)
}

async fn read_binding_state(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    order_id: Uuid,
) -> Result<BindingState, sqlx::Error> {
    let active: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM courier_assignments
          WHERE order_id = $1 AND status IN ('offered','assigned','accepted','picked_up') LIMIT 1",
    )
    .bind(order_id)
    .fetch_optional(&mut **txn)
    .await?;
    let delivered: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM courier_assignments WHERE order_id = $1 AND status = 'delivered' LIMIT 1",
    )
    .bind(order_id)
    .fetch_optional(&mut **txn)
    .await?;
    Ok(BindingState {
        has_active_binding: active.is_some(),
        has_delivered_binding: delivered.is_some(),
    })
}

#[allow(clippy::type_complexity)]
async fn load_modifier_snapshot(
    txn: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    product_ids: &[Uuid],
) -> Result<
    (
        std::collections::HashMap<String, ModifierInfo>,
        std::collections::HashMap<String, Vec<GroupInfo>>,
    ),
    sqlx::Error,
> {
    // Available modifiers for these products (batch), keyed "{product_id}_{modifier_id}".
    let mod_rows: Vec<(Uuid, Uuid, String, i64, Uuid)> = sqlx::query_as(
        "SELECT pmg.product_id, m.id, m.name, m.price_delta::bigint, m.group_id
           FROM product_modifier_groups pmg
           JOIN modifiers m ON m.group_id = pmg.group_id
          WHERE pmg.product_id = ANY($1) AND m.available = true",
    )
    .bind(product_ids)
    .fetch_all(&mut **txn)
    .await?;
    let mut mod_map = std::collections::HashMap::new();
    for (product_id, mod_id, name, price_delta, group_id) in mod_rows {
        mod_map.insert(
            format!("{product_id}_{mod_id}"),
            ModifierInfo {
                name,
                price_delta: Lek::new(price_delta).unwrap_or(Lek::ZERO),
                group_id: group_id.to_string(),
            },
        );
    }
    let group_rows: Vec<(Uuid, Uuid, i32, i32, bool)> = sqlx::query_as(
        "SELECT pmg.product_id, mg.id, mg.min_select, mg.max_select, mg.required
           FROM product_modifier_groups pmg
           JOIN modifier_groups mg ON mg.id = pmg.group_id
          WHERE pmg.product_id = ANY($1)",
    )
    .bind(product_ids)
    .fetch_all(&mut **txn)
    .await?;
    let mut groups_by_product: std::collections::HashMap<String, Vec<GroupInfo>> =
        std::collections::HashMap::new();
    for (product_id, group_id, min_select, max_select, required) in group_rows {
        groups_by_product
            .entry(product_id.to_string())
            .or_default()
            .push(GroupInfo {
                id: group_id.to_string(),
                min_select: i64::from(min_select),
                max_select: i64::from(max_select),
                required,
            });
    }
    Ok((mod_map, groups_by_product))
}

// ─────────────────────────────── small pure helpers ───────────────────────────────

/// (id, location_id, status::text, subtotal, total, delivery_instructions, created_at::text) —
/// the `CREATE_ORDER_SQL` RETURNING shape.
type CreatedRow = (Uuid, Uuid, String, i64, i64, Option<String>, String);

fn order_row_from(r: CreatedRow) -> Result<OrderCreatedResponse, RepoError> {
    order_row(r)
}

fn order_row(r: CreatedRow) -> Result<OrderCreatedResponse, RepoError> {
    let (id, location_id, status_str, subtotal, total, delivery_instructions, created_at) = r;
    let status = parse_status(&status_str)
        .ok_or_else(|| RepoError(sqlx::Error::Protocol("unknown order status".into())))?;
    Ok(OrderCreatedResponse {
        id,
        location_id,
        status,
        subtotal: Lek::new(subtotal).unwrap_or(Lek::ZERO),
        total: Lek::new(total).unwrap_or(Lek::ZERO),
        delivery_instructions,
        created_at: Some(created_at),
    })
}

fn parse_status(s: &str) -> Option<OrderStatus> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

/// The Pg `order_status` enum text for a status (SCREAMING_SNAKE — matches the frozen serde repr).
fn status_pg(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Pending => "PENDING",
        OrderStatus::Confirmed => "CONFIRMED",
        OrderStatus::Preparing => "PREPARING",
        OrderStatus::Ready => "READY",
        OrderStatus::InDelivery => "IN_DELIVERY",
        OrderStatus::Delivered => "DELIVERED",
        OrderStatus::Rejected => "REJECTED",
        OrderStatus::Cancelled => "CANCELLED",
        OrderStatus::Scheduled => "SCHEDULED",
        OrderStatus::PickedUp => "PICKED_UP",
    }
}

fn parse_ts_millis(ts: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

/// Classify a create-tx statement error and roll back → the create outcome.
async fn classify_create_error(
    e: sqlx::Error,
    txn: sqlx::Transaction<'_, sqlx::Postgres>,
) -> CreateOutcome {
    txn.rollback().await.ok();
    classify_commit_error(e)
}

/// Classify a create error (statement or commit) by SQLSTATE (orders.ts:718-728).
fn classify_commit_error(e: sqlx::Error) -> CreateOutcome {
    use sqlx::error::DatabaseError;
    let sqlstate = e
        .as_database_error()
        .and_then(DatabaseError::code)
        .map(|c| c.to_string());
    match sqlstate.as_deref().map(state::classify_pg_error) {
        Some(state::PgErrorClass::Conflict) => CreateOutcome::Rejected(
            ErrorCode::IdempotencyConflict,
            "Idempotency key conflict".to_string(),
        ),
        Some(state::PgErrorClass::Transient) => CreateOutcome::Transient,
        _ => CreateOutcome::Rejected(ErrorCode::Internal, "Internal server error".to_string()),
    }
}

/// Maps `crate::db::TenantTxnError` onto `RepoError` — same rationale as `owner::menu_availability`.
fn map_txn_err(err: crate::db::TenantTxnError) -> RepoError {
    use crate::db::TenantTxnError as E;
    let sqlx_err = match err {
        E::Begin(e) | E::SetTenant(e) | E::Work(e) | E::Commit(e) => e,
        E::WorkThenRollbackFailed { work, .. } => work,
    };
    RepoError(sqlx_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure helper: the Pg `order_status` text round-trips with the frozen serde repr.
    #[test]
    fn status_pg_matches_frozen_serde_repr() {
        for status in domain::ALL_STATUSES {
            let pg = status_pg(status);
            assert_eq!(parse_status(pg), Some(status), "{status:?} round-trips");
        }
    }

    /// REV-S5-1 DISCRIMINATING NOBYPASSRLS PROBE (the DoD gate). Requires a live Postgres on a
    /// NOBYPASSRLS role. Proves the create-path tenancy in BOTH directions:
    ///   (a) an `orders` INSERT with `app.user_id` UNSET is ADMITTED (anonymous_insert WITH CHECK
    ///       `app_current_user() IS NULL`);
    ///   (b) the SAME INSERT with `app.user_id` SET is REJECTED (the WITH CHECK becomes false).
    /// This is the check the packet's "seating helps" non-discriminating probe would have missed —
    /// it proves seating a GUC BREAKS the create, which is exactly why the port is GUC-less.
    #[tokio::test]
    #[ignore = "requires a live NOBYPASSRLS Postgres — set DATABASE_URL_OPERATIONAL and run with --ignored"]
    async fn anonymous_create_admitted_only_with_no_app_user_id() {
        let config =
            crate::config::Config::from_env().expect("env must be valid to run this ignored probe");
        let pools = crate::db::Pools::connect(&config)
            .await
            .expect("pools must connect");

        // Seed a published location to satisfy the FK (best-effort; a real staging DB has one).
        let location_id = Uuid::new_v4();

        // (a) GUC-less INSERT → ADMITTED.
        let admitted = {
            let mut txn = pools.operational.begin().await.unwrap();
            // NO set_config here (GUC-LESS BY DESIGN).
            let res = sqlx::query(
                "INSERT INTO orders (location_id, type, status, subtotal, total, payment_method, request_hash)
                 VALUES ($1, 'pickup', 'PENDING', 0, 0, 'cash', md5(random()::text))",
            )
            .bind(location_id)
            .execute(&mut *txn)
            .await;
            txn.rollback().await.ok();
            res.is_ok()
        };

        // (b) SAME INSERT with app.user_id SET → REJECTED (WITH CHECK app_current_user() IS NULL false).
        let rejected_when_seated = {
            let mut txn = pools.operational.begin().await.unwrap();
            sqlx::query("SELECT set_config('app.user_id', $1, true)")
                .bind(Uuid::new_v4().to_string())
                .execute(&mut *txn)
                .await
                .unwrap();
            let res = sqlx::query(
                "INSERT INTO orders (location_id, type, status, subtotal, total, payment_method, request_hash)
                 VALUES ($1, 'pickup', 'PENDING', 0, 0, 'cash', md5(random()::text))",
            )
            .bind(location_id)
            .execute(&mut *txn)
            .await;
            txn.rollback().await.ok();
            res.is_err()
        };

        assert!(admitted, "GUC-less anonymous create must be ADMITTED");
        assert!(
            rejected_when_seated,
            "seating app.user_id on the create path must be REJECTED (this is why the port is GUC-less)"
        );
    }

    /// Deterministic guardrail (no DB): the crown-jewel INSERT must persist EVERY money value the
    /// create fn computes. A dropped column here is a silent money-row bug or (request_hash) a
    /// 500 on every create — the exact regression the staging oracle caught 2026-07-05.
    #[test]
    fn create_order_sql_persists_every_computed_money_column() {
        for col in [
            "request_hash", // NOT NULL, no default — its absence 500'd every create
            "subtotal",
            "delivery_fee", // computed at step 6 — must not fall back to the 0 default
            "tax_total",    // gross VAT (orders.ts:573), not the charged tax
            "total",
        ] {
            assert!(
                CREATE_ORDER_SQL.contains(col),
                "CREATE_ORDER_SQL must persist `{col}` — omitting a computed money column diverges \
                 the DB row from the charged total (or, for request_hash, 500s the whole create)"
            );
        }
        // discount_total is the deliberate REV-S5-6 CARRY (0 default) — it must NOT be bound here.
        assert!(
            !CREATE_ORDER_SQL.contains("discount_total"),
            "discount_total stays the table default (REV-S5-6 CARRY), never bound"
        );
        // `type` is the `order_type` ENUM — a bound text param needs an explicit cast or Postgres
        // rejects it ("expression is of type text"). This 500'd every create (staging oracle 2026-07-05).
        assert!(
            CREATE_ORDER_SQL.contains("$3::order_type"),
            "the bound `type` param must cast to order_type — Postgres won't coerce text→enum"
        );
        // Create-response parity (staging oracle): Node returns camelCase locationId +
        // deliveryInstructions + createdAt — the RETURNING must surface them so order_row echoes.
        assert!(
            CREATE_ORDER_SQL.contains("RETURNING id, location_id"),
            "RETURNING must lead with id, location_id for the Node-parity create response"
        );
        for col in ["delivery_instructions", "created_at"] {
            assert!(
                CREATE_ORDER_SQL.contains(col),
                "RETURNING must surface `{col}` for the Node-parity create response"
            );
        }
    }
}
