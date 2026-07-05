import crypto from 'node:crypto';
import type { Pool } from 'pg';
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
    // LC7 fix 4 — FULL-COVERAGE row counts. The manifest must describe EVERY public base table
    // (not a hand-picked 9), because the restore drill (checkRowCounts) asserts strict per-table
    // parity against it: any table the manifest omits is a table whose truncation/loss the drill
    // can never catch. Enumerate all public BASE TABLEs (excludes views + materialized views) and
    // count each.
    //
    // FOLLOW-ON (snapshot consistency): these are straight COUNT(*)s taken AFTER the pg_dump, so on
    // a busy DB a high-churn table (orders, backup_audit_log) can drift by a few rows between dump
    // and count and make strict parity false-FAIL. That is intentionally the safe direction (a
    // false-FAIL alerts a human; a false-GREEN hides data loss). The real fix is to take the dump
    // and these counts inside one `pg_export_snapshot()` transaction — tracked as a follow-on.
    const tblRes = await client.query(
      `SELECT table_name FROM information_schema.tables
       WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
       ORDER BY table_name`,
    );
    const tables: string[] = tblRes.rows.map((r: { table_name: string }) => r.table_name);

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
