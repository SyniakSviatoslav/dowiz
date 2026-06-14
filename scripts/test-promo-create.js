async function main() {
  const authRes = await fetch('https://dowiz.fly.dev/api/dev/mock-auth', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ role: 'owner', locationSlug: 'demo' })
  });
  const auth = await authRes.json();
  console.log('auth status:', authRes.status);
  
  const body = {
    code: 'TEST-PROMO-' + Date.now(),
    type: 'percentage',
    discount_value: 10,
    min_order_amount: 50000,
    valid_from: new Date().toISOString(),
    is_active: true,
    description: 'E2E test'
  };
  console.log('sending:', JSON.stringify(body));
  
  const res = await fetch('https://dowiz.fly.dev/api/owner/promotions', {
    method: 'POST',
    headers: { 'Authorization': 'Bearer ' + auth.access_token, 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
  });
  console.log('status:', res.status);
  const text = await res.text();
  console.log('response:', text.slice(0, 500));
}
main().catch(e => console.log('error:', e.message));
