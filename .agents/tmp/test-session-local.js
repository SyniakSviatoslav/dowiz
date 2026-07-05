const { Pool } = require('pg');
const pool = new Pool({
  connectionString: process.env.DATABASE_URL_SESSION,
  max: 1,
  ssl: { rejectUnauthorized: false },
});

async function main() {
  const client = await pool.connect();
  try {
    console.log('✅ Session pool connected');
    const res = await client.query('SELECT 1');
    console.log('✅ Query worked:', res.rows[0]);
  } catch (e) {
    console.log('❌ Connection failed:', e.message);
  } finally {
    client.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });