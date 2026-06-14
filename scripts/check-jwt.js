async function main() {
  const authRes = await fetch('https://dowiz.fly.dev/api/dev/mock-auth', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ role: 'owner', locationSlug: 'demo' })
  });
  const auth = await authRes.json();
  const payload = JSON.parse(Buffer.from(auth.access_token.split('.')[1], 'base64url').toString());
  console.log('JWT payload:', JSON.stringify(payload, null, 2));
  console.log('Has activeLocationId:', 'activeLocationId' in payload);
  console.log('activeLocationId value:', payload.activeLocationId);
}
main().catch(e => console.log('error:', e.message));
