// Row 23: the owner forgets a customer (Law 124/2024 / GDPR Art. 17). A TEST
// order with a phone used by no one else; forget by its key; the customer list
// no longer holds it, and the venue's erasure count (health.redacted) grew.
import { place, reject } from './_order.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const phone = `+35569${String(Date.now()).slice(-7)}`;
  const tail = phone.slice(-2);
  const h0 = await lib.own(`/api/owner/health?location_id=${lib.LOC}`);
  must(h0.status === 200 && Number.isInteger(h0.body?.redacted), `health ${h0.status}: no integer redacted`);
  const o = await place(ctx, {}, phone);
  await reject(ctx, o.id);
  const list = async () => (await lib.own(`/api/owner/customers?location_id=${lib.LOC}`)).body?.customers || [];
  const row = (await list()).find(x => x.phone.endsWith(tail) && x.lastAt >= o.body.created_at_ms && x.orders === 1);
  must(row, 'the fresh TEST customer is not in the owner list');
  const f = await lib.own(`/api/owner/customers/${row.key}/forget?location_id=${lib.LOC}`, { reason: `${ctx.run} live proof of erasure`, lang: 'en' });
  must(f.status === 200, `forget ${f.status} ${f.text.slice(0, 120)}`);
  check('response_schema', f.body);
  must(!(await list()).some(x => x.key === row.key), `customer ${row.key} is still listed after forget`);
  const g = await lib.api(`/api/order/${encodeURIComponent(o.id)}`, { token: o.token });
  must(!JSON.stringify(g.body).includes(phone), 'the order read-back still carries the forgotten phone');
  const h1 = await lib.own(`/api/owner/health?location_id=${lib.LOC}`);
  must(h1.body?.redacted > h0.body.redacted, `health.redacted ${h0.body.redacted} -> ${h1.body?.redacted}, did not grow`);
  note(`forgot ${row.key}: gone from the list, the order read-back holds no phone, health.redacted ${h0.body.redacted} -> ${h1.body.redacted}`);
}
