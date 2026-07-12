// Verifies the fresh-owner onboarding unblock + pickup toggle on a deployed env.
// Before the fix, POST /api/owner/onboarding 401'd a location-less owner (→ logout).
// Usage: BASE=https://dowiz-staging.fly.dev SECRET=stg-e2e-secret node scripts/verify-onboarding-unblock.mjs
const BASE = process.env.BASE || 'https://dowiz-staging.fly.dev';
const SECRET = process.env.SECRET || 'stg-e2e-secret';
const H = { 'content-type': 'application/json', 'x-dev-auth-secret': SECRET };
const results = [];
const ok = (name, cond, extra = '') => { results.push(!!cond); console.log(`${cond ? 'PASS' : 'FAIL'}: ${name}${extra ? ' — ' + extra : ''}`); };
const j = async (r) => { try { return await r.json(); } catch { return null; } };
const post = (p, body, auth) => fetch(`${BASE}${p}`, { method: 'POST', headers: auth ? { ...H, authorization: `Bearer ${auth}` } : H, body: JSON.stringify(body) });
const get = (p, auth) => fetch(`${BASE}${p}`, { headers: auth ? { ...H, authorization: `Bearer ${auth}` } : H });

const main = async () => {
  // Fresh owner: valid token, NO location yet (the case that used to 401).
  const owner = await j(await post('/api/dev/mock-auth', { fresh: true }));
  ok('mock-auth fresh owner', !!owner?.access_token);
  const tok = owner.access_token;
  const slug = 'qa-onb-' + Math.random().toString(36).slice(2, 8);

  // Wizard "Publish now" → POST /owner/onboarding. Must provision (200), not 401.
  const pub = await post('/api/owner/onboarding', {
    name: 'QA Diner', phone: '+355691234567', slug,
    lat: 41.331, lng: 19.817, delivery_radius_km: 3,
    menu_items: [{ name: 'Margherita', price: 800 }, { name: 'Espresso', price: 150 }],
    courier_option: 'skip', primary_color: '#ea4f16',
  }, tok);
  ok('POST /owner/onboarding provisions (was 401)', pub.status === 200, `HTTP ${pub.status}`);

  // The owner now has a location (no more location-less 401s on subsequent calls).
  const settings = await j(await get('/api/owner/settings', tok));
  const locId = settings?.id;
  ok('owner now has a provisioned location', !!locId, locId || '(none)');
  if (!locId) return finish();

  // Courier-invite mid-wizard no longer 401s a fresh owner (separate token).
  const fresh2 = await j(await post('/api/dev/mock-auth', { fresh: true }));
  const inv = await post('/api/owner/courier-invites', { phone: '+355690000000' }, fresh2.access_token);
  ok('courier-invite no longer 401s a fresh owner', inv.status === 200, `HTTP ${inv.status}`);

  // Pickup toggle: gate must accept pickup as the fulfillment path.
  const before = await j(await get(`/api/owner/activation/${locId}/status`, tok));
  const toggled = await j(await post(`/api/owner/activation/${locId}/pickup`, { enabled: true }, tok));
  ok('pickup toggle sets pickupEnabled', toggled?.pickupEnabled === true);
  ok('enabling pickup flips fulfillmentReady (phone present)', toggled?.gate?.fulfillmentReady === true,
     `before=${before?.gate?.fulfillmentReady} after=${toggled?.gate?.fulfillmentReady}`);

  finish();
};

const finish = () => {
  const passed = results.filter(Boolean).length;
  console.log(`\n${passed}/${results.length} checks passed`);
  process.exit(passed === results.length ? 0 : 1);
};

main().catch((e) => { console.error('ERROR', e?.message); process.exit(1); });
