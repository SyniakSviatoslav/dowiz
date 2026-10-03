// THE ORDER LIST, THE PURE PART (W-STOCK P5): the text a supplier receives,
// in the supplier's language, and the phone's Share sheet with copying as the
// fallback. No DOM: `order-list-logic.test.mjs`.
//
// ASCII QUOTES ONLY in this file.

/// A quantity as a supplier reads it: 2000 g is "2 kg", 500 ml "500 ml", 12 pieces "12".
export function qtyText(q, unit){
  if (unit === 'g' || unit === 'ml') {
    const big = unit === 'g' ? 'kg' : 'l';
    return q >= 1000 ? `${String(q / 1000).replace('.', ',')} ${big}` : `${q} ${unit}`;
  }
  return String(q);
}

/// The lines to send: every line with a quantity above zero, as the order
/// note stores them ({item, qty}).
export function toSend(lines){
  return (lines || []).filter(l => Number.isInteger(l.qty) && l.qty > 0).map(l => ({ item: l.id, qty: l.qty }));
}

/// The message. `w(key)` answers a word in the SUPPLIER's language.
export function orderText({ venue, supplier, lines }, w){
  const rows = (lines || []).filter(l => l.qty > 0).map(l => {
    const packs = l.pack?.qty > 0 && l.qty % l.pack.qty === 0 ? ` (${l.qty / l.pack.qty} x ${qtyText(l.pack.qty, l.unit)})` : '';
    return `- ${l.name}: ${qtyText(l.qty, l.unit)}${packs}`;
  });
  return [`${w('ot_hello')}${supplier ? ' ' + supplier : ''}!`, `${w('ot_from')} ${venue || ''}:`.replace(' :', ':'), ...rows, w('ot_thanks')].join('\n');
}

/// The phone's own Share sheet; where there is none (a desktop), the
/// clipboard. Answers 'shared', 'copied', 'cancelled' or 'failed'.
export async function shareOrCopy(text, nav){
  if (nav?.share) {
    try { await nav.share({ text }); return 'shared'; } catch (e) { if (e?.name === 'AbortError') return 'cancelled'; }
  }
  try { await nav?.clipboard?.writeText(text); return nav?.clipboard ? 'copied' : 'failed'; } catch { return 'failed'; }
}
