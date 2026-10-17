// SEEDS A LOCAL qa-durres HUB (tools/ci/local-qa-hub.sh): the venue, its first owner, and one dish on sale.
//
//   QA_HUB_HOST=http://127.0.0.1:8787 BOOTSTRAP_SECRET=... OWNER_EMAIL=... OWNER_PASSWORD=... \
//     node tools/ci/qa-hub-seed.mjs
//
// The same bootstrap door as e2e/walk/qa-setup.mjs, trimmed to what the archived-order check
// needs: a venue the storefront answers for, an owner who can sign in to it, and one dish that is
// on sale. Exit 0 when all of that holds, 1 otherwise. Prints each step; never prints a secret.
//
// THE FIRST BOOTSTRAP LANDS IN THE WRONG OBJECT (see e2e/walk/qa-setup.mjs): for a slug the
// registry does not know yet, the catalogue goes to the UNNAMED_VENUE object, and the registry row
// is written too. A second identical call reaches the venue's own object, so this retries up to
// three times until the storefront answers for its slug.

const HOST = process.env.QA_HUB_HOST;
const LOC = 'qa-durres';
const SECRET = process.env.BOOTSTRAP_SECRET;
const EMAIL = process.env.OWNER_EMAIL;
const PASSWORD = process.env.OWNER_PASSWORD;
if (!HOST || !SECRET || !EMAIL || !PASSWORD) {
  console.log('qa-hub-seed: FAIL QA_HUB_HOST, BOOTSTRAP_SECRET, OWNER_EMAIL and OWNER_PASSWORD must all be set');
  process.exit(2);
}

const UA = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36';

async function call(path, { method = 'GET', body, token, headers = {} } = {}) {
  const r = await fetch(`${HOST}${path}`, {
    method,
    headers: { 'content-type': 'application/json', 'user-agent': UA, ...(token ? { authorization: `Bearer ${token}` } : {}), ...headers },
    body: body == null ? undefined : JSON.stringify(body),
  });
  const text = await r.text();
  let json; try { json = JSON.parse(text); } catch { json = null; }
  return { status: r.status, body: json, text };
}

let failed = 0;
function step(ok, what, detail = '') {
  console.log(`qa-hub-seed: ${ok ? 'ok  ' : 'FAIL'} ${what}${detail ? ` -- ${detail}` : ''}`);
  if (!ok) failed++;
}
const dishesOf = m => (m?.categories || []).flatMap(c => c.products || []);

const location = {
  id: LOC, slug: LOC, name: 'QA Durres', phone: '+355690000000', address: 'Local QA stand -- not a restaurant',
  status: 'open', closes_at: null, delivery_eta: '30-45', delivery_fee: 0, free_delivery_threshold: null,
  min_order: 0, currency_code: 'ALL', menu_version: 1, supported_locales: '["sq","en","uk"]',
  default_locale: 'en', delivery_paused: 0,
};
const owner = { email: EMAIL, password: PASSWORD, name: 'QA owner' };

async function main() {
  let pub = await call(`/api/public/locations/${LOC}/menu`);
  for (let i = 0; i < 3 && pub.status !== 200; i++) {
    const b = await call('/api/bootstrap', { method: 'POST', headers: { 'x-dowiz-bootstrap': SECRET }, body: { location, owner } });
    step(b.status === 200, `bootstrap try ${i + 1}`, `${b.status} ${b.text.slice(0, 120)}`);
    pub = await call(`/api/public/locations/${LOC}/menu`);
  }
  step(pub.status === 200 && pub.body?.location?.slug === LOC, 'the storefront menu answers for its slug', `${pub.status}`);

  const login = await call('/api/auth/login', { method: 'POST', body: { email: EMAIL, password: PASSWORD, location_id: LOC } });
  const token = login.body?.access_token;
  step(login.status === 200 && !!token, 'the owner signs in to the venue', `${login.status}`);

  if (token) {
    const cat = await call('/api/owner/categories', { method: 'POST', token, body: { location_id: LOC, id: 'qa-local', name: 'QA Local', sort_order: 0 } });
    step(cat.status === 200, 'category qa-local created', `${cat.status} ${cat.text.slice(0, 120)}`);
    // A dish goes on sale only once its allergens are declared (409 otherwise): off sale first.
    const made = await call('/api/owner/products', { method: 'POST', token, body: { location_id: LOC, category_id: 'qa-local', id: 'qa-local-dish', name: 'QA Local dish', price: 900, available: false } });
    step(made.status === 200, 'dish qa-local-dish created', `${made.status} ${made.text.slice(0, 120)}`);
    const sale = await call('/api/owner/products/qa-local-dish', { method: 'POST', token, body: { location_id: LOC, allergens: ['sesame'], available: true } });
    step(sale.status === 200, 'dish qa-local-dish declared and on sale', `${sale.status} ${sale.text.slice(0, 120)}`);
  }

  const menu = await call(`/api/public/locations/${LOC}/menu?fresh=1`);
  const onSale = dishesOf(menu.body).filter(d => d.available);
  step(menu.status === 200 && onSale.length >= 1, 'the storefront lists an available dish', `${menu.status} ${onSale.length} on sale`);
  console.log(failed ? `qa-hub-seed: RED (${failed} failed)` : 'qa-hub-seed: GREEN');
  return failed ? 1 : 0;
}

// A host that does not answer is a failed seed, said in the same words as any other failure.
main().then(
  code => process.exit(code),
  e => { console.log(`qa-hub-seed: FAIL the hub at ${HOST} did not answer: ${e.cause?.code || e.message}`); process.exit(1); },
);
