// W-OCR's words (2026-10-04): the invoice sheet (photo and e-invoice file).
// One dictionary with NO import, so a node test reads it as it is;
// `ingredients.js` merges it into the console's T on import, so the Stock
// screen's tools row has its word before the sheet opens.
//
// ASCII QUOTES ONLY as string delimiters; an apostrophe inside a word is U+02BC.

export const WORDS = {
  sq: {
    rp_title: 'Fatura në magazinë', rp_tool: 'Nga fatura',
    rp_hint: 'Fotografoni faturën e furnitorit ose ngarkoni e-faturën (XML). Kontrolloni çdo rresht: leximi është vetëm propozim deri sa ta konfirmoni. Fotoja lexohet në telefon, nuk dërgohet askund.',
    rp_supplier: 'Furnitori', rp_pickSupplier: 'Zgjidhni furnitorin', rp_noCards: 'Krijoni së pari kartelën e furnitorit: aty mbahen emrat e rreshtave të tij.',
    rp_photo: 'Nga një foto', rp_file: 'Skedar e-fature', rp_loading: 'Po ngarkohet lexuesi (rreth 5 MB, vetëm herën e parë)...', rp_reading: 'Po lexohet',
    rp_check: 'Kontrolloni çdo rresht dhe përbërësin e tij.', rp_noLines: 'Asnjë rresht nuk u lexua. Provoni një foto më të drejtë dhe më të ndriçuar.',
    rp_skip: 'Mos e merr', rp_byAlias: 'e mbajtur mend', rp_byName: 'propozim',
    rp_f_mismatch: 'sasia x çmimi nuk jep vlerën', rp_f_no_price: 'pa çmim njësie', rp_f_rounded: 'qindarkat u rrumbullakuan', rp_f_unit: 'njësi e panjohur',
    rp_confirm: 'Konfirmo dhe merr në magazinë', rp_wrote: 'Rreshta të marrë', rp_pickSupply: 'Zgjidhni përbërësin', rp_badQty: 'Sasia nuk përputhet me njësinë e përbërësit',
    ei_bad: 'Skedari nuk është XML i vlefshëm', ei_notInvoice: 'Ky nuk është e-faturë (UBL Invoice)', ei_credit: 'Kjo është notë krediti, jo faturë',
    ei_currency: 'Fatura nuk është në lekë', ei_line: 'Një rresht i faturës nuk ka sasi, vlerë ose emër', ei_pdfNoXml: 'Ky PDF nuk ka e-faturë brenda: përdorni foton',
  },
  en: {
    rp_title: 'Invoice into stock', rp_tool: 'From an invoice',
    rp_hint: 'Photograph the supplier invoice or upload the e-invoice (XML). Check every line: what is read is a proposal until you confirm it. The photo is read on this phone and sent nowhere.',
    rp_supplier: 'Supplier', rp_pickSupplier: 'Choose the supplier', rp_noCards: 'Make the supplier card first: it remembers what their lines are called.',
    rp_photo: 'From a photo', rp_file: 'E-invoice file', rp_loading: 'Loading the reader (about 5 MB, the first time only)...', rp_reading: 'Reading',
    rp_check: 'Check every line and its ingredient.', rp_noLines: 'No lines were read. Try a straighter, brighter photo.',
    rp_skip: 'Do not receive', rp_byAlias: 'remembered', rp_byName: 'suggested',
    rp_f_mismatch: 'qty x price is not the amount', rp_f_no_price: 'no unit price', rp_f_rounded: 'qindarka rounded', rp_f_unit: 'unknown unit',
    rp_confirm: 'Confirm and receive', rp_wrote: 'Lines received', rp_pickSupply: 'Choose the ingredient', rp_badQty: 'The quantity does not fit the ingredient unit',
    ei_bad: 'The file is not valid XML', ei_notInvoice: 'This is not an e-invoice (UBL Invoice)', ei_credit: 'This is a credit note, not an invoice',
    ei_currency: 'The invoice is not in lek', ei_line: 'An invoice line has no quantity, amount or name', ei_pdfNoXml: 'This PDF has no e-invoice inside: use the photo',
  },
  uk: {
    rp_title: 'Накладна на склад', rp_tool: 'З накладної',
    rp_hint: 'Сфотографуйте накладну постачальника або завантажте е-фактуру (XML). Перевірте кожен рядок: прочитане лише пропозиція, доки ви не підтвердите. Фото читається на телефоні й нікуди не надсилається.',
    rp_supplier: 'Постачальник', rp_pickSupplier: 'Оберіть постачальника', rp_noCards: 'Спершу створіть картку постачальника: вона памʼятає, як звуться його рядки.',
    rp_photo: 'З фото', rp_file: 'Файл е-фактури', rp_loading: 'Завантажується читач (близько 5 МБ, лише вперше)...', rp_reading: 'Читаю',
    rp_check: 'Перевірте кожен рядок і його інгредієнт.', rp_noLines: 'Жодного рядка не прочитано. Спробуйте рівніше й світліше фото.',
    rp_skip: 'Не приймати', rp_byAlias: 'запамʼятовано', rp_byName: 'пропозиція',
    rp_f_mismatch: 'кількість x ціна не дає суму', rp_f_no_price: 'без ціни за одиницю', rp_f_rounded: 'кіндарки округлено', rp_f_unit: 'невідома одиниця',
    rp_confirm: 'Підтвердити й прийняти', rp_wrote: 'Прийнято рядків', rp_pickSupply: 'Оберіть інгредієнт', rp_badQty: 'Кількість не пасує до одиниці інгредієнта',
    ei_bad: 'Файл не є коректним XML', ei_notInvoice: 'Це не е-фактура (UBL Invoice)', ei_credit: 'Це кредит-нота, а не фактура',
    ei_currency: 'Фактура не в леках', ei_line: 'У рядку фактури немає кількості, суми чи назви', ei_pdfNoXml: 'У цьому PDF немає е-фактури: скористайтеся фото',
  },
  ru: {
    rp_title: 'Накладная на склад', rp_tool: 'Из накладной',
    rp_hint: 'Сфотографируйте накладную поставщика или загрузите э-фактуру (XML). Проверьте каждую строку: прочитанное лишь предложение, пока вы не подтвердите. Фото читается на телефоне и никуда не отправляется.',
    rp_supplier: 'Поставщик', rp_pickSupplier: 'Выберите поставщика', rp_noCards: 'Сначала создайте карточку поставщика: она помнит, как называются его строки.',
    rp_photo: 'С фото', rp_file: 'Файл э-фактуры', rp_loading: 'Загружается читатель (около 5 МБ, только в первый раз)...', rp_reading: 'Читаю',
    rp_check: 'Проверьте каждую строку и её ингредиент.', rp_noLines: 'Ни одной строки не прочитано. Попробуйте более ровное и светлое фото.',
    rp_skip: 'Не принимать', rp_byAlias: 'запомнено', rp_byName: 'предложение',
    rp_f_mismatch: 'количество x цена не даёт сумму', rp_f_no_price: 'без цены за единицу', rp_f_rounded: 'киндарки округлены', rp_f_unit: 'неизвестная единица',
    rp_confirm: 'Подтвердить и принять', rp_wrote: 'Принято строк', rp_pickSupply: 'Выберите ингредиент', rp_badQty: 'Количество не подходит к единице ингредиента',
    ei_bad: 'Файл не является корректным XML', ei_notInvoice: 'Это не э-фактура (UBL Invoice)', ei_credit: 'Это кредит-нота, а не фактура',
    ei_currency: 'Фактура не в леках', ei_line: 'В строке фактуры нет количества, суммы или названия', ei_pdfNoXml: 'В этом PDF нет э-фактуры: используйте фото',
  },
};
