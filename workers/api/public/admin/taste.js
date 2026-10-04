// THE GUEST'S TASTE, as the venue scored it (W-MR0 row MR8; DECISIONS.md D0 amendment 2026-10-04,
// operator ruling: automatic for every guest who gave a phone, unless they turned it off). The card shows the top tags
// and categories, the segment and the rule that decided it ("why"), and what it was built from;
// the customers list shows how many guests are in each segment. A SEGMENT IS NOT A TIER: it says
// when a guest last came, never what they are worth, and nothing here sets a price, a discount or
// a refusal (tools/gates/no-scoring.sh). The scoring itself is `services/customers/taste.rs`.
//
// ASCII QUOTES ONLY as string delimiters.

import { $, esc, api, retranslate } from '/admin/core.js';
import { T, LANGS } from '/admin/i18n.js';
import * as ui from '/lib/ui/index.js';

const SEGMENTS = ['new', 'regular', 'at_risk', 'lapsed'];

const WORDS = {
  sq: { tasteCard: 'Shija', tasteNone: 'Asgjë: klienti e ka fikur, ose nuk ka porositur me telefon.',
        tasteTags: 'Shijet kryesore', tasteCats: 'Kategoritë kryesore', tasteWhy: 'Pse', tasteFrom: 'Nga', tasteFromOrders: 'porosi', tasteFromDevice: 'përmbledhja e telefonit të tij',
        tasteNever: 'Vetëm për të sugjeruar pjata. Kurrë për çmime, zbritje apo refuzime.', tasteSegments: 'Klientët sipas segmentit (pa ata që e fikën)',
        seg_new: 'I ri', seg_regular: 'I rregullt', seg_at_risk: 'Nuk ka ardhur prej kohësh', seg_lapsed: 'Nuk vjen më' },
  en: { tasteCard: 'Taste', tasteNone: 'Nothing: the guest turned it off, or has not ordered with a phone.',
        tasteTags: 'Top tastes', tasteCats: 'Top categories', tasteWhy: 'Why', tasteFrom: 'From', tasteFromOrders: 'orders', tasteFromDevice: 'the summary from their phone',
        tasteNever: 'Only to suggest dishes. Never for prices, discounts or refusals.', tasteSegments: 'Guests by segment (not those who turned it off)',
        seg_new: 'New', seg_regular: 'Regular', seg_at_risk: 'Not seen for a while', seg_lapsed: 'No longer coming' },
  uk: { tasteCard: 'Смак', tasteNone: 'Нічого: гість це вимкнув або не замовляв із телефоном.',
        tasteTags: 'Головні смаки', tasteCats: 'Головні категорії', tasteWhy: 'Чому', tasteFrom: 'З чого', tasteFromOrders: 'замовлень', tasteFromDevice: 'підсумок із його телефона',
        tasteNever: 'Лише щоб підказати страви. Ніколи для цін, знижок чи відмов.', tasteSegments: 'Гості за сегментами (без тих, хто вимкнув)',
        seg_new: 'Новий', seg_regular: 'Постійний', seg_at_risk: 'Давно не був', seg_lapsed: 'Більше не приходить' },
  ru: { tasteCard: 'Вкус', tasteNone: 'Ничего: гость это выключил или не заказывал с телефоном.',
        tasteTags: 'Главные вкусы', tasteCats: 'Главные категории', tasteWhy: 'Почему', tasteFrom: 'Из чего', tasteFromOrders: 'заказов', tasteFromDevice: 'сводка с его телефона',
        tasteNever: 'Только чтобы подсказать блюда. Никогда для цен, скидок или отказов.', tasteSegments: 'Гости по сегментам (без тех, кто выключил)',
        seg_new: 'Новый', seg_regular: 'Постоянный', seg_at_risk: 'Давно не был', seg_lapsed: 'Больше не приходит' },
};
for (const l of LANGS) Object.assign(T[l], WORDS[l]);

/// PURE. The card section from `GET /owner/customers/:key/taste` (`taste.rs::view`).
export function tasteMarkup(d){
  const p = d?.taste;
  if (!p) return `<p class="eyebrow mt-3" data-t="tasteCard"></p><p class="muted small" data-t="tasteNone"></p>`;
  const list = rows => (rows || []).map(r => esc(r.key)).join(', ') || '-';
  const from = [`${Number(p.orders) | 0} <span data-t="tasteFromOrders"></span>`, p.device ? '<span data-t="tasteFromDevice"></span>' : ''].filter(Boolean).join(' + ');
  return `<p class="eyebrow mt-3" data-t="tasteCard"></p>
    <p class="small" data-tour="customers.taste"><b data-t="seg_${SEGMENTS.includes(p.segment) ? p.segment : 'new'}"></b></p>
    <p class="small"><span data-t="tasteWhy"></span>: ${esc(p.why)}</p>
    <p class="small"><span data-t="tasteTags"></span>: ${list(p.tags)}</p>
    <p class="small"><span data-t="tasteCats"></span>: ${list(p.cats)}</p>
    <p class="small muted"><span data-t="tasteFrom"></span>: ${from}</p>
    <p class="muted small" data-t="tasteNever"></p>`;
}

/// PURE. The venue's counts, from `GET /owner/customers/taste/segments`.
export function segmentsMarkup(d){
  const s = d?.segments;
  if (!s) return '';
  return `<p class="eyebrow mt-3" data-t="tasteSegments"></p>
    <div class="chips" data-tour="customers.segments">${SEGMENTS.map(k => `${ui.chip({ label: { t: 'seg_' + k } })} <b class="mono">${Number(s[k]) | 0}</b>`).join(' ')}</div>`;
}

/// Fill `#cdTaste` on an open card. Silent on failure: the card is the page.
export async function mountTaste(key){
  const el = $('#cdTaste');
  if (!el) return;
  try {
    const d = await api(`/owner/customers/${encodeURIComponent(key)}/taste`);
    if (!$('#cdTaste')) return;
    el.innerHTML = tasteMarkup(d);
    retranslate(el);
  } catch { /* the card stands without it */ }
}

/// Fill `#cuSegments` above the customers list.
export async function mountSegments(){
  const el = $('#cuSegments');
  if (!el) return;
  try {
    const d = await api('/owner/customers/taste/segments');
    if (!$('#cuSegments')) return;
    el.innerHTML = segmentsMarkup(d);
    retranslate(el);
  } catch { /* the list stands without it */ }
}
