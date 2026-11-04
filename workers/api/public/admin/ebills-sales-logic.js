// The till tab's arithmetic (W-EBILLS-TAB), pure and tested: bar heights,
// the unmatched share as a percent, the search over the rows, the words for
// a line's match and a row's payment. The hub (`ebills/sales.rs`) did every
// sum; nothing here adds money up a second way.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

/// Bar heights, 0..100, of `series[i][key]` against the tallest.
export function heights(series, key = 'revenue'){
  const max = Math.max(0, ...series.map(x => Number(x?.[key]) || 0));
  return series.map(x => (max > 0 ? Math.round((100 * (Number(x?.[key]) || 0)) / max) : 0));
}

/// A share in parts per million as a percent with one decimal: 123456 -> '12.3%'.
export function pct(ppm){
  const n = Number(ppm);
  if (!Number.isFinite(n)) return '';
  const tenths = Math.round(n / 1000);
  return `${Math.trunc(tenths / 10)}.${Math.abs(tenths % 10)}%`;
}

/// Monday-first weekday names in the viewer's locale (the hub's weekday 0 is Monday).
export function weekdays(locale = 'en'){
  return [0, 1, 2, 3, 4, 5, 6].map(i => new Date(Date.UTC(2024, 0, 1 + i)).toLocaleDateString(locale, { weekday: 'short', timeZone: 'UTC' }));
}

/// How a line was matched: by the owner's tap, by its name, or not at all.
export const matchOf = it => (it?.product_id ? (it.matched_by === 'name' ? 'name' : 'map') : 'open');

/// The key of the word for the payment the till recorded.
export const payKey = p => ({ CASH: 'cash', CARD: 'card', CARD_ON_POS: 'card', POK_CARD: 'card' }[String(p || '').toUpperCase()] || null);

const fold = s => String(s ?? '').toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '');

/// The rows whose sale id, table, FIC or any line's name or code carry every term.
export function search(rows, query){
  const terms = fold(query).trim().split(/\s+/).filter(Boolean);
  if (!terms.length) return rows;
  return rows.filter(r => {
    const hay = fold([r.sale_id, r.inv_ord_num, r.table, r.fic, ...(r.items || []).flatMap(i => [i.name, i.code])].filter(x => x != null).join(' '));
    return terms.every(tm => hay.includes(tm));
  });
}

/// The rows of one local day (`YYYY-MM-DD`, the hub's `by_day[].day`), or all.
export function ofDay(rows, day, dayOf){
  return day ? rows.filter(r => dayOf(r.at_ms) === day) : rows;
}

/// A CSV of the rows: one line per till line, RFC 4180 quoting.
export function csv(rows){
  const q = v => { const s = String(v ?? ''); return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s; };
  const head = ['sale_id', 'inv_ord_num', 'at_ms', 'kind', 'table', 'payment', 'payment_status', 'status', 'code', 'name', 'product_id', 'matched_by', 'quantity', 'unit_price', 'total'];
  const lines = rows.flatMap(r => (r.items || []).map(i => [r.sale_id, r.inv_ord_num, r.at_ms, r.kind, r.table, r.payment, r.payment_status, r.status, i.code, i.name, i.product_id, i.matched_by, i.quantity, i.unit_price, r.total].map(q).join(',')));
  return [head.join(','), ...lines].join('\n');
}
