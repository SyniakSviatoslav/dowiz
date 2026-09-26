// The Telegram screen, drawn (W-TG T5). PURE: the answer of
// GET /api/owner/telegram in, escaped HTML out -- no DOM, no clock, no
// network -- so every piece renders in node (telegram.test.mjs).
//
// ONE SCREEN, EVERYTHING VISIBLE (operator): the bot, every group with its
// language, customer data, quiet hours, summary time, health and buttons,
// and the matrix of what goes where. No hidden menus.
//
// The rows of the matrix come from the HUB (`events`), never from a list
// here: a row the hub does not know cannot be drawn.
//
// ASCII QUOTES ONLY in this file.
import * as ui from '../lib/ui/index.js';
import { btn, field, input, chips, pill, rowDiv, empty, k } from './parts.js';

export const MODES = ['now', 'digest', 'off'];

/// The next mode when a cell is tapped. A summary is itself a summary:
/// it is now or off.
export function nextMode(mode, scheduled){
  if (scheduled) return mode === 'now' ? 'off' : 'now';
  return MODES[(MODES.indexOf(mode) + 1) % MODES.length] || 'now';
}

/// Venue-local minutes <-> "HH:MM" for a time input.
export const hhmm = m => `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;
export function minutes(s){
  const m = /^(\d{1,2}):(\d{2})$/.exec(String(s || '').trim());
  if (!m) return null;
  const v = Number(m[1]) * 60 + Number(m[2]);
  return v >= 0 && v < 1440 ? v : null;
}

/// The events by area, in the hub's order.
export function areas(events){
  const out = [];
  for (const e of events || []) {
    let a = out.find(x => x.area === e.area);
    if (!a) out.push(a = { area: e.area, events: [] });
    a.events.push(e);
  }
  return out;
}

const MODE_KEY = { now: 'tg_now', digest: 'tg_sum', off: 'tg_off' };
const MODE_ICON = { now: 'send', digest: 'note', off: 'x' };
const STATE_TONE = { active: 'ok', muted: 'warn', left: 'bad' };

/// One cell of the matrix: a chip that cycles.
export function cell(g, e){
  const mode = (g.subs || {})[e.key] || 'off';
  const off = !e.live || g.state === 'left';
  return ui.chip({ as: 'button', selected: mode !== 'off', label: k(MODE_KEY[mode]), icon: MODE_ICON[mode],
    attrs: { disabled: off || null, data: { cell: `${g.id}/${e.key}`, mode, scheduled: e.scheduled ? '1' : '' } } });
}

export function matrix(d){
  const gs = d.groups || [];
  if (!gs.length) return '';
  const head = `<tr><th></th>${gs.map(g => `<th scope="col">${ui.esc(g.title || g.id)}</th>`).join('')}</tr>`;
  const body = areas(d.events).map(a => `<tr class="tg-area"><th colspan="${gs.length + 1}" scope="rowgroup" data-t="tg_area_${ui.esc(a.area)}">${ui.esc(ui.tr('tg_area_' + a.area))}</th></tr>`
    + a.events.map(e => `<tr><th scope="row"><span data-t="tgev_${ui.esc(e.key)}">${ui.esc(ui.tr('tgev_' + e.key))}</span>${e.live ? '' : ' ' + pill('', { key: 'tg_soon' })}</th>`
      + gs.map(g => `<td>${cell(g, e)}</td>`).join('') + '</tr>').join('')).join('');
  return `<section class="group mt-3" id="tgMatrix"><p class="eyebrow" data-t="tg_matrix"></p><p class="muted small" data-t="tg_matrixHint"></p>
    <div class="tg-scroll"><table class="tg-matrix">${head}${body}</table></div></section>`;
}

/// When, in the reader's words; the caller passes the formatter (the clock is the DOM's).
function healthLine(g, when){
  const h = g.health || {};
  if (h.failing) return `<span class="warn" data-t="tg_lastErr">${ui.esc(ui.tr('tg_lastErr'))}</span> ${ui.esc(when(h.err_ms))}: ${ui.esc(h.err)}`;
  if (h.ok_ms) return `<span data-t="tg_lastOk">${ui.esc(ui.tr('tg_lastOk'))}</span> ${ui.esc(when(h.ok_ms))}`;
  return `<span data-t="tg_never">${ui.esc(ui.tr('tg_never'))}</span>`;
}

export function groupCard(g, d, when = String){
  const id = ui.esc(g.id);
  const where = [g.kind, g.thread ? `${ui.tr('tg_topic')} ${g.thread}` : ''].filter(Boolean).join(' · ');
  const waiting = g.waiting ? ` · ${g.waiting} <span data-t="tg_waitingN">${ui.esc(ui.tr('tg_waitingN'))}</span>` : '';
  const langs = chips({ values: (d.langs || []).map(l => ({ value: l, label: l.toUpperCase() })), value: g.lang, attr: 'glang', labelKey: 'tg_lang' });
  const pii = chips({ values: ['none', 'fulfil', 'full'].map(v => ({ value: v, key: 'tg_pii_' + v })), value: g.pii, attr: 'gpii', labelKey: 'tg_pii' });
  const q = g.quiet;
  const quiet = `<div class="grid3 tg-times">
      ${ui.chip({ as: 'button', selected: !!q, label: k('tg_quietOn'), icon: 'moon', attrs: { data: { gquiet: g.id } } })}
      ${input({ type: 'time', key: 'tg_from', value: q ? hhmm(q.from) : '23:00', data: { gqf: g.id } })}
      ${input({ type: 'time', key: 'tg_to', value: q ? hhmm(q.to) : '07:00', data: { gqt: g.id } })}</div>
    <p class="muted small" data-t="tg_quietHint"></p>`;
  const acts = [
    btn({ icon: 'send', key: 'tg_test', data: { gtest: g.id }, tour: d.groups[0] === g ? 'notify.test' : undefined }),
    btn({ icon: g.state === 'muted' ? 'player-play' : 'power', key: g.state === 'muted' ? 'tg_resume' : 'tg_pause', data: { gmute: g.id }, disabled: g.state === 'left' }),
    btn({ icon: 'trash', key: 'tg_unlink', variant: 'ghost', data: { gunlink: g.id } }),
  ].join('');
  return `<article class="tg-group" data-group="${id}">
    ${rowDiv({ leading: ui.icon('brand-telegram'), title: g.title || g.id, sub: `${ui.esc(where)} · ${healthLine(g, when)}${waiting}`,
      trailing: pill(STATE_TONE[g.state] || '', { key: 'tg_state_' + (g.state || 'active') }) })}
    <p class="ui-label" data-t="tg_lang"></p>${langs}
    <p class="ui-label" data-t="tg_pii"></p>${pii}<p class="muted small" data-t="tg_piiHint"></p>
    <p class="ui-label" data-t="tg_quiet"></p>${quiet}
    ${input({ type: 'time', key: 'tg_digestAt', value: hhmm(g.digest_at ?? 540), data: { gdig: g.id } })}
    <div class="btn-row">${acts}</div></article>`;
}

/// The code, the two steps and the deep link, while a code is pending.
export function wizard(p, left){
  if (!p) return '';
  if (left <= 0) return `<p class="warn small" data-t="tg_expired"></p>`;
  const cmd = p.command || `/link ${p.code}`;
  const mm = `${Math.floor(left / 60000)}:${String(Math.floor(left / 1000) % 60).padStart(2, '0')}`;
  return `<div class="tg-wizard" id="tgWizard" role="status">
    <ol class="tg-steps"><li data-t="tg_step1"></li><li data-t="tg_step2"></li></ol>
    <p class="tg-code mono" id="tgCmd">${ui.esc(cmd)}</p>
    <div class="btn-row">${btn({ id: 'tgCopy', icon: 'copy', key: 'tg_copy' })}${p.deepLink ? btn({ icon: 'external-link', key: 'tg_openTg', href: p.deepLink, target: '_blank' }) : ''}</div>
    <p class="muted small"><span data-t="tg_waiting"></span> <span data-t="tg_expires"></span> <span class="mono" id="tgLeft">${ui.esc(mm)}</span></p></div>`;
}

export function botBlock(d){
  const b = d.bot;
  const sub = b ? `@${ui.esc(b.username)} · <span data-t="tg_connected">${ui.esc(ui.tr('tg_connected'))}</span>` : `<span data-t="tg_notConnected">${ui.esc(ui.tr('tg_notConnected'))}</span>`;
  return `<section class="group"><p class="eyebrow" data-t="tg_bot"></p>
    ${rowDiv({ leading: ui.icon('brand-telegram'), title: k('tg_bot'), sub, trailing: pill(b ? 'ok' : 'warn', { key: b ? 'on' : 'off' }), tour: 'notify.telegramState' })}
    ${field({ id: 'tgToken', key: 'tg_token', hintKey: 'tg_tokenHint', autocomplete: 'off', spellcheck: false, placeholder: d.tokenSet ? '•••• set' : '123456:ABC...', tour: 'notify.tgToken' })}
    <div class="btn-row">${btn({ id: 'tgConnect', icon: 'key', key: 'tg_connect', variant: b ? 'secondary' : 'primary' })}</div></section>`;
}

/// The whole screen. `now` and `when` come from the caller.
export function page(d, pending, now, when = String){
  const gs = d.groups || [];
  const groups = gs.length
    ? gs.map(g => groupCard(g, d, when)).join('')
    : empty('message-2', { key: 'tg_noGroups' });
  return `<p class="eyebrow" data-t="settings"></p><h2 data-t="tg_title"></h2><p class="muted small" data-t="tg_hint"></p>
    ${botBlock(d)}
    <section class="group mt-3"><p class="eyebrow" data-t="tg_groups"></p>
      ${d.legacy && gs.length ? '<p class="muted small" data-t="tg_legacy"></p>' : ''}
      <div class="btn-row">${btn({ id: 'tgAdd', icon: 'plus', key: 'tg_addGroup', variant: d.bot && !gs.length ? 'primary' : 'secondary', disabled: !d.bot, tour: 'notify.tgChat' })}</div>
      <div id="tgWiz">${wizard(pending, pending ? pending.exp_ms - now : 0)}</div>
      <div class="tg-groups">${groups}</div></section>
    ${matrix(d)}`;
}
