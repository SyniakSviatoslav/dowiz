// Row 38: the owner's customer list masks each phone with a key that is
// HMAC-SHA256(AUTH_SIGNING_KEY, digits) cut to 8 bytes
// (services/customers/handlers.rs customer_key). If the Worker secret is
// unset the code falls back to the public string "dowiz-unconfigured", and
// anyone can turn a phone into its key: computing the key with that fallback
// and finding it equal is the FAILED.
import crypto from 'node:crypto';
import { place, reject } from './_order.mjs';
const keyOf = (secret, phone) => {
  let d = phone.replace(/\D/g, ''); if (d.startsWith('00')) d = d.slice(2);
  return crypto.createHmac('sha256', secret).update(d).digest('hex').slice(0, 16);
};
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const phone = '+355690000019';
  const o = await place(ctx, {}, phone);
  await reject(ctx, o.id);
  const c = await lib.own(`/api/owner/customers?location_id=${lib.LOC}`);
  must(c.status === 200, `customers ${c.status}`);
  check('response_schema', c.body);
  const row = (c.body.customers || []).find(x => x.phone === '+355•••••19' && x.lastAt >= o.body.created_at_ms);
  must(row, 'the TEST customer is not in the owner list');
  must(!JSON.stringify(c.body).includes(phone), 'the owner list carries the full TEST phone');
  const fallback = keyOf('dowiz-unconfigured', phone);
  must(row.key !== fallback, `AUTH_SIGNING_KEY is unset on the live Worker: the key ${row.key} is HMAC("dowiz-unconfigured", phone)`);
  note(`phone shown ${row.phone}; key ${row.key} != fallback HMAC ${fallback}: the Worker signs with its own secret`);
}
