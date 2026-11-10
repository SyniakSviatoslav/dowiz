// A TYPED NAME BECOMES A POSITION, THE RULES, PURE: no DOM, no clock, no
// network (`supply-resolve.test.mjs`).
//
// A supply's identity is its id: recipes, lots and write-offs key on it, and
// the name is a label. So a name somebody types is RESOLVED before anything
// is written:
//
//   match   the fold of what was typed equals one supply's name or id
//   near    the same word with another ending, a typo, another word order:
//           a QUESTION the person answers, never a silent merge
//   none    nothing like it: a new position
//
// THE FOLD: lower case, accents off, the Ukrainian and Russian spellings of
// one word together (ё є э -> е; і ї ы й -> и; ґ -> г; ь ъ and apostrophes
// dropped), letters and digits split, everything else one space. The Worker
// decides "this name exists" by the same fold (supplies/quick.rs), and both
// read `supply-resolve-cases.json`, so two phones cannot disagree.
//
// ASCII QUOTES ONLY in this file.

const MAP = {
  'ё': 'е', 'є': 'е', 'э': 'е',
  'і': 'и', 'ї': 'и', 'ы': 'и', 'й': 'и',
  'ґ': 'г',
};
const LATIN = [['a', 'àáâãäåāăą'], ['c', 'çćč'], ['d', 'ďđ'],
  ['e', 'èéêëēėęě'], ['g', 'ğ'], ['i', 'ìíîïīı'], ['l', 'ł'],
  ['n', 'ñńň'], ['o', 'òóôõöøōő'], ['r', 'ř'], ['s', 'śşš'],
  ['t', 'ţť'], ['u', 'ùúûüūůű'], ['y', 'ýÿ'], ['z', 'źżž']];
for (const [to, from] of LATIN) for (const c of from) MAP[c] = to;
/// Soft and hard signs, and every apostrophe a keyboard offers.
const DROP = new Set(['ь', 'ъ', '\'', 'ʼ', '’', '‘', '`', '´']);
const LETTER = /\p{Alphabetic}/u;
const VOWELS = new Set([...'aeiouyаеиоуюя']);

/// The form names are compared in. Tokens are runs of letters or of digits,
/// one space between them.
export function fold(s){
  let out = '', prev = 0;
  for (const ch of String(s ?? '').toLowerCase()) {
    const cp = ch.codePointAt(0);
    if ((cp >= 0x300 && cp <= 0x36f) || DROP.has(ch)) continue;
    const c = MAP[ch] || ch;
    const kind = c >= '0' && c <= '9' ? 2 : LETTER.test(c) ? 1 : 0;
    if (kind && out && prev !== kind) out += ' ';
    if (kind) out += c;
    prev = kind;
  }
  return out;
}

const isNum = w => w[0] >= '0' && w[0] <= '9';

function prefixLen(a, b){
  const n = Math.min(a.length, b.length);
  let i = 0;
  while (i < n && a[i] === b[i]) i++;
  return i;
}

/// At most one insertion, deletion or substitution apart. Linear.
function oneEdit(a, b){
  if (Math.abs(a.length - b.length) > 1) return false;
  const p = prefixLen(a, b);
  const [s, l] = a.length <= b.length ? [a, b] : [b, a];
  return s.length === l.length ? s.slice(p + 1) === l.slice(p + 1) : s.slice(p) === l.slice(p + 1);
}

/// Two folded words that are one word: the same, another ending, one vowel
/// swapped, or one typo in a long word. Short words differing in a consonant
/// are different things (rice and cheese are an anagram apart in Ukrainian).
export function nearWord(a, b){
  if (a === b) return true;
  const p = prefixLen(a, b), ra = a.length - p, rb = b.length - p;
  if (p >= 3 && ra <= 1 && rb <= 1) return true;
  if (p >= 4 && ra <= 3 && rb <= 3) return true;
  if (a.length === b.length && a.length >= 3 && a.slice(p + 1) === b.slice(p + 1) && VOWELS.has(a[p]) && VOWELS.has(b[p])) return true;
  return a.length >= 5 && b.length >= 5 && oneEdit(a, b);
}

const parts = f => {
  const words = [], nums = [];
  for (const w of f ? f.split(' ') : []) (isNum(w) ? nums : words).push(w);
  return { words, nums };
};

/// Two names that may be one position. A differing number is never near
/// (50 ml and 100 ml cups); a name with no number may be the one with it.
/// Every word of the shorter name must be a word of the longer, in any order.
function nearName(a, b){
  if (a.nums.length && b.nums.length && a.nums.join(' ') !== b.nums.join(' ')) return false;
  const [s, l] = a.words.length <= b.words.length ? [a.words, b.words] : [b.words, a.words];
  if (!s.length) return false;
  const used = new Array(l.length).fill(false);
  let real = false;
  for (const w of s) {
    let hit = -1;
    for (let i = 0; i < l.length && hit < 0; i++) if (!used[i] && nearWord(w, l[i])) hit = i;
    if (hit < 0) return false;
    used[hit] = true;
    if (Math.min(w.length, l[hit].length) >= 3) real = true;
  }
  return real;
}

const CACHE = new WeakMap();
/// One supply as the rules read it; folded once per object and name.
function entry(s){
  const c = CACHE.get(s);
  if (c && c.name === s.name && c.category === s.category) return c;
  const f = fold(s.name), e = { name: s.name, category: s.category, f, idf: fold(s.id), hay: fold(`${s.name || ''} ${s.category || ''}`), ...parts(f) };
  CACHE.set(s, e);
  return e;
}
const live = supplies => (supplies || []).filter(s => s && s.active !== false);

/// How many candidates a question or a suggestion list shows.
export const NEAR_MAX = 4;

/// What a typed name is among `supplies`: `{match}`, `{near: [..]}` or
/// `{none: true}` (with `empty` when nothing was typed). Retired supplies are
/// not candidates. Two supplies with the same fold are a question, not a match.
export function resolve(typed, supplies){
  const f = fold(typed);
  if (!f) return { none: true, empty: true };
  const all = live(supplies);
  const exact = all.filter(s => { const e = entry(s); return e.f === f || e.idf === f; });
  if (exact.length === 1) return { match: exact[0] };
  if (exact.length > 1) return { near: exact.slice(0, NEAR_MAX) };
  const mine = parts(f);
  const near = all.filter(s => nearName(mine, entry(s)));
  return near.length ? { near: near.slice(0, NEAR_MAX) } : { none: true };
}

/// A search predicate for `q`: the supply's name or category contains the
/// fold of what was typed, or its name is near it. Fold `q` once, test many.
export function finder(q){
  const f = fold(q);
  if (!f) return () => true;
  const mine = parts(f);
  return s => { const e = entry(s); return e.hay.includes(f) || e.idf.includes(f) || nearName(mine, e); };
}

/// The shelf's search: `finder`, or the supply's code or barcode.
export function stockSearch(q){
  const found = finder(q), f = fold(q);
  return s => found(s) || fold(`${s.code || ''} ${s.barcode || ''}`).includes(f);
}

/// The suggestions under a typed name: those that start with it, then those
/// that contain it, then the near ones.
export function suggest(typed, supplies, max = NEAR_MAX){
  const f = fold(typed);
  if (!f) return [];
  const has = finder(typed);
  const rank = s => { const e = entry(s); return e.f === f ? 0 : e.f.startsWith(f) ? 1 : e.hay.includes(f) ? 2 : 3; };
  return live(supplies).filter(has).map((s, i) => [rank(s), i, s]).sort((a, b) => a[0] - b[0] || a[1] - b[1]).slice(0, max).map(x => x[2]);
}

/// A typed category under the spelling the venue already uses: the same fold
/// is the same group ("овочи" is saved as "Овочі"). A new one is kept as typed.
export function canonCategory(typed, known){
  const raw = String(typed ?? '').trim(), f = fold(raw);
  if (!f) return '';
  return (known || []).find(c => fold(c) === f) || raw;
}

/// The venue's categories, each once, in first-seen order.
export function categoriesOf(supplies){
  const seen = new Set(), out = [];
  for (const s of supplies || []) {
    const c = String(s?.category || '').trim(), f = fold(c);
    if (f && !seen.has(f)) { seen.add(f); out.push(c); }
  }
  return out;
}
