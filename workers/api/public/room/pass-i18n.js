// The door's words (W-WIRE), merged into the room's table without renaming
// any word it already has. A language the room gains later reads English.
import { T } from './i18n.js';

export const WORDS = {
  sq: { pass: 'Biletat', passTitle: 'Kontrollo biletën', passHint: 'Shkruani kodin nga bileta e klientit (ose ngjisni atë që lexoi skaneri).',
    passCode: 'Kodi i biletës', passCheck: 'Kontrollo', passYes: 'E vlefshme', passNo: 'Jo e vlefshme', passParty: '{n} persona', passNeedCode: 'Shkruani kodin.' },
  en: { pass: 'Passes', passTitle: 'Check a pass', passHint: 'Type the code from the guestʼs pass (or paste what a scanner read).',
    passCode: 'Pass code', passCheck: 'Check', passYes: 'Valid', passNo: 'Not valid', passParty: '{n} people', passNeedCode: 'Type the code.' },
  uk: { pass: 'Перепустки', passTitle: 'Перевірити перепустку', passHint: 'Введіть код із перепустки гостя (або вставте те, що прочитав сканер).',
    passCode: 'Код перепустки', passCheck: 'Перевірити', passYes: 'Дійсна', passNo: 'Недійсна', passParty: '{n} осіб', passNeedCode: 'Введіть код.' },
  ru: { pass: 'Пропуска', passTitle: 'Проверить пропуск', passHint: 'Введите код из пропуска гостя (или вставьте то, что прочитал сканер).',
    passCode: 'Код пропуска', passCheck: 'Проверить', passYes: 'Действителен', passNo: 'Недействителен', passParty: '{n} чел.', passNeedCode: 'Введите код.' },
};

for (const [l, dict] of Object.entries(WORDS)) {
  T[l] = T[l] || {};
  for (const [key, v] of Object.entries(dict)) if (!(key in T[l])) T[l][key] = v;
}
