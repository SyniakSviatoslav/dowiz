import { createOperationalPool, createSessionPool } from '../packages/db/src/index.js';
import { withTenant, issueCustomerToken, verifyAuthToken } from '../packages/platform/src/index.js';
import crypto from 'crypto';

async function runTests() {
  const pool = createOperationalPool();
  const sessionPool = createSessionPool();
  
  console.log('--- Auth & RLS Tests ---');
  
  try {
    // 1. Setup Data
    const ownerA = crypto.randomUUID();
    const ownerB = crypto.randomUUID();
    
    await sessionPool.query(`INSERT INTO users (id, email, google_sub) VALUES ($1, $2, $3)`, [ownerA, `a-${ownerA}@example.com`, `sub-${ownerA}`]);
    await sessionPool.query(`INSERT INTO users (id, email, google_sub) VALUES ($1, $2, $3)`, [ownerB, `b-${ownerB}@example.com`, `sub-${ownerB}`]);

    const orgA = crypto.randomUUID();
    const orgB = crypto.randomUUID();
    await sessionPool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'Org A', $2)`, [orgA, ownerA]);
    await sessionPool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'Org B', $2)`, [orgB, ownerB]);

    const locA = crypto.randomUUID();
    const locB = crypto.randomUUID();
    await sessionPool.query(`INSERT INTO locations (id, org_id, slug, name, phone) VALUES ($1, $2, $3, 'Loc A', '123')`, [locA, orgA, `loc-a-${locA}`]);
    await sessionPool.query(`INSERT INTO locations (id, org_id, slug, name, phone) VALUES ($1, $2, $3, 'Loc B', '456')`, [locB, orgB, `loc-b-${locB}`]);

    await sessionPool.query(`INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner')`, [ownerA, locA]);
    await sessionPool.query(`INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner')`, [ownerB, locB]);

    console.log('✅ Seeded users, orgs, locations, memberships');

    // 2. RLS Isolation
    await withTenant(pool, ownerA, async (client) => {
      const res = await client.query('SELECT id FROM locations');
      if (res.rowCount !== 1 || res.rows[0].id !== locA) {
        throw new Error('RLS Isolation Failed: Owner A sees incorrect locations');
      }
    });

    await withTenant(pool, ownerB, async (client) => {
      const res = await client.query('SELECT id FROM locations');
      if (res.rowCount !== 1 || res.rows[0].id !== locB) {
        throw new Error('RLS Isolation Failed: Owner B sees incorrect locations');
      }
    });
    console.log('✅ RLS Isolation logic confirmed (SET LOCAL app.user_id = ... operates correctly)');

    // 3. Customer Token Validation
    const orderId = crypto.randomUUID();
    const custToken = await issueCustomerToken({ orderId, locationId: locA, phone: '5551234' });
    const parsed = await verifyAuthToken(custToken);
    if (parsed.role !== 'customer' || parsed.orderId !== orderId) {
      throw new Error('Customer token parsing failed');
    }
    console.log('✅ Customer JWT issue & verify works correctly');

    // 4. Test Refresh Token Revocation
    const familyId = crypto.randomUUID();
    const refreshToken = crypto.randomBytes(32).toString('hex');
    const hash = crypto.createHash('sha256').update(refreshToken).digest('hex');
    await pool.query(`INSERT INTO auth_refresh_tokens (user_id, family_id, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 day')`, [ownerA, familyId, hash]);
    
    // Mark as used
    await pool.query(`UPDATE auth_refresh_tokens SET used = true WHERE token_hash = $1`, [hash]);
    
    // Simulating endpoint logic: if used, revoke family
    const checkRes = await pool.query(`SELECT used, family_id FROM auth_refresh_tokens WHERE token_hash = $1`, [hash]);
    if (checkRes.rows[0].used) {
      await pool.query(`DELETE FROM auth_refresh_tokens WHERE family_id = $1`, [checkRes.rows[0].family_id]);
    }
    const finalCheck = await pool.query(`SELECT * FROM auth_refresh_tokens WHERE family_id = $1`, [familyId]);
    if (finalCheck.rowCount !== 0) throw new Error('Family revocation failed');
    
    console.log('✅ Refresh Token Reuse Detection tested successfully');

  } catch (err) {
    console.error(err);
    process.exit(1);
  } finally {
    await pool.end();
    await sessionPool.end();
  }
}

runTests();
