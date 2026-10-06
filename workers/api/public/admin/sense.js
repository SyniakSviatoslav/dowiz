// A DISH'S TASTE, TEXTURE AND AROMA in the owner's dish sheet (W-SENSE row 1), and the compact
// line on the menu's rows (row 2). Six taste axes 0..5, textures and aromas 1..3, all optional:
// an axis nobody touched is not declared, and the storefront draws nothing for it. "Suggest" asks
// `POST /owner/products/:id/sense/suggest` for a DRAFT from the dish's own words (and, for the
// common sushi dishes, the starter defaults); the draft fills the editor and is saved only by the
// ordinary Save, after the owner looked at it. Replaces the old five-axis, three-level editor:
// a dish that still has the old `taste` opens with it read on the new scale (x5/3).
//
// ASCII QUOTES ONLY as string delimiters.

import { $, $$, post, withLoc, toast, retranslate } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import * as ui from '/lib/ui/index.js';
import { SENSE_WORDS, wordKey } from '/lib/sense-words.js';
import { RECIPE_WORDS, recipeMarkup, fillEmpty } from '/admin/sense-recipe-view.js';
import { senseOf, TASTE, TEXTURE, AROMA, TASTE_MAX, TAG_MAX } from '/store/sense.js';
import { cardSense } from '/store/sense-view.js';

const WORDS = {
  sq: { sxEdit: 'Shija, struktura dhe aroma', sxEditHint: 'Gjithçka me dëshirë. Prekni një nivel; prekeni sërish për ta hequr. Çfarë nuk prekni nuk shfaqet te klienti.',
        sxSuggest: 'Sugjero nga përshkrimi', sxSuggested: 'Draft: kontrollojeni dhe ruajeni. Asgjë nuk u ruajt ende.', sxSuggestNone: 'Asnjë fjalë e njohur në përshkrim.', sxLevel: 'Niveli', sxSuggestedAi: 'Draft nga fjalët dhe modeli juaj AI: kontrollojeni dhe ruajeni. Asgjë nuk u ruajt ende.' },
  en: { sxEdit: 'Taste, texture and aroma', sxEditHint: 'All optional. Tap a level; tap it again to clear. What you leave untouched is not shown to guests.',
        sxSuggest: 'Suggest from the description', sxSuggested: 'A draft: check it, then save. Nothing was saved yet.', sxSuggestNone: 'No word in the description it knows.', sxLevel: 'Level', sxSuggestedAi: 'A draft from the words and your AI model: check it, then save. Nothing was saved yet.' },
  uk: { sxEdit: 'Смак, текстура й аромат', sxEditHint: 'Усе за бажанням. Торкніться рівня; торкніться ще раз, щоб зняти. Чого не торкалися, гостям не показується.',
        sxSuggest: 'Підказати з опису', sxSuggested: 'Чернетка: перевірте й збережіть. Нічого ще не збережено.', sxSuggestNone: 'В описі немає знайомих слів.', sxLevel: 'Рівень', sxSuggestedAi: 'Чернетка зі слів і вашої AI-моделі: перевірте й збережіть. Нічого ще не збережено.' },
  ru: { sxEdit: 'Вкус, текстура и аромат', sxEditHint: 'Всё по желанию. Нажмите уровень; нажмите ещё раз, чтобы снять. Что не трогали, гостям не показывается.',
        sxSuggest: 'Подсказать из описания', sxSuggested: 'Черновик: проверьте и сохраните. Ничего ещё не сохранено.', sxSuggestNone: 'В описании нет знакомых слов.', sxLevel: 'Уровень', sxSuggestedAi: 'Черновик из слов и вашей AI-модели: проверьте и сохраните. Ничего ещё не сохранено.' },
};
for (const l of LANGS) Object.assign(T[l], SENSE_WORDS[l], WORDS[l], RECIPE_WORDS[l]);

let draft = { taste: {}, texture: {}, aroma: {} };
let start = '';
const snap = () => JSON.stringify(draft);

const levels = (dim, id, max, from) => Array.from({ length: max - from + 1 }, (_, i) => i + from)
  .map(n => ui.chip({ as: 'button', selected: draft[dim][id] === n, label: String(n), ariaLabel: `${n}/${max}`, attrs: { data: { sxd: dim, sxi: id, sxn: n, tour: 'sense.' + dim } } })).join('');

function body(){
  const rows = (dim, words, pre, max, from) => words.map(id => `<div class="taste-row"><span class="taste-ax"><span data-t="${wordKey(`${pre}:${id}`)}"></span></span>
    <span class="taste-lv">${levels(dim, id, max, from)}</span></div>`).join('');
  return `<p class="eyebrow mt-2" data-t="sx_taste"></p>${rows('taste', TASTE, 't', TASTE_MAX, 0)}
    <p class="eyebrow mt-2" data-t="sx_texture"></p>${rows('texture', TEXTURE, 'x', TAG_MAX, 1)}
    <p class="eyebrow mt-2" data-t="sx_aroma"></p>${rows('aroma', AROMA, 'a', TAG_MAX, 1)}`;
}

/// The editor in the dish sheet. Opens with what the dish declares (the old field read on the new scale).
export function senseEditor(p){
  const s = senseOf(p) || { taste: {}, texture: {}, aroma: {} };
  draft = { taste: { ...s.taste }, texture: { ...s.texture }, aroma: { ...s.aroma } };
  start = snap();
  return `<p class="eyebrow mt-3" data-t="sxEdit"></p><p class="muted small" data-t="sxEditHint"></p>
    ${ui.button({ variant: 'ghost', icon: 'sparkles', label: { t: 'sxSuggest' }, id: 'sxSuggest', attrs: { data: { tour: 'sense.suggest' } } })}
    <p class="small muted" id="sxNote" hidden></p>
    <div id="sxRecipe" hidden></div>
    <div class="taste" id="sxBox">${body()}</div>`;
}

function redraw(){ const b = $('#sxBox'); if (!b) return; b.innerHTML = body(); retranslate(b); bindLevels(); }
function bindLevels(){
  for (const b of $$('[data-sxd]', $('#sxBox'))) b.onclick = () => {
    const { sxd: dim, sxi: id } = b.dataset, n = +b.dataset.sxn;
    if (draft[dim][id] === n) delete draft[dim][id]; else draft[dim][id] = n; // tapped again: not declared
    for (const x of $$(`[data-sxd="${dim}"][data-sxi="${id}"]`, $('#sxBox'))) x.setAttribute('aria-pressed', String(draft[dim][id] === +x.dataset.sxn));
  };
}

export function bindSenseEditor(p){
  bindLevels();
  const sg = $('#sxSuggest'); if (!sg) return;
  sg.onclick = async () => {
    try {
      const d = await post(`/owner/products/${encodeURIComponent(p.id)}/sense/suggest`, withLoc({}));
      const got = d?.draft;
      const note = $('#sxNote');
      // W-TASTE2 S7b: the recipe's draft, beside this one; "Use it" fills only what is empty.
      const rb = $('#sxRecipe'), rh = recipeMarkup(d?.recipe);
      if (rb) { rb.innerHTML = rh; rb.hidden = !rh; retranslate(rb); const u = $('#srUse'); if (u) u.onclick = () => { draft = fillEmpty(draft, d.recipe); redraw(); }; }
      if (!got) { note.hidden = false; note.dataset.t = 'sxSuggestNone'; retranslate(note.parentElement); return; }
      // The draft fills what is empty; what the owner already set stays theirs.
      for (const dim of ['taste', 'texture', 'aroma']) for (const [id, n] of Object.entries(got[dim] || {})) if (!(id in draft[dim])) draft[dim][id] = n;
      // W-TASTE: say when the venue's AI model added to the draft (it is held to the vocabulary server-side).
      note.hidden = false; note.dataset.t = d.source === 'lexicon+model' ? 'sxSuggestedAi' : 'sxSuggested'; retranslate(note.parentElement);
      redraw();
    } catch (e) { toast(String(e.message || e)); }
  };
}

/// What the Save sends: `sense` only when the owner changed it.
export const senseEdits = () => (snap() === start ? {} : { sense: { v: 1, ...draft } });

/// The menu row's compact line: the declared taste bars and two chips, or ''.
export const rowSense = p => cardSense(p);
