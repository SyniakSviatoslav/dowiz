#!/usr/bin/env node
// Provision a REAL shadow tenant on staging and mint a claim invite bound to the mock-auth dev owner
// (dev@deliveryos.com), so the §6 claim happy-path E2E (flow-simpl-s6-claim.spec.ts) can run green.
//
// WHY THIS IS NOT PURELY HTTP: the /internal/acquisition ops API can only walk SOURCED→ENRICHED via
// /acquisition/extract, which scrapes + AI-parses a LIVE website (external, flaky) — wrong dependency
// for a deterministic claim E2E. So we seed the ENRICHED state + a tiny menu_draft directly in the DB
// (faithful to the module's advance()/MenuDraft; exactly what provision-rls.test.ts does), then drive
// the real claim chain over HTTP. The shadow MUST carry items or the P6-6 ProvisionVerifier rejects it
// (markVerified → NOT_VERIFIABLE on an empty preview).
//
// Prereqs:
//   1. PROVISION_OPS_SECRET = the staging value (write-only on Fly — operator-provided or rotated).
//   2. A DB tunnel to staging:  flyctl proxy 5433:5432 -a dowiz-staging-db
//      DATABASE_URL = the staging migrations URL rewritten to @localhost:5433 (do NOT hardcode creds).
//
// Usage:
//   PROVISION_OPS_SECRET=… BASE=https://dowiz-staging.fly.dev DATABASE_URL=postgres://…@localhost:5433/… \
//     node e2e/scripts/provision-claim-shadow.mjs
// Then the printed:
//   E2E_CLAIM_TOKEN=… VITE_BASE_URL=… DEV_AUTH_SECRET=stg-e2e-secret \
//     pnpm exec playwright test e2e/tests/flow-simpl-s6-claim.spec.ts --project=desktop --reporter=list
// (single-use token → one viewport per token; mint a fresh one per run.)
import crypto from 'node:crypto';
import pg from 'pg';

const BASE = process.env.BASE || 'https://dowiz-staging.fly.dev';
const SECRET = process.env.PROVISION_OPS_SECRET;
const DB = process.env.DATABASE_URL;
const CONTACT = process.env.CLAIM_CONTACT || 'dev@deliveryos.com'; // must match the mock-auth owner email
if (!SECRET) { console.error('PROVISION_OPS_SECRET is required (staging value; write-only on Fly).'); process.exit(2); }
if (!DB) { console.error('DATABASE_URL is required (staging via `flyctl proxy 5433:5432 -a dowiz-staging-db`).'); process.exit(2); }

const headers = { 'content-type': 'application/json', 'x-provision-ops-secret': SECRET };
async function call(path, body) {
  const res = await fetch(`${BASE}/internal${path}`, { method: 'POST', headers, body: JSON.stringify(body) });
  const json = await res.json().catch(() => ({}));
  if (res.status === 404) { console.error(`\n❌ ${path} → 404 — ops secret wrong (fail-closed).`); process.exit(1); }
  if (res.status >= 400) { console.error(`\n❌ ${path} → ${res.status}: ${JSON.stringify(json)}`); process.exit(1); }
  return json;
}

const tag = crypto.randomBytes(3).toString('hex');
const slug = `e2e-claim-${tag}`;

console.log(`→ 1/6 acquisition (place_id=${slug})`);
const src = await call('/acquisition', { place_id: slug });
const sourceId = src.id ?? src.acquisition_source_id;

console.log('→ 2/6 DB walk SOURCED→PLACE_INGESTED→MENU_EXTRACTED→ENRICHED (+ seed menu_draft)');
const draft = { categories: [{ name: 'Pizzas', sort_order: 0, products: [{ name: 'Margherita', price: 1200, sort_order: 0 }] }] };
const client = new pg.Client({ connectionString: DB, ssl: /sslmode=disable/.test(DB) ? false : { rejectUnauthorized: false } });
await client.connect();
try {
  const walk = async (from, to, extra = '') =>
    client.query(`UPDATE acquisition_sources SET state=$2${extra}, updated_at=now() WHERE id=$1 AND state=$3`, [sourceId, to, from]);
  await walk('SOURCED', 'PLACE_INGESTED');
  await walk('PLACE_INGESTED', 'MENU_EXTRACTED');
  await client.query(
    `UPDATE acquisition_sources SET state='ENRICHED', menu_draft=$2::jsonb, updated_at=now() WHERE id=$1 AND state='MENU_EXTRACTED'`,
    [sourceId, JSON.stringify(draft)],
  );
  const { rows } = await client.query('SELECT state FROM acquisition_sources WHERE id=$1', [sourceId]);
  if (rows[0]?.state !== 'ENRICHED') { console.error(`state walk failed: ${rows[0]?.state}`); process.exit(1); }
} finally { await client.end(); }

console.log('→ 3/6 provision/mint');
const mint = await call('/acquisition/provision/mint', { acquisition_source_id: sourceId });
console.log('→ 4/6 provision/spine (writes the menu from the draft)');
const spine = await call('/acquisition/provision/spine', { acquisition_source_id: sourceId, token: mint.token, name: `E2E Claim ${tag}`, slug, phone: '+355690000000' });
console.log(`   location_id=${spine.location_id}`);
console.log('→ 5/6 claim/verify (ProvisionVerifier — preview must have items)');
await call('/acquisition/claim/verify', { acquisition_source_id: sourceId });
console.log(`→ 6/6 claim/mint (invited_contact=${CONTACT})`);
const claim = await call('/acquisition/claim/mint', { acquisition_source_id: sourceId, invited_contact: CONTACT, base_url: BASE });

console.log('\n✅ shadow provisioned + verified + claim invite minted. Run the happy-path:\n');
console.log(`E2E_CLAIM_TOKEN=${claim.token} \\`);
console.log(`  VITE_BASE_URL=${BASE} DEV_AUTH_SECRET=stg-e2e-secret \\`);
console.log('  pnpm exec playwright test e2e/tests/flow-simpl-s6-claim.spec.ts --project=desktop --reporter=list');
console.log(`\ncleanup: curl -s ${BASE}/internal/acquisition/provision/hard-delete -XPOST -H 'content-type: application/json' -H 'x-provision-ops-secret: …' -d '{"acquisition_source_id":"${sourceId}"}'`);
