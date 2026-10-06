// THE OWNER'S SEGMENT BUILDER and a PERSONALISED OFFER to it (W-SENSE row 7), above the customers
// list. "Loves smoky, not seen 4+ days, orders in the evening": a count, a six-month trend, where in
// the week they order, and what to stock (the dishes that carry the taste and their supplies).
// Never a list of people. "Make an offer" files a CAMPAIGN (segment `taste`): the server writes
// its words from the dish's public price, labelled "Personalised offer", with no code; it reaches
// only guests who gave marketing consent on its channel, and is sent from the campaigns screen
// (template, preview, send) like any other.
//
// ASCII QUOTES ONLY as string delimiters.

import { $, $$, api, post, toast, retranslate, esc } from '/admin/core.js';
import { T, LANGS, lang } from '/admin/i18n.js';
import * as ui from '/lib/ui/index.js';
import '/admin/sense.js';
import { wordKey } from '/lib/sense-words.js';
import { TASTE, TEXTURE, AROMA } from '/store/sense.js';
import { weatherSourceLine } from '/store/sense-view.js';

const WORDS = {
  sq: { sbTitle: 'Segment sipas shijes', sbKey: 'Shija', sbMin: 'Sa fort (nga 1000)', sbNotSeen: 'Pa ardhur të paktën (ditë)', sbOrders: 'Të paktën porosi',
        sbBand: 'Zakonisht porosit', sbWeather: 'Ka porositur në mot', sbAny: 'Çdo', sbCount: 'Klientë', sbOf: 'nga', sbTrend: 'Gjashtë muajt e fundit', sbBusiest: 'Më aktiv',
        sbPlan: 'Çfarë të keni gati', sbDishes: 'Pjatat', sbSupplies: 'Furnizimet', sbOffer: 'Bëj një ofertë', sbOfferDish: 'Pjata e ofertës', sbOfferLang: 'Gjuha',
        sbOfferNote: 'Teksti shkruhet nga sistemi: \'Ofertë e personalizuar\', pjata dhe çmimi i menysë. Pa kod zbritjeje. Shkon vetëm te ata që pranuan mesazhe në atë kanal; dërgohet nga ekrani i fushatave.',
        sbOfferDone: 'Oferta u ruajt si fushatë', sbNever: 'Kurrë çmim tjetër, kurrë refuzim. Alergjenet nuk zgjidhen këtu.',
        band_morning: 'Mëngjes', band_midday: 'Drekë', band_afternoon: 'Pasdite', band_evening: 'Mbrëmje', band_night: 'Natë', wx_rain: 'Shi', wx_cold: 'Ftohtë', wx_hot: 'Nxehtë', wx_mild: 'Butë' },
  en: { sbTitle: 'Segment by taste', sbKey: 'Taste', sbMin: 'How strongly (of 1000)', sbNotSeen: 'Not seen for at least (days)', sbOrders: 'At least orders',
        sbBand: 'Usually orders in the', sbWeather: 'Has ordered in', sbAny: 'Any', sbCount: 'Guests', sbOf: 'of', sbTrend: 'The last six months', sbBusiest: 'Busiest',
        sbPlan: 'What to have ready', sbDishes: 'Dishes', sbSupplies: 'Supplies', sbOffer: 'Make an offer', sbOfferDish: 'Dish to offer', sbOfferLang: 'Language',
        sbOfferNote: 'The words are written by the system: \'Personalised offer\', the dish and its menu price. No discount code. It reaches only guests who agreed to messages on that channel; send it from the campaigns screen.',
        sbOfferDone: 'The offer was saved as a campaign', sbNever: 'Never another price, never a refusal. Allergens cannot be chosen here.',
        band_morning: 'Morning', band_midday: 'Midday', band_afternoon: 'Afternoon', band_evening: 'Evening', band_night: 'Night', wx_rain: 'Rain', wx_cold: 'Cold', wx_hot: 'Hot', wx_mild: 'Mild' },
  uk: { sbTitle: 'Сегмент за смаком', sbKey: 'Смак', sbMin: 'Наскільки сильно (з 1000)', sbNotSeen: 'Не приходив щонайменше (днів)', sbOrders: 'Щонайменше замовлень',
        sbBand: 'Зазвичай замовляє', sbWeather: 'Замовляв у погоду', sbAny: 'Будь-коли', sbCount: 'Гостей', sbOf: 'з', sbTrend: 'Останні шість місяців', sbBusiest: 'Найактивніше',
        sbPlan: 'Що мати напоготові', sbDishes: 'Страви', sbSupplies: 'Продукти', sbOffer: 'Зробити пропозицію', sbOfferDish: 'Страва для пропозиції', sbOfferLang: 'Мова',
        sbOfferNote: 'Текст пише система: «Персоналізована пропозиція», страва й ціна з меню. Без коду знижки. Дійде лише до тих, хто погодився на повідомлення в цьому каналі; надсилається з екрана кампаній.',
        sbOfferDone: 'Пропозицію збережено як кампанію', sbNever: 'Ніколи інша ціна, ніколи відмова. Алергени тут обрати не можна.',
        band_morning: 'Ранок', band_midday: 'Обід', band_afternoon: 'Після обіду', band_evening: 'Вечір', band_night: 'Ніч', wx_rain: 'Дощ', wx_cold: 'Холод', wx_hot: 'Спека', wx_mild: 'Помірно' },
  ru: { sbTitle: 'Сегмент по вкусу', sbKey: 'Вкус', sbMin: 'Насколько сильно (из 1000)', sbNotSeen: 'Не приходил минимум (дней)', sbOrders: 'Минимум заказов',
        sbBand: 'Обычно заказывает', sbWeather: 'Заказывал в погоду', sbAny: 'Любое', sbCount: 'Гостей', sbOf: 'из', sbTrend: 'Последние шесть месяцев', sbBusiest: 'Активнее всего',
        sbPlan: 'Что держать наготове', sbDishes: 'Блюда', sbSupplies: 'Продукты', sbOffer: 'Сделать предложение', sbOfferDish: 'Блюдо для предложения', sbOfferLang: 'Язык',
        sbOfferNote: 'Текст пишет система: «Персонализированное предложение», блюдо и цена из меню. Без кода скидки. Дойдёт только до тех, кто согласился на сообщения в этом канале; отправляется с экрана кампаний.',
        sbOfferDone: 'Предложение сохранено как кампания', sbNever: 'Никогда другая цена, никогда отказ. Аллергены здесь выбрать нельзя.',
        band_morning: 'Утро', band_midday: 'Обед', band_afternoon: 'После обеда', band_evening: 'Вечер', band_night: 'Ночь', wx_rain: 'Дождь', wx_cold: 'Холод', wx_hot: 'Жара', wx_mild: 'Умеренно' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

const KEYS = [...TASTE.map(x => 't:' + x), ...TEXTURE.map(x => 'x:' + x), ...AROMA.map(x => 'a:' + x)];
const BANDS = ['morning', 'midday', 'afternoon', 'evening', 'night'];
const WX = ['rain', 'cold', 'hot', 'mild'];

/// PURE. The query string of a filter (`key` always; the rest only when set).
export function queryOf(f){
  const q = new URLSearchParams({ key: f.key });
  for (const k of ['min', 'not_seen_days', 'min_orders', 'band', 'weather']) if (f[k] !== '' && f[k] != null) q.set(k, String(f[k]));
  return q.toString();
}

/// PURE. The answer of `GET /owner/customers/taste/builder` as markup.
export function reportMarkup(d){
  if (!d) return '';
  const trend = (d.trend || []).map(r => `<span class="mono">${esc(r.month)} <b>${Number(r.count) | 0}</b></span>`).join(' · ');
  const plan = d.plan || {};
  return `<p class="small"><span data-t="sbCount"></span>: <b class="mono">${Number(d.count) | 0}</b> <span data-t="sbOf"></span> <span class="mono">${Number(d.of) | 0}</span></p>
    <p class="small muted"><span data-t="sbTrend"></span>: ${trend || '-'}</p>
    ${d.busiest ? `<p class="small"><span data-t="sbBusiest"></span>: <b class="mono">${esc(d.busiest)}</b></p>` : ''}
    ${(plan.dishes || []).length ? `<p class="small"><span data-t="sbPlan"></span> — <span data-t="sbDishes"></span>: ${plan.dishes.map(x => esc(x.name || x.id)).join(', ')}${(plan.supplies || []).length ? `; <span data-t="sbSupplies"></span>: ${plan.supplies.map(esc).join(', ')}` : ''}</p>` : ''}`;
}

const sel = (id, opts, tour) => `<select class="ui-input" id="${id}" data-tour="${tour}">${opts}</select>`;
const opt = (v, key, on) => `<option value="${esc(v)}"${on ? ' selected' : ''} data-t="${key}"></option>`;

function formMarkup(){
  return `<details class="sb" data-tour="customers.senseBuilder"><summary class="eyebrow mt-3" data-t="sbTitle"></summary>
    <div class="sb-form">
      <label><span data-t="sbKey"></span> ${sel('sbKey', KEYS.map((k, i) => opt(k, wordKey(k), i === KEYS.indexOf('a:smoky'))).join(''), 'builder.key')}</label>
      ${ui.field({ id: 'sbMin', type: 'number', label: { t: 'sbMin' }, value: 500, attrs: { min: 1, max: 1000, step: 50 } })}
      ${ui.field({ id: 'sbNotSeen', type: 'number', label: { t: 'sbNotSeen' }, attrs: { min: 1, max: 365 } })}
      ${ui.field({ id: 'sbOrders', type: 'number', label: { t: 'sbOrders' }, attrs: { min: 1, max: 1000 } })}
      <label><span data-t="sbBand"></span> ${sel('sbBand', opt('', 'sbAny', true) + BANDS.map(b => opt(b, 'band_' + b)).join(''), 'builder.band')}</label>
      <label><span data-t="sbWeather"></span> ${sel('sbWx', opt('', 'sbAny', true) + WX.map(w => opt(w, 'wx_' + w)).join(''), 'builder.weather')}</label>
      ${weatherSourceLine()}
    </div>
    <div id="sbOut" aria-live="polite"></div>
    <p class="small muted" data-t="sbNever"></p>
    <div id="sbOffer" hidden>
      <label><span data-t="sbOfferDish"></span> <select class="ui-input" id="sbDish" data-tour="builder.offerDish"></select></label>
      <label><span data-t="sbOfferLang"></span> ${sel('sbLang', LANGS.map(l => `<option value="${l}"${l === lang ? ' selected' : ''}>${l}</option>`).join(''), 'builder.offerLang')}</label>
      <p class="small muted" data-t="sbOfferNote"></p>
      ${ui.button({ variant: 'ghost', icon: 'send', label: { t: 'sbOffer' }, id: 'sbMake', attrs: { data: { tour: 'builder.offer' } } })}
    </div></details>`;
}

const val = id => $('#' + id)?.value ?? '';
const filter = () => ({ key: val('sbKey'), min: val('sbMin'), not_seen_days: val('sbNotSeen'), min_orders: val('sbOrders'), band: val('sbBand'), weather: val('sbWx') });

async function run(){
  const out = $('#sbOut'); if (!out) return;
  try {
    const d = await api('/owner/customers/taste/builder?' + queryOf(filter()));
    out.innerHTML = reportMarkup(d); retranslate(out);
    const dishes = d?.plan?.dishes || [];
    $('#sbOffer').hidden = !dishes.length;
    $('#sbDish').innerHTML = dishes.map(x => `<option value="${esc(x.id)}">${esc(x.name || x.id)}</option>`).join('');
  } catch (e) { out.innerHTML = `<p class="small muted">${esc(String(e.message || e))}</p>`; }
}

/// Fill `#cuBuilder` above the customers list. Silent on failure: the list stands without it.
export function mountBuilder(){
  const el = $('#cuBuilder'); if (!el) return;
  el.innerHTML = formMarkup(); retranslate(el);
  for (const x of $$('select, input', el)) x.onchange = run;
  $('details', el)?.addEventListener('toggle', e => { if (e.target.open) run(); }, { once: true });
  $('#sbMake').onclick = async () => {
    const f = filter();
    const seg = { kind: 'taste', filter: Object.fromEntries(Object.entries({ ...f, min: Number(f.min) || 500, not_seen_days: f.not_seen_days ? Number(f.not_seen_days) : null,
      min_orders: f.min_orders ? Number(f.min_orders) : null, band: f.band || null, weather: f.weather || null }).filter(([, v]) => v !== null)), dish: val('sbDish'), lang: val('sbLang') };
    try {
      // `text` is required by the campaign shape and replaced by the server's own words.
      await post('/owner/campaigns', { name: `${T[lang]?.[wordKey(f.key)] || f.key} · ${new Date().toISOString().slice(0, 10)}`.slice(0, 60), text: '-', segment: seg });
      toast(T[lang]?.sbOfferDone || 'ok');
    } catch (e) { toast(String(e.message || e)); }
  };
}
