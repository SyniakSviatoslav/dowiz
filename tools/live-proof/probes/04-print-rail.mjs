// Row 4: the kitchen printer rail (print_rail.rs, CloudPRNT-style). With the
// venue's printer named (setting print.kitchen) a new order queues a ticket;
// a printer holding a venue key polls, fetches the ticket (it names the dish),
// acks with DELETE, and the owner's order carries kitchen.printed. The setting
// is put back as found and the key revoked, whatever happens.
import { place, close } from './_order.mjs';
const SETTING = 'print.kitchen';
export default async function (ctx) {
  const { lib, run, check, must, note } = ctx;
  const s0 = await lib.own(`/api/owner/settings?location_id=${lib.LOC}`);
  must(s0.status === 200, `settings ${s0.status}`);
  const found = (s0.body.values || {})[SETTING] || '';
  const setTo = v => lib.own(`/api/owner/settings?location_id=${lib.LOC}`, { key: SETTING, value: v });
  if (!found) must((await setTo(`${run}-printer`)).status === 200, 'naming the TEST printer');
  const k = await lib.own(`/api/owner/apikeys?location_id=${lib.LOC}`, { label: `${run} printer` });
  must(k.status === 200, `minting the printer key ${k.status}`);
  let o;
  try {
    o = await place(ctx);
    let job;
    for (let i = 0; i < 6 && !job; i++) {           // the printer polls; another job may be ahead of ours
      const p = await lib.api('/api/print/poll', { method: 'POST', token: k.body.key, body: {} });
      must(p.status === 200, `poll ${p.status} ${p.text.slice(0, 100)}`);
      check('response_schema', p.body);
      if (!p.body.jobReady) { await lib.sleep(1000); continue; }
      const j = await fetch(`${lib.HOST}/api/print/job/${p.body.jobToken}`, { headers: { authorization: `Bearer ${k.body.key}`, 'user-agent': lib.UA } });
      const text = await j.text();
      must(j.status === 200, `job ${j.status}`);
      const a = await lib.api(`/api/print/job/${p.body.jobToken}`, { method: 'DELETE', token: k.body.key });
      must(a.status === 200 || a.status === 204, `ack ${a.status} ${a.text.slice(0, 80)}`);
      if (text.includes(o.id.slice(-4).toUpperCase()) || text.includes(o.id.slice(0, 8)) || text.includes(o.id)) job = text;
    }
    must(job, 'no ticket for the TEST order reached the printer in 6 polls');
    must(/QA Water/.test(job), `the ticket does not name the dish: ${job.slice(0, 120)}`);
    const w = await lib.ownerOrder(o.id);
    must(w?.kitchen?.printed, `the owner's order carries no kitchen.printed: ${JSON.stringify(w?.kitchen || {})}`);
    note(`ticket (${job.length} B) names QA Water; acked; owner order kitchen.printed set`);
  } finally {
    if (o) await close(ctx, o.id);
    await lib.own(`/api/owner/apikeys/revoke?location_id=${lib.LOC}`, { id: k.body.id });
    if (!found) await setTo('');
  }
}
