// The money sheet's words (W-FINANCE), in the four languages: merged into T at load.
//
// ASCII QUOTES ONLY in this file, and no apostrophes inside the words: a
// typographic quote once took down the whole console.

import { T, LANGS } from '/admin/i18n.js';

export const WORDS = {
  en: {
    finance: 'Money', financeSub: 'taxes, expenses, plus / minus / net',
    fn_title: 'Money this month', fn_hint: 'Taxes and spending you enter by hand, and what is left: sales minus purchases, taxes, other spending and payroll.',
    fn_profit: 'Plus / minus / net', fn_tax: 'Taxes', fn_other: 'Expenses',
    fn_plus: 'Plus', fn_minus: 'Minus', fn_net: 'Net',
    fn_src_orders: 'Orders', fn_src_till: 'Till sales (ebills)', fn_src_purchases: 'Product purchases (stock)', fn_src_taxes: 'Taxes', fn_src_other: 'Other expenses', fn_src_payroll: 'Payroll',
    fn_noReceipts: 'No priced delivery recorded this month: enter a price when you receive stock.', fn_uncosted: 'Deliveries without a price are not counted:',
    fn_history: 'Month by month', fn_prev: 'Previous month', fn_next: 'Next month', fn_taxTotal: 'Taxes this month', fn_otherTotal: 'Expenses this month',
    fn_add: 'Add', fn_edit: 'Edit', fn_date: 'Date', fn_name: 'What for', fn_amount: 'Amount', fn_exTax: 'e.g. VAT November', fn_exOther: 'e.g. Fuel, Halloween decorations',
    fn_none: 'Nothing this month yet.', fn_noDate: 'Enter the date.', fn_noName: 'Say what it was for.', fn_noAmount: 'Enter the amount as a positive number.',
    fn_archiveError: 'The archive of older orders could not be read: older sales may be missing.',
    fn_undated: 'Deliveries with no date are in no month:', fn_futureDate: 'That date has not happened yet.',
  },
  uk: {
    finance: 'Гроші', financeSub: 'податки, витрати, плюс / мінус / чисті',
    fn_title: 'Гроші за місяць', fn_hint: 'Податки та витрати ви вносите вручну, а тут видно, що лишилось: продажі мінус закупівлі, податки, інші витрати й зарплати.',
    fn_profit: 'Плюс / мінус / чисті', fn_tax: 'Податки', fn_other: 'Витрати',
    fn_plus: 'Плюс', fn_minus: 'Мінус', fn_net: 'Чисті',
    fn_src_orders: 'Замовлення', fn_src_till: 'Продажі каси (ebills)', fn_src_purchases: 'Закупівля продуктів (склад)', fn_src_taxes: 'Податки', fn_src_other: 'Інші витрати', fn_src_payroll: 'Зарплати',
    fn_noReceipts: 'Цього місяця немає приходу з ціною: вкажіть ціну при прийманні товару.', fn_uncosted: 'Приходи без ціни не враховано:',
    fn_history: 'По місяцях', fn_prev: 'Попередній місяць', fn_next: 'Наступний місяць', fn_taxTotal: 'Податки за місяць', fn_otherTotal: 'Витрати за місяць',
    fn_add: 'Додати', fn_edit: 'Змінити', fn_date: 'Дата', fn_name: 'На що', fn_amount: 'Сума', fn_exTax: 'напр. ПДВ за листопад', fn_exOther: 'напр. Бензин, Прикраси на Хеловін',
    fn_none: 'Цього місяця ще нічого немає.', fn_noDate: 'Вкажіть дату.', fn_noName: 'Вкажіть, на що.', fn_noAmount: 'Вкажіть суму додатним числом.',
    fn_archiveError: 'Не вдалося прочитати архів старих замовлень: давніші продажі можуть бути не враховані.',
    fn_undated: 'Приходи без дати не потрапили в жоден місяць:', fn_futureDate: 'Ця дата ще не настала.',
  },
  sq: {
    finance: 'Paratë', financeSub: 'taksat, shpenzimet, plus / minus / neto',
    fn_title: 'Paratë e muajit', fn_hint: 'Taksat dhe shpenzimet i shkruani vetë; këtu shihni çfarë mbetet: shitjet minus blerjet, taksat, shpenzimet e tjera dhe pagat.',
    fn_profit: 'Plus / minus / neto', fn_tax: 'Taksat', fn_other: 'Shpenzimet',
    fn_plus: 'Plus', fn_minus: 'Minus', fn_net: 'Neto',
    fn_src_orders: 'Porositë', fn_src_till: 'Shitjet e arkës (ebills)', fn_src_purchases: 'Blerje produktesh (magazina)', fn_src_taxes: 'Taksat', fn_src_other: 'Shpenzime të tjera', fn_src_payroll: 'Pagat',
    fn_noReceipts: 'Asnjë furnizim me çmim këtë muaj: shënoni çmimin kur merrni mallin.', fn_uncosted: 'Furnizimet pa çmim nuk llogariten:',
    fn_history: 'Muaj pas muaji', fn_prev: 'Muaji i kaluar', fn_next: 'Muaji tjetër', fn_taxTotal: 'Taksat e muajit', fn_otherTotal: 'Shpenzimet e muajit',
    fn_add: 'Shto', fn_edit: 'Ndrysho', fn_date: 'Data', fn_name: 'Për çfarë', fn_amount: 'Shuma', fn_exTax: 'p.sh. TVSH nëntor', fn_exOther: 'p.sh. Benzinë, Zbukurime Halloween',
    fn_none: 'Ende asgjë këtë muaj.', fn_noDate: 'Shkruani datën.', fn_noName: 'Shkruani për çfarë.', fn_noAmount: 'Shkruani shumën si numër pozitiv.',
    fn_archiveError: 'Arkivi i porosive të vjetra nuk u lexua: shitjet e vjetra mund të mungojnë.',
    fn_undated: 'Furnizimet pa datë nuk janë në asnjë muaj:', fn_futureDate: 'Kjo datë nuk ka ardhur ende.',
  },
  ru: {
    finance: 'Деньги', financeSub: 'налоги, расходы, плюс / минус / чистые',
    fn_title: 'Деньги за месяц', fn_hint: 'Налоги и расходы вы вносите вручную, а здесь видно, что осталось: продажи минус закупки, налоги, прочие расходы и зарплаты.',
    fn_profit: 'Плюс / минус / чистые', fn_tax: 'Налоги', fn_other: 'Расходы',
    fn_plus: 'Плюс', fn_minus: 'Минус', fn_net: 'Чистые',
    fn_src_orders: 'Заказы', fn_src_till: 'Продажи кассы (ebills)', fn_src_purchases: 'Закупка продуктов (склад)', fn_src_taxes: 'Налоги', fn_src_other: 'Прочие расходы', fn_src_payroll: 'Зарплаты',
    fn_noReceipts: 'В этом месяце нет прихода с ценой: указывайте цену при приёмке товара.', fn_uncosted: 'Приходы без цены не учтены:',
    fn_history: 'По месяцам', fn_prev: 'Предыдущий месяц', fn_next: 'Следующий месяц', fn_taxTotal: 'Налоги за месяц', fn_otherTotal: 'Расходы за месяц',
    fn_add: 'Добавить', fn_edit: 'Изменить', fn_date: 'Дата', fn_name: 'На что', fn_amount: 'Сумма', fn_exTax: 'напр. НДС за ноябрь', fn_exOther: 'напр. Бензин, Украшения на Хэллоуин',
    fn_none: 'В этом месяце пока ничего нет.', fn_noDate: 'Укажите дату.', fn_noName: 'Укажите, на что.', fn_noAmount: 'Укажите сумму положительным числом.',
    fn_archiveError: 'Не удалось прочитать архив старых заказов: более давние продажи могут быть не учтены.',
    fn_undated: 'Приходы без даты не попали ни в один месяц:', fn_futureDate: 'Эта дата ещё не наступила.',
  },
};

for (const l of LANGS) if (T[l] && WORDS[l]) Object.assign(T[l], WORDS[l]);
