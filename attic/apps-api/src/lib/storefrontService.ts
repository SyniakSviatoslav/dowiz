import type { PoolClient } from 'pg';
import crypto from 'node:crypto';

/**
 * Storefront accepting-orders toggle (locations.delivery_paused) for the Telegram
 * inbound action. Council-hardened (docs/design/telegram-notifications-actions/):
 *
 *  - BR-13: every mutation runs inside an explicit BEGIN/COMMIT so the transaction-local
 *    `app.user_id` GUC persists across statements (the codebase's order/webhook paths run
 *    in autocommit, which would reset it after one statement).
 *  - BR-23: sets the canonical `app.user_id` GUC (the locations FORCE policy reads it, NOT
 *    `app.current_tenant`). The cur-FOR-UPDATE CTE makes rowCount=0 mean "row invisible /
 *    not found" (-> 'denied', never reported as success) while distinguishing a legitimate
 *    idempotent double-tap (was === paused -> 'noop') from a real change.
 *  - BR-15/19: close is gated by a one-shot nonce consumed atomically in the same txn as
 *    the toggle, so a replayed confirm cannot re-toggle.
 *
 * Authority (chatId<->membership<->location) is verified by the caller BEFORE these run;
 * the `app.user_id` set here is defence-in-depth that also works if the operational role
 * is ever made non-bypassing.
 */

export type StorefrontToggleResult =
  | { result: 'changed'; paused: boolean }
  | { result: 'noop'; paused: boolean }
  | { result: 'denied' }
  | { result: 'nonce_invalid' };

const CLOSE_NONCE_TTL_MS = 2 * 60 * 1000;

export async function getLocationStorefront(
  client: PoolClient,
  locationId: string,
): Promise<{ name: string; paused: boolean } | null> {
  const res = await client.query(
    `SELECT name, delivery_paused FROM locations WHERE id = $1`,
    [locationId],
  );
  if (res.rowCount === 0) return null;
  return { name: res.rows[0].name, paused: res.rows[0].delivery_paused ?? false };
}

/** Create a one-shot close-confirmation nonce (TTL 2 min). Returns the nonce string. */
export async function createCloseNonce(
  client: PoolClient,
  locationId: string,
  userId: string,
  chatId: string,
): Promise<string> {
  // Short token (12 hex) so `store.confirm:<uuid-loc>:<nonce>` fits Telegram's 64-byte
  // callback_data limit. One-shot + 2-min TTL + bound to (location,user); UNIQUE column
  // guards the negligible collision chance.
  const nonce = crypto.randomBytes(6).toString('hex');
  const expiresAt = new Date(Date.now() + CLOSE_NONCE_TTL_MS).toISOString();
  await client.query('BEGIN');
  try {
    await client.query("SELECT set_config('app.user_id', $1, true)", [userId]);
    await client.query(
      `INSERT INTO telegram_action_nonces (nonce, location_id, user_id, action, chat_id, expires_at)
       VALUES ($1, $2, $3, 'store.close', $4, $5)`,
      [nonce, locationId, userId, chatId, expiresAt],
    );
    await client.query('COMMIT');
  } catch (err) {
    await client.query('ROLLBACK').catch(() => {});
    throw err;
  }
  return nonce;
}

/**
 * Idempotent, visibility-aware toggle of locations.delivery_paused. When `consumeNonce`
 * is given (close-confirm), the nonce is consumed atomically with the toggle — if it is
 * missing/expired the whole txn rolls back ('nonce_invalid') and nothing is toggled.
 */
export async function setStorefrontPaused(
  client: PoolClient,
  locationId: string,
  actingUserId: string,
  paused: boolean,
  opts?: { consumeNonce?: string },
): Promise<StorefrontToggleResult> {
  await client.query('BEGIN');
  try {
    await client.query("SELECT set_config('app.user_id', $1, true)", [actingUserId]);

    if (opts?.consumeNonce) {
      const n = await client.query(
        `DELETE FROM telegram_action_nonces
          WHERE nonce = $1 AND location_id = $2 AND user_id = $3
            AND action = 'store.close' AND expires_at > now()
          RETURNING id`,
        [opts.consumeNonce, locationId, actingUserId],
      );
      if (n.rowCount === 0) {
        await client.query('ROLLBACK');
        return { result: 'nonce_invalid' };
      }
    }

    const res = await client.query(
      `WITH cur AS (
         SELECT delivery_paused AS was FROM locations WHERE id = $1 FOR UPDATE
       )
       UPDATE locations l SET delivery_paused = $2
       FROM cur WHERE l.id = $1
       RETURNING cur.was`,
      [locationId, paused],
    );
    if (res.rowCount === 0) {
      await client.query('ROLLBACK');
      return { result: 'denied' };
    }
    const was = res.rows[0].was as boolean;
    await client.query('COMMIT');
    return { result: was === paused ? 'noop' : 'changed', paused };
  } catch (err) {
    await client.query('ROLLBACK').catch(() => {});
    throw err;
  }
}
