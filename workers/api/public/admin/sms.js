// The SMS screen (W-SMS): order-status texts to customers who ticked the SMS
// box, sent from the venue's OWN Android phone (SMSGate, free) or the owner's
// own textbee / Twilio account. Never a platform key.
//
// The state is the hub's (GET /api/owner/sms): the secret is "set" or not,
// never shown. A Test button proves the gateway before customer texts are on.
// The venue travels as ?location_id=, because the hub refuses unknown body fields.
//
// ASCII QUOTES ONLY in this file.

import { $, esc, icon, t, api, post, toast, sheet, store, busy, ago, switchEl } from '/admin/core.js';
import { T, LANGS, retranslate } from '/admin/i18n.js';
import { k, btn, field, select, pill, loading, rowDiv } from '/admin/parts.js';
import { WORDS } from '/admin/sms-words.js';
import { statusOf, editBody } from '/admin/sms-view.js';

// Merged, never replaced: a language with no row of its own reads English.
for (const l of LANGS) for (const [key, v] of Object.entries({ ...WORDS.en, ...(WORDS[l] || {}) })) if (T[l] && !(key in T[l])) T[l][key] = v;

const q = () => '?location_id=' + encodeURIComponent(store.loc || '');
const fail = e => toast(String((e && e.message) || e));
const head = '<p class="eyebrow" data-t="settings"></p><h2 data-t="sms"></h2><p class="muted small" data-t="sms_hint"></p>';

export async function open(){
  sheet(`${head}<div id="smsBody">${loading(2)}</div>`, { name: 'sms', keepScroll: true });
  let d;
  try { d = await api('/owner/sms' + q()); } catch (e) { return fail(e); }
  const cfg = d.config || {}, h = d.health || {};
  const st = statusOf(cfg, h);
  const why = h.last_err ? `${esc(ago(h.last_err.at_ms))} · ${esc(t(h.last_err.why))} (${esc(h.last_err.code)})` : '';
  const providers = (d.providers || ['smsgate']).map(p => ({ value: p, key: 'sms_p_' + p }));
  const body = `
    <div class="rows">
      ${rowDiv({ leading: icon('phone'), title: { t: 'sms_status' }, sub: `<span class="mono">${esc(h.sent || 0)} ${esc(t('sms_sentToday'))} · ${esc(d.waiting || 0)} ${esc(t('sms_waiting'))} · ${esc(h.stopped || 0)} ${esc(t('sms_stopped'))} · ${esc(h.over_budget || 0)} ${esc(t('sms_overBudget'))}</span>`, trailing: pill(st[0], { key: st[1] }), tour: 'sms.status' })}
      ${rowDiv({ leading: icon('check'), title: { t: 'sms_lastOk' }, sub: `<span class="mono">${h.last_ok_ms ? esc(ago(h.last_ok_ms)) : esc(t('sms_never'))}</span>` })}
      ${why ? rowDiv({ leading: icon('alert-triangle'), title: { t: 'sms_lastErr' }, sub: `<span class="mono">${why}</span>` }) : ''}
    </div>
    <section class="group mt-3"><p class="eyebrow" data-t="sms_setupTitle"></p><p class="muted small" data-t="sms_setup" data-tour="sms.setup"></p>
      ${switchEl('sms-on', !!cfg.on, 'sms_on', 'sms_onHint', 'sms.on')}
      ${select({ id: 'sms-provider', key: 'sms_provider', value: cfg.provider || 'smsgate', options: providers, tour: 'sms.provider' })}
      ${field({ id: 'sms-url', key: 'sms_url', inputmode: 'url', value: cfg.url || '', placeholder: cfg.default_url || '', tour: 'sms.url' })}
      <div class="grid2">${field({ id: 'sms-user', key: 'sms_user', autocomplete: 'off', value: cfg.user || '', tour: 'sms.user' })}
        ${field({ id: 'sms-secret', key: 'sms_secret', type: 'password', autocomplete: 'new-password', phKey: cfg.secret_set ? 'sms_secretSet' : undefined, tour: 'sms.secret' })}</div>
      <div class="grid2">${field({ id: 'sms-from', key: 'sms_from', inputmode: 'tel', value: cfg.from || '', placeholder: '+355...', tour: 'sms.from' })}
        ${field({ id: 'sms-daily', key: 'sms_daily', inputmode: 'numeric', value: cfg.daily || 60, hintKey: 'sms_dailyHint', tour: 'sms.daily' })}</div>
      <div class="btn-row">${btn({ id: 'smsSave', variant: 'primary', icon: 'check', key: 'save', tour: 'sms.save' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="sms_test"></p>
      ${field({ id: 'sms-testPhone', key: 'sms_testPhone', inputmode: 'tel', placeholder: '+355 69 ...', tour: 'sms.testPhone' })}
      <div class="btn-row">${btn({ id: 'smsTest', icon: 'send', key: 'sms_test', tour: 'sms.test' })}</div></section>
    <section class="group mt-3"><p class="eyebrow" data-t="sms_stopTitle"></p><p class="muted small" data-t="sms_stopHint"></p>
      ${field({ id: 'sms-stopPhone', key: 'phone', inputmode: 'tel', placeholder: '+355 69 ...', tour: 'sms.stopPhone' })}
      <div class="btn-row">${btn({ id: 'smsStop', variant: 'danger', icon: 'x', key: 'sms_stopBtn', tour: 'sms.stop' })}</div></section>`;
  sheet(head + `<div id="smsBody">${body}</div>`, { name: 'sms', keepScroll: true });
  wire();
}

function wire(){
  $('#smsSave').onclick = async () => {
    const body = editBody({ on: $('#sms-on').checked, provider: $('#sms-provider').value, url: $('#sms-url').value, user: $('#sms-user').value, secret: $('#sms-secret').value, from: $('#sms-from').value, daily: $('#sms-daily').value });
    try { await busy($('#smsSave'), () => post('/owner/sms' + q(), body)); toast(t('saved')); open(); } catch (e) { fail(e); }
  };
  $('#smsTest').onclick = async () => {
    const phone = $('#sms-testPhone').value.trim(); if (!phone) return toast(k('sms_testPhone'));
    try {
      const r = await busy($('#smsTest'), () => post('/owner/sms/test' + q(), { phone }));
      toast(r.ok ? t('sms_testOk') : `${t(r.why || 'sms_why_provider')}${r.status ? ' (' + r.status + ')' : ''}`);
    } catch (e) { fail(e); }
  };
  $('#smsStop').onclick = async () => {
    const phone = $('#sms-stopPhone').value.trim(); if (!phone) return;
    try { await busy($('#smsStop'), () => post('/owner/sms/stop' + q(), { phone })); toast(t('sms_stoppedOk')); $('#sms-stopPhone').value = ''; } catch (e) { fail(e); }
  };
}

/// The health pane's SMS row (more.js openHealth), when texts are on here.
export function healthRow(host, s){
  if (!host || !s || !s.on) return;
  const tone = s.error || s.missing ? 'bad' : (s.health?.last_err && s.health.last_err.at_ms > (s.health.last_ok_ms || 0)) ? 'warn' : 'ok';
  const word = s.error ? esc(s.error) : s.missing ? esc(t(s.missing)) : `${esc(s.health?.sent || 0)} ${esc(t('sms_sentToday'))} · ${esc(s.waiting || 0)} ${esc(t('sms_waiting'))}`;
  host.insertAdjacentHTML('beforeend', `<div class="rows mt-3">${rowDiv({ leading: icon('phone'), title: { t: 'sms' }, sub: `<span class="mono">${word}</span>`, trailing: pill(tone, { key: tone === 'ok' ? 'sms_live' : 'sms_failing' }), tour: 'health.sms' })}</div>`);
  retranslate(host);
}
