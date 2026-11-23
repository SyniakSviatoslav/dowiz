// The week's purchase list's words, PURE: no imports, so node tests the table
// itself (stock-week-logic.test.mjs). `stock-week-i18n.js` merges it into the
// console's dictionary at import. lib/langs.js is the list of languages; the
// langs gate holds every language here to every key.
//
// ASCII QUOTES ONLY as delimiters; an apostrophe inside a word is U+02BC.
export const WORDS = {
  sq: {
    wp_tile: 'Për javën', wp_title: 'Blerjet për javën', wp_hint: 'Përdorimi mesatar javor i katër javëve të fundit të plota, plus ditët deri sa të vijë malli, minus çʼka të lirë në raft, rrumbullakuar lart te paketa.',
    wp_week: 'javë', wp_weeks: 'javë', wp_days: 'ditë', wp_since: 'që nga', wp_noData: 'Asnjë shitje nuk ka marrë ende nga magazina: nuk ka mesatare.',
    wp_weekly: 'në javë', wp_free: 'të lira', wp_notCounted: 'magazina e panumëruar', wp_belowZero: 'nën zero: furnizimet nuk janë regjistruar',
    wp_toBuy: 'për të blerë', wp_need: 'nevoja e javës', wp_lead: 'ditë deri sa vjen', wp_cost: 'kosto e përafërt', wp_noPrice: 'pa çmim',
    wp_total: 'Kosto e përafërt e listës', wp_unpriced: 'rreshta pa çmim, jashtë shumës',
    wp_floor: 'të shitjeve janë pjata pa recetë: ato nuk marrin asgjë nga magazina, ndaj kjo listë është minimumi.',
    wp_top: 'Pjatat pa recetë, sipas të ardhurave', wp_copy: 'Kopjo për furnitorin', wp_copied: 'U kopjua', wp_noSupplier: 'Pa furnitor',
    wp_empty: 'Asgjë për të blerë këtë javë.', wp_period: 'Mesatarja nga',
  },
  en: {
    wp_tile: 'For the week', wp_title: 'Purchases for the week', wp_hint: 'The average weekly use of the last four whole weeks, plus the days until the goods arrive, less what is free on the shelf, rounded up to the pack.',
    wp_week: 'week', wp_weeks: 'weeks', wp_days: 'days', wp_since: 'since', wp_noData: 'No sale has drawn from the stock yet: there is no average.',
    wp_weekly: 'a week', wp_free: 'free', wp_notCounted: 'stock not counted', wp_belowZero: 'below zero: deliveries not recorded',
    wp_toBuy: 'to buy', wp_need: 'the weekʼs need', wp_lead: 'days until delivery', wp_cost: 'est. cost', wp_noPrice: 'no price',
    wp_total: 'Estimated cost of the list', wp_unpriced: 'rows with no price, left out of the sum',
    wp_floor: 'of sales are dishes with no recipe: they draw nothing from the stock, so this list is a floor.',
    wp_top: 'Dishes with no recipe, by revenue', wp_copy: 'Copy for the supplier', wp_copied: 'Copied', wp_noSupplier: 'No supplier',
    wp_empty: 'Nothing to buy this week.', wp_period: 'Average over',
  },
  uk: {
    wp_tile: 'На тиждень', wp_title: 'Закупівля на тиждень', wp_hint: 'Середня тижнева витрата за останні чотири повні тижні, плюс дні до поставки, мінус вільне на складі, з округленням угору до упаковки.',
    wp_week: 'тиждень', wp_weeks: 'тижні', wp_days: 'дн.', wp_since: 'від', wp_noData: 'Жоден продаж ще не списав зі складу: середнього немає.',
    wp_weekly: 'на тиждень', wp_free: 'вільно', wp_notCounted: 'залишок не пораховано', wp_belowZero: 'нижче нуля: поставки не внесені',
    wp_toBuy: 'купити', wp_need: 'потреба на тиждень', wp_lead: 'дн. до поставки', wp_cost: 'орієнт. вартість', wp_noPrice: 'без ціни',
    wp_total: 'Орієнтовна вартість списку', wp_unpriced: 'рядків без ціни, поза сумою',
    wp_floor: 'продажів — страви без техкарти: вони нічого не списують зі складу, тож цей список — мінімум.',
    wp_top: 'Страви без техкарти, за виручкою', wp_copy: 'Скопіювати для постачальника', wp_copied: 'Скопійовано', wp_noSupplier: 'Без постачальника',
    wp_empty: 'Цього тижня нічого купувати.', wp_period: 'Середнє за',
  },
  ru: {
    wp_tile: 'На неделю', wp_title: 'Закупка на неделю', wp_hint: 'Средний недельный расход за последние четыре полные недели, плюс дни до поставки, минус свободное на складе, с округлением вверх до упаковки.',
    wp_week: 'неделя', wp_weeks: 'недели', wp_days: 'дн.', wp_since: 'с', wp_noData: 'Ни одна продажа ещё не списала со склада: среднего нет.',
    wp_weekly: 'в неделю', wp_free: 'свободно', wp_notCounted: 'остаток не посчитан', wp_belowZero: 'ниже нуля: поставки не внесены',
    wp_toBuy: 'купить', wp_need: 'потребность на неделю', wp_lead: 'дн. до поставки', wp_cost: 'ориент. стоимость', wp_noPrice: 'без цены',
    wp_total: 'Ориентировочная стоимость списка', wp_unpriced: 'строк без цены, вне суммы',
    wp_floor: 'продаж — блюда без техкарты: они ничего не списывают со склада, поэтому этот список — минимум.',
    wp_top: 'Блюда без техкарты, по выручке', wp_copy: 'Скопировать для поставщика', wp_copied: 'Скопировано', wp_noSupplier: 'Без поставщика',
    wp_empty: 'На этой неделе покупать нечего.', wp_period: 'Среднее за',
  },
};
