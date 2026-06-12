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
    // List schemas
    const schemas = await client.query("SELECT schema_name FROM information_schema.schemata WHERE schema_name NOT IN ('information_schema', 'pg_catalog') ORDER BY schema_name");
    console.log('=== SCHEMAS ===');
    console.log(JSON.stringify(schemas.rows, null, 2));

    // Check for job tables in each schema
    for (const schemaRow of schemas.rows) {
      const schema = schemaRow.schema_name;
      try {
        const count = await client.query(`SELECT COUNT(*) as count FROM "${schema}".job`);
        console.log(`\n=== JOB COUNT IN ${schema} ===`);
        console.log(JSON.stringify(count.rows, null, 2));
      } catch (e) {
        // Silently skip schemas without job table or no permission
      }
    }

    // Check for tables with 'job' in name
    const tables = await client.query("SELECT table_schema, table_name FROM information_schema.tables WHERE table_name LIKE '%job%' AND table_schema NOT IN ('information_schema', 'pg_catalog') ORDER BY table_schema, table_name");
    console.log('\n=== TABLES WITH "job" IN NAME ===');
    console.log(JSON.stringify(tables.rows, null, 2));
  } finally {
    client.release();
    await pool.end();
  }
}
main().catch(e => { console.error(e); process.exit(1); });