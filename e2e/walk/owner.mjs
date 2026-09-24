// OWNER (console) -- the per-role live walk, one phase per run:
//   node owner.mjs setup     floor-plan editor (TEST zone + 91/92), staff invites (TEST waiter + kitchen), table QR page
//   node owner.mjs bookings  Bookings screen: confirm B1, decline B2, phone-book B3 then cancel it
//   node owner.mjs orders    storefront TEST orders: reject X, P confirm..ready then refund, D confirm..ready (for the courier)
//   node owner.mjs refunds   refund the room's READY rounds (R1, R2) and anything TEST still REFUNDING
//   node owner.mjs panes     customers link/unlink, campaigns preview, stamp block, CSV dry runs, printer,
//                            exceptions, eBills (read), fiscal (arming refused), health
//   node owner.mjs cleanup   plan emptied in the editor, TEST staff suspended, final read-back of every artefact
// Every step prints PASS/FAIL/INFO with the API read-back; exit code = FAILs.
import fs from 'node:fs';
import { HOST, OUT, creds, watch, csp, browser, api, ownerToken, step, end, sst, ssave, LOC, consoleIn, tile, closeSheet, ownerOrder, issues } from './_lib.mjs';

const PHASE = process.argv[2] || 'setup';
const tok = await ownerToken();
const b = await browser();
const ctx = await b.newContext({ viewport: { width: 420, height: 900 }, serviceWorkers: 'block', permissions: ['clipboard-read', 'clipboard-write'] });
const o = await consoleIn(ctx);
step(!(await o.$('#e')), 'console signs in');
const writes = [];
o.on('response', async r => {
  const m = r.request().method();
  if (m === 'POST' && /\/api\/(owner|staff)\/|\/reservations$/.test(r.url())) {
    let t = ''; try { t = (await r.text()).slice(0, 300); } catch {}
    writes.push({ status: r.status(), url: r.url().replace(HOST, ''), body: t.replace(/"code":"[^"]+"/, '"code":"…"') });
  }
});
const lastWrite = re => [...writes].reverse().find(w => re.test(w.url));
const toast = () => o.$eval('#toast', e => e.hidden ? '' : e.textContent).catch(() => '');
const S = sst();

try {
  if (PHASE === 'setup') {
    // ── floor-plan editor: a TEST room with tables 91 and 92 ──
    let txt = await tile(o, 'floorPlan');
    step(null, 'floor-plan sheet', txt.slice(0, 200));
    await o.click('#fpAddZone'); await o.waitForTimeout(600);
    await o.fill('#fp-zname', 'TEST walk'); await o.dispatchEvent('#fp-zname', 'change'); await o.waitForTimeout(600);
    await o.click('#fpAddT'); await o.waitForTimeout(500);
    await o.fill('#fp-n', '91'); await o.dispatchEvent('#fp-n', 'change'); await o.waitForTimeout(500);
    await o.fill('#fp-seats', '4'); await o.dispatchEvent('#fp-seats', 'change'); await o.waitForTimeout(400);
    await o.click('[data-nudge="1,0"]').catch(() => {}); await o.waitForTimeout(300);
    await o.click('#fpAddT'); await o.waitForTimeout(500);
    await o.fill('#fp-n', '92'); await o.dispatchEvent('#fp-n', 'change'); await o.waitForTimeout(500);
    await o.click('[data-shape="circle"]').catch(() => {}); await o.waitForTimeout(400);
    const drawn = await o.$$eval('#fpSvg [data-n]', els => els.map(e => e.dataset.n));
    step(drawn.includes('91') && drawn.includes('92'), 'editor draws tables 91 and 92 before save', drawn.join(','));
    const issuesTxt = await o.$eval('.fp-issues', e => e.innerText).catch(() => '');
    step(!issuesTxt, 'editor raises no plan issues', issuesTxt);
    await o.screenshot({ path: `${OUT}/o1-floorplan.png`, fullPage: true });
    await o.click('#fpSave'); await o.waitForTimeout(3500);
    const w = lastWrite(/\/owner\/floorplan/);
    step(w?.status === 200, 'Save the plan lands', `${JSON.stringify(w)} toast=${await toast()}`);
    const fp = await api('/api/owner/floorplan', { token: tok });
    const z = (fp.body.zones || []).find(z => z.name === 'TEST walk');
    step(!!z && z.tables.map(t => t.n).sort().join() === '91,92', 'API read-back: TEST walk zone with 91, 92', JSON.stringify(fp.body.zones));
    ssave({ zone: z?.id });
    await closeSheet(o);
    // reopen: the editor shows what the hub holds
    txt = await tile(o, 'floorPlan');
    step(/TEST walk/.test(txt), 'editor re-opens with the saved TEST room', txt.slice(0, 160));
    await csp(o, 'floorplan'); await closeSheet(o);

    // ── staff: invite a TEST waiter and a TEST kitchen through the console ──
    const stamp = Date.now().toString(36);
    for (const [role, key] of [['waiter', 'waiter'], ['kitchen', 'kitchen']]) {
      await tile(o, 'staff');
      await o.click('#invite'); await o.waitForTimeout(1200);
      const email = `test-walk-${role}-${stamp}@example.com`;
      await o.fill('#i-email', email); await o.fill('#i-name', `TEST walk ${role}`);
      await o.selectOption('#i-role', role);
      await o.click('#iGo'); await o.waitForTimeout(3000);
      const code = await o.$eval('#iCode', e => e.textContent.trim()).catch(() => '');
      const w2 = lastWrite(/staff\/invite/);
      step(!!code && w2?.status === 200, `invite TEST ${role} shows a code once`, `${w2?.status} code ${code ? 'shown (' + code.length + ' chars)' : 'MISSING'}`);
      ssave({ [key]: { email, code, password: `Tw-${stamp}-${role}-pass!`, name: `TEST walk ${role}` } });
      await o.screenshot({ path: `${OUT}/o2-invite-${role}.png` });
      await closeSheet(o);
    }
    const st = await api('/api/owner/staff', { token: tok });
    step((st.body.invites || []).filter(i => /TEST walk/.test(i.name)).length === 2, 'API: two TEST invites waiting', JSON.stringify(st.body.invites));

    // ── table QR page ──
    txt = await tile(o, 'tableQr', 4500);
    const cards = await o.$$eval('#sheetIn .qr-card', els => els.map(e => e.innerText.replace(/\s+/g, ' ')));
    const imgs = await o.$$eval('#sheetIn .qr-card img', els => els.map(e => e.naturalWidth));
    step(cards.length >= 2 && imgs.every(w => w > 0), 'table QR page shows a drawn code per TEST table', `${JSON.stringify(cards)} widths=${imgs}`);
    await o.screenshot({ path: `${OUT}/o3-tableqr.png`, fullPage: true });
    await csp(o, 'tableqr');
    const qr = await api('/api/owner/tables/qr', { token: tok });
    const q91 = (qr.body.tables || []).find(t => t.n === 91);
    step(!!q91?.url && /[?&]t=/.test(q91.url), 'API: table 91 has a signed link', q91?.url?.replace(/\.[0-9a-f]{16}$/, '.<sig>'));
    ssave({ qr91: q91?.url });
    const svg = await fetch(`${HOST}/api/owner/tables/${encodeURIComponent(q91?.zone || '')}/91/qr.svg`, { headers: { authorization: 'Bearer ' + tok } });
    step(svg.status === 200 && /svg/.test(svg.headers.get('content-type') || ''), 'one-code SVG download route', `${svg.status} ${svg.headers.get('content-type')}`);
    await closeSheet(o);
  }

  if (PHASE === 'bookings') {
    const day = S.bookDay ?? 0;
    await tile(o, 'bookings', 4000);
    await o.click(`[data-day="${day}"]`).catch(() => {}); await o.waitForTimeout(3500);
    const rows = await o.$$eval('.rs-row', els => els.map(e => e.innerText.replace(/\s+/g, ' ')));
    const b1 = rows.find(r => /TEST walk B1/.test(r)), b2 = rows.find(r => /TEST walk B2/.test(r));
    step(!!b1 && !!b2, 'Bookings screen lists the two guest bookings', JSON.stringify(rows.filter(r => /TEST/.test(r))));
    await o.screenshot({ path: `${OUT}/o4-bookings.png`, fullPage: true });
    const rsv = async (from, to) => (await api(`/api/owner/reservations?from=${from}&to=${to}`, { token: tok })).body.reservations || [];
    const slotRange = [S.b1?.slotMin - 1440, S.b1?.slotMin + 1440];
    const byId = async id => (await rsv(...slotRange)).find(r => r.id === id) || (await rsv(S.b3?.slotMin - 1440, S.b3?.slotMin + 1440)).find(r => r.id === id);
    // confirm B1
    const btn = async (id, to) => o.$(`[data-id="${id}"][data-to="${to}"]`);
    let x = null;
    if (process.env.ONLY_B3) { step(null, 'B1/B2 skipped (ONLY_B3)'); } else {
    x = await btn(S.b1?.id, 'CONFIRMED');
    step(!!x, 'B1 offers Confirm', S.b1?.id);
    await x?.click(); await o.waitForTimeout(3500);
    step((await byId(S.b1?.id))?.status === 'CONFIRMED', 'Confirm B1 -> CONFIRMED (API)', `${JSON.stringify(lastWrite(/reservations/))} status=${(await byId(S.b1?.id))?.status}`);
    // decline B2 (asks a reason)
    x = await btn(S.b2?.id, 'DECLINED');
    step(!!x, 'B2 offers Decline', S.b2?.id);
    await x?.click(); await o.waitForTimeout(1200);
    await o.fill('#cf-reason', 'TEST walk, not a real booking').catch(() => {});
    await o.click('#cfYes').catch(e => step(false, 'decline confirm', e.message.slice(0, 80))); await o.waitForTimeout(3500);
    step((await byId(S.b2?.id))?.status === 'DECLINED', 'Decline B2 -> DECLINED (API)', `${JSON.stringify(lastWrite(/reservations/))} status=${(await byId(S.b2?.id))?.status}`);
    }
    // phone booking B3, tomorrow 20:00, table 92 -- lands CONFIRMED, then cancel it
    const nowMin = Math.floor(Date.now() / 60000);
    const findB3 = async (id = null) => (await rsv(nowMin, nowMin + 3 * 1440)).find(r => (id ? r.id === id : !/CANCELLED|DECLINED|NO_SHOW/.test(r.status)) && (r.name === 'TEST walk B3' || r.contactName === 'TEST walk B3'));   // an earlier run's ENDED B3 is not this run's
    await closeSheet(o);
    await tile(o, 'bookings', 3500);
    await o.click('[data-day="1"]'); await o.waitForTimeout(3000);
    let r3 = await findB3();
    if (!r3) {
      await o.click('#rsNew'); await o.waitForTimeout(1500);
      await o.fill('#rn-name', 'TEST walk B3'); await o.fill('#rn-phone', '+355 69 000 0033');
      await o.fill('#rn-time', '20:00');
      const opts = await o.$$eval('#rn-table option', els => els.map(e => e.value));
      const t92 = opts.find(v => /\|92$/.test(v));
      step(!!t92, 'New booking offers the TEST tables', opts.join(','));
      if (t92) await o.selectOption('#rn-table', t92);
      await o.click('#rnGo');
      for (let i = 0; i < 24 && !lastWrite(/reservations$/); i++) await o.waitForTimeout(500);   // bounded: 12 s for the POST's answer
      await o.waitForTimeout(1500);
      const w3 = lastWrite(/reservations$/);
      step(w3?.status === 200, 'phone booking B3 lands', `${w3?.status} ${w3?.body?.replace(/"access_token":"[^"]+"/, '"access_token":"…"').slice(0, 200)}`);
      r3 = await findB3();
    } else step(null, 'B3 already booked by an earlier run', r3.id);
    const rowsB3 = await o.$$eval('.rs-row', els => els.map(e => e.innerText.replace(/\s+/g, ' ')));
    step(rowsB3.some(r => /TEST walk B3/.test(r)), 'Bookings (tomorrow) lists B3', JSON.stringify(rowsB3.filter(r => /TEST/.test(r))));
    step(r3?.status === 'CONFIRMED', 'B3 is CONFIRMED (a venue booking needs no confirm) (API)', JSON.stringify(r3));
    ssave({ b3: { id: r3?.id, slotMin: r3?.slotMin } });
    x = r3?.id ? await btn(r3.id, 'CANCELLED_BY_VENUE') : null;
    step(!!x, 'B3 offers Cancel');
    await x?.click(); await o.waitForTimeout(1200);
    await o.fill('#cf-reason', 'TEST walk cleanup').catch(() => {});
    await o.click('#cfYes').catch(() => {}); await o.waitForTimeout(3500);
    const r3b = await findB3(r3?.id);
    step(r3b?.status === 'CANCELLED_BY_VENUE', 'Cancel B3 -> CANCELLED_BY_VENUE (API)', JSON.stringify(r3b));
    await o.screenshot({ path: `${OUT}/o5-bookings-after.png`, fullPage: true });
    await csp(o, 'bookings');
  }

  if (PHASE === 'orders') {
    const want = async (id, st) => (await ownerOrder(tok, id))?.status === st;
    const sOf = async id => (await ownerOrder(tok, id))?.status;
    const X = S.orders?.X, P = S.orders?.P, D = S.orders?.D;
    await o.click('#nav [data-tab="orders"]'); await o.waitForTimeout(3000);
    for (const [k, id] of [['X', X], ['P', P], ['D', D]]) step(!!(await o.$(`.orow[data-o="${id}"]`)), `orders list shows TEST ${k}`, id);
    await o.screenshot({ path: `${OUT}/o6-orders.png`, fullPage: true });
    // reject X (and any extra TEST order a crashed run left PENDING) from the row (asks a reason)
    for (const id of [X, ...(S.extra || [])]) {
      if ((await sOf(id)) !== 'PENDING') { step(null, `${id.slice(0, 8)} not PENDING`, await sOf(id)); continue; }
      await o.click(`[data-act="reject"][data-o="${id}"]`).catch(e => step(false, `reject button on ${id.slice(0, 8)}`, e.message.slice(0, 80)));
      await o.waitForTimeout(1200);
      await o.fill('#cf-reason', 'TEST walk, not a real order').catch(() => {});
      await o.click('#cfYes').catch(() => {}); await o.waitForTimeout(3500);
      step(await want(id, 'REJECTED'), `reject ${id.slice(0, 8)} -> REJECTED (API)`, `${JSON.stringify(lastWrite(/orders\/.*\/action/))} status=${await sOf(id)}`);
    }
    // P and D through the kitchen from the rows
    for (const id of [P, D]) for (const [act, st] of [['confirm', 'CONFIRMED'], ['preparing', 'PREPARING'], ['ready', 'READY']]) {
      const bt = await o.$(`.orow [data-act="${act}"][data-o="${id}"]`);
      if (!bt) { step(false, `${act} button on ${id.slice(0, 8)}`, 'not on screen'); break; }
      await bt.click(); await o.waitForTimeout(3500);
      step(await want(id, st), `console ${act} ${id.slice(0, 8)} -> ${st}`, `status=${await sOf(id)}`);
    }
    // P is a pickup: READY offers "collected"; we REFUND instead (a TEST sale must not stand)
    await o.click(`.orow[data-o="${P}"]`); await o.waitForTimeout(1500);
    const sheetP = await o.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    step(!!(await o.$(`#sheetIn [data-act="collected"][data-o="${P}"]`)), 'pickup at READY offers Collected', sheetP.slice(0, 200));
    const rf = await o.$(`[data-refund="${P}"]`);
    step(!!rf, 'order sheet offers Refund on READY');
    await rf?.click(); await o.waitForTimeout(1000);
    await o.selectOption('#rf-why', 'other'); await o.fill('#rf-note', 'TEST walk 2026-09-24, not a real sale');
    await o.click('#rfGo'); await o.waitForTimeout(4000);
    step(['REFUNDING', 'COMPENSATED_REFUND'].includes(await sOf(P)), 'refund P started', `${JSON.stringify(lastWrite(/refund$/))} status=${await sOf(P)}`);
    const mb = await o.$(`[data-moneyback="${P}"]`);
    if (mb) { await mb.click(); await o.waitForTimeout(3500); }
    step(await want(P, 'COMPENSATED_REFUND'), 'P money handed back -> COMPENSATED_REFUND', `status=${await sOf(P)}`);
    await o.screenshot({ path: `${OUT}/o7-refunded.png` });
    await closeSheet(o);
    await csp(o, 'orders');
  }

  if (PHASE === 'refunds') {
    const ids = [S.r1, S.r2, ...(process.env.IDS || '').split(',').filter(Boolean)].filter(Boolean);
    await o.click('#nav [data-tab="orders"]'); await o.waitForTimeout(3000);
    for (const id of ids) {
      let s = (await ownerOrder(tok, id))?.status;
      if (s === 'COMPENSATED_REFUND') { step(true, `${id.slice(0, 8)} already COMPENSATED_REFUND`); continue; }
      const row = await o.$(`.orow[data-o="${id}"]`);
      if (!row) { step(false, `row ${id.slice(0, 8)} on the console`, `status=${s}`); continue; }
      await row.click(); await o.waitForTimeout(1500);
      if (s !== 'REFUNDING') {
        const rf = await o.$(`[data-refund="${id}"]`);
        step(!!rf, `${id.slice(0, 8)} sheet offers Refund (${s})`);
        await rf?.click(); await o.waitForTimeout(1000);
        await o.selectOption('#rf-why', 'other'); await o.fill('#rf-note', 'TEST walk 2026-09-24, not a real sale');
        await o.click('#rfGo'); await o.waitForTimeout(4000);
      }
      const mb = await o.$(`[data-moneyback="${id}"]`);
      if (mb) { await mb.click(); await o.waitForTimeout(3500); }
      s = (await ownerOrder(tok, id))?.status;
      step(s === 'COMPENSATED_REFUND', `refund ${id.slice(0, 8)} -> COMPENSATED_REFUND`, `status=${s} last=${JSON.stringify(lastWrite(/refund$/))}`);
      await closeSheet(o);
    }
    const from = Date.now() - 6 * 3600e3;
    const tips = await api(`/api/staff/till/tips?location_id=${LOC}&from_ms=${from}`, { token: tok });
    step(null, 'API tips after refunds (6 h)', JSON.stringify(tips.body).slice(0, 400));
    await csp(o, 'refunds');
  }

  if (PHASE === 'panes') {
    // ── customers: two spellings of one phone are one row; link then unlink ──
    const list = async () => (await api(`/api/owner/customers?sort=recent&location_id=${LOC}`, { token: tok })).body.customers || [];
    await tile(o, 'customers', 4000);
    let L = await list();
    const r21 = L.find(c => /21$/.test(c.phone || '') && /T\. W\./.test(c.name || '')) || L.find(c => /0021$/.test((c.phone || '').replace(/\D/g, '')));
    const r22 = L.find(c => /0022$/.test((c.phone || '').replace(/\D/g, '')));
    step(!!r21 && r21.orders >= 2, 'API: +355 69 000 0021 and 069 000 0021 are one customer', JSON.stringify(r21));
    step(!!r22, 'API: 0022 is its own customer', JSON.stringify(r22));
    const uiRows = await o.$$eval('#cuBody [data-key]', els => els.map(e => ({ key: e.dataset.key, text: e.innerText.replace(/\s+/g, ' ') })));
    step(uiRows.some(r => r.key === r21?.key) && uiRows.some(r => r.key === r22?.key), 'Customers screen shows both TEST rows', JSON.stringify(uiRows.filter(r => r.key === r21?.key || r.key === r22?.key)));
    if (r21 && r22) {
      await o.click(`#cuBody [data-key="${r22.key}"]`); await o.waitForTimeout(1500);
      await o.click('#rvCard'); await o.waitForTimeout(1500);
      await o.fill('#cd-lreason', 'TEST walk: same person');
      await o.click('#cdLink'); await o.waitForTimeout(3000);
      const picks = await o.$$eval('#cdPick [data-to]', els => els.map(e => e.dataset.to));
      step(picks.includes(r21.key), 'Link to... lists the 0021 row', `${picks.length} candidates`);
      await o.click(`#cdPick [data-to="${r21.key}"]`).catch(() => {}); await o.waitForTimeout(3000);
      L = await list();
      const m = L.find(c => (c.linked || []).includes(r22.key));
      step(!!m, 'API after link: 0022 linked into 0021', JSON.stringify(m));
      await closeSheet(o); await closeSheet(o);
      await tile(o, 'customers', 4000);
      await o.click(`#cuBody [data-key="${m?.key || r21.key}"]`).catch(() => {}); await o.waitForTimeout(1500);
      await o.click('#rvCard').catch(() => {}); await o.waitForTimeout(1500);
      await o.fill('#cd-lreason', 'TEST walk: undo').catch(() => {});
      const ub = await o.$(`[data-unlink="${r22.key}"]`);
      step(!!ub, 'card lists the linked 0022 with Unlink');
      await ub?.click(); await o.waitForTimeout(3000);
      L = await list();
      step(!L.some(c => (c.linked || []).includes(r22.key)), 'API after unlink: 0022 stands alone again', JSON.stringify(L.filter(c => [r21.key, r22.key].includes(c.key))));
      await closeSheet(o); await closeSheet(o);
    }
    await csp(o, 'customers');

    // ── campaigns: create a TEST one with a fake template, preview the count, never send ──
    await tile(o, 'campaigns', 3000);
    await o.click('#cpNew'); await o.waitForTimeout(1200);
    await o.fill('#cp-name', 'TEST walk campaign');
    await o.fill('#cp-text', 'TEST walk -- preview only, never sent');
    await o.fill('#cp-tpl', 'test_walk_fake_template');
    await o.click('#cpSave'); await o.waitForTimeout(3500);
    const wc = lastWrite(/owner\/campaigns$/);
    let camp = null; try { camp = JSON.parse(wc?.body || '{}').campaign; } catch {}
    step(wc?.status === 200 && !!camp?.id, 'TEST campaign saved', `${wc?.status} ${camp?.id}`);
    ssave({ campaign: camp?.id });
    await o.click('#cpPrev').catch(() => {}); await o.waitForTimeout(3500);
    const wp = lastWrite(/preview$/);
    const out = await o.$eval('#cpOut', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    const sendOn = await o.$eval('#cpSend', e => !e.disabled).catch(() => null);
    step(wp?.status === 200 || (wp && wp.status >= 400 && wp.status < 500), 'preview answers a count or a clear refusal', `${JSON.stringify(wp)} screen="${out}" toast=${await toast()}`);
    step(null, 'Send button enabled after preview (NOT pressed)', String(sendOn));
    const cd = camp?.id ? await api(`/api/owner/campaigns/${camp.id}`, { token: tok }) : null;
    step((cd?.body?.report?.sent || 0) === 0, 'API: campaign sent nothing', JSON.stringify(cd?.body?.report));
    await o.screenshot({ path: `${OUT}/o8-campaign.png`, fullPage: true });
    await closeSheet(o);

    // ── stamp card settings block renders (not enabled) ──
    await tile(o, 'promos', 4000);
    const stamp = await o.$eval('#pStamps', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
    const on = await o.$eval('#st-on', e => e.checked).catch(() => null);
    step(stamp.length > 20 && on === false, 'stamp card block renders, switched off', `${stamp.slice(0, 200)} on=${on}`);
    const sv = await api('/api/owner/settings', { token: tok });
    step(null, 'API loyalty.stamps.enabled', JSON.stringify(sv.body.values?.['loyalty.stamps.enabled'] ?? null));
    await closeSheet(o);

    // ── recipe / supply CSV import: DRY RUN only ──
    for (const [tab, btnSel, file] of [['menu', '#mRecipes', 'recipes.csv'], ['stock', '#importSupplies', 'supplies.csv']]) {
      await o.click(`#nav [data-tab="${tab}"]`); await o.waitForTimeout(3000);
      const bt = await o.$(btnSel);
      if (!bt) { step(false, `${tab}: import button ${btnSel}`, 'missing'); continue; }
      await bt.click(); await o.waitForTimeout(1200);
      await o.setInputFiles('#bkFile', `/root/dowiz/tools/recipes/samples/${file}`); await o.waitForTimeout(800);
      await o.click('#bkDry'); await o.waitForTimeout(5000);
      const rep = await o.$eval('#bkOut', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
      const w = lastWrite(/import/);
      step(w?.status === 200 && !/apply=1/.test(w.url), `${file} DRY RUN reports`, `${w?.status} ${w?.url} :: ${rep.slice(0, 300)}`);
      step(null, `${file} Apply button enabled (NOT pressed)`, String(await o.$eval('#bkApply', e => !e.disabled).catch(() => null)));
      await o.screenshot({ path: `${OUT}/o9-dry-${file}.png`, fullPage: true });
      await closeSheet(o);
    }
    step(!writes.some(w => /apply=1/.test(w.url)), 'no import was applied', writes.filter(w => /import/.test(w.url)).map(w => w.url).join(' '));

    // ── printer, exceptions, eBills (read), fiscal, health ──
    for (const key of ['printer', 'exceptions', 'ebills', 'health']) {
      const before = issues.length;
      const txt = await tile(o, key, 4500);
      step(txt.length > 30 && issues.length === before, `pane ${key} renders without errors`, txt.slice(0, 350));
      await o.screenshot({ path: `${OUT}/o10-${key}.png`, fullPage: true });
      if (key === 'ebills') {
        await o.click('#ebFiscal'); await o.waitForTimeout(4000);
        const fx = await o.$eval('#sheetIn', e => e.innerText.replace(/\s+/g, ' ')).catch(() => '');
        const pill = await o.$eval('#fxBody .ui-badge [data-t]', e => e.dataset.t || e.textContent).catch(() => '');
        step(pill === 'fx_off' || /off|jo aktiv|вимкн/i.test(fx), 'fiscal pane says sending is switched off', `${pill} :: ${fx.slice(0, 300)}`);
        const phrase = await o.$eval('label[for="fx-phrase"] b', e => e.textContent).catch(() => '');
        if (phrase) {
          await o.fill('#fx-phrase', phrase);
          await o.click('#fxArm'); await o.waitForTimeout(3500);
          const wa = lastWrite(/fiscal\/ebills/);
          step(wa?.status === 409, 'arming with the exact phrase is REFUSED 409', `${JSON.stringify(wa)} toast=${await toast()}`);
        } else step(false, 'arming phrase shown', 'no phrase');
        const fr = await api(`/api/owner/fiscal?location_id=${LOC}`, { token: tok });
        if (fr.body?.arming?.armed) {
          const d = await api(`/api/owner/fiscal/ebills?location_id=${LOC}`, { method: 'POST', token: tok, body: { armed: false } });
          step(false, 'FISCAL WAS ARMED -- disarmed at once', `${d.status} ${JSON.stringify(d.body).slice(0, 200)}`);
        }
        step(fr.body?.armed === false && fr.body?.arming?.armed === false, 'API read-back: fiscal not armed', JSON.stringify({ armed: fr.body?.armed, arming: fr.body?.arming }));
        await o.screenshot({ path: `${OUT}/o11-fiscal.png`, fullPage: true });
      }
      if (key === 'health') {
        const h = await api('/api/owner/health', { token: tok });
        step(h.status === 200, 'API health', JSON.stringify({ errors: (h.body.errors || []).slice(0, 3), orders: h.body.orders }).slice(0, 500));
      }
      await csp(o, `pane-${key}`);
      await closeSheet(o);
    }
    const ex = await api(`/api/owner/exceptions?location_id=${LOC}&from_ms=${Date.now() - 6 * 3600e3}`, { token: tok });
    step(ex.status === 200, 'API exceptions (6 h) lists the TEST refunds', JSON.stringify(ex.body).slice(0, 600));
  }

  if (PHASE === 'cleanup') {
    // ── the floor plan: remove the TEST room in the editor, save the empty plan ──
    await tile(o, 'floorPlan');
    const chips = await o.$$eval('#sheetIn [data-zi]', els => els.map(e => e.textContent.trim()));
    const i = chips.findIndex(c => /TEST walk/.test(c));
    if (i >= 0) {
      await o.click(`[data-zi="${i}"]`); await o.waitForTimeout(600);
      await o.click('#fpDelZ'); await o.waitForTimeout(800);
      await o.click('#cfYes').catch(() => {}); await o.waitForTimeout(1200);
      await o.click('#fpSave').catch(e => step(false, 'save after removing the room', e.message.slice(0, 80))); await o.waitForTimeout(3500);
      step(lastWrite(/owner\/floorplan/)?.status === 200, 'empty plan saves', JSON.stringify(lastWrite(/owner\/floorplan/)));
    } else step(null, 'no TEST room left in the editor', chips.join(','));
    const fp = await api('/api/owner/floorplan', { token: tok });
    step((fp.body.zones || []).length === 0, 'API read-back: the plan is EMPTY again', JSON.stringify(fp.body.zones));
    const tb = await api(`/api/public/locations/${LOC}/tables?slotMin=${Math.floor(Date.now() / 60000) + 120}&party=2`);
    step(!(tb.body.zones || []).some(z => (z.tables || []).length), 'storefront /tables shows no tables', JSON.stringify(tb.body).slice(0, 200));
    await closeSheet(o);
    // ── TEST staff: suspend through the staff sheet ──
    const st0 = await api('/api/owner/staff', { token: tok });
    for (const s of (st0.body.staff || []).filter(s => /TEST walk/.test(s.name) && s.active)) {
      await tile(o, 'staff', 3500);
      await o.click(`[data-s="${s.id}"]`); await o.waitForTimeout(1500);
      await o.click('label.switch:has(#s-active)').catch(async () => { await o.$eval('#s-active', e => { e.checked = false; e.dispatchEvent(new Event('change')); }); });
      await o.waitForTimeout(1200);
      await o.click('#cfYes').catch(() => {}); await o.waitForTimeout(3000);
      await closeSheet(o);
    }
    const st = await api('/api/owner/staff', { token: tok });
    const tst = (st.body.staff || []).filter(s => /TEST walk/.test(s.name));
    step(st.status === 200 && tst.length > 0 && tst.every(s => !s.active), 'API: every TEST staff member is suspended', JSON.stringify(tst));
    for (const k of ['waiter', 'kitchen']) if (S[k]) {
      const l = await api('/api/staff/login', { method: 'POST', body: { email: S[k].email, password: S[k].password } });
      step(l.status === 401 || l.status === 403, `suspended TEST ${k} can no longer sign in (a 503 proves nothing)`, `${l.status} ${JSON.stringify(l.body).slice(0, 120)}`);
    }
    step(null, 'TEST invites left', JSON.stringify(st.body.invites));
    await csp(o, 'cleanup');
  }
  await csp(o, PHASE);
} catch (e) {
  step(false, `phase ${PHASE} threw`, e.stack?.slice(0, 400));
  await o.screenshot({ path: `${OUT}/owner-${PHASE}-crash.png` }).catch(() => {});
}
fs.appendFileSync(`${OUT}/writes-owner-${PHASE}.json`, JSON.stringify(writes, null, 1));
await end(b);
