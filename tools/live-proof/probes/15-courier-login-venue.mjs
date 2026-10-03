// Row 15: a courier's login takes its venue from the HOST (accounts.rs): the
// qa hub's courier logs in on qa-durres and gets a qa-durres token; the same
// credential naming another venue in the body is refused.
export default async function ({ lib, check, must, note }) {
  const { phone, password } = lib.courierCreds();
  must(phone && password, 'QA_HUB_COURIER_* missing from /root/.dowiz_owner');
  const r = await lib.api('/api/courier/auth/login', { method: 'POST', body: { phone, password } });
  must(r.status === 200, `courier login ${r.status} ${r.text.slice(0, 100)}`);
  const venue = check('response_schema', r.body).courier.locationId;
  must(venue === lib.LOC, `the login's venue is ${venue}, not ${lib.LOC}`);
  // The token itself must open this venue's courier routes.
  const tasks = await lib.api('/api/courier/tasks', { token: r.body.jwt });
  must(tasks.status === 200, `GET /api/courier/tasks with the token: ${tasks.status}`);
  const other = await lib.api('/api/courier/auth/login', { method: 'POST', body: { phone, password, location_id: 'sushi-durres' } });
  must(other.status >= 400 || other.body?.courier?.locationId === lib.LOC,
    `naming sushi-durres in the body logged in to ${other.body?.courier?.locationId} (${other.status})`);
  note(`venue ${venue}; courier tasks 200; body naming sushi-durres -> ${other.status}${other.body?.courier ? ` (still ${other.body.courier.locationId})` : ''}`);
}
