// "More" -- everything that is not the day's service, as one list of rows,
// each a sheet: marketing (promo codes, posts, social), analytics, customers,
// and the settings a venue sets once (the venue itself, hours, delivery,
// payments, notifications, order channels, brand, features, API keys, health,
// backup). The list is the map; nothing is buried under a fold.

import { $, $$, esc, icon, t, S, api, post, withLoc, toast, sheet, closeSheet, money, moneyEl, busy, confirm, switchEl, store, day, hydrate } from '/admin/core.js';
import { retranslate, lang, LANGS } from '/admin/i18n.js';
import { loadVenue, rerender } from '/admin/app.js';

/// The rows, in groups, with the sheet each opens.
const GROUPS = [
  ['marketing', [['promos', 'ticket', openPromos], ['posts', 'send', openPosts], ['social', 'sparkles', openSocial]]],
  ['analytics', [['analytics', 'chart-bar', openAnalytics], ['customers', 'user', openCustomers]]],
  ['settings',  [['venue', 'home', openVenue], ['hours', 'clock', openHours], ['deliveryTerms', 'bike', openDelivery], ['payments', 'coin-hole', openPayments],
                 ['notifications', 'brand-telegram', openNotifications], ['channels', 'scroll', openChannels], ['branding', 'fan', openBranding],
                 ['features', 'tools-kitchen-2', openFeatures], ['apiKeys', 'key', openKeys], ['activation', 'check', openActivation], ['health', 'cube-3d-sphere', openHealth]]],
];
/// The analytics windows the hub answers, in days.
const WINDOWS = [7, 30];
/// The hours grid: minutes of a day, and the minute steps a venue picks from.
const DAY_MIN = 24 * 60;
/// Promo kinds the hub knows.
const PROMO_KINDS = ['percent', 'amount'];
/// A photograph's max side for the logo upload.
const LOGO_MAX_PX = 800;

export async function render(host){
  host.innerHTML = `<div class="screen-h"><div><p class="eyebrow" data-t="tabMore"></p><h1>${esc(S.venue?.name || '')}</h1></div></div>
    ${GROUPS.map(([g, rows]) => `<section class="group"><p class="eyebrow" data-t="${g}"></p><div class="rows">
      ${rows.map(([key, ic]) => `<button type="button" class="rowc" data-open="${key}">${icon(ic)}<span class="t"><b data-t="${key}"></b></span>${icon('chevron-right', 'chev')}</button>`).join('')}</div></section>`).join('')}
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
    $('#pList').innerHTML = list.length ? `<div class="rows">${list.map(p => `<div class="rowc">${icon('ticket')}<span class="t"><b class="mono">${esc(p.code)}</b><small>${p.kind === 'percent' ? `−${p.value}%` : `−${money(p.value)}`}${p.minOrder ? ` · ${t('minOrder')} ${money(p.minOrder)}` : ''} · ${p.used || 0}${p.maxUses ? '/' + p.maxUses : ''}</small></span>
      <span class="pill ${p.active ? 'ok' : ''}" data-t="${p.active ? 'on' : 'off'}"></span><button type="button" class="act danger" data-del="${esc(p.code)}">${icon('trash')}</button></div>`).join('')}</div>` : `<div class="empty">${icon('ticket')}<b data-t="none"></b></div>`;
    paint();
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
      if ($('#pr-from').value) body.fromMs = new Date($('#pr-from').value).getTime();
      if ($('#pr-until').value) body.untilMs = new Date($('#pr-until').value).getTime() + DAY_MIN * 60_000 - 1;
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
  sheet(`${head('marketing', 'social')}<p class="muted small" data-t="autopostHint"></p>
    ${switchEl('so-on', v['social.enabled'] === '1', 'autopost')}
    <label for="so-ch" data-t="tgChannel"></label><input id="so-ch" value="${esc(v['social.telegram.channel'] || '')}" placeholder="@channel">
    <div class="rows mt-3">
      ${['instagram', 'facebook', 'tiktok'].map(n => `<div class="rowc off">${icon('sparkles')}<span class="t"><b data-t="${n}"></b><small data-t="socialNotYet"></small></span><span class="pill" data-t="comingSoon"></span></div>`).join('')}
    </div>
    <div class="btn-row"><button class="btn" id="soSave">${icon('check')}<span data-t="save"></span></button></div>`, { name: 'social' });
  $('#soSave').onclick = async () => {
    try {
      await post('/owner/settings', { key: 'social.enabled', value: $('#so-on').checked ? '1' : '0' });
      await post('/owner/settings', { key: 'social.telegram.channel', value: $('#so-ch').value.trim() });
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
      <div class="stat"><small data-t="avgCheck"></small><b>${money(a.averageOrder || 0)}</b></div><div class="stat"><small data-t="delivery"></small><b>${a.delivery ?? 0}/${a.pickup ?? 0}</b></div></div>
    <p class="eyebrow" data-t="byDay"></p><div class="bars">${byDay.map(d => `<i data-h="${Math.round(100 * (d.revenue || 0) / maxRev)}"></i>`).join('')}</div>
    <div class="axis"><span>${byDay.length ? day(byDay[0].at) : ''}</span><span>${byDay.length ? day(byDay[byDay.length - 1].at) : ''}</span></div>
    <p class="eyebrow mt-3" data-t="byHour"></p><div class="bars">${byHour.map((n, h) => `<i class="${n === maxH ? 'hi' : ''}" data-h="${Math.round(100 * n / maxH)}"></i>`).join('')}</div><div class="axis"><span>00</span><span>12</span><span>23</span></div>
    <p class="eyebrow mt-3" data-t="topDishes"></p><div class="rows">${(a.topProducts || []).slice(0, 8).map(p => `<div class="rowc">${icon('bowl-chopsticks')}<span class="t"><b>${esc(p.name || p.id)}</b></span><span class="mono">${p.count ?? p.orders ?? ''}</span></div>`).join('')}</div>`;
  paint();
}
async function openCustomers(){
  sheet(`${head('analytics', 'customers')}<div id="cuBody"><div class="skel skel-row"></div></div>`, { name: 'customers' });
  let d; try { d = await api('/owner/customers'); } catch (e) { return fail(e); }
  const list = d.customers || [];
  $('#cuBody').innerHTML = list.length ? `<div class="rows">${list.map(c => `<div class="rowc">${icon('user')}<span class="t"><b>${esc(c.name || c.phone || c.key)}</b><small class="mono">${esc(c.phone || '')} · ${c.orders} · ${money(c.spent || 0)}</small></span>${c.lastAt ? `<small class="muted">${esc(day(c.lastAt))}</small>` : ''}</div>`).join('')}</div>` : `<div class="empty">${icon('user')}<b data-t="none"></b></div>`;
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
  sheet(`${head('settings', 'notifications')}
    <div class="rows">
      <div class="rowc">${icon('brand-telegram')}<span class="t"><b data-t="telegram"></b><small data-t="${live ? 'tgHow' : 'tgNotSet'}"></small></span><span class="pill ${live ? 'ok' : 'warn'}" data-t="${live ? 'on' : 'off'}"></span></div>
    </div>
    <label for="n-token" data-t="tgToken"></label><input id="n-token" autocomplete="off" spellcheck="false" placeholder="${tokenSet ? esc(SECRET_SET_MARK) : '123456:ABC…'}"><p class="hint" data-t="tgTokenHint"></p>
    <label for="n-chat" data-t="ownerChat"></label><input id="n-chat" inputmode="numeric" value="${esc(chat)}" placeholder="chat id"><p class="hint" data-t="tgChatHint"></p>
    <div class="btn-row"><button class="btn ghost" id="nTest">${icon('send')}<span data-t="testMessage"></span></button><button class="btn" id="nSave">${icon('check')}<span data-t="save"></span></button></div>
    <div class="rows mt-3"><div class="rowc off">${icon('phone')}<span class="t"><b data-t="whatsapp"></b><small data-t="waNotYet"></small></span><span class="pill" data-t="comingSoon"></span></div></div>`, { name: 'notify' });
  const save = async () => {
    // An empty value CLEARS a setting on the hub, so the token is only sent when typed.
    const tok = $('#n-token').value.trim();
    if (tok) await post('/owner/settings', { key: 'notify.telegram.token', value: tok });
    await post('/owner/settings', { key: 'notify.telegram.chat', value: $('#n-chat').value.trim() });
  };
  $('#nSave').onclick = async () => { try { await busy($('#nSave'), save); toast(t('saved')); openNotifications(); } catch (e) { fail(e); } };
  $('#nTest').onclick = async () => { try { await busy($('#nTest'), async () => { await save(); await post('/owner/notify/test', withLoc()); }); toast(t('testOk')); } catch (e) { fail(e); } };
}
async function openChannels(){
  sheet(`${head('settings', 'channels')}
    <div class="rows">
      <div class="rowc">${icon('bowl-chopsticks')}<span class="t"><b data-t="chStore"></b><small class="mono">${esc(location.host)}</small></span><span class="pill ok" data-t="on"></span></div>
      <div class="rowc">${icon('phone')}<span class="t"><b data-t="chPhone"></b><small>${esc(S.venue?.phone || '')}</small></span><span class="pill ok" data-t="on"></span></div>
      <div class="rowc off">${icon('brand-telegram')}<span class="t"><b data-t="chTelegramBot"></b></span><span class="pill" data-t="comingSoon"></span></div>
      <div class="rowc">${icon('key')}<span class="t"><b data-t="chApi"></b><small data-t="apiHint"></small></span><button type="button" class="act" id="toKeys">${icon('chevron-right')}</button></div>
      <div class="rowc off">${icon('scroll')}<span class="t"><b data-t="chAggregators"></b></span><span class="pill" data-t="comingSoon"></span></div>
    </div>`, { name: 'channels' });
  $('#toKeys').onclick = openKeys;
}
async function openKeys(){
  sheet(`${head('settings', 'apiKeys')}<p class="muted small" data-t="apiHint"></p><div id="kList"><div class="skel skel-row"></div></div>
    <label for="k-label" data-t="name"></label><input id="k-label"><div class="btn-row"><button class="btn" id="kNew">${icon('key')}<span data-t="newKey"></span></button></div><div id="kOut"></div>`, { name: 'keys' });
  const draw = async () => { let d; try { d = await api('/owner/apikeys'); } catch (e) { return fail(e); }
    $('#kList').innerHTML = (d.keys || []).length ? `<div class="rows">${d.keys.map(k => `<div class="rowc">${icon('key')}<span class="t"><b>${esc(k.label || '')}</b><small class="mono">${esc(k.session || '')}</small></span><button type="button" class="act danger" data-rev="${esc(k.session || '')}">${icon('trash')}</button></div>`).join('')}</div>` : `<div class="empty">${icon('key')}<b data-t="none"></b></div>`;
    paint(); for (const b of $$('[data-rev]', $('#kList'))) b.onclick = async () => { try { await post('/owner/apikeys/revoke', { session: b.dataset.rev }); draw(); } catch (e) { fail(e); } }; };
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
  sheet(`${head('settings', 'features')}${(d.features || []).map(f => `<label class="switch"><input type="checkbox" data-f="${esc(f.key)}" ${f.on ? 'checked' : ''}><span class="switch-k"></span><span class="t">${esc(f.label)}<small>${esc(f.hint || '')}</small></span></label>`).join('')}`, { name: 'features' });
  for (const el of $$('[data-f]', $('#sheetIn'))) el.onchange = async () => { try { await post('/owner/features', { key: el.dataset.f, on: el.checked }); toast(t('saved')); } catch (e) { fail(e); el.checked = !el.checked; } };
}
async function openActivation(){
  let a; try { a = await api('/owner/activation'); } catch (e) { return fail(e); }
  sheet(`${head('settings', 'activation')}
    <div class="rows">${(a.missing || []).map(m => `<div class="rowc">${icon('alert-circle')}<span class="t"><b>${esc(m.key)}</b><small>${esc(m.why)}</small></span><span class="pill warn">!</span></div>`).join('')}
      ${!(a.missing || []).length ? `<div class="rowc">${icon('check')}<span class="t"><b data-t="hubOk"></b></span><span class="pill ok" data-t="on"></span></div>` : ''}</div>
    <p class="hint mono">${Object.entries(a.facts || {}).map(([k, v]) => `${esc(k)}: ${esc(String(v))}`).join(' · ')}</p>`, { name: 'activation' });
}
async function openHealth(){
  sheet(`${head('settings', 'health')}<div id="hBody"><div class="skel skel-row"></div></div>
    <div class="btn-row"><a class="btn ghost" id="hBackup" href="/api/owner/backup" download>${icon('download')}<span data-t="backup"></span></a></div>`, { name: 'health' });
  let h; try { h = await api('/owner/health'); } catch (e) { return fail(e); }
  $('#hBody').innerHTML = `<div class="rows">${Object.entries(h.images || {}).map(([k, im]) => `<div class="rowc">${icon('cube-3d-sphere')}<span class="t"><b>${esc(k)}</b><small class="mono">${im.usedCells}/${im.ceilingCells} · gen ${im.generation}</small><span class="gauge"><i class="${im.usedPerMille > 850 ? 'bad' : im.usedPerMille > 650 ? 'warn' : ''}" data-w="${Math.round(im.usedPerMille / 10)}"></i></span></span></div>`).join('')}</div>
    <p class="hint mono">${esc(h.verdict || '')} · ${h.orders ?? ''}</p>`;
  paint();
  $('#hBackup').onclick = async e => { e.preventDefault(); try { const r = await fetch('/api/owner/backup', { headers: { authorization: 'Bearer ' + store.t } }); const blob = await r.blob(); const a = document.createElement('a'); a.href = URL.createObjectURL(blob); a.download = `dowiz-${store.loc}-${new Date().toISOString().slice(0, 10)}.json`; a.click(); } catch (err) { fail(err); } };
}
