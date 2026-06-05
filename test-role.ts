import { createOperationalPool } from './packages/db/src/index.js';
async function run() {
  const p = createOperationalPool();
  const res = await p.query("SELECT rolname, rolsuper, rolbypassrls FROM pg_roles WHERE rolname = 'deliveryos_api_user'");
  console.log('Role:', res.rows);
  const res2 = await p.query("SELECT * FROM pg_policies WHERE tablename = 'orders'");
  console.log('Policies:', res2.rows);
  const res3 = await p.query("SELECT relrowsecurity, relforcerowsecurity FROM pg_class WHERE relname = 'orders'");
  console.log('Force RLS:', res3.rows);
  process.exit(0);
}
run();
