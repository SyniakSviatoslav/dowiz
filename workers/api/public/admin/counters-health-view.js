// PURE. The words and the subtitle of the object-counters row on the owner's health page (AX0).
// `GET /api/owner/health` -> `counters`: what the venue's server object did since it last woke --
// reads, writes, catch-ups it could not answer, and the cost of the last menu saves. A figure with
// a neutral pill: nothing acts on it and it is never red. Relative imports only, so node renders it.
// ASCII QUOTES ONLY in this file.

import { esc } from '../lib/ui/core.js';

export const WORDS = {
  sq: { cnt_title: 'Serveri i lokalit që kur u zgjua', cnt_sub: 'lexime · shkrime · përditësime pa përgjigje / gjithsej · ruajtje menuje',
        cnt_note: 'Vetëm informacion: numrat fillojnë nga zero sa herë serveri fle.' },
  en: { cnt_title: 'The venue server since it woke', cnt_sub: 'reads · writes · catch-ups unanswered / total · menu saves',
        cnt_note: 'For information only: the counts start from zero each time the server sleeps.' },
  uk: { cnt_title: 'Сервер закладу від пробудження', cnt_sub: 'читань · записів · оновлень без відповіді / усього · збережень меню',
        cnt_note: 'Лише для інформації: лічба починається з нуля щоразу, як сервер засинає.' },
  ru: { cnt_title: 'Сервер заведения с пробуждения', cnt_sub: 'чтений · записей · обновлений без ответа / всего · сохранений меню',
        cnt_note: 'Только для информации: счёт начинается с нуля каждый раз, когда сервер засыпает.' },
};

/// PURE. The row's subtitle from `counters`; '' when the answer is missing.
export function countersSub(c){
  if (!c || typeof c !== 'object') return '';
  if (c.error) return esc(String(c.error));
  const n = k => Number(c[k]) | 0;
  const w = c.wake || {};
  return `<span class="mono">${Number(w.reads) | 0} · ${Number(w.writes) | 0} · ${n('since_none')}/${n('since_total')} · ${n('cat_writes')}</span> <span class="muted" data-t="cnt_sub"></span><br><span class="small muted" data-t="cnt_note"></span>`;
}
