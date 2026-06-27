import type { QueueProvider } from '@deliveryos/platform';
import { QUEUE_NAMES } from './registry.js';
import { generateOpaqueToken } from './otp.js';
import type { PricedOrderItemRow } from './order-pricing.js';

// Side-effectful persistence block extracted from POST /orders (sections 11-14).
// Owns the order INSERT, velocity event, order_items/modifiers, idempotency key,
// customer track grant, and the two transactional enqueues. It does NOT own
// transaction control — the caller keeps BEGIN/COMMIT/ROLLBACK so the whole
// block stays inside the same write transaction. Takes the live `client` and
// `queue` so it can be unit-tested with a stub that captures SQL + params.

/** Minimal pg-client surface used here (matches the in-tx client from db.connect()). */
export interface TxClient {
  query(sql: string, params?: unknown[]): Promise<{ rows: any[]; rowCount?: number | null }>;
}

export interface InsertOrderInput {
  locationId: string;
  resolvedCustomerId: string | null;
  deliveryAddressText: string | null;
  pin: { lat: number; lng: number } | null;
  subtotal: number;
  deliveryFee: number;
  taxTotal: number;
  discountTotal: number;
  total: number;
  cashPayWith: number | null | undefined;
  currencyCode: string;
  menuVersion: string;
  clientMenuVersion: string | null;
  requestHash: string;
  timeoutAt: Date;
  rawInstructions: string | null | undefined;
  otpServerVerified: boolean;
  clientIpHash: string | undefined;
  preflightMeta: string;
  type: string;
  messengerKind: string | null;
  messengerHandle: string | null;
  deliveryPhotoKey: string | null;
  tipAmount: number | undefined;
  orderItemRows: PricedOrderItemRow[];
  idempotencyKey: string;
  phoneHash: string | undefined;
  custPhone: string | undefined;
}

export interface PersistedOrder {
  order: { id: string; status: string; subtotal: number; total: number; created_at: string; timeout_at: string };
  /** Raw single-use track code (sha256 persisted), present only for a resolved customer. */
  trackCode?: string;
}

/**
 * Persists the order and all dependent rows, then enqueues the timeout +
 * notification jobs transactionally. Mirrors orders.ts sections 11-14 exactly,
 * including the $1..$24 positional INSERT mapping and the falsy/nullish
 * coercions. Returns the order row + (optional) track code for the post-commit
 * response. Caller COMMITs after this resolves.
 */
export async function insertOrderWithItems(
  client: TxClient,
  queue: QueueProvider,
  input: InsertOrderInput,
): Promise<PersistedOrder> {
  // 11. Insert Orders
  const orderRes = await client.query(
    `INSERT INTO orders (
      location_id, customer_id, type, status,
      delivery_address, delivery_lat, delivery_lng,
      subtotal, delivery_fee, tax_total, discount_total, total,
      payment_method, cash_pay_with, currency_code,
      menu_version, client_menu_version, request_hash, timeout_at,
      delivery_instructions,
      metadata,
      preflight,
      customer_messenger_kind, customer_messenger_handle,
      delivery_photo_key, tip_amount
     )
     VALUES ($1, $2, $20, 'PENDING', $3, $4, $5, $6, $7, $8, $9, $10, 'cash', $11, $12, $13, $14, $15, $16, $17, $18, $19, $21, $22, $23, $24)
     RETURNING id, status, subtotal, total, created_at::text, timeout_at::text`,
    [
      input.locationId, input.resolvedCustomerId, input.deliveryAddressText, input.pin?.lat ?? null, input.pin?.lng ?? null,
      input.subtotal, input.deliveryFee, input.taxTotal, input.discountTotal, input.total,
      input.cashPayWith ?? null, input.currencyCode,
      input.menuVersion, input.clientMenuVersion || null, input.requestHash, input.timeoutAt,
      input.rawInstructions || null,
      JSON.stringify({ otp_verified: input.otpServerVerified, client_ip_hash: input.clientIpHash }),
      input.preflightMeta,
      input.type,
      input.messengerKind || null, input.messengerHandle || null,
      input.deliveryPhotoKey || null,
      input.tipAmount || 0,
    ],
  );
  const order = orderRes.rows[0];

  // 12b. Record velocity event (for FX-4 throttle + signal computation)
  if (input.phoneHash || input.clientIpHash) {
    await client.query(
      `INSERT INTO velocity_events (location_id, phone_hash, client_ip_hash, kind, window_started_at)
       VALUES ($1, $2, $3, 'order_placed', $4)`,
      [input.locationId, input.phoneHash || null, input.clientIpHash || null, new Date().toISOString()],
    );
  }

  // 13. Insert Order Items & Modifiers
  for (const row of input.orderItemRows) {
    const oiRes = await client.query(
      `INSERT INTO order_items (order_id, product_id, name_snapshot, price_snapshot, quantity)
       VALUES ($1, $2, $3, $4, $5) RETURNING id`,
      [order.id, row.productId, row.nameSnapshot, row.priceSnapshot, row.quantity],
    );
    const oiId = oiRes.rows[0].id;

    for (const mod of row.modifiers) {
      await client.query(
        `INSERT INTO order_item_modifiers (order_item_id, modifier_id, name_snapshot, price_delta_snapshot)
         VALUES ($1, $2, $3, $4)`,
        [oiId, mod.modifierId, mod.nameSnapshot, mod.priceDeltaSnapshot],
      );
    }
  }

  // 13. Idempotency Key
  await client.query(
    `INSERT INTO idempotency_keys (key, location_id, request_hash, order_id, response_code)
     VALUES ($1, $2, $3, $4, 201)`,
    [input.idempotencyKey, input.locationId, input.requestHash, order.id],
  );

  // 13b. Customer track grant — single-use opaque code backing the ?t= tracking
  // link. Only minted when there's a resolved customer (same condition as the
  // customer JWT). Only the sha256 hash is persisted; the raw code is returned.
  let trackCode: string | undefined;
  if (input.custPhone && input.resolvedCustomerId) {
    const { token, hash } = generateOpaqueToken();
    await client.query(
      `INSERT INTO customer_track_grants (order_id, location_id, token_hash, expires_at)
       VALUES ($1, $2, $3, now() + interval '14 days')`,
      [order.id, input.locationId, hash],
    );
    trackCode = token;
  }

  // 14. Transactional Enqueue — timeout sweep + NX-3 notification outbox, both
  // written inside this transaction so they're durable independent of NOTIFY.
  const txDb = { executeSql: (sql: string, values: any[]) => client.query(sql, values) };

  await queue.enqueue(QUEUE_NAMES.ORDER_TIMEOUT, { orderId: order.id, locationId: input.locationId }, {
    singletonKey: order.id,
    startAfter: new Date(input.timeoutAt),
    db: txDb,
  });

  const dedupKey = `order.created:${order.id}:${input.locationId}`;
  await queue.enqueue(QUEUE_NAMES.NOTIFY_TELEGRAM_SEND, {
    event: 'order.created',
    entity_id: order.id,
    location_id: input.locationId,
    dedupKey,
  }, {
    singletonKey: dedupKey,
    db: txDb,
  });
  console.log(`[ORDERS] NX-3: notify.telegram.send enqueued transactionally for order ${order.id} (key: ${dedupKey})`);

  return { order, trackCode };
}
