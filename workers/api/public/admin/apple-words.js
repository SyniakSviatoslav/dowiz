// THE WORDS NEXT TO THE CONTROLS (lane W-APPLE, 2026-09-26,
// docs/design/HUB-APPLE-2026-09-26.md section 2). PURE: no imports, so node
// tests the tables themselves (apple-words.test.mjs); admin/apple-i18n.js
// merges WORDS into the console's dictionary at import. This file imports
// only the language files beside it, so node and the browser load the same.
//
// Three tables:
//   WORDS     sq/en/uk (one file per language, apple-words-<lang>.js) -- purpose lines (ap_h_*), examples (ap_ex_*), footers
//             (ap_f_*), consequences of a destructive act (ap_why_*), labels
//             that were missing (ap_l_*). A language the console gains later
//             is one more key here; the merge fills its gaps from English.
//   EXAMPLES  field id or label key -> the example's word key. A field asks
//             by its id first, its label key second (parts.js field()).
//   FOOTERS   the same, for the one-line explanation under the field.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console. Albanian uses the guillemets « » inside a string, which are
// not quotes to JavaScript.

import { SQ } from './apple-words-sq.js';
import { EN } from './apple-words-en.js';
import { UK } from './apple-words-uk.js';

/// The languages, by code. A fourth is one more import and one more key.
export const WORDS = { sq: SQ, en: EN, uk: UK };

/// Field id or label key -> the example's word key. Ids first (a `name` is a
/// courier on one sheet and a dish on another), keys as the fallback.
export const EXAMPLES = {
  'i-name': 'ap_ex_personName', 'i-phone': 'ap_ex_phone', 'i-email': 'ap_ex_email', 'rn-name': 'ap_ex_personName', 'rn-phone': 'ap_ex_phone',
  'v-name': 'ap_ex_venueName', 'v-phone': 'ap_ex_phone', 'v-addr': 'ap_ex_address', 'nd-name': 'ap_ex_dishName', 'nd-price': 'ap_ex_price', 'nd-desc': 'ap_ex_description',
  'nc-name': 'ap_ex_category', 's-name': 'ap_ex_supplyName', 's-cat': 'ap_ex_category', 's-kcal': 'ap_ex_kcal', 's-prot': 'ap_ex_protein', 's-fat': 'ap_ex_fat', 's-carb': 'ap_ex_carbs',
  's-cost': 'ap_ex_cost', 's-low': 'ap_ex_minLevel', 's-shelf': 'ap_ex_shelfDays', 's-wpu': 'ap_ex_weightPerUnit', 's-clean': 'ap_ex_pct', 's-cook': 'ap_ex_pct',
  'm-qty': 'ap_ex_qty', 'm-price': 'ap_ex_cost', 'm-sup': 'ap_ex_supplier', 'm-doc': 'ap_ex_doc', 'm-lot': 'ap_ex_lot', 'p-in': 'ap_ex_qtyIn', 'p-out': 'ap_ex_qtyOut',
  'd-sup': 'ap_ex_supplier', 'd-doc': 'ap_ex_doc', 'd-price': 'ap_ex_price', 'd-cook': 'ap_ex_cookingMin', 'd-note': 'ap_ex_offNote', 'd-ings': 'ap_ex_ingredients',
  'd-kcal': 'ap_ex_kcal', 'd-prot': 'ap_ex_protein', 'd-fat': 'ap_ex_fat', 'd-carb': 'ap_ex_carbs', 'd-weight': 'ap_ex_weight', 'd-size': 'ap_ex_sizeCm',
  'pr-code': 'ap_ex_promo', 'pr-value': 'ap_ex_discount', 'pr-min': 'ap_ex_minOrder', 'pr-max': 'ap_ex_maxUses',
  'd-fee': 'ap_ex_deliveryFee', 'd-free': 'ap_ex_freeOver', 'd-min': 'ap_ex_minOrder', 'ig-user': 'ap_ex_igUser',
  'cd-table': 'ap_ex_table', 'cd-note': 'ap_ex_note', 'rv-reason': 'ap_ex_reason', 'cd-lreason': 'ap_ex_reason', 'rf-note': 'ap_ex_refundNote', 'rn-party': 'ap_ex_party',
  'cp-name': 'ap_ex_cpName', 'cp-text': 'ap_ex_cpText', 'cp-days': 'ap_ex_days', 'cp-promo': 'ap_ex_promo', 'fp-zname': 'ap_ex_zone', 'fp-seats': 'ap_ex_seats',
  'ex-thr': 'ap_ex_threshold', 'ex-late': 'ap_ex_minutes', 'k-label': 'ap_ex_keyLabel', 'ai-model': 'ap_ex_model', 'st-n': 'ap_ex_stampN', 'st-reward': 'ap_ex_stampReward',
  'eb-user': 'ap_ex_ebUser', 'eb-pos': 'ap_ex_pos', 'wa-phone': 'ap_ex_waPhone', 'wa-to': 'ap_ex_waTo', 'as-q': 'ap_ex_ask', 'pr-name': 'ap_ex_printer', 'cl-bucket': 'ap_ex_bucket',
  'kdsWhy': 'ap_ex_why', 'vReason': 'ap_ex_why', 'cf-reason': 'ap_ex_why', 'e': 'ap_ex_loginEmail', 's-seal': 'ap_ex_seal', 'ag-total': 'ap_ex_price', 'ag-disc': 'ap_ex_discount', 'ag-x': 'ap_ex_aggNumber',
  inv_qty: 'ap_ex_qty', inv_price: 'ap_ex_cost', inv_lot: 'ap_ex_lot', inv_supplier: 'ap_ex_supplier', inv_doc: 'ap_ex_doc', phone: 'ap_ex_phone', email: 'ap_ex_email',
  name: 'ap_ex_personName', price: 'ap_ex_price', category: 'ap_ex_category', kcal: 'ap_ex_kcal', reason: 'ap_ex_why', search: 'ap_ex_search',
};

/// Field id or label key -> the footer's word key.
export const FOOTERS = {
  'nd-price': 'ap_f_price', 'd-price': 'ap_f_price', 'nd-name': 'ap_f_dishNameNew', 's-low': 'ap_f_minLevel', 's-cost': 'ap_f_cost', 's-shelf': 'ap_f_shelfDays',
  's-clean': 'ap_f_cleanPct', 's-cook': 'ap_f_cookPct', 's-wpu': 'ap_f_weightPerUnit', 's-cat': 'ap_f_category', 's-unit': 'ap_f_unit',
  'd-cook': 'ap_f_cookingMin', 'd-weight': 'ap_f_weight', 'd-note': 'ap_f_offNote', 'd-fee': 'ap_f_deliveryFee', 'd-free': 'ap_f_freeOver', 'd-min': 'ap_f_minOrder',
  'pr-code': 'ap_f_promo', 'pr-value': 'ap_f_discount', 'pr-min': 'ap_f_promoMin', 'pr-max': 'ap_f_maxUses', 'pr-from': 'ap_f_from', 'pr-until': 'ap_f_until',
  'i-phone': 'ap_f_courierPhone', 'i-email': 'ap_f_staffEmail', 'i-role': 'ap_f_role', 's-role': 'ap_f_role', 'v-phone': 'ap_f_venuePhone', 'v-addr': 'ap_f_venueAddress',
  'so-ch': 'ap_f_tgChannel', 'ig-user': 'ap_f_igUser', 'cd-table': 'ap_f_table', 'cd-bd': 'ap_f_birthday', 'rv-reason': 'ap_f_reason',
  'm-price': 'ap_f_qtyPrice', 'm-sup': 'ap_f_supplier', 'm-doc': 'ap_f_doc', 'm-lot': 'ap_f_lot', 'm-exp': 'ap_f_expiry', 'd-sup': 'ap_f_supplier', 'd-doc': 'ap_f_doc',
  'rn-party': 'ap_f_party', 'rn-time': 'ap_f_rsTime', 'rn-table': 'ap_f_rsTable', 'cp-days': 'ap_f_days', 'ex-thr': 'ap_f_threshold', 'ex-late': 'ap_f_minutes',
  'k-label': 'ap_f_keyLabel', 'st-n': 'ap_f_stampN', 'st-reward': 'ap_f_stampReward', 'ai-model': 'ap_f_model', 's-seal': 'ap_f_seal', 'fp-zname': 'ap_f_zone', 'eb-pos': 'ap_f_pos',
  'wa-phone': 'ap_f_wa',
  // switches, by id
  'c-active': 'ap_f_activeC', 's-active': 'ap_f_activeS', 'd-avail': 'ap_f_onSale', 'v-pickup': 'ap_f_pickup', 'pauseD': 'ap_f_paused', 'so-on': 'ap_f_autopost', 'st-on': 'ap_f_stampOn',
};

/// Merge these words into a console dictionary `T` ({ lang: { key: word } }),
/// language by language. A language `T` has that WORDS lacks reads English,
/// so a key never falls through `t()` as itself. Returns `T`.
export function merge(T, words = WORDS){
  const en = words.en || {};
  for (const lang of new Set([...Object.keys(T), ...Object.keys(words)])) {
    T[lang] = Object.assign(T[lang] || {}, en, words[lang] || {});
  }
  return T;
}

/// The word key of the example (or footer) for a field: by id, then by key.
export const lookup = (table, id, key) => (id && table[id]) || (key && table[key]) || null;
