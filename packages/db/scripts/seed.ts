import { createSessionPool } from '../src/index.js';
import { randomUUID } from 'crypto';

async function run() {
  const pool = createSessionPool();
  try {
    const orgId = randomUUID();
    const demoLocationId = randomUUID();
    const demo2LocationId = randomUUID();
    const ownerAId = randomUUID();
    const ownerBId = randomUUID();
    const courierId = randomUUID();
    
    // We are seeding with the Session Pool, which connects as the postgres role.
    // Assuming 'postgres' can bypass RLS or since we force RLS, we must either disable RLS briefly
    // or just execute these queries directly because 'postgres' with BYPASSRLS can bypass FORCE RLS? 
    // Wait, FORCE RLS applies to the owner, but BYPASSRLS bypasses even FORCE RLS. So 'postgres' should be able to insert.
    // If not, we can just not set app.user_id, wait, if we don't set it and BYPASSRLS is active, it works.
    
    await pool.query('BEGIN');

    // Users — use UPDATE to force RETURNING on conflict
    const ownerARes = await pool.query(
      `INSERT INTO users (id, email, display_name) VALUES ($1, $2, $3)
       ON CONFLICT (email) DO UPDATE SET display_name = EXCLUDED.display_name RETURNING id`,
      [ownerAId, 'ownera@demo.com', 'Owner A']
    );
    const ownerARealId = ownerARes.rows[0].id;
    const ownerBRes = await pool.query(
      `INSERT INTO users (id, email, display_name) VALUES ($1, $2, $3)
       ON CONFLICT (email) DO UPDATE SET display_name = EXCLUDED.display_name RETURNING id`,
      [ownerBId, 'ownerb@demo2.com', 'Owner B']
    );
    const ownerBRealId = ownerBRes.rows[0].id;
    const courierRes = await pool.query(
      `INSERT INTO users (id, email, display_name) VALUES ($1, $2, $3)
       ON CONFLICT (email) DO UPDATE SET display_name = EXCLUDED.display_name RETURNING id`,
      [courierId, 'courier@demo.com', 'Courier']
    );
    const courierRealId = courierRes.rows[0].id;

    // Test user for local login (password: test123456)
    const testUserId = randomUUID();
    const testUserRes = await pool.query(
      `INSERT INTO users (id, email, display_name) VALUES ($1, $2, $3)
       ON CONFLICT (email) DO UPDATE SET display_name = EXCLUDED.display_name RETURNING id`,
      [testUserId, 'test@dowiz.com', 'Test Owner']
    );
    const testUserRealId = testUserRes.rows[0].id;

    // Assign test user to demo location as owner (use actual existing demo location)
    const demoLoc = await pool.query(`SELECT id FROM locations WHERE slug = 'demo' LIMIT 1`);
    if (demoLoc.rows.length > 0) {
      await pool.query(
        `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING`,
        [testUserRealId, demoLoc.rows[0].id]
      );
    }

    // Organization
    const orgRes = await pool.query(
      `INSERT INTO organizations (id, name, owner_id) VALUES ($1, $2, $3)
       ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name RETURNING id`,
      [orgId, 'Demo Org', ownerARealId]
    );
    const realOrgId = orgRes.rows[0].id;

    // Locations — with locale and commerce columns that exist in schema
    const loc1Res = await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, default_locale, supported_locales, currency_code, currency_minor_unit, delivery_fee_flat, min_order_value, free_delivery_threshold, status)
       VALUES ($1, $2, $3, $4, $5, 'sq', ARRAY['sq','en'], 'ALL', 0, 200, 500, 2000, 'active')
       ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name, default_locale = 'sq', supported_locales = ARRAY['sq','en'], status = 'active', delivery_fee_flat = 200, min_order_value = 500, free_delivery_threshold = 2000 RETURNING id`,
      [demoLocationId, realOrgId, 'demo', 'Demo Location', '+355691234567']
    );
    const realDemoLocId = loc1Res.rows[0].id;
    const loc2Res = await pool.query(
      `INSERT INTO locations (id, org_id, slug, name, phone, default_locale, supported_locales, currency_code, currency_minor_unit, delivery_fee_flat, status)
       VALUES ($1, $2, $3, $4, $5, 'sq', ARRAY['sq','en'], 'ALL', 0, 300, 'active')
       ON CONFLICT (slug) DO UPDATE SET name = EXCLUDED.name, default_locale = 'sq', supported_locales = ARRAY['sq','en'], status = 'active', delivery_fee_flat = 200, min_order_value = 500, free_delivery_threshold = 2000 RETURNING id`,
      [demo2LocationId, realOrgId, 'demo2', 'Demo 2 Location', '+355691234568']
    );
    const realDemo2LocId = loc2Res.rows[0].id;

    // Memberships
    await pool.query(
      `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING`,
      [ownerARealId, realDemoLocId, 'owner']
    );
    await pool.query(
      `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING`,
      [ownerBRealId, realDemo2LocId, 'owner']
    );
    await pool.query(
      `INSERT INTO memberships (user_id, location_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING`,
      [courierRealId, realDemoLocId, 'courier']
    );

    // Categories & Products (Idempotent check)
    let categoryId: string;
    const catCheck = await pool.query(`SELECT id FROM categories WHERE location_id = $1 AND name = 'Pizzas'`, [realDemoLocId]);
    if (catCheck.rowCount && catCheck.rowCount > 0) {
      categoryId = catCheck.rows[0].id;
    } else {
      categoryId = randomUUID();
      await pool.query(`INSERT INTO categories (id, location_id, name) VALUES ($1, $2, $3)`, [categoryId, realDemoLocId, 'Pizzas']);
    }

    let product1Id: string;
    const prod1Check = await pool.query(`SELECT id FROM products WHERE location_id = $1 AND name = 'Margherita'`, [realDemoLocId]);
    if (prod1Check.rowCount && prod1Check.rowCount > 0) {
      product1Id = prod1Check.rows[0].id;
    } else {
      product1Id = randomUUID();
      await pool.query(`INSERT INTO products (id, location_id, category_id, name, price) VALUES ($1, $2, $3, $4, $5)`, [product1Id, realDemoLocId, categoryId, 'Margherita', 1200]);
    }

    let product2Id: string;
    const prod2Check = await pool.query(`SELECT id FROM products WHERE location_id = $1 AND name = 'Pepperoni'`, [realDemoLocId]);
    if (prod2Check.rowCount && prod2Check.rowCount > 0) {
      product2Id = prod2Check.rows[0].id;
    } else {
      product2Id = randomUUID();
      await pool.query(`INSERT INTO products (id, location_id, category_id, name, price) VALUES ($1, $2, $3, $4, $5)`, [product2Id, realDemoLocId, categoryId, 'Pepperoni', 1500]);
    }

    // Modifiers — clean existing data to avoid FK conflicts
    await pool.query(`DELETE FROM product_modifier_groups WHERE product_id IN (SELECT id FROM products WHERE location_id = $1)`, [realDemoLocId]);
    await pool.query(`DELETE FROM modifier_groups WHERE location_id = $1`, [realDemoLocId]); // CASCADE to modifiers
    const modGrp1Res = await pool.query(
      `INSERT INTO modifier_groups (location_id, name, min_select, max_select, required)
       VALUES ($1, $2, 1, 1, true) RETURNING id`,
      [realDemoLocId, 'Розмір']
    );
    const modGrp1Id = modGrp1Res.rows[0].id;

    await pool.query(
      `INSERT INTO modifiers (group_id, location_id, name, price_delta) VALUES ($1, $2, $3, $4)`,
      [modGrp1Id, realDemoLocId, 'Мала', 0]
    );
    await pool.query(
      `INSERT INTO modifiers (group_id, location_id, name, price_delta) VALUES ($1, $2, $3, $4)`,
      [modGrp1Id, realDemoLocId, 'Велика', 300]
    );

    const modGrp2Res = await pool.query(
      `INSERT INTO modifier_groups (location_id, name, min_select, max_select, required)
       VALUES ($1, $2, 0, 3, false) RETURNING id`,
      [realDemoLocId, 'Додатки']
    );
    const modGrp2Id = modGrp2Res.rows[0].id;

    await pool.query(
      `INSERT INTO modifiers (group_id, location_id, name, price_delta) VALUES ($1, $2, $3, $4)`,
      [modGrp2Id, realDemoLocId, 'Сирок', 200]
    );
    await pool.query(
      `INSERT INTO modifiers (group_id, location_id, name, price_delta) VALUES ($1, $2, $3, $4)`,
      [modGrp2Id, realDemoLocId, 'Гриби', 150]
    );

    await pool.query(
      `INSERT INTO product_modifier_groups (product_id, group_id, location_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING`,
      [product1Id, modGrp1Id, realDemoLocId]
    );
    await pool.query(
      `INSERT INTO product_modifier_groups (product_id, group_id, location_id) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING`,
      [product1Id, modGrp2Id, realDemoLocId]
    );

    // Translations — Albanian + English
    await pool.query(
      `INSERT INTO product_translations (product_id, locale, name, description)
       VALUES ($1, 'en', 'Margherita', 'Tomato sauce, mozzarella')
       ON CONFLICT (product_id, locale) DO UPDATE SET name = EXCLUDED.name, description = EXCLUDED.description`,
      [product1Id]
    );
    await pool.query(
      `INSERT INTO product_translations (product_id, locale, name, description)
       VALUES ($1, 'sq', 'Margherita', 'Salce domate, mocarela')
       ON CONFLICT (product_id, locale) DO UPDATE SET name = EXCLUDED.name, description = EXCLUDED.description`,
      [product1Id]
    );
    await pool.query(
      `INSERT INTO product_translations (product_id, locale, name)
       VALUES ($1, 'sq', 'Pepperoni')
       ON CONFLICT (product_id, locale) DO UPDATE SET name = EXCLUDED.name`,
      [product2Id]
    );

    await pool.query(
      `INSERT INTO category_translations (category_id, locale, name)
       VALUES ($1, 'en', 'Pizzas')
       ON CONFLICT (category_id, locale) DO UPDATE SET name = EXCLUDED.name`,
      [categoryId]
    );
    await pool.query(
      `INSERT INTO category_translations (category_id, locale, name)
       VALUES ($1, 'sq', 'Picat')
       ON CONFLICT (category_id, locale) DO UPDATE SET name = EXCLUDED.name`,
      [categoryId]
    );

    // Delivery Tiers
    await pool.query(`DELETE FROM delivery_tiers WHERE location_id = $1`, [realDemoLocId]);
    await pool.query(
      `INSERT INTO delivery_tiers (location_id, max_distance_km, fee)
       VALUES ($1, 5, 200)`,
      [realDemoLocId]
    );

    // Theme — ensure demo location has theme data
    const themeCheck = await pool.query(`SELECT 1 FROM location_themes WHERE location_id = $1`, [realDemoLocId]);
    if (themeCheck.rowCount === 0) {
      await pool.query(`INSERT INTO location_themes (location_id, frame_ancestors) VALUES ($1, ARRAY['*'])`, [realDemoLocId]);
    }

    const themeVerCheck = await pool.query(`SELECT 1 FROM theme_versions WHERE location_id = $1`, [realDemoLocId]);
    if (themeVerCheck.rowCount === 0) {
      await pool.query(
        `INSERT INTO theme_versions (location_id, version, css_hash, css_body)
         VALUES ($1, 1, md5(random()::text),
         E':root{--brand-primary:#ea4f16;--brand-primary-hover:#ffa12e;--brand-bg:#121212;--brand-surface:#1e1e1e;--brand-text:#ffffff;--brand-text-muted:#a8a8a8;--brand-border:#2c2c2c;--brand-radius:12px;--color-success:#059669;--color-warning:#D97706;--color-danger:#DC2626;--color-info:#2563EB}')`,
        [realDemoLocId]
      );
    }

    // Menu version
    await pool.query(
      `INSERT INTO menu_versions (location_id, version)
       VALUES ($1, 1)
       ON CONFLICT (location_id) DO NOTHING`,
      [realDemoLocId]
    );

    // Reservations
    await pool.query(`DELETE FROM reservations WHERE location_id = $1`, [realDemoLocId]);
    await pool.query(
      `INSERT INTO reservations (location_id, slot_at, party_size)
       VALUES ($1, now() + interval '1 day', 4)`,
      [realDemoLocId]
    );

    // Order history (attempt to get one order)
    const orderRes = await pool.query(`SELECT id FROM orders WHERE location_id = $1 LIMIT 1`, [realDemoLocId]);
    if (orderRes.rowCount && orderRes.rowCount > 0) {
      await pool.query(
        `INSERT INTO order_status_history (order_id, location_id, to_status, actor)
         VALUES ($1, $2, 'PENDING', 'system')`,
        [orderRes.rows[0].id, realDemoLocId]
      );
    }

    await pool.query('COMMIT');

    console.log('✅ Seed completed successfully.');
    console.log('UUIDs for verify-rls:');
    console.log(`OWNER_A_ID: ${ownerARealId}`);
    console.log(`OWNER_B_ID: ${ownerBRealId}`);
    console.log(`COURIER_ID: ${courierRealId}`);
    console.log(`DEMO_LOCATION_ID: ${realDemoLocId}`);
    console.log(`DEMO2_LOCATION_ID: ${realDemo2LocId}`);

  } catch (err) {
    await pool.query('ROLLBACK');
    console.error('❌ Seed failed:', err);
    process.exit(1);
  } finally {
    await pool.end();
  }
}

run();
