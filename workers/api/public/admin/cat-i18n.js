// CATEGORY NAMES IN EVERY LANGUAGE (W-WIRE row 3). A dish carries its own
// translations; a category heading had no write side at all until
// `POST /api/owner/i18n`, so every heading stayed in the venue's language
// whatever the customer chose. One field per category per language the
// console speaks (never a hard-coded list: a fourth language is a new field),
// read back from the storefront's own menu in that language.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, $$, esc, t, api, post, busy, toast, withLoc, S, LANGS } from '/admin/core.js';
import { btn, field, loading } from '/admin/parts.js';
import { venuePath, fail, open, paint } from '/admin/wire-core.js';

/// The entries to write: every field whose text changed.
export function changed(fields, before){
  return fields.filter(f => (before[f.id] || {})[f.locale] !== f.value.trim())
    .map(f => ({ entity: 'category', id: f.id, locale: f.locale, field: 'name', value: f.value.trim() }));
}

export async function openCategoryWords(){
  const host = open('w_catWords', 'w_catWordsHint', 'catWords');
  host.innerHTML = loading(3);
  const base = (S.venue && S.venue.default_locale) || LANGS[0];
  const others = LANGS.filter(l => l !== base);
  const names = {};
  let cats = [];
  try {
    const menus = await Promise.all([base, ...others].map(l => api(venuePath(`/menu?locale=${encodeURIComponent(l)}&fresh=1`)).then(m => [l, m])));
    for (const [l, m] of menus) for (const c of m.categories || []) {
      (names[c.id] = names[c.id] || {})[l] = c.name || '';
      if (l === base) cats.push(c);
    }
  } catch (e) { return fail(e); }
  host.innerHTML = cats.map(c => `<p class="eyebrow mt-3">${esc(names[c.id][base] || c.name)}</p>
    <div class="grid2">${others.map(l => field({ id: `cw-${c.id}-${l}`, label: l.toUpperCase(), value: names[c.id][l] === names[c.id][base] ? '' : names[c.id][l],
      placeholder: names[c.id][base], data: { cw: c.id, lang: l }, autocomplete: 'off', maxlength: 80 })).join('')}</div>`).join('')
    + `<div class="btn-row">${btn({ id: 'cwSave', variant: 'primary', icon: 'check', key: 'save' })}</div>`;
  paint(host);
  // What the storefront showed = the base name when no translation exists.
  const before = {};
  for (const c of cats) for (const l of others) (before[c.id] = before[c.id] || {})[l] = names[c.id][l] === names[c.id][base] ? '' : names[c.id][l];
  $('#cwSave').onclick = async () => {
    const fields = $$('[data-cw]', host).map(i => ({ id: i.dataset.cw, locale: i.dataset.lang, value: i.value }));
    const entries = changed(fields, before);
    if (!entries.length) return toast(t('w_nothingChanged'));
    try { await busy($('#cwSave'), () => post('/owner/i18n', withLoc({ entries }))); toast(t('saved')); openCategoryWords(); } catch (e) { fail(e); }
  };
}
