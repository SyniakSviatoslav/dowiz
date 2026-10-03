// Row 14: the owner invites a courier; the invite is a shareable link (W-URGENT,
// no secret leaves the hub); a TEST phone claims it on the qa host, logs in,
// is listed in the owner's couriers and opens /api/courier/tasks; then the
// courier is switched off and every session revoked.
export default async function ({ lib, run, check, must, note }) {
  const phone = `+35567${String(Date.now()).slice(-7)}`, password = `${run}-Courier-1`;
  const inv = await lib.own(`/api/owner/couriers/invite?location_id=${lib.LOC}`, { phone, name: `${run} courier` });
  must(inv.status === 200 && inv.body?.code, `invite ${inv.status} ${inv.text.slice(0, 120)}`);
  check('response_schema', inv.body);
  must(typeof inv.body.url === 'string' && inv.body.url.startsWith(lib.HOST), `the invite link is not on this host: ${inv.body.url}`);
  let id;
  try {
    const cl = await lib.api('/api/courier/auth/claim', { method: 'POST', body: { phone, code: inv.body.code, password } });
    must(cl.status === 200, `claim ${cl.status} ${cl.text.slice(0, 120)}`);
    const li = await lib.api('/api/courier/auth/login', { method: 'POST', body: { phone, password } });
    must(li.status === 200 && li.body?.jwt && li.body.courier?.locationId === lib.LOC, `login ${li.status} ${li.text.slice(0, 120)}`);
    id = li.body.courier.id;
    const list = await lib.own(`/api/owner/couriers?location_id=${lib.LOC}`);
    must(list.status === 200 && JSON.stringify(list.body).includes(id), `the owner's couriers (${list.status}) do not list ${id}`);
    const t = await lib.api('/api/courier/tasks', { token: li.body.jwt });
    must(t.status === 200, `courier tasks ${t.status}`);
    const off = await lib.own(`/api/owner/couriers/${id}/active?location_id=${lib.LOC}`, { active: false });
    must(off.status === 200, `switching the courier off ${off.status}`);
    const t2 = await lib.api('/api/courier/tasks', { token: li.body.jwt });
    must(t2.status === 401 || t2.status === 403, `the switched-off courier's token still opens tasks: ${t2.status}`);
    note(`invite link on ${new URL(inv.body.url).host}; claimed, login venue ${lib.LOC}; listed; tasks 200; switched off -> tasks ${t2.status}`);
  } finally {
    if (id) await lib.own(`/api/owner/couriers/${id}/active?location_id=${lib.LOC}`, { active: false });
  }
}
