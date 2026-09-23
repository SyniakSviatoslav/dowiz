// What the till screen SAYS about the drawer. PURE: an answer from
// `/api/staff/till/*` in, HTML out, so the blind count is provable in a test.
//
// A BLIND COUNT (`services/orders/room/till.rs`): the counter enters what the
// drawer holds without having seen what it should hold, or the count is a
// copy of the screen. The server already sends `expected` and `over_short`
// only on the close. This renderer does not trust that: it draws them ONLY
// for a `till.closed` answer, and drops them from any other -- a server that
// one day leaks them into a count answer still shows the counter nothing.
import { esc, money, TILL_CURRENCIES } from './logic.js';

export const CLOSED = 'till.closed';

/// The currencies a set of per-currency maps mentions, the drawer's two first.
function codesOf(...maps) {
  const seen = new Set(TILL_CURRENCIES);
  for (const m of maps) for (const k of Object.keys(m || {})) seen.add(k);
  return [...seen].filter(c => maps.some(m => m && c in m));
}

const cell = (m, c, locale) => (m && c in m ? money(m[c], c, locale) : '—');

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
  if (!v) return `<p class="muted">${esc(t('tillUnknown'))}</p>`;
  const when = v.opened_at ? ` · ${esc(t('openedAt'))} ${esc(timeOf(v.opened_at))}` : '';
  if (v.kind !== CLOSED) {
    const codes = codesOf(v.float, v.counted);
    const rows = codes.map(c => `<tr><th scope="row">${esc(c)}</th><td>${cell(v.float, c, locale)}</td>${v.counted ? `<td>${cell(v.counted, c, locale)}</td>` : ''}</tr>`).join('');
    return `<p class="state ok">${esc(t(v.open ? 'tillOpen' : 'tillClosedWord'))}${when}</p>
      <table class="z"><thead><tr><th></th><th>${esc(t('tillFloat'))}</th>${v.counted ? `<th>${esc(t('counted'))}</th>` : ''}</tr></thead><tbody>${rows}</tbody></table>
      ${v.counted ? `<p class="muted">${esc(t('blindNote'))}</p>` : ''}`;
  }
  const codes = codesOf(v.expected, v.counted, v.over_short);
  const rows = codes.map(c => {
    const os = v.over_short && c in v.over_short ? v.over_short[c] : null;
    const cls = os == null ? '' : os < 0 ? ' short' : os > 0 ? ' over' : ' even';
    return `<tr><th scope="row">${esc(c)}</th><td>${cell(v.expected, c, locale)}</td><td>${cell(v.counted, c, locale)}</td><td class="os${cls}">${cell(v.over_short, c, locale)}</td></tr>`;
  }).join('');
  return `<p class="state">${esc(t('tillClosedWord'))}${v.closed_at ? ` · ${esc(timeOf(v.closed_at))}` : ''}</p>
    <table class="z"><thead><tr><th></th><th>${esc(t('expected'))}</th><th>${esc(t('counted'))}</th><th>${esc(t('overShort'))}</th></tr></thead><tbody>${rows}</tbody></table>`;
}
