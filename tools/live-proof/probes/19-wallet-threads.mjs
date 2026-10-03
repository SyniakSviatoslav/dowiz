// Row 19: the guest's own token opens the guest's wallet and the order's
// thread (thread id = order id, social/party.rs). The guest posts a TEST
// message; the owner's inbox lists the thread and reads the message back;
// another order's token is refused on this thread.
import { place, reject } from './_order.mjs';
export default async function (ctx) {
  const { lib, run, check, must, note } = ctx;
  const base = `/api/public/locations/${lib.LOC}`;
  const a = await place(ctx), b = await place(ctx);
  try {
    const w = await lib.api(`${base}/wallet`, { token: a.token });
    must(w.status === 200, `wallet ${w.status} ${w.text.slice(0, 100)}`);
    check('response_schema', w.body);
    const st = await lib.api(`${base}/wallet/statement`, { token: a.token });
    must(st.status === 200, `wallet statement ${st.status} ${st.text.slice(0, 100)}`);
    check('response_schema', st.body);
    const text = `${run} hello from the live proof`;
    const s = await lib.api(`${base}/threads/${a.id}/messages`, { method: 'POST', token: a.token, body: { from: 'CUSTOMER', body: text, clientId: `${run}-19` } });
    must(s.status === 200, `send ${s.status} ${s.text.slice(0, 120)}`);
    check('response_schema', s.body);
    const inbox = await lib.own(`/api/owner/threads?location_id=${lib.LOC}`);
    must(inbox.status === 200 && JSON.stringify(inbox.body).includes(a.id), `the owner's inbox (${inbox.status}) does not list thread ${a.id}`);
    const read = await lib.api(`${base}/threads/${a.id}?after=0`, { token: await lib.owner() });
    must(read.status === 200 && JSON.stringify(read.body).includes(text), `the owner's read of the thread (${read.status}) lacks the message`);
    const other = await lib.api(`${base}/threads/${a.id}?after=0`, { token: b.token });
    must(other.status === 404, `another order's token read this thread: ${other.status}`);
    note(`wallet + statement 200; message posted; owner inbox lists ${a.id.slice(0, 8)} and reads it; another order's token 404`);
  } finally { await reject(ctx, a.id); await reject(ctx, b.id); }
}
