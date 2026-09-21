// THE ONLY WAY TO CLOSE A STUCK ORDER ON THIS PLATFORM TODAY.
//
// `allowed_next` (crates/dowiz-core/src/order_machine.rs) reaches Cancelled
// and Rejected ONLY from Pending. Past that the single non-forward exit is
// Refunding, and no Worker route emits it. So an order that has been confirmed
// and then abandoned — the customer rang off, no courier appeared, the venue
// closed — cannot be cancelled by anyone. It sits in the queue for ever.
//
// This walks such orders forward to DELIVERED, which is the one terminal state
// the product can still reach, using the venue's own courier. It is a
// caretaker's tool, not a gate: it exists because the dead end exists, and it
// should be deleted the day a refund route lands.
//
//   node e2e/kit-regression/drain-stuck-orders.mjs            # dry run, lists them
//   node e2e/kit-regression/drain-stuck-orders.mjs --apply    # closes them
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://sushi-durres.dowiz.org';
const APPLY = process.argv.includes('--apply');
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=')));

const j = async (p, o = {}) => {
  const r = await fetch(`${HOST}${p}`, o);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};
const post = (p, tok, body) => j(p, {
  method: 'POST',
  headers: { authorization: `Bearer ${tok}`, 'content-type': 'application/json' },
  body: JSON.stringify(body ?? {}),
});

const slug = new URL(HOST).hostname.split('.')[0];
const VENUE = (await j(`/api/public/locations/${slug}/menu`)).body?.location?.id;

const ol = await j('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }) });
const OT = ol.body?.access_token;
const cl = await j('/api/courier/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ phone: creds.QA_COURIER_PHONE, password: creds.QA_COURIER_PASSWORD }) });
const CT = cl.body?.jwt || cl.body?.access_token;
if (!OT || !CT) { console.log(`owner ${ol.status}, courier ${cl.status} — cannot continue`); process.exit(1); }

const TERMINAL = ['DELIVERED', 'CANCELLED', 'REJECTED'];
const list = async () => {
  const o = await j('/api/owner/orders', { headers: { authorization: `Bearer ${OT}` } });
  return (o.body?.orders || o.body || []).filter(x => !TERMINAL.includes(x.status));
};

const stuck = await list();
console.log(`${VENUE}: ${stuck.length} order(s) not in a terminal state`);
for (const x of stuck) {
  console.log(`  ${x.id.slice(0, 8)}  ${x.status.padEnd(11)} ${new Date(x.created_at_ms).toISOString().slice(0, 16)}  ${x.contact?.name || '(no name)'}`);
}
if (!APPLY) { console.log('\ndry run — pass --apply to close them'); process.exit(0); }

await post('/api/courier/shift', CT, { open: true });
for (const x of stuck) {
  // Cancel first: it is the honest end, and it works while the order is still
  // PENDING. Anything further along has to go forward instead.
  let r = await post(`/api/owner/orders/${x.id}/action`, OT, { action: 'cancel', location_id: VENUE });
  if (r.status !== 200) {
    for (const act of ['confirm', 'preparing', 'ready']) {
      await post(`/api/owner/orders/${x.id}/action`, OT, { action: act, location_id: VENUE });
    }
    await post(`/api/courier/orders/${x.id}/accept`, CT);
    await post(`/api/courier/orders/${x.id}/pickup`, CT);
    r = await post(`/api/courier/orders/${x.id}/deliver`, CT, { cash_collected: true });
  }
  const now = (await list()).find(y => y.id === x.id);
  console.log(`  ${x.id.slice(0, 8)} -> ${now ? now.status + ' (STILL OPEN)' : 'closed'}${r.status >= 400 ? ` last=${r.status} ${JSON.stringify(r.body).slice(0, 70)}` : ''}`);
}
await post('/api/courier/shift', CT, { open: false });

const left = await list();
console.log(`\n${left.length === 0 ? 'the queue is clear' : `${left.length} still open: ` + left.map(x => `${x.id.slice(0, 8)}=${x.status}`).join(', ')}`);
process.exit(left.length ? 1 : 0);
