import pg from 'pg';
const PROD = new pg.Client({ connectionString: process.env.DATABASE_URL_SESSION, ssl: { rejectUnauthorized: false } });
const STG  = new pg.Client({ connectionString: process.env.STAGING_DB_URL });
await PROD.connect(); await STG.connect();

// 1. READ prod sushi-durres — ALLOW-LIST: locations(config)+categories+products only.
//    NEVER read customers/orders. prodPhone is read ONLY to prove the scrub (never written).
const prodLoc = (await PROD.query(`SELECT id, name, phone FROM locations WHERE slug='sushi-durres'`)).rows[0];
if (!prodLoc) throw new Error('sushi-durres not found on prod');
const prodPhone = prodLoc.phone;
const cats  = (await PROD.query(`SELECT id, name, sort_order FROM categories WHERE location_id=$1 ORDER BY sort_order`, [prodLoc.id])).rows;
const prods = (await PROD.query(`SELECT id, category_id, name, description, price, is_available, image_url, sort_order FROM products WHERE location_id=$1 ORDER BY sort_order`, [prodLoc.id])).rows;
await PROD.end();
console.log(`[read] prod sushi-durres: ${cats.length} categories, ${prods.length} products (0 modifiers)`);

// 2. staging demo
const demo = (await STG.query(`SELECT id FROM locations WHERE slug='demo'`)).rows[0];
if (!demo) throw new Error('demo not found on staging');
const before = (await STG.query(
  `SELECT (SELECT count(*) FROM customers WHERE location_id=$1)::int c,
          (SELECT count(*) FROM orders    WHERE location_id=$1)::int o`, [demo.id])).rows[0];
console.log(`[staging] demo before: customers=${before.c} orders=${before.o}`);
// Pre-existing staging TEST orders/customers (not prod PII). Release the FK from
// historical order_items to the old products (name_snapshot/price_snapshot keep the
// order history intact), so the menu can be rebuilt without touching orders/customers.

const DEMO_PHONE = '+355690000000';
await STG.query('BEGIN');
// release historical FK refs (product_id is nullable; snapshots preserved)
await STG.query(`UPDATE order_items SET product_id=NULL WHERE product_id IN (SELECT id FROM products WHERE location_id=$1)`, [demo.id]);
// idempotent rebuild (products before categories for FK)
await STG.query(`DELETE FROM products   WHERE location_id=$1`, [demo.id]);
await STG.query(`DELETE FROM categories WHERE location_id=$1`, [demo.id]);
const catMap = new Map();
for (const cat of cats) {
  const r = await STG.query(`INSERT INTO categories (location_id, name, sort_order) VALUES ($1,$2,$3) RETURNING id`, [demo.id, cat.name, cat.sort_order]);
  catMap.set(cat.id, r.rows[0].id);
}
for (const p of prods) {
  await STG.query(
    `INSERT INTO products (location_id, category_id, name, description, price, is_available, image_url, sort_order)
     VALUES ($1,$2,$3,$4,$5,$6,$7,$8)`,
    [demo.id, p.category_id ? catMap.get(p.category_id) : null, p.name, p.description, p.price, p.is_available, p.image_url, p.sort_order]);
}
// scrub: label as demo, placeholder phone, demo-safe fallback_config (NEVER copy prod phone/fallback)
await STG.query(
  `UPDATE locations SET name=$2, phone=$3, public_phone=$3, fallback_config=$4::jsonb,
          menu_version = COALESCE(menu_version,0)+1 WHERE id=$1`,
  [demo.id, 'Dubin & Sushi (demo)', DEMO_PHONE, JSON.stringify({ phone: DEMO_PHONE, showPhoneOnError: true, showPhoneOnOffline: true })]);
await STG.query('COMMIT');

// 3. VERIFY
const after = (await STG.query(
  `SELECT (SELECT count(*) FROM customers  WHERE location_id=$1)::int c,
          (SELECT count(*) FROM orders     WHERE location_id=$1)::int o,
          (SELECT count(*) FROM categories WHERE location_id=$1)::int cat,
          (SELECT count(*) FROM products   WHERE location_id=$1)::int p,
          (SELECT phone FROM locations WHERE id=$1) phone,
          (SELECT fallback_config::text FROM locations WHERE id=$1) fc`, [demo.id])).rows[0];
await STG.end();
const fcLeak = !!(prodPhone && after.fc && after.fc.includes(prodPhone));
console.log(`[after] demo: categories=${after.cat} products=${after.p} customers=${after.c} orders=${after.o}`);
console.log('--- SCRUB PROOFS ---');
console.log(`customers delta (load brought 0 PII): ${after.c - before.c}  => ${after.c - before.c === 0 ? 'PASS' : 'FAIL'}`);
console.log(`orders delta (load brought 0 PII):    ${after.o - before.o}  => ${after.o - before.o === 0 ? 'PASS' : 'FAIL'}`);
console.log(`demo.phone !== prod.phone:            ${after.phone !== prodPhone ? 'PASS' : 'FAIL'} (demo=${after.phone})`);
console.log(`fallback_config has no prod phone:    ${!fcLeak ? 'PASS' : 'FAIL'}`);
console.log(`menu mirrors prod (cats ${after.cat}/${cats.length}, prods ${after.p}/${prods.length}): ${after.cat===cats.length && after.p===prods.length ? 'PASS' : 'FAIL'}`);
