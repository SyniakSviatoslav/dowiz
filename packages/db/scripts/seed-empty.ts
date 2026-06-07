import { createSessionPool } from '../src/index.js';
import { randomUUID } from 'crypto';
import * as argon2 from 'argon2';

async function run() {
  const pool = createSessionPool();
  try {
    const passwordHash = await argon2.hash('empty123456');

    const emptyUserId = randomUUID();
    const userRes = await pool.query(
      `INSERT INTO users (id, email, display_name, password_hash) VALUES ($1, $2, $3, $4)
       ON CONFLICT (email) DO UPDATE SET password_hash = EXCLUDED.password_hash, display_name = EXCLUDED.display_name RETURNING id`,
      [emptyUserId, 'empty@dowiz.com', 'Empty Client', passwordHash]
    );
    const realUserId = userRes.rows[0].id;

    // Organization
    const orgId = randomUUID();
    const orgRes = await pool.query(
      `INSERT INTO organizations (id, name, owner_id) VALUES ($1, $2, $3)
       ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name RETURNING id`,
      [orgId, 'Empty Org', realUserId]
    );
    const realOrgId = orgRes.rows[0].id;

    // Location
    const locId = randomUUID();
    const locRes = await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, default_locale, supported_locales, currency_code, currency_minor_unit, delivery_fee_flat, status)
       VALUES ($1, $2, $3, $4, $5, 'sq', ARRAY['sq','en'], 'ALL', 0, 0, 'active')
       ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name RETURNING id`,
      [locId, realOrgId, 'empty-demo', 'Empty Location', '+355690000000']
    );
    const realLocId = locRes.rows[0].id;

    // Membership
    await pool.query(
      `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
      [realUserId, realLocId]
    );

    await pool.query('COMMIT');

    console.log('✅ Empty Client seeded successfully.');
    console.log('Credentials:');
    console.log('Email: empty@dowiz.com');
    console.log('Password: empty123456');
    console.log('Location slug: empty-demo');

  } catch (err) {
    await pool.query('ROLLBACK');
    console.error('❌ Seed failed:', err);
    process.exit(1);
  } finally {
    await pool.end();
  }
}

run();
