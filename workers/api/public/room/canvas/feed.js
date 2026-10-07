// THE BOARD'S FEED, pure (node-tested in feed.test.mjs): the hub's two reads flattened into the
// tab-separated lines crates/dowiz-canvas src/board/feed.rs reads. Money is formatted HERE with the
// room's one formatter (logic.js -> lib/money.js); Rust never does money arithmetic.
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
import { parseCaps, sittingDue, money, guestWaiting } from '../logic.js';

const STATION = { sushi: 1, kitchen: 2, bar: 3 };
export const ROLE = { waiter: 0, 'counter-manager': 1, kitchen: 2, owner: 3 };
export const BUMPS = ['confirm', 'preparing', 'ready', 'collected'];

/// A value as one feed field: no tab, no newline.
export const clean = v => String(v ?? '').replace(/[\t\n\r]+/g, ' ');

/// The kitchen's tickets and the room's sittings as the feed Rust reads. PURE (node-tested).
/// `when(ms)` formats a scheduled hour for the screen; money goes through the room's `money`.
export function toFeed({ orders = [], sittings = [], currency = null, locale = 'sq', now = Date.now(), when = () => '' } = {}) {
  const out = [];
  for (const o of orders || []) {
    if (!o || !o.id) continue;
    const f = o.fulfilment || {};
    const sched = Number(o.scheduled_for_ms) || 0;
    const startMs = sched > 0 ? sched : Number(o.created_at_ms) || 0;
    const kind = f.table ? 't' : f.kind === 'delivery' ? 'd' : 'p';
    out.push(['T', o.id, o.status, Math.trunc(startMs), o.kitchen && o.kitchen.seen ? 1 : 0, kind, f.table, f.note, sched > now ? when(sched) : ''].map(clean).join('\t'));
    for (const l of o.items || []) {
      out.push(['L', Math.max(0, Number(l.quantity) | 0), STATION[l.station] || 2, l.name || l.product_id, l.note].map(clean).join('\t'));
    }
  }
  for (const s of sittings || []) {
    const rounds = s.rounds || [];
    out.push(['R', s.sitting_id, s.table, rounds.length, money(sittingDue(s), currency, locale),
      rounds.some(guestWaiting) ? 1 : 0, rounds.map(r => r.status).join(',')].map(clean).join('\t'));
  }
  return out.join('\n') + '\n';
}

/// What the session's caps open: [pass, floor].
export function opens(s) {
  const caps = parseCaps(s?.staff?.caps);
  return [caps.has('advance'), caps.has('take_orders')];
}
