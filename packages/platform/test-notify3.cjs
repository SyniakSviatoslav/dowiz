const { Pool } = require('pg');
const pool1 = new Pool({ connectionString: 'postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db' });
const pool2 = new Pool({ connectionString: 'postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db' });
async function run() {
  const listener = await pool1.connect();
  listener.on('notification', (msg) => {
    console.log('Received:', msg.payload);
  });
  await listener.query(`LISTEN "order:test"`);
  console.log('Listening on pool1');
  
  await pool2.query(`NOTIFY "order:test", 'hello'`);
  console.log('Notified on pool2');
  
  await new Promise(r => setTimeout(r, 1000));
  process.exit(0);
}
run().catch(console.error);
