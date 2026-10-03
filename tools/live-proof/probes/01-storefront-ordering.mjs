// Row 1: a guest's order goes through the Worker into the venue's object, and
// the guest (by the key minted with the order) and the owner read back the
// same order. API level with the kit's own payload shape; the Chromium
// checkout in front of it is e2e/flows/f1-customer.mjs.
import { payload, place, reject } from './_order.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  check('request_schema', payload(ctx.run));
  const o = await place(ctx);
  check('response_schema', o.body);
  const g = await lib.api(`/api/order/${encodeURIComponent(o.id)}`, { token: o.token });
  must(g.status === 200, `guest read-back ${g.status}`);
  check('readback_schema', g.body);
  const w = await lib.ownerOrder(o.id);
  must(w && w.status === 'PENDING' && w.contact?.name === `${ctx.run} guest`, `owner read-back: ${JSON.stringify(w)?.slice(0, 120)}`);
  await reject(ctx, o.id);
  note(`placed ${o.id}; guest and owner read PENDING; owner rejected`);
}
