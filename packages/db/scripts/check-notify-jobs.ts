import { createOperationalPool } from '../src/index.js';

async function main() {
  const pool = createOperationalPool();
  const client = await pool.connect();
  try {
    // Check recent notify.dispatch jobs
    const dispatchJobs = await client.query(`
      SELECT id, name, state, data, attempts, failure_message, created_on, finished_on
      FROM pgboss.job
      WHERE name = 'notify.dispatch'
      ORDER BY created_on DESC
      LIMIT 5
    `);
    console.log('=== NOTIFY.DISPATCH JOBS ===');
    console.log(JSON.stringify(dispatchJobs.rows, null, 2));

    // Check recent notify.telegram.send jobs
    const telegramJobs = await client.query(`
      SELECT id, name, state, data, attempts, failure_message, created_on, finished_on
      FROM pgboss.job
      WHERE name = 'notify.telegram.send'
      ORDER BY created_on DESC
      LIMIT 5
    `);
    console.log('\n=== NOTIFY.TELEGRAM.SEND JOBS ===');
    console.log(JSON.stringify(telegramJobs.rows, null, 2));

    // Also check for any failed jobs in general
    const failedJobs = await client.query(`
      SELECT id, name, state, failure_message, created_on, finished_on
      FROM pgboss.job
      WHERE state = 'failed'
      ORDER BY created_on DESC
      LIMIT 5
    `);
    console.log('\n=== FAILED JOBS (any) ===');
    console.log(JSON.stringify(failedJobs.rows, null, 2));
  } finally {
    client.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });