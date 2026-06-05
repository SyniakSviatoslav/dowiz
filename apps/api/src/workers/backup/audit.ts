// @ts-nocheck
import { Pool } from 'pg';

export async function logBackupAudit(
  pool: Pool,
  backupId: string,
  action: 'started' | 'completed' | 'failed' | 'restore_drill_started' | 'restore_drill_completed' | 'key_rotated' | 'retention_violated',
  actorKind: 'system' | 'owner' | 'admin',
  actorId: string | null = null,
  metadata: Record<string, any> = {}
): Promise<void> {
  const client = await pool.connect();
  try {
    await client.query(
      `INSERT INTO backup_audit_log (backup_id, action, actor_kind, actor_id, metadata, created_at)
       VALUES ($1, $2, $3, $4, $5, now())`,
      [backupId, action, actorKind, actorId, JSON.stringify(metadata)]
    );
  } finally {
    client.release();
  }
}

export async function updateBackupMetadata(
  pool: Pool,
  id: string,
  updates: {
    status?: 'in_progress' | 'completed' | 'failed';
    completed_at?: Date;
    size_bytes?: number;
    duration_ms?: number;
    checksum_sha256?: string;
    r2_key?: string;
    error_message?: string;
    row_counts?: Record<string, number>;
  }
): Promise<void> {
  const client = await pool.connect();
  try {
    const setClauses: string[] = [];
    const values: any[] = [];
    let i = 1;

    for (const [key, value] of Object.entries(updates)) {
      if (value !== undefined) {
        setClauses.push(`${key} = $${i}`);
        values.push(key === 'row_counts' ? JSON.stringify(value) : value);
        i++;
      }
    }

    if (setClauses.length === 0) return;

    values.push(id);
    await client.query(
      `UPDATE backup_metadata SET ${setClauses.join(', ')} WHERE id = $${i}`,
      values
    );
  } finally {
    client.release();
  }
}

export async function createBackupMetadata(
  pool: Pool,
  data: {
    id: string;
    type: string;
    triggered_by?: string;
  }
): Promise<void> {
  const client = await pool.connect();
  try {
    await client.query(
      `INSERT INTO backup_metadata (id, type, status, triggered_by, created_at)
       VALUES ($1, $2, 'in_progress', $3, now())`,
      [data.id, data.type, data.triggered_by || 'cron']
    );
  } finally {
    client.release();
  }
}
