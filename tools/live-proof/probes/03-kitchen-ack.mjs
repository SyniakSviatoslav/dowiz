// Row 3: the kitchen sees an order. A TEST member of staff with the kitchen
// role is invited, claims the code and is signed in; the pass board lists the
// TEST order without the guest; kitchen-ack marks it seen; the owner's order
// carries kitchen.seen and health.kitchen.unseen no longer names it. The
// order is ended and the member of staff suspended, whatever happens.
import { place, close } from './_order.mjs';
export default async function (ctx) {
  const { lib, run, check, must, note } = ctx;
  const tag = run.toLowerCase();
  const email = `${tag}@example.com`, password = `${run}-Kitchen-pass1`;
  const inv = await lib.own(`/api/owner/staff/invite?location_id=${lib.LOC}`, { email, name: `${run} kitchen`, role: 'kitchen', location_id: lib.LOC });
  must(inv.status === 200 && inv.body?.code, `invite ${inv.status} ${inv.text.slice(0, 120)}`);
  let o;
  try {
    const cl = await lib.api('/api/staff/claim', { method: 'POST', body: { email, code: inv.body.code, password } });
    let jwt = cl.body?.jwt;
    if (!jwt) jwt = (await lib.api('/api/staff/login', { method: 'POST', body: { email, password, location_id: lib.LOC } })).body?.jwt;
    must(jwt, `kitchen claim ${cl.status} ${cl.text.slice(0, 120)}`);
    o = await place(ctx);
    const board = await lib.api(`/api/staff/kitchen?location_id=${lib.LOC}`, { token: jwt });
    must(board.status === 200, `kitchen board ${board.status}`);
    const s = JSON.stringify(board.body);
    must(s.includes(o.id), 'the kitchen board does not list the TEST order');
    must(!s.includes('+355690000019'), 'the kitchen board carries the guest phone');
    const ack = await lib.api(`/api/staff/orders/${o.id}/kitchen-ack`, { method: 'POST', token: jwt, body: { location_id: lib.LOC } });
    must(ack.status === 200, `kitchen-ack ${ack.status} ${ack.text.slice(0, 120)}`);
    check('response_schema', ack.body);
    const w = await lib.ownerOrder(o.id);
    must(w?.kitchen?.seen, `the owner's order carries no kitchen.seen: ${JSON.stringify(w?.kitchen || {})}`);
    const h = await lib.own(`/api/owner/health?location_id=${lib.LOC}`);
    must(!(h.body?.kitchen?.unseen || []).some(u => u.orderId === o.id), 'health.kitchen.unseen still names the acknowledged order');
    note(`kitchen ${email} claimed; board lists the order without the phone; ack -> kitchen.seen; health.unseen excludes it`);
  } finally {
    if (o) await close(ctx, o.id);
    const list = await lib.own(`/api/owner/staff?location_id=${lib.LOC}`);
    const me = (list.body?.staff || []).find(x => x.name === `${run} kitchen`);
    if (me) await lib.own(`/api/owner/staff/${me.id}?location_id=${lib.LOC}`, { active: false, location_id: lib.LOC });
  }
}
