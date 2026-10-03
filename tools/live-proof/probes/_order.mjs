// A TEST order on the qa venue, the way the kit's orderPayload (kit/data.js)
// builds it, and the promise to end it. Every order carries the run's LIVE-
// prefix in the contact name; its id outlives the run in a pending file until
// the owner has rejected it (plan §1 F-E rule 3), so a run that dies half way
// is closed by the next one (`sweep`).
import fs from 'node:fs';
import os from 'node:os';

const pendingFile = LOC => `${os.tmpdir()}/dowiz-live/pending-${LOC}.json`;
const readPending = LOC => { try { return JSON.parse(fs.readFileSync(pendingFile(LOC), 'utf8')); } catch { return []; } };
const writePending = (LOC, ids) => { fs.mkdirSync(`${os.tmpdir()}/dowiz-live`, { recursive: true }); fs.writeFileSync(pendingFile(LOC), JSON.stringify(ids)); };

/// The request body: one QA Water for delivery in Durres, cash.
export const payload = (run, extra = {}, phone = '+355690000019') => ({
    items: [{ product_id: 'qa-water', modifier_ids: [], quantity: 1 }],
    contact: { name: `${run} guest`, phone },
    fulfilment: { kind: 'delivery', address: { line: `${run} Rruga Isuf Ferra 1, Durres`, note: null, lat_udeg: 41323100, lon_udeg: 19441400 } },
    payment: 'cash', locale: 'en', ...extra,
});

/// Place it. Answers {id, token, body, sent}.
export async function place({ lib, run, must }, extra = {}, phone) {
  const sent = payload(run, extra, phone);
  const r = await lib.api(`/api/public/locations/${lib.LOC}/orders`, { method: 'POST', body: sent });
  must(r.status === 200 && r.body?.id, `placing a TEST order: ${r.status} ${r.text.slice(0, 140)}`);
  writePending(lib.LOC, [...readPending(lib.LOC), r.body.id]);
  return { id: r.body.id, token: r.body.access_token, body: r.body, sent };
}

/// The owner rejects it (PENDING -> REJECTED), and it leaves the pending file.
export async function reject({ lib, must }, id) {
  const r = await lib.own(`/api/owner/orders/${encodeURIComponent(id)}/action`, { action: 'reject', location_id: lib.LOC });
  must(r.status === 200 && r.body?.status === 'REJECTED', `rejecting ${id}: ${r.status} ${r.text.slice(0, 120)}`);
  writePending(lib.LOC, readPending(lib.LOC).filter(x => x !== id));
}

/// End a TEST order from wherever it is: PENDING -> reject; past PENDING the
/// kernel's only exit is the refund route (REFUNDING -> COMPENSATED_REFUND,
/// e2e/flows/f4-cleanup.mjs), which ends it in one turn when no money was taken.
export async function close({ lib, run, must }, id) {
  const o = await lib.ownerOrder(id);
  if (!o || lib.TERMINAL.has(o.status)) { writePending(lib.LOC, readPending(lib.LOC).filter(x => x !== id)); return o?.status; }
  if (o.status === 'PENDING') { await reject({ lib, must }, id); return 'REJECTED'; }
  const r = await lib.api(`/api/staff/orders/${encodeURIComponent(id)}/refund`, { method: 'POST', token: await lib.owner(),
    headers: { 'idempotency-key': `${run}-refund-${id}` }, body: { location_id: lib.LOC, reason: 'venue_cancelled', note: 'live-proof cleanup' } });
  must(r.status === 200, `refund ${id} from ${o.status}: ${r.status} ${r.text.slice(0, 120)}`);
  const after = (await lib.ownerOrder(id))?.status;
  must(lib.TERMINAL.has(after), `closing ${id} from ${o.status} left it ${after}`);
  writePending(lib.LOC, readPending(lib.LOC).filter(x => x !== id));
  return after;
}

/// Close every TEST order an earlier run left open. Answers what it did.
export async function sweep(lib) {
  const left = [];
  for (const id of readPending(lib.LOC)) {
    const o = await lib.ownerOrder(id);
    if (o && !lib.TERMINAL.has(o.status)) {
      try { await close({ lib, run: lib.RUN, must: (ok, why) => { if (!ok) throw new Error(why); } }, id); }
      catch { left.push(id); continue; }
    }
  }
  writePending(lib.LOC, left);
  return left;
}
