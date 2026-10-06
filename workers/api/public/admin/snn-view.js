// W-SNN: the PURE half of the owner's shadow section (admin/snn.js draws it). Its words in the
// console's four languages, and the markup from `GET /api/owner/snn`. Node renders it
// (snn-view.test.mjs). ASCII QUOTES ONLY as string delimiters.

import { chip, esc } from '../lib/ui/index.js';

export const MODES = ['shadow', 'on', 'off'];

export const WORDS = {
  sq: { snnSettled: 'porositë e radhës të kontrolluara', snnQCurrent: 'në 3 të parat e tanishme', snnQNet: 'në 3 të parat e rrjetit',
        snnTitle: 'Rekomandimet me rrjet nervor (provë)', snnNone: 'Ende asnjë krahasim.',
        snnCompared: 'krahasime', snnTop1: 'e njëjta pjatë e parë', snnOverlap: 'të përbashkëta në 3 të parat', snnUnusable: 'nuk mundi të punojë',
        snnHint: 'Në "provë" rrjeti vetëm llogarit: klienti sheh renditjen e tanishme. "Ndezur" vetëm pasi të fitojë krahasimin. "Fikur" nuk llogarit asgjë.',
        snn_shadow: 'Provë', snn_on: 'Ndezur', snn_off: 'Fikur' },
  en: { snnSettled: 'next orders checked', snnQCurrent: 'in the current top 3', snnQNet: 'in the network top 3',
        snnTitle: 'Neural network suggestions (trial)', snnNone: 'No comparisons yet.',
        snnCompared: 'comparisons', snnTop1: 'same first dish', snnOverlap: 'shared in the top 3', snnUnusable: 'could not run',
        snnHint: 'In "trial" the network only computes: guests see the current ranking. Turn it "on" only after it wins the comparison. "Off" computes nothing.',
        snn_shadow: 'Trial', snn_on: 'On', snn_off: 'Off' },
  uk: { snnSettled: 'наступних замовлень перевірено', snnQCurrent: 'у нинішніх перших 3', snnQNet: 'у перших 3 мережі',
        snnTitle: 'Підказки нейромережі (випробування)', snnNone: 'Порівнянь ще немає.',
        snnCompared: 'порівнянь', snnTop1: 'та сама перша страва', snnOverlap: 'спільних у перших 3', snnUnusable: 'не змогла спрацювати',
        snnHint: 'У режимі «випробування» мережа лише рахує: гість бачить нинішній порядок. Вмикайте лише після того, як вона виграє порівняння. «Вимкнено» не рахує нічого.',
        snn_shadow: 'Випробування', snn_on: 'Увімкнено', snn_off: 'Вимкнено' },
  ru: { snnSettled: 'следующих заказов проверено', snnQCurrent: 'в нынешних первых 3', snnQNet: 'в первых 3 сети',
        snnTitle: 'Подсказки нейросети (испытание)', snnNone: 'Сравнений пока нет.',
        snnCompared: 'сравнений', snnTop1: 'то же первое блюдо', snnOverlap: 'общих в первых 3', snnUnusable: 'не смогла сработать',
        snnHint: 'В режиме «испытание» сеть только считает: гость видит нынешний порядок. Включайте только после того, как она выиграет сравнение. «Выключено» не считает ничего.',
        snn_shadow: 'Испытание', snn_on: 'Включено', snn_off: 'Выключено' },
};

const pct = pm => `${Math.round((Number(pm) | 0) / 10)}%`;

/// PURE. The section from `GET /owner/snn` (`taste/snn.rs::health_json`).
export function snnMarkup(d){
  if (!d || typeof d !== 'object') return '';
  const mode = MODES.includes(d.mode) ? d.mode : 'shadow';
  const n = Number(d.compared) | 0;
  const facts = n
    ? `${n} <span data-t="snnCompared"></span> · ${pct(d.top1Pm)} <span data-t="snnTop1"></span> · ${pct(d.overlapPm)} <span data-t="snnOverlap"></span>`
    : '<span data-t="snnNone"></span>';
  const settled = Number(d.settled) | 0;
  const quality = settled
    ? `<p class="small" data-snn-quality="1">${settled} <span data-t="snnSettled"></span>: ${pct(d.currentHitPm)} <span data-t="snnQCurrent"></span> · ${pct(d.snnHitPm)} <span data-t="snnQNet"></span></p>`
    : '';
  const unusable = (Number(d.unusable) | 0) ? ` · ${Number(d.unusable) | 0} <span data-t="snnUnusable"></span>` : '';
  const error = d.error ? `<p class="warn small">${esc(String(d.error))}</p>` : '';
  const chips = MODES.map(m => chip({ as: 'button', selected: m === mode, label: { t: 'snn_' + m }, attrs: { data: { snnMode: m } } })).join(' ');
  return `<p class="eyebrow mt-3" data-t="snnTitle"></p>
    <p class="small" data-snn-state="${mode}">${facts}${unusable}</p>${quality}${error}
    <div class="chips">${chips}</div>
    <p class="muted small" data-t="snnHint"></p>`;
}

