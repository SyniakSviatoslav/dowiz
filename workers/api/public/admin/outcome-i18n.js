// The words of a write's notice (`outcome-logic.js`), merged into the
// console's T on import. ASCII QUOTES ONLY as string delimiters.
import { T } from '/admin/i18n.js';

export const WORDS = {
  en: { out_notSaved: 'Not saved', out_bad: 'check the fields', out_denied: 'no permission for this', out_missing: 'it no longer exists', out_conflict: 'it changed or conflicts with what is stored', out_tooBig: 'too large', out_busy: 'too many tries, wait a minute', out_server: 'server error, try again', out_offline: 'no connection, try again', out_failed: 'error' },
  sq: { out_notSaved: 'Nuk u ruajt', out_bad: 'kontrolloni fushat', out_denied: 'nuk keni leje', out_missing: 'nuk ekziston më', out_conflict: 'ka ndryshuar ose bie ndesh me të ruajturat', out_tooBig: 'shumë e madhe', out_busy: 'shumë përpjekje, prisni një minutë', out_server: 'gabim serveri, provoni përsëri', out_offline: 'nuk ka lidhje, provoni përsëri', out_failed: 'gabim' },
  uk: { out_notSaved: 'Не збережено', out_bad: 'перевірте поля', out_denied: 'немає прав на цю дію', out_missing: 'цього вже не існує', out_conflict: 'змінилося або суперечить збереженому', out_tooBig: 'завеликий обсяг', out_busy: 'забагато спроб, зачекайте хвилину', out_server: 'помилка сервера, спробуйте ще раз', out_offline: 'немає зʼєднання, спробуйте ще раз', out_failed: 'помилка' },
  ru: { out_notSaved: 'Не сохранено', out_bad: 'проверьте поля', out_denied: 'нет прав на это действие', out_missing: 'этого больше нет', out_conflict: 'изменилось или противоречит сохранённому', out_tooBig: 'слишком большой объём', out_busy: 'слишком много попыток, подождите минуту', out_server: 'ошибка сервера, попробуйте ещё раз', out_offline: 'нет соединения, попробуйте ещё раз', out_failed: 'ошибка' },
};
for (const [l, w] of Object.entries(WORDS)) if (T[l]) Object.assign(T[l], w);
