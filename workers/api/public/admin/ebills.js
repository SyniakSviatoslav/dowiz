// The till link: dine-in sales from ebills.al beside the delivery orders,
// one stock ledger. Three things on one sheet: is the link alive (and if
// not, why -- in words), which till codes are not yet matched to a dish,
// and the floor as the till sees it.
//
// A MAPPING IS ONLY EVER SET BY THE OWNER'S TAP. The hub sends suggestions
// (same name, and whether the price agrees); nothing here applies one.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { $, $$, esc, icon, t, api, post, toast, sheet, money, ago, store, busy, switchEl } from '/admin/core.js';

/// Every call names the venue in the query: the bodies are closed shapes.
const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="settings"></p><h2 data-t="ebills"></h2><p class="muted small" data-t="ebillsHint"></p>`;

export async function open(){
  sheet(`${head}<div id="ebBody"><div class="skel skel-row"></div><div class="skel skel-row"></div></div>`, { name: 'ebills', keepScroll: true });
  let d;
  try { d = await api('/owner/ebills' + q()); } catch (e) { return fail(e); }
  const st = d.state || {}, cfg = d.config || {};
  const products = d.products || [];
  // Switched on and not yet read is 'not yet', never 'off': the owner just saved it.
  const status = st.halted ? ['bad', 'eb_halted'] : st.last_error ? ['warn', 'eb_failing'] : !cfg.enabled ? ['', 'off'] : st.last_ok_ms ? ['ok', 'eb_live'] : ['', 'eb_never'];
  const opts = (sug) => {
    const first = (sug || []).map(s => `<option value="${esc(s.product_id)}">${esc(s.product || s.product_id)}${s.price_agrees ? '' : ' (' + esc(t('eb_priceDiffers')) + ')'}</option>`).join('');
    const rest = products.map(p => `<option value="${esc(p.id)}">${esc(p.name)} - ${esc(money(p.price))}</option>`).join('');
    return `<option value="" data-t="eb_pick"></option>${first}${first ? '<option disabled>---</option>' : ''}${rest}`;
  };
  const row = (ic, label, value, tone = '') => `<div class="rowc">${icon(ic)}<span class="t"><b data-t="${label}"></b><small class="mono">${value}</small></span>${tone ? `<span class="pill ${tone}"></span>` : ''}</div>`;
  const body = `
    <div class="rows">
      <div class="rowc">${icon('receipt')}<span class="t"><b data-t="ebills"></b><small class="mono">${cfg.user ? esc(cfg.user) + ' · POS ' + esc(cfg.pos_id) : ''}</small></span><span class="pill ${status[0]}" data-t="${status[1]}"></span></div>
      ${row('clock', 'eb_lastOk', st.last_ok_ms ? esc(ago(st.last_ok_ms)) : esc(t('eb_never')))}
      ${st.last_error ? row('alert-triangle', 'eb_lastError', `${esc(ago(st.last_error.at_ms))} · ${esc(st.last_error.why)}`) : ''}
      ${row('download', 'eb_imported', `${st.placed || 0} ${esc(t('eb_placed'))} · ${st.paid || 0} ${esc(t('eb_paid'))} · ${st.noted || 0} ${esc(t('eb_noted'))} · #${esc(st.watermark || 0)}`)}
      ${(st.pending || []).length ? row('clock-hour-4', 'eb_pending', st.pending.map(b => `${esc(t('table'))} ${esc(b.table)} · ${esc(money(b.total))}`).join(' / ')) : ''}
      ${(st.short || []).length ? row('alert-circle', 'eb_short', st.short.map(([i, n]) => `${esc(i)} ${esc(n)}`).join(' · ')) : ''}
      ${(st.refused || []).length ? row('x', 'eb_refused', st.refused.slice(-5).map(r => `#${esc(r.sale_id)} ${esc(r.why)}`).join(' / ')) : ''}
    </div>
    <section class="group mt-3"><p class="eyebrow" data-t="eb_unmatched"></p><p class="muted small" data-t="eb_unmatchedHint"></p>
      <div class="rows">${(d.unmatched || []).map(u => `<div class="rowc" data-code="${esc(u.code)}">${icon('plug-connected-x')}<span class="t"><b>${esc(u.name)}</b><small class="mono">${esc(u.code)} · ${esc(money(u.price))}</small>
        <select class="eb-pick">${opts(u.suggest)}</select></span><button class="btn ghost eb-map">${icon('check')}<span data-t="eb_map"></span></button></div>`).join('') || `<p class="muted small" data-t="eb_allMatched"></p>`}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="eb_mapped"></p>
      <div class="rows">${(d.mapped || []).map(m => `<div class="rowc" data-code="${esc(m.code)}">${icon('receipt')}<span class="t"><b>${esc(m.name || m.code)}</b><small class="mono">${esc(m.code)} &rarr; ${esc(m.product || m.product_id)}</small></span><button class="btn ghost eb-clear">${icon('x')}<span data-t="eb_clear"></span></button></div>`).join('')}</div></section>
    ${d.floor && d.floor.tables ? `<section class="group mt-3"><p class="eyebrow" data-t="eb_floor"></p><p class="muted small">${esc(ago(d.floor.at_ms))}</p>
      <div class="chips">${d.floor.tables.map(f => `<span class="chip ${f.occupied ? 'on' : ''}">${esc(f.table)}${f.unpaid != null ? ' · ' + esc(money(f.unpaid)) : ''}</span>`).join('')}</div></section>` : ''}
    <section class="group mt-3"><p class="eyebrow" data-t="eb_settings"></p>
      ${switchEl('eb-on', !!cfg.enabled, 'eb_enabled', 'eb_enabledHint')}
      <div class="grid2"><div><label for="eb-user" data-t="eb_user"></label><input id="eb-user" autocomplete="off" value="${esc(cfg.user || '')}"></div>
        <div><label for="eb-pos" data-t="eb_pos"></label><input id="eb-pos" inputmode="numeric" value="${esc(cfg.pos_id || 1)}"></div></div>
      <label for="eb-pass" data-t="eb_password"></label><input id="eb-pass" type="password" autocomplete="new-password" placeholder="${cfg.secret_set ? esc(t('eb_passwordSet')) : ''}">
      <p class="muted small" data-t="eb_roleHint"></p>
      <div class="btn-row"><button class="btn" id="ebSave">${icon('check')}<span data-t="save"></span></button>
        ${cfg.user || cfg.secret_set ? `<button class="btn ghost danger" id="ebForget">${icon('x')}<span data-t="eb_forget"></span></button>` : ''}</div></section>`;
  sheet(head + `<div id="ebBody">${body}</div>`, { name: 'ebills', keepScroll: true });
  wire();
}

function wire(){
  for (const b of $$('.eb-map', $('#ebBody'))) b.onclick = async () => {
    const r = b.closest('[data-code]'); const pid = $('.eb-pick', r).value;
    if (!pid) return toast(t('eb_pick'));
    try { await busy(b, () => post('/owner/ebills/map' + q(), { code: r.dataset.code, product_id: pid })); toast(t('saved')); open(); } catch (e) { fail(e); }
  };
  for (const b of $$('.eb-clear', $('#ebBody'))) b.onclick = async () => {
    const r = b.closest('[data-code]');
    try { await busy(b, () => post('/owner/ebills/map' + q(), { code: r.dataset.code })); open(); } catch (e) { fail(e); }
  };
  $('#ebSave').onclick = async () => {
    const pos = Number($('#eb-pos').value.trim());
    const body = { enabled: $('#eb-on').checked, pos_id: Number.isInteger(pos) ? pos : 0, user: $('#eb-user').value.trim() };
    const pass = $('#eb-pass').value;
    if (pass) body.password = pass;
    try { await busy($('#ebSave'), () => post('/owner/ebills/config' + q(), body)); toast(t('saved')); open(); } catch (e) { fail(e); }
  };
  // DISCONNECT (`glue::apply_config`): off with no user forgets the password.
  const forget = $('#ebForget');
  if (forget) forget.onclick = async () => {
    try { await busy(forget, () => post('/owner/ebills/config' + q(), { enabled: false, pos_id: 0, user: '' })); toast(t('eb_forgotten')); open(); } catch (e) { fail(e); }
  };
}
