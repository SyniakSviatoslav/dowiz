// The words of the "what the notes say" card (P16b, admin/kitchen-topics.js),
// merged into the console's table at import like `assistant-i18n.js`.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { T } from '/admin/i18n.js';
import { merge } from '/admin/kitchen-i18n.js';

export const WORDS = {
  sq: {
    ft_title: 'Çfarë thonë shënimet', ft_hint: 'Shënimet e klientëve, sipas pjatës dhe javës. Asnjëherë sipas personit; numrat dhe emrat janë hequr.',
    ft_none: 'Asnjë shënim me temë në këto ditë.', ft_week: 'Java', ft_topic: 'Tema', ft_notes: 'Shënime', ft_example: 'Shembull',
    ft_cold: 'e ftohtë', ft_late: 'me vonesë', ft_salty: 'e kripur', ft_small_portion: 'porcion i vogël', ft_not_tasty: 'pa shije',
    ft_stale: 'jo e freskët', ft_spicy: 'djegëse', ft_packaging: 'paketimi', ft_tasty: 'e shijshme', ft_fresh: 'e freskët',
  },
  en: {
    ft_title: 'What the notes say', ft_hint: 'Guests’ notes, by dish and week. Never by person; numbers and names are taken out.',
    ft_none: 'No note with a topic in these days.', ft_week: 'Week', ft_topic: 'Topic', ft_notes: 'Notes', ft_example: 'Example',
    ft_cold: 'cold', ft_late: 'late', ft_salty: 'salty', ft_small_portion: 'small portion', ft_not_tasty: 'not tasty',
    ft_stale: 'not fresh', ft_spicy: 'spicy', ft_packaging: 'packaging', ft_tasty: 'tasty', ft_fresh: 'fresh',
  },
  uk: {
    ft_title: 'Про що пишуть гості', ft_hint: 'Відгуки гостей за стравою і тижнем. Ніколи за людиною; номери та імена прибрано.',
    ft_none: 'За ці дні немає відгуків із темою.', ft_week: 'Тиждень', ft_topic: 'Тема', ft_notes: 'Відгуків', ft_example: 'Приклад',
    ft_cold: 'холодне', ft_late: 'запізнення', ft_salty: 'пересолене', ft_small_portion: 'мала порція', ft_not_tasty: 'несмачно',
    ft_stale: 'несвіже', ft_spicy: 'гостре', ft_packaging: 'упаковка', ft_tasty: 'смачно', ft_fresh: 'свіже',
  },
  ru: {
    ft_title: 'О чём пишут гости', ft_hint: 'Отзывы гостей по блюду и неделе. Никогда по человеку; номера и имена убраны.',
    ft_none: 'За эти дни нет отзывов с темой.', ft_week: 'Неделя', ft_topic: 'Тема', ft_notes: 'Отзывов', ft_example: 'Пример',
    ft_cold: 'холодное', ft_late: 'опоздание', ft_salty: 'пересолено', ft_small_portion: 'маленькая порция', ft_not_tasty: 'невкусно',
    ft_stale: 'несвежее', ft_spicy: 'острое', ft_packaging: 'упаковка', ft_tasty: 'вкусно', ft_fresh: 'свежее',
  },
};

merge(T, WORDS);
