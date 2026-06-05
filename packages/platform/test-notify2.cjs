const { Pool } = require('pg');
const pool = new Pool({ connectionString: 'postgresql://postgres.elxukhxvuycnftqwaghg:7V%23KxApMx8Z5B5.@aws-1-eu-central-1.pooler.supabase.com:5432/postgres' });
async function run() {
  const listener = await pool.connect();
  listener.on('notification', (msg) => {
    console.log('Received:', msg.payload);
  });
  await listener.query(`LISTEN "order:test"`);
  console.log('Listening');
  
  await pool.query(`NOTIFY "order:test", 'hello'`);
  console.log('Notified');
  
  await new Promise(r => setTimeout(r, 1000));
  process.exit(0);
}
run().catch(console.error);
