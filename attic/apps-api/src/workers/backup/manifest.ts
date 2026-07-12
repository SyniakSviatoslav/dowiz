// @ts-nocheck
import crypto from 'node:crypto';
import { Pool } from 'pg';
import { createReadStream } from 'node:fs';

export interface BackupManifest {
  backupId: string;
  type: string;
  createdAt: string;
  durationMs: number;
  sizeBytes: number;
  checksumSha256: string;
  r2Key: string;
  tablesIncluded: string[];
  rowCounts: Record<string, number>;
  pgVersion: string;
  appVersion: string;
  encryption: {
    algorithm: string;
    keyId: string;
    iv: string;
    authTag: string;
  };
  piiRedacted: boolean;
  piiEncrypted: boolean;
}

export async function generateManifest(
  pool: Pool,
  partial: Omit<BackupManifest, 'rowCounts' | 'tablesIncluded' | 'pgVersion' | 'piiRedacted' | 'piiEncrypted' | 'appVersion'>
): Promise<BackupManifest> {
  const client = await pool.connect();
  try {
    // Get row counts for critical tables (approximate is fine for massive DBs, but exact is better for audits)
    // We will do a fast estimation via pg_class to avoid locking or slow scans, or exact count for small DBs.
    // For PII-free backup auditing, exact count is usually fine during a consistent snapshot.
    // Let's use exact counts for the specific tables we care about in this phase.
    
    const tables = [
      'orders', 'order_items', 'order_item_modifiers', 
      'courier_assignments', 'settlement_items', 'courier_payouts', 
      'backup_audit_log', 'locations', 'customers'
    ];

    const rowCounts: Record<string, number> = {};
    for (const t of tables) {
      const res = await client.query(`SELECT COUNT(*) as c FROM "${t}"`);
      rowCounts[t] = parseInt(res.rows[0].c, 10);
    }

    const pgVer = await client.query('SHOW server_version;');

    return {
      ...partial,
      tablesIncluded: tables,
      rowCounts,
      pgVersion: pgVer.rows[0].server_version,
      appVersion: process.env.RENDER_GIT_COMMIT || 'unknown',
      piiRedacted: false, // We use encryption at rest, not pure redaction
      piiEncrypted: true
    };
  } finally {
    client.release();
  }
}

export async function calculateFileChecksum(filePath: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = createReadStream(filePath);
    stream.on('error', err => reject(err));
    stream.on('data', chunk => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}
