// F4 CLEANUP, API only, and it ALWAYS runs: every order this run (or an
// earlier crashed run -- anything whose customer name starts FLOWS-) left
// open is closed; the dish name, the venue's pickup switch and the courier's
// shift are put back as found; then a final sweep proves nothing with the
// prefix is left open or lying about.
//
// HOW AN ORDER IS CLOSED. PENDING -> cancel (the owner's action). Past
// PENDING the kernel's only exit is REFUNDING -> COMPENSATED_REFUND
// (order_machine.rs allowed_next; memory dowiz-order-fsm-has-no-exit), which
// the refund route takes in one turn when no money was taken. DELIVERED is
// terminal: F3's delivered order stays in the history, and is reported.
import { LOC, RUN, PREFIX, S, Fail, api, own, owner, ownerOrders, statusOf, courierCreds, menu, TERMINAL, restored } from './lib.mjs';

async function close(o) {
  const st = o.status;
  if (TERMINAL.has(st)) return `${st} (terminal)`;
  if (st === 'PENDING') {
    const r = await own(`/api/owner/orders/${o.id}/action`, { action: 'cancel', location_id: LOC });
    if (r.status !== 200) throw new Fail(`cancel ${o.id}: ${r.status} ${r.text.slice(0, 120)}`);
  } else {
    const r = await api(`/api/staff/orders/${o.id}/refund`, { method: 'POST', token: await owner(),
      headers: { 'idempotency-key': `${RUN}-refund-${o.id}` }, body: { location_id: LOC, reason: 'venue_cancelled', note: 'QA flows gate cleanup' } });
    if (r.status !== 200) throw new Fail(`refund ${o.id} from ${st}: ${r.status} ${r.text.slice(0, 120)}`);
  }
  const after = await statusOf(o.id) ?? (await ownerOrders()).find(x => x.id === o.id)?.status;
  if (!TERMINAL.has(after)) throw new Fail(`closing ${o.id} from ${st} left it ${after}`);
  return `${st} -> ${after}`;
}

export async function f4(log) {
  const problems = [];
  // A step that died on the box's network ("fetch failed": measured
  // 2026-09-30 on the dish-name restore) is tried again, at most three times;
  // a refusal from the hub is not.
  const attempt = async (what, fn) => {
    for (let i = 1; ; i++) {
      try { const r = await fn(); if (r) log(`${what}: ${r}`); return; } catch (e) {
        if (i < 3 && /fetch failed/.test(String(e.message))) { console.log(`  !! F4 ${what}: ${e.message}, try ${i + 1} of 3`); continue; }
        problems.push(`${what}: ${e.message}`); return;
      }
    }
  };

  // ── orders: this run's, then any FLOWS- order an earlier run left open ──
  // The box's network failed five connects in a row on 2026-09-30 and took
  // the whole cleanup down; the list is asked three times before giving up.
  let all = null;
  for (let i = 1; i <= 3 && !all; i++) {
    try { all = await ownerOrders(); } catch (e) { if (i === 3) throw new Fail(`reading the orders to close: ${e.message}`); console.log(`  !! F4: reading the orders failed (${String(e.message).slice(0, 80)}), try ${i + 1} of 3`); }
  }
  const mine = new Set(S.orders.map(o => o.id));
  const targets = all.filter(o => mine.has(o.id) || String(o.contact?.name || '').startsWith(PREFIX));
  for (const id of mine) if (!all.some(o => o.id === id)) problems.push(`order ${id} (placed by this run) is missing from the owner's list`);
  for (const o of targets) await attempt(`order ${o.id} ${o.contact?.name || ''}`, () => close(o));

  // ── the dish name, the pickup switch, the courier's shift ───────────────
  if (S.restore.dishName !== undefined) await attempt('dish qa-water name', async () => {
    const r = await own(`/api/owner/products?location_id=${LOC}`);
    const cur = (r.body?.products || []).find(x => x.id === 'qa-water')?.name;
    if (cur === S.restore.dishName) return `"${cur}" (as found)`;
    const w = await own('/api/owner/products/qa-water', { location_id: LOC, name: S.restore.dishName });
    if (w.status !== 200) throw new Fail(`restoring the name: ${w.status} ${w.text.slice(0, 100)}`);
    return `restored "${cur}" -> "${S.restore.dishName}"`;
  });
  if (S.restore.bom !== undefined) await attempt('qa-tuna-roll recipe', async () => {
    const r = await own(`/api/owner/products?location_id=${LOC}`);
    const cur = JSON.stringify(((r.body?.products || []).find(x => x.id === 'qa-tuna-roll')?.bom || []).map(l => [l.supply, l.qty]));
    const want = JSON.stringify(S.restore.bom.map(l => [l.supply, l.qty]));
    if (cur === want) return `${cur} (as found)`;
    const w = await own('/api/owner/products/qa-tuna-roll', { location_id: LOC, bom: S.restore.bom });
    if (w.status !== 200) throw new Fail(`restoring the recipe: ${w.status} ${w.text.slice(0, 100)}`);
    return `restored ${cur} -> ${want}`;
  });
  // The ПФ cards F2 made (id `flows-…-pf`), this run's or a crashed one's.
  await attempt('QA ПФ cards', async () => {
    const r = await own(`/api/owner/preps?location_id=${LOC}`);
    const mine = (r.body?.preps || []).filter(x => String(x.id).startsWith(PREFIX.toLowerCase())).map(x => x.id);
    for (const id of mine) {
      const d = await own('/api/owner/supplies/delete', { ids: [id], location_id: LOC, confirmUses: true });
      if (d.status !== 200) throw new Fail(`deleting ${id}: ${d.status} ${d.text.slice(0, 100)}`);
    }
    return mine.length ? `deleted ${mine.join(', ')}` : '';
  });
  if (S.restore.pickup !== undefined) await attempt('venue pickup switch', async () => {
    const cur = !!(await menu()).location?.pickup;
    if (cur === S.restore.pickup) return `${cur} (as found)`;
    const w = await own('/api/owner/location', { location_id: LOC, pickup: S.restore.pickup });
    if (w.status !== 200) throw new Fail(`restoring pickup: ${w.status} ${w.text.slice(0, 100)}`);
    return `restored ${cur} -> ${S.restore.pickup}`;
  });
  if (S.restore.courierShift === 'opened-by-F3') await attempt('courier shift', async () => {
    const lg = await api('/api/courier/auth/login', { method: 'POST', body: courierCreds() });
    const w = await api('/api/courier/shift', { method: 'POST', token: lg.body?.jwt, body: { open: false } });
    if (w.status !== 200) throw new Fail(`closing the shift F3 opened: ${w.status} ${w.text.slice(0, 100)}`);
    return 'closed (F3 had opened it)';
  });

  // ── the final sweep: nothing with the prefix open, nothing named RUN ────
  await attempt('final sweep', async () => {
    const now = await ownerOrders();
    const left = now.filter(o => (mine.has(o.id) || String(o.contact?.name || '').startsWith(PREFIX)) && !TERMINAL.has(o.status));
    if (left.length) throw new Fail(`${left.length} ${PREFIX} orders still open: ${left.map(o => `${o.id}=${o.status}`).join(', ')}`);
    const named = [];
    for (const [path, key] of [['products', 'products'], ['categories', 'categories'], ['stock', 'supplies'], ['preps', 'preps']]) {
      const r = await own(`/api/owner/${path}?location_id=${LOC}`);
      if (r.status !== 200) throw new Fail(`GET /api/owner/${path} ${r.status}`);
      for (const x of (r.body?.[key] || [])) if (String(x.name || '').includes(PREFIX) || String(x.id || '').includes(PREFIX.toLowerCase())) named.push(`${path}:${x.id}`);
    }
    if (named.length) throw new Fail(`objects carrying ${PREFIX} left: ${named.join(', ')}`);
    const m = await menu();
    if (S.restore.pickup !== undefined && !!m.location?.pickup !== S.restore.pickup) throw new Fail(`pickup is ${!!m.location?.pickup}, was ${S.restore.pickup}`);
    const kept = now.filter(o => mine.has(o.id) && o.status === 'DELIVERED').map(o => o.id);
    return `no ${PREFIX} order open, no ${PREFIX} object, settings as found${kept.length ? `; kept as terminal DELIVERED sale: ${kept.join(',')}` : ''}`;
  });
  if (!problems.length) restored();
  if (problems.length) throw new Fail(`${problems[0]}${problems.length > 1 ? ` (+${problems.length - 1} more: ${problems.slice(1).join(' | ')})` : ''}`);
}
