import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
loadEnv();
const pool = createSessionPool();
const r = await pool.query(`SELECT column_name FROM information_schema.columns WHERE table_name = 'orders' ORDER BY ordinal_position`);
console.log(r.rows.map(x => x.column_name).join(', '));
await pool.end();
