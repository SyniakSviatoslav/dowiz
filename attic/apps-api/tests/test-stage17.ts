import { createSessionPool } from '@deliveryos/db';
import { encryptPII, decryptPII } from '../src/lib/pii-cipher.js';
import crypto from 'node:crypto';
import argon2 from 'argon2';

async function runTests() {
  console.log('--- Stage 17: Courier Auth & Identity ---');

  const db = createSessionPool();
  const client = await db.connect();
  let orgId: string | undefined;
  try {
    await client.query(`DELETE FROM locations WHERE slug = 'courier-test-loc-17'`);
    await client.query(`DELETE FROM organizations WHERE name = 'Courier Test Org 17'`);
    
    // 1. Setup Test Data
    orgId = crypto.randomUUID();
    await client.query(`INSERT INTO organizations (id, name) VALUES ($1, 'Courier Test Org 17')`, [orgId]);
    
    const locId = crypto.randomUUID();
    await client.query(
      `INSERT INTO locations (id, org_id, slug, name, phone) VALUES ($1, $2, 'courier-test-loc-17', 'Loc 17', '123')`,
      [locId, orgId]
    );

    const ownerId = crypto.randomUUID();
    await client.query(
      `INSERT INTO users (id, email) VALUES ($1, 'test-owner-17@example.com')`,
      [ownerId]
    );
    await client.query(
      `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner')`,
      [ownerId, locId]
    );

    // 2. Create Invite
    console.log('Testing Invite Creation...');
    const emailPlain = 'courier17@example.com';
    const emailHash = crypto.createHash('sha256').update(emailPlain).digest('hex');
    const codePlain = 'TESTCODE123';
    const codeHash = await argon2.hash(codePlain);

    const inviteRes = await client.query(
      `INSERT INTO courier_invites (location_id, created_by_owner_id, role, invited_email_hash, code_hash, expires_at)
       VALUES ($1, $2, 'courier', $3, $4, now() + interval '1 hour') RETURNING id`,
      [locId, ownerId, emailHash, codeHash]
    );
    const inviteId = inviteRes.rows[0].id;
    console.log('✓ Invite created successfully');

    // 3. Redeem Invite
    console.log('Testing Invite Redeem...');
    // Simulated redeem payload:
    const password = 'SuperSecret123!';
    const fullName = 'John Courier';

    const pwHash = await argon2.hash(password);
    const emailEncrypted = encryptPII(emailPlain);
    const fnEncrypted = encryptPII(fullName);

    const courierRes = await client.query(
      `INSERT INTO couriers (email_encrypted, email_hash, full_name_encrypted, password_hash)
       VALUES ($1, $2, $3, $4) RETURNING id`,
      [emailEncrypted, emailHash, fnEncrypted, pwHash]
    );
    const courierId = courierRes.rows[0].id;

    await client.query(
      `INSERT INTO courier_locations (courier_id, location_id, role, added_by_owner_id)
       VALUES ($1, $2, 'courier', $3)`,
      [courierId, locId, ownerId]
    );

    await client.query(`UPDATE courier_invites SET used_at = now(), used_by_courier_id = $1 WHERE id = $2`, [courierId, inviteId]);
    console.log('✓ Invite redeemed successfully');

    // 4. Verify PII Decryption
    console.log('Testing PII Security...');
    const row = await client.query(`SELECT email_encrypted, full_name_encrypted FROM couriers WHERE id = $1`, [courierId]);
    const decryptedEmail = decryptPII(row.rows[0].email_encrypted);
    const decryptedFn = decryptPII(row.rows[0].full_name_encrypted);

    if (decryptedEmail !== emailPlain) throw new Error('Email decryption failed');
    if (decryptedFn !== fullName) throw new Error('Full name decryption failed');
    console.log('✓ PII encryption/decryption roundtrip successful');

    console.log('All Stage 17 database tests passed.');
  } finally {
    // Cleanup
    if (orgId) {
      await client.query(`DELETE FROM locations WHERE org_id = $1`, [orgId]);
      await client.query(`DELETE FROM organizations WHERE id = $1`, [orgId]);
    }
    client.release();
    db.end();
  }
}

runTests().catch(e => {
  console.error(e);
  process.exit(1);
});
