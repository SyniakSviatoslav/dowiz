import { createOperationalPool } from '@deliveryos/db';
import { loadEnv } from '@deliveryos/config';
import assert from 'node:assert/strict';

async function main() {
  const env = loadEnv();
  const pool = createOperationalPool();
  
  // E2E for Stage 13: SSR & SEO Edge Cache Architecture
  // Note: To fully test, the Fastify app must be running, but we can verify DB components 
  // and schema functions directly.

  console.log('[Stage13] Test suite ready.');

  try {
    const client = await pool.connect();
    
    // Test the helper function exists
    const res = await client.query(`
      SELECT exists(
        SELECT * 
        FROM pg_proc 
        WHERE proname = 'read_public_menu_all_locales'
      ) as func_exists;
    `);
    
    assert.equal(res.rows[0].func_exists, true, 'read_public_menu_all_locales function must exist');
    
    client.release();
    console.log('[Stage13] DB components verified successfully!');
  } catch (err: any) {
    console.log(`[Stage13] Verification Failed: ${err.message}`);
  }

  await pool.end();
}

main().catch(console.error);
