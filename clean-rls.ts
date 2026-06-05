import { createSessionPool } from './packages/db/src/index.js';
async function run() {
  const p = createSessionPool();
  await p.query("DROP POLICY IF EXISTS anonymous_select ON orders");
  await p.query("DROP POLICY IF EXISTS anonymous_select ON order_items");
  console.log("Cleaned test RLS policies");
  process.exit(0);
}
run();
