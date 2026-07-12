import { Pool } from 'pg';

async function probe(label: string, url: string) {
  const pool = new Pool({ connectionString: url, max: 1, connectionTimeoutMillis: 8000 });
  try {
    const r = await pool.query(
      `SELECT current_user,
              inet_server_port() AS port,
              has_schema_privilege(current_user,'public','CREATE') AS can_create_public`
    );
    const row = r.rows[0];
    console.log(`${label}: user=${row.current_user} port=${row.port} can_create_public=${row.can_create_public}`);
  } catch (e: any) {
    console.log(`${label}: ERROR ${e.message}`);
  } finally {
    await pool.end();
  }
}

const opUrl = process.env.***REDACTED***!;
const sessUrl = process.env.***REDACTED***!;
const migUrl = process.env.***REDACTED***!;

// pg-boss constructed URL: operational URL with port 5432
const bossUrl = new URL(opUrl);
bossUrl.port = '5432';

await probe('OPERATIONAL (6543)', opUrl);
await probe('SESSION (5432)', sessUrl);
await probe('MIGRATIONS (5432)', migUrl);
await probe('PG-BOSS (constructed 5432)', bossUrl.toString());
