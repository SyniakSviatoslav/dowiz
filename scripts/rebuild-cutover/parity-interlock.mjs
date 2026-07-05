// AR-2 (S2 cutover amendment RESOLVE) — the MECHANIZED verification-parity interlock.
//
// Proves the Node<->Rust JWT round-trip in BOTH directions with live requests before any
// authed surface (S2/S3/S4/S5/S7) is allowed to flip:
//   1. mint on Node  -> verify on Rust   (body-kid + strict claims + leeway 0 both sides)
//   2. mint on Rust  -> verify on Node
//   3. refresh a Node-minted family THROUGH Rust (gate-iv byte-identical rotation SQL)
//   4. refresh a Rust-minted family THROUGH Node
//   5. re-verify each rotated access token on the OPPOSITE stack
//
// GREEN (exit 0) is the precondition for setting `cutover_flags.readiness_ok=true` on any
// authed surface — readiness_ok IS "the flag the front-door reads; no flip while red"
// (front-door refuses target=rust while readiness_ok=false at read time).
//
// Run INSIDE the staging private network (the Rust app is flycast-only):
//   flyctl ssh console -a dowiz-staging -C "node -e '<this file base64-piped>'"
// or copy it to the box. Credentials come from PARITY_EMAIL/PARITY_PASSWORD (a test owner
// with an active owner membership); NEVER a real owner account.
//
// A verify arm PASSES on any non-401 (403/400 still prove the token VERIFIED — the parity
// question is signature/claims acceptance, not endpoint authorization).

const NODE = process.env.PARITY_NODE_BASE || 'https://dowiz-staging.fly.dev';
const RUST = process.env.PARITY_RUST_BASE || 'http://dowiz-rust-staging.flycast';
const EMAIL = process.env.PARITY_EMAIL || 'test@dowiz.com';
const PASS = process.env.PARITY_PASSWORD || 'test123456';

const out = [];
const j = async (r) => { try { return await r.json(); } catch { return {}; } };

async function login(base, label) {
  const r = await fetch(`${base}/api/auth/local/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: EMAIL, password: PASS }),
  });
  const b = await j(r);
  if (r.status !== 200 || !b.access_token || !b.refresh_token) {
    throw new Error(`${label} login ${r.status} ${JSON.stringify(b).slice(0, 200)}`);
  }
  out.push(`PASS mint ${label}: 200 (access+refresh)`);
  return b;
}

async function verify(base, token, label) {
  const r = await fetch(`${base}/api/owner/menu/products`, {
    headers: { authorization: `Bearer ${token}` },
  });
  const ok = r.status !== 401;
  out.push(`${ok ? 'PASS' : 'FAIL'} verify ${label}: ${r.status}`);
  return ok;
}

async function refresh(base, rt, label) {
  const r = await fetch(`${base}/api/auth/refresh`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ refresh_token: rt }),
  });
  const b = await j(r);
  const ok = r.status === 200 && !!b.access_token && !!b.refresh_token;
  out.push(`${ok ? 'PASS' : 'FAIL'} refresh ${label}: ${r.status}`);
  return ok ? b : null;
}

(async () => {
  let fails = 0;
  const n = await login(NODE, 'node');
  const ru = await login(RUST, 'rust');
  if (!(await verify(RUST, n.access_token, 'node-minted on RUST'))) fails++;
  if (!(await verify(NODE, ru.access_token, 'rust-minted on NODE'))) fails++;
  const nr = await refresh(RUST, n.refresh_token, 'node-family via RUST'); if (!nr) fails++;
  const rr = await refresh(NODE, ru.refresh_token, 'rust-family via NODE'); if (!rr) fails++;
  if (nr && !(await verify(NODE, nr.access_token, 'RUST-rotated node-family on NODE'))) fails++;
  if (rr && !(await verify(RUST, rr.access_token, 'NODE-rotated rust-family on RUST'))) fails++;
  console.log(out.join('\n'));
  console.log(fails === 0
    ? 'PARITY-INTERLOCK: GREEN (both directions incl. cross-stack rotation)'
    : `PARITY-INTERLOCK: RED (${fails} arm(s) failed) — readiness_ok must stay false`);
  process.exit(fails === 0 ? 0 : 1);
})().catch((e) => {
  console.log(out.join('\n'));
  console.log(`PARITY-INTERLOCK: RED (exception) ${e.message} — readiness_ok must stay false`);
  process.exit(1);
});
