import type { Pool } from 'pg';
import { flagTerminal } from './service.js';
import { hardDeleteShadow, reapExpiredGrants } from './provisioning.js';
import { reapExpiredInvites } from './claim.js';

// P6 retention sweep (council H-abandoned-TTL / C5 + breaker HIGH — GDPR Art-5(e) storage-limitation).
// A PUBLIC shadow that is never claimed is unconsented third-party data + the aggregator-squatting posture;
// it must self-erase on a SHORT TTL. This sweep also reaps expired one-time grants + claim invites. Runs on
// the operational pool (the hard-deletes need the same authority as every ops write today).

const DEFAULT_ABANDONED_TTL_DAYS = 30;

/**
 * Hard-delete + ABANDON shadows that were provisioned/verified/offered but NEVER claimed within the TTL.
 * Only touches sources that still have a spine (org_id set) and are pre-CLAIMED — a CLAIMED source is a real,
 * consented tenant and is never reaped. Returns the count reaped.
 */
export async function reapAbandonedShadows(pool: Pool, ttlDays = DEFAULT_ABANDONED_TTL_DAYS): Promise<number> {
  const res = await pool.query(
    `SELECT id FROM acquisition_sources
      WHERE state IN ('PROVISIONED', 'VERIFIED', 'CLAIM_OFFERED')
        AND org_id IS NOT NULL
        AND created_at < now() - ($1 * interval '1 day')`,
    [ttlDays],
  );
  let reaped = 0;
  for (const row of res.rows as { id: string }[]) {
    // hardDeleteShadow erases the spine + place_raw + menu_draft + grants; then mark the lifecycle terminal.
    await hardDeleteShadow(pool, row.id);
    await flagTerminal(pool, row.id, 'ABANDONED', `retention TTL: unclaimed public shadow expired after ${ttlDays}d`);
    reaped++;
  }
  return reaped;
}

/** One sweep over all three retention surfaces. Wire to a recurring trigger (cron → the ops route). */
export async function runRetentionSweep(
  pool: Pool,
  opts: { abandonedTtlDays?: number } = {},
): Promise<{ grants: number; invites: number; shadows: number }> {
  const grants = await reapExpiredGrants(pool);
  const invites = await reapExpiredInvites(pool);
  const shadows = await reapAbandonedShadows(pool, opts.abandonedTtlDays);
  return { grants, invites, shadows };
}
