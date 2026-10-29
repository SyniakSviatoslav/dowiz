// THE PRICE ON THE RECEIPT, PROVED (LV-PCD-C, AC-LV-PCD-03; docs/design/SPEC-PCD-2026-10-27.md).
//
// The venue publishes, in its `manifest.json`, an RFC 9162 Merkle root over every dish's price
// (`hubdo/publish/root.rs`), signed with ML-DSA-65 when the operator installed the venue key. A
// receipt line carries a CERTIFICATE -- the dish id, the price, its leaf index, the leaf count, the
// root and the path of sibling hashes -- as a QR that opens /store/verify.html. This page climbs the
// path to the root IN THE BROWSER, with the wasm32 reader (`bw_verify`, `verify.wasm`, the same code
// as `bebop_store::merkle`), so a price check needs no network. When the network is there, it also
// compares the root with the venue's live manifest and says whether that root is signed.
//
// PURE where it can be: the producer (`pricesOf`, `certFor`) and the codecs take their inputs and
// return values; `verify.test.mjs` drives them against the gate fixtures. ASCII quotes only (rule 11).

const enc = new TextEncoder();

/// The menu's prices from a fragment: `[[id, price]]` in id byte order, a dish listed in two
/// categories once; `null` when one id carries two prices (no root then -- `root.rs prices`).
export function pricesOf(fragment) {
  const seen = new Map();
  for (const cat of (fragment && fragment.categories) || []) {
    for (const p of cat.products || []) {
      if (typeof p.id !== 'string' || !Number.isSafeInteger(p.price)) continue;
      if (seen.has(p.id) && seen.get(p.id) !== p.price) return null;
      seen.set(p.id, p.price);
    }
  }
  const cmp = (a, b) => { const x = enc.encode(a), y = enc.encode(b); for (let i = 0; i < Math.min(x.length, y.length); i++) if (x[i] !== y[i]) return x[i] - y[i]; return x.length - y.length; };
  return [...seen.entries()].sort((a, b) => cmp(a[0], b[0]));
}

const cat = (...parts) => { const n = parts.reduce((s, p) => s + p.length, 0); const o = new Uint8Array(n); let at = 0; for (const p of parts) { o.set(p, at); at += p.length; } return o; };
const u64 = (v) => { const b = new Uint8Array(8); new DataView(b.buffer).setBigUint64(0, BigInt(v), true); return b; };
const i64 = (v) => { const b = new Uint8Array(8); new DataView(b.buffer).setBigInt64(0, BigInt(v), true); return b; };
const field = (b) => cat(u64(b.length), b);
const sha = async (b) => new Uint8Array(await globalThis.crypto.subtle.digest('SHA-256', b));

export const leafHash = (id, price) => sha(cat(Uint8Array.of(0), field(enc.encode(id)), field(i64(price))));
const nodeHash = (l, r) => sha(cat(Uint8Array.of(1), l, r));
const split = (n) => { let k = 1; while (k * 2 < n) k *= 2; return k; };

/// MTH (RFC 9162 §2.1.1).
export async function mth(hs) {
  if (hs.length === 0) return sha(new Uint8Array(0));
  if (hs.length === 1) return hs[0];
  const k = split(hs.length);
  return nodeHash(await mth(hs.slice(0, k)), await mth(hs.slice(k)));
}

/// The bottom-up path of leaf m (RFC 9162 §2.1.3.1).
export async function path(hs, m) {
  if (hs.length <= 1) return [];
  const k = split(hs.length);
  return m < k ? [...await path(hs.slice(0, k), m), await mth(hs.slice(k))] : [...await path(hs.slice(k), m - k), await mth(hs.slice(0, k))];
}

/// The certificate frame: 'V' ‖ field(id) ‖ field(price) ‖ field(index) ‖ field(n) ‖ field(root) ‖ field(path).
export function encodeCert({ id, price, index, n, root, sibs }) {
  return cat(Uint8Array.of(86), field(enc.encode(id)), field(i64(price)), field(u64(index)), field(u64(n)), field(root), field(cat(...sibs)));
}

/// The certificate of `id` at `price` against the fragment's root; `null` when that dish is not on
/// the menu at that price (the receipt's price is then NOT the published one).
export async function certFor(fragment, id, price) {
  const rows = pricesOf(fragment);
  if (!rows) return null;
  const index = rows.findIndex((r) => r[0] === id);
  if (index < 0 || rows[index][1] !== price) return null;
  const hs = await Promise.all(rows.map(([i, p]) => leafHash(i, p)));
  return encodeCert({ id, price, index, n: hs.length, root: await mth(hs), sibs: await path(hs, index) });
}

export const hex = (b) => Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
export function b64url(b) { let s = ''; for (const x of b) s += String.fromCharCode(x); return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, ''); }
export function unb64url(s) { const t = atob(String(s).replace(/-/g, '+').replace(/_/g, '/')); return Uint8Array.from(t, (c) => c.charCodeAt(0)); }

/// The link a receipt's QR holds.
export const verifyLink = (origin, slug, cert) => `${origin}/store/verify.html#s=${encodeURIComponent(slug)}&c=${b64url(cert)}`;

/// The certificate's id, price and named root, read back for display (no checking here).
export function readCert(b) {
  const dv = new DataView(b.buffer, b.byteOffset, b.byteLength);
  let at = 1;
  const next = () => { const n = Number(dv.getBigUint64(at, true)); const s = b.subarray(at + 8, at + 8 + n); at += 8 + n; return s; };
  try {
    const id = new TextDecoder().decode(next());
    const price = Number(new DataView(next().slice().buffer).getBigInt64(0, true));
    next(); next();
    return { id, price, root: hex(next()) };
  } catch { return null; }
}

/// `bw_verify` over the module's exports: `{ status, root }` (0 ok, 50 malformed, 51 mismatch).
export function verifyWith(x, bytes) {
  const at = x.bw_alloc(Math.max(bytes.length, 1));
  new Uint8Array(x.memory.buffer, at, bytes.length).set(bytes);
  const out = x.bw_alloc(32);
  const status = x.bw_verify(at, bytes.length, out);
  const root = hex(new Uint8Array(x.memory.buffer, out, 32));
  x.bw_free(at, Math.max(bytes.length, 1));
  x.bw_free(out, 32);
  return { status, root };
}

/// The QR grid of `text` from `bw_qr`: `{ side, cells }` (1 = dark), or null.
export function qrWith(x, text) {
  const t = enc.encode(text);
  const cap = 177 * 177;
  const at = x.bw_alloc(Math.max(t.length, 1));
  new Uint8Array(x.memory.buffer, at, t.length).set(t);
  const out = x.bw_alloc(cap);
  const side = x.bw_qr(at, t.length, out, cap);
  const cells = side > 0 ? new Uint8Array(x.memory.buffer, out, side * side).slice() : null;
  x.bw_free(at, Math.max(t.length, 1));
  x.bw_free(out, cap);
  return cells ? { side, cells } : null;
}

export async function loadReader(url = '/store/verify.wasm') {
  const r = await fetch(url);
  const { instance } = await WebAssembly.instantiate(await r.arrayBuffer(), {});
  return instance.exports;
}

const WORDS = {
  en: { ok: 'This price is in the menu the venue published.', bad: 'This price is NOT in the menu this receipt names.', broken: 'This code is damaged and proves nothing.', live: 'Matches the venue\'s menu now.', old: 'The venue\'s menu has changed since this receipt.', signed: 'The venue publishes an ML-DSA-65 signature of this menu (key {k}); this page does not check it.', unsigned: 'The venue has not signed its menu yet: the check above is against the root printed on the receipt.', offline: 'Offline: checked on this phone only.' },
  sq: { ok: 'Ky çmim është në menunë që lokali publikoi.', bad: 'Ky çmim NUK është në menunë që emërton fatura.', broken: 'Ky kod është i dëmtuar dhe nuk provon asgjë.', live: 'Përputhet me menunë e lokalit tani.', old: 'Menuja e lokalit ka ndryshuar që nga kjo faturë.', signed: 'Lokali publikon një nënshkrim ML-DSA-65 të menusë (çelësi {k}); kjo faqe nuk e kontrollon.', unsigned: 'Lokali nuk e ka nënshkruar ende menunë: kontrolli është kundrejt rrënjës në faturë.', offline: 'Pa lidhje: kontrolluar vetëm në këtë telefon.' },
  uk: { ok: 'Ця ціна є в меню, яке заклад опублікував.', bad: 'Цієї ціни НЕМАЄ в меню, яке названо в чеку.', broken: 'Код пошкоджено, він нічого не доводить.', live: 'Збігається з поточним меню закладу.', old: 'Меню закладу змінилося після цього чека.', signed: 'Заклад публікує підпис ML-DSA-65 цього меню (ключ {k}); ця сторінка його не перевіряє.', unsigned: 'Заклад ще не підписав меню: перевірка йде проти кореня, надрукованого в чеку.', offline: 'Без мережі: перевірено лише на цьому телефоні.' },
  ru: { ok: 'Эта цена есть в меню, которое опубликовало заведение.', bad: 'Этой цены НЕТ в меню, указанном в чеке.', broken: 'Код повреждён и ничего не доказывает.', live: 'Совпадает с текущим меню заведения.', old: 'Меню заведения изменилось после этого чека.', signed: 'Заведение публикует подпись ML-DSA-65 этого меню (ключ {k}); эта страница её не проверяет.', unsigned: 'Заведение ещё не подписало меню: проверка идёт по корню, напечатанному в чеке.', offline: 'Без сети: проверено только на этом телефоне.' },
};
const say = (k, v = {}) => { const l = (navigator.language || 'en').slice(0, 2); return ((WORDS[l] || WORDS.en)[k] || WORDS.en[k]).replace('{k}', v.k || ''); };

/// The page: /store/verify.html#s=<slug>&c=<certificate>.
export async function boot(doc = document) {
  const el = doc.getElementById('verify');
  if (!el) return;
  const h = new URLSearchParams(location.hash.slice(1));
  const line = (t, cls) => { const p = doc.createElement('p'); p.textContent = t; if (cls) p.className = cls; el.appendChild(p); };
  let bytes;
  try { bytes = unb64url(h.get('c') || ''); } catch { bytes = new Uint8Array(0); }
  const c = readCert(bytes);
  if (c) line(`${c.id}: ${c.price}`, 'v-line');
  const x = await loadReader().catch(() => null);
  if (!x) { line(say('broken'), 'v-bad'); return; }
  const v = verifyWith(x, bytes);
  line(say(v.status === 0 ? 'ok' : v.status === 51 ? 'bad' : 'broken'), v.status === 0 ? 'v-ok' : 'v-bad');
  if (v.status !== 0) return;
  try {
    const { cdnOrigin } = await import('/store/shell.js');
    const m = await (await fetch(`${cdnOrigin()}/v/${encodeURIComponent(h.get('s') || '')}/manifest.json`)).json();
    line(say(m.root === c.root ? 'live' : 'old'));
    line(m.sig && m.pk_id ? say('signed', { k: m.pk_id }) : say('unsigned'));
  } catch { line(say('offline')); }
}

if (typeof document !== 'undefined' && document.getElementById('verify')) boot();
