// AN E-INVOICE FILE -> A DRAFT RECEIPT (W-OCR, research 2026-10-03 row P11,
// the FALLBACK half), PURE: no DOM, no DOMParser, no network, so it runs in
// node (`einvoice-logic.test.mjs`).
//
// Albania's B2B e-invoices are UBL 2.1 `Invoice` documents exchanged through
// the CIS (fiscalisation law, in force 2021-07-01). The buyer receives the XML
// (and often a PDF) by email or from the e-Fatura portal; the owner uploads
// that file in the console and THIS parse is exact -- no OCR. There is NO live
// CIS call and no change to the ebills client: a buyer-side API was not found
// (research P11 spike), so the file is the interface.
//
// What is read, by UBL 2.1 element (OASIS UBL 2.1, Invoice schema
// UBL-Invoice-2.1.xsd, http://docs.oasis-open.org/ubl/os-UBL-2.1/UBL-2.1.html,
// accessed 2026-10-04):
//   cbc:ID                         the invoice number           -> doc
//   cbc:IssueDate                  its date                     -> date
//   cbc:DocumentCurrencyCode       must be ALL (lek)            -> refused otherwise
//   cac:AccountingSupplierParty/cac:Party
//     cac:PartyTaxScheme/cbc:CompanyID | cac:PartyIdentification/cbc:ID  -> nipt
//     cac:PartyName/cbc:Name | cac:PartyLegalEntity/cbc:RegistrationName -> supplier
//   cac:InvoiceLine (each)
//     cbc:InvoicedQuantity @unitCode (UN/ECE Rec 20)  -> qty, unit
//     cbc:LineExtensionAmount                          -> total (whole lek)
//     cac:Item/cbc:Name                                -> text
//     cac:Price/cbc:PriceAmount (per cbc:BaseQuantity) -> unit_price
//
// The lines come out in the SAME shape as `receipt-photo-logic.js` parseLine,
// so the photo sheet's table, matching and confirmation are reused unchanged.

import { lek, aliasKey } from './receipt-photo-logic.js';

/// UN/ECE Recommendation 20 unit codes -> the line units the console scales.
export const UNIT_CODES = { KGM: 'kg', GRM: 'g', LTR: 'l', MLT: 'ml', H87: 'copë', C62: 'copë', XPP: 'copë', EA: 'copë', PCE: 'copë', XBX: 'copë', XPK: 'copë' };

const ENT = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'" };
const unescape = s => s.replace(/&(#x[0-9a-f]+|#\d+|\w+);/gi, (m, e) => {
  if (e[0] === '#') { const c = e[1] === 'x' || e[1] === 'X' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10); return Number.isFinite(c) ? String.fromCodePoint(c) : m; }
  return ENT[e] ?? m;
});

/// A small, strict XML reader: elements (namespace prefixes dropped), attributes,
/// text. Comments, processing instructions and a DOCTYPE are skipped; a DOCTYPE
/// with an internal subset is REFUSED (no entity expansion, ever). Throws on a
/// tag that does not close.
export function xml(src){
  const s = String(src ?? '');
  if (/<!DOCTYPE[^>]*\[/i.test(s)) throw new Error('ei_bad');
  const root = { name: '#root', attrs: {}, kids: [], text: '' };
  const stack = [root];
  const re = /<!--[\s\S]*?-->|<\?[\s\S]*?\?>|<!\[CDATA\[([\s\S]*?)\]\]>|<!DOCTYPE[^>]*>|<\/([\w:.-]+)\s*>|<([\w:.-]+)((?:\s+[\w:.-]+\s*=\s*(?:"[^"]*"|'[^']*'))*)\s*(\/?)>|([^<]+)/g;
  let m, at = 0;
  while ((m = re.exec(s))) {
    if (m.index !== at) throw new Error('ei_bad');
    at = re.lastIndex;
    const top = stack[stack.length - 1];
    if (m[1] != null) top.text += m[1];
    else if (m[2]) {
      const local = m[2].split(':').pop();
      if (top.name !== local || stack.length === 1) throw new Error('ei_bad');
      stack.pop();
    } else if (m[3]) {
      const el = { name: m[3].split(':').pop(), attrs: {}, kids: [], text: '' };
      for (const a of m[4].matchAll(/([\w:.-]+)\s*=\s*(?:"([^"]*)"|'([^']*)')/g)) el.attrs[a[1].split(':').pop()] = unescape(a[2] ?? a[3]);
      top.kids.push(el);
      if (!m[5]) stack.push(el);
    } else if (m[6] != null) top.text += unescape(m[6]);
  }
  if (at !== s.length || stack.length !== 1) throw new Error('ei_bad');
  return root;
}

const kid = (el, ...path) => path.reduce((e, n) => e?.kids.find(k => k.name === n), el);
const kids = (el, n) => (el?.kids || []).filter(k => k.name === n);
const txt = el => (el?.text || '').trim();

/// A UBL decimal ("4500.00", "2.500": always a dot, NEVER a thousands
/// separator, so not `num`, which reads "2.500" as 2500) as exact {v, d}.
const ubl = raw => {
  const s = String(raw || '').trim();
  const m = /^(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) return null;
  const f = (m[2] || '').replace(/0+$/, '');
  const v = Number(m[1] + f);
  return Number.isSafeInteger(v) ? { v, d: f.length } : null;
};

/// The UBL 2.1 Invoice -> { supplier, nipt, doc, date, lines } or throws Error('ei_*').
export function parseInvoice(src){
  const root = xml(src);
  const inv = root.kids.find(k => k.name === 'Invoice');
  if (!inv) throw new Error(root.kids.some(k => k.name === 'CreditNote') ? 'ei_credit' : 'ei_notInvoice');
  const cur = txt(kid(inv, 'DocumentCurrencyCode'));
  if (cur && cur.toUpperCase() !== 'ALL') throw new Error('ei_currency');
  const party = kid(inv, 'AccountingSupplierParty', 'Party');
  const nipt = (txt(kid(party, 'PartyTaxScheme', 'CompanyID')) || txt(kid(party, 'PartyIdentification', 'ID'))).toUpperCase().replace(/\s+/g, '');
  const supplier = txt(kid(party, 'PartyName', 'Name')) || txt(kid(party, 'PartyLegalEntity', 'RegistrationName'));
  const lines = [];
  for (const l of kids(inv, 'InvoiceLine')) {
    const q = kid(l, 'InvoicedQuantity');
    const qty = ubl(txt(q));
    const unit = UNIT_CODES[(q?.attrs.unitCode || '').toUpperCase()] || '';
    const total = lek(ubl(txt(kid(l, 'LineExtensionAmount'))));
    const text = txt(kid(l, 'Item', 'Name'));
    if (!qty || !total || !text) throw new Error('ei_line');
    const flags = [];
    const priceEl = kid(l, 'Price', 'PriceAmount');
    const price = priceEl ? lek(ubl(txt(priceEl))) : null;
    const base = ubl(txt(kid(l, 'Price', 'BaseQuantity')));
    // A price per BaseQuantity other than 1 is not a per-unit price; keep the total.
    const perOne = !base || (base.v === 1 && base.d === 0);
    if (total.rounded || price?.rounded) flags.push('rounded');
    if (!unit) flags.push('unit');
    lines.push({ text, key: aliasKey(text), qty, unit, unit_price: price && perOne ? price.lek : null, total: total.lek, flags });
  }
  if (!lines.length) throw new Error('ei_line');
  return { supplier, nipt, doc: txt(kid(inv, 'ID')), date: txt(kid(inv, 'IssueDate')), lines };
}

/// A PDF's text (bytes as latin-1) -> the UBL XML it embeds UNCOMPRESSED, or ''.
/// Compressed embedded streams are inflated by the sheet (DecompressionStream)
/// and fed back here; a PDF with no XML at all is "use the photo".
export function xmlInPdf(latin1){
  const s = String(latin1 ?? '');
  const a = s.search(/<(?:[\w-]+:)?Invoice[\s>]/);
  if (a < 0) return '';
  const head = s.lastIndexOf('<?xml', a);
  const start = head >= 0 && a - head < 4096 ? head : a;
  const end = s.search(/<\/(?:[\w-]+:)?Invoice\s*>/);
  if (end < 0) return '';
  const close = s.indexOf('>', end) + 1;
  // latin-1 -> UTF-8: the bytes were read one per char.
  const bytes = Uint8Array.from(s.slice(start, close), c => c.charCodeAt(0));
  return new TextDecoder('utf-8').decode(bytes);
}

/// Which supplier card an e-invoice belongs to: by NIPT (remembered by an
/// earlier import), else by name, else none -- the owner picks.
export function cardFor(inv, cards, aliases){
  const byNipt = Object.entries(aliases || {}).find(([, k]) => k.nipt && k.nipt === inv.nipt);
  if (byNipt && (cards || []).some(c => c.id === byNipt[0])) return { id: byNipt[0], by: 'nipt' };
  const n = String(inv.supplier || '').trim().toLowerCase();
  const byName = (cards || []).find(c => String(c.name || '').trim().toLowerCase() === n);
  return byName ? { id: byName.id, by: 'name' } : { id: '', by: '' };
}
