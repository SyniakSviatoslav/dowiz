// "More" -- everything that is not the day's service, as one list of rows,
// each a sheet: marketing (promo codes, posts, social), analytics, customers,
// and the settings a venue sets once (the venue itself, hours, delivery,
// payments, notifications, order channels, brand, features, API keys, health,
// backup). The list is the map; nothing is buried under a fold.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, money, moneyEl, busy, confirm, switchEl, store, day, ago, clock, hydrate } from '/admin/core.js';
import { retranslate, lang, LANGS } from '/admin/i18n.js';
import { loadVenue, rerender } from '/admin/app.js';

/// The rows, in groups, with the sheet each opens.
const GROUPS = [
  ['inbox', [['inbox', 'message-2', openInbox]]],
  ['marketing', [['promos', 'ticket', openPromos], ['posts', 'send', openPosts], ['social', 'sparkles', openSocial]]],
  ['analytics', [['analytics', 'chart-bar', openAnalytics], ['customers', 'user', openCustomers]]],
  ['settings',  [['integrations', 'check', openIntegrations], ['venue', 'home', openVenue], ['hours', 'clock', openHours], ['deliveryTerms', 'bike', openDelivery], ['payments', 'coin-hole', openPayments],
                 ['notifications', 'brand-telegram', openNotifications], ['channels', 'scroll', openChannels], ['mcp', 'cube-3d-sphere', openMcp], ['cloud', 'cloud-upload', openCloud], ['branding', 'fan', openBranding],
                 ['features', 'tools-kitchen-2', openFeatures], ['assistant', 'sparkles', openAssistant], ['apiKeys', 'key', openKeys], ['activation', 'check', openActivation], ['health', 'cube-3d-sphere', openHealth]]],
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
      ${rows.map(([key, ic]) => `<button type="button" class="tile" data-open="${key}"><span class="tile-ic">${icon(ic)}</span><b data-t="${key}"></b><small data-t="${key}Sub"></small></button>`).join('')}</div></section>`).join('')}
    <p class="hint mono">${esc(store.loc)} · ${esc(S.venue?.slug || '')}</p>`;
  host.onclick = e => { const r = e.target.closest('[data-open]'); if (!r) return; for (const [, rows] of GROUPS) for (const [key, , fn] of rows) if (key === r.dataset.open) fn(); };
}

const head = (eyebrow, title) => `<p class="eyebrow" data-t="${eyebrow}"></p><h2 data-t="${title}"></h2>`;
const fail = e => toast(String(e.message || e));
const paint = () => { retranslate($('#sheetIn')); hydrate($('#sheetIn')); };

// ── marketing ───────────────────────────────────────────────────────────────
async function openPromos(){
  sheet(`${head('marketing', 'promos')}<div id="pList"><div class="skel skel-row"></div></div>
    <button class="btn mt-3" id="pNew">${icon('plus')}<span data-t="add"></span></button>`, { name: 'promos' });
  const draw = async () => {
    let d; try { d = await api('/owner/promotions'); } catch (e) { return fail(e); }
    const list = d.promotions || [];
    $('#pList').innerHTML = list.length ? `<div class="rows">${list.map(p => { const stt = promoStatus(p); return `<div class="rowc ${stt === 'active' ? '' : 'off'}">${icon('ticket')}<span class="t"><b class="mono">${esc(p.code)}</b><small>${p.kind === 'percent' ? `−${p.value}%` : `−${money(p.value)}`}${p.minOrder ? ` · ${t('minOrder')} ${money(p.minOrder)}` : ''} · ${p.used || 0}${p.maxUses ? '/' + p.maxUses : ''}${p.fromMs ? ` · ${t('when')} ${day(p.fromMs)}` : ''}${p.untilMs ? ` · ${t('until')} ${day(p.untilMs - 1)}` : ''}</small></span>
      <button type="button" class="pill ${stt === 'active' ? 'ok' : stt === 'scheduled' ? 'warn' : ''}" data-flip="${esc(p.code)}" data-t="promo_${stt}"></button><button type="button" class="act danger" data-del="${esc(p.code)}">${icon('trash')}</button></div>`; }).join('')}</div>` : `<div class="empty">${icon('ticket')}<b data-t="none"></b></div>`;
    paint();
    for (const b of $$('[data-flip]', $('#pList'))) b.onclick = async () => { const p = list.find(x => x.code === b.dataset.flip); if (!p) return; try { await busy(b, () => post('/owner/promotions', { ...p, active: !p.active, used: undefined, status: undefined })); draw(); } catch (e) { fail(e); } };
    for (const b of $$('[data-del]', $('#pList'))) b.onclick = async () => { const c = await confirm(t('remove'), b.dataset.del, { danger: true }); if (!c) return openPromos(); try { await post(`/owner/promotions/${encodeURIComponent(b.dataset.del)}/delete`, withLoc()); openPromos(); } catch (e) { fail(e); } };
  };
  draw();
  $('#pNew').onclick = () => {
    sheet(`${head('marketing', 'promos')}
      <label for="pr-code" data-t="promo"></label><input id="pr-code" autocapitalize="characters" spellcheck="false">
      <div class="seg" id="prKind">${PROMO_KINDS.map((k, i) => `<button type="button" class="seg-b ${i === 0 ? 'on' : ''}" data-k="${k}">${k === 'percent' ? '%' : esc(S.venue?.currencyCode || 'ALL')}</button>`).join('')}</div>
      <div class="grid2"><div><label for="pr-value" data-t="discount"></label><input id="pr-value" inputmode="numeric"></div><div><label for="pr-min" data-t="minOrder"></label><input id="pr-min" inputmode="numeric"></div></div>
      <div class="grid2"><div><label for="pr-from" data-t="when"></label><input id="pr-from" type="date"></div><div><label for="pr-until" data-t="until"></label><input id="pr-until" type="date"></div></div>
      <label for="pr-max" data-t="maxUses"></label><input id="pr-max" inputmode="numeric">
      <div class="btn-row"><button class="btn" id="prSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'promo' });
    let kind = PROMO_KINDS[0];
    for (const b of $$('[data-k]', $('#sheetIn'))) b.onclick = () => { kind = b.dataset.k; for (const x of $$('[data-k]', $('#sheetIn'))) x.classList.toggle('on', x === b); };
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
  sheet(`${head('marketing', 'posts')}<p class="muted small" data-t="autopostHint"></p><div id="postList"><div class="skel skel-row"></div></div>
    <button class="btn mt-3" id="postDraft">${icon('sparkles')}<span data-t="makeDraft"></span></button>`, { name: 'posts' });
  const draw = async () => {
    let d; try { d = await api('/owner/posts'); } catch (e) { return fail(e); }
    const list = d.posts || [];
    $('#postList').innerHTML = `<p class="hint">${d.enabled ? `${t('autopost')}: ${t('on')}` : `${t('autopost')}: ${t('off')}`} · ${t('tgChannel')}: ${esc(d.channel || '—')}</p>` +
      (list.length ? list.map(p => `<div class="rowc" data-post="${esc(p.id)}">${icon('send')}<span class="t"><b>${esc(p.text).slice(0, 80)}</b><small>${esc(p.about || '')} · <span data-t="${p.state === 'draft' ? 'draft' : p.state === 'published' ? 'published' : p.state === 'failed' ? 'failed' : 'rejectPost'}"></span>${p.error ? ' · ' + esc(p.error) : ''}</small></span></div>`).join('') : `<div class="empty">${icon('send')}<b data-t="noPosts"></b></div>`);
    paint();
    for (const r of $$('[data-post]', $('#postList'))) r.onclick = () => openPost(list.find(p => p.id === r.dataset.post));
  };
  draw();
  $('#postDraft').onclick = async () => { try { await busy($('#postDraft'), () => post('/owner/posts/draft', withLoc())); draw(); } catch (e) { fail(e); } };
}
function openPost(p){
  if (!p) return;
  sheet(`${head('posts', 'draft')}<textarea id="postText" rows="5">${esc(p.text)}</textarea><p class="hint">${esc(p.about || '')}</p>
    <div class="btn-row">${p.state === 'draft' ? `<button class="btn danger" id="postNo">${icon('x')}<span data-t="rejectPost"></span></button><button class="btn" id="postYes">${icon('send')}<span data-t="approve"></span></button>` : `<button class="btn ghost" id="postBack" data-t="back"></button>`}</div>`, { name: 'post' });
  const yes = $('#postYes'); if (yes) yes.onclick = async () => { try { await busy(yes, () => post(`/owner/posts/${encodeURIComponent(p.id)}/approve`, { text: $('#postText').value.trim() })); toast(t('published')); openPosts(); } catch (e) { fail(e); } };
  const no = $('#postNo'); if (no) no.onclick = async () => { try { await post(`/owner/posts/${encodeURIComponent(p.id)}/reject`, withLoc()); openPosts(); } catch (e) { fail(e); } };
  const back = $('#postBack'); if (back) back.onclick = openPosts;
}

async function openSocial(){
  let s; try { s = await api('/owner/settings'); } catch (e) { return fail(e); }
  const v = s.values || {};
  const igOn = v['social.instagram.token'] === SECRET_SET_MARK && !!(v['social.instagram.user_id'] || '').trim();
  sheet(`${head('marketing', 'social')}<p class="muted small" data-t="autopostHint"></p>
    ${switchEl('so-on', v['social.enabled'] === '1', 'autopost')}
    <label for="so-ch" data-t="tgChannel"></label><input id="so-ch" value="${esc(v['social.telegram.channel'] || '')}" placeholder="@channel">
    <p class="eyebrow mt-3" data-t="instagram"></p>
    <div class="rows"><div class="rowc ${igOn ? '' : 'off'}">${icon('sparkles')}<span class="t"><b data-t="instagram"></b><small data-t="${igOn ? 'instagramOn' : 'socialNotYet'}"></small></span><span class="pill ${igOn ? 'ok' : ''}" data-t="${igOn ? 'on' : 'off'}"></span></div></div>
    <p class="hint" data-t="instagramHint"></p>
    <label for="ig-token" data-t="instagramToken"></label><input id="ig-token" autocomplete="off" spellcheck="false" placeholder="${igOn ? esc(SECRET_SET_MARK) : 'EAAB…'}">
    <label for="ig-user" data-t="instagramUserId"></label><input id="ig-user" inputmode="numeric" value="${esc(v['social.instagram.user_id'] || '')}">
    <div class="rows mt-3">
      ${['facebook', 'tiktok'].map(n => `<div class="rowc off">${icon('sparkles')}<span class="t"><b data-t="${n}"></b><small data-t="socialNotYet"></small></span><span class="pill" data-t="comingSoon"></span></div>`).join('')}
    </div>
    <div class="btn-row"><button class="btn" id="soSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'social' });
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
    <div class="seg">${WINDOWS.map(w => `<button type="button" class="seg-b ${w === days ? 'on' : ''}" data-days="${w}" data-t="${w === 7 ? 'week' : 'month'}"></button>`).join('')}</div>
    <div id="anBody"><div class="skel skel-row"></div><div class="skel skel-row"></div></div>`, { name: 'analytics' });
  for (const b of $$('[data-days]', $('#sheetIn'))) b.onclick = () => openAnalytics(Number(b.dataset.days));
  let a; try { a = await api(`/owner/analytics?days=${days}`); } catch (e) { return fail(e); }
  const byDay = a.byDay || [], maxRev = Math.max(1, ...byDay.map(d => d.revenue || 0));
  const byHour = a.byHour || [], maxH = Math.max(1, ...byHour);
  $('#anBody').innerHTML = `
    <div class="stats"><div class="stat"><small data-t="orders7"></small><b>${a.orders ?? 0}</b></div><div class="stat"><small data-t="revenue7"></small><b>${money(a.revenue || 0)}</b></div>
      <div class="stat"><small data-t="avgCheck"></small><b>${money(a.averageOrder || 0)}</b></div><div class="stat ${a.rejected ? 'warn' : ''}"><small data-t="rejected"></small><b>${a.rejected ?? 0}</b></div></div>
    <div class="rows"><div class="rowc">${icon('bike')}<span class="t"><b data-t="delivery"></b></span><b class="mono">${a.delivery ?? 0}</b></div><div class="rowc">${icon('walk')}<span class="t"><b data-t="pickup"></b></span><b class="mono">${a.pickup ?? 0}</b></div></div>
    <p class="eyebrow mt-3" data-t="byDay"></p><div class="bars" role="img" aria-label="${esc(t('byDay'))} · max ${esc(money(maxRev))}">${byDay.map(d => `<i data-h="${Math.round(100 * (d.revenue || 0) / maxRev)}" title="${esc(day(d.at))} · ${esc(money(d.revenue || 0))}"></i>`).join('')}</div>
    <div class="axis"><span>${byDay.length ? day(byDay[0].at) : ''}</span><span>${byDay.length ? day(byDay[byDay.length - 1].at) : ''}</span></div>
    <p class="eyebrow mt-3" data-t="byHour"></p><div class="bars" role="img" aria-label="${esc(t('byHour'))} · max ${maxH}">${byHour.map((n, h) => `<i class="${n === maxH ? 'hi' : ''}" data-h="${Math.round(100 * n / maxH)}" title="${String(h).padStart(2, '0')}:00 · ${n}"></i>`).join('')}</div><div class="axis"><span>00</span><span>12</span><span>23</span></div>
    <p class="eyebrow mt-3" data-t="topDishes"></p><div class="rows">${(a.topProducts || []).slice(0, 8).map(p => `<div class="rowc">${icon('bowl-chopsticks')}<span class="t"><b>${esc(p.name || p.id)}</b><small class="mono">${p.quantity ?? 0} ${esc(t('portions'))}</small></span><span class="money">${money(p.revenue || 0)}</span></div>`).join('')}</div>`;
  paint();
}
/// Customers are MASKED by default; a name and phone are revealed one at a
/// time, for a written reason, and every reveal is in a log the owner can read.
const CUSTOMER_SORTS = ['spent', 'orders', 'recent'];
const REVEAL_REASON_MIN = 3;
async function openCustomers(sort = CUSTOMER_SORTS[0]){
  sheet(`${head('analytics', 'customers')}
    <div class="seg">${CUSTOMER_SORTS.map(k => `<button type="button" class="seg-b ${k === sort ? 'on' : ''}" data-sort="${k}" data-t="sort_${k}"></button>`).join('')}</div>
    <div id="cuBody"><div class="skel skel-row"></div></div>
    <div class="btn-row"><button class="btn ghost" id="cuCsv">${icon('download')}<span data-t="exportCsv"></span></button><button class="btn ghost" id="cuLog">${icon('eye')}<span data-t="revealLog"></span></button></div>`, { name: 'customers' });
  for (const b of $$('[data-sort]', $('#sheetIn'))) b.onclick = () => openCustomers(b.dataset.sort);
  let d; try { d = await api(`/owner/customers?sort=${sort}`); } catch (e) { return fail(e); }
  const list = d.customers || [];
  $('#cuBody').innerHTML = list.length ? `<div class="rows">${list.map(c => `<button type="button" class="rowc" data-key="${esc(c.key)}">${icon('user')}<span class="t"><b>${esc(c.name || c.phone || c.key)}</b><small class="mono">${esc(c.phone || '')} · ${c.orders} · ${money(c.spent || 0)}</small></span>${c.lastAt ? `<small class="muted">${esc(day(c.lastAt))}</small>` : ''}${icon('chevron-right', 'chev')}</button>`).join('')}</div>` : `<div class="empty">${icon('user')}<b data-t="none"></b></div>`;
  paint();
  for (const b of $$('[data-key]', $('#cuBody'))) b.onclick = () => openReveal(b.dataset.key, list.find(c => c.key === b.dataset.key));
  $('#cuCsv').onclick = () => {
    const cell = v => { const x = String(v ?? ''); return /[",\n;]/.test(x) ? '"' + x.replace(/"/g, '""') + '"' : x; };
    const rows = [[t('customer'), t('phone'), t('orders7'), t('total'), t('lastSeen')].map(cell).join(','), ...list.map(c => [c.name, c.phone, c.orders, c.spent ?? 0, c.lastAt ? new Date(c.lastAt).toISOString() : ''].map(cell).join(','))];
    const blob = new Blob(['\ufeff' + rows.join('\n')], { type: 'text/csv;charset=utf-8' });
    const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = `dowiz-customers-${new Date().toISOString().slice(0, 10)}.csv`; a.click(); requestAnimationFrame(() => URL.revokeObjectURL(a.href));
  };
  $('#cuLog').onclick = openRevealLog;
}
async function openReveal(key, c){
  sheet(`${head('customers', 'reveal')}<p class="muted small" data-t="revealHint"></p>
    <div class="fact">${icon('user')}<span class="v">${esc(c?.name || key)}<br><small class="mono">${esc(c?.phone || '')}</small></span></div>
    <label for="rv-reason" data-t="revealReason"></label><input id="rv-reason" autocomplete="off">
    <div class="btn-row"><button class="btn" id="rvGo">${icon('eye')}<span data-t="reveal"></span></button></div><div id="rvOut"></div>`, { name: 'reveal' });
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
  sheet(`${head('customers', 'revealLog')}<div id="rlBody"><div class="skel skel-row"></div></div>`, { name: 'reveals' });
  let d; try { d = await api('/owner/customers/reveals'); } catch (e) { return fail(e); }
  const list = d.reveals || [];
  $('#rlBody').innerHTML = list.length ? `<div class="rows">${list.map(r => `<div class="rowc">${icon('eye')}<span class="t"><b>${esc(r.reason || '')}</b><small class="mono">${esc(r.by || '')} · ${esc(day(r.atMs || r.at || 0))} ${esc(clock(r.atMs || r.at || 0))}</small></span></div>`).join('')}</div>` : `<div class="empty">${icon('eye')}<b data-t="none"></b></div>`;
  paint();
}

// ── settings ────────────────────────────────────────────────────────────────
async function openVenue(){
  const v = S.venue || {};
  sheet(`${head('settings', 'venue')}
    <label for="v-name" data-t="venueName"></label><input id="v-name" value="${esc(v.name || '')}">
    <label for="v-phone" data-t="venuePhone"></label><input id="v-phone" type="tel" value="${esc(v.phone || '')}">
    <label for="v-addr" data-t="venueAddress"></label><input id="v-addr" value="${esc(v.address || '')}">
    ${switchEl('v-pickup', !!v.pickup, 'pickupOn')}
    <div class="btn-row"><button class="btn" id="vSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'venue' });
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
async function openHours(){
  const week = Array.isArray(S.venue?.hours) && S.venue.hours.length === 7 ? S.venue.hours : Array.from({ length: 7 }, () => []);
  sheet(`${head('settings', 'hours')}
    ${week.map((w, i) => `<div class="grid3 hours-row"><label class="switch"><input type="checkbox" data-day="${i}" ${w.length ? 'checked' : ''}><span class="switch-k"></span><span class="t">${esc(t('day')[i])}</span></label>
      <input type="time" data-open="${i}" value="${w.length ? hhmm(w[0].open) : '11:00'}"><input type="time" data-close="${i}" value="${w.length ? hhmm(w[0].close) : '23:00'}"></div>`).join('')}
    <div class="btn-row"><button class="btn" id="hSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'hours' });
  $('#hSave').onclick = async () => {
    const hours = week.map((_, i) => { const on = $(`[data-day="${i}"]`).checked; if (!on) return []; const o = toMin($(`[data-open="${i}"]`).value), c = toMin($(`[data-close="${i}"]`).value); return o !== null && c !== null ? [{ open: o, close: c }] : []; });
    try { await busy($('#hSave'), () => post('/owner/place', { hours })); toast(t('saved')); await loadVenue(); closeSheet(); } catch (e) { fail(e); }
  };
}
async function openDelivery(){
  const v = S.venue || {};
  sheet(`${head('settings', 'deliveryTerms')}
    <div class="grid2"><div><label for="d-fee" data-t="deliveryFee"></label><input id="d-fee" inputmode="numeric" value="${v.deliveryFee ?? 0}"></div>
      <div><label for="d-free" data-t="freeOver"></label><input id="d-free" inputmode="numeric" value="${v.freeDeliveryThreshold ?? ''}"></div></div>
    <label for="d-min" data-t="minOrder"></label><input id="d-min" inputmode="numeric" value="${v.minOrder ?? 0}">
    <div class="btn-row"><button class="btn" id="dSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'delivery' });
  $('#dSave').onclick = async () => {
    const free = $('#d-free').value.trim();
    try { await busy($('#dSave'), () => post('/owner/location', withLoc({ delivery_fee: Number($('#d-fee').value) || 0, min_order: Number($('#d-min').value) || 0, free_delivery_threshold: free ? Number(free) : null }))); toast(t('saved')); await loadVenue(); closeSheet(); } catch (e) { fail(e); }
  };
}
async function openPayments(){
  const v = S.venue || {}, pay = v.payments || {}, wallets = pay.crypto || [];
  sheet(`${head('settings', 'payments')}
    <div class="rows">
      <div class="rowc">${icon('cash')}<span class="t"><b data-t="cash"></b></span><span class="pill ok" data-t="on"></span></div>
      <div class="rowc ${pay.card ? '' : 'off'}">${icon('credit-card')}<span class="t"><b data-t="stripe"></b><small data-t="${pay.card ? 'on' : 'stripeNotSet'}"></small></span><span class="pill ${pay.card ? 'ok' : 'warn'}" data-t="${pay.card ? 'on' : 'off'}"></span></div>
    </div>
    <p class="eyebrow mt-3" data-t="cryptoWallets"></p>
    <div id="wallets">${wallets.map((w, i) => walletRow(w, i)).join('')}</div>
    <button type="button" class="act mt-2" id="wAdd">${icon('plus')}<span data-t="add"></span></button>
    <div class="btn-row"><button class="btn" id="wSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'payments' });
  $('#wAdd').onclick = () => { $('#wallets').insertAdjacentHTML('beforeend', walletRow({}, $$('.wallet-row').length)); paint(); };
  $('#sheetIn').addEventListener('click', e => { const x = e.target.closest('[data-wdel]'); if (x) x.closest('.wallet-row').remove(); });
  $('#wSave').onclick = async () => {
    const list = $$('.wallet-row', $('#sheetIn')).map(r => ({ network: $('[data-f="network"]', r).value.trim(), symbol: $('[data-f="symbol"]', r).value.trim(), address: $('[data-f="address"]', r).value.trim() })).filter(w => w.address);
    try { await busy($('#wSave'), () => post('/owner/location', withLoc({ crypto_wallets: list }))); toast(t('saved')); await loadVenue(); closeSheet(); } catch (e) { fail(e); }
  };
}
const walletRow = (w, i) => `<div class="wallet-row grid3"><div><label data-t="network"></label><input data-f="network" value="${esc(w.network || '')}" placeholder="TRC20"></div>
  <div><label data-t="symbol"></label><input data-f="symbol" value="${esc(w.symbol || '')}" placeholder="USDT"></div>
  <div><label data-t="walletAddress"></label><div class="grid2"><input data-f="address" value="${esc(w.address || '')}"><button type="button" class="act danger" data-wdel="${i}">${icon('trash')}</button></div></div></div>`;

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
      <div class="rowc">${icon('brand-telegram')}<span class="t"><b data-t="telegram"></b><small data-t="${live ? 'tgHow' : 'tgNotSet'}"></small></span><span class="pill ${live ? 'ok' : 'warn'}" data-t="${live ? 'on' : 'off'}"></span></div>
    </div>
    <label for="n-token" data-t="tgToken"></label><input id="n-token" autocomplete="off" spellcheck="false" placeholder="${tokenSet ? esc(SECRET_SET_MARK) : '123456:ABC…'}"><p class="hint" data-t="tgTokenHint"></p>
    <label for="n-chat" data-t="ownerChat"></label><input id="n-chat" inputmode="numeric" value="${esc(chat)}" placeholder="chat id"><p class="hint" data-t="tgChatHint"></p>
    <div class="btn-row"><button class="btn ghost" id="nTest">${icon('send')}<span data-t="testMessage"></span></button><button class="btn" id="nSave">${icon('check')}<span data-t="save"></span></button></div>
    <p class="eyebrow mt-3" data-t="whatsapp"></p>
    <div class="rows"><div class="rowc ${waOn ? '' : 'off'}">${icon('phone')}<span class="t"><b data-t="whatsapp"></b><small data-t="${waOn ? 'whatsappOn' : 'waNotYet'}"></small></span><span class="pill ${waOn ? 'ok' : 'warn'}" data-t="${waOn ? 'on' : 'off'}"></span></div></div>
    <p class="hint" data-t="whatsappHint"></p>
    <label for="wa-token" data-t="whatsappToken"></label><input id="wa-token" autocomplete="off" spellcheck="false" placeholder="${waSet ? esc(SECRET_SET_MARK) : 'EAAB…'}">
    <div class="grid2"><div><label for="wa-phone" data-t="whatsappPhoneId"></label><input id="wa-phone" inputmode="numeric" value="${esc(v['notify.whatsapp.phone_id'] || '')}"></div>
      <div><label for="wa-to" data-t="whatsappTo"></label><input id="wa-to" inputmode="numeric" value="${esc(v['notify.whatsapp.to'] || '')}"></div></div>
    <div class="btn-row"><button class="btn ghost" id="nTest2">${icon('send')}<span data-t="testMessage"></span></button><button class="btn" id="nSave2">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'notify' });
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
      <div class="rowc">${icon('bowl-chopsticks')}<span class="t"><b data-t="chStore"></b><small class="mono">${esc(location.host)}</small></span><span class="pill ok" data-t="on"></span></div>
      <div class="rowc">${icon('phone')}<span class="t"><b data-t="chPhone"></b><small>${esc(S.venue?.phone || '')}</small></span><span class="pill ok" data-t="on"></span></div>
      <button type="button" class="rowc ${waOn ? '' : 'off'}" data-go="notifications">${icon('brand-whatsapp')}<span class="t"><b data-t="whatsapp"></b><small data-t="${waOn ? 'whatsappOn' : 'waNotYet'}"></small></span><span class="pill ${waOn ? 'ok' : ''}" data-t="${waOn ? 'on' : 'off'}"></span></button>
      <button type="button" class="rowc ${igOn ? '' : 'off'}" data-go="social">${icon('sparkles')}<span class="t"><b data-t="instagram"></b><small data-t="${igOn ? 'instagramOn' : 'socialNotYet'}"></small></span><span class="pill ${igOn ? 'ok' : ''}" data-t="${igOn ? 'on' : 'off'}"></span></button>
      <div class="rowc off">${icon('brand-telegram')}<span class="t"><b data-t="chTelegramBot"></b></span><span class="pill" data-t="comingSoon"></span></div>
      <button type="button" class="rowc" data-go="mcp">${icon('cube-3d-sphere')}<span class="t"><b data-t="mcp"></b><small class="mono">${esc(location.origin)}/api/mcp</small></span><span class="pill ok" data-t="on"></span></button>
      <button type="button" class="rowc" data-go="keys">${icon('key')}<span class="t"><b data-t="chApi"></b><small data-t="apiHint"></small></span>${icon('chevron-right', 'chev')}</button>
      <div class="rowc off">${icon('scroll')}<span class="t"><b data-t="chAggregators"></b></span><span class="pill" data-t="comingSoon"></span></div>
    </div>
    <p class="eyebrow mt-3" data-t="webhookUrl"></p>
    <div class="code small" id="whUrl">${esc(webhook)}</div><p class="hint" data-t="webhookHint"></p>
    <label for="wh-verify" data-t="verifyToken"></label><input id="wh-verify" autocomplete="off" value="${esc(v['notify.whatsapp.verify'] || '')}"><p class="hint" data-t="verifyHint"></p>
    <label for="wh-secret" data-t="appSecret"></label><input id="wh-secret" autocomplete="off" placeholder="${v['notify.meta.secret'] === SECRET_SET_MARK ? esc(SECRET_SET_MARK) : ''}">
    <div class="btn-row"><button class="btn ghost" id="whCopy">${icon('copy')}<span data-t="copy"></span></button><button class="btn" id="whSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'channels' });
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

/// The venue as an MCP server: the URL, the auth, the tools it offers.
async function openMcp(){
  sheet(`${head('settings', 'mcp')}<p class="muted small" data-t="mcpHint"></p>
    <p class="eyebrow mt-3">URL</p><div class="code small" id="mcpUrl">${esc(location.origin)}/api/mcp</div>
    <p class="eyebrow mt-3">Authorization</p><div class="code small">Bearer dowiz_…</div>
    <div class="btn-row"><button class="btn ghost" id="mcpCopy">${icon('copy')}<span data-t="copy"></span></button><button class="btn" id="mcpKeys">${icon('key')}<span data-t="apiKeys"></span></button></div>
    <div id="mcpTools"><div class="skel skel-row"></div></div>`, { name: 'mcp' });
  $('#mcpCopy').onclick = async () => { try { await navigator.clipboard.writeText(`${location.origin}/api/mcp`); toast(t('copied')); } catch {} };
  $('#mcpKeys').onclick = openKeys;
  try {
    const d = await fetch('/api/mcp').then(r => r.json());
    $('#mcpTools').innerHTML = `<p class="eyebrow mt-3">${(d.tools || []).length} ${esc(t('tools'))}</p><div class="chips">${(d.tools || []).map(n => `<span class="chip mono">${esc(n)}</span>`).join('')}</div>`;
  } catch (e) { $('#mcpTools').innerHTML = ''; }
}

/// Off-site copies in the venue's own bucket.
async function openCloud(){
  let s = { values: {} }, st = {}; try { [s, st] = await Promise.all([api('/owner/settings'), api('/owner/backup/cloud')]); } catch (e) { fail(e); }
  const v = s.values || {};
  sheet(`${head('settings', 'cloud')}<p class="muted small" data-t="cloudHint"></p>
    <div class="rows"><div class="rowc ${st.configured ? '' : 'off'}">${icon('cloud-upload')}<span class="t"><b data-t="lastCopy"></b><small class="mono">${st.last ? `${esc(day(st.last.atMs))} · ${Math.round((st.last.bytes || 0) / 1024)} KB · ${esc(st.last.key || '')}` : t('neverPushed')}</small></span><span class="pill ${st.configured ? 'ok' : ''}" data-t="${st.configured ? 'nightly' : 'off'}"></span></div></div>
    <label for="cl-endpoint" data-t="endpoint"></label><input id="cl-endpoint" inputmode="url" value="${esc(v['cloud.s3.endpoint'] || '')}" placeholder="https://<account>.r2.cloudflarestorage.com">
    <div class="grid2"><div><label for="cl-region" data-t="region"></label><input id="cl-region" value="${esc(v['cloud.s3.region'] || 'auto')}"></div><div><label for="cl-bucket" data-t="bucket"></label><input id="cl-bucket" value="${esc(v['cloud.s3.bucket'] || '')}"></div></div>
    <div class="grid2"><div><label for="cl-key" data-t="accessKey"></label><input id="cl-key" autocomplete="off" placeholder="${v['cloud.s3.key'] === SECRET_SET_MARK ? esc(SECRET_SET_MARK) : ''}"></div><div><label for="cl-secret" data-t="secretKey"></label><input id="cl-secret" autocomplete="off" placeholder="${v['cloud.s3.secret'] === SECRET_SET_MARK ? esc(SECRET_SET_MARK) : ''}"></div></div>
    <label for="cl-prefix" data-t="prefix"></label><input id="cl-prefix" value="${esc(v['cloud.s3.prefix'] || 'dowiz')}">
    <div class="btn-row"><button class="btn ghost" id="clPush" ${st.configured ? '' : 'disabled'}>${icon('cloud-upload')}<span data-t="pushNow"></span></button><button class="btn" id="clSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'cloud' });
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
  sheet(`${head('inbox', 'inbox')}<p class="muted small" data-t="inboxHint"></p><div id="ibList"><div class="skel skel-row"></div></div>`, { name: 'inbox' });
  let d; try { d = await api('/owner/inbox'); } catch (e) { return fail(e); }
  const th = d.threads || [];
  $('#ibList').innerHTML = th.length ? `<div class="rows">${th.map(x => `<button type="button" class="rowc" data-peer="${esc(x.peer)}" data-ch="${esc(x.channel)}">${icon(x.channel === 'whatsapp' ? 'brand-whatsapp' : 'sparkles')}
      <span class="t"><b>${esc(x.name || x.peer)}</b><small>${x.fromThem ? '' : '↩ '}${esc(x.last)}</small></span>${x.unread ? `<span class="pill warn">${x.unread}</span>` : `<small class="muted">${esc(ago(x.atMs))}</small>`}</button>`).join('')}</div>`
    : `<div class="empty">${icon('message-2')}<b data-t="noMessages"></b><span class="muted small">${d.channels?.whatsapp || d.channels?.instagram ? '' : t('waNotYet')}</span></div>`;
  paint();
  for (const b of $$('[data-peer]', $('#ibList'))) b.onclick = () => openThread(b.dataset.ch, b.dataset.peer, th.find(x => x.peer === b.dataset.peer && x.channel === b.dataset.ch)?.name);
}
async function openThread(channel, peer, name){
  sheet(`<p class="eyebrow">${esc(channel)}</p><h2>${esc(name || peer)}</h2><div class="thread" id="thread"><div class="skel skel-row"></div></div>
    <div class="reply-row"><input id="rp-text" data-t-attr="placeholder:writeReply" autocomplete="off"><button type="button" class="act pri" id="rpGo">${icon('send')}</button></div>`, { name: 'thread' });
  retranslate($('#sheetIn'));
  const draw = async () => {
    let d; try { d = await api(`/owner/inbox/${encodeURIComponent(peer)}?channel=${encodeURIComponent(channel)}`); } catch (e) { return fail(e); }
    $('#thread').innerHTML = (d.messages || []).map(m => `<div class="msg ${m.fromThem ? 'them' : 'me'}"><span>${esc(m.text)}</span><small>${esc(clock(m.atMs))}</small></div>`).join('') || `<div class="empty"><b data-t="noMessages"></b></div>`;
    paint(); $('#thread').scrollTop = $('#thread').scrollHeight;
  };
  draw();
  const send = async () => { const text = $('#rp-text').value.trim(); if (!text) return; try { await busy($('#rpGo'), () => post(`/owner/inbox/${encodeURIComponent(peer)}`, withLoc({ channel, text }))); $('#rp-text').value = ''; draw(); } catch (e) { fail(e); } };
  $('#rpGo').onclick = send; $('#rp-text').onkeydown = e => { if (e.key === 'Enter') send(); };
}

async function openKeys(){
  sheet(`${head('settings', 'apiKeys')}<p class="muted small" data-t="apiHint"></p><div id="kList"><div class="skel skel-row"></div></div>
    <label for="k-label" data-t="name"></label><input id="k-label"><div class="btn-row"><button class="btn" id="kNew">${icon('key')}<span data-t="newKey"></span></button></div><div id="kOut"></div>`, { name: 'keys' });
  const draw = async () => { let d; try { d = await api('/owner/apikeys'); } catch (e) { return fail(e); }
    $('#kList').innerHTML = (d.keys || []).length ? `<div class="rows">${d.keys.map(k => `<div class="rowc">${icon('key')}<span class="t"><b>${esc(k.label || '')}</b><small class="mono">${esc(k.id.slice(0, 8))} · ${k.lastUsedMs ? esc(ago(k.lastUsedMs)) : '—'} · ${t('until')} ${esc(day(k.expiresMs || 0))}</small></span><button type="button" class="act danger" data-rev="${esc(k.id)}">${icon('trash')}</button></div>`).join('')}</div>` : `<div class="empty">${icon('key')}<b data-t="none"></b></div>`;
    paint(); for (const b of $$('[data-rev]', $('#kList'))) b.onclick = async () => { const ok = await confirm(t('remove'), t('revokeHint'), { danger: true }); if (!ok) return openKeys(); try { await post('/owner/apikeys/revoke', { id: b.dataset.rev }); draw(); } catch (e) { fail(e); } }; };
  draw();
  $('#kNew').onclick = async () => { try { const d = await busy($('#kNew'), () => post('/owner/apikeys', { label: $('#k-label').value.trim() || 'api' })); $('#kOut').innerHTML = `<div class="code">${esc(d.key || d.token || JSON.stringify(d))}</div><p class="hint" data-t="keyOnce"></p>`; paint(); draw(); } catch (e) { fail(e); } };
}
async function openBranding(){
  let b; try { b = await api('/owner/branding'); } catch (e) { return fail(e); }
  const st = S.venue?.stage || {};
  sheet(`${head('settings', 'branding')}
    <div class="grid3"><div><label data-t="primary"></label><input type="color" id="b-primary" value="${esc(b.brand?.primary || '#c9a35a')}"></div>
      <div><label data-t="paper"></label><input type="color" id="b-paper" value="${esc(b.brand?.paper || '#0b1717')}"></div>
      <div><label data-t="typePair"></label><select id="b-type">${(b.typePairs || [{ id: 'classic' }]).map(p => `<option value="${esc(p.id)}" ${p.id === (b.brand?.typePair || 'classic') ? 'selected' : ''}>${esc(p.id)}</option>`).join('')}</select></div></div>
    <label for="b-logo" data-t="uploadPhoto"></label><input type="file" id="b-logo" accept="image/*">
    <p class="eyebrow mt-3" data-t="seal"></p>
    <div class="grid2"><div><label data-t="seal"></label><input id="s-seal" value="${esc(st.seal || '')}" maxlength="12"></div>
      <div><label data-t="motif"></label><select id="s-motif">${['leaf', 'wave', 'none'].map(m => `<option value="${m}" ${st.motif === m ? 'selected' : ''}>${esc(t(m === 'none' ? 'noneMotif' : m))}</option>`).join('')}</select></div></div>
    <div class="grid2"><div><label data-t="warmTone"></label><input type="color" id="s-warm" value="${esc(st.warm || '#e0754d')}"></div><div><label data-t="sageTone"></label><input type="color" id="s-sage" value="${esc(st.sage || '#8a9a7b')}"></div></div>
    <div class="btn-row"><button class="btn ghost" id="bPreview">${icon('eye')}<span data-t="preview"></span></button><button class="btn" id="bSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'brand' });
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
  sheet(`${head('settings', 'features')}${groups.map(g => `<p class="eyebrow mt-3">${esc(tl('surface_' + g, g))}</p>${(d.features || []).filter(f => (f.surface || 'storefront') === g).map(f => `<label class="switch"><input type="checkbox" data-f="${esc(f.key)}" ${f.on ? 'checked' : ''}><span class="switch-k"></span><span class="t">${esc(tl('feat_' + f.key, f.label))}${f.defaultOn != null && f.on !== f.defaultOn ? ` <span class="pill warn">${esc(t('changed'))}</span>` : ''}<small>${esc(tl('feat_' + f.key + '_h', f.hint || ''))}</small></span></label>`).join('')}`).join('')}`, { name: 'features' });
  for (const el of $$('[data-f]', $('#sheetIn'))) el.onchange = async () => { try { await post('/owner/features', { key: el.dataset.f, on: el.checked }); toast(t('saved')); } catch (e) { fail(e); el.checked = !el.checked; } };
}
async function openActivation(){
  let a; try { a = await api('/owner/activation'); } catch (e) { return fail(e); }
  const tl = (k, fb) => { const v = t(k); return v === k ? fb : v; };
  const word = v => typeof v === 'boolean' ? t(v ? 'yes' : 'no') : String(v);
  const missing = new Map((a.missing || []).map(m => [m.key, m.why]));
  const CHECKS = [['menu', ['sellableDishes']], ['notifications', ['telegramChats']], ['fulfilment', ['hasVenuePhone', 'deliveryConfigured', 'pickupEnabled']]];
  sheet(`${head('settings', 'activation')}
    <div class="rows">${CHECKS.map(([key, facts]) => `<div class="rowc ${missing.has(key) ? '' : ''}">${icon(missing.has(key) ? 'alert-circle' : 'check')}<span class="t"><b>${esc(tl('req_' + key, key))}</b><small>${missing.has(key) ? esc(missing.get(key)) : facts.map(f => `${esc(tl('fact_' + f, f))}: ${esc(word(a.facts?.[f]))}`).join(' · ')}</small></span><span class="pill ${missing.has(key) ? 'warn' : 'ok'}">${missing.has(key) ? '!' : '✓'}</span></div>`).join('')}</div>
    ${!(a.missing || []).length ? `<p class="ok mt-3" data-t="hubOk"></p>` : ''}`, { name: 'activation' });
}
/// Fullness per mille at which a fixed-size image turns amber, then red.
const HEALTH_WARN_PM = 650, HEALTH_BAD_PM = 850;
async function openHealth(){
  sheet(`${head('settings', 'health')}<div id="hBody"><div class="skel skel-row"></div></div>
    <div class="btn-row"><a class="btn ghost" id="hBackup" href="/api/owner/backup" download>${icon('download')}<span data-t="backup"></span></a></div>`, { name: 'health' });
  let h; try { h = await api('/owner/health'); } catch (e) { return fail(e); }
  // A self-growing image is never amber: fullness is not a warning when the
  // ceiling moves. Fixed-size images first, then the fullest.
  const entries = Object.entries(h.images || {}).sort(([, a], [, b]) => (a.grows === b.grows ? (b.usedPerMille || 0) - (a.usedPerMille || 0) : a.grows ? 1 : -1));
  const tone = im => im.grows ? '' : im.usedPerMille > HEALTH_BAD_PM ? 'bad' : im.usedPerMille > HEALTH_WARN_PM ? 'warn' : '';
  $('#hBody').innerHTML = `<div class="rows">${entries.map(([k, im]) => `<div class="rowc">${icon('cube-3d-sphere')}<span class="t"><b>${esc(t('img_' + k) === 'img_' + k ? k : t('img_' + k))}</b><small class="mono">${Math.round((im.usedPerMille || 0) / 10)}% · ${im.usedCells}/${im.ceilingCells} · gen ${im.generation}${im.grows ? ` · ${esc(t('grows'))}` : ''}</small><span class="gauge"><i class="${tone(im)}" data-w="${Math.round((im.usedPerMille || 0) / 10)}"></i></span></span></div>`).join('')}</div>
    <div class="rows mt-3"><div class="rowc">${icon(h.verdict === 'ok' ? 'check' : 'alert-circle')}<span class="t"><b data-t="verdict_${esc(h.verdict || 'ok')}"></b><small class="mono">${h.orders ?? ''} · ${esc(t('orders7'))}</small></span><span class="pill ${h.verdict === 'ok' ? 'ok' : h.verdict === 'watch' ? 'warn' : 'bad'}">${esc(h.verdict || '')}</span></div></div>`;
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
    <label for="as-q" data-t="ask"></label><textarea id="as-q" rows="2"></textarea>
    <div class="btn-row"><button class="btn" id="asGo">${icon('sparkles')}<span data-t="ask"></span></button></div>
    <div id="asOut"></div>
    <p class="eyebrow mt-3" data-t="aiSettings"></p>
    ${switchEl('ai-on', v['ai.enabled'] === '1', 'aiEnabled', 'aiEnabledHint')}
    <label for="ai-endpoint" data-t="aiEndpoint"></label><input id="ai-endpoint" inputmode="url" value="${esc(v['ai.endpoint'] || '')}" placeholder="https://…/v1">
    <div class="grid2"><div><label for="ai-model" data-t="aiModel"></label><input id="ai-model" value="${esc(v['ai.model'] || '')}"></div>
      <div><label for="ai-token" data-t="aiToken"></label><input id="ai-token" autocomplete="off" placeholder="${v['ai.token'] === SECRET_SET_MARK ? esc(SECRET_SET_MARK) : ''}"></div></div>
    <div class="btn-row"><button class="btn" id="aiSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'assistant' });
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
async function openIntegrations(){
  sheet(`${head('settings', 'integrations')}<p class="muted small" data-t="integrationsHint"></p><div id="igList"><div class="skel skel-row"></div><div class="skel skel-row"></div></div>
    <div class="btn-row"><button class="btn" id="igAll">${icon('check')}<span data-t="checkAll"></span></button></div>`, { name: 'integrations' });
  let st; try { st = await api('/owner/integrations'); } catch (e) { return fail(e); }
  $('#igList').innerHTML = `<div class="rows">${INTEGRATIONS.map(([k, ic]) => `<div class="igrow ${integrationOn(k, st) ? 'on' : ''}" data-ig="${k}">
      <div class="igrow-h">${icon(ic)}<b data-t="${k}"></b><span class="pill ${integrationOn(k, st) ? 'ok' : ''}" data-t="${integrationOn(k, st) ? 'on' : 'off'}"></span></div>
      <small class="ig-line">${esc(integrationLine(k, st))}</small><small class="ig-out mono" hidden></small>
      <div class="igrow-a"><button type="button" class="act" data-cfg="${k}">${icon('adjustments')}<span data-t="configure"></span></button><button type="button" class="act pri" data-chk="${k}">${icon('check')}<span data-t="check"></span></button></div></div>`).join('')}</div>`;
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
