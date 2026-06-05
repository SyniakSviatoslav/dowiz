/**
 * Backup Verification CLI
 *
 * Usage:
 *   pnpm backup:verify                              # Verify latest daily backup
 *   pnpm backup:verify --backup-id=<id>              # Verify specific backup
 *   pnpm backup:verify --full-hash                   # With full data integrity hash check
 */

import { loadEnv } from '@deliveryos/config';
import { createOperationalPool } from '@deliveryos/db';
import { runRestoreVerify } from '../apps/api/src/workers/backup/backup-verify.js';

async function main() {
  const env = loadEnv();
  const args = process.argv.slice(2);

  const backupIdIdx = args.findIndex(a => a.startsWith('--backup-id='));
  const fullHashIdx = args.findIndex(a => a === '--full-hash');

  const backupId = backupIdIdx >= 0 ? args[backupIdIdx].split('=')[1] : undefined;
  const fullHash = fullHashIdx >= 0;

  console.log(`\n=== Backup Verify ===`);
  console.log(`Backup ID: ${backupId || 'latest daily'}`);
  console.log(`Full hash: ${fullHash}`);
  console.log('');

  const pool = createOperationalPool();
  try {
    const result = await runRestoreVerify(pool, { backupId, fullHash });
    console.log(JSON.stringify(result, null, 2));
    if (!result.success) {
      console.error('\n❌ Verify FAILED');
      process.exit(1);
    }
    console.log('\n✅ Verify PASSED');
  } finally {
    await pool.end();
  }
}

main().catch(err => {
  console.error('Fatal:', err.message);
  process.exit(1);
});
