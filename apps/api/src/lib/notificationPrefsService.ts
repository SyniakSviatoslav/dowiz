import type { PoolClient } from 'pg';

/**
 * Single writer for notification CATEGORY preferences (operational / quality), shared by
 * the web preference-centre and the Telegram /settings toggle so both mutate the same
 * source of truth. Council-hardened (docs/design/telegram-notifications-actions/):
 *
 *  - BR-4: atomic per-cell `jsonb_set` under a FOR UPDATE row lock — NO read-merge-write,
 *    so concurrent web<->telegram toggles of different categories cannot lose updates.
 *  - BR-16: the consent audit row is INSERTed in the SAME transaction as the prefs change,
 *    so the GDPR trail can never desync from the value. PII-free.
 *  - sets the canonical `app.user_id` GUC (FORCE-ready; harmless if the role bypasses RLS).
 */

export type NotificationCategoryKey = 'operational' | 'quality';

export const TOGGLEABLE_CATEGORIES: NotificationCategoryKey[] = ['operational', 'quality'];

export function isToggleableCategory(key: string): key is NotificationCategoryKey {
  return key === 'operational' || key === 'quality';
}

export async function setCategoryPref(
  client: PoolClient,
  params: {
    targetId: string;
    locationId: string;
    userId: string | null;
    category: NotificationCategoryKey;
    value: boolean;
    changedVia: 'web' | 'telegram';
  },
): Promise<{ ok: boolean; oldValue: boolean | null; newValue: boolean }> {
  const { targetId, locationId, userId, category, value, changedVia } = params;
  await client.query('BEGIN');
  try {
    if (userId) await client.query("SELECT set_config('app.user_id', $1, true)", [userId]);

    const res = await client.query(
      `WITH prev AS (
         SELECT (prefs -> $1) AS old_val
         FROM owner_notification_targets
         WHERE id = $2 AND location_id = $3
         FOR UPDATE
       )
       UPDATE owner_notification_targets t
          SET prefs = jsonb_set(t.prefs, ARRAY[$1], to_jsonb($4::boolean), true)
         FROM prev
        WHERE t.id = $2 AND t.location_id = $3
        RETURNING prev.old_val AS old_val`,
      [category, targetId, locationId, value],
    );
    if (res.rowCount === 0) {
      await client.query('ROLLBACK');
      return { ok: false, oldValue: null, newValue: value };
    }
    const oldRaw = res.rows[0].old_val;
    const oldValue = oldRaw === null || oldRaw === undefined ? null : oldRaw === true;

    await client.query(
      `INSERT INTO notification_prefs_audit
         (target_id, location_id, user_id, category, old_value, new_value, changed_via)
       VALUES ($1, $2, $3, $4, $5, $6, $7)`,
      [targetId, locationId, userId, category, oldValue, value, changedVia],
    );
    await client.query('COMMIT');
    return { ok: true, oldValue, newValue: value };
  } catch (err) {
    await client.query('ROLLBACK').catch(() => {});
    throw err;
  }
}
