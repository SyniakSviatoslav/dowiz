// The words of editing a dish in the venue's own language, moving it between
// categories and ordering dishes and categories (lane W-CRUD, 2026-09-29), in
// the lane's own dictionary (never admin/i18n.js). Merged into the console's
// T once, on import, for every language this file has.
//
// ASCII QUOTES ONLY as string delimiters (DOWIZ-COMMON-RULES rule 11);
// apostrophes inside words use U+02BC.

import { T } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    ownLanguage: 'Në gjuhën e lokalit', position: 'Renditja', positionHint: 'Lart ose poshtë: kështu e sheh klienti.',
    moveUp: 'Ngjite', moveDown: 'Zbrite', moved: 'U zhvendos',
  },
  en: {
    ownLanguage: 'In the venueʼs language', position: 'Order', positionHint: 'Up or down: the order the customer sees.',
    moveUp: 'Move up', moveDown: 'Move down', moved: 'Moved',
  },
  uk: {
    ownLanguage: 'Мовою закладу', position: 'Порядок', positionHint: 'Вгору чи вниз: так бачить клієнт.',
    moveUp: 'Вище', moveDown: 'Нижче', moved: 'Переміщено',
  },
  ru: {
    ownLanguage: 'На языке заведения', position: 'Порядок', positionHint: 'Вверх или вниз: так видит клиент.',
    moveUp: 'Выше', moveDown: 'Ниже', moved: 'Перемещено',
  },
};

for (const [l, words] of Object.entries(WORDS)) if (T[l]) Object.assign(T[l], words);
