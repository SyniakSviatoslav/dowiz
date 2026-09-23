// The customer CARD: what the venue would otherwise write on a paper card by
// the till (§3.1 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22). A note, tags
// from a CLOSED list, EU-14 allergens, the usual table, a language and a
// birthday with no year. Everything else about a person -- orders, spend,
// last visit, name, phone -- is a fold over the orders and is never typed in.
//
// THE CLOSED LISTS ARE THE HUB'S (`services/customers/record.rs`), repeated
// here only to draw the chips; the hub refuses anything else with a reason.

import { $, $$, esc, icon, t, api, toast, sheet, busy, hydrate } from '/admin/core.js';
import { T, LANGS, retranslate } from '/admin/i18n.js';

/// `record.rs::TAGS`. A tag is not a tier: nothing here ranks a person.
const TAGS = ['regular', 'office_lunch', 'family', 'group', 'takeaway', 'delivery', 'tourist', 'event'];
/// `dowiz_hub::allergens::EU14`, in the regulation's order.
const EU14 = ['gluten', 'crustaceans', 'eggs', 'fish', 'peanuts', 'soy', 'milk', 'nuts', 'celery', 'mustard', 'sesame', 'sulphites', 'lupin', 'molluscs'];
/// `record.rs::NOTE_MAX_CHARS` and `TABLE_MAX_CHARS`.
const NOTE_MAX = 280, TABLE_MAX = 16;

const WORDS = {
  sq: { card: 'Kartela', cardHint: 'Vetëm çfarë do shkruanit në një kartelë letre. Shënimi mund t’i tregohet klientit nëse e kërkon.', cardNote: 'Shënim', cardTags: 'Etiketa', cardAllergens: 'Alergji (porosia refuzohet për pjatat që i përmbajnë)', cardTable: 'Tavolina e zakonshme', cardLang: 'Gjuha', cardBirthday: 'Ditëlindja (MM-DD, pa vit)', cardSave: 'Ruaj kartelën', cardSaved: 'Kartela u ruajt',
        tag_regular: 'i rregullt', tag_office_lunch: 'drekë zyre', tag_family: 'familje', tag_group: 'grup', tag_takeaway: 'me vete', tag_delivery: 'dërgesë', tag_tourist: 'turist', tag_event: 'eveniment',
        al_gluten: 'Gluten', al_crustaceans: 'Guaskorë', al_eggs: 'Vezë', al_fish: 'Peshk', al_peanuts: 'Kikirikë', al_soy: 'Soja', al_milk: 'Qumësht', al_nuts: 'Arra', al_celery: 'Selino', al_mustard: 'Mustardë', al_sesame: 'Susam', al_sulphites: 'Sulfite', al_lupin: 'Lupin', al_molluscs: 'Molusqe' },
  en: { card: 'Card', cardHint: 'Only what you would write on a paper card. The note can be shown to the customer if they ask.', cardNote: 'Note', cardTags: 'Tags', cardAllergens: 'Allergies (orders with dishes that contain them are refused)', cardTable: 'Usual table', cardLang: 'Language', cardBirthday: 'Birthday (MM-DD, no year)', cardSave: 'Save card', cardSaved: 'Card saved',
        tag_regular: 'regular', tag_office_lunch: 'office lunch', tag_family: 'family', tag_group: 'group', tag_takeaway: 'takeaway', tag_delivery: 'delivery', tag_tourist: 'tourist', tag_event: 'event',
        al_gluten: 'Gluten', al_crustaceans: 'Crustaceans', al_eggs: 'Eggs', al_fish: 'Fish', al_peanuts: 'Peanuts', al_soy: 'Soy', al_milk: 'Milk', al_nuts: 'Nuts', al_celery: 'Celery', al_mustard: 'Mustard', al_sesame: 'Sesame', al_sulphites: 'Sulphites', al_lupin: 'Lupin', al_molluscs: 'Molluscs' },
  uk: { card: 'Картка', cardHint: 'Лише те, що ви записали б на паперовій картці. Нотатку можна показати клієнтові на його прохання.', cardNote: 'Нотатка', cardTags: 'Мітки', cardAllergens: 'Алергії (замовлення зі стравами, що їх містять, відхиляється)', cardTable: 'Звичний стіл', cardLang: 'Мова', cardBirthday: 'День народження (ММ-ДД, без року)', cardSave: 'Зберегти картку', cardSaved: 'Картку збережено',
        tag_regular: 'постійний', tag_office_lunch: 'офісний обід', tag_family: 'сім’я', tag_group: 'група', tag_takeaway: 'з собою', tag_delivery: 'доставка', tag_tourist: 'турист', tag_event: 'подія',
        al_gluten: 'Глютен', al_crustaceans: 'Ракоподібні', al_eggs: 'Яйця', al_fish: 'Риба', al_peanuts: 'Арахіс', al_soy: 'Соя', al_milk: 'Молоко', al_nuts: 'Горіхи', al_celery: 'Селера', al_mustard: 'Гірчиця', al_sesame: 'Кунжут', al_sulphites: 'Сульфіти', al_lupin: 'Люпин', al_molluscs: 'Молюски' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// The card's fields, as one line under the row. Empty when there is no card.
export function cardLine(c){
  const bits = [];
  if (c.allergens?.length) bits.push('⚠ ' + c.allergens.map(a => t('al_' + a)).join(', '));
  if (c.tags?.length) bits.push(c.tags.map(x => t('tag_' + x)).join(', '));
  if (c.note) bits.push(c.note);
  return bits.join(' · ');
}

const chips = (name, all, on, label) => `<div class="chips" role="group" aria-label="${esc(t(name))}">${all.map(v =>
  `<label class="chip"><input type="checkbox" data-${name}="${esc(v)}" ${on.includes(v) ? 'checked' : ''}><span>${esc(t(label + v))}</span></label>`).join('')}</div>`;

/// Open one customer's card. `c` is the row from `/owner/customers`; `done`
/// is called with the saved record so the list can repaint without a refetch.
export function openCard(c, done){
  sheet(`<p class="eyebrow" data-t="customers"></p><h2 data-t="card"></h2><p class="muted small" data-t="cardHint"></p>
    <label for="cd-note" data-t="cardNote"></label><textarea id="cd-note" maxlength="${NOTE_MAX}" rows="3">${esc(c.note || '')}</textarea>
    <p class="eyebrow mt-3" data-t="cardAllergens"></p>${chips('al', EU14, c.allergens || [], 'al_')}
    <p class="eyebrow mt-3" data-t="cardTags"></p>${chips('tag', TAGS, c.tags || [], 'tag_')}
    <label for="cd-table" data-t="cardTable"></label><input id="cd-table" maxlength="${TABLE_MAX}" value="${esc(c.usualTable || '')}" autocomplete="off">
    <label for="cd-lang" data-t="cardLang"></label><select id="cd-lang"><option value=""></option>${LANGS.map(l => `<option value="${l}" ${c.lang === l ? 'selected' : ''}>${l}</option>`).join('')}</select>
    <label for="cd-bd" data-t="cardBirthday"></label><input id="cd-bd" inputmode="numeric" placeholder="MM-DD" maxlength="5" value="${esc(c.birthdayMd || '')}" autocomplete="off">
    <div class="btn-row"><button class="btn" id="cdGo">${icon('check')}<span data-t="cardSave"></span></button></div>`, { name: 'card' });
  retranslate($('#sheetIn')); hydrate($('#sheetIn'));
  $('#cdGo').onclick = async () => {
    const picked = k => $$(`[data-${k}]`, $('#sheetIn')).filter(i => i.checked).map(i => i.dataset[k]);
    const body = { note: $('#cd-note').value, tags: picked('tag'), allergens: picked('al'),
                   usualTable: $('#cd-table').value, lang: $('#cd-lang').value, birthdayMd: $('#cd-bd').value.trim() };
    try {
      const r = await busy($('#cdGo'), () => api(`/owner/customers/${encodeURIComponent(c.key)}/record`, { method: 'PUT', body }));
      toast(t('cardSaved'));
      done?.(r.record || {});
    } catch (e) { toast(String(e.message || e)); }
  };
}
