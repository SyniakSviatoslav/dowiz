// "More" -- everything that is not the day's service, as one list of rows,
// each a sheet: marketing (promo codes, posts, social), analytics, customers,
// and the settings a venue sets once (the venue itself, hours, delivery,
// payments, notifications, order channels, brand, features, API keys, health,
// backup). The list is the map; nothing is buried under a fold.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, money, moneyEl, busy, confirm, switchEl, store, day, ago, clock, hydrate } from '/admin/core.js';
import { retranslate, lang, LANGS } from '/admin/i18n.js';
import { loadVenue, loadStaff, rerender } from '/admin/app.js';
import { openCard, cardLine } from '/admin/customers.js';
import { ui, k, btn, iconBtn, field, input, select, check, pill, pillBtn, empty, loading, rowBtn, rowDiv, chips, press } from '/admin/parts.js';
import { openMcp as openMcpSheet } from '/admin/mcp.js';

/// The rows, in groups, with the sheet each opens.
const GROUPS = [
  ['learnGroup', [['learn', 'player-play', openLearn]]],
  ['inbox', [['inbox', 'message-2', openInbox]]],
  ['roomGroup', [['bookings', 'tools-kitchen-2', openBookings], ['floorPlan', 'category', openFloorPlan]]],
  ['marketing', [['promos', 'ticket', openPromos], ['posts', 'send', openPosts], ['campaigns', 'brand-whatsapp', openCampaigns], ['social', 'sparkles', openSocial]]],
  ['analytics', [['analytics', 'chart-bar', openAnalytics], ['customers', 'user', openCustomers], ['staff', 'apron', openStaff], ['exceptions', 'alert-triangle', openExceptions]]],
  ['settings',  [['integrations', 'check', openIntegrations], ['ebills', 'receipt', openEbills], ['printer', 'receipt', openPrinter], ['tableQr', 'receipt', openTableQr], ['preview', 'eye', openPreview], ['venue', 'home', openVenue], ['hours', 'clock', openHours], ['deliveryTerms', 'bike', openDelivery], ['deliveryArea', 'map-pin', openZones], ['payments', 'coin-hole', openPayments],
                 ['notifications', 'brand-telegram', openNotifications], ['channels', 'scroll', openChannels], ['mcp', 'cube-3d-sphere', openMcp], ['cloud', 'cloud-upload', openCloud], ['branding', 'fan', openBranding],
                 ['features', 'tools-kitchen-2', openFeatures], ['assistant', 'sparkles', openAssistant], ['apiKeys', 'key', openKeys], ['activation', 'check', openActivation], ['health', 'cube-3d-sphere', openHealth], ['dpa', 'shield-check', openDpa]]],
];
/// The analytics windows the hub answers, in days.
const WINDOWS = [7, 30];
/// The hours grid: minutes of a day, and the minute steps a venue picks from.
const DAY_MIN = 24 * 60;
/// Promo kinds the hub knows.
const PROMO_KINDS = ['percent', 'fixed'];
/// One day, for a promo window that ends at the NEXT local midnight (half-open).
const DAY_MS = 24 * 60 * 60 * 1000;
/// What a promo is right now, from its own fields (the hub sends `status` too).
const promoStatus = p => p.status || (!p.active ? 'inactive' : p.maxUses && (p.used || 0) >= p.maxUses ? 'exhausted' : p.fromMs && p.fromMs > Date.now() ? 'scheduled' : p.untilMs && p.untilMs < Date.now() ? 'expired' : 'active');
/// A photograph's max side for the logo upload.
const LOGO_MAX_PX = 800;

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabMore"></p><h1>${esc(S.venue?.name || '')}</h1></div></div>
    <p class="screen-hint" data-t="moreHint"></p>
    ${GROUPS.map(([g, rows]) => `<section class="group"><p class="eyebrow" data-t="${g}"></p><div class="tiles">
      ${rows.map(([key, ic]) => rowBtn({ cls: 'tile', leading: `<span class="tile-ic">${icon(ic)}</span>`, title: k(key), sub: `<span data-t="${key}Sub"></span>`, data: { open: key }, tour: 'more.tile.' + key })).join('')}</div></section>`).join('')}
    <p class="hint mono">${esc(store.loc)} · ${esc(S.venue?.slug || '')}</p>`;
  host.onclick = e => { const r = e.target.closest('[data-open]'); if (!r) return; for (const [, rows] of GROUPS) for (const [key, , fn] of rows) if (key === r.dataset.open) fn(); };
}

const head = (eyebrow, title) => `<p class="eyebrow" data-t="${eyebrow}"></p><h2 data-t="${title}"></h2>`;
const fail = e => toast(String(e.message || e));
const paint = () => { retranslate($('#sheetIn')); hydrate($('#sheetIn')); };
/// The sheet's one main action, and an info row (icon, title, sub, trailing).
const saveBtn = (id, tour, key = 'save', ic = 'check') => btn({ id, variant: 'primary', icon: ic, key, tour });
const info = (ic, o) => rowDiv({ leading: icon(ic), ...o });
const onOff = (on, tone = 'warn') => pill(on ? 'ok' : tone, { key: on ? 'on' : 'off' });

// ── lessons (/lib/learn.js): the console's own, run as tours on this page ──
async function learnFor(){
  const [{ createLearn, loadLessons }, { createGuide }] = await Promise.all([import('/lib/learn.js'), import('/lib/guide.js')]);
  const lessons = await loadLessons(); if (!lessons.length) return null;
  return createLearn({ role: 'owner', lessons, lang: () => lang, createGuide, toast, words: { title: t('learn'), new: t('learnNew'), done: t('learnDone'),
    paused: t('learnPaused'), watch: t('learnWatch'), writes: t('learnWrites'), steps: n => t('learnSteps').replace('{n}', n), empty: t('learnEmpty'), offline: t('learnOffline') } });
}
async function openLearn(){
  const L = await learnFor(); if (!L) return toast(t('learnOffline'));
  sheet(`${head('learnGroup', 'learn')}<div id="learnList">${L.renderList()}</div>`, { name: 'learn' });
  L.bindList($('#learnList'), () => closeSheet());
}
/// `/admin/#learn=O1b` opens that lesson (the wiki links here).
export async function learnFromHash(){ if (/learn=/.test(location.hash)) (await learnFor())?.deepLink(location.hash); }

// ── the till link (ebills.al) ───────────────────────────────────────────────
async function openEbills(){ (await import('/admin/ebills.js')).open(); }
// The data processing agreement (P9): read and accept it for the venue.
async function openDpa(){ (await import('/admin/dpa.js')).open(); }

// ── exceptions: voids, comps, refunds, pay-outs, never a score ─────────────────
async function openExceptions(){ (await import('/admin/exceptions.js')).open(); }

// ── campaigns (C6): segment, preview the count, send only to the consented ───
async function openCampaigns(){ (await import('/admin/campaigns.js')).open(); }

// ── the kitchen printer (print.kitchen): the setting had no control ─────────
async function openPrinter(){ (await import('/admin/printer.js')).open(); }

// ── table QR codes (A9): order at the table, a waiter confirms ──────────────
async function openTableQr(){ (await import('/admin/tableqr.js')).open(); }

// ── bookings and the floor plan (A2/A3): the day's list, the room's tables ───
async function openBookings(){ (await import('/admin/bookings.js')).open(); }
async function openFloorPlan(){ (await import('/admin/floorplan.js')).open(); }

// ── staff ───────────────────────────────────────────────────────────────────
async function openStaff(){
  const { render } = await import('/admin/staff.js');
  sheet(`<div id="staffIn"></div>`, { name: 'staff' });
  await loadStaff();
  await render($('#staffIn'));
}

// ── marketing ───────────────────────────────────────────────────────────────
async function openPromos(){
  sheet(`${head('marketing', 'promos')}<div id="pList">${loading()}</div>
    <div class="btn-row">${btn({ id: 'pNew', variant: 'primary', icon: 'plus', key: 'add', tour: 'promos.new' })}</div><div id="pStamps"></div>`, { name: 'promos' });
  import('/admin/stamps.js').then(m => m.mount($('#pStamps'))).catch(fail);
  const draw = async () => {
    let d; try { d = await api('/owner/promotions'); } catch (e) { return fail(e); }
    const list = d.promotions || [];
    $('#pList').innerHTML = list.length ? `<div class="rows">${list.map(p => { const stt = promoStatus(p); return info('ticket', { cls: stt === 'active' ? '' : 'off', title: p.code, tour: 'promos.row', sub: `${p.kind === 'percent' ? `−${p.value}%` : `−${money(p.value)}`}${p.minOrder ? ` · ${t('minOrder')} ${money(p.minOrder)}` : ''} · ${p.used || 0}${p.maxUses ? '/' + p.maxUses : ''}${p.fromMs ? ` · ${t('when')} ${day(p.fromMs)}` : ''}${p.untilMs ? ` · ${t('until')} ${day(p.untilMs - 1)}` : ''}`,
      trailing: pillBtn(stt === 'active' ? 'ok' : stt === 'scheduled' ? 'warn' : '', { key: 'promo_' + stt, data: { flip: p.code }, tour: 'promos.flip' }) + iconBtn({ icon: 'trash', ariaKey: 'remove', variant: 'plain', data: { del: p.code }, tour: 'promos.delete' }) }); }).join('')}</div>` : empty('ticket', { key: 'none' });
    paint();
    for (const b of $$('[data-flip]', $('#pList'))) b.onclick = async () => { const p = list.find(x => x.code === b.dataset.flip); if (!p) return; try { await busy(b, () => post('/owner/promotions', { ...p, active: !p.active, used: undefined, status: undefined })); draw(); } catch (e) { fail(e); } };
    for (const b of $$('[data-del]', $('#pList'))) b.onclick = async () => { const c = await confirm(t('remove'), b.dataset.del, { danger: true }); if (!c) return openPromos(); try { await post(`/owner/promotions/${encodeURIComponent(b.dataset.del)}/delete`, withLoc()); openPromos(); } catch (e) { fail(e); } };
  };
  draw();
  $('#pNew').onclick = () => {
    sheet(`${head('marketing', 'promos')}
      ${field({ id: 'pr-code', key: 'promo', autocapitalize: 'characters', spellcheck: false, tour: 'promos.code' })}
      ${chips({ id: 'prKind', values: PROMO_KINDS.map(x => ({ value: x, label: x === 'percent' ? '%' : (S.venue?.currencyCode || 'ALL') })), value: PROMO_KINDS[0], attr: 'k', tour: 'promos.kind' })}
      <div class="grid2">${field({ id: 'pr-value', key: 'discount', inputmode: 'numeric', tour: 'promos.value' })}${field({ id: 'pr-min', key: 'minOrder', inputmode: 'numeric', tour: 'promos.minOrder' })}</div>
      <div class="grid2">${input({ id: 'pr-from', type: 'date', key: 'when', tour: 'promos.from' })}${input({ id: 'pr-until', type: 'date', key: 'until', tour: 'promos.until' })}</div>
      ${field({ id: 'pr-max', key: 'maxUses', inputmode: 'numeric', tour: 'promos.maxUses' })}
      <div class="btn-row">${saveBtn('prSave', 'promos.save')}</div>`, { name: 'promo' });
    let kind = PROMO_KINDS[0];
    for (const b of $$('[data-k]', $('#sheetIn'))) b.onclick = () => { kind = b.dataset.k; press($$('[data-k]', $('#sheetIn')), b); };
    $('#prSave').onclick = async () => {
      const body = { code: $('#pr-code').value.trim().toUpperCase(), kind, value: Number($('#pr-value').value) || 0, active: true };
      if ($('#pr-min').value) body.minOrder = Number($('#pr-min').value);
      if ($('#pr-max').value) body.maxUses = Number($('#pr-max').value);
      // Local midnight, not UTC: "from the 20th" means the venue's 20th.
      if ($('#pr-from').value) body.fromMs = new Date($('#pr-from').value + 'T00:00').getTime();
      if ($('#pr-until').value) body.untilMs = new Date($('#pr-until').value + 'T00:00').getTime() + DAY_MS;
      try { await busy($('#prSave'), () => post('/owner/promotions', body)); toast(t('saved')); openPromos(); } catch (e) { fail(e); }
    };
  };
}

async function openPosts(){
  sheet(`${head('marketing', 'posts')}<p class="muted small" data-t="autopostHint"></p><div id="postList">${loading()}</div>
    <div class="btn-row">${btn({ id: 'postDraft', variant: 'primary', icon: 'sparkles', key: 'makeDraft', tour: 'posts.draft' })}</div>`, { name: 'posts' });
  const draw = async () => {
    let d; try { d = await api('/owner/posts'); } catch (e) { return fail(e); }
    const list = d.posts || [];
    $('#postList').innerHTML = `<p class="hint">${d.enabled ? `${t('autopost')}: ${t('on')}` : `${t('autopost')}: ${t('off')}`} · ${t('tgChannel')}: ${esc(d.channel || '—')}</p>` +
      (list.length ? `<div class="rows">${list.map(p => rowBtn({ leading: icon('send'), title: p.text.slice(0, 80), data: { post: p.id }, tour: 'posts.row', sub: `${esc(p.about || '')} · <span data-t="${p.state === 'draft' ? 'draft' : p.state === 'published' ? 'published' : p.state === 'failed' ? 'failed' : 'rejectPost'}"></span>${p.error ? ' · ' + esc(p.error) : ''}` })).join('')}</div>` : empty('send', { key: 'noPosts' }));
    paint();
    for (const r of $$('[data-post]', $('#postList'))) r.onclick = () => openPost(list.find(p => p.id === r.dataset.post));
  };
  draw();
  $('#postDraft').onclick = async () => { try { await busy($('#postDraft'), () => post('/owner/posts/draft', withLoc())); draw(); } catch (e) { fail(e); } };
}
function openPost(p){
  if (!p) return;
  sheet(`${head('posts', 'draft')}${field({ id: 'postText', rows: 5, value: p.text, attrs: { 'aria-label': t('draft') }, tour: 'posts.text' })}<p class="hint">${esc(p.about || '')}</p>
    <div class="btn-row">${p.state === 'draft' ? btn({ id: 'postNo', variant: 'danger', icon: 'x', key: 'rejectPost', tour: 'posts.reject' }) + btn({ id: 'postYes', variant: 'primary', icon: 'send', key: 'approve', tour: 'posts.approve' }) : btn({ id: 'postBack', variant: 'ghost', icon: 'arrow-left', key: 'back' })}</div>`, { name: 'post' });
  const yes = $('#postYes'); if (yes) yes.onclick = async () => { try { await busy(yes, () => post(`/owner/posts/${encodeURIComponent(p.id)}/approve`, { text: $('#postText').value.trim() })); toast(t('published')); openPosts(); } catch (e) { fail(e); } };
  const no = $('#postNo'); if (no) no.onclick = async () => { try { await post(`/owner/posts/${encodeURIComponent(p.id)}/reject`, withLoc()); openPosts(); } catch (e) { fail(e); } };
  const back = $('#postBack'); if (back) back.onclick = openPosts;
}

async function openSocial(){
  let s; try { s = await api('/owner/settings'); } catch (e) { return fail(e); }
  const v = s.values || {};
  const igOn = v['social.instagram.token'] === SECRET_SET_MARK && !!(v['social.instagram.user_id'] || '').trim();
  sheet(`${head('marketing', 'social')}<p class="muted small" data-t="autopostHint"></p>
    ${switchEl('so-on', v['social.enabled'] === '1', 'autopost', null, 'social.autopost')}
    ${field({ id: 'so-ch', key: 'tgChannel', value: v['social.telegram.channel'] || '', placeholder: '@channel', tour: 'social.channel' })}
    <p class="eyebrow mt-3" data-t="instagram"></p>
    <div class="rows">${info('sparkles', { cls: igOn ? '' : 'off', title: k('instagram'), sub: `<span data-t="${igOn ? 'instagramOn' : 'socialNotYet'}"></span>`, trailing: onOff(igOn, '') })}</div>
    <p class="hint" data-t="instagramHint"></p>
    ${field({ id: 'ig-token', key: 'instagramToken', autocomplete: 'off', spellcheck: false, placeholder: igOn ? SECRET_SET_MARK : 'EAAB…', tour: 'social.igToken' })}
    ${field({ id: 'ig-user', key: 'instagramUserId', inputmode: 'numeric', value: v['social.instagram.user_id'] || '', tour: 'social.igUser' })}
    <div class="rows mt-3">
      ${['facebook', 'tiktok'].map(n => info('sparkles', { cls: 'off', title: k(n), sub: '<span data-t="socialNotYet"></span>', trailing: pill('', { key: 'comingSoon' }) })).join('')}
    </div>
    <div class="btn-row">${saveBtn('soSave', 'social.save')}</div>`, { name: 'social' });
  $('#soSave').onclick = async () => {
    try {
      await busy($('#soSave'), async () => {
        await post('/owner/settings', { key: 'social.enabled', value: $('#so-on').checked ? '1' : '0' });
        await post('/owner/settings', { key: 'social.telegram.channel', value: $('#so-ch').value.trim() });
        const tok = $('#ig-token').value.trim(); if (tok) await post('/owner/settings', { key: 'social.instagram.token', value: tok });
        await post('/owner/settings', { key: 'social.instagram.user_id', value: $('#ig-user').value.trim() });
      });
      toast(t('saved')); closeSheet();
    } catch (e) { fail(e); }
  };
}

// ── analytics, customers ────────────────────────────────────────────────────
async function openAnalytics(days = WINDOWS[0]){
  sheet(`${head('analytics', 'analytics')}
    ${chips({ values: WINDOWS.map(w => ({ value: w, key: w === 7 ? 'week' : 'month' })), value: days, attr: 'days', tour: 'analytics.window' })}
    <div id="anBody">${loading(2)}</div>`, { name: 'analytics' });
  for (const b of $$('[data-days]', $('#sheetIn'))) b.onclick = () => openAnalytics(Number(b.dataset.days));
  let a; try { a = await api(`/owner/analytics?days=${days}`); } catch (e) { return fail(e); }
  const byDay = a.byDay || [], maxRev = Math.max(1, ...byDay.map(d => d.revenue || 0));
  const byHour = a.byHour || [], maxH = Math.max(1, ...byHour);
  $('#anBody').innerHTML = `
    <div class="stats"><div class="stat"><small data-t="orders7"></small><b>${a.orders ?? 0}</b></div><div class="stat"><small data-t="revenue7"></small><b>${money(a.revenue || 0)}</b></div>
      <div class="stat"><small data-t="avgCheck"></small><b>${money(a.averageOrder || 0)}</b></div><div class="stat ${a.rejected ? 'warn' : ''}"><small data-t="rejected"></small><b>${a.rejected ?? 0}</b></div></div>
    <div class="rows">${info('bike', { title: k('delivery'), trailing: `<b class="mono">${a.delivery ?? 0}</b>` })}${info('walk', { title: k('pickup'), trailing: `<b class="mono">${a.pickup ?? 0}</b>` })}</div>
    <p class="eyebrow mt-3" data-t="byDay"></p><div class="bars" role="img" aria-label="${esc(t('byDay'))} · max ${esc(money(maxRev))}">${byDay.map(d => `<i data-h="${Math.round(100 * (d.revenue || 0) / maxRev)}" title="${esc(day(d.at))} · ${esc(money(d.revenue || 0))}"></i>`).join('')}</div>
    <div class="axis"><span>${byDay.length ? day(byDay[0].at) : ''}</span><span>${byDay.length ? day(byDay[byDay.length - 1].at) : ''}</span></div>
    <p class="eyebrow mt-3" data-t="byHour"></p><div class="bars" role="img" aria-label="${esc(t('byHour'))} · max ${maxH}">${byHour.map((n, h) => `<i class="${n === maxH ? 'hi' : ''}" data-h="${Math.round(100 * n / maxH)}" title="${String(h).padStart(2, '0')}:00 · ${n}"></i>`).join('')}</div><div class="axis"><span>00</span><span>12</span><span>23</span></div>
    <p class="eyebrow mt-3" data-t="topDishes"></p><div class="rows">${(a.topProducts || []).slice(0, 8).map(p => info('bowl-chopsticks', { title: p.name || p.id, sub: `<span class="mono">${p.quantity ?? 0} ${esc(t('portions'))}</span>`, trailing: ui.amount(money(p.revenue || 0)) })).join('')}</div>`;
  paint();
}
/// Customers are MASKED by default; a name and phone are revealed one at a
/// time, for a written reason, and every reveal is in a log the owner can read.
const CUSTOMER_SORTS = ['spent', 'orders', 'recent'];
const REVEAL_REASON_MIN = 3;
async function openCustomers(sort = CUSTOMER_SORTS[0]){
  sheet(`${head('analytics', 'customers')}
    ${chips({ values: CUSTOMER_SORTS.map(x => ({ value: x, key: 'sort_' + x })), value: sort, attr: 'sort', tour: 'customers.sort' })}
    <div id="cuBody">${loading()}</div>
    <div class="btn-row">${btn({ id: 'cuCsv', icon: 'download', key: 'exportCsv', tour: 'customers.csv' })}${btn({ id: 'cuLog', icon: 'eye', key: 'revealLog', tour: 'customers.revealLog' })}</div>`, { name: 'customers' });
  for (const b of $$('[data-sort]', $('#sheetIn'))) b.onclick = () => openCustomers(b.dataset.sort);
  let d; try { d = await api(`/owner/customers?sort=${sort}`); } catch (e) { return fail(e); }
  const list = d.customers || [];
  $('#cuBody').innerHTML = list.length ? `<div class="rows">${list.map(c => rowBtn({ leading: icon('user'), title: c.name || c.phone || c.key, data: { key: c.key }, tour: 'customers.row', sub: `<span class="mono">${esc(c.phone || '')} · ${c.orders} · ${money(c.spent || 0)}</span>${cardLine(c) ? `<br>${esc(cardLine(c))}` : ''}`,
    trailing: `${c.lastAt ? `<small class="muted">${esc(day(c.lastAt))}</small>` : ''}${icon('chevron-right', 'chev')}` })).join('')}</div>` : empty('user', { key: 'none' });
  paint();
  for (const b of $$('[data-key]', $('#cuBody'))) b.onclick = () => openReveal(b.dataset.key, list.find(c => c.key === b.dataset.key), sort);
  $('#cuCsv').onclick = () => {
    const cell = v => { const x = String(v ?? ''); return /[",\n;]/.test(x) ? '"' + x.replace(/"/g, '""') + '"' : x; };
    const rows = [[t('customer'), t('phone'), t('orders7'), t('total'), t('lastSeen')].map(cell).join(','), ...list.map(c => [c.name, c.phone, c.orders, c.spent ?? 0, c.lastAt ? new Date(c.lastAt).toISOString() : ''].map(cell).join(','))];
    const blob = new Blob(['\ufeff' + rows.join('\n')], { type: 'text/csv;charset=utf-8' });
    const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = `dowiz-customers-${new Date().toISOString().slice(0, 10)}.csv`; a.click(); requestAnimationFrame(() => URL.revokeObjectURL(a.href));
  };
  $('#cuLog').onclick = openRevealLog;
}
async function openReveal(key, c, sort){
  sheet(`${head('customers', 'reveal')}<p class="muted small" data-t="revealHint"></p>
    <div class="fact">${icon('user')}<span class="v">${esc(c?.name || key)}<br><small class="mono">${esc(c?.phone || '')}</small></span></div>
    ${field({ id: 'rv-reason', key: 'revealReason', autocomplete: 'off', tour: 'customers.revealReason' })}
    <div class="btn-row">${btn({ id: 'rvGo', variant: 'primary', icon: 'eye', key: 'reveal', tour: 'customers.reveal' })}${btn({ id: 'rvCard', icon: 'user', key: 'card', tour: 'customers.card' })}</div><div id="rvOut"></div>`, { name: 'reveal' });
  $('#rvCard').onclick = () => openCard(c || { key }, rec => openReveal(key, { ...c, ...{ note: rec.note, tags: rec.tags, allergens: rec.allergens, lang: rec.lang, usualTable: rec.usual_table, birthdayMd: rec.birthday_md } }, sort), () => openCustomers(sort));
  $('#rvGo').onclick = async () => {
    const reason = $('#rv-reason').value.trim(); if (reason.length < REVEAL_REASON_MIN) return toast(t('required'));
    try {
      const r = await busy($('#rvGo'), () => post(`/owner/customers/${encodeURIComponent(key)}/reveal`, { reason }));
      $('#rvOut').innerHTML = `<div class="fact mt-3">${icon('phone')}<span class="v"><b>${esc(r.name || '')}</b><br>${r.phone ? `<a href="tel:${esc(r.phone)}">${esc(r.phone)}</a>` : ''}</span></div>
        <p class="eyebrow mt-3" data-t="lastOrders"></p>${(r.orders || []).map(o => `<div class="line"><span class="n">${esc(day(o.at || o.created_at_ms || 0))}<small>${esc(o.address || '')}</small></span>${moneyEl(o.total || 0)}</div>`).join('')}`;
      paint();
    } catch (e) { fail(e); }
  };
}
async function openRevealLog(){
  sheet(`${head('customers', 'revealLog')}<div id="rlBody">${loading()}</div>`, { name: 'reveals' });
  let d; try { d = await api('/owner/customers/reveals'); } catch (e) { return fail(e); }
  const list = d.reveals || [];
  $('#rlBody').innerHTML = list.length ? `<div class="rows">${list.map(r => info('eye', { title: r.reason || '', tour: 'customers.revealRow', sub: `<span class="mono">${esc(r.by || '')} · ${esc(day(r.atMs || r.at || 0))} ${esc(clock(r.atMs || r.at || 0))}</span>` })).join('')}</div>` : empty('eye', { key: 'none' });
  paint();
}

// ── settings ────────────────────────────────────────────────────────────────
async function openVenue(){
  const v = S.venue || {};
  sheet(`${head('settings', 'venue')}
    ${field({ id: 'v-name', key: 'venueName', value: v.name || '', tour: 'venue.name' })}
    ${field({ id: 'v-phone', key: 'venuePhone', type: 'tel', value: v.phone || '', tour: 'venue.phone' })}
    ${field({ id: 'v-addr', key: 'venueAddress', value: v.address || '', tour: 'venue.address' })}
    ${switchEl('v-pickup', !!v.pickup, 'pickupOn', null, 'venue.pickup')}
    <div class="btn-row">${saveBtn('vSave', 'venue.save')}</div>`, { name: 'venue' });
  $('#vSave').onclick = async () => {
    try {
      await busy($('#vSave'), async () => {
        await post('/owner/location', withLoc({ name: $('#v-name').value.trim(), phone: $('#v-phone').value.trim(), pickup: $('#v-pickup').checked }));
        const addr = $('#v-addr').value.trim(); if (addr && addr !== v.address) await post('/owner/place', { address: addr });
      });
      toast(t('saved')); await loadVenue(); closeSheet(); rerender();
    } catch (e) { fail(e); }
  };
}
const hhmm = m => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
const toMin = s => { const [h, m] = String(s || '').split(':').map(Number); return Number.isFinite(h) ? h * 60 + (m || 0) : null; };
/// The zones the server will accept. Kept in step with `dowiz_hub::tz::NAMES`
/// by the list the menu payload carries -- a second list here would drift, and
/// the one that drifts is the one the owner picks from.
const ZONES = ['Europe/Tirane', 'Europe/Belgrade', 'Europe/Berlin', 'Europe/Podgorica', 'Europe/Prague', 'Europe/Rome', 'Europe/Skopje', 'Europe/Vienna', 'Europe/Warsaw',
  'Europe/Zagreb', 'Europe/Athens', 'Europe/Bucharest', 'Europe/Chisinau', 'Europe/Kyiv', 'Europe/Sofia', 'Europe/London', 'Europe/Lisbon', 'UTC'];

async function openHours(){
  const week = Array.isArray(S.venue?.hours) && S.venue.hours.length === 7 ? S.venue.hours : Array.from({ length: 7 }, () => []);
  // A WEEKLY SCHEDULE WITHOUT ITS ZONE IS A SCHEDULE IN AN UNSTATED TIMEZONE.
  // These hours used to be read against a hard-coded +2, which is Tirane's
  // SUMMER offset: from 25 October a kitchen closing at 23:00 would have been
  // reported closed from 22:00. The zone belongs on this sheet because it is
  // what these numbers mean.
  const tz = S.venue?.tz || 'Europe/Tirane';
  sheet(`${head('settings', 'hours')}
    ${select({ id: 'h-tz', key: 'timezone', value: tz, options: ZONES.map(z => ({ value: z, label: z.replace('_', ' ') })), tour: 'hours.timezone' })}
    <p class="hint" data-t="timezoneHint"></p>
    ${week.map((w, i) => `<div class="grid3 hours-row">${check({ label: t('day')[i], checked: w.length > 0, data: { day: i }, tour: 'hours.day' })}${input({ type: 'time', value: w.length ? hhmm(w[0].open) : '11:00', data: { open: i }, tour: 'hours.open' })}${input({ type: 'time', value: w.length ? hhmm(w[0].close) : '23:00', data: { close: i }, tour: 'hours.close' })}</div>`).join('')}
    <div class="btn-row">${saveBtn('hSave', 'hours.save')}</div>`, { name: 'hours' });
  $('#hSave').onclick = async () => {
    const hours = week.map((_, i) => { const on = $(`[data-day="${i}"]`).checked; if (!on) return []; const o = toMin($(`[data-open="${i}"]`).value), c = toMin($(`[data-close="${i}"]`).value); return o !== null && c !== null ? [{ open: o, close: c }] : []; });
    const picked = $('#h-tz').value;
    try {
      // Two writes, and the zone goes FIRST: if it is refused the hours are not
      // saved against a zone the owner did not get. `/owner/location` is where
      // the venue record lives; `/owner/place` is where the schedule does.
      if (picked !== tz) await post('/owner/location', withLoc({ timezone: picked }));
      await busy($('#hSave'), () => post('/owner/place', { hours }));
      toast(t('saved')); await loadVenue(); closeSheet();
    } catch (e) { fail(e); }
  };
}
async function openZones(){ (await import('/admin/zones.js')).open(); }

async function openDelivery(){
  const v = S.venue || {};
  sheet(`${head('settings', 'deliveryTerms')}
    <div class="grid2">${field({ id: 'd-fee', key: 'deliveryFee', inputmode: 'numeric', value: v.deliveryFee ?? 0, tour: 'delivery.fee' })}${field({ id: 'd-free', key: 'freeOver', inputmode: 'numeric', value: v.freeDeliveryThreshold ?? '', tour: 'delivery.freeOver' })}</div>
    ${field({ id: 'd-min', key: 'minOrder', inputmode: 'numeric', value: v.minOrder ?? 0, tour: 'delivery.minOrder' })}
    <div class="btn-row">${saveBtn('dSave', 'delivery.save')}</div>`, { name: 'delivery' });
  $('#dSave').onclick = async () => {
    const free = $('#d-free').value.trim();
    try { await busy($('#dSave'), () => post('/owner/location', withLoc({ delivery_fee: Number($('#d-fee').value) || 0, min_order: Number($('#d-min').value) || 0, free_delivery_threshold: free ? Number(free) : null }))); toast(t('saved')); await loadVenue(); closeSheet(); } catch (e) { fail(e); }
  };
}
async function openPayments(){
  const v = S.venue || {}, pay = v.payments || {}, wallets = pay.crypto || [];
  sheet(`${head('settings', 'payments')}
    <div class="rows">
      ${info('cash', { title: k('cash'), trailing: onOff(true), tour: 'payments.cash' })}
      ${info('credit-card', { cls: pay.card ? '' : 'off', title: k('stripe'), sub: `<span data-t="${pay.card ? 'on' : 'stripeNotSet'}"></span>`, trailing: onOff(pay.card), tour: 'payments.stripe' })}
    </div>
    <p class="eyebrow mt-3" data-t="cryptoWallets"></p>
    <div id="wallets">${wallets.map((w, i) => walletRow(w, i)).join('')}</div>
    ${btn({ id: 'wAdd', icon: 'plus', key: 'add', cls: 'mt-2', tour: 'payments.addWallet' })}
    <div class="btn-row">${saveBtn('wSave', 'payments.save')}</div>`, { name: 'payments' });
  $('#wAdd').onclick = () => { $('#wallets').insertAdjacentHTML('beforeend', walletRow({}, $$('.wallet-row').length)); paint(); };
  $('#sheetIn').addEventListener('click', e => { const x = e.target.closest('[data-wdel]'); if (x) x.closest('.wallet-row').remove(); });
  $('#wSave').onclick = async () => {
    const list = $$('.wallet-row', $('#sheetIn')).map(r => ({ network: $('[data-f="network"]', r).value.trim(), symbol: $('[data-f="symbol"]', r).value.trim(), address: $('[data-f="address"]', r).value.trim() })).filter(w => w.address);
    try { await busy($('#wSave'), () => post('/owner/location', withLoc({ crypto_wallets: list }))); toast(t('saved')); await loadVenue(); closeSheet(); } catch (e) { fail(e); }
  };
}
const walletRow = (w, i) => `<div class="wallet-row grid3">${field({ key: 'network', value: w.network || '', placeholder: 'TRC20', data: { f: 'network' }, tour: 'payments.network' })}${field({ key: 'symbol', value: w.symbol || '', placeholder: 'USDT', data: { f: 'symbol' } })}
  <div><p class="ui-label" data-t="walletAddress">${esc(t('walletAddress'))}</p>${ui.inputRow({ label: k('walletAddress'), attrs: { value: w.address || '', data: { f: 'address' } }, action: iconBtn({ icon: 'trash', ariaKey: 'remove', data: { wdel: i } }) })}</div></div>`;

/// What the hub shows for a secret that IS set (dowiz_hub::settings::redacted).
const SECRET_SET_MARK = '\u2022\u2022\u2022\u2022 set';
async function openNotifications(){
  let s = { values: {}, known: [] }; try { s = await api('/owner/settings'); } catch {}
  const v = s.values || {};
  const tokenSet = v['notify.telegram.token'] === SECRET_SET_MARK || !!(S.venue?.telegramBot);
  const chat = v['notify.telegram.chat'] || '';
  const live = tokenSet && !!chat.trim();
  const waSet = v['notify.whatsapp.token'] === SECRET_SET_MARK;
  const waOn = waSet && !!(v['notify.whatsapp.phone_id'] || '').trim();
  sheet(`${head('settings', 'notifications')}
    <div class="rows">
      ${info('brand-telegram', { title: k('telegram'), sub: `<span data-t="${live ? 'tgHow' : 'tgNotSet'}"></span>`, trailing: onOff(live), tour: 'notify.telegramState' })}
    </div>
    ${field({ id: 'n-token', key: 'tgToken', hintKey: 'tgTokenHint', autocomplete: 'off', spellcheck: false, placeholder: tokenSet ? SECRET_SET_MARK : '123456:ABC…', tour: 'notify.tgToken' })}
    ${field({ id: 'n-chat', key: 'ownerChat', hintKey: 'tgChatHint', inputmode: 'numeric', value: chat, placeholder: 'chat id', tour: 'notify.tgChat' })}
    <div class="btn-row">${btn({ id: 'nTest', icon: 'send', key: 'testMessage', tour: 'notify.test' })}${saveBtn('nSave', 'notify.save')}</div>
    <p class="eyebrow mt-3" data-t="whatsapp"></p>
    <div class="rows">${info('phone', { cls: waOn ? '' : 'off', title: k('whatsapp'), sub: `<span data-t="${waOn ? 'whatsappOn' : 'waNotYet'}"></span>`, trailing: onOff(waOn), tour: 'notify.whatsappState' })}</div>
    <p class="hint" data-t="whatsappHint"></p>
    ${field({ id: 'wa-token', key: 'whatsappToken', autocomplete: 'off', spellcheck: false, placeholder: waSet ? SECRET_SET_MARK : 'EAAB…', tour: 'notify.waToken' })}
    <div class="grid2">${field({ id: 'wa-phone', key: 'whatsappPhoneId', inputmode: 'numeric', value: v['notify.whatsapp.phone_id'] || '', tour: 'notify.waPhone' })}${field({ id: 'wa-to', key: 'whatsappTo', inputmode: 'numeric', value: v['notify.whatsapp.to'] || '', tour: 'notify.waTo' })}</div>
    <div class="btn-row">${btn({ id: 'nTest2', icon: 'send', key: 'testMessage', tour: 'notify.test2' })}${btn({ id: 'nSave2', variant: 'secondary', icon: 'check', key: 'save', tour: 'notify.save2' })}</div>`, { name: 'notify' });
  const save = async () => {
    // An empty value CLEARS a setting on the hub, so a token is only sent when typed.
    const tok = $('#n-token').value.trim();
    if (tok) await post('/owner/settings', { key: 'notify.telegram.token', value: tok });
    await post('/owner/settings', { key: 'notify.telegram.chat', value: $('#n-chat').value.trim() });
    const wtok = $('#wa-token').value.trim();
    if (wtok) await post('/owner/settings', { key: 'notify.whatsapp.token', value: wtok });
    await post('/owner/settings', { key: 'notify.whatsapp.phone_id', value: $('#wa-phone').value.trim() });
    await post('/owner/settings', { key: 'notify.whatsapp.to', value: $('#wa-to').value.trim() });
  };
  const test = async b => { try { const r = await busy(b, async () => { await save(); return post('/owner/notify/test', withLoc()); }); toast(`${t('telegram')}: ${verdict(r.telegram)} · WhatsApp: ${verdict(r.whatsapp)}`); } catch (e) { fail(e); } };
  for (const id of ['nSave', 'nSave2']) $('#' + id).onclick = async () => { try { await busy($('#' + id), save); toast(t('saved')); openNotifications(); } catch (e) { fail(e); } };
  for (const id of ['nTest', 'nTest2']) $('#' + id).onclick = () => test($('#' + id));
}
/// A channel's verdict from /owner/notify/test, as one word or Meta's/Telegram's reason.
const verdict = v => v === 'ok' ? t('testOk') : v === 'unset' ? t('off') : (v && v.error) || String(v);
async function openChannels(){
  let s = { values: {} }; try { s = await api('/owner/settings'); } catch {}
  const v = s.values || {};
  const waOn = v['notify.whatsapp.token'] === SECRET_SET_MARK && !!(v['notify.whatsapp.phone_id'] || '').trim();
  const igOn = v['social.instagram.token'] === SECRET_SET_MARK && !!(v['social.instagram.user_id'] || '').trim();
  const webhook = `${location.origin}/api/webhooks/meta`;
  sheet(`${head('settings', 'channels')}
    <div class="rows">
      ${info('bowl-chopsticks', { title: k('chStore'), sub: `<span class="mono">${esc(location.host)}</span>`, trailing: onOff(true) })}
      ${info('phone', { title: k('chPhone'), sub: esc(S.venue?.phone || ''), trailing: onOff(true) })}
      ${rowBtn({ cls: waOn ? '' : 'off', leading: icon('brand-whatsapp'), title: k('whatsapp'), sub: `<span data-t="${waOn ? 'whatsappOn' : 'waNotYet'}"></span>`, trailing: onOff(waOn, ''), data: { go: 'notifications' }, tour: 'channels.whatsapp' })}
      ${rowBtn({ cls: igOn ? '' : 'off', leading: icon('sparkles'), title: k('instagram'), sub: `<span data-t="${igOn ? 'instagramOn' : 'socialNotYet'}"></span>`, trailing: onOff(igOn, ''), data: { go: 'social' }, tour: 'channels.instagram' })}
      ${info('brand-telegram', { cls: 'off', title: k('chTelegramBot'), trailing: pill('', { key: 'comingSoon' }) })}
      ${rowBtn({ leading: icon('cube-3d-sphere'), title: k('mcp'), sub: `<span class="mono">${esc(location.origin)}/api/mcp</span>`, trailing: onOff(true), data: { go: 'mcp' }, tour: 'channels.mcp' })}
      ${rowBtn({ leading: icon('key'), title: k('chApi'), sub: '<span data-t="apiHint"></span>', trailing: icon('chevron-right', 'chev'), data: { go: 'keys' }, tour: 'channels.api' })}
      ${info('scroll', { cls: 'off', title: k('chAggregators'), trailing: pill('', { key: 'comingSoon' }) })}
    </div>
    <p class="eyebrow mt-3" data-t="webhookUrl"></p>
    <div class="code small" id="whUrl" data-tour="channels.webhookUrl">${esc(webhook)}</div><p class="hint" data-t="webhookHint"></p>
    ${field({ id: 'wh-verify', key: 'verifyToken', hintKey: 'verifyHint', autocomplete: 'off', value: v['notify.whatsapp.verify'] || '', tour: 'channels.verify' })}
    ${field({ id: 'wh-secret', key: 'appSecret', autocomplete: 'off', placeholder: v['notify.meta.secret'] === SECRET_SET_MARK ? SECRET_SET_MARK : '', tour: 'channels.secret' })}
    <div class="btn-row">${btn({ id: 'whCopy', icon: 'copy', key: 'copy', tour: 'channels.copy' })}${saveBtn('whSave', 'channels.save')}</div>`, { name: 'channels' });
  const go = { notifications: openNotifications, social: openSocial, mcp: openMcp, keys: openKeys };
  for (const b of $$('[data-go]', $('#sheetIn'))) b.onclick = () => go[b.dataset.go]();
  $('#whCopy').onclick = async () => { try { await navigator.clipboard.writeText(webhook); toast(t('copied')); } catch {} };
  $('#whSave').onclick = async () => {
    try {
      await busy($('#whSave'), async () => {
        await post('/owner/settings', { key: 'notify.whatsapp.verify', value: $('#wh-verify').value.trim() });
        const sec = $('#wh-secret').value.trim(); if (sec) await post('/owner/settings', { key: 'notify.meta.secret', value: sec });
      });
      toast(t('saved'));
    } catch (e) { fail(e); }
  };
}

/// The venue as an MCP server, per role (admin/mcp.js): the URL, the tools
/// each role's key gets, client setup, and every person's key.
function openMcp(){
  return openMcpSheet({ lang: () => lang, sheet, root: () => $('#sheetIn'), head: head('settings', 'mcp'), api, post, toast, openKeys });
}

/// Off-site copies in the venue's own bucket.
async function openCloud(){
  let s = { values: {} }, st = {}; try { [s, st] = await Promise.all([api('/owner/settings'), api('/owner/backup/cloud')]); } catch (e) { fail(e); }
  const v = s.values || {};
  sheet(`${head('settings', 'cloud')}<p class="muted small" data-t="cloudHint"></p>
    <div class="rows">${info('cloud-upload', { cls: st.configured ? '' : 'off', title: k('lastCopy'), tour: 'cloud.last', sub: `<span class="mono">${st.last ? `${esc(day(st.last.atMs))} · ${Math.round((st.last.bytes || 0) / 1024)} KB · ${esc(st.last.key || '')}` : esc(t('neverPushed'))}</span>`,
      trailing: pill(st.configured ? 'ok' : '', { key: st.configured ? 'nightly' : 'off' }) })}</div>
    ${field({ id: 'cl-endpoint', key: 'endpoint', inputmode: 'url', value: v['cloud.s3.endpoint'] || '', placeholder: 'https://<account>.r2.cloudflarestorage.com', tour: 'cloud.endpoint' })}
    <div class="grid2">${field({ id: 'cl-region', key: 'region', value: v['cloud.s3.region'] || 'auto', tour: 'cloud.region' })}${field({ id: 'cl-bucket', key: 'bucket', value: v['cloud.s3.bucket'] || '', tour: 'cloud.bucket' })}</div>
    <div class="grid2">${field({ id: 'cl-key', key: 'accessKey', autocomplete: 'off', placeholder: v['cloud.s3.key'] === SECRET_SET_MARK ? SECRET_SET_MARK : '', tour: 'cloud.key' })}${field({ id: 'cl-secret', key: 'secretKey', autocomplete: 'off', placeholder: v['cloud.s3.secret'] === SECRET_SET_MARK ? SECRET_SET_MARK : '', tour: 'cloud.secret' })}</div>
    ${field({ id: 'cl-prefix', key: 'prefix', value: v['cloud.s3.prefix'] || 'dowiz', tour: 'cloud.prefix' })}
    <div class="btn-row">${btn({ id: 'clPush', icon: 'cloud-upload', key: 'pushNow', disabled: !st.configured, tour: 'cloud.push' })}${saveBtn('clSave', 'cloud.save')}</div>`, { name: 'cloud' });
  $('#clSave').onclick = async () => {
    try {
      await busy($('#clSave'), async () => {
        for (const [id, key] of [['cl-endpoint', 'cloud.s3.endpoint'], ['cl-region', 'cloud.s3.region'], ['cl-bucket', 'cloud.s3.bucket'], ['cl-prefix', 'cloud.s3.prefix']]) await post('/owner/settings', { key, value: $('#' + id).value.trim() });
        for (const [id, key] of [['cl-key', 'cloud.s3.key'], ['cl-secret', 'cloud.s3.secret']]) { const val = $('#' + id).value.trim(); if (val) await post('/owner/settings', { key, value: val }); }
      });
      toast(t('saved')); openCloud();
    } catch (e) { fail(e); }
  };
  $('#clPush').onclick = async () => { try { const r = await busy($('#clPush'), () => post('/owner/backup/cloud', withLoc())); toast(`${t('pushed')} · ${Math.round((r.bytes || 0) / 1024)} KB`); openCloud(); } catch (e) { fail(e); } };
}

/// Customers who wrote on WhatsApp or Instagram, and the answers.
async function openInbox(){
  sheet(`${head('inbox', 'inbox')}<p class="muted small" data-t="inboxHint"></p><div id="ibList">${loading()}</div>`, { name: 'inbox' });
  let d; try { d = await api('/owner/inbox'); } catch (e) { return fail(e); }
  const th = d.threads || [];
  $('#ibList').innerHTML = th.length ? `<div class="rows">${th.map(x => rowBtn({ leading: icon(x.channel === 'whatsapp' ? 'brand-whatsapp' : 'sparkles'), title: x.name || x.peer, sub: `${x.fromThem ? '' : '↩ '}${esc(x.last)}`, data: { peer: x.peer, ch: x.channel }, tour: 'inbox.thread',
      trailing: x.unread ? pill('warn', { label: String(x.unread) }) : `<small class="muted">${esc(ago(x.atMs))}</small>` })).join('')}</div>`
    : empty('message-2', { key: 'noMessages', body: d.channels?.whatsapp || d.channels?.instagram ? '' : t('waNotYet') });
  paint();
  for (const b of $$('[data-peer]', $('#ibList'))) b.onclick = () => openThread(b.dataset.ch, b.dataset.peer, th.find(x => x.peer === b.dataset.peer && x.channel === b.dataset.ch)?.name);
}
async function openThread(channel, peer, name){
  sheet(`<p class="eyebrow">${esc(channel)}</p><h2>${esc(name || peer)}</h2><div class="thread" id="thread">${loading()}</div>
    <div class="reply-row">${ui.inputRow({ id: 'rp-text', label: k('writeReply'), placeholder: k('writeReply'), attrs: { data: { tour: 'inbox.reply' } }, action: btn({ id: 'rpGo', variant: 'primary', icon: 'send', ariaKey: 'sendReply', tour: 'inbox.send' }) })}</div>`, { name: 'thread' });
  retranslate($('#sheetIn'));
  const draw = async () => {
    let d; try { d = await api(`/owner/inbox/${encodeURIComponent(peer)}?channel=${encodeURIComponent(channel)}`); } catch (e) { return fail(e); }
    $('#thread').innerHTML = (d.messages || []).map(m => `<div class="msg ${m.fromThem ? 'them' : 'me'}"><span>${esc(m.text)}</span><small>${esc(clock(m.atMs))}</small></div>`).join('') || empty('message-2', { key: 'noMessages' });
    paint(); $('#thread').scrollTop = $('#thread').scrollHeight;
  };
  draw();
  const send = async () => { const text = $('#rp-text').value.trim(); if (!text) return; try { await busy($('#rpGo'), () => post(`/owner/inbox/${encodeURIComponent(peer)}`, withLoc({ channel, text }))); $('#rp-text').value = ''; draw(); } catch (e) { fail(e); } };
  $('#rpGo').onclick = send; $('#rp-text').onkeydown = e => { if (e.key === 'Enter') send(); };
}

async function openKeys(){
  sheet(`${head('settings', 'apiKeys')}<p class="muted small" data-t="apiHint"></p><div id="kList">${loading()}</div>
    ${field({ id: 'k-label', key: 'name', tour: 'keys.label' })}<div class="btn-row">${saveBtn('kNew', 'keys.new', 'newKey', 'key')}</div><div id="kOut"></div>`, { name: 'keys' });
  const draw = async () => { let d; try { d = await api('/owner/apikeys'); } catch (e) { return fail(e); }
    $('#kList').innerHTML = (d.keys || []).length ? `<div class="rows">${d.keys.map(x => info('key', { title: x.label || '', tour: 'keys.row', sub: `<span class="mono">${esc(x.id.slice(0, 8))} · ${x.lastUsedMs ? esc(ago(x.lastUsedMs)) : '—'} · ${esc(t('until'))} ${esc(day(x.expiresMs || 0))}</span>`,
      trailing: iconBtn({ icon: 'trash', ariaKey: 'remove', variant: 'plain', data: { rev: x.id }, tour: 'keys.revoke' }) })).join('')}</div>` : empty('key', { key: 'none' });
    paint(); for (const b of $$('[data-rev]', $('#kList'))) b.onclick = async () => { const ok = await confirm(t('remove'), t('revokeHint'), { danger: true }); if (!ok) return openKeys(); try { await post('/owner/apikeys/revoke', { id: b.dataset.rev }); draw(); } catch (e) { fail(e); } }; };
  draw();
  $('#kNew').onclick = async () => { try { const d = await busy($('#kNew'), () => post('/owner/apikeys', { label: $('#k-label').value.trim() || 'api' })); $('#kOut').innerHTML = `<div class="code">${esc(d.key || d.token || JSON.stringify(d))}</div><p class="hint" data-t="keyOnce"></p>`; paint(); draw(); } catch (e) { fail(e); } };
}
async function openBranding(){
  let b; try { b = await api('/owner/branding'); } catch (e) { return fail(e); }
  const st = S.venue?.stage || {};
  sheet(`${head('settings', 'branding')}
    <div class="grid3">${input({ type: 'color', id: 'b-primary', key: 'primary', value: b.brand?.primary || '#c9a35a', tour: 'branding.primary' })}${input({ type: 'color', id: 'b-paper', key: 'paper', value: b.brand?.paper || '#0b1717', tour: 'branding.paper' })}${select({ id: 'b-type', key: 'typePair', value: b.brand?.typePair || 'classic', options: (b.typePairs || [{ id: 'classic' }]).map(p => ({ value: p.id, label: p.id })), tour: 'branding.typePair' })}</div>
    ${input({ id: 'b-logo', type: 'file', accept: 'image/*', key: 'uploadPhoto', tour: 'branding.logo' })}
    <p class="eyebrow mt-3" data-t="seal"></p>
    <div class="grid2">${field({ id: 's-seal', key: 'seal', value: st.seal || '', maxlength: 12, tour: 'branding.seal' })}${select({ id: 's-motif', key: 'motif', value: st.motif, options: ['leaf', 'wave', 'none'].map(m => ({ value: m, key: m === 'none' ? 'noneMotif' : m })), tour: 'branding.motif' })}</div>
    <div class="grid2">${input({ type: 'color', id: 's-warm', key: 'warmTone', value: st.warm || '#e0754d', tour: 'branding.warm' })}${input({ type: 'color', id: 's-sage', key: 'sageTone', value: st.sage || '#8a9a7b', tour: 'branding.sage' })}</div>
    <div class="btn-row">${btn({ id: 'bPreview', icon: 'eye', key: 'preview', tour: 'branding.preview' })}${saveBtn('bSave', 'branding.save')}</div>`, { name: 'brand' });
  $('#bPreview').onclick = () => window.open(`${location.origin}/?s=${encodeURIComponent(S.venue?.slug || store.loc)}`, '_blank');
  $('#b-logo').onchange = async e => { const f = e.target.files?.[0]; if (!f) return; try { const { shrinkImage } = await import('/lib/shrink.js'); const blob = await shrinkImage(f, { max: LOGO_MAX_PX }).catch(() => f); await api('/owner/logo', { method: 'POST', body: blob, headers: { 'content-type': 'application/octet-stream' } }); toast(t('saved')); await loadVenue(); } catch (err) { fail(err); } };
  $('#bSave').onclick = async () => {
    try {
      await busy($('#bSave'), async () => {
        await post(`/owner/branding?location_id=${encodeURIComponent(store.loc)}`, { primary: $('#b-primary').value, paper: $('#b-paper').value, typePair: $('#b-type').value });
        await post('/owner/location', withLoc({ stage: { seal: $('#s-seal').value.trim(), motif: $('#s-motif').value, warm: $('#s-warm').value, sage: $('#s-sage').value } }));
      });
      toast(t('saved')); await loadVenue(); closeSheet();
    } catch (e) { fail(e); }
  };
}
async function openFeatures(){
  let d; try { d = await api('/owner/features'); } catch (e) { return fail(e); }
  const tl = (k, fb) => { const v = t(k); return v === k ? fb : v; };
  const groups = [...new Set((d.features || []).map(f => f.surface || 'storefront'))];
  sheet(`${head('settings', 'features')}${groups.map(g => `<p class="eyebrow mt-3">${esc(tl('surface_' + g, g))}</p>${(d.features || []).filter(f => (f.surface || 'storefront') === g).map(f => `<div class="feat">${check({ label: tl('feat_' + f.key, f.label), checked: f.on, data: { f: f.key }, tour: 'features.' + f.key })}
    ${f.defaultOn != null && f.on !== f.defaultOn ? pill('warn', { key: 'changed' }) : ''}<p class="hint">${esc(tl('feat_' + f.key + '_h', f.hint || ''))}</p></div>`).join('')}`).join('')}`, { name: 'features' });
  for (const el of $$('[data-f]', $('#sheetIn'))) el.onchange = async () => { try { await post('/owner/features', { key: el.dataset.f, on: el.checked }); toast(t('saved')); } catch (e) { fail(e); el.checked = !el.checked; } };
}
async function openActivation(){
  let a; try { a = await api('/owner/activation'); } catch (e) { return fail(e); }
  const tl = (k, fb) => { const v = t(k); return v === k ? fb : v; };
  const word = v => typeof v === 'boolean' ? t(v ? 'yes' : 'no') : String(v);
  const missing = new Map((a.missing || []).map(m => [m.key, m.why]));
  const CHECKS = [['menu', ['sellableDishes']], ['notifications', ['telegramChats']], ['fulfilment', ['hasVenuePhone', 'deliveryConfigured', 'pickupEnabled']]];
  sheet(`${head('settings', 'activation')}
    <div class="rows">${CHECKS.map(([key, facts]) => info(missing.has(key) ? 'alert-circle' : 'check', { title: tl('req_' + key, key), tour: 'activation.' + key, sub: missing.has(key) ? esc(missing.get(key)) : facts.map(f => `${esc(tl('fact_' + f, f))}: ${esc(word(a.facts?.[f]))}`).join(' · '),
      trailing: pill(missing.has(key) ? 'warn' : 'ok', { label: missing.has(key) ? '!' : '✓' }) })).join('')}</div>
    ${!(a.missing || []).length ? `<p class="ok mt-3" data-t="hubOk"></p>` : ''}`, { name: 'activation' });
}
/// Fullness per mille at which a fixed-size image turns amber, then red.
const HEALTH_WARN_PM = 650, HEALTH_BAD_PM = 850;
async function openHealth(){
  sheet(`${head('settings', 'health')}<div id="hBody">${loading()}</div>
    <div class="btn-row">${btn({ id: 'hBackup', href: '/api/owner/backup', icon: 'download', key: 'backup', attrs: { download: true }, tour: 'health.backup' })}</div>`, { name: 'health' });
  let h; try { h = await api('/owner/health'); } catch (e) { return fail(e); }
  // A self-growing image is never amber: fullness is not a warning when the
  // ceiling moves. Fixed-size images first, then the fullest.
  const entries = Object.entries(h.images || {}).sort(([, a], [, b]) => (a.grows === b.grows ? (b.usedPerMille || 0) - (a.usedPerMille || 0) : a.grows ? 1 : -1));
  const tone = im => im.grows ? '' : im.usedPerMille > HEALTH_BAD_PM ? 'bad' : im.usedPerMille > HEALTH_WARN_PM ? 'warn' : '';
  $('#hBody').innerHTML = `<div class="rows">${entries.map(([x, im]) => info('cube-3d-sphere', { title: t('img_' + x) === 'img_' + x ? x : t('img_' + x), tour: 'health.image', sub: `<span class="mono">${Math.round((im.usedPerMille || 0) / 10)}% · ${im.usedCells}/${im.ceilingCells} · gen ${im.generation}${im.grows ? ` · ${esc(t('grows'))}` : ''}</span><span class="gauge"><i class="${tone(im)}" data-w="${Math.round((im.usedPerMille || 0) / 10)}"></i></span>` })).join('')}</div>
    <div class="rows mt-3">${info(h.verdict === 'ok' ? 'check' : 'alert-circle', { title: k('verdict_' + (h.verdict || 'ok')), sub: `<span class="mono">${h.orders ?? ''} · ${esc(t('orders7'))}</span>`, tour: 'health.verdict', trailing: pill(h.verdict === 'ok' ? 'ok' : h.verdict === 'watch' ? 'warn' : 'bad', { label: h.verdict || '' }) })}</div>`;
  paint();
  $('#hBackup').onclick = async e => { e.preventDefault(); try { const r = await fetch('/api/owner/backup', { headers: { authorization: 'Bearer ' + store.t } }); const blob = await r.blob(); const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = `dowiz-${store.loc}-${new Date().toISOString().slice(0, 10)}.json`; a.click(); } catch (err) { fail(err); } };
}

/// The owner's assistant: a question about the venue's own live data, answered
/// by the model the venue chose (a local Ollama by default, a hosted one by
/// token), with the provenance line the old console printed.
async function openAssistant(){
  let s = { values: {} }; try { s = await api('/owner/settings'); } catch {}
  const v = s.values || {};
  sheet(`${head('settings', 'assistant')}<p class="muted small" data-t="askHint"></p>
    ${field({ id: 'as-q', key: 'ask', rows: 2, tour: 'assistant.question' })}
    <div class="btn-row">${saveBtn('asGo', 'assistant.ask', 'ask', 'sparkles')}</div>
    <div id="asOut"></div>
    <p class="eyebrow mt-3" data-t="aiSettings"></p>
    ${switchEl('ai-on', v['ai.enabled'] === '1', 'aiEnabled', 'aiEnabledHint', 'assistant.enabled')}
    ${v['ai.enabled'] === '1' && !String(v['ai.endpoint'] || '').startsWith('https://')
      ? `<p class="warn small" data-t="aiNotHttps"></p>` : ''}
    ${field({ id: 'ai-endpoint', key: 'aiEndpoint', inputmode: 'url', value: v['ai.endpoint'] || '', placeholder: 'https://…/v1', tour: 'assistant.endpoint' })}
    <div class="grid2">${field({ id: 'ai-model', key: 'aiModel', value: v['ai.model'] || '', tour: 'assistant.model' })}${field({ id: 'ai-token', key: 'aiToken', autocomplete: 'off', placeholder: v['ai.token'] === SECRET_SET_MARK ? SECRET_SET_MARK : '', tour: 'assistant.token' })}</div>
    <div class="btn-row">${btn({ id: 'aiSave', icon: 'check', key: 'save', tour: 'assistant.save' })}</div>`, { name: 'assistant' });
  $('#asGo').onclick = async () => {
    const question = $('#as-q').value.trim(); if (!question) return;
    try {
      const r = await busy($('#asGo'), () => post('/owner/assist', withLoc({ question })));
      $('#asOut').innerHTML = `<div class="answer mt-3">${esc(r.answer || '')}</div><p class="hint" data-t="${r.local ? 'localModel' : 'cloudModel'}"></p>`; paint();
    } catch (e) { fail(e); }
  };
  $('#aiSave').onclick = async () => {
    try {
      await busy($('#aiSave'), async () => {
        await post('/owner/settings', { key: 'ai.enabled', value: $('#ai-on').checked ? '1' : '0' });
        await post('/owner/settings', { key: 'ai.endpoint', value: $('#ai-endpoint').value.trim() });
        await post('/owner/settings', { key: 'ai.model', value: $('#ai-model').value.trim() });
        const tok = $('#ai-token').value.trim(); if (tok) await post('/owner/settings', { key: 'ai.token', value: tok });
      });
      toast(t('saved'));
    } catch (e) { fail(e); }
  };
}

/// Every outside connection, its state, and a proof button. The proof is the
/// provider's own answer (a bot's username, a number's verified name, an
/// object's etag), never a message to a customer.
const INTEGRATIONS = [
  ['telegram', 'brand-telegram', () => openNotifications()],
  ['whatsapp', 'brand-whatsapp', () => openNotifications()],
  ['instagram', 'sparkles', () => openSocial()],
  ['webhook', 'scroll', () => openChannels()],
  ['cloud', 'cloud-upload', () => openCloud()],
  ['mcp', 'cube-3d-sphere', () => openMcp()],
  ['stripe', 'credit-card', () => openPayments()],
  ['ai', 'sparkles', () => openAssistant()],
];
const integrationOn = (k, st) => ({ telegram: st.telegram?.configured, whatsapp: st.whatsapp?.configured, instagram: st.instagram?.configured, webhook: st.webhook?.verifySet, cloud: st.cloud?.configured, mcp: true, stripe: st.stripe?.configured, ai: st.ai?.enabled })[k];
const integrationLine = (k, st) => ({
  telegram: st.telegram?.configured ? t('telegramOn') : t('tgNotSet'),
  whatsapp: st.whatsapp?.configured ? `${t('whatsappOn')}${st.whatsapp.notifies ? '' : ' · ' + t('whatsappTo') + ': —'}` : t('waNotYet'),
  instagram: st.instagram?.configured ? t('instagramOn') : t('socialNotYet'),
  webhook: `${st.webhook?.lastMs ? t('lastDelivery') + ' ' + ago(st.webhook.lastMs) : t('noDelivery')}${st.webhook?.secretSet ? ' · HMAC' : ''}`,
  cloud: st.cloud?.configured ? st.cloud.bucket : t('cloudSub'),
  mcp: `${st.mcp?.tools ?? 0} ${t('tools')}`,
  stripe: st.stripe?.configured ? t('on') : t('stripeNotSet'),
  ai: st.ai?.enabled ? st.ai.endpoint : t('assistantSub'),
})[k] || '';
const detailWords = (k, d) => {
  if (!d) return '';
  switch (k) {
    case 'telegram': return `${t('botName')} @${d.bot ?? ''}`;
    case 'whatsapp': return [d.name, d.number, d.quality].filter(Boolean).join(' · ');
    case 'instagram': return `@${d.username ?? ''} · ${d.followers ?? 0} ${t('followers')}`;
    case 'webhook': return d.url;
    case 'cloud': return `${t('probeWritten')} · ${d.key}`;
    case 'mcp': return `${d.tools} ${t('tools')}`;
    case 'ai': return `${d.models} ${t('models')} · ${d.model}`;
    default: return t('on');
  }
};
async function openPreview(){
  sheet(`<p class="eyebrow" data-t="previewH"></p><h2 data-t="previewH"></h2>
    <p class="muted small" data-t="previewP"></p>
    <div class="phone"><div class="phone-screen"><iframe class="preview-frame" id="previewFrame" title="Storefront" loading="lazy" src="/?preview=1" referrerpolicy="same-origin"></iframe></div></div>
    <div class="preview-row">${btn({ id: 'previewReload', icon: 'refresh', key: 'previewReload', tour: 'preview.reload' })}${btn({ variant: 'primary', href: '/', target: '_blank', icon: 'external-link', key: 'previewOpen', tour: 'preview.open' })}</div>`, { name: 'preview' });
  $('#previewReload').onclick = () => { const frame = $('#previewFrame'); if (frame) frame.src = frame.src; };
  paint();
}

async function openIntegrations(){
  sheet(`${head('settings', 'integrations')}<p class="muted small" data-t="integrationsHint"></p><div id="igList">${loading(2)}</div>
    <div class="btn-row">${saveBtn('igAll', 'integrations.checkAll', 'checkAll')}</div>`, { name: 'integrations' });
  let st; try { st = await api('/owner/integrations'); } catch (e) { return fail(e); }
  $('#igList').innerHTML = `<div class="rows">${INTEGRATIONS.map(([k, ic]) => `<div class="igrow ${integrationOn(k, st) ? 'on' : ''}" data-ig="${k}">
      <div class="igrow-h">${icon(ic)}<b data-t="${k}"></b>${onOff(integrationOn(k, st), '')}</div>
      <small class="ig-line">${esc(integrationLine(k, st))}</small><small class="ig-out mono" hidden></small>
      <div class="igrow-a">${btn({ icon: 'adjustments', key: 'configure', data: { cfg: k }, tour: 'integrations.configure' })}${btn({ icon: 'check', key: 'check', data: { chk: k }, tour: 'integrations.check' })}</div></div>`).join('')}</div>`;
  paint();
  const run = async k => {
    const row = $(`[data-ig="${k}"]`), out = $('.ig-out', row), b = $(`[data-chk="${k}"]`, row);
    out.hidden = false; out.className = 'ig-out mono'; out.textContent = '…';
    try { const r = await busy(b, () => post('/owner/integrations/check', withLoc({ which: k }))); out.textContent = '✓ ' + detailWords(k, r.detail); out.classList.add('ok'); row.classList.remove('off'); }
    catch (e) { const code = e.code || (e.body && e.body.code); const word = code && t('ck_' + code) !== 'ck_' + code ? t('ck_' + code) : String(e.message || e); out.textContent = '✕ ' + word; out.classList.add('err'); }
  };
  for (const b of $$('[data-chk]', $('#igList'))) b.onclick = () => run(b.dataset.chk);
  for (const b of $$('[data-cfg]', $('#igList'))) b.onclick = () => INTEGRATIONS.find(([k]) => k === b.dataset.cfg)[2]();
  $('#igAll').onclick = async () => { for (const [k] of INTEGRATIONS) await run(k); };
}
