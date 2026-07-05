import { Pool, type PoolClient } from 'pg';
import { GetObjectCommand } from '@aws-sdk/client-s3';
import { createReadStream, createWriteStream } from 'node:fs';
import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import { getS3Client } from './upload.js';
import type { R2Config } from './upload.js';
import { createDecryptionStream, resolveBackupKey } from './encrypt.js';
import type { BackupManifest } from './manifest.js';
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

// LC7 fix 5 — the DB (backup_metadata) only tells us WHICH backup and WHERE its artifact lives.
// All integrity material (iv/authTag/keyId/checksum/rowCounts) is read from the R2 *manifest*
// (loadManifest): backup_metadata has no `metadata` column (the old metadata->'encryption'->>'iv'
// SELECT threw at runtime), and in a real disaster the DB is exactly the thing we've lost.
interface BackupLocator {
  id: string;
  type: string;
  r2Key: string;
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

// ADR-admin-platform-authz F2 / RA2-2 — ONE lock, ONE owner, held on a DEDICATED client.
// The prior code did pg_try_advisory_lock then released the client to the pool in `finally` — the
// SESSION-level lock stayed held on a pooled connection (leak forever → every later drill 409s), and
// releaseLock connected a DIFFERENT session whose pg_advisory_unlock was a no-op. Fix: return the
// locked client, hold it across the whole drill, unlock-then-release on the SAME session. Crash-safe:
// if the process dies the backend session ends and PG drops the session lock.
async function acquireLock(pool: Pool, lockId: number): Promise<PoolClient | null> {
  const client = await pool.connect();
  try {
    const res = await client.query('SELECT pg_try_advisory_lock($1) AS locked', [lockId]);
    if (!res.rows[0].locked) { client.release(); return null; }
    return client; // HOLD this client for the drill's lifetime — do NOT release here
  } catch (err) {
    client.release();
    throw err;
  }
}

async function releaseLock(client: PoolClient | null, lockId: number): Promise<void> {
  if (!client) return;
  try {
    await client.query('SELECT pg_advisory_unlock($1)', [lockId]);
  } finally {
    client.release(); // unlock BEFORE release, on the SAME session that holds the lock
  }
}

async function selectBackup(pool: Pool, backupId?: string): Promise<BackupLocator> {
  if (backupId) {
    const res = await pool.query(
      `SELECT id, type, r2_key FROM backup_metadata WHERE id = $1 AND status = 'completed'`,
      [backupId],
    );
    if (res.rows.length === 0) throw new Error(`Backup ${backupId} not found or not completed`);
    const row = res.rows[0];
    return { id: row.id, type: row.type, r2Key: row.r2_key };
  }

  const res = await pool.query(
    `SELECT id, type, r2_key FROM backup_metadata
     WHERE type = 'daily' AND status = 'completed'
     ORDER BY created_at DESC LIMIT 1`,
  );
  if (res.rows.length === 0) throw new Error('No completed daily backup found');
  const row = res.rows[0];
  return { id: row.id, type: row.type, r2Key: row.r2_key };
}

// LC7 fix 5 — read the integrity material from the R2 manifest JSON (iv/authTag/keyId/checksum/
// rowCounts), NOT the DB. The manifest key is derived from the artifact key.
async function loadManifest(r2Key: string): Promise<BackupManifest> {
  const config = getR2Config();
  const s3 = getS3Client(config);
  const manifestKey = r2Key.replace('.enc.parts', '.manifest.json');
  const response = await s3.send(new GetObjectCommand({ Bucket: config.bucket, Key: manifestKey }));
  const body = await response.Body?.transformToString();
  if (!body) throw new Error(`Empty or missing manifest at ${manifestKey}`);
  return JSON.parse(body) as BackupManifest;
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
  key: string,
): Promise<void> {
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

export async function sha256File(filePath: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = createReadStream(filePath);
    stream.on('error', reject);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

// LC7 fix 2 — the smoke pool MUST target the freshly-restored SCRATCH database, never live prod.
// The prior code called createSessionPool(sandboxUrl), but that factory takes NO argument and
// hardwires env.DATABASE_URL_SESSION, so sandboxUrl was silently discarded and EVERY smoke check
// ran against PROD. We build the scratch pool here from the passed connection string directly.
// NOTE: the spec's preferred fix — an optional connectionString param on createSessionPool
// (packages/db/src/index.ts) — is blocked by the protect-paths governance guardrail on that
// package and needs the lead's manual approval. This app-code pool is the equivalent, safe now.
export function createScratchPool(connectionString: string): Pool {
  const pool = new Pool({
    connectionString,
    max: 3,
    idleTimeoutMillis: 30000,
    connectionTimeoutMillis: 5000,
    ssl: /[?&]sslmode=disable/.test(connectionString) ? false : { rejectUnauthorized: false },
  });
  pool.on('connect', (client) => {
    void client.query("SET statement_timeout = '30s'");
  });
  return pool;
}

// LC7 fix 1 — the writer stores sha256 of the PLAINTEXT dump (index.ts calculateFileChecksum(
// dump.tempFile), pre-encryption). The drill MUST therefore hash the DECRYPTED (plaintext) file;
// hashing the ciphertext (the old sha256File(encryptedPath)) can never match manifest.checksumSha256.
export async function computeArtifactChecksum(paths: {
  encryptedPath: string;
  decryptedPath: string;
}): Promise<string> {
  return sha256File(paths.decryptedPath);
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
  let lockClient: PoolClient | null = null;

  try {
    // ── 1. Acquire singleton lock (held on a dedicated client for the whole drill — F2) ──
    result.stage = 'lock';
    lockClient = await acquireLock(pool, BACKUP_VERIFY_LOCK);
    if (!lockClient) {
      result.error = 'Another verify in progress';
      result.durationMs = Date.now() - startTime;
      return result;
    }

    // ── 2. Select backup + load the R2 manifest (source of truth for integrity material) ──
    result.stage = 'select';
    const backup = await selectBackup(pool, opts.backupId);
    result.backupId = backup.id;
    await writeAudit(pool, 'restore_drill_started', { backupId: backup.id, stage: 'select' });

    const manifest = await loadManifest(backup.r2Key);
    // Fail loud on an unknown keyId BEFORE downloading anything (LC7 fix 7).
    const encKey = resolveBackupKey(manifest.encryption.keyId);

    const tempDir = path.join(process.cwd(), '.tmp', `restore-verify-${backup.id}`);
    await fs.mkdir(tempDir, { recursive: true, mode: 0o600 });
    const encryptedPath = path.join(tempDir, `${backup.id}.enc`);
    const decryptedPath = path.join(tempDir, `${backup.id}.dump`);

    try {
      // ── 3. Download from R2 ──
      result.stage = 'download';
      await downloadFromR2(backup.r2Key, encryptedPath);

      // ── 4. Decrypt (iv/authTag/keyId all from the R2 manifest, not the DB) ──
      result.stage = 'decrypt';
      if (!manifest.encryption.authTag) {
        throw new Error('Manifest is missing encryption.authTag — cannot decrypt (corrupt manifest)');
      }
      await decryptBackup(
        encryptedPath,
        decryptedPath,
        manifest.encryption.iv,
        manifest.encryption.authTag,
        encKey,
      );

      // ── 5. Checksum — hash the DECRYPTED (plaintext) dump vs manifest.checksumSha256 (LC7 fix 1) ──
      result.stage = 'checksum';
      const actualChecksum = await computeArtifactChecksum({ encryptedPath, decryptedPath });
      if (actualChecksum !== manifest.checksumSha256) {
        throw new Error(`Checksum mismatch: expected ${manifest.checksumSha256}, got ${actualChecksum}`);
      }

      // ── 6. Restore to sandbox ──
      result.stage = 'restore';
      const sandboxUrl = await createSandboxDatabase();
      let sandboxPool: Pool | null = null;

      try {
        await pgRestore(decryptedPath, sandboxUrl);

        // ── 7. Smoke-checks ──
        result.stage = 'smoke_check';
        sandboxPool = createScratchPool(sandboxUrl);
        result.smokeChecks = await runSmokeChecks(sandboxPool, {
          fullHash: opts.fullHash,
          manifestRowCounts: manifest.rowCounts,
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
    await releaseLock(lockClient, BACKUP_VERIFY_LOCK);
  }

  return result;
}
