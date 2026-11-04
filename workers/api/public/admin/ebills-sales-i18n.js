// The till tab's words (W-EBILLS-TAB), in the four languages. The screen's
// own file, like analytics-i18n.js keeps its words: merged into T at load.
//
// ASCII QUOTES ONLY in this file, and no apostrophes inside the words: a
// typographic quote once took down the whole console.

import { T, LANGS } from '/admin/i18n.js';

export const WORDS = {
  en: {
    es_title: 'Till sales (ebills)', es_hint: 'Every sale the till rang up in the last 30 days, as the hub imported it, and the numbers over them. Read-only: nothing here writes to ebills.',
    es_list: 'Orders', es_numbers: 'Numbers', es_orders30: 'Orders, 30 days', es_revenue30: 'Revenue, 30 days', es_avgCheck: 'Average check', es_unmatchedShare: 'No-dish share',
    es_byDay: 'Revenue by day', es_byHour: 'Orders by hour', es_byWeekday: 'By weekday', es_payment: 'Payment', es_kind: 'Where', es_dineIn: 'At a table', es_counter: 'At the counter',
    es_tables: 'Tables', es_operators: 'Till operators', es_operator: 'Operator', es_topItems: 'Top dishes (till lines)', es_unmatched: 'Till names with no dish',
    es_unmatchedHint: 'Counted in revenue, but no stock is written off: no dish of that name, or two of them. Match the code in the till link sheet.',
    es_matchedMap: 'matched by you', es_matchedName: 'matched by name', es_open: 'no dish', es_unpaid: 'courses not yet paid by a bill', es_notTaken: 'voided / refunded',
    es_search: 'Search: sale, table, dish', es_none: 'No till sale in the last 30 days.', es_sale: 'Sale', es_invoice: 'Invoice', es_fic: 'FIC', es_table: 'Table',
    es_paidByBill: 'paid', es_unpaidCourse: 'bill not yet in', es_changed: 'changed at the till after import', es_void: 'void of sale', es_export: 'Export CSV',
    es_openLink: 'Till link settings', es_lines: 'lines', es_portions: 'portions', es_all: 'All days', es_cash: 'Cash', es_card: 'Card', es_unknownPay: 'Other',
    es_noWriteOff: 'Stock is written off only for lines whose dish has a recipe, at the moment of import; a dish matched later does not write off past sales.',
    es_seeSales: 'Open the till sales tab', es_seeSalesSub: 'every sale of the last 30 days, with numbers',
  },
  sq: {
    es_title: 'Shitjet e arkes (ebills)', es_hint: 'Cdo shitje qe arka regjistroi ne 30 ditet e fundit, sic e importoi qendra, dhe shifrat mbi to. Vetem lexim: asgje ketu nuk shkruan ne ebills.',
    es_list: 'Porosite', es_numbers: 'Shifrat', es_orders30: 'Porosi, 30 dite', es_revenue30: 'Te ardhura, 30 dite', es_avgCheck: 'Fatura mesatare', es_unmatchedShare: 'Pjesa pa pjate',
    es_byDay: 'Te ardhurat sipas dites', es_byHour: 'Porosite sipas ores', es_byWeekday: 'Sipas dites se javes', es_payment: 'Pagesa', es_kind: 'Ku', es_dineIn: 'Ne tavoline', es_counter: 'Ne banak',
    es_tables: 'Tavolinat', es_operators: 'Operatoret e arkes', es_operator: 'Operatori', es_topItems: 'Pjatat kryesore (rreshtat e arkes)', es_unmatched: 'Emra te arkes pa pjate',
    es_unmatchedHint: 'Numerohen ne te ardhura, por nuk zbresin magazinen: nuk ka pjate me kete emer, ose ka dy. Lidhni kodin ne faqen e lidhjes me arken.',
    es_matchedMap: 'lidhur nga ju', es_matchedName: 'lidhur sipas emrit', es_open: 'pa pjate', es_unpaid: 'pjata ende pa fature', es_notTaken: 'anuluar / rimbursuar',
    es_search: 'Kerko: shitje, tavoline, pjate', es_none: 'Asnje shitje e arkes ne 30 ditet e fundit.', es_sale: 'Shitja', es_invoice: 'Fatura', es_fic: 'FIC', es_table: 'Tavolina',
    es_paidByBill: 'paguar', es_unpaidCourse: 'fatura ende nuk ka ardhur', es_changed: 'ndryshuar ne arke pas importit', es_void: 'anulim i shitjes', es_export: 'Eksporto CSV',
    es_openLink: 'Cilesimet e lidhjes me arken', es_lines: 'rreshta', es_portions: 'porcione', es_all: 'Te gjitha ditet', es_cash: 'Kesh', es_card: 'Karte', es_unknownPay: 'Tjeter',
    es_noWriteOff: 'Magazina zbritet vetem per rreshtat pjata e te cileve ka recete, ne castin e importit; nje pjate e lidhur me vone nuk zbret shitjet e kaluara.',
    es_seeSales: 'Hap faqen e shitjeve te arkes', es_seeSalesSub: 'cdo shitje e 30 diteve te fundit, me shifra',
  },
  uk: {
    es_title: 'Продажі каси (ebills)', es_hint: 'Кожен продаж, який каса пробила за останні 30 днів, як його імпортував хаб, і цифри по них. Лише читання: нічого тут не пише в ebills.',
    es_list: 'Замовлення', es_numbers: 'Цифри', es_orders30: 'Замовлень, 30 днів', es_revenue30: 'Виручка, 30 днів', es_avgCheck: 'Середній чек', es_unmatchedShare: 'Частка без страви',
    es_byDay: 'Виручка по днях', es_byHour: 'Замовлення по годинах', es_byWeekday: 'По днях тижня', es_payment: 'Оплата', es_kind: 'Де', es_dineIn: 'За столом', es_counter: 'Біля каси',
    es_tables: 'Столи', es_operators: 'Оператори каси', es_operator: 'Оператор', es_topItems: 'Топ страв (рядки каси)', es_unmatched: 'Назви з каси без страви',
    es_unmatchedHint: 'Рахуються у виручці, але склад не списується: немає страви з такою назвою або їх дві. Зіставте код у налаштуваннях каси.',
    es_matchedMap: 'зіставили ви', es_matchedName: 'зіставлено за назвою', es_open: 'без страви', es_unpaid: 'страв ще без рахунку', es_notTaken: 'скасовано / повернуто',
    es_search: 'Пошук: продаж, стіл, страва', es_none: 'Жодного продажу каси за останні 30 днів.', es_sale: 'Продаж', es_invoice: 'Рахунок', es_fic: 'FIC', es_table: 'Стіл',
    es_paidByBill: 'оплачено', es_unpaidCourse: 'рахунок ще не прийшов', es_changed: 'змінено на касі після імпорту', es_void: 'скасування продажу', es_export: 'Експорт CSV',
    es_openLink: 'Налаштування каси', es_lines: 'рядків', es_portions: 'порцій', es_all: 'Усі дні', es_cash: 'Готівка', es_card: 'Картка', es_unknownPay: 'Інше',
    es_noWriteOff: 'Склад списується лише за рядками, страва яких має техкарту, у момент імпорту; страва, зіставлена пізніше, не списує минулі продажі.',
    es_seeSales: 'Відкрити вкладку продажів каси', es_seeSalesSub: 'кожен продаж за останні 30 днів, з цифрами',
  },
  ru: {
    es_title: 'Продажи кассы (ebills)', es_hint: 'Каждая продажа, которую касса пробила за последние 30 дней, как ее импортировал хаб, и цифры по ним. Только чтение: ничего здесь не пишет в ebills.',
    es_list: 'Заказы', es_numbers: 'Цифры', es_orders30: 'Заказов, 30 дней', es_revenue30: 'Выручка, 30 дней', es_avgCheck: 'Средний чек', es_unmatchedShare: 'Доля без блюда',
    es_byDay: 'Выручка по дням', es_byHour: 'Заказы по часам', es_byWeekday: 'По дням недели', es_payment: 'Оплата', es_kind: 'Где', es_dineIn: 'За столом', es_counter: 'У кассы',
    es_tables: 'Столы', es_operators: 'Операторы кассы', es_operator: 'Оператор', es_topItems: 'Топ блюд (строки кассы)', es_unmatched: 'Названия с кассы без блюда',
    es_unmatchedHint: 'Считаются в выручке, но склад не списывается: нет блюда с таким названием или их два. Сопоставьте код на листе связи с кассой.',
    es_matchedMap: 'сопоставили вы', es_matchedName: 'сопоставлено по названию', es_open: 'без блюда', es_unpaid: 'блюд еще без счета', es_notTaken: 'отменено / возвращено',
    es_search: 'Поиск: продажа, стол, блюдо', es_none: 'Ни одной продажи кассы за последние 30 дней.', es_sale: 'Продажа', es_invoice: 'Счет', es_fic: 'FIC', es_table: 'Стол',
    es_paidByBill: 'оплачено', es_unpaidCourse: 'счет еще не пришел', es_changed: 'изменено на кассе после импорта', es_void: 'отмена продажи', es_export: 'Экспорт CSV',
    es_openLink: 'Настройки связи с кассой', es_lines: 'строк', es_portions: 'порций', es_all: 'Все дни', es_cash: 'Наличные', es_card: 'Карта', es_unknownPay: 'Другое',
    es_noWriteOff: 'Склад списывается только по строкам, у блюда которых есть техкарта, в момент импорта; блюдо, сопоставленное позже, не списывает прошлые продажи.',
    es_seeSales: 'Открыть вкладку продаж кассы', es_seeSalesSub: 'каждая продажа за последние 30 дней, с цифрами',
  },
};

for (const l of LANGS) if (T[l] && WORDS[l]) Object.assign(T[l], WORDS[l]);
