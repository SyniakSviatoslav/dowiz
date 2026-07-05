// @ts-nocheck
import { Pool } from 'pg';
import { loadEnv } from '@deliveryos/config';
import { createQueueWithDefaults } from '@deliveryos/platform';
import { runRestoreVerify } from './backup-verify.js';
import { runR2Verify } from './r2-verify.js';
import { QUEUE_NAMES } from '../../lib/registry.js';

export class BackupVerifyWorker {
  private pool: Pool;

  constructor(
    private operationalPool: Pool,
    private boss: any,
  ) {
    this.pool = operationalPool;
  }

  async start() {
    const env = loadEnv();
    if (env.BACKUP_ENABLED !== 'true') {
      console.log('[BackupVerify] BACKUP_ENABLED not true, skipping');
      return;
    }

    // H1/H2 (2026-07-03 reliability audit): bare createQueue() left these on pg-boss
    // v10 defaults — policy `standard` (which silently no-ops the singletonKey-only
    // dedup already used by the schedule() calls below) and no retry/DLQ config.
    const queueOptions = { policy: 'short' as const, deadLetter: true as const };

    // Restore-test worker (daily 04:00 UTC)
    await this.boss.work(QUEUE_NAMES.BACKUP_VERIFY_RESTORE, { singletonKey: QUEUE_NAMES.BACKUP_VERIFY_RESTORE }, async () => {
      const env = loadEnv();
      await runRestoreVerify(this.pool, { fullHash: env.RESTORE_VERIFY_FULL_HASH === 'true' });
    });
    await createQueueWithDefaults(this.boss, QUEUE_NAMES.BACKUP_VERIFY_RESTORE, queueOptions);
    await this.boss.schedule(QUEUE_NAMES.BACKUP_VERIFY_RESTORE, env.RESTORE_VERIFY_CRON || '0 4 * * *', null, {
      singletonKey: QUEUE_NAMES.BACKUP_VERIFY_RESTORE,
    });

    // R2 sample verify (separate, lighter — every 6h)
    await this.boss.work(QUEUE_NAMES.BACKUP_VERIFY_R2, { singletonKey: QUEUE_NAMES.BACKUP_VERIFY_R2 }, async () => {
      await runR2Verify(this.pool);
    });
    await createQueueWithDefaults(this.boss, QUEUE_NAMES.BACKUP_VERIFY_R2, queueOptions);
    await this.boss.schedule(QUEUE_NAMES.BACKUP_VERIFY_R2, '0 */6 * * *', null, {
      singletonKey: QUEUE_NAMES.BACKUP_VERIFY_R2,
    });

    console.log('[BackupVerify] Restore-test + R2 verify workers registered');
  }
}
