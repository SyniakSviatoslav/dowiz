// @ts-nocheck
import { Pool } from 'pg';
import { loadEnv } from '@deliveryos/config';
import { runRestoreVerify } from './backup-verify.js';
import { runR2Verify } from './r2-verify.js';

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

    // Restore-test worker (daily 04:00 UTC)
    await this.boss.work('backup.verify.restore', { singletonKey: 'backup.verify.restore' }, async () => {
      const env = loadEnv();
      await runRestoreVerify(this.pool, { fullHash: env.RESTORE_VERIFY_FULL_HASH === 'true' });
    });
    await this.boss.createQueue('backup.verify.restore');
    await this.boss.schedule('backup.verify.restore', env.RESTORE_VERIFY_CRON || '0 4 * * *', null, {
      singletonKey: 'backup.verify.restore',
    });

    // R2 sample verify (separate, lighter — every 6h)
    await this.boss.work('backup.verify.r2', { singletonKey: 'backup.verify.r2' }, async () => {
      await runR2Verify(this.pool);
    });
    await this.boss.createQueue('backup.verify.r2');
    await this.boss.schedule('backup.verify.r2', '0 */6 * * *', null, {
      singletonKey: 'backup.verify.r2',
    });

    console.log('[BackupVerify] Restore-test + R2 verify workers registered');
  }
}
