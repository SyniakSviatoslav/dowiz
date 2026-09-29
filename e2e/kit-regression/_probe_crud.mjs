// DELETING AND EDITING DISHES, THE MENU AND CATEGORIES, on a live hub (W-CRUD, 2026-09-29).
//
// Makes its own category and dishes on the QA hub, walks every edit the console
// offers (price, stop-list, name in each language, category move, order, photo
// set and clear, category rename) and every delete (one dish, many, a category
// with its dishes), and after EACH step reads the owner list AND the public menu
// -- `?fresh=1` as the console does, and cached as a customer does, in every
// language -- and says what each showed. It cleans up after itself.
//
// QA HUB ONLY. Never point it at a real venue: it writes the menu.
//   HOST=https://qa-durres.dowiz.org node e2e/kit-regression/_probe_crud.mjs
import fs from 'node:fs';

const HOST = process.env.HOST || 'https://qa-durres.dowiz.org';
if (!/qa-/.test(HOST)) { console.log('refusing: this probe writes the menu and runs on a QA hub only'); process.exit(2); }
const creds = Object.fromEntries(fs.readFileSync('/root/.dowiz_owner', 'utf8')
  .split('\n').filter(l => l.startsWith('export ')).map(l => l.slice(7).split('=').map(s => s.trim())));
const LANGS = ['sq', 'en', 'uk', 'ru'];
const TAG = `qacrud${Date.now().toString(36).slice(-4)}`;

const fails = [], notes = [];
const step = (name, ok, detail = '') => { if (!ok) fails.push(name); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${detail ? ' :: ' + detail : ''}`); };
const info = (name, detail) => { notes.push(`${name}: ${detail}`); console.log(`info ${name} :: ${detail}`); };
const short = b => JSON.stringify(b).slice(0, 160);

const j = async (p, o = {}) => {
  const r = await fetch(`${HOST}${p}`, o);
  const t = await r.text();
  try { return { status: r.status, body: JSON.parse(t) }; } catch { return { status: r.status, body: t }; }
};
let JWT = '';
const own = (p, body, method = 'POST', jwt = JWT) => j(p, {
  method, headers: { authorization: `Bearer ${jwt}`, 'content-type': 'application/json' },
  ...(body === undefined ? {} : { body: JSON.stringify(body) }),
});
const raw = (p, bytes, jwt = JWT) => j(p, { method: 'POST', headers: { authorization: `Bearer ${jwt}`, 'content-type': 'application/octet-stream' }, body: bytes });

// ── sign in; the venue is the host's ────────────────────────────────────────
const login = await j('/api/auth/login', { method: 'POST', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }) });
JWT = login.body?.access_token || '';
const slug = new URL(HOST).hostname.split('.')[0];
const menu0 = await j(`/api/public/locations/${slug}/menu?fresh=1`);
const VENUE = menu0.body?.location?.id;
const BASE = menu0.body?.location?.defaultLocale || 'sq';
step('owner signs in and the venue is known', login.status === 200 && !!VENUE, `${VENUE} base=${BASE} token.loc=${login.body?.user?.locationId}`);
if (!VENUE) process.exit(1);
const loc = body => ({ location_id: VENUE, ...body });
{
  // Does a `deny_unknown_fields` route refuse an extra field on the live build? 400 = yes; 404 = the field passed and the id was looked up.
  const r = await own('/api/owner/categories/nothing-here/delete', loc({ zzz: 1 }));
  info('an unknown field on a deny_unknown_fields route answers', `${r.status} ${short(r.body)}`);
}

// ── readers ─────────────────────────────────────────────────────────────────
const ownerList = async () => (await own('/api/owner/products', undefined, 'GET')).body?.products || [];
const ownerCats = async () => (await own('/api/owner/categories', undefined, 'GET')).body?.categories || [];
const pub = async (l = BASE, fresh = true) => (await j(`/api/public/locations/${slug}/menu?locale=${l}${fresh ? '&fresh=1' : ''}`)).body;
const findPub = (m, id) => { for (const c of m?.categories || []) { const p = (c.products || []).find(x => x.id === id); if (p) return { ...p, cat: c.id, catName: c.name }; } return null; };
const findCat = (m, id) => (m?.categories || []).find(c => c.id === id) || null;
/// The public menu in every language, fresh; and cached in the base language.
const everywhere = async id => {
  const out = {};
  for (const l of LANGS) out[l] = findPub(await pub(l, true), id);
  out.cached = findPub(await pub(BASE, false), id);
  return out;
};

// ── 1. a category ───────────────────────────────────────────────────────────
const cat = await own('/api/owner/categories', loc({ name: `QA Crud ${TAG}` }));
const CAT = cat.body?.id;
step('category is made', cat.status === 200 && !!CAT, `${cat.status} ${short(cat.body)}`);
step('empty category is in the owner list', (await ownerCats()).some(c => c.id === CAT));
step('empty category is NOT on the public menu', !findCat(await pub(), CAT));
const cat2 = await own('/api/owner/categories', loc({ name: `QA Crud B ${TAG}` }));
const CAT2 = cat2.body?.id;

// ── 2. dishes ───────────────────────────────────────────────────────────────
const mk = async (name, price) => { const r = await own('/api/owner/products', loc({ name, category_id: CAT, price })); step(`dish "${name}" is made`, r.status === 200 && !!r.body?.id, `${r.status} ${short(r.body)}`); return r.body?.id; };
const A = await mk(`Alpha ${TAG}`, 500), B = await mk(`Beta ${TAG}`, 600);
{
  const ev = await everywhere(A);
  step('new dish is on the fresh public menu in the base language', !!ev[BASE], short(ev[BASE]));
  step('new dish is in the owner list', (await ownerList()).some(p => p.id === A));
  info('new dish on the CACHED menu right after create', ev.cached ? 'visible' : 'not yet (edge cache)');
  info('sortOrder of A, B', `${ev[BASE]?.sortOrder}, ${findPub(await pub(), B)?.sortOrder}`);
}

// ── 3. price, stop-list ─────────────────────────────────────────────────────
{
  const r = await own(`/api/owner/products/${A}`, loc({ price: 550 }));
  const p = findPub(await pub(), A);
  step('price edit reaches the fresh public menu', r.status === 200 && p?.price === 550, `${r.status} price=${p?.price}`);
  const r2 = await own(`/api/owner/products/${A}`, loc({ available: false, unavailable_note: 'qa off' }));
  const p2 = findPub(await pub(), A);
  step('stop-list reaches the fresh public menu', r2.status === 200 && p2?.unavailableNote === 'qa off' && p2?.available === false, `${r2.status} ${short(p2)}`);
  // The allergen publish gate may be ON at this venue: 'none of the fourteen' is declared with the switch.
  const r3 = await own(`/api/owner/products/${A}`, loc({ available: true, allergens: [] }));
  const p3 = findPub(await pub(), A);
  step('back on sale clears the note', r3.status === 200 && p3?.available === true && !p3?.unavailableNote, `${r3.status} ${short(p3)}`);
}

// ── 4. names in every language ──────────────────────────────────────────────
{
  // What the console's dish sheet is prefilled from: the fresh menu in each other language.
  const before = await everywhere(A);
  const untranslated = LANGS.filter(l => l !== BASE).map(l => `${l}=${before[l]?.name}`);
  info('an UNTRANSLATED dish reads back in the other languages as', untranslated.join(' '));
  const tr = { en: { name: `Alpha EN ${TAG}`, description: 'en desc' }, uk: { name: `Альфа UK ${TAG}` }, ru: { name: `Альфа RU ${TAG}` } };
  delete tr[BASE];
  const r = await own(`/api/owner/products/${A}`, loc({ translations: tr }));
  const ev = await everywhere(A);
  for (const l of Object.keys(tr)) step(`name in ${l} reaches the fresh public menu`, r.status === 200 && ev[l]?.name === tr[l].name, `${ev[l]?.name}`);
  step('the base name is untouched by translations', ev[BASE]?.name === `Alpha ${TAG}`, ev[BASE]?.name);
  // The base language: the name and description the venue itself uses.
  const rn = await own(`/api/owner/products/${A}`, loc({ name: `Alpha2 ${TAG}`, description: 'base desc' }));
  const after = await everywhere(A);
  step('base-language RENAME reaches the fresh public menu', rn.status === 200 && after[BASE]?.name === `Alpha2 ${TAG}` && after[BASE]?.description === 'base desc', `${rn.status} ${short(rn.body)} name=${after[BASE]?.name} desc=${after[BASE]?.description}`);
  for (const l of Object.keys(tr)) step(`rename keeps the ${l} name`, after[l]?.name === tr[l].name, after[l]?.name);
  // An empty translation deletes it: the storefront falls back to the base name.
  const rd = await own(`/api/owner/products/${A}`, loc({ translations: { uk: { name: '' } } }));
  const gone = await everywhere(A);
  step('an emptied uk name falls back to the base name', rd.status === 200 && gone.uk?.name === after[BASE]?.name, gone.uk?.name);
}

// ── 5. category move and order ──────────────────────────────────────────────
{
  const r = await own(`/api/owner/products/${A}`, loc({ category_id: CAT2 }));
  const p = findPub(await pub(), A);
  step('CATEGORY MOVE reaches the fresh public menu', r.status === 200 && p?.cat === CAT2, `${r.status} ${short(r.body)} cat=${p?.cat}`);
  const back = await own(`/api/owner/products/${A}`, loc({ category_id: CAT }));
  step('moved back', back.status === 200 && findPub(await pub(), A)?.cat === CAT);
  const cats = await ownerCats();
  info('owner category counts after the move', cats.filter(c => c.id === CAT || c.id === CAT2).map(c => `${c.id}=${c.count}`).join(' '));
  // Order: B before A.
  const ro = await own(`/api/owner/products/${B}`, loc({ move: 'up' }));
  const m = await pub();
  const ids = (findCat(m, CAT)?.products || []).map(x => x.id);
  step('ORDER: moving B up puts it before A on the fresh public menu', ro.status === 200 && ids.indexOf(B) < ids.indexOf(A), `${ro.status} ${short(ro.body)} order=${ids.join(',')}`);
  const rc = await own('/api/owner/categories', loc({ id: CAT2, name: `QA Crud B ${TAG}`, move: 'up' }));
  const mc = (await pub()).categories.map(c => c.id);
  info('category order move', `${rc.status} ${short(rc.body)}`);
  const cs = (await ownerCats()).map(c => c.id);
  step('ORDER: moving category B up puts it before A in the owner list', rc.status === 200 && cs.indexOf(CAT2) < cs.indexOf(CAT), `owner=${cs.filter(c => c.startsWith('qa-crud')).join(',')} public=${mc.filter(c => c.startsWith('qa-crud')).join(',')}`);
}

// ── 6. photo set and clear ──────────────────────────────────────────────────
{
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==', 'base64');
  const r = await raw(`/api/owner/products/${A}/image`, png);
  const p = findPub(await pub(), A);
  step('photo set reaches the fresh public menu', r.status === 200 && !!p?.imageUrl, `${r.status} ${short(r.body)} url=${p?.imageUrl}`);
  const rs = await raw(`/api/owner/products/${A}/image?variant=small`, png);
  const ps = findPub(await pub(), A);
  step('small variant set', rs.status === 200 && !!ps?.imageUrlSmall, `${rs.status}`);
  const v0 = (await pub()).location.menuVersion;
  const rc = await own(`/api/owner/products/${A}/image/clear`, loc({}));
  const m = await pub();
  const pc = findPub(m, A);
  step('photo clear reaches the fresh public menu (both renderings)', rc.status === 200 && !pc?.imageUrl && !pc?.imageUrlSmall, `${rc.status} url=${pc?.imageUrl} small=${pc?.imageUrlSmall}`);
  info('menuVersion moved on photo clear', `${v0} -> ${m.location.menuVersion}`);
}

// ── 7. category rename keeps its translations ───────────────────────────────
{
  const ri = await own('/api/owner/i18n', loc({ entries: [{ entity: 'category', id: CAT, locale: 'uk', field: 'name', value: `Кат UK ${TAG}` }] }));
  step('category uk name written', ri.status === 200 && ri.body?.written === 1, short(ri.body));
  const rr = await own('/api/owner/categories', loc({ id: CAT, name: `QA Crud R ${TAG}` }));
  const m = await pub(), mu = await pub('uk');
  step('category rename reaches the fresh public menu', rr.status === 200 && findCat(m, CAT)?.name === `QA Crud R ${TAG}`, `${rr.status} ${findCat(m, CAT)?.name}`);
  step('category rename keeps the uk heading', findCat(mu, CAT)?.name === `Кат UK ${TAG}`, findCat(mu, CAT)?.name);
  step('renamed category keeps its dishes', (findCat(m, CAT)?.products || []).length === 2, `${(findCat(m, CAT)?.products || []).length}`);
}

// ── 8. delete one ───────────────────────────────────────────────────────────
{
  const r = await own(`/api/owner/products/${B}/delete`, loc({}));
  const ev = await everywhere(B);
  step('single delete: gone from the fresh public menu in every language', r.status === 200 && LANGS.every(l => !ev[l]), `${r.status} ${LANGS.map(l => `${l}=${ev[l] ? 'STILL' : 'gone'}`).join(' ')}`);
  step('single delete: gone from the owner list', !(await ownerList()).some(p => p.id === B));
  info('single delete on the CACHED menu right after', ev.cached ? 'STILL VISIBLE (edge cache)' : 'gone');
  info('single delete: translations swept', `${r.body?.translations}`);
  const again = await own(`/api/owner/products/${B}/delete`, loc({}));
  step('deleting twice is a 404', again.status === 404, `${again.status}`);
  // ONE bounded wait, and what it waits for: the cached menu's max-age is 30 s;
  // does a customer's next load after that window still show the deleted dish?
  if (ev.cached && !process.env.NO_WAIT) {
    await new Promise(r => setTimeout(r, 35_000));
    info('single delete on the CACHED menu 35 s later', findPub(await pub(BASE, false), B) ? 'STILL VISIBLE' : 'gone');
  }
}

// ── 9. delete many ──────────────────────────────────────────────────────────
{
  const C = await mk(`Gamma ${TAG}`, 700), D = await mk(`Delta ${TAG}`, 800);
  const r = await own('/api/owner/products/delete', loc({ ids: [C, D] }));
  step('bulk delete answers both ids', r.status === 200 && (r.body?.deleted || []).length === 2, `${r.status} ${short(r.body)}`);
  const m = await pub();
  step('bulk delete: both gone from the fresh public menu', !findPub(m, C) && !findPub(m, D));
  step('bulk delete: both gone from the owner list', !(await ownerList()).some(p => p.id === C || p.id === D));
}

// ── 10. a recreated dish does not inherit the deleted one's words ───────────
{
  const r = await own('/api/owner/products', loc({ name: `Beta ${TAG}`, category_id: CAT, price: 600 }));
  step('the same name makes the same id again', r.body?.id === B, `${r.body?.id} vs ${B}`);
  // A language that is NOT the base: the base language's translation is never read.
  const other = LANGS.find(l => l !== BASE);
  await own(`/api/owner/products/${B}`, loc({ translations: { [other]: { name: `Beta ${other.toUpperCase()} ${TAG}` } } }));
  const del = await own(`/api/owner/products/${B}/delete`, loc({}));
  info('delete of a translated dish: translations swept', `${del.body?.translations}`);
  const r2 = await own('/api/owner/products', loc({ name: `Beta ${TAG}`, category_id: CAT, price: 600 }));
  const ev = await everywhere(r2.body?.id);
  step(`a recreated dish does not inherit the deleted one's ${other} name`, ev[other]?.name === `Beta ${TAG}`, `${other}=${ev[other]?.name}`);
}

// ── 11. delete a category with dishes ───────────────────────────────────────
{
  const n = (await ownerCats()).find(c => c.id === CAT)?.count;
  const r0 = await own(`/api/owner/categories/${CAT}/delete`, loc({}));
  step('a category with dishes is refused without the count', r0.status === 409 && r0.body?.dishes === n, `${r0.status} ${short(r0.body)} n=${n}`);
  const r1 = await own(`/api/owner/categories/${CAT}/delete`, loc({ with_dishes: n }));
  const m = await pub();
  step('category delete with its dishes', r1.status === 200 && !findCat(m, CAT) && !findPub(m, A), `${r1.status} ${short(r1.body)}`);
  step('category delete: gone from the owner categories', !(await ownerCats()).some(c => c.id === CAT));
  step('category delete: its dishes gone from the owner list', !(await ownerList()).some(p => p.id === A));
  const mu = await pub('uk');
  step('category delete: gone in uk too', !findCat(mu, CAT));
  const rc = await own('/api/owner/categories', loc({ name: `QA Crud R ${TAG}`, id: CAT }));
  const mu2 = await pub('uk');
  const rcd = await own('/api/owner/products', loc({ name: `Omega ${TAG}`, category_id: CAT, price: 100 }));
  const mu3 = await pub('uk');
  step('a recreated category does not inherit the deleted one\'s uk heading', rc.status === 200 && findCat(mu3, CAT)?.name === `QA Crud R ${TAG}`, `uk=${findCat(mu3, CAT)?.name}`);
  await own(`/api/owner/categories/${CAT}/delete`, loc({ with_dishes: 1 }));
  void mu2; void rcd;
}

// ── 12. the kitchen role: a member of staff made for this run ───────────────
{
  const email = `qa-kitchen-${TAG}@example.com`, pass = `Kitchen-${TAG}-pass1`;
  const inv = await own('/api/owner/staff/invite', loc({ email, name: `QA Kitchen ${TAG}`, role: 'kitchen' }));
  const code = inv.body?.code;
  step('a kitchen invite is minted', inv.status === 200 && !!code, `${inv.status} ${short(inv.body)}`);
  let KJ = null;
  if (code) {
    const cl = await j('/api/staff/claim', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, code, password: pass }) });
    KJ = cl.body?.jwt || null;
    if (!KJ) { const sl = await j('/api/staff/login', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ email, password: pass }) }); KJ = sl.body?.jwt || null; info('kitchen claim then login', `${cl.status} ${short(cl.body)} / ${sl.status}`); }
    step('the kitchen claims the code and is signed in', !!KJ, `${cl.status} ${short(cl.body)}`);
  }
  if (KJ) {
    const kc = await own('/api/owner/categories', loc({ name: `QA Kitchen ${TAG}` }), 'POST', KJ);
    const KC = kc.body?.id;
    step('kitchen makes a category', kc.status === 200, `${kc.status} ${short(kc.body)}`);
    const kd = await own('/api/owner/products', loc({ name: `Kitchen dish ${TAG}`, category_id: KC, price: 100 }), 'POST', KJ);
    step('kitchen makes a dish', kd.status === 200, `${kd.status} ${short(kd.body)}`);
    const ke = await own(`/api/owner/products/${kd.body?.id}`, loc({ price: 120, name: `Kitchen dish2 ${TAG}` }), 'POST', KJ);
    step('kitchen edits a dish (price and name)', ke.status === 200, `${ke.status} ${short(ke.body)}`);
    const kn = await own(`/api/owner/products/${kd.body?.id}`, loc({ move: 'up' }), 'POST', KJ);
    step('kitchen moves a dish', kn.status === 200, `${kn.status} ${short(kn.body)}`);
    const kl = await own('/api/owner/categories', undefined, 'GET', KJ);
    step('kitchen lists categories', kl.status === 200, `${kl.status}`);
    const kp = await own(`/api/owner/products?id=${kd.body?.id}`, undefined, 'GET', KJ);
    step('kitchen reads the owner product list', kp.status === 200, `${kp.status}`);
    const kx = await own(`/api/owner/products/${kd.body?.id}/delete`, loc({}), 'POST', KJ);
    step('kitchen deletes a dish', kx.status === 200, `${kx.status}`);
    const kb = await own('/api/owner/products/delete', loc({ ids: ['nothing'] }), 'POST', KJ);
    info('kitchen bulk delete (owner-only by design)', `${kb.status}`);
    const kcd = await own(`/api/owner/categories/${KC}/delete`, loc({}), 'POST', KJ);
    step('kitchen deletes an empty category', kcd.status === 200, `${kcd.status} ${short(kcd.body)}`);
  }
  // The member of staff is suspended again: their sessions end.
  const list = await own('/api/owner/staff', undefined, 'GET');
  const me = (list.body?.staff || []).find(x => String(x.email || '').toLowerCase() === email || x.name === `QA Kitchen ${TAG}`);
  if (me) { const off = await own(`/api/owner/staff/${me.id}`, loc({ active: false })); info('kitchen account suspended', `${off.status} ${short(off.body)}`); }
  else info('kitchen account after the run', `not in the staff list (${list.status})`);
}

// ── cleanup: anything of this run still there ───────────────────────────────
{
  const mine = (await ownerList()).filter(p => String(p.name).includes(TAG)).map(p => p.id);
  if (mine.length) await own('/api/owner/products/delete', loc({ ids: mine }));
  for (const c of (await ownerCats()).filter(c => String(c.name).includes(TAG))) await own(`/api/owner/categories/${c.id}/delete`, loc(c.count ? { with_dishes: c.count } : {}));
  const left = (await ownerList()).filter(p => String(p.name).includes(TAG)).length + (await ownerCats()).filter(c => String(c.name).includes(TAG)).length;
  step('cleanup: nothing of this run is left', left === 0, `${left}`);
}

console.log(`\n${fails.length ? 'FAIL' : 'PASS'} ${fails.length} failed; notes:\n  ${notes.join('\n  ')}`);
process.exit(fails.length ? 1 : 0);
