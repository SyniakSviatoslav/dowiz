// A QUANTITY ON THE WIRE, ONE ADAPTER (lane W-TKUI, 2026-10-12, operator TZ 1;
// the format is /root/lanes/CONTRACT-QTY.md section 1). The editor types a
// decimal ("0,5", "40.5", "1,5 kg"); the Worker parses the TEXT into integer
// milli-units (thousandths of the supply's base unit), never through f64.
// So does this file: a quantity is held as an integer `milli` in JS, shown as
// a short decimal, sent as a decimal STRING (or the integer when whole).
// Every arithmetic the editor previews (cost, grams) takes `milli`, so no
// `Number(qty) * x` drifts. No DOM, no network (`qty-adapter.test.mjs`).
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

export const MILLI = 1000;
const SCALE = { g: 1, ml: 1, kg: 1000, l: 1000 };

// TZ 2026-10-16 ("enter everything as is, no ingredient lost"): the kitchen types
// in its own words. Unit words of every console language map to the four the
// parsers read; a piece word is dropped (a piece supply has no other unit).
const UNIT_WORDS = [
  [/^(?:г|гр|грам|грама|грамів|граммов|грамм|грамма|граммы|gr|gram|grams|gramm|g)\.?$/, 'g'],
  [/^(?:кг|кіло|кило|кілограм|кілограмів|килограмм|kilo|kilos|kgs|kg)\.?$/, 'kg'],
  [/^(?:мл|ml|millilitre|milliliter|millilitres|milliliters)\.?$/, 'ml'],
  [/^(?:л|літр|літра|літрів|литр|литра|литров|litre|liter|litres|liters|lt|l)\.?$/, 'l'],
  [/^(?:шт|штук|штуки|штука|pcs|pc|piece|pieces|unit|units|copë|cope)\.?$/, ''],
];
const VULGAR = { '½': '1/2', '¼': '1/4', '¾': '3/4', '⅛': '1/8', '⅜': '3/8', '⅝': '5/8', '⅞': '7/8', '⅕': '1/5', '⅖': '2/5', '⅗': '3/5', '⅘': '4/5', '⅓': '1/3', '⅔': '2/3' };

/// Typed text in the kitchen's words -> the canonical form the parsers read:
/// "0,5 г" -> "0,5 g", "1,5кг" -> "1,5 kg", "1 1/2" -> "1.5", "½ шт" -> "0.5".
/// A fraction becomes a decimal only when it is exact in thousandths (1/3 stays
/// as typed and is refused by name, never rounded). Anything unknown is left as
/// typed, so the parser's refusal still names it.
export function canonQty(raw){
  let s = String(raw ?? '').trim().toLowerCase().replace(/\s+/g, ' ');
  if (!s) return s;
  s = s.replace(/[½¼¾⅛⅜⅝⅞⅕⅖⅗⅘⅓⅔]/g, c => ` ${VULGAR[c]}`).trim().replace(/\s+/g, ' ');
  // Split a glued unit off the number: "0,5г" -> "0,5 г".
  s = s.replace(/^(-?[\d.,/ ]*\d)\s*([^\d\s.,/].*)$/u, '$1 $2');
  const m = /^(.*?\d)(?: (\D+))?$/u.exec(s);
  if (!m) return s;
  let [, num, word = ''] = m;
  word = word.trim();
  if (word) {
    const hit = UNIT_WORDS.find(([re]) => re.test(word));
    if (!hit) return s;
    word = hit[1];
  }
  const frac = /^(?:(\d+) )?(\d+)\/(\d+)$/.exec(num.trim());
  if (frac) {
    const [, w = '0', a, b] = frac, den = BigInt(b);
    if (den === 0n) return s;
    const thousandths = (BigInt(w) * den + BigInt(a)) * 1000n;
    if (thousandths % den !== 0n) return s; // not exact in thousandths: refused by name
    const t = thousandths / den;
    num = `${t / 1000n}.${String(t % 1000n).padStart(3, '0')}`.replace(/\.?0+$/, '');
  }
  return word ? `${num} ${word}` : num;
}

/// Typed text -> integer milli base units, or null (empty / not a number /
/// more than 3 decimals / a kg|l word on a piece supply / negative).
/// `unit` is the SUPPLY's base unit: 'g' | 'ml' | 'unit'.
export function parseMilli(raw, unit = 'g'){
  const s = canonQty(raw);
  if (!s) return null;
  const m = /^(\d+(?:[.,]\d+)?|[.,]\d+)\s*(kg|g|ml|l)?$/.exec(s.replace(/(\d) (?=\d{3}\b)/g, '$1'));
  if (!m) return null;
  if (m[2] && unit === 'unit') return null;
  const scale = m[2] ? SCALE[m[2]] : 1;
  const [whole = '', frac = ''] = m[1].replace(',', '.').split('.');
  if (frac.length > 3 && /[1-9]/.test(frac.slice(3))) return null;
  const f3 = (frac + '000').slice(0, 3);
  const milli = BigInt(whole || '0') * 1000n * BigInt(scale) + BigInt(f3) * BigInt(scale);
  const n = Number(milli);
  return Number.isSafeInteger(n) ? n : null;
}

/// Milli -> the number the hub answers as `qty` (a decimal with <=3 places).
export const fromMilli = milli => milli / MILLI;
/// The hub's `qty` (40 or 40.5) or `qtyMilli` -> milli, exactly (via the text).
export const toMilli = (qty, qtyMilli = null) => (qtyMilli != null ? Math.round(Number(qtyMilli)) : parseMilli(String(qty ?? '')));

/// Milli as the short decimal the owner reads: 40500 -> "40.5", 40000 -> "40", 500 -> "0.5".
export function showMilli(milli){
  if (milli == null || !Number.isFinite(milli)) return '';
  const neg = milli < 0 ? '-' : '', a = Math.abs(Math.round(milli));
  const frac = String(a % MILLI).padStart(3, '0').replace(/0+$/, '');
  return `${neg}${Math.floor(a / MILLI)}${frac ? '.' + frac : ''}`;
}

/// What goes on the wire: the integer when whole, else the decimal as a string.
export const wireQty = milli => (milli % MILLI === 0 ? milli / MILLI : showMilli(milli));

/// A step of `stepMilli`, never below zero.
export const stepMilli = (milli, by) => Math.max(0, milli + by);
