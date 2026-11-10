// A POSITION TYPED INSTEAD OF CHOSEN, the DOM half: one text field with live
// suggestions, and under it what the typed name turned out to be
// (`supply-resolve.js` decides):
//
//   match   the supply's name and unit
//   near    a question that must be answered: "is it this one?" -- yes takes
//           that id, "no, a new position" makes one. Never merged silently.
//   none    a new position: its unit and its type, both mandatory, and an
//           optional category (free text, the venue's own groups suggested)
//
// A new position is written through `POST /api/owner/supplies/bulk`, one call
// for everything a form made, and the answer's ids are what gets received
// onto (`createAll`). A name the hub says it has already is that record.
//
// ASCII QUOTES ONLY in this file.

import { $, $$, esc, t, post, withLoc, toast, sheet, busy, retranslate } from '/admin/core.js';
import { btn, field, chips, press, pill, rowBtn } from '/admin/parts.js';
import { KINDS } from '/admin/ingredients-view.js';
import * as R from '/admin/supply-resolve.js';

export const UNITS = ['g', 'ml', 'unit'];
const say = (key, name) => t(key).replace('{name}', name);

/// The markup of one field; `n` is unique on the sheet.
export function markup(n, { labelKey = 'ka_ingredient', tour } = {}){
  return `<div data-sr="${esc(n)}">${field({ id: 'sr-' + n, key: labelKey, phKey: 'sr_type', autocomplete: 'off', tour, attrs: { data: { srq: '1' } } })}
    <div class="rows" data-srs></div><div data-sra></div></div>`;
}

/// Bind the field under `root` (the element `markup` drew). `supplies()` is
/// read on every keystroke, so a list refreshed behind the sheet is the one
/// searched. `allowNew: false` is a picker that only finds.
export function bind(root, { supplies, allowNew = true, onChange = () => {} }){
  const inp = $('[data-srq]', root), sugg = $('[data-srs]', root), ans = $('[data-sra]', root);
  let st = { mode: 'empty' };
  const all = () => supplies() || [];
  const set = next => { st = next; draw(); onChange(read()); };

  function draw(){
    if (st.mode === 'match') ans.innerHTML = `<p class="mono small">${pill('ok', { label: st.sup.name || st.sup.id })} ${esc(st.sup.unit || '')}</p>`;
    else if (st.mode === 'ask') {
      const one = st.cands.length === 1;
      ans.innerHTML = `<p class="ui-label">${esc(one ? say('sr_isIt', st.cands[0].name) : t('sr_which'))}</p>
        <div class="btn-row">${st.cands.map(s => btn({ variant: 'primary', icon: 'check', label: one ? t('sr_yes') : `${s.name} (${s.unit})`, data: { sryes: s.id } })).join('')}
        ${allowNew ? btn({ icon: 'plus', key: 'sr_new', data: { srnew: '1' } }) : ''}</div>`;
    } else if (st.mode === 'new' && allowNew) {
      ans.innerHTML = `<p class="hint">${esc(say('sr_newHint', st.name))}</p>
        <p class="ui-label" data-t="unit"></p>${chips({ values: UNITS.map(u => ({ value: u, key: 'sr_unit_' + u })), value: st.unit, attr: 'srunit', labelKey: 'unit' })}
        <p class="ui-label" data-t="kind"></p>${chips({ values: KINDS.map(([k, ic]) => ({ value: k, key: 'kind_' + k, icon: ic })), value: st.kind, attr: 'srkind', labelKey: 'kind' })}
        ${field({ key: 'category', value: st.category || '', autocomplete: 'off', attrs: { list: 'srCats-' + root.dataset.sr, data: { srcat: '1' } } })}
        <datalist id="srCats-${esc(root.dataset.sr)}">${R.categoriesOf(all()).map(c => `<option value="${esc(c)}">`).join('')}</datalist>`;
    } else if (st.mode === 'new') ans.innerHTML = `<p class="hint" data-t="sr_noSuch"></p>`;
    else ans.innerHTML = '';
    retranslate(ans);
    for (const b of $$('[data-sryes]', ans)) b.onclick = () => pick(st.cands.find(s => s.id === b.dataset.sryes));
    const nw = $('[data-srnew]', ans); if (nw) nw.onclick = () => set({ mode: 'new', name: inp.value.trim(), declined: st.cands.map(s => s.id) });
    for (const b of $$('[data-srunit]', ans)) b.onclick = () => { st.unit = b.dataset.srunit; press($$('[data-srunit]', ans), b); onChange(read()); };
    for (const b of $$('[data-srkind]', ans)) b.onclick = () => { st.kind = b.dataset.srkind; press($$('[data-srkind]', ans), b); onChange(read()); };
    const cat = $('[data-srcat]', ans); if (cat) cat.onchange = () => { st.category = R.canonCategory(cat.value, R.categoriesOf(all())); cat.value = st.category; };
  }

  function pick(sup){
    if (!sup) return;
    inp.value = sup.name || sup.id; sugg.innerHTML = '';
    set({ mode: 'match', sup });
  }

  /// Decide what the typed text is, unless the person already answered for it.
  function settle(){
    const name = inp.value.trim();
    if (st.mode !== 'typing' && st.mode !== 'empty') return read();
    const r = R.resolve(name, all());
    if (r.empty) set({ mode: 'empty' });
    else if (r.match) set({ mode: 'match', sup: r.match });
    else if (r.near) set({ mode: 'ask', cands: r.near });
    else set({ mode: 'new', name });
    return read();
  }

  /// `{state}`: empty | ok (`sup`) | ask (a question is open) | incomplete
  /// (a new position without its unit or type) | new (`name, unit, kind, category`).
  function read(){
    if (st.mode === 'empty' || (st.mode === 'typing' && !inp.value.trim())) return { state: 'empty' };
    if (st.mode === 'match') return { state: 'ok', sup: st.sup };
    if (st.mode === 'new' && allowNew) {
      const cat = $('[data-srcat]', ans);
      const category = R.canonCategory(cat ? cat.value : st.category, R.categoriesOf(all()));
      return st.unit && st.kind ? { state: 'new', name: st.name, unit: st.unit, kind: st.kind, category, notNear: st.declined || [] } : { state: 'incomplete' };
    }
    return { state: 'ask' };
  }

  inp.oninput = () => {
    st = { mode: 'typing' }; ans.innerHTML = '';
    sugg.innerHTML = R.suggest(inp.value, all()).map(s => rowBtn({ title: s.name || s.id, sub: esc(`${s.category || ''} · ${s.unit || ''}`), data: { srpick: s.id } })).join('');
    for (const b of $$('[data-srpick]', sugg)) b.onclick = () => pick(all().find(s => s.id === b.dataset.srpick));
  };
  inp.onchange = () => { settle(); };
  return { read, settle, pick, ask: cands => set({ mode: 'ask', cands }), focus: () => inp.focus(), el: root, input: inp };
}

/// Write the new positions of one form in ONE call. `news`: what `read()`
/// answered for each (`{name, unit, kind, category}`); the same name typed on
/// two lines is one position. Answers a Map from the fold of each name to its
/// supply (`retired: true` when the hub has that name on a retired record),
/// and adds what was created to `ctx.data.supplies`, so every list, group and
/// suggestion on the screen has it before the next refresh.
export async function createAll(news, ctx){
  const byFold = new Map();
  for (const n of news) if (!byFold.has(R.fold(n.name))) byFold.set(R.fold(n.name), n);
  const items = [...byFold.values()].map(n => ({ name: n.name, unit: n.unit, kind: n.kind, ...(n.category ? { category: n.category } : {}), notNear: n.notNear || [] }));
  const out = new Map();
  if (!items.length) return out;
  const r = await post('/owner/supplies/bulk', withLoc({ items }));
  const have = ctx?.data?.supplies;
  for (const a of r?.added || []) {
    const sup = { id: a.id, name: a.name, unit: a.unit || byFold.get(R.fold(a.name))?.unit || 'g', kind: a.kind, category: a.category || '', onHand: 0, reserved: 0, available: 0, counted: false, lots: [] };
    out.set(R.fold(a.name), sup);
    if (Array.isArray(have) && !have.some(s => s.id === sup.id)) have.push(sup);
  }
  for (const m of r?.matched || []) {
    const known = (have || []).find(s => s.id === m.id);
    out.set(R.fold(m.typed), { ...(known || { id: m.id, name: m.name, unit: m.unit }), retired: !!m.retired });
  }
  // A name the hub has and this phone has not read yet: a question for the person, on the screen they are on.
  for (const m of r?.near || []) {
    let known = (have || []).find(s => s.id === m.id);
    if (!known) { known = { id: m.id, name: m.name, unit: m.unit, kind: '', category: '', onHand: 0, reserved: 0, available: 0 }; have?.push(known); }
    const f = R.fold(m.typed), row = out.get(f) || { near: [] };
    row.near.push(known);
    out.set(f, row);
  }
  return out;
}

/// The save of a one-position form: an answered match, or a new position made.
const saveNew = (f, ctx, then) => async () => {
  const got = f.settle();
  if (got.state === 'ok') return then(got.sup);
  if (got.state !== 'new') return toast(t(got.state === 'ask' ? 'sr_answer' : got.state === 'incomplete' ? 'sr_fillNew' : 'required'));
  try {
    const made = await busy($('#srGo'), () => createAll([got], ctx));
    const sup = made.get(R.fold(got.name));
    if (sup?.near) { f.ask(sup.near); return toast(t('sr_answer')); }
    if (!sup || sup.retired) return toast(t(sup ? 'sr_retired' : 'sr_noId'));
    ctx.reload?.();
    then(sup);
  } catch (e) { toast(String(e.message || e)); }
};

/// The same form inside `host`, for a picker whose sheet must stay open.
export function inlineNew(host, ctx, name, then){
  host.innerHTML = `${markup('new', { labelKey: 'name' })}<div class="btn-row">${btn({ id: 'srGo', variant: 'primary', icon: 'check', key: 'save' })}</div>`;
  retranslate(host);
  const f = bind($('[data-sr="new"]', host), { supplies: () => ctx.data?.supplies });
  f.input.value = name || '';
  f.settle();
  $('#srGo').onclick = saveNew(f, ctx, then);
}

/// A new position from a picker: its name (typed in the search box), unit,
/// type and category, then `then(sup)`.
export function openNew(ctx, name, then){
  sheet(`<p class="eyebrow" data-t="supplies"></p><h2 data-t="addSupply"></h2>${markup('new', { labelKey: 'name' })}
    <div class="btn-row">${btn({ id: 'srGo', variant: 'primary', icon: 'check', key: 'save' })}</div>`, { name: 'supplyNew' });
  const f = bind($('[data-sr="new"]', $('#sheetIn')), { supplies: () => ctx.data?.supplies });
  f.input.value = name || '';
  f.settle();
  $('#srGo').onclick = saveNew(f, ctx, then);
}
