/**
 * List recent backups
 *
 * Usage:
 *   pnpm backup:list                          # Last 20
 *   pnpm backup:list --since=1d|7d|30d
 */

import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';

async function main() {
  const env = loadEnv();
  const args = process.argv.slice(2);

  const sinceIdx = args.findIndex(a => a.startsWith('--since='));
  let interval = '30 days';

  if (sinceIdx >= 0) {
    const val = args[sinceIdx].split('=')[1];
    const match = val.match(/^(\d+)([dh])$/);
    if (match) {
      const num = match[1];
      const unit = match[2] === 'd' ? 'days' : 'hours';
      interval = `${num} ${unit}`;
    } else {
      interval = '30 days';
    }
  }

  const pool = createSessionPool();
  try {
    const res = await pool.query(
      `SELECT id, type, status, created_at,
              size_bytes, duration_ms,
              checksum_sha256 IS NOT NULL AS has_checksum,
              row_counts
       FROM backup_metadata
       WHERE created_at >= now() - interval '${interval}'
       ORDER BY created_at DESC
       LIMIT 50`,
    );

    if (res.rows.length === 0) {
      console.log('No backups found in the specified period.');
      return;
    }

    console.log(`\n=== Recent Backups (last ${interval}) ===\n`);
    console.log(`${'ID'.padEnd(38)} ${'Type'.padEnd(10)} ${'Status'.padEnd(14)} ${'Created'.padEnd(28)} ${'Size'.padEnd(10)} ${'Duration'.padEnd(10)}`);
    console.log('-'.repeat(110));

    for (const row of res.rows) {
      const size = row.size_bytes ? (row.size_bytes / 1024 / 1024).toFixed(1) + ' MB' : '—';
      const dur = row.duration_ms ? (row.duration_ms / 1000).toFixed(0) + 's' : '—';
      console.log(
        `${row.id.padEnd(38)} ${row.type.padEnd(10)} ${row.status.padEnd(14)} ${row.created_at.toISOString().padEnd(28)} ${size.padEnd(10)} ${dur.padEnd(10)}`,
      );
    }

    console.log(`\n${res.rows.length} backup(s) found.`);
  } finally {
    await pool.end();
  }
}

main().catch(err => {
  console.error('Fatal:', err.message);
  process.exit(1);
});
