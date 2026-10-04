// PURE: the bag insert's form and words (W-QR). No DOM, no fetch -- tested in
// bag-logic.test.mjs. Money is INTEGER minor units of the venue's currency
// (lek has none): typed whole, sent whole, drawn by the caller's `money`
// (/lib/money.js through admin/core.js). Never `/ 100`, never Intl here.
//
// ASCII QUOTES ONLY in this file.

/// The offer kinds the hub knows (`services/loyalty/welcome.rs`).
export const KINDS = ['off', 'fixed', 'gift', 'stamps'];

/// A whole amount as typed, or null. "1 500" and "1500" are 1500; "15.5" is not money here.
export function whole(raw){
  const s = String(raw ?? '').replace(/[\s_]/g, '');
  return /^\d{1,9}$/.test(s) ? Number(s) : null;
}

/// The body `POST /api/owner/bag` takes. Fields the kind does not use are not sent
/// (the hub refuses an unknown field, and an unused one would only confuse a reader).
export function formBody({ kind, value, min, product, pct }){
  const k = KINDS.includes(kind) ? kind : 'off';
  const offer = { kind: k };
  if (k === 'fixed') { offer.value = whole(value) ?? 0; offer.min = whole(min) ?? 0; }
  if (k === 'gift') offer.product = String(product || '');
  return { offer, commission_pct: String(pct ?? '').trim() };
}

/// The bonus in words: "300 off from 2,000", "Edamame on the house", "a double stamp".
export function bonusText(offer, t, money, dishName = id => id){
  if (!offer || !offer.kind || offer.kind === 'off') return '';
  if (offer.kind === 'fixed') {
    const v = money(offer.value || 0);
    return offer.min ? t('bag_fixedMin').replace('{value}', v).replace('{min}', money(offer.min)) : t('bag_fixed').replace('{value}', v);
  }
  if (offer.kind === 'gift') return t('bag_giftOf').replace('{dish}', dishName(offer.product));
  if (offer.kind === 'stamps') return t('bag_stamps');
  return '';
}

/// The one line printed on the card: "Next time order direct - 300 off".
export const cardLine = (offer, t, money, dishName) => {
  const b = bonusText(offer, t, money, dishName);
  return b ? `${t('bag_line')} - ${b}` : t('bag_line');
};

/// A campaign as the hub accepts it, or '' (the input is cleaned as typed).
export const campaign = raw => String(raw || '').toLowerCase().replace(/[^a-z0-9-]/g, '').slice(0, 24);

/// The card's rows: [labelKey, value-or-null, noteKey-or-null]. Scans are never
/// counted (nothing is written before an order), and the saving is shown only
/// when the owner typed a percent -- the card invents nothing.
export function statRows(st, money){
  const s = st || {};
  return [
    ['bag_scans', null, 'bag_scansNone'],
    ['bag_orders', String(s.orders || 0), null],
    ['bag_guests', String(s.guests || 0), null],
    ['bag_repeat', String(s.repeat || 0), null],
    ['bag_saved', s.saved == null ? null : money(s.saved), s.saved == null ? 'bag_savedNone' : null],
  ];
}

/// The QR as an image the CSP allows; `#` must be `%23` in a data URL.
export const svgSrc = svg => 'data:image/svg+xml,' + encodeURIComponent(svg || '');
