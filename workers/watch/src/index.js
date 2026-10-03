// dowiz-watch: probes production every 5 minutes from inside Cloudflare, keeps the last result per
// target, mails on down/up transitions, and answers GET /status and GET /healthz on its workers.dev
// URL for GitHub's heartbeat to read. See wrangler.toml for why it exists and how it deploys.
import { targets, probe } from './probes.js';
import { mailRaw, send } from './mail.js';
export { WatchState } from './state.js';
import { statusDoc } from './fold.js';


const stub = (env) => env.STATE.get(env.STATE.idFromName('watch'));

const json = (v, status = 200) =>
  new Response(JSON.stringify(v, null, 1), {
    status,
    headers: { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' },
  });

/// One tick: probe everything in parallel, fold it in, mail the transitions.
export async function tick(env, atMs, statusUrl, fetchFn = fetch) {
  const results = await Promise.all(targets(env).map((t) => probe(t, fetchFn)));
  const s = stub(env);
  const { transitions } = await s.record(results, atMs);
  let mail = null;
  if (transitions.length) {
    const raw = mailRaw({ from: env.MAIL_FROM, to: env.MAIL_TO, changes: transitions, atMs, statusUrl, id: `watch-${atMs}` });
    const sent = await send(env, raw);
    mail = { atMs, changes: transitions.map((c) => `${c.name}: ${c.from || 'unknown'} -> ${c.to}`), ...sent };
    await s.noteMail(mail);
  }
  return { results, transitions, mail };
}


export default {
  async scheduled(event, env, ctx) {
    const statusUrl = env.WATCH_URL ? `${env.WATCH_URL}/status` : 'GET /status on the dowiz-watch workers.dev URL';
    // A tick that throws is LOUD in the Worker's logs; /healthz then goes stale within 15 min.
    ctx.waitUntil(tick(env, event.scheduledTime || Date.now(), statusUrl).catch((e) => console.error('dowiz-watch tick failed:', e && e.stack || e)));
  },

  async fetch(req, env) {
    const url = new URL(req.url);
    if (req.method !== 'GET') return json({ error: 'GET only' }, 405);
    const doc = statusDoc(await stub(env).read(), env, Date.now());
    if (url.pathname === '/status') return json(doc);
    if (url.pathname === '/healthz') {
      // The WATCHER's health: is it still ticking? Production's state is /status's `ok`.
      return doc.stale
        ? new Response(`stale: last tick ${doc.tickAgeSeconds == null ? 'never' : `${doc.tickAgeSeconds} s ago`}`, { status: 503 })
        : new Response('ok', { status: 200, headers: { 'cache-control': 'no-store' } });
    }
    return json({ routes: ['/status', '/healthz'], watcher: doc.watcher }, 404);
  },
};
