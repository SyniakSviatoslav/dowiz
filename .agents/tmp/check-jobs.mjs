import pg from 'pg';
const { Pool } = pg;
const pool = new Pool({
  connectionString: 'postgresql://deliveryos_api_user.elxukhxvuycnftqwaghg:DeliveryOS_Api_User_Secure_123@aws-1-eu-central-1.pooler.supabase.com:6543/postgres',
  max: 1,
  ssl: { rejectUnauthorized: false },
});

async function main() {
  const c = await pool.connect();
  try {
    // Check pg-boss jobs
    const jobs = await c.query("SELECT id, name, state, created_on FROM pg_boss.job ORDER BY created_on DESC LIMIT 10");
    console.log('=== PG-BOSS JOBS ===');
    console.log(JSON.stringify(jobs.rows, null, 2));

    // Check notification targets
    const targets = await c.query("SELECT id, channel, address, status, locale, prefs FROM owner_notification_targets WHERE id = '5e7b8534-6331-4589-b6d1-93ab2b6ccd74'");
    console.log('\n=== NOTIFICATION TARGET ===');
    console.log(JSON.stringify(targets.rows, null, 2));

    // Check if there's an order.created event in the message bus log
    try {
      const events = await c.query("SELECT * FROM pg_boss.job WHERE name LIKE '%notify%' ORDER BY created_on DESC LIMIT 5");
      console.log('\n=== NOTIFY JOBS ===');
      console.log(JSON.stringify(events.rows, null, 2));
    } catch { console.log('\n=== No notify jobs found ==='); }

  } finally {
    c.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });
