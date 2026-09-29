// The words of lane W-PF2 (2026-09-29): the recipes import that creates
// semi-finished products, and the PRODUCTION ACT (akt pripravy) -- a batch of
// a semi-finished product cooked ahead: raw items off the shelf by its card,
// the batch onto it, the loss on cooking shown. In the lane's own dictionary
// (never admin/i18n.js), merged into the console's T once, on import.
//
// ASCII QUOTES ONLY as string delimiters (DOWIZ-COMMON-RULES rule 11);
// apostrophes inside words use U+02BC.

import { T } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    bulkPreps: 'Gjysmëprodukte', bulkPrepUpdate: 'përditësohet',
    bulkPrepsHint: 'Krijohen si gjysmëprodukte të vërtetë: pjatat i emërtojnë, ndaj kur ndryshoni një kartë ndryshojnë të gjitha pjatat.',
    pf_cook: 'Gatuaj një përgatitje', pf_cookHint: 'Lëndët e para dalin nga magazina sipas kartës, përgatitja hyn në magazinë.',
    pf_cookQty: 'Sa sipas kartës', pf_cookQtyHint: 'Sasia gati që parashikon karta për lëndët që përdorët.',
    pf_cookOut: 'Sa doli, e peshuar', pf_cookOutHint: 'Bosh: aq sa parashikon karta.',
    pf_cookDone: 'U regjistrua', pf_onShelf: 'Në magazinë', pf_cookLoss: 'Humbje në gatim', pf_cookDrawn: 'Doli nga magazina',
    pf_cookValue: 'Kostoja e përgatitjes', pf_notStocked: 'Nuk mbahet gati: çdo shitje merr lëndët e para.',
    pf_stockFirst: 'Shitja merr së pari nga përgatitja gati; kur mbaron, merr lëndët e para.',
  },
  en: {
    bulkPreps: 'Semi-finished products', bulkPrepUpdate: 'update',
    bulkPrepsHint: 'Created as real semi-finished products: the dishes name them, so editing one card updates every dish.',
    pf_cook: 'Cook a batch', pf_cookHint: 'The raw items leave the shelf by the card; the batch goes onto it.',
    pf_cookQty: 'How much by the card', pf_cookQtyHint: 'The ready amount the card makes from what you used.',
    pf_cookOut: 'How much came out, weighed', pf_cookOutHint: 'Empty: what the card says.',
    pf_cookDone: 'Recorded', pf_onShelf: 'On the shelf', pf_cookLoss: 'Loss on cooking', pf_cookDrawn: 'Taken off the shelf',
    pf_cookValue: 'Batch cost', pf_notStocked: 'Not kept ready: every sale takes the raw items.',
    pf_stockFirst: 'A sale takes the ready batch first; when it runs out, the raw items.',
  },
  uk: {
    bulkPreps: 'Напівфабрикати', bulkPrepUpdate: 'оновлення',
    bulkPrepsHint: 'Створюються як справжні напівфабрикати: страви посилаються на них, тож зміна однієї картки оновлює всі страви.',
    pf_cook: 'Приготувати партію', pf_cookHint: 'Сировина списується зі складу за карткою, партія надходить на склад.',
    pf_cookQty: 'Скільки за карткою', pf_cookQtyHint: 'Вихід готового, який дає картка з використаної сировини.',
    pf_cookOut: 'Скільки вийшло, зважено', pf_cookOutHint: 'Порожньо: як у картці.',
    pf_cookDone: 'Записано', pf_onShelf: 'На складі', pf_cookLoss: 'Втрати при приготуванні', pf_cookDrawn: 'Списано зі складу',
    pf_cookValue: 'Собівартість партії', pf_notStocked: 'Не зберігається готовим: кожен продаж списує сировину.',
    pf_stockFirst: 'Продаж спершу бере готову партію; коли вона закінчиться — сировину.',
  },
  ru: {
    bulkPreps: 'Полуфабрикаты', bulkPrepUpdate: 'обновление',
    bulkPrepsHint: 'Создаются как настоящие полуфабрикаты: блюда ссылаются на них, поэтому изменение одной карты обновляет все блюда.',
    pf_cook: 'Приготовить партию', pf_cookHint: 'Сырьё списывается со склада по карте, партия поступает на склад.',
    pf_cookQty: 'Сколько по карте', pf_cookQtyHint: 'Выход готового, который даёт карта из использованного сырья.',
    pf_cookOut: 'Сколько вышло, взвешено', pf_cookOutHint: 'Пусто: как в карте.',
    pf_cookDone: 'Записано', pf_onShelf: 'На складе', pf_cookLoss: 'Потери при приготовлении', pf_cookDrawn: 'Списано со склада',
    pf_cookValue: 'Себестоимость партии', pf_notStocked: 'Не хранится готовым: каждая продажа списывает сырьё.',
    pf_stockFirst: 'Продажа сначала берёт готовую партию; когда она закончится — сырьё.',
  },
};

for (const [lang, words] of Object.entries(WORDS)) Object.assign(T[lang] ||= {}, words);
