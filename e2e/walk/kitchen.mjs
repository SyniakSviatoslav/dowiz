// KITCHEN (a TEST staff member with the Kitchen preset) -- the per-role live walk:
//   node kitchen.mjs claim     claim the invite in the room app; what the kitchen is shown there
//   node kitchen.mjs cook      with the kitchen's own token: "seen" (kitchen-ack) and R1/R2 to READY;
//                              what the kitchen may NOT do (read the room) is refused
// The kitchen has no screen of its own in the product (the room app says "the kitchen screen"),
// so its moves are the routes the console's buttons call, signed by the kitchen.
// PASS/FAIL/INFO per step with the API read-back; exit code = FAILs.
import { OUT, csp, browser, api, step, end, sst, LOC, roomIn, staffLogin, ownerToken, ownerOrder } from './_lib.mjs';

const PHASE = process.argv[2] || 'claim';
const S = sst();
const K = S.kitchen;
if (!K?.email) { console.log('no TEST kitchen in suite state: run owner.mjs setup first'); process.exit(2); }
let b = null;
try {
  if (PHASE === 'claim') {
    b = await browser();
    const ctx = await b.newContext({ viewport: { width: 400, height: 860 }, serviceWorkers: 'block' });
    const { p, answer } = await roomIn(ctx, K.email, K.password, K.code, 'room-kitchen');
    step(answer?.status === 200, 'the TEST kitchen claims the invite in the room app', JSON.stringify(answer));
    const view = await p.evaluate(() => ({ text: document.querySelector('#app').innerText.replace(/\s+/g, ' ').slice(0, 300), acts: [...document.querySelectorAll('[data-act]')].map(e => e.dataset.act) }));
    step(!view.acts.includes('open') && !view.acts.includes('till'), 'the room app gives the kitchen no floor, till or open-table controls', view);
    step(null, 'what the kitchen is told', view.text);
    await p.screenshot({ path: `${OUT}/k1-room.png` });
    await csp(p, 'room-kitchen');
  }
  if (PHASE === 'cook') {
    const l = await staffLogin(K.email, K.password);
    step(l.status === 200 && !!l.jwt, 'kitchen signs in (API)', `${l.status} role=${l.body?.role || l.body?.staff?.role || '?'}`);
    const tok = l.jwt, own = await ownerToken();
    const denied = await api(`/api/staff/room?location_id=${LOC}`, { token: tok });
    step(denied.status === 403, 'the kitchen may not read the room (403)', `${denied.status} ${JSON.stringify(denied.body).slice(0, 120)}`);
    for (const id of [S.r1, S.r2]) {
      const ack = await api(`/api/staff/orders/${id}/kitchen-ack`, { method: 'POST', token: tok, body: { location_id: LOC }, headers: { 'idempotency-key': crypto.randomUUID() } });
      step(ack.status === 200, `kitchen "seen" on ${id.slice(0, 8)}`, `${ack.status} ${JSON.stringify(ack.body?.order?.kitchen || ack.body).slice(0, 200)}`);
      let st = (await ownerOrder(own, id))?.status;
      const path = st === 'PENDING' ? ['confirm', 'preparing', 'ready'] : st === 'CONFIRMED' ? ['preparing', 'ready'] : st === 'PREPARING' ? ['ready'] : [];
      for (const action of path) {
        const r = await api(`/api/owner/orders/${id}/action`, { method: 'POST', token: tok, body: { action, location_id: LOC } });
        st = (await ownerOrder(own, id))?.status;
        step(r.status === 200, `kitchen ${action} ${id.slice(0, 8)} -> ${st}`, `${r.status} ${JSON.stringify(r.body).slice(0, 160)}`);
      }
      step(st === 'READY', `${id.slice(0, 8)} is READY (owner read-back)`, st);
    }
  }
} catch (e) {
  step(false, `phase ${PHASE} threw`, e.stack?.slice(0, 400));
}
await end(b);
