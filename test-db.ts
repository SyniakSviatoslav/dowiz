import { createSessionPool } from './packages/db/src/index.js';

async function test() {
  const p = createSessionPool();
  try {
    const res = await p.query("SELECT tablename, policyname, roles, cmd, qual, with_check FROM pg_policies WHERE schemaname = 'public'");
    console.log(res.rows);
  } finally {
    await p.end();
  }
}

test();
