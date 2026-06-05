import PgBoss from 'pg-boss';
import { createOperationalPool, createSessionPool } from '@deliveryos/db';
import { loadEnv } from '@deliveryos/config';
import pg from 'pg';

async function run() {
  const env = loadEnv();
  
  // 1. Open Session pool (max 3)
  const sessionPool = createSessionPool();
  const sessionClients = [];
  for (let i = 0; i < 3; i++) sessionClients.push(await sessionPool.connect());
  
  // 2. Open Operational pool (max 8)
  const opPool = createOperationalPool();
  const opClients = [];
  for (let i = 0; i < 8; i++) opClients.push(await opPool.connect());
  
  // 3. Start pg-boss (max 3)
  const boss = new PgBoss({ connectionString: env.DATABASE_URL_SESSION, max: 3 });
  await boss.start();
  
  // 4. Run transient migration-like connection
  const migrationClient = new pg.Client({ connectionString: env.DATABASE_URL_SESSION });
  await migrationClient.connect();
  
  // Now we have 3 + 8 + 3 + 1 = 15 max connections, but let's measure backend connections.
  const res = await migrationClient.query("SELECT count(*) as active_connections FROM pg_stat_activity WHERE datname = 'postgres'");
  console.log('Active Connections in DB:', res.rows[0].active_connections);
  
  const res2 = await migrationClient.query("SHOW max_connections");
  console.log('Max Connections allowed:', res2.rows[0].max_connections);
  
  console.log('✅ H2 SUCCESS: All pools connected simultaneously without errors on Free tier.');
  
  // Cleanup
  await migrationClient.end();
  for (const c of opClients) c.release();
  for (const c of sessionClients) c.release();
  await sessionPool.end();
  await opPool.end();
  await boss.stop();
  process.exit(0);
}

run();
