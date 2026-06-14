import { createSessionPool } from '@deliveryos/db';

async function test() {
  const p = createSessionPool();
  try {
    await p.query(`
      INSERT INTO ops_worker_heartbeat (worker_id, last_seen_at) 
      VALUES ('test-worker', now()) 
      ON CONFLICT (worker_id) DO UPDATE SET last_seen_at = now()
    `);
    const res = await p.query('SELECT * FROM ops_worker_heartbeat');
    console.log(res.rows);
  } catch (err) {
    console.error(err);
  } finally {
    await p.end();
  }
}

test();
