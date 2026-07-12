import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
loadEnv();
const pool = createSessionPool();
try {
  await pool.query('ALTER TABLE owner_notification_targets ADD COLUMN IF NOT EXISTS user_id UUID');
  console.log('✅ Added user_id to owner_notification_targets');
} catch (e: any) {
  console.log('⚠️ owner_notification_targets user_id:', e.message);
}
try {
  await pool.query('ALTER TABLE telegram_connect_tokens RENAME COLUMN owner_id TO user_id');
  console.log('✅ Renamed owner_id → user_id in telegram_connect_tokens');
} catch (e: any) {
  console.log('⚠️ telegram_connect_tokens rename:', e.message);
}
try {
  await pool.query(`CREATE UNIQUE INDEX IF NOT EXISTS idx_ont_loc_channel_addr ON owner_notification_targets (location_id, channel, address)`);
  console.log('✅ Created unique index on owner_notification_targets');
} catch (e: any) {
  console.log('⚠️ idx_ont_loc_channel_addr:', e.message);
}
await pool.end();
console.log('Done.');
