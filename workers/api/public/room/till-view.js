// What the till screen SAYS about the drawer. PURE: an answer from
// `/api/staff/till/*` in, HTML out, so the blind count is provable in a test.
//
// A BLIND COUNT (`services/orders/room/till.rs`): the counter enters what the
// drawer holds without having seen what it should hold, or the count is a
// copy of the screen. The server already sends `expected` and `over_short`
// only on the close. This renderer does not trust that: it draws them ONLY
// for a `till.closed` answer, and drops them from any other -- a server that
// one day leaks them into a count answer still shows the counter nothing.
import { money, TILL_CURRENCIES } from './logic.js';
import { ui } from './parts.js';

const esc = ui.esc;

export const CLOSED = 'till.closed';

/// The currencies a set of per-currency maps mentions, the drawer's two first.
function codesOf(...maps) {
  const seen = new Set(TILL_CURRENCIES);
  for (const m of maps) for (const k of Object.keys(m || {})) seen.add(k);
  return [...seen].filter(c => maps.some(m => m && c in m));
}

const cell = (m, c, locale) => (m && c in m ? ui.amount(money(m[c], c, locale)) : '—');

/// The fields a person may see for this answer. Exported for the test.
export function visible(ans) {
  if (!ans || typeof ans !== 'object') return null;
  const closed = ans.kind === CLOSED;
  const out = { kind: ans.kind, open: !!ans.open, opened_at: ans.opened_at, float: ans.float || {} };
  if ('counted' in ans) out.counted = ans.counted || {};
  if (closed) {
    for (const k of ['expected', 'over_short', 'pay_in', 'pay_out', 'cash_paid', 'closed_at']) out[k] = ans[k];
  }
  return out;
}

/// The drawer's state as HTML. `t` is the i18n lookup, `locale` for money.
export function renderTill(ans, t, locale, timeOf = ms => new Date(ms).toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })) {
  const v = visible(ans);
  if (!v) return ui.para(t('tillUnknown'), { hint: true });
  const when = v.opened_at ? ` · ${esc(t('openedAt'))} ${esc(timeOf(v.opened_at))}` : '';
  if (v.kind !== CLOSED) {
    const codes = codesOf(v.float, v.counted);
    const rows = codes.map(c => `<tr><th scope="row">${esc(c)}</th><td>${cell(v.float, c, locale)}</td>${v.counted ? `<td>${cell(v.counted, c, locale)}</td>` : ''}</tr>`).join('');
    return `<p class="state">${ui.badge({ tone: v.open ? 'success' : 'neutral', dot: true, label: t(v.open ? 'tillOpen' : 'tillClosedWord') })}${when}</p>
      <table class="z" data-tour="till.z"><thead><tr><th></th><th>${esc(t('tillFloat'))}</th>${v.counted ? `<th>${esc(t('counted'))}</th>` : ''}</tr></thead><tbody>${rows}</tbody></table>
      ${v.counted ? ui.para(t('blindNote'), { hint: true }) : ''}`;
  }
  const codes = codesOf(v.expected, v.counted, v.over_short);
  const rows = codes.map(c => {
    const os = v.over_short && c in v.over_short ? v.over_short[c] : null;
    const cls = os == null ? '' : os < 0 ? ' short' : os > 0 ? ' over' : ' even';
    return `<tr><th scope="row">${esc(c)}</th><td>${cell(v.expected, c, locale)}</td><td>${cell(v.counted, c, locale)}</td><td class="os${cls}">${cell(v.over_short, c, locale)}</td></tr>`;
  }).join('');
  return `<p class="state">${ui.badge({ label: t('tillClosedWord') })}${v.closed_at ? ` · ${esc(timeOf(v.closed_at))}` : ''}</p>
    <table class="z" data-tour="till.z"><thead><tr><th></th><th>${esc(t('expected'))}</th><th>${esc(t('counted'))}</th><th>${esc(t('overShort'))}</th></tr></thead><tbody>${rows}</tbody></table>`;
}

/// THE TIP RECORD beside the Z report (`GET /api/staff/till/tips`): which
/// period to ask about. PURE. The drawer's opening to its close (or to now
/// while it is open) when this phone knows the drawer; otherwise NO START,
/// which the server reads as the venue's day so far -- a card-only day opens
/// no till, and its tips are still somebody's (live walk 2026-09-24).
export function tipsQuery(ans, loc) {
  if (!loc) return null;
  const base = `/staff/till/tips?location_id=${encodeURIComponent(loc)}`;
  const v = visible(ans);
  if (!v || !v.opened_at) return base;
  let q = `${base}&from_ms=${v.opened_at}`;
  if (v.kind === CLOSED && v.closed_at) q += `&to_ms=${v.closed_at}`;
  return q;
}

/// Who took how much, per currency, as the server folded it. No shares, no
/// pool: the venue's policy is not this screen's. `res` is the route's answer.
export function renderTips(res, t, locale) {
  const rows = (res && Array.isArray(res.tips) ? res.tips : []).filter(r => r && r.amount > 0);
  const names = (res && res.names) || {};
  const head = `<h3>${esc(t('tipsTitle'))}</h3>${ui.para(t(res && res.day ? 'tipsHintDay' : 'tipsHint'), { hint: true })}`;
  if (!rows.length) return `${head}${ui.emptyState({ icon: 'coins', title: t('tipsNone') })}`;
  const body = rows.map(r => `<tr><th scope="row">${esc(names[r.by] || r.by || '-')}</th><td>${ui.amount(money(r.amount, r.currency, locale))}</td></tr>`).join('');
  return `${head}<table class="z tips"><tbody>${body}</tbody></table>`;
}
