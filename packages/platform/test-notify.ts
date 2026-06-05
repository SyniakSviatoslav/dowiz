import { Pool } from 'pg';
const pool = new Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/deliveryos' });
async function run() {
  const listener = await pool.connect();
  listener.on('notification', (msg) => {
    console.log('Received:', msg);
  });
  await listener.query(`LISTEN "order:test"`);
  console.log('Listening');
  
  await pool.query(`NOTIFY "order:test", 'hello'`);
  console.log('Notified');
  
  await new Promise(r => setTimeout(r, 1000));
  process.exit(0);
}
run();
