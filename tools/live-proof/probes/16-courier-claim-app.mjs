// Row 16: the courier claims from the pool, picks up and is refused at the
// door, all through the courier's own routes; meanwhile the live socket
// (/api/live) opens for the courier's token, which is what the courier app
// keeps open (a node WebSocket, not Chromium: the same upgrade).
import { courierRun } from './_courier.mjs';
export default async function (ctx) {
  const { lib, check, must, note } = ctx;
  const r = await courierRun(ctx, async ({ o, tok }) => {
    check('response_schema', await lib.ownerOrder(o.id));
    // lib/live.js: the second subprotocol IS the token; the server echoes `bearer`.
    // Bot Fight Mode answers a UA-less upgrade with a challenge, so the browser's UA is sent.
    const url = `${lib.HOST.replace(/^http/, 'ws')}/api/live`;
    const opened = await new Promise(res => {
      const ws = new WebSocket(url, { protocols: ['bearer', tok], headers: { 'user-agent': lib.UA } });
      const to = setTimeout(() => { try { ws.close(); } catch {} res('timeout'); }, 15000);
      ws.onopen = () => { clearTimeout(to); const p = ws.protocol; ws.close(); res(p === 'bearer' ? 'open' : `open, protocol '${p}'`); };
      ws.onerror = e => { clearTimeout(to); res(`error ${e.message || ''}`); };
    });
    must(opened === 'open', `the courier's live socket: ${opened}`);
    return opened;
  });
  note(`pool -> accept -> IN_DELIVERY -> refused at the door -> ${r.st}; live socket ${r.extra}`);
}
