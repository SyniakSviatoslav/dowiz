// Row 2: an order for later keeps its time. The guest asks for now+3h; the
// owner's copy carries the same scheduled_for_ms (storefront.rs PlaceIn).
import { payload, place, reject } from './_order.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const when = Math.ceil((Date.now() + 3 * 3600e3) / 900e3) * 900e3; // on a 15-minute slot
  check('request_schema', payload(ctx.run, { scheduled_for_ms: when }));
  const o = await place(ctx, { scheduled_for_ms: when });
  check('response_schema', o.body);
  const w = await lib.ownerOrder(o.id);
  must(w, `owner list does not hold ${o.id}`);
  try {
    check('readback_schema', w);
    must(w.scheduled_for_ms === when, `owner copy scheduled_for_ms ${w.scheduled_for_ms}, sent ${when}`);
  } finally { await reject(ctx, o.id); }
  note(`placed ${o.id} for ${new Date(when).toISOString()}; the owner's copy carries the same ms; rejected`);
}
