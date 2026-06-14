import pg from 'pg';
const { Pool } = pg;

async function main() {
  const connectionString = 'postgresql://deliveryos_api_user.elxukhxvuycnftqwaghg:DeliveryOS_Api_User_Secure_123@aws-1-eu-central-1.pooler.supabase.com:6543/postgres';
  const pool = new Pool({
    connectionString,
    max: 1,
    ssl: { rejectUnauthorized: false }
  });

  const client = await pool.connect();
  try {
    // Check recent notify.dispatch jobs
    console.log('=== RECENT NOTIFY.DISPATCH JOBS (LAST 5) ===');
    const dispatchJobs = await client.query(`
      SELECT id, name, state, data, attempts, failure_message, created_on, finished_on
      FROM job
      WHERE name = 'notify.dispatch'
      ORDER BY created_on DESC
      LIMIT 5
    `);
    for (const job of dispatchJobs.rows) {
      console.log(`ID: ${job.id}`);
      console.log(`  State: ${job.state}`);
      console.log(`  Attempts: ${job.attempts}`);
      console.log(`  Created: ${job.created_on}`);
      console.log(`  Finished: ${job.finished_on || 'NULL'}`);
      if (job.failure_message) {
        console.log(`  Failure: ${job.failure_message}`);
      }
      // Parse and show key data fields
      if (job.data) {
        const data = typeof job.data === 'string' ? JSON.parse(job.data) : job.data;
        console.log(`  Data: targetId=${data.targetId}, eventType=${data.eventType}, orderId=${data.orderId || 'NULL'}, testMessage=${data.testMessage || 'NULL'}`);
      }
      console.log('');
    }

    // Check recent notify.telegram.send jobs
    console.log('\n=== RECENT NOTIFY.TELEGRAM.SEND JOBS (LAST 5) ===');
    const telegramJobs = await client.query(`
      SELECT id, name, state, data, attempts, failure_message, created_on, finished_on
      FROM job
      WHERE name = 'notify.telegram.send'
      ORDER BY created_on DESC
      LIMIT 5
    `);
    for (const job of telegramJobs.rows) {
      console.log(`ID: ${job.id}`);
      console.log(`  State: ${job.state}`);
      console.log(`  Attempts: ${job.attempts}`);
      console.log(`  Created: ${job.created_on}`);
      console.log(`  Finished: ${job.finished_on || 'NULL'}`);
      if (job.failure_message) {
        console.log(`  Failure: ${job.failure_message}`);
      }
      // Parse and show key data fields
      if (job.data) {
        const data = typeof job.data === 'string' ? JSON.parse(job.data) : job.data;
        console.log(`  Data: event=${data.event}, entity_id=${data.entity_id || 'NULL'}, location_id=${data.location_id}`);
      }
      console.log('');
    }

    // Summary counts by state
    console.log('\n=== JOB STATE SUMMARY ===');
    const stateSummary = await client.query(`
      SELECT name, state, COUNT(*) as count
      FROM job
      WHERE name IN ('notify.dispatch', 'notify.telegram.send')
      GROUP BY name, state
      ORDER BY name, state
    `);
    for (const row of stateSummary.rows) {
      console.log(`${row.name}: ${row.state} = ${row.count}`);
    }

    // Check for any failed jobs in these two types
    console.log('\n=== FAILED NOTIFY JOBS ===');
    const failedNotifyJobs = await client.query(`
      SELECT id, name, state, failure_message, created_on
      FROM job
      WHERE name IN ('notify.dispatch', 'notify.telegram.send')
        AND state = 'failed'
      ORDER BY created_on DESC
      LIMIT 10
    `);
    if (failedNotifyJobs.rows.length === 0) {
      console.log('No failed notify jobs found.');
    } else {
      for (const job of failedNotifyJobs.rows) {
        console.log(`ID: ${job.id} | Name: ${job.name} | Failed: ${job.failure_message} | Created: ${job.created_on}`);
        if (job.data) {
          const data = typeof job.data === 'string' ? JSON.parse(job.data) : job.data;
          if (job.name === 'notify.dispatch') {
            console.log(`  Dispatch data: targetId=${data.targetId}, event=${data.eventType}`);
          } else if (job.name === 'notify.telegram.send') {
            console.log(`  Telegram data: event=${data.event}, entity_id=${data.entity_id}`);
          }
        }
      }
    }
  } finally {
    client.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });