import pg from 'pg';
const { Pool } = pg;

async function main() {
  const connectionString = 'postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db';
  const pool = new Pool({
    connectionString,
    max: 1,
    ssl: { rejectUnauthorized: false }
  });

  const client = await pool.connect();
  try {
    // Check job table schema
    const columns = await client.query(`
      SELECT column_name, data_type, is_nullable
      FROM information_schema.columns
      WHERE table_schema = 'public' 
        AND table_name = 'job'
      ORDER BY ordinal_position
    `);
    console.log('=== JOB TABLE SCHEMA ===');
    console.log(JSON.stringify(columns.rows, null, 2));
  } finally {
    client.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });