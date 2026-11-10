// The words of the payroll tab (W-PAYROLL). Imported by more.js (the row's own
// label) and payroll.js. Merged into the shared table on import, never
// replacing a key it has.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console.

import { T, LANGS } from '/admin/i18n.js';

export const WORDS = {
  sq: {
    payroll: 'Paga', payrollSub: 'orët e punës dhe paga',
    pay_hint: 'Zgjidhni punonjësin, datën dhe orët. Paga llogaritet nga tarifa për orë.',
    pay_add: 'Shto orë', pay_employee: 'Punonjësi', pay_date: 'Data', pay_hours: 'Orë', pay_hoursHint: 'Si 8, 7.5 ose 7:30 (çerekët e orës)',
    pay_note: 'Shënim (opsional)', pay_total: 'Gjithsej', pay_sumHours: 'Orë', pay_sumPay: 'Paga',
    pay_noStaff: 'Ende asnjë punonjës', pay_noEntries: 'Asnjë orë këtë muaj', pay_rate: 'Tarifa për orë', pay_rateHint: 'Vlen për hyrjet e reja; hyrjet e vjetra mbajnë tarifën e tyre.',
    pay_rateFirst: 'Vendosni fillimisht tarifën për orë.', pay_setRate: 'Vendos tarifën', pay_badHours: 'Orët duhet të jenë në çerekë ore, deri në 24', pay_badRate: 'Shkruani një tarifë më të madhe se zero',
    pay_history: 'Paga sipas muajve', pay_noHistory: 'Ende asnjë muaj', pay_replaces: 'zëvendëson hyrjen e kësaj dite', pay_delete: 'Fshi', pay_deleteAsk: 'Të fshihet kjo ditë?',
    pay_back: 'Mbrapa', pay_prev: 'Muaji i kaluar', pay_next: 'Muaji tjetër', pay_open: 'Paga', pay_hoursShort: 'orë',
    pay_dateOutside: 'Data duhet të jetë brenda muajit në ekran dhe jo pas ditës së sotme.', pay_gone: 'Ky person nuk është më në staf: ditët e shënuara mund të lexohen dhe të fshihen, jo të ndryshohen.',
  },
  en: {
    payroll: 'Payroll', payrollSub: 'hours worked and pay',
    pay_hint: 'Pick an employee, a date and the hours. Pay follows from the hourly rate.',
    pay_add: 'Add hours', pay_employee: 'Employee', pay_date: 'Date', pay_hours: 'Hours', pay_hoursHint: 'Like 8, 7.5 or 7:30 (quarters of an hour)',
    pay_note: 'Note (optional)', pay_total: 'Total', pay_sumHours: 'Hours', pay_sumPay: 'Pay',
    pay_noStaff: 'No staff yet', pay_noEntries: 'No hours this month', pay_rate: 'Hourly rate', pay_rateHint: 'Applies to new entries; entries already made keep their rate.',
    pay_rateFirst: 'Set the hourly rate first.', pay_setRate: 'Set rate', pay_badHours: 'Hours must be quarters of an hour, up to 24', pay_badRate: 'Enter a rate above zero',
    pay_history: 'Pay by month', pay_noHistory: 'No months yet', pay_replaces: 'replaces this day', pay_delete: 'Delete', pay_deleteAsk: 'Delete this day?',
    pay_back: 'Back', pay_prev: 'Previous month', pay_next: 'Next month', pay_open: 'Payroll', pay_hoursShort: 'h',
    pay_dateOutside: 'The date must be inside the month on screen and not after today.', pay_gone: 'This person is no longer on the staff: the days already entered can be read and deleted, not changed.',
  },
  uk: {
    payroll: 'Зарплата', payrollSub: 'відпрацьовані години та зарплата',
    pay_hint: 'Оберіть працівника, дату й години. Зарплата рахується за погодинною ставкою.',
    pay_add: 'Додати години', pay_employee: 'Працівник', pay_date: 'Дата', pay_hours: 'Години', pay_hoursHint: 'Наприклад 8, 7.5 або 7:30 (чверті години)',
    pay_note: 'Примітка (за бажанням)', pay_total: 'Разом', pay_sumHours: 'Години', pay_sumPay: 'Зарплата',
    pay_noStaff: 'Персоналу ще немає', pay_noEntries: 'Цього місяця годин немає', pay_rate: 'Погодинна ставка', pay_rateHint: 'Діє для нових записів; вже внесені зберігають свою ставку.',
    pay_rateFirst: 'Спершу вкажіть погодинну ставку.', pay_setRate: 'Вказати ставку', pay_badHours: 'Години мають бути чвертями години, до 24', pay_badRate: 'Вкажіть ставку більшу за нуль',
    pay_history: 'Зарплата за місяцями', pay_noHistory: 'Місяців ще немає', pay_replaces: 'замінює запис цього дня', pay_delete: 'Видалити', pay_deleteAsk: 'Видалити цей день?',
    pay_back: 'Назад', pay_prev: 'Попередній місяць', pay_next: 'Наступний місяць', pay_open: 'Зарплата', pay_hoursShort: 'год',
    pay_dateOutside: 'Дата має бути в межах місяця на екрані й не пізніше сьогодні.', pay_gone: 'Цієї людини вже немає серед персоналу: внесені дні можна переглянути й видалити, але не змінити.',
  },
  ru: {
    payroll: 'Зарплата', payrollSub: 'отработанные часы и зарплата',
    pay_hint: 'Выберите сотрудника, дату и часы. Зарплата считается по часовой ставке.',
    pay_add: 'Добавить часы', pay_employee: 'Сотрудник', pay_date: 'Дата', pay_hours: 'Часы', pay_hoursHint: 'Например 8, 7.5 или 7:30 (четверти часа)',
    pay_note: 'Заметка (необязательно)', pay_total: 'Итого', pay_sumHours: 'Часы', pay_sumPay: 'Зарплата',
    pay_noStaff: 'Персонала ещё нет', pay_noEntries: 'В этом месяце часов нет', pay_rate: 'Часовая ставка', pay_rateHint: 'Действует для новых записей; уже внесённые сохраняют свою ставку.',
    pay_rateFirst: 'Сначала укажите часовую ставку.', pay_setRate: 'Указать ставку', pay_badHours: 'Часы должны быть четвертями часа, до 24', pay_badRate: 'Укажите ставку больше нуля',
    pay_history: 'Зарплата по месяцам', pay_noHistory: 'Месяцев ещё нет', pay_replaces: 'заменяет запись этого дня', pay_delete: 'Удалить', pay_deleteAsk: 'Удалить этот день?',
    pay_back: 'Назад', pay_prev: 'Предыдущий месяц', pay_next: 'Следующий месяц', pay_open: 'Зарплата', pay_hoursShort: 'ч',
    pay_dateOutside: 'Дата должна быть в пределах месяца на экране и не позже сегодняшнего дня.', pay_gone: 'Этого человека уже нет среди персонала: внесённые дни можно посмотреть и удалить, но не изменить.',
  },
};
for (const l of LANGS) for (const [key, v] of Object.entries({ ...WORDS.en, ...(WORDS[l] || {}) })) if (T[l] && !(key in T[l])) T[l][key] = v;
