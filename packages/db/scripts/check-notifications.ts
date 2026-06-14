import pg from 'pg';
const { Pool } = pg;
const pool = new Pool({
  connectionString: process.env.***REDACTED***,
  max: 1,
  ssl: { rejectUnauthorized: false },
});

async function main() {
  const c = await pool.connect();
  try {
    // Check notification targets
    const targets = await c.query("SELECT id, channel, address, status, locale, prefs FROM owner_notification_targets WHERE id = '5e7b8534-6331-4589-b6d1-93ab2b6ccd74'");
    console.log('=== NOTIFICATION TARGET ===');
    console.log(JSON.stringify(targets.rows, null, 2));

    // Check available schemas
    const schemas = await c.query("SELECT schema_name FROM information_schema.schemata WHERE schema_name LIKE '%boss%' OR schema_name LIKE '%pg%'");
    console.log('\n=== SCHEMAS ===');
    console.log(JSON.stringify(schemas.rows, null, 2));

    // Check search path
    const path = await c.query("SHOW search_path");
    console.log('\n=== SEARCH PATH ===');
    console.log(JSON.stringify(path.rows, null, 2));

    // Check for order
    const orders = await c.query("SELECT id, short_id, status, location_id, total FROM orders WHERE location_id = '4c88d024-d7ad-4c59-ba9c-0fe581aac549' ORDER BY created_at DESC LIMIT 3");
    console.log('\n=== RECENT ORDERS ===');
    console.log(JSON.stringify(orders.rows, null, 2));
  } finally {
    c.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });
