import type { Pool, PoolClient } from 'pg';

export type AuditStatus =
  | 'no_target'
  | 'unknown_event'
  | 'quiet_hours'
  | 'dedup'
  | 'target_inactive'
  | 'prefs_disabled'
  | 'order_not_found'
  | 'circuit_open'
  | 'rate_limited'
  | 'sending'
  | 'delivered'
  | 'failed';

export type AuditEntry = {
  event: string;
  targetId?: string;
  locationId: string;
  channel: string;
  status: AuditStatus;
  attempts?: number;
  errorMessage?: string;
};

export async function writeAudit(
  client: PoolClient | Pool,
  entry: AuditEntry,
): Promise<void> {
  await (client as Pool).query(
    `INSERT INTO notification_outbox_audit
     (event, target_id, location_id, channel, status, attempts, error_message)
     VALUES ($1, $2, $3, $4, $5, $6, $7)`,
    [
      entry.event,
      entry.targetId || null,
      entry.locationId,
      entry.channel,
      entry.status,
      entry.attempts ?? 1,
      entry.errorMessage || null,
    ],
  );
}
