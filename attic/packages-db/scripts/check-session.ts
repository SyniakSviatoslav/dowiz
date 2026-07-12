import pg from 'pg';
const { Pool } = pg;
const pool = new Pool({
  connectionString: process.env.DATABASE_URL_SESSION,
  max: 1,
  ssl: { rejectUnauthorized: false },
});

async function main() {
  const c = await pool.connect();
  try {
    // Check for pgboss tables
    const tables = await c.query("SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename LIKE '%boss%'");
    console.log('=== PBBOSS TABLES ===');
    console.log(JSON.stringify(tables.rows, null, 2));
    
    // Check job count
    try {
      const jobs = await c.query("SELECT COUNT(*) as count FROM pgboss.job");
      console.log('\n=== JOB COUNT (pgboss schema) ===');
      console.log(JSON.stringify(jobs.rows, null, 2));
    } catch (e) {
      console.log('\n=== JOB COUNT FAILED ===');
      console.log(e.message);
    }
    
    // Check notification target
    const targets = await c.query("SELECT id, locale, prefs FROM owner_notification_targets WHERE id = '5e7b8534-6331-4589-b6d1-93ab2b6ccd74'");
    console.log('\n=== TARGET ===');
    console.log(JSON.stringify(targets.rows, null, 2));
  } finally {
    c.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });