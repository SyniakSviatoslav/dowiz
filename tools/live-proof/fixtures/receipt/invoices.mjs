// SELF-MADE INVOICES (W-OCR, 2026-10-04): the data behind the fixture images.
// Every supplier, NIPT and number is INVENTED. `make.mjs` renders each one to
// HTML and a PNG (headless Chromium), `measure.mjs` reads the PNGs back with
// the vendored tesseract.js and scores the parse against `want`.
// These are SYNTHETIC images: clean type on white. Real phone photos of real
// Durrës invoices are UNMEASURED (operator declined to supply them).

/// The venue's shelf the lines are matched against.
export const SUPPLIES = [
  { id: 'salmon', name: 'Salmon', unit: 'g' }, { id: 'tuna', name: 'Tuna', unit: 'g' }, { id: 'shrimp', name: 'Shrimp', unit: 'g' },
  { id: 'nori', name: 'Nori sheets', unit: 'unit' }, { id: 'rice', name: 'Sushi rice', unit: 'unit' }, { id: 'avocado', name: 'Avocado', unit: 'unit' },
  { id: 'cucumber', name: 'Cucumber', unit: 'g' }, { id: 'cream-cheese', name: 'Cream cheese', unit: 'unit' }, { id: 'ginger', name: 'Ginger (pickled)', unit: 'g' },
  { id: 'lemon', name: 'Lemon', unit: 'unit' },
];

const peshku = { supplier: 'Peshku i Detit sh.p.k.', card: 'peshku-i-detit', nipt: 'K12345678A', lang: 'sq' };
const L = (text, unit, qty, price, total, item) => ({ text, unit, qty, price, total, item });

export const INVOICES = [
  { id: 'inv1-peshku-sq', ...peshku, doc: '118/2026', date: '02.10.2026', fmt: 'sq', lines: [
    L('Salmon fileto Norvegjeze', 'kg', '2,500', 1800, 4500, 'salmon'),
    L('Ton i freskët', 'kg', '1,2', 3200, 3840, 'tuna'),
    L('Karkaleca deti', 'kg', '1', 2400, 2400, 'shrimp'),
    L('Fletë nori', 'copë', '20', 150, 3000, 'nori'),
  ] },
  { id: 'inv2-peshku-sq', ...peshku, doc: '131/2026', date: '09.10.2026', fmt: 'sq', lines: [
    L('Salmon fileto Norvegjeze', 'kg', '3', 1800, 5400, 'salmon'),
    L('Ton i freskët', 'kg', '0,8', 3250, 2600, 'tuna'),
    L('Karkaleca deti', 'kg', '2', 2400, 4800, 'shrimp'),
    L('Fletë nori', 'copë', '40', 150, 6000, 'nori'),
    L('Xhenxhefil turshi', 'kg', '1', 900, 900, 'ginger'),
  ] },
  { id: 'inv3-market-en', supplier: 'Fresh Market Ltd', card: 'fresh-market', nipt: 'M23456789C', lang: 'en', doc: 'INV-2041', date: '2026-10-03', fmt: 'en', lines: [
    L('Avocado Hass', 'pcs', '24', 120, 2880, 'avocado'),
    L('Cucumber', 'kg', '5.5', 90, 495, 'cucumber'),
    L('Cream cheese 1,5 kg', 'pcs', '4', 1450, 5800, 'cream-cheese'),
    L('Sushi rice 10 kg bag', 'pcs', '3', 2100, 6300, 'rice'),
  ] },
  { id: 'inv4-perime-sq', supplier: 'Perime Durrësi', card: 'perime-durresi', nipt: 'L34567890D', lang: 'sq', doc: '77', date: '03.10.2026', fmt: 'sqL', lines: [
    L('Avokado', 'copë', '12', 130, 1560, 'avocado'),
    L('Kastravec', 'kg', '3', 80, 240, 'cucumber'),
    L('Limon', 'copë', '30', 25, 750, 'lemon'),
    L('Xhenxhefil', 'kg', '0,5', 900, 450, 'ginger'),
  ] },
];
// The fifth image: invoice 1 photographed badly -- turned 2 degrees and blurred.
export const SKEWED = { id: 'inv5-peshku-skewed', of: 'inv1-peshku-sq', rotate: 2, blur: 0.6 };

/// Money as each invoice prints it.
export function money(n, fmt){
  const g = String(n).replace(/\B(?=(\d{3})+(?!\d))/g, fmt === 'en' ? ',' : '.');
  return fmt === 'en' ? `${g}.00` : fmt === 'sqL' ? `${g} L` : g;
}

/// What the parse should produce for an invoice: qty as a number.
export const want = inv => inv.lines.map(l => ({ text: l.text, item: l.item, qty: Number(l.qty.replace(',', '.')), unit_price: l.price, total: l.total }));
