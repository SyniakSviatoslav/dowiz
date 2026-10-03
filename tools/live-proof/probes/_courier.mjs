// The QA hub's courier (QA_HUB_COURIER_*) takes one TEST delivery from the
// pool: READY -> accept -> pickup (IN_DELIVERY), optionally posts a position,
// and then reports it REFUSED at the door, which ends the order through the
// refund in one turn (courier/door.rs) -- never DELIVERED. The shift is put
// back as found.
import { place, close } from './_order.mjs';
export async function courierRun(ctx, during) {
  const { lib, must } = ctx;
  const { phone, password } = lib.courierCreds();
  must(phone && password, 'QA_HUB_COURIER_* missing from /root/.dowiz_owner');
  const lg = await lib.api('/api/courier/auth/login', { method: 'POST', body: { phone, password } });
  must(lg.status === 200 && lg.body?.courier?.locationId === lib.LOC, `courier login ${lg.status}`);
  const tok = lg.body.jwt, me = lg.body.courier.id;
  const C = (path, body) => lib.api(path, { method: body === undefined ? 'GET' : 'POST', token: tok, body });
  const t0 = await C('/api/courier/tasks');
  must(t0.status === 200, `tasks ${t0.status}`);
  must(!(t0.body.mine || []).length, `precondition: the QA courier already carries ${(t0.body.mine || []).map(o => o.id).join(', ')}`);
  const wasOpen = t0.body.available === true;
  const o = await place(ctx);
  let ended = false;
  try {
    for (const a of ['confirm', 'preparing', 'ready']) {
      const x = await lib.own(`/api/owner/orders/${o.id}/action`, { action: a, location_id: lib.LOC });
      must(x.status === 200, `owner ${a} ${x.status}`);
    }
    if (!wasOpen) must((await C('/api/courier/shift', { open: true })).status === 200, 'opening the shift');
    const pool = await C('/api/courier/tasks');
    must(JSON.stringify(pool.body.pool ?? pool.body).includes(o.id), 'the READY delivery is not in the courier pool');
    const acc = await C(`/api/courier/orders/${o.id}/accept`, {});
    must(acc.status === 200, `accept ${acc.status} ${acc.text.slice(0, 100)}`);
    must((await lib.ownerOrder(o.id))?.courier_id === me, 'after accept the order does not name this courier');
    const pk = await C(`/api/courier/orders/${o.id}/pickup`, {});
    must(pk.status === 200 && (await lib.statusOf(o.id)) === 'IN_DELIVERY', `pickup ${pk.status} -> ${await lib.statusOf(o.id)}`);
    const extra = await during({ o, tok, C, me });
    const rf = await C(`/api/courier/orders/${o.id}/refused`, {});
    must(rf.status === 200, `refused ${rf.status} ${rf.text.slice(0, 120)}`);
    const st = (await lib.ownerOrder(o.id))?.status;
    must(lib.TERMINAL.has(st) && st !== 'DELIVERED', `after refused the order is ${st}`);
    ended = true;
    return { o, st, extra };
  } finally {
    if (!ended) await close(ctx, o.id);
    else await close(ctx, o.id); // drops it from the pending file
    if (!wasOpen) await C('/api/courier/shift', { open: false });
  }
}
