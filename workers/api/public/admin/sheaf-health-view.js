// PURE. The words and the subtitle of the consistency-radius row on the owner's health page
// (W-TASTE2 S7a). Relative imports only, so node renders it (sheaf-health-view.test.mjs).
// ASCII QUOTES ONLY in this file.

import { esc } from '../lib/ui/core.js';

export const WORDS = {
  sq: { shR_title: 'Telefoni dhe lokali për shijet', shR_none: 'Ende asnjë klient me të dyja; asgjë për të krahasuar.',
        shR_sub: 'klientë të krahasuar · mospërputhja më e madhe · mesatarja (nga 1000)', shR_note: 'Vetëm informacion: asgjë nuk ndryshon prej tij.' },
  en: { shR_title: 'Phone and venue on tastes', shR_none: 'No guest with both yet; nothing to compare.',
        shR_sub: 'guests compared · largest disagreement · median (of 1000)', shR_note: 'For information only: nothing changes because of it.' },
  uk: { shR_title: 'Телефон і заклад про смаки', shR_none: 'Ще немає гостя з обома; нема що порівнювати.',
        shR_sub: 'гостей порівняно · найбільша розбіжність · медіана (з 1000)', shR_note: 'Лише для інформації: від нього нічого не змінюється.' },
  ru: { shR_title: 'Телефон и заведение о вкусах', shR_none: 'Пока нет гостя с обоими; нечего сравнивать.',
        shR_sub: 'гостей сравнено · наибольшее расхождение · медиана (из 1000)', shR_note: 'Только для информации: от него ничего не меняется.' },
};
/// PURE. The row's subtitle from `sheaf.taste`; '' when the answer is missing.
export function radiusSub(s){
  if (!s || typeof s !== 'object') return '';
  if (s.error) return esc(String(s.error));
  const n = Number(s.compared) | 0;
  if (!n) return '<span data-t="shR_none"></span>';
  return `<span class="mono">${n} · ${Number(s.maxPm) | 0} · ${Number(s.medianPm) | 0}</span> <span class="muted" data-t="shR_sub"></span><br><span class="small muted" data-t="shR_note"></span>`;
}

