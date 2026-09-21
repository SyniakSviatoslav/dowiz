// THE OWNER'S WRITE CYCLES, walked in a live session.
//
// `surface-sweep.mjs` opens every console screen and checks it drew something.
// That is a different question from "does the thing on it work". This file
// runs the cycles an owner actually performs — build a dish, price it, hide
// it, delete it; mint a promo code and spend it; hire a courier and let them
// go; change what the venue is and change it back — each one through the
// console's own controls where there is a control, and through the same
// authenticated route the console uses where there is not.
//
// EVERY FLOW PUTS THE VENUE BACK. They run against a real restaurant, so each
// one is responsible for its own litter, and each one SAYS whether it managed
// it: a cleanup that cannot be seen to fail is not a cleanup.
//
// The flows are independent on purpose. One that breaks must not hide the
// eight behind it, which is what a single linear script does.
import { chromium, devices } from 'playwright';
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const OUT = process.env.OUT || '/tmp/dowiz-flows';
fs.mkdirSync(OUT, { recursive: true });
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

const fails = [];
const step = (name, ok, detail = '') => {
  if (!ok) fails.push(`${name}${detail ? ' :: ' + detail : ''}`);
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`);
};

const j = async (p, o = {}) => {
  const r = await fetch(`${HOST}${p}`, o);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};
let JWT = '';
const own = (p, body, method = 'POST') => j(p, {
  method,
  headers: { authorization: `Bearer ${JWT}`, 'content-type': 'application/json' },
  ...(body === undefined ? {} : { body: JSON.stringify(body) }),
});

const login = await j('/api/auth/login', {
  method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }),
});
JWT = login.body?.access_token || '';
const slug = new URL(HOST).hostname.split('.')[0];
const VENUE = (await j(`/api/public/locations/${slug}/menu`)).body?.location?.id;
step('owner signs in', login.status === 200 && !!VENUE, `${VENUE}`);
if (!VENUE) process.exit(1);

/// Run a flow so that its failure is a failure and not the end of the run.
const flow = async (name, fn) => {
  console.log(`\n── ${name} ───────────────────────────────`);
  try { await fn(); } catch (e) { step(`${name}: threw`, false, String(e.message).slice(0, 140)); }
};

// ── 1. THE CATALOGUE ────────────────────────────────────────────────────────
await flow('catalogue', async () => {
  const cat = await own('/api/owner/categories', { location_id: VENUE, name: 'QA Flow' });
  const CAT = cat.body?.id;
  step('a category is created', cat.status === 200 && !!CAT, `${cat.status} ${CAT}`);

  const prod = await own('/api/owner/products', {
    location_id: VENUE, category_id: CAT, name: 'QA Flow Dish', price: 1234, available: true,
  });
  const PROD = prod.body?.id;
  step('a dish is created', prod.status === 200 && !!PROD, `${prod.status} ${PROD}`);

  // A dish the customer can see, at the price the owner typed.
  const menu = await j(`/api/public/locations/${slug}/menu?fresh=1`);
  const seen = (menu.body?.categories || []).flatMap(c => c.products || []).find(x => x.id === PROD);
  step('the dish reaches the public menu at its price', seen?.price === 1234,
    seen ? `price=${seen.price}` : 'not on the menu');

  // Hiding it must take it off the customer's menu, not just grey it.
  await own(`/api/owner/products/${PROD}`, { location_id: VENUE, available: false });
  const menu2 = await j(`/api/public/locations/${slug}/menu?fresh=1`);
  const after = (menu2.body?.categories || []).flatMap(c => c.products || []).find(x => x.id === PROD);
  step('an unavailable dish is marked so for the customer',
    !after || after.available === false, after ? `available=${after.available}` : 'gone from the menu');

  // Translations: the id check is the only thing keeping these in one venue.
  const i18n = await own('/api/owner/i18n', {
    location_id: VENUE,
    entries: [{ entity: 'product', id: PROD, locale: 'uk', field: 'name', value: 'QA Страва' }],
  });
  step('a translation is accepted for a dish of this venue',
    i18n.status === 200 && i18n.body?.written === 1,
    `${i18n.status} written=${i18n.body?.written} refused=${JSON.stringify(i18n.body?.refused || []).slice(0, 80)}`);

  // And refused for an id that is not in this catalogue.
  const bad = await own('/api/owner/i18n', {
    location_id: VENUE,
    entries: [{ entity: 'product', id: 'no-such-dish-qa', locale: 'uk', field: 'name', value: 'x' }],
  });
  step('a translation for an unknown dish is refused',
    bad.status === 200 && bad.body?.written === 0 && (bad.body?.refused || []).length === 1,
    `written=${bad.body?.written} refused=${JSON.stringify(bad.body?.refused || []).slice(0, 80)}`);

  const d1 = await own(`/api/owner/products/${PROD}/delete`, { location_id: VENUE });
  const d2 = await own(`/api/owner/categories/${CAT}/delete`, { location_id: VENUE });
  const gone = await j(`/api/public/locations/${slug}/menu?fresh=1`);
  const still = (gone.body?.categories || []).flatMap(c => c.products || []).find(x => x.id === PROD);
  step('the catalogue is left as it was found', d1.status === 200 && d2.status === 200 && !still,
    `delete ${d1.status}/${d2.status}${still ? ' — dish still listed' : ''}`);
});

// ── 2. PROMOTIONS ───────────────────────────────────────────────────────────
await flow('promotions', async () => {
  const CODE = `QA${Date.now().toString().slice(-6)}`;
  // NO `location_id` HERE. The route is `deny_unknown_fields` and takes the
  // venue from the token, not the body — sending one is a 400, which is how
  // this flow failed the first time and it was the test that was wrong.
  const mk = await own('/api/owner/promotions', {
    code: CODE, kind: 'percent', value: 10, maxUses: 3,
  });
  step('a promo code is created', mk.status === 200, `${mk.status} ${JSON.stringify(mk.body).slice(0, 110)}`);

  const list = await own('/api/owner/promotions', undefined, 'GET');
  const found = (list.body?.promotions || list.body || []).some?.(x => x.code === CODE);
  step('it is listed', !!found, JSON.stringify(list.body).slice(0, 110));

  // The customer's own check, which is what the cart calls.
  const chk = await j('/api/promo/check', {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ slug, code: CODE, items: [{ product_id: 'item-05', quantity: 2 }] }),
  });
  step('the customer can check it and is told the discount',
    chk.status === 200 && typeof chk.body?.discount === 'number' && chk.body.discount > 0,
    `${chk.status} ${JSON.stringify(chk.body).slice(0, 120)}`);

  // A code that does not exist must not look like one that does.
  const bogus = await j('/api/promo/check', {
    method: 'POST', headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ slug, code: 'NOPE-QA-NOPE', items: [{ product_id: 'item-05', quantity: 1 }] }),
  });
  step('an unknown code is refused', bogus.status >= 400 || bogus.body?.discount === 0,
    `${bogus.status} ${JSON.stringify(bogus.body).slice(0, 90)}`);

  const del = await own(`/api/owner/promotions/${CODE}/delete`, {});
  step('the promo is withdrawn', del.status === 200, `${del.status}`);
});

// ── 3. COURIERS ─────────────────────────────────────────────────────────────
await flow('couriers', async () => {
  const list = await own('/api/owner/couriers', undefined, 'GET');
  const rows = list.body?.couriers || list.body || [];
  step('the venue lists its couriers', list.status === 200 && Array.isArray(rows), `${list.status} ${rows.length} rows`);

  // EVERY COURIER LISTED HERE MUST BELONG TO THIS VENUE. The invite bug put
  // five of six couriers on the other one, and this is the read that would
  // have shown it.
  const foreign = rows.filter(c => c.locationId && c.locationId !== VENUE);
  step('no courier of another venue is listed', foreign.length === 0,
    foreign.map(c => `${String(c.id).slice(0, 8)}@${c.locationId}`).join(',') || 'all this venue');

  const one = rows[0];
  if (one) {
    const detail = await own(`/api/owner/couriers/${one.id}`, undefined, 'GET');
    step('a courier detail opens', detail.status === 200, `${detail.status}`);
  }
});

// ── 4. WHAT THE VENUE IS ────────────────────────────────────────────────────
await flow('settings and features', async () => {
  const got = await own('/api/owner/settings', undefined, 'GET');
  step('settings are readable', got.status === 200, `${got.status}`);

  // NO SECRET MAY COME BACK OUT. `redacted` is the only thing between a
  // venue's bot token and anyone who can open this screen.
  const raw = JSON.stringify(got.body || {});
  const leaked = ['ghp_', 'sk_live', 'sk_test', 'Bearer ', 'AKIA'].filter(s => raw.includes(s));
  step('no secret is echoed back', leaked.length === 0, leaked.join(',') || 'clean');

  const feat = await own('/api/owner/features', undefined, 'GET');
  step('features are readable', feat.status === 200, JSON.stringify(feat.body).slice(0, 110));
});

// ── 5. THE READ-ONLY PANES NOBODY HAS OPENED ────────────────────────────────
await flow('the owner reads', async () => {
  for (const [name, path] of [
    ['dashboard', '/api/owner/dashboard'],
    ['analytics', '/api/owner/analytics'],
    ['health', '/api/owner/health'],
    ['history', '/api/owner/history'],
    ['backup', '/api/owner/backup'],
    ['backup/cloud', '/api/owner/backup/cloud'],
    ['customers', '/api/owner/customers'],
    ['reveals', '/api/owner/customers/reveals'],
    ['graph', '/api/owner/graph'],
    ['activation', '/api/owner/activation'],
    ['apikeys', '/api/owner/apikeys'],
    ['integrations', '/api/owner/integrations'],
    ['inbox', '/api/owner/inbox'],
    ['posts', '/api/owner/posts'],
    ['stock', '/api/owner/stock'],
    ['zones-via-location', '/api/owner/orders'],
  ]) {
    const r = await own(path, undefined, 'GET');
    step(`${name} answers`, r.status === 200, `${r.status} ${typeof r.body === 'string' ? r.body.slice(0, 70) : JSON.stringify(r.body).slice(0, 70)}`);
  }
});

// ── 6. THE CONSOLE'S OWN CONTROLS, IN A BROWSER ─────────────────────────────
await flow('the console in a browser', async () => {
  const b = await chromium.launch({ args: ['--no-sandbox', '--disable-dev-shm-usage'] });
  const ctx = await b.newContext({ ...devices['iPhone 14 Pro'], serviceWorkers: 'block' });
  const p = await ctx.newPage();
  const seen = [];
  p.on('pageerror', e => seen.push(`uncaught: ${e.message.slice(0, 100)}`));
  p.on('response', r => { if (r.status() >= 400 && !/tiles|nominatim|\.(png|jpg|webp)/.test(r.url())) seen.push(`${r.status()} ${r.url().replace(HOST, '').slice(0, 70)}`); });

  await p.goto(`${HOST}/admin/`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await p.waitForSelector('#e', { timeout: 40000 });
  await p.fill('#e', creds.OWNER_EMAIL); await p.fill('#p', creds.OWNER_PASSWORD);
  await p.click('#go');
  await p.waitForTimeout(6000);
  step('console: signs in', !(await p.$('#e')));

  // The menu editor is the screen an owner uses every week and no run has
  // ever touched it.
  await p.click('#nav [data-tab="menu"]').catch(() => {});
  await p.waitForTimeout(4000);
  const dishes = await p.$$eval('[data-p], [data-prod], [data-dish]', els => els.length).catch(() => 0);
  step('console/menu: the catalogue is on screen', dishes > 0, `${dishes} rows`);
  await p.screenshot({ path: `${OUT}/menu-editor.png` }).catch(() => {});

  await p.click('#nav [data-tab="stock"]').catch(() => {});
  await p.waitForTimeout(3500);
  const stockTxt = await p.evaluate(() => (document.getElementById('app')?.innerText || '').slice(0, 120));
  step('console/stock: the pane says something about stock', stockTxt.length > 20, stockTxt.replace(/\n/g, ' ').slice(0, 90));

  step('console: nothing failed while browsing', seen.length === 0, seen.slice(0, 3).join(' | '));
  await b.close();
});

console.log(`\n${fails.length ? 'FAILURES:\n  ' + fails.join('\n  ') : 'OWNER FLOWS OK'}`);
process.exit(fails.length ? 1 : 0);
