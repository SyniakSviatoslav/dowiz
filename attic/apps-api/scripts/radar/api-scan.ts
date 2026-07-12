const BASE = 'https://dowiz.fly.dev';

async function main() {
  const results: { method: string; path: string; status: number; expected: string }[] = [];

  async function probe(method: string, path: string, opts?: any) {
    try {
      const res = await fetch(BASE + path, { method, ...opts });
      return res.status;
    } catch { return 0; }
  }

  function emit(method: string, path: string, status: number, expected: string) {
    const ok = expected === 'any' ? true
      : expected.startsWith('<') ? status < parseInt(expected.slice(1))
      : expected.startsWith('>') ? status > parseInt(expected.slice(1))
      : status === parseInt(expected);
    console.log(`${ok ? 'OK' : 'ISSUE'}|${method}|${path}|${status}|expected=${expected}`);
    results.push({ method, path, status, expected });
  }

  // Auth
  const authRes = await fetch(BASE + '/api/dev/mock-auth', { method: 'POST' });
  const auth = await authRes.json();
  const token = auth.access_token;
  const ctHeaders = { 'Content-Type': 'application/json' };
  const authHeaders = { Authorization: 'Bearer ' + token, 'Content-Type': 'application/json' };
  const ownerHeaders = { Authorization: 'Bearer ' + token, 'Content-Type': 'application/json' };

  // PUBLIC
  emit('GET', '/health', await probe('GET', '/health'), '200');
  emit('GET', '/public/locations/demo/info', await probe('GET', '/public/locations/demo/info'), '200');
  emit('GET', '/public/theme/demo', await probe('GET', '/public/theme/demo'), '200');
  emit('GET', '/public/locations/demo/menu', await probe('GET', '/public/locations/demo/menu'), '200');

  // AUTH
  emit('POST', '/api/auth/local/login', await probe('POST', '/api/auth/local/login', { headers: ctHeaders, body: JSON.stringify({email:'test@dowiz.com', password:'test123456'}) }), '200');

  // OWNER
  emit('GET', '/api/owner/settings', await probe('GET', '/api/owner/settings', { headers: ownerHeaders }), '200');
  emit('GET', '/api/owner/orders', await probe('GET', '/api/owner/orders', { headers: ownerHeaders }), '200');
  emit('GET', '/api/owner/menu/categories', await probe('GET', '/api/owner/menu/categories', { headers: ownerHeaders }), '200');
  emit('GET', '/api/owner/couriers', await probe('GET', '/api/owner/couriers', { headers: ownerHeaders }), '200');
  emit('GET', '/api/owner/brand', await probe('GET', '/api/owner/brand', { headers: ownerHeaders }), '200');
  emit('GET', '/api/owner/analytics', await probe('GET', '/api/owner/analytics', { headers: ownerHeaders }), '200');

  const locId = '1f609add-062a-4bb5-89bf-d695f963ede6';
  emit('GET', `/api/owner/locations/${locId}/dashboard/snapshot`, await probe('GET', `/api/owner/locations/${locId}/dashboard/snapshot`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/alerts`, await probe('GET', `/api/owner/locations/${locId}/alerts`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/signals`, await probe('GET', `/api/owner/locations/${locId}/signals`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/settlements`, await probe('GET', `/api/owner/locations/${locId}/settlements`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/settings/dwell`, await probe('GET', `/api/owner/locations/${locId}/settings/dwell`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/settings/fallback`, await probe('GET', `/api/owner/locations/${locId}/settings/fallback`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/settings/retention`, await probe('GET', `/api/owner/locations/${locId}/settings/retention`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/notifications/targets`, await probe('GET', `/api/owner/locations/${locId}/notifications/targets`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/push/state`, await probe('GET', `/api/owner/locations/${locId}/push/state`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/couriers/live`, await probe('GET', `/api/owner/locations/${locId}/couriers/live`, { headers: ownerHeaders }), '200');
  emit('GET', `/api/owner/locations/${locId}/courier-invites`, await probe('GET', `/api/owner/locations/${locId}/courier-invites`, { headers: ownerHeaders }), '200');

  // COURIER (should be denied with owner token)
  emit('GET', '/api/courier/me/assignments', await probe('GET', '/api/courier/me/assignments', { headers: ownerHeaders }), '403');
  emit('GET', '/api/courier/me/shift', await probe('GET', '/api/courier/me/shift', { headers: ownerHeaders }), '403');
  emit('GET', '/api/courier/me/earnings', await probe('GET', '/api/courier/me/earnings', { headers: ownerHeaders }), '403');

  // CUSTOMER (should be denied with owner token)
  emit('GET', '/api/customer/orders/some-id/status', await probe('GET', '/api/customer/orders/some-id/status', { headers: ownerHeaders }), '403');

  // ADMIN
  emit('GET', '/api/admin/backups', await probe('GET', '/api/admin/backups', { headers: ownerHeaders }), '200');
  emit('GET', '/api/admin/backups/dr-report', await probe('GET', '/api/admin/backups/dr-report', { headers: ownerHeaders }), '200');
  emit('GET', '/api/admin/fallback/health', await probe('GET', '/api/admin/fallback/health', { headers: ownerHeaders }), '200');

  // SUMMARY
  const issues = results.filter(r => {
    if (r.expected === 'any') return false;
    if (r.expected.startsWith('<')) return !(r.status < parseInt(r.expected.slice(1)));
    if (r.expected.startsWith('>')) return !(r.status > parseInt(r.expected.slice(1)));
    return r.status !== parseInt(r.expected);
  });
  console.log('\n=== SUMMARY ===');
  console.log(`Total endpoints: ${results.length}`);
  console.log(`OK: ${results.length - issues.length}`);
  console.log(`Issues: ${issues.length}`);
  for (const i of issues) {
    console.log(`ISSUE: ${i.method} ${i.path} — got ${i.status}, expected ${i.expected}`);
  }
}

main().catch(e => console.error('FATAL', e));
