import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import crypto from 'crypto';
import { signAuthToken } from '@deliveryos/platform';

const env = loadEnv();

async function runTests() {
  const pool = createSessionPool();
  try {
    console.log('--- Stage 9: E2E Menu CRUD Tests ---');

    const locId = crypto.randomUUID();
    const userId = crypto.randomUUID();

    const orgId = crypto.randomUUID();

    await pool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`, [userId, `owner9-${Date.now()}@test.com`]);
    await pool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'Org9', $2) ON CONFLICT DO NOTHING`, [orgId, userId]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, default_locale, supported_locales) VALUES ($1, $2, $3, 'Loc9', '123', 'open', 'sq', '{"sq", "en"}') ON CONFLICT DO NOTHING`, [locId, orgId, `loc9-${Date.now()}`]);
    await pool.query(`INSERT INTO memberships (user_id, location_id, role, status) VALUES ($1, $2, 'owner', 'active') ON CONFLICT DO NOTHING`, [userId, locId]);

    const ownerToken = await signAuthToken({ role: 'owner', userId }, '15m');
    const authHeaders = { 'Content-Type': 'application/json', 'Authorization': `Bearer ${ownerToken}` };

    // Get initial menu version
    let res = await pool.query(`SELECT version FROM menu_versions WHERE location_id = $1`, [locId]);
    let menuVersion = res.rows[0]?.version || 0;

    console.log('Testing Category CRUD & Version Bump...');
    const catRes = await fetch(`http://127.0.0.1:3003/api/owner/locations/${locId}/categories`, {
      method: 'POST',
      headers: authHeaders,
      body: JSON.stringify({ name: 'Burgers' })
    });
    if (catRes.status !== 201) throw new Error(`Category creation failed: ${await catRes.text()}`);
    const category = await catRes.json() as any;

    res = await pool.query(`SELECT version FROM menu_versions WHERE location_id = $1`, [locId]);
    if (!res.rows[0] || parseInt(res.rows[0].version) <= menuVersion) throw new Error('menu_version did not bump on Category insert');
    menuVersion = parseInt(res.rows[0].version);
    console.log('✅ menu_version bumped on Category insert');

    console.log('Testing Zod Strict...');
    const strictRes = await fetch(`http://127.0.0.1:3003/api/owner/locations/${locId}/categories`, {
      method: 'POST',
      headers: authHeaders,
      body: JSON.stringify({ name: 'Drinks', evil: 1 })
    });
    if (strictRes.status !== 400) throw new Error('Zod strict failed to reject unknown keys');
    console.log('✅ Zod strict rejected unknown key with 400');

    console.log('Testing Product CRUD...');
    const prodRes = await fetch(`http://127.0.0.1:3003/api/owner/locations/${locId}/products`, {
      method: 'POST',
      headers: authHeaders,
      body: JSON.stringify({ category_id: category.id, name: 'Cheeseburger', price: 1500 })
    });
    if (prodRes.status !== 201) throw new Error(`Product creation failed: ${await prodRes.text()}`);
    const product = await prodRes.json() as any;

    res = await pool.query(`SELECT version FROM menu_versions WHERE location_id = $1`, [locId]);
    if (parseInt(res.rows[0].version) <= menuVersion) throw new Error('menu_version did not bump on Product insert');
    menuVersion = parseInt(res.rows[0].version);

    console.log('Testing i18n Fallbacks...');
    await fetch(`http://127.0.0.1:3003/api/owner/locations/${locId}/products/${product.id}/translations/en`, {
      method: 'PUT',
      headers: authHeaders,
      body: JSON.stringify({ name: 'Cheeseburger EN' })
    });

    const pubResEn = await fetch(`http://127.0.0.1:3003/public/locations/${locId}/menu?locale=en`);
    const pubDataEn = await pubResEn.json() as any;
    if (pubDataEn.categories[0].products[0].name !== 'Cheeseburger EN') throw new Error('i18n EN failed');

    const pubResDe = await fetch(`http://127.0.0.1:3003/public/locations/${locId}/menu?locale=de`);
    const pubDataDe = await pubResDe.json() as any;
    if (pubDataDe.categories[0].products[0].name !== 'Cheeseburger') throw new Error('i18n fallback to default/original failed');
    console.log('✅ i18n public read and fallback passed');

    console.log('Testing Stop-list (available=false)...');
    await fetch(`http://127.0.0.1:3003/api/owner/locations/${locId}/products/${product.id}`, {
      method: 'PATCH',
      headers: authHeaders,
      body: JSON.stringify({ available: false })
    });
    
    const pubResStop = await fetch(`http://127.0.0.1:3003/public/locations/${locId}/menu`);
    const pubDataStop = await pubResStop.json() as any;
    if (pubDataStop.categories[0].products.length !== 0) throw new Error('Stop-list failed to hide product');
    console.log('✅ Stop-list hides product from public menu');

    console.log('Testing Hard Delete & FK SET NULL semantics...');
    const orderId = crypto.randomUUID();
    const orderItemId = crypto.randomUUID();
    await pool.query(`INSERT INTO orders (id, location_id, subtotal, total) VALUES ($1, $2, 1500, 1500)`, [orderId, locId]);
    await pool.query(`INSERT INTO order_items (id, order_id, product_id, quantity, price_snapshot, name_snapshot) VALUES ($1, $2, $3, 1, 1500, 'Snapshot Name')`, [orderItemId, orderId, product.id]);
    
    const delRes = await fetch(`http://127.0.0.1:3003/api/owner/locations/${locId}/products/${product.id}`, {
      method: 'DELETE',
      headers: { 'Authorization': `Bearer ${ownerToken}` }
    });
    if (delRes.status !== 204) throw new Error(`Delete failed: ${await delRes.text()}`);

    const oiRes = await pool.query(`SELECT product_id, name_snapshot FROM order_items WHERE id = $1`, [orderItemId]);
    if (oiRes.rows[0].product_id !== null) throw new Error('product_id was not set to NULL on order_items');
    if (oiRes.rows[0].name_snapshot !== 'Snapshot Name') throw new Error('Snapshot was modified');
    console.log('✅ Hard delete semantics verified (FK SET NULL, snapshot survives)');

    console.log('Testing Rate Limit...');
    let rateLimited = false;
    for (let i = 0; i < 110; i++) {
      const rlRes = await fetch(`http://127.0.0.1:3003/api/owner/locations/${locId}/categories`, {
        method: 'POST',
        headers: authHeaders,
        body: JSON.stringify({ name: `Test ${i}` })
      });
      if (rlRes.status === 429) {
        rateLimited = true;
        break;
      }
    }
    if (!rateLimited) throw new Error('Rate limit was not triggered after 100 requests');
    console.log('✅ Rate Limit enforced (429)');

    console.log('🎉 All Stage 9 tests passed!');

  } finally {
    await pool.end();
  }
}

runTests().catch(err => {
  console.error('Test failed:', err);
  process.exit(1);
});
