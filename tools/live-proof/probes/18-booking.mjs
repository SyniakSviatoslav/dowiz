// Row 18: a guest books a table for a slot the floor says is free; the owner
// sees and confirms it; the guest's own token reads it and gets a pass the
// venue's door verifies; the guest cancels (an event, nothing erased).
export default async function ({ lib, run, check, must, note }) {
  const day = new Date(); day.setUTCDate(day.getUTCDate() + 2); day.setUTCHours(17, 0, 0, 0);
  const slot = Math.floor(day.getTime() / 60000);
  const base = `/api/public/locations/${lib.LOC}`;
  const t = await lib.api(`${base}/tables?slotMin=${slot}&party=2`);
  must(t.status === 200, `tables ${t.status}`);
  const free = t.body.zones.flatMap(z => z.tables.filter(x => !x.occupied && !x.tooSmall).map(x => ({ zoneId: z.id, tableN: x.n })))[0];
  must(free, `no free table for party 2 at slot ${slot}`);
  const sent = { party: 2, slotMin: slot, contactName: `${run} guest`, contactPhone: '+355690000019', requestId: `${run}-18`, ...free };
  check('request_schema', sent);
  const c = await lib.api(`${base}/reservations`, { method: 'POST', body: sent });
  must(c.status === 200 && c.body?.id, `create ${c.status} ${c.text.slice(0, 120)}`);
  check('response_schema', c.body);
  const { id, access_token: tok } = c.body;
  let cancelled = false;
  try {
    const o = await lib.own(`/api/owner/reservations?location_id=${lib.LOC}&from=${slot - 60}&to=${slot + 60}`);
    must(o.status === 200 && (o.body?.reservations || []).some(r => r.id === id), `the owner's day (${o.status}) does not list ${id}`);
    const conf = await lib.own(`/api/owner/reservations/${id}/action?location_id=${lib.LOC}`, { to: 'CONFIRMED' });
    must(conf.status === 200, `owner confirm ${conf.status} ${conf.text.slice(0, 120)}`);
    const d = await lib.api(`${base}/reservations/${id}`, { token: tok });
    must(d.status === 200 && d.body?.status === 'CONFIRMED', `guest read-back ${d.status} ${d.body?.status}`);
    const p = await lib.api(`${base}/reservations/${id}/pass`, { token: tok });
    must(p.status === 200, `pass ${p.status} ${p.text.slice(0, 120)}`);
    const code = p.body?.code || p.body?.pass;
    must(typeof code === 'string' && code.length > 8, `the pass carries no code: ${p.text.slice(0, 120)}`);
    // The booking is two days out, so the door answers "too early" -- which it
    // says only AFTER the tag and the venue held (crates/dowiz-core/src/pass.rs).
    // One flipped character must instead fail the tag.
    const ownerTok = await lib.owner();
    const verify = c2 => lib.api(`${base}/pass/verify`, { method: 'POST', token: ownerTok, body: { code: c2 } });
    const v = await verify(code);
    must(v.status === 200 && v.body?.ok === false && /^too early/.test(v.body.why || ''), `genuine pass: ${v.status} ${v.text.slice(0, 160)}`);
    const i = Math.floor(code.length / 2);
    const forged = await verify(code.slice(0, i) + (code[i] === 'A' ? 'B' : 'A') + code.slice(i + 1));
    must(forged.body?.ok === false && !/^too early/.test(forged.body.why || ''), `a forged pass answered: ${forged.text.slice(0, 160)}`);
    const x = await lib.api(`${base}/reservations/${id}/action`, { method: 'POST', token: tok, body: { to: 'CANCELLED_BY_GUEST' } });
    must(x.status === 200, `guest cancel ${x.status} ${x.text.slice(0, 120)}`);
    cancelled = true;
    const after = await lib.api(`${base}/reservations/${id}`, { token: tok });
    must(after.body?.status === 'CANCELLED_BY_GUEST' && after.body.history?.length >= 3, `after cancel: ${after.body?.status}, ${after.body?.history?.length} history rows`);
    note(`${id} on ${free.zoneId} #${free.tableN}: owner lists + confirms, guest reads CONFIRMED, pass passes tag+venue (door: too early), a forged one fails, guest cancels (history ${after.body.history.length})`);
  } finally {
    if (!cancelled) await lib.own(`/api/owner/reservations/${id}/action?location_id=${lib.LOC}`, { to: 'CANCELLED_BY_VENUE', reason: run });
  }
}
