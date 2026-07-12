// @ts-nocheck
import { Pool } from 'pg';
import { loadEnv } from '@deliveryos/config';
import { S3Client, GetObjectCommand } from '@aws-sdk/client-s3';
import { createReadStream, createWriteStream } from 'node:fs';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import { getS3Client } from './upload.js';
import type { R2Config } from './upload.js';
import { createDecryptionStream } from './encrypt.js';
import { createSessionPool } from '@deliveryos/db';
import { runSmokeChecks } from './smoke-checks.js';
import type { SmokeCheck } from './smoke-checks.js';
import { createSandboxDatabase, dropSandboxDatabase } from '../../lib/restore-sandbox.js';
import { PiiRedactor } from '../../lib/pii-redactor.js';

const BACKUP_VERIFY_LOCK = 3;
const TIMEOUT_MS = 30 * 60 * 1000; // 30 min

const piiRedactor = new PiiRedactor();

export interface VerifyResult {
  success: boolean;
  backupId?: string;
  stage: string;
  durationMs: number;
  smokeChecks: SmokeCheck[];
  error?: string;
}

interface BackupRecord {
  id: string;
  type: string;
  r2_key: string;
  checksum_sha256: string;
  encryption_iv: string;
  encryption_auth_tag: string;
  encryption_algorithm: string;
  row_counts: Record<string, number>;
}

function getR2Config(): R2Config {
  return {
    accountId: '',
    endpoint: process.env.R2_ENDPOINT || '',
    accessKeyId: process.env.R2_ACCESS_KEY_ID || '',
    secretAccessKey: process.env.R2_SECRET_ACCESS_KEY || '',
    bucket: process.env.R2_BUCKET || '',
  };
}

function redactPII(msg: string): string {
  return piiRedactor.redact(msg).text;
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function acquireLock(pool: Pool, lockId: number): Promise<boolean> {
  const client = await pool.connect();
  try {
    const res = await client.query('SELECT pg_try_advisory_lock($1) AS locked', [lockId]);
    return res.rows[0].locked;
  } finally {
    client.release();
  }
}

async function releaseLock(pool: Pool, lockId: number): Promise<void> {
  const client = await pool.connect();
  try {
    await client.query('SELECT pg_advisory_unlock($1)', [lockId]);
  } finally {
    client.release();
  }
}

async function selectBackup(pool: Pool, backupId?: string): Promise<BackupRecord> {
  if (backupId) {
    const res = await pool.query(
      `SELECT id, type, r2_key, checksum_sha256,
              metadata->'encryption'->>'iv' AS encryption_iv,
              metadata->'encryption'->>'auth_tag' AS encryption_auth_tag,
              metadata->'encryption'->>'algorithm' AS encryption_algorithm,
              COALESCE(row_counts, '{}'::jsonb) AS row_counts
       FROM backup_metadata
       WHERE id = $1 AND status = 'completed'`,
      [backupId],
    );
    if (res.rows.length === 0) throw new Error(`Backup ${backupId} not found or not completed`);
    return res.rows[0];
  }

  const res = await pool.query(
    `SELECT id, type, r2_key, checksum_sha256,
            metadata->'encryption'->>'iv' AS encryption_iv,
            metadata->'encryption'->>'auth_tag' AS encryption_auth_tag,
            metadata->'encryption'->>'algorithm' AS encryption_algorithm,
            COALESCE(row_counts, '{}'::jsonb) AS row_counts
     FROM backup_metadata
     WHERE type = 'daily' AND status = 'completed'
     ORDER BY created_at DESC LIMIT 1`,
  );
  if (res.rows.length === 0) throw new Error('No completed daily backup found');
  return res.rows[0];
}

async function downloadFromR2(r2Key: string, destPath: string): Promise<void> {
  const config = getR2Config();
  const s3 = getS3Client(config);
  const command = new GetObjectCommand({ Bucket: config.bucket, Key: r2Key });
  const response = await s3.send(command);
  const writeStream = createWriteStream(destPath);
  await new Promise<void>((resolve, reject) => {
    (response.Body as any).pipe(writeStream);
    writeStream.on('finish', resolve);
    writeStream.on('error', reject);
  });
}

async function decryptBackup(
  encryptedPath: string,
  decryptedPath: string,
  iv: string,
  authTag: string,
): Promise<void> {
  const key = process.env.BACKUP_ENCRYPTION_KEY;
  if (!key) throw new Error('BACKUP_ENCRYPTION_KEY is required');

  const decipher = createDecryptionStream(key, iv, authTag);
  const readStream = createReadStream(encryptedPath);
  const writeStream = createWriteStream(decryptedPath);

  await new Promise<void>((resolve, reject) => {
    readStream.pipe(decipher).pipe(writeStream);
    writeStream.on('finish', resolve);
    writeStream.on('error', reject);
    decipher.on('error', reject);
  });
}

async function sha256File(filePath: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = createReadStream(filePath);
    stream.on('error', reject);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

async function pgRestore(decryptedPath: string, sandboxUrl: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn('pg_restore', [
      '--dbname=' + sandboxUrl,
      '--clean',
      '--if-exists',
      '--no-owner',
      '--no-acl',
      '--jobs=2',
      decryptedPath,
    ], { stdio: ['ignore', 'pipe', 'pipe'] });

    let stderr = '';
    child.stderr.on('data', (d: Buffer) => { stderr += d.toString(); });

    child.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`pg_restore failed (exit ${code}): ${stderr.substring(0, 500)}`));
    });
    child.on('error', reject);

    setTimeout(() => reject(new Error('pg_restore timed out after 25 min')), 25 * 60 * 1000);
  });
}

async function cleanupTempDir(dir: string): Promise<void> {
  try {
    const files = await fs.readdir(dir);
    for (const f of files) {
      const fp = path.join(dir, f);
      await fs.unlink(fp).catch(() => {});
    }
    await fs.rmdir(dir).catch(() => {});
    } catch (err: any) {
      console.debug('[backup-verify] temp dir not found during cleanup:', err?.message);
    }
}

async function writeAudit(
  pool: Pool,
  kind: string,
  data: { backupId?: string; stage?: string; durationMs?: number; error?: string },
): Promise<void> {
  try {
    const { logBackupAudit } = await import('./audit.js');
    await logBackupAudit(pool, data.backupId || 'unknown', kind as any, 'system', null, {
      stage: data.stage,
      durationMs: data.durationMs,
      error: data.error ? redactPII(data.error) : undefined,
    });
  } catch (err: any) {
    console.debug('[backup-verify] audit log write failed:', err?.message);
  }
}

async function alertFailure(result: VerifyResult): Promise<void> {
  try {
    const { loadEnv } = await import('@deliveryos/config');
    const { Pool } = await import('pg');

    // Sentry via import
    try {
      const sentryMod = await import('../../lib/sentry.js');
      const sentry = sentryMod.getSentry();
      if (sentry) {
        sentry.captureException(new Error(`backup.verify.failed: ${result.stage}`), {
          tags: {
            'backup.id': result.backupId || 'unknown',
            'backup.type': 'daily',
            'verify.stage': result.stage,
            'verify.error_kind': result.error?.substring(0, 100) || 'unknown',
          },
          extra: { durationMs: result.durationMs, smokeChecks: result.smokeChecks.length },
        });
      }
    } catch (err: any) { console.debug('[backup-verify] sentry capture failed:', err?.message); }

    // MessageBus event for Telegram
    try {
      const { RedisMessageBus } = await import('@deliveryos/platform');
      const bus = new RedisMessageBus();
      await bus.connect();
      await bus.publish('backup.verify.failed', {
        backupId: result.backupId,
        stage: result.stage,
        error: redactPII(result.error || 'Unknown error'),
        durationMs: result.durationMs,
        timestamp: new Date().toISOString(),
      });
      await bus.close();
    } catch (err: any) { console.debug('[backup-verify] messageBus publish failed:', err?.message); }
  } catch (err: any) { console.debug('[backup-verify] alert dispatch failed:', err?.message); }
}

export async function runRestoreVerify(
  pool: Pool,
  opts: { backupId?: string; fullHash?: boolean } = {},
): Promise<VerifyResult> {
  const startTime = Date.now();
  const result: VerifyResult = { success: false, stage: 'init', durationMs: 0, smokeChecks: [] };

  try {
    // ── 1. Acquire singleton lock ──
    result.stage = 'lock';
    const locked = await acquireLock(pool, BACKUP_VERIFY_LOCK);
    if (!locked) {
      result.error = 'Another verify in progress';
      result.durationMs = Date.now() - startTime;
      return result;
    }

    // ── 2. Select backup ──
    result.stage = 'select';
    const backup = await selectBackup(pool, opts.backupId);
    result.backupId = backup.id;
    await writeAudit(pool, 'restore_drill_started', { backupId: backup.id, stage: 'select' });

    const tempDir = path.join(process.cwd(), '.tmp', `restore-verify-${backup.id}`);
    await fs.mkdir(tempDir, { recursive: true, mode: 0o600 });
    const encryptedPath = path.join(tempDir, `${backup.id}.enc`);
    const decryptedPath = path.join(tempDir, `${backup.id}.dump`);

    try {
      // ── 3. Download from R2 ──
      result.stage = 'download';
      await downloadFromR2(backup.r2_key, encryptedPath);

      // ── 4. Decrypt ──
      result.stage = 'decrypt';
      await decryptBackup(
        encryptedPath,
        decryptedPath,
        backup.encryption_iv,
        backup.encryption_auth_tag,
      );

      // ── 5. Checksum ──
      result.stage = 'checksum';
      const actualChecksum = await sha256File(encryptedPath);
      if (actualChecksum !== backup.checksum_sha256) {
        throw new Error(`Checksum mismatch: expected ${backup.checksum_sha256}, got ${actualChecksum}`);
      }

      // ── 6. Restore to sandbox ──
      result.stage = 'restore';
      const sandboxUrl = await createSandboxDatabase();
      let sandboxPool: Pool | null = null;

      try {
        await pgRestore(decryptedPath, sandboxUrl);

        // ── 7. Smoke-checks ──
        result.stage = 'smoke_check';
        sandboxPool = createSessionPool(sandboxUrl);
        result.smokeChecks = await runSmokeChecks(sandboxPool, {
          fullHash: opts.fullHash,
          baselineRowCounts: backup.row_counts,
        });

        const allPassed = result.smokeChecks.every(c => c.passed);
        if (!allPassed) {
          const failedNames = result.smokeChecks.filter(c => !c.passed).map(c => c.name);
          throw new Error(`Smoke checks failed: ${failedNames.join(', ')}`);
        }

        result.stage = 'cleanup';
        if (sandboxPool) await sandboxPool.end();
        await dropSandboxDatabase(sandboxUrl);
        await cleanupTempDir(tempDir);

        result.success = true;
        result.durationMs = Date.now() - startTime;
        await writeAudit(pool, 'restore_drill_completed', {
          backupId: backup.id,
          stage: 'completed',
          durationMs: result.durationMs,
        });

      } catch (err: any) {
        // Sandbox cleanup on failure
        try {
          if (sandboxPool) await sandboxPool.end();
          await dropSandboxDatabase(sandboxUrl);
        } catch (err: any) {
          console.debug('[backup-verify] sandbox cleanup failed during error recovery:', err?.message);
        }
        throw err;
      }
    } catch (err: any) {
      await cleanupTempDir(tempDir);
      throw err;
    }

  } catch (err: any) {
    result.error = redactPII(err.message);
    result.durationMs = Date.now() - startTime;
    await writeAudit(pool, 'restore_drill_completed', {
      backupId: result.backupId,
      stage: result.stage,
      durationMs: result.durationMs,
      error: result.error,
    });
    await alertFailure(result);

  } finally {
    await releaseLock(pool, BACKUP_VERIFY_LOCK);
  }

  return result;
}
