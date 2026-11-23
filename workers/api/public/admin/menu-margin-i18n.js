// The menu margin's words (operator 2026-11-23), in the four languages,
// merged into T at load like ebills-sales-i18n.js.
//
// ASCII QUOTES ONLY in this file, and no apostrophes inside the words: a
// typographic quote once took down the whole console.

import { T, LANGS } from '/admin/i18n.js';

export const WORDS = {
  en: {
    mg_unknown: 'cost unknown', mg_noRecipe: 'no recipe', mg_title: 'Margin, last 30 days', mg_today: 'Today, one portion', mg_portion: 'per portion',
    mg_sold: 'Portions sold', mg_revenue: 'Revenue', mg_cogs: 'Cost of goods', mg_margin: 'Margin', mg_foodCost: 'Food cost', mg_unknownCost: 'unknown',
    mg_stamped: 'at the cost when sold', mg_recipeToday: 'at the recipe of today', mg_portions: 'portions',
    mg_attributed: 'of them till lines counted under this dish now (stock is not re-drawn)', mg_notSold: 'Not sold in the last 30 days.',
    mg_missing: 'The cost waits on:', mg_addRecipe: 'add a recipe', mg_till: 'Till codes with no dish', mg_link: 'Link to a dish',
    mg_tillHint: 'Counted in revenue, under no dish. Link each to its dish: the history counts it from then on; stock is not re-drawn.',
  },
  uk: {
    mg_unknown: 'собівартість невідома', mg_noRecipe: 'немає техкарти', mg_title: 'Маржа за останні 30 днів', mg_today: 'Сьогодні, одна порція', mg_portion: 'за порцію',
    mg_sold: 'Продано порцій', mg_revenue: 'Виторг', mg_cogs: 'Собівартість', mg_margin: 'Маржа', mg_foodCost: 'Фудкост', mg_unknownCost: 'невідомо',
    mg_stamped: 'за ціною на момент продажу', mg_recipeToday: 'за сьогоднішньою техкартою', mg_portions: 'порц.',
    mg_attributed: 'з них рядки каси, зараховані цій страві тепер (склад не перераховано)', mg_notSold: 'За останні 30 днів не продавалась.',
    mg_missing: 'Собівартість чекає на:', mg_addRecipe: 'додати техкарту', mg_till: 'Коди каси без страви', mg_link: 'Зіставити зі стравою',
    mg_tillHint: 'Враховані у виторгу, але без страви. Зіставте кожен зі стравою: історія врахує його відтоді; склад не перераховується.',
  },
  sq: {
    mg_unknown: 'kosto e panjohur', mg_noRecipe: 'pa recetë', mg_title: 'Marzhi, 30 ditët e fundit', mg_today: 'Sot, një porcion', mg_portion: 'për porcion',
    mg_sold: 'Porcione të shitura', mg_revenue: 'Të ardhura', mg_cogs: 'Kostoja e mallrave', mg_margin: 'Marzhi', mg_foodCost: 'Kostoja e ushqimit', mg_unknownCost: 'e panjohur',
    mg_stamped: 'me koston në kohën e shitjes', mg_recipeToday: 'me recetën e sotme', mg_portions: 'porc.',
    mg_attributed: 'prej tyre rreshta arke të llogaritur tani për këtë pjatë (stoku nuk rillogaritet)', mg_notSold: 'Nuk është shitur në 30 ditët e fundit.',
    mg_missing: 'Kostoja pret:', mg_addRecipe: 'shto recetën', mg_till: 'Kode arke pa pjatë', mg_link: 'Lidh me një pjatë',
    mg_tillHint: 'Të llogaritura në të ardhura, por pa pjatë. Lidhni secilin me pjatën e tij: historiku e numëron që atëherë; stoku nuk rillogaritet.',
  },
  ru: {
    mg_unknown: 'себестоимость неизвестна', mg_noRecipe: 'нет техкарты', mg_title: 'Маржа за последние 30 дней', mg_today: 'Сегодня, одна порция', mg_portion: 'за порцию',
    mg_sold: 'Продано порций', mg_revenue: 'Выручка', mg_cogs: 'Себестоимость', mg_margin: 'Маржа', mg_foodCost: 'Фудкост', mg_unknownCost: 'неизвестно',
    mg_stamped: 'по цене на момент продажи', mg_recipeToday: 'по сегодняшней техкарте', mg_portions: 'порц.',
    mg_attributed: 'из них строки кассы, засчитанные этому блюду сейчас (склад не пересчитан)', mg_notSold: 'За последние 30 дней не продавалось.',
    mg_missing: 'Себестоимость ждёт:', mg_addRecipe: 'добавить техкарту', mg_till: 'Коды кассы без блюда', mg_link: 'Сопоставить с блюдом',
    mg_tillHint: 'Учтены в выручке, но без блюда. Сопоставьте каждый с блюдом: история учтёт его с этого момента; склад не пересчитывается.',
  },
};

for (const l of LANGS) if (T[l] && WORDS[l]) Object.assign(T[l], WORDS[l]);
