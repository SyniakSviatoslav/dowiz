const BASE = 'https://dowiz.fly.dev';

async function main() {
  // Theme endpoint
  let r = await fetch(BASE + '/api/public/theme/demo');
  console.log('GET /api/public/theme/demo ->', r.status, r.ok ? (await r.text()).substring(0, 100) : await r.text().then(t => t?.substring(0,200)).catch(() => ''));

  // Settlements endpoint (with auth)
  const auth = await (await fetch(BASE + '/api/dev/mock-auth', { method: 'POST' })).json();
  const headers = { Authorization: 'Bearer ' + auth.access_token };
  r = await fetch(BASE + '/api/owner/locations/1f609add-062a-4bb5-89bf-d695f963ede6/settlements', { headers });
  console.log('GET /settlements ->', r.status, r.ok ? 'OK' : await r.text().then(t => t?.substring(0,500)).catch(() => ''));
}

main().catch(e => console.error(e));
