// The words of SEMI-FINISHED PRODUCTS (lane W-PF, 2026-09-29): the three
// kinds of item (raw / semi-finished / dish), the card, the editor, where
// an item is used, and what one sale takes off the shelf. In the lane's own
// dictionary (never admin/i18n.js), merged into the console's T once, on
// import, for every language.
//
// ASCII QUOTES ONLY as string delimiters (DOWIZ-COMMON-RULES rule 11);
// apostrophes inside words use U+02BC.

import { T } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    kind_raw: 'Lëndë e parë', kind_prep: 'Gjysmëprodukt', kind_dish: 'Pjata', pf_toMenu: 'Pjatat janë te Menyja',
    pf_title: 'Gjysmëprodukt', pf_add: 'Shto gjysmëprodukt', pf_edit: 'Ndrysho gjysmëproduktin',
    pf_hint: 'Përgatitet në kuzhinë nga lëndë të para dhe gjysmëprodukte të tjera. Shkruani sasitë bruto dhe sa del gati: kostoja dhe shpenzimi i pjatave llogariten vetë.',
    pf_lines: 'Përbërja (bruto)', pf_addLine: 'Shto përbërës', pf_pick: 'Zgjidhni një përbërës ose gjysmëprodukt', pf_noLines: 'Shtoni të paktën një përbërës.',
    pf_yield: 'Dalja gati (neto)', pf_yieldHint: 'Sa peshon i gjithë përgatitimi kur është gati, në njësinë e mësipërme.',
    pf_k: 'Koeficienti K', pf_kHint: 'Dalja / shuma bruto. Nën 100 % avullon; mbi 100 % thith ujë (orizi).', pf_kUnknown: 'i panjohur (një copë pa peshë)',
    pf_batchCost: 'Kostoja e përgatitimit', pf_costPerKg: 'për kg', pf_costPerL: 'për l', pf_costPerPiece: 'për copë', pf_costUnknown: 'kosto e panjohur (një përbërës pa çmim)',
    pf_lineCost: 'Kosto', pf_gross: 'bruto', pf_untracked: 'Jashtë magazinës (ujë)', pf_untrackedHint: 'Mbetet në recetë dhe në K, por nuk shlyhet nga magazina.',
    pf_usedIn: 'Përdoret në', pf_usedNowhere: 'Nuk përdoret askund ende.', pf_preps: 'gjysmëprodukte', pf_dishes: 'pjata',
    pf_takes: 'Një shitje merr nga magazina', pf_takesHint: 'Lëndët e para të zbërthyera nga çdo gjysmëprodukt, për një porcion, të sakta deri në miligram.',
    pf_takesTotal: 'Kosto e porcionit', pf_takesNone: 'Kjo pjatë nuk ka recetë.', pf_refused: 'Nuk zbërthehet',
    pf_deleteUsed: 'Përdoret ende. Nëse e fshini, rreshti hiqet nga çdo kartë dhe recetë e mëposhtme:', pf_qtyBad: 'Sasia është 1 deri 100 000, e plotë.',
    pf_yieldBad: 'Dalja është 1 deri 100 000, e plotë.', pf_count: 'gjysmëprodukte', pf_saved: 'U ruajt; pjata u përditësuan',
  },
  en: {
    kind_raw: 'Raw item', kind_prep: 'Semi-finished', kind_dish: 'Dishes', pf_toMenu: 'Dishes live on the Menu tab',
    pf_title: 'Semi-finished product', pf_add: 'Add semi-finished', pf_edit: 'Edit semi-finished',
    pf_hint: 'Made in the kitchen from raw items and other semi-finished products. Type the gross quantities and how much comes out ready: cost and every dishʼs write-off follow by themselves.',
    pf_lines: 'Components (gross)', pf_addLine: 'Add component', pf_pick: 'Choose a raw item or a semi-finished product', pf_noLines: 'Add at least one component.',
    pf_yield: 'Yield, ready (net)', pf_yieldHint: 'What the whole batch weighs when it is ready, in the unit above.',
    pf_k: 'Ratio K', pf_kHint: 'Yield / sum of gross. Under 100 % it boils down; over 100 % it takes up water (rice).', pf_kUnknown: 'unknown (a piece without a weight)',
    pf_batchCost: 'Batch cost', pf_costPerKg: 'per kg', pf_costPerL: 'per l', pf_costPerPiece: 'per piece', pf_costUnknown: 'cost unknown (a component has no price)',
    pf_lineCost: 'Cost', pf_gross: 'gross', pf_untracked: 'Not stocked (water)', pf_untrackedHint: 'Stays in the recipe and in K, but is never taken off the shelf.',
    pf_usedIn: 'Used in', pf_usedNowhere: 'Not used anywhere yet.', pf_preps: 'semi-finished', pf_dishes: 'dishes',
    pf_takes: 'One sale takes off the shelf', pf_takesHint: 'The raw items, expanded through every semi-finished product, for one portion, exact to the milligram.',
    pf_takesTotal: 'Portion cost', pf_takesNone: 'This dish has no recipe.', pf_refused: 'Cannot expand',
    pf_deleteUsed: 'Still in use. Deleting it removes the line from every card and recipe below:', pf_qtyBad: 'A quantity is 1 to 100 000, whole.',
    pf_yieldBad: 'The yield is 1 to 100 000, whole.', pf_count: 'semi-finished', pf_saved: 'Saved; dishes updated',
  },
  uk: {
    kind_raw: 'Сировина', kind_prep: 'Напівфабрикат', kind_dish: 'Страви', pf_toMenu: 'Страви — у вкладці Меню',
    pf_title: 'Напівфабрикат', pf_add: 'Додати напівфабрикат', pf_edit: 'Змінити напівфабрикат',
    pf_hint: 'Готується на кухні із сировини та інших напівфабрикатів. Введіть маси брутто і вихід готового: собівартість і списання страв рахуються самі.',
    pf_lines: 'Склад (брутто)', pf_addLine: 'Додати компонент', pf_pick: 'Оберіть сировину або напівфабрикат', pf_noLines: 'Додайте хоча б один компонент.',
    pf_yield: 'Вихід готового (нетто)', pf_yieldHint: 'Скільки важить увесь заміс у готовому вигляді, в одиниці вище.',
    pf_k: 'Коефіцієнт K', pf_kHint: 'Вихід / сума брутто. Менше 100 % — уварюється; більше 100 % — вбирає воду (рис).', pf_kUnknown: 'невідомий (штука без ваги)',
    pf_batchCost: 'Собівартість замісу', pf_costPerKg: 'за кг', pf_costPerL: 'за л', pf_costPerPiece: 'за шт', pf_costUnknown: 'собівартість невідома (компонент без ціни)',
    pf_lineCost: 'Собівартість', pf_gross: 'брутто', pf_untracked: 'Не на складі (вода)', pf_untrackedHint: 'Лишається в рецепті та в K, але ніколи не списується зі складу.',
    pf_usedIn: 'Використовується в', pf_usedNowhere: 'Поки ніде не використовується.', pf_preps: 'напівфабрикатах', pf_dishes: 'стравах',
    pf_takes: 'Один продаж списує зі складу', pf_takesHint: 'Сировина, розгорнута через кожен напівфабрикат, на одну порцію, з точністю до міліграма.',
    pf_takesTotal: 'Собівартість порції', pf_takesNone: 'У цієї страви немає рецепта.', pf_refused: 'Не розгортається',
    pf_deleteUsed: 'Ще використовується. Видалення прибере рядок з кожної картки та рецепта нижче:', pf_qtyBad: 'Кількість — від 1 до 100 000, ціла.',
    pf_yieldBad: 'Вихід — від 1 до 100 000, цілий.', pf_count: 'напівфабрикатів', pf_saved: 'Збережено; страви оновлено',
  },
  ru: {
    kind_raw: 'Сырьё', kind_prep: 'Полуфабрикат', kind_dish: 'Блюда', pf_toMenu: 'Блюда — во вкладке Меню',
    pf_title: 'Полуфабрикат', pf_add: 'Добавить полуфабрикат', pf_edit: 'Изменить полуфабрикат',
    pf_hint: 'Готовится на кухне из сырья и других полуфабрикатов. Введите массы брутто и выход готового: себестоимость и списание блюд считаются сами.',
    pf_lines: 'Состав (брутто)', pf_addLine: 'Добавить компонент', pf_pick: 'Выберите сырьё или полуфабрикат', pf_noLines: 'Добавьте хотя бы один компонент.',
    pf_yield: 'Выход готового (нетто)', pf_yieldHint: 'Сколько весит весь замес в готовом виде, в единице выше.',
    pf_k: 'Коэффициент K', pf_kHint: 'Выход / сумма брутто. Меньше 100 % — уваривается; больше 100 % — впитывает воду (рис).', pf_kUnknown: 'неизвестен (штука без веса)',
    pf_batchCost: 'Себестоимость замеса', pf_costPerKg: 'за кг', pf_costPerL: 'за л', pf_costPerPiece: 'за шт', pf_costUnknown: 'себестоимость неизвестна (компонент без цены)',
    pf_lineCost: 'Себестоимость', pf_gross: 'брутто', pf_untracked: 'Не на складе (вода)', pf_untrackedHint: 'Остаётся в рецепте и в K, но никогда не списывается со склада.',
    pf_usedIn: 'Используется в', pf_usedNowhere: 'Пока нигде не используется.', pf_preps: 'полуфабрикатах', pf_dishes: 'блюдах',
    pf_takes: 'Одна продажа списывает со склада', pf_takesHint: 'Сырьё, развёрнутое через каждый полуфабрикат, на одну порцию, с точностью до миллиграмма.',
    pf_takesTotal: 'Себестоимость порции', pf_takesNone: 'У этого блюда нет рецепта.', pf_refused: 'Не разворачивается',
    pf_deleteUsed: 'Ещё используется. Удаление уберёт строку из каждой карты и рецепта ниже:', pf_qtyBad: 'Количество — от 1 до 100 000, целое.',
    pf_yieldBad: 'Выход — от 1 до 100 000, целый.', pf_count: 'полуфабрикатов', pf_saved: 'Сохранено; блюда обновлены',
  },
};

for (const [lang, words] of Object.entries(WORDS)) Object.assign(T[lang] ||= {}, words);
