// The menu's claims in the owner console (W-MR0, 2026-10-04): the tag the owner sets says what
// it is ("Venue's pick"), the dishes nobody has declared allergens for are counted and listed on the
// menu screen, the dish sheet has the fourteen to declare, and the analytics pane shows the same
// "most ordered this week" number the storefront badge shows (services/analytics/week_top.rs).
//
// ASCII QUOTES ONLY as string delimiters (admin/ingredients-i18n.js says why).

import { $, $$, esc, icon, t, S, retranslate } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import { ui, k, btn, rowBtn, rowDiv, pill } from '/admin/parts.js';

/// `dowiz_hub::allergens::EU14`, in the regulation's order (also admin/customers.js).
export const EU14 = ['gluten', 'crustaceans', 'eggs', 'fish', 'peanuts', 'soy', 'milk', 'nuts', 'celery', 'mustard', 'sesame', 'sulphites', 'lupin', 'molluscs'];

/// The allergen names, the same words admin/customers.js registers (that screen may not be loaded).
const NAMES = {
  sq: { al_gluten: 'Gluten', al_crustaceans: 'Guaskorë', al_eggs: 'Vezë', al_fish: 'Peshk', al_peanuts: 'Kikirikë', al_soy: 'Soja', al_milk: 'Qumësht', al_nuts: 'Arra', al_celery: 'Selino', al_mustard: 'Mustardë', al_sesame: 'Susam', al_sulphites: 'Sulfite', al_lupin: 'Lupin', al_molluscs: 'Molusqe' },
  en: { al_gluten: 'Gluten', al_crustaceans: 'Crustaceans', al_eggs: 'Eggs', al_fish: 'Fish', al_peanuts: 'Peanuts', al_soy: 'Soy', al_milk: 'Milk', al_nuts: 'Nuts', al_celery: 'Celery', al_mustard: 'Mustard', al_sesame: 'Sesame', al_sulphites: 'Sulphites', al_lupin: 'Lupin', al_molluscs: 'Molluscs' },
  uk: { al_gluten: 'Глютен', al_crustaceans: 'Ракоподібні', al_eggs: 'Яйця', al_fish: 'Риба', al_peanuts: 'Арахіс', al_soy: 'Соя', al_milk: 'Молоко', al_nuts: 'Горіхи', al_celery: 'Селера', al_mustard: 'Гірчиця', al_sesame: 'Кунжут', al_sulphites: 'Сульфіти', al_lupin: 'Люпин', al_molluscs: 'Молюски' },
  ru: { al_gluten: 'Глютен', al_crustaceans: 'Ракообразные', al_eggs: 'Яйца', al_fish: 'Рыба', al_peanuts: 'Арахис', al_soy: 'Соя', al_milk: 'Молоко', al_nuts: 'Орехи', al_celery: 'Сельдерей', al_mustard: 'Горчица', al_sesame: 'Кунжут', al_sulphites: 'Сульфиты', al_lupin: 'Люпин', al_molluscs: 'Моллюски' },
};
const WORDS = {
  sq: { mtag_popular: 'Zgjedhja e lokalit', mtag_salmon: 'Salmon', mtag_tuna: 'Ton', mtag_shrimp: 'Karkalec', mtag_vegetarian: 'Vegjetariane', mtag_hot: 'E nxehtë', mf_tagsHint: '“Zgjedhja e lokalit” është fjala juaj; “Më e porositura këtë javë” numërohet vetë nga porositë.', mf_undeclared: 'pjata pa alergjenë të deklaruar', mf_allDeclared: 'Çdo pjatë i ka alergjenët e deklaruar.', mf_gateOn: 'Klientët i shohin alergjenët: një pjatë pa deklaratë nuk del në shitje.', mf_gateOff: 'Filtri i alergjenëve është i fikur në vitrinë: klientët nuk i shohin, por deklarata vlen.', mf_allergens: 'Alergjenët (14 të BE-së)', mf_alHint: 'Prekni çfarë përmban pjata. “Asnjë nga 14” është përgjigje; një fushë e paprekur nuk është.', mf_notDeclared: 'Nuk janë deklaruar', mf_noneOf14: 'Asnjë nga 14 alergjenët', mf_declared: 'Të deklaruar', mf_markNone: 'Asnjë nga 14', mf_needDeclare: 'Deklaroni alergjenët para se ta vini në shitje (“Asnjë nga 14” vlen).', mf_heldUndeclared: 'Pjata u krijua jashtë shitjes: deklaroni alergjenët për ta vënë në shitje.', mf_week: 'Më e porositura këtë javë (shenja në vitrinë)', mf_weekHint: '7 ditët e fundit, vetëm porositë e pranuara; porositë TEST lihen jashtë. Shenja shfaqet nga', mf_weekTest: 'test', mf_weekShown: 'shenjë', mf_weekNone: 'Asnjë porosi këtë javë.' },
  en: { mtag_popular: 'Venue’s pick', mtag_salmon: 'Salmon', mtag_tuna: 'Tuna', mtag_shrimp: 'Shrimp', mtag_vegetarian: 'Vegetarian', mtag_hot: 'Hot', mf_tagsHint: '“Venue’s pick” is your word; “Most ordered this week” is counted from the orders by itself.', mf_undeclared: 'dishes without declared allergens', mf_allDeclared: 'Every dish has its allergens declared.', mf_gateOn: 'Guests see allergens: an undeclared dish cannot go on sale.', mf_gateOff: 'The allergen filter is off on your storefront: guests do not see them, but the declaration still counts.', mf_allergens: 'Allergens (the EU 14)', mf_alHint: 'Tap what the dish contains. “None of the 14” is an answer; an untouched field is not.', mf_notDeclared: 'Not declared', mf_noneOf14: 'None of the 14 allergens', mf_declared: 'Declared', mf_markNone: 'None of the 14', mf_needDeclare: 'Declare the allergens before putting it on sale (“None of the 14” counts).', mf_heldUndeclared: 'The dish was created off sale: declare its allergens to put it on sale.', mf_week: 'Most ordered this week (the storefront badge)', mf_weekHint: 'The last 7 days, taken orders only; TEST orders are left out. The badge shows from', mf_weekTest: 'test', mf_weekShown: 'badge', mf_weekNone: 'No orders this week.' },
  uk: { mtag_popular: 'Вибір закладу', mtag_salmon: 'Лосось', mtag_tuna: 'Тунець', mtag_shrimp: 'Креветка', mtag_vegetarian: 'Вегетаріанське', mtag_hot: 'Гаряче', mf_tagsHint: '«Вибір закладу» — ваше слово; «Найчастіше замовляють цього тижня» рахується із замовлень само.', mf_undeclared: 'страв без заявлених алергенів', mf_allDeclared: 'У кожної страви алергени заявлено.', mf_gateOn: 'Гості бачать алергени: страва без заяви не потрапить у продаж.', mf_gateOff: 'Фільтр алергенів на вітрині вимкнено: гості їх не бачать, але заява все одно важлива.', mf_allergens: 'Алергени (14 ЄС)', mf_alHint: 'Позначте, що містить страва. «Жодного з 14» — це відповідь; незаповнене поле — ні.', mf_notDeclared: 'Не заявлено', mf_noneOf14: 'Жодного з 14 алергенів', mf_declared: 'Заявлено', mf_markNone: 'Жодного з 14', mf_needDeclare: 'Заявіть алергени, перш ніж ставити страву в продаж («Жодного з 14» теж відповідь).', mf_heldUndeclared: 'Страву створено поза продажем: заявіть алергени, щоб поставити її в продаж.', mf_week: 'Найчастіше замовляють цього тижня (позначка на вітрині)', mf_weekHint: 'Останні 7 днів, лише прийняті замовлення; TEST-замовлення не рахуються. Позначка з', mf_weekTest: 'тест', mf_weekShown: 'позначка', mf_weekNone: 'Цього тижня замовлень немає.' },
  ru: { mtag_popular: 'Выбор заведения', mtag_salmon: 'Лосось', mtag_tuna: 'Тунец', mtag_shrimp: 'Креветка', mtag_vegetarian: 'Вегетарианское', mtag_hot: 'Горячее', mf_tagsHint: '«Выбор заведения» — ваше слово; «Чаще всего заказывают на этой неделе» считается по заказам само.', mf_undeclared: 'блюд без заявленных аллергенов', mf_allDeclared: 'У каждого блюда аллергены заявлены.', mf_gateOn: 'Гости видят аллергены: блюдо без заявления не попадёт в продажу.', mf_gateOff: 'Фильтр аллергенов на витрине выключен: гости их не видят, но заявление всё равно важно.', mf_allergens: 'Аллергены (14 ЕС)', mf_alHint: 'Отметьте, что содержит блюдо. «Ни одного из 14» — это ответ; незаполненное поле — нет.', mf_notDeclared: 'Не заявлено', mf_noneOf14: 'Ни одного из 14 аллергенов', mf_declared: 'Заявлено', mf_markNone: 'Ни одного из 14', mf_needDeclare: 'Заявите аллергены, прежде чем ставить блюдо в продажу («Ни одного из 14» тоже ответ).', mf_heldUndeclared: 'Блюдо создано вне продажи: заявите аллергены, чтобы поставить его в продажу.', mf_week: 'Чаще всего заказывают на этой неделе (значок на витрине)', mf_weekHint: 'Последние 7 дней, только принятые заказы; TEST-заказы не считаются. Значок с', mf_weekTest: 'тест', mf_weekShown: 'значок', mf_weekNone: 'На этой неделе заказов нет.' },
};
for (const l of LANGS) Object.assign(T[l], NAMES[l], WORDS[l]);

// ── tags ────────────────────────────────────────────────────────────────────
/// A tag's label in the owner's language; a tag the console has no word for shows as written.
export const tagLabel = tg => { const w = t('mtag_' + tg); return w === 'mtag_' + tg ? tg : w; };

// ── allergens: the menu screen's count and list ────────────────────────────
/// The venue's storefront shows allergens (and the hub refuses sale of an undeclared dish) unless
/// the owner switched the filter off: the hub's `features::is_on`, default on.
export const gateOn = () => S.venue?.features?.allergen_filter !== false;
/// The hub's three states: an absent list is UNDECLARED (`crates/dowiz-hub/src/allergens.rs`).
export const isDeclared = p => Array.isArray(p?.allergens);
export const undeclared = products => (products || []).filter(p => !isDeclared(p));

/// The count and, folded under it, every undeclared dish; a row opens the dish (`[data-p]`).
export function undeclaredPanel(products){
  const list = undeclared(products);
  if (!list.length) return `<p class="hint ok" id="mfUndeclared" data-t="mf_allDeclared"></p>`;
  return `<details class="fold" id="mfUndeclared" data-tour="menu.undeclared"><summary>${icon('alert-triangle')} <b class="mono" id="mfUndeclaredN">${list.length}</b> <span data-t="mf_undeclared"></span></summary>
    <p class="hint" data-t="${gateOn() ? 'mf_gateOn' : 'mf_gateOff'}"></p>
    <div class="rows">${list.map(p => rowBtn({ data: { p: p.id }, tour: 'menu.undeclaredDish', title: p.name, sub: esc(p.categoryName || ''),
      trailing: pill(p.available ? 'warn' : 'bad', { key: p.available ? 'onSale' : 'stopList' }) })).join('')}</div></details>`;
}

// ── allergens: the dish sheet ──────────────────────────────────────────────
/// What this sheet will send: `undefined` = untouched (nothing sent), `[]` = "none of the 14",
/// a list = what the dish contains. Emptying the chips is NOT "none": only the button says that.
let draft;
let initial = null;
const stateKey = list => (list == null ? 'mf_notDeclared' : list.length ? 'mf_declared' : 'mf_noneOf14');

export function allergenMarkup(p){
  initial = isDeclared(p) ? [...p.allergens] : null;
  draft = undefined;
  return `<p class="ui-label" data-t="mf_allergens"></p><p class="hint" data-t="mf_alHint"></p>
    <p class="hint ${initial ? '' : 'warn'}" id="alState" data-t="${stateKey(initial)}"></p>
    <div class="chips" id="alPick" role="group">${EU14.map(c => ui.chip({ as: 'button', selected: !!initial?.includes(c), label: k('al_' + c), attrs: { data: { al: c, tour: 'dish.allergen' } } })).join('')}</div>
    <div class="btn-row">${btn({ id: 'alNone', variant: 'ghost', icon: 'check', key: 'mf_markNone', tour: 'dish.allergenNone' })}</div>`;
}

export function bindAllergens(){
  const box = $('#alPick'); if (!box) return;
  const paint = () => {
    const now = draft === undefined ? initial : draft;
    const el = $('#alState'); el.dataset.t = stateKey(now); el.classList.toggle('warn', now == null); retranslate(el.parentNode);
  };
  for (const b of $$('[data-al]', box)) b.onclick = () => {
    b.setAttribute('aria-pressed', String(b.getAttribute('aria-pressed') !== 'true'));
    const picked = $$('[data-al][aria-pressed="true"]', box).map(x => x.dataset.al);
    draft = picked.length ? picked : undefined;
    paint();
  };
  $('#alNone').onclick = () => {
    for (const b of $$('[data-al]', box)) b.setAttribute('aria-pressed', 'false');
    draft = [];
    paint();
  };
}

/// The `allergens` field for the save body: present only when the owner declared something here.
export const allergenEdits = () => (draft === undefined ? {} : { allergens: draft });

/// Would this save put an undeclared dish on sale where the hub refuses it? The console says why
/// BEFORE the round trip; the hub's 409 stays the authority.
export const refusesSale = (p, onSale) => gateOn() && onSale && (draft === undefined ? initial : draft) == null;

// ── analytics: the storefront badge's own number ───────────────────────────
/// `weekTop` of `GET /api/owner/analytics?v=2` (contract `menu.week-top.v1`).
export function weekTopCard(w){
  if (!w || !Array.isArray(w.dishes)) return '';
  const rows = w.dishes.slice(0, 12).map(d => rowDiv({ leading: icon(d.badge ? 'flame' : 'chart-bar'), title: typeof d.name === 'string' ? d.name : d.id,
    sub: `<span class="mono">${esc(d.n)}${d.test ? ` · ${esc(d.test)} <span data-t="mf_weekTest"></span>` : ''}</span>`,
    trailing: d.badge ? pill('ok', { key: 'mf_weekShown' }) : '' })).join('');
  return `<section class="group mt-3" id="anWeekTop" data-tour="analytics.weekTop"><p class="eyebrow" data-t="mf_week"></p>
    <p class="muted small"><span data-t="mf_weekHint"></span> <b class="mono">${esc(w.threshold)}</b> · ${esc(w.from)} - ${esc(w.to)}</p>
    <div class="rows" role="list">${rows || `<p class="hint" data-t="mf_weekNone"></p>`}</div></section>`;
}
