import { Pool } from 'pg';
import { loadEnv } from '@deliveryos/config';
import crypto from 'node:crypto';
import { createLogicalDump } from './dump.js';
import { createEncryptionStream } from './encrypt.js';
import { uploadStream, uploadJson } from './upload.js';
import { generateManifest, calculateFileChecksum } from './manifest.js';
import { logBackupAudit, createBackupMetadata, updateBackupMetadata } from './audit.js';

export class BackupCronWorker {
  constructor(
    private operationalPool: Pool,
    private backupPool: Pool,
    private boss: any,
    private messageBus: any,
    private deps: {
      createLogicalDump: typeof createLogicalDump;
      uploadStream: typeof uploadStream;
      uploadJson: typeof uploadJson;
    } = { createLogicalDump, uploadStream, uploadJson }
  ) {}

  async start() {
    const env = loadEnv();
    if (env.BACKUP_ENABLED !== 'true') {
      console.log('[Backup] BACKUP_ENABLED is not true, skipping worker registration.');
      return;
    }

    console.log('[Backup] Registering BackupCronWorker jobs...');

    await this.boss.work('backup.hourly', async () => this.handleBackup('hourly'));
    await this.boss.work('backup.daily', async () => this.handleBackup('daily'));
    await this.boss.work('backup.weekly', async () => this.handleBackup('weekly'));
    await this.boss.work('backup.monthly', async () => this.handleBackup('monthly'));

    await this.boss.createQueue('backup.hourly');
    await this.boss.schedule('backup.hourly', env.BACKUP_HOURLY_CRON);
    await this.boss.createQueue('backup.daily');
    await this.boss.schedule('backup.daily', env.BACKUP_DAILY_CRON);
    await this.boss.createQueue('backup.weekly');
    await this.boss.schedule('backup.weekly', env.BACKUP_WEEKLY_CRON);
    await this.boss.createQueue('backup.monthly');
    await this.boss.schedule('backup.monthly', env.BACKUP_MONTHLY_CRON);
  }

  private async acquireLock(client: any, type: string): Promise<boolean> {
    const lockKey = this.getLockKey(type);
    const res = await client.query(`SELECT pg_try_advisory_lock($1) as locked`, [lockKey]);
    return res.rows[0].locked;
  }

  private async releaseLock(client: any, type: string): Promise<void> {
    const lockKey = this.getLockKey(type);
    await client.query(`SELECT pg_advisory_unlock($1)`, [lockKey]);
  }

  private async sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  private getLockKey(type: string): number {
    // Use different lock keys per backup type: hash the type string to a bigint
    const hash = crypto.createHash('sha256').update(`backup_lock_${type}`).digest();
    return hash.readUInt32BE(0);
  }

  async handleBackup(type: 'hourly' | 'daily' | 'weekly' | 'monthly') {
    const env = loadEnv();
    
    const lockClient = await this.operationalPool.connect();
    let locked = false;
    const backupId = crypto.randomUUID();
    const startTime = Date.now();
    const maxRetries = 3;
    const retryDelays = [60_000, 300_000, 900_000]; // 1min, 5min, 15min

    try {
      locked = await this.acquireLock(lockClient, type);
      if (!locked) {
        console.log(`[Backup] Worker on another instance is handling ${type} backup. Skipping.`);
        return;
      }

      console.log(`[Backup] Starting ${type} backup ${backupId}...`);
      await createBackupMetadata(this.operationalPool, { id: backupId, type });
      await logBackupAudit(this.operationalPool, backupId, 'started', 'system');

      let lastError: Error | null = null;
      for (let attempt = 1; attempt <= maxRetries; attempt++) {
        try {
          // 1. Create Dump Stream
          const dump = await this.deps.createLogicalDump(env.***REDACTED***, backupId);

          // 2. Encryption wrapper
          if (!env.BACKUP_ENCRYPTION_KEY) {
            throw new Error('BACKUP_ENCRYPTION_KEY is missing');
          }
          const { stream: cipherStream, meta: encMeta, getAuthTag } = createEncryptionStream(env.BACKUP_ENCRYPTION_KEY);
          
          const encryptedStream = dump.stream.pipe(cipherStream);

          // 3. Upload encrypted stream to R2
          const dateStr = new Date().toISOString().split('T')[0];
          const r2Key = `dowiz-backups/${env.NODE_ENV}/${type}/${dateStr}/${backupId}.enc.parts`;

          console.log(`[Backup] Uploading to R2: ${r2Key}...`);
          await this.deps.uploadStream(
            {
              accountId: '',
              endpoint: env.R2_ENDPOINT || '',
              accessKeyId: env.R2_ACCESS_KEY_ID || '',
              secretAccessKey: env.R2_SECRET_ACCESS_KEY || '',
              bucket: env.R2_BUCKET || ''
            },
            r2Key,
            encryptedStream
          );

          const checksumSha256 = await calculateFileChecksum(dump.tempFile);
          encMeta.authTag = getAuthTag();
          await dump.cleanup();

          // 4. Generate & Upload Manifest
          const manifest = await generateManifest(this.backupPool, {
            backupId,
            type,
            createdAt: new Date().toISOString(),
            durationMs: Date.now() - startTime,
            sizeBytes: 0,
            checksumSha256,
            r2Key,
            encryption: encMeta,
          });

          const manifestKey = `dowiz-backups/${env.NODE_ENV}/${type}/${dateStr}/${backupId}.manifest.json`;
          await this.deps.uploadJson(
            {
              accountId: '',
              endpoint: env.R2_ENDPOINT || '',
              accessKeyId: env.R2_ACCESS_KEY_ID || '',
              secretAccessKey: env.R2_SECRET_ACCESS_KEY || '',
              bucket: env.R2_BUCKET || ''
            },
            manifestKey,
            manifest
          );

          // 5. Audit & Complete
          const durationMs = Date.now() - startTime;
          await updateBackupMetadata(this.operationalPool, backupId, {
            status: 'completed',
            completed_at: new Date(),
            checksum_sha256: checksumSha256,
            r2_key: r2Key,
            duration_ms: durationMs,
            row_counts: manifest.rowCounts,
          });

          await logBackupAudit(this.operationalPool, backupId, 'completed', 'system', null, { durationMs });

          await this.messageBus.publish('backup.completed', {
            backupId,
            type,
            durationMs,
            r2Key
          });

          console.log(`[Backup] ${type} backup ${backupId} completed in ${durationMs}ms (attempt ${attempt})`);
          lastError = null;
          break; // Success — exit retry loop
        } catch (err: any) {
          lastError = err;
          console.error(`[Backup] Attempt ${attempt}/${maxRetries} failed for ${type} backup ${backupId}:`, err.message);
          
          if (attempt < maxRetries) {
            const delay = retryDelays[attempt - 1];
            console.log(`[Backup] Retrying in ${delay / 1000}s...`);
            await updateBackupMetadata(this.operationalPool, backupId, {
              error_message: `attempt ${attempt}: ${err.message}`
            });
            await logBackupAudit(this.operationalPool, backupId, 'failed', 'system', null, { attempt, error: err.message });
            await this.sleep(delay);
          }
        }
      }

      if (lastError) {
        throw lastError;
      }
    } catch (err: any) {
      console.error(`[Backup] Failed to run ${type} backup ${backupId}:`, err);
      
      await updateBackupMetadata(this.operationalPool, backupId, {
        status: 'failed',
        error_message: err.message
      });
      await logBackupAudit(this.operationalPool, backupId, 'failed', 'system', null, { error: err.message });

      await this.messageBus.publish('backup.failed', {
        backupId,
        type,
        reason: err.message
      });
    } finally {
      if (locked) {
        await this.releaseLock(lockClient);
      }
      lockClient.release();
    }
  }
}
