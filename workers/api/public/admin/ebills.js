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
import { ui, k, btn, field, select, pill, loading, rowDiv } from '/admin/parts.js';

/// Every call names the venue in the query: the bodies are closed shapes.
const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String(e.message || e));
const head = `<p class="eyebrow" data-t="settings"></p><h2 data-t="ebills"></h2><p class="muted small" data-t="ebillsHint"></p>`;

export async function open(){
  sheet(`${head}<div id="ebBody">${loading(2)}</div>`, { name: 'ebills', keepScroll: true });
  let d;
  try { d = await api('/owner/ebills' + q()); } catch (e) { return fail(e); }
  const st = d.state || {}, cfg = d.config || {};
  const products = d.products || [];
  // Switched on and not yet read is 'not yet', never 'off': the owner just saved it.
  const status = st.halted ? ['bad', 'eb_halted'] : st.last_error ? ['warn', 'eb_failing'] : !cfg.enabled ? ['', 'off'] : st.last_ok_ms ? ['ok', 'eb_live'] : ['', 'eb_never'];
  // The pick for one till code: the hub's suggestions first, then every dish.
  const opts = (sug) => {
    const first = (sug || []).map(s => ({ value: s.product_id, label: (s.product || s.product_id) + (s.price_agrees ? '' : ' (' + t('eb_priceDiffers') + ')') }));
    const rest = products.map(p => ({ value: p.id, label: `${p.name} - ${money(p.price)}` }));
    return [{ value: '', key: 'eb_pick' }, ...first, ...(first.length ? [{ value: '---', label: '---', disabled: true }] : []), ...rest];
  };
  const row = (ic, label, value, tour) => rowDiv({ leading: icon(ic), title: { t: label }, sub: `<span class="mono">${value}</span>`, tour });
  const body = `
    <div class="rows">
      ${rowDiv({ leading: icon('receipt'), title: { t: 'ebills' }, sub: `<span class="mono">${cfg.user ? esc(cfg.user) + ' · POS ' + esc(cfg.pos_id) : ''}</span>`, trailing: pill(status[0], { key: status[1] }), tour: 'ebills.status' })}
      ${row('clock', 'eb_lastOk', st.last_ok_ms ? esc(ago(st.last_ok_ms)) : esc(t('eb_never')))}
      ${st.last_error ? row('alert-triangle', 'eb_lastError', `${esc(ago(st.last_error.at_ms))} · ${esc(st.last_error.why)}`) : ''}
      ${row('download', 'eb_imported', `${st.placed || 0} ${esc(t('eb_placed'))} · ${st.paid || 0} ${esc(t('eb_paid'))} · ${st.noted || 0} ${esc(t('eb_noted'))} · #${esc(st.watermark || 0)}`)}
      ${(st.pending || []).length ? row('clock-hour-4', 'eb_pending', st.pending.map(b => `${esc(t('table'))} ${esc(b.table)} · ${esc(money(b.total))}`).join(' / '), 'ebills.pending') : ''}
      ${(st.short || []).length ? row('alert-circle', 'eb_short', st.short.map(([i, n]) => `${esc(i)} ${esc(n)}`).join(' · ')) : ''}
      ${(st.refused || []).length ? row('x', 'eb_refused', st.refused.slice(-5).map(r => `#${esc(r.sale_id)} ${esc(r.why)}`).join(' / ')) : ''}
      ${rowDiv({ leading: icon('receipt'), title: { t: 'fx_title' }, sub: ui.label(k('fx_openHint')), trailing: btn({ id: 'ebFiscal', key: 'fx_open', tour: 'ebills.fiscal' }) })}
    </div>
    <section class="group mt-3"><p class="eyebrow" data-t="eb_unmatched"></p><p class="muted small" data-t="eb_unmatchedHint"></p>
      <div class="rows" data-tour="ebills.unmatched">${(d.unmatched || []).map(u => rowDiv({ leading: icon('plug-connected-x'), title: u.name, data: { code: u.code },
        sub: `<span class="mono">${esc(u.code)} · ${esc(money(u.price))}</span>${select({ controlCls: 'eb-pick', ariaLabel: t('eb_pick'), options: opts(u.suggest), tour: 'ebills.pick' })}`,
        trailing: btn({ cls: 'eb-map', icon: 'check', key: 'eb_map', tour: 'ebills.map' }) })).join('') || `<p class="muted small" data-t="eb_allMatched"></p>`}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="eb_mapped"></p>
      <div class="rows" data-tour="ebills.mapped">${(d.mapped || []).map(m => rowDiv({ leading: icon('receipt'), title: m.name || m.code, data: { code: m.code },
        sub: `<span class="mono">${esc(m.code)} &rarr; ${esc(m.product || m.product_id)}</span>`, trailing: btn({ cls: 'eb-clear', variant: 'ghost', icon: 'x', key: 'eb_clear', tour: 'ebills.clear' }) })).join('')}</div></section>
    ${d.floor && d.floor.tables ? `<section class="group mt-3"><p class="eyebrow" data-t="eb_floor"></p><p class="muted small">${esc(ago(d.floor.at_ms))}</p>
      <div class="chips" data-tour="ebills.floor">${d.floor.tables.map(f => ui.chip({ tone: f.occupied ? 'accent' : 'neutral', dot: !!f.occupied, label: `${f.table}${f.unpaid != null ? ' · ' + money(f.unpaid) : ''}` })).join('')}</div></section>` : ''}
    <section class="group mt-3"><p class="eyebrow" data-t="eb_settings"></p>
      ${switchEl('eb-on', !!cfg.enabled, 'eb_enabled', 'eb_enabledHint', 'ebills.enabled')}
      <div class="grid2">${field({ id: 'eb-user', key: 'eb_user', autocomplete: 'off', value: cfg.user || '', tour: 'ebills.user' })}
        ${field({ id: 'eb-pos', key: 'eb_pos', inputmode: 'numeric', value: cfg.pos_id || 1, tour: 'ebills.pos' })}</div>
      ${field({ id: 'eb-pass', key: 'eb_password', type: 'password', autocomplete: 'new-password', phKey: cfg.secret_set ? 'eb_passwordSet' : undefined, tour: 'ebills.password' })}
      <p class="muted small" data-t="eb_roleHint"></p>
      <div class="btn-row">${btn({ id: 'ebSave', variant: 'primary', icon: 'check', key: 'save', tour: 'ebills.save' })}
        ${cfg.user || cfg.secret_set ? btn({ id: 'ebForget', variant: 'danger', icon: 'x', key: 'eb_forget', tour: 'ebills.forget' }) : ''}</div></section>`;
  sheet(head + `<div id="ebBody">${body}</div>`, { name: 'ebills', keepScroll: true });
  wire();
}

function wire(){
  // FISCAL SENDING (card L70) has its own sheet: arming it is a legal act.
  $('#ebFiscal').onclick = async () => (await import('/admin/fiscal.js')).open();
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
