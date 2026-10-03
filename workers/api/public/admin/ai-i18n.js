// The owner's AI's words (lane W-AI, 2026-10-03), merged into the console's
// table at import like `assistant-i18n.js`. The `ai_q_*` starters are SENT as
// typed, so each is written in words the hub's lexicon reads
// (`services/engagement/ai/lexicon.rs`, pinned by `ai-logic.test.mjs`).
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { T } from '/admin/i18n.js';
import { merge } from '/admin/kitchen-i18n.js';

export const WORDS = {
  sq: {
    ai_free_note: 'Modelet falas kanë kufij (rreth 50 pyetje në ditë) dhe disa ofrues mund t\'i ruajnë pyetjet. Dowiz u dërgon numra dhe porosi pa emrat, telefonat ose adresat e klientëve.',
    ai_title: 'AI', ai_hint: 'Pyetni për numrat e lokalit. Çdo numër vjen nga regjistrat tuaj dhe hapet me një prekje.',
    ai_ask_h: 'Pyetni numrat', ai_ask_ph: 'p.sh. Cila pjatë u shit më pak të hënën?', ai_send: 'Pyet',
    ai_q_revenue: 'Sa të ardhura këtë javë?', ai_q_best: 'Cila pjatë u shit më shumë këtë javë?', ai_q_worst_monday: 'Cila pjatë u shit më pak të hënën?',
    ai_q_hour: 'Cila është ora më e ngarkuar këtë muaj?', ai_q_food_cost: 'Sa është kostoja e ushqimit këtë javë?', ai_q_low: 'Çfarë po mbaron?',
    ai_source: 'Burimi', ai_records: 'Regjistrat e ditës', ai_by_model: 'Pyetjen e lexoi AI', ai_reworded: 'Fjalët nga AI; numrat u kontrolluan',
    ai_not_understood: 'Nuk e kuptova.', ai_settings_h: 'Lidhja me AI', ai_provider: 'Kush përgjigjet',
    ai_mode_auto: 'Çelësi im, pastaj Workers AI', ai_mode_own: 'Vetëm çelësi im', ai_mode_workers: 'Vetëm Workers AI',
    ai_openrouter: 'Përdor OpenRouter (modele falas)', ai_get_key: 'Merrni një çelës falas', ai_key_set: 'Një çelës është ruajtur',
    ai_key_clear: 'Hiq çelësin', ai_test: 'Provo lidhjen', ai_budget: 'Pjesa juaj e Workers AI sot', ai_neurons: 'neurone',
    ai_state_ok: 'Lidhja punon.', ai_state_needs_key: 'Duhet një çelës.', ai_state_no_binding: 'Workers AI nuk është i disponueshëm.',
    ai_state_budget: 'Pjesa e sotme mbaroi; rinis në mesnatë UTC.', ai_state_disabled: 'AI është i fikur.', ai_state_failed: 'Lidhja dështoi.',
    ai_why_needs_key: 'pa çelës', ai_why_no_binding: 'Workers AI mungon', ai_why_budget: 'pjesa e sotme mbaroi', ai_why_disabled: 'i fikur', ai_why_not_chosen: 'jo i zgjedhur',
    ai_route_own: 'Çelësi juaj', ai_route_workers: 'Workers AI', ai_answered_by: 'U përgjigj',
    ai_explain_h: 'Çfarë thonë numrat', ai_explain_reword: 'Thuaje me AI',
  },
  en: {
    ai_free_note: 'Free models have limits (about 50 questions a day) and some providers may keep the prompts. Dowiz sends them numbers and orders without customers\' names, phones or addresses.',
    ai_title: 'AI', ai_hint: 'Ask about your venue\'s numbers. Every number comes from your own records and opens with one tap.',
    ai_ask_h: 'Ask your numbers', ai_ask_ph: 'e.g. What sold worst on Monday?', ai_send: 'Ask',
    ai_q_revenue: 'What was the revenue this week?', ai_q_best: 'What sold best this week?', ai_q_worst_monday: 'What sold worst on Monday?',
    ai_q_hour: 'What is the busiest hour this month?', ai_q_food_cost: 'What is the food cost this week?', ai_q_low: 'What is running low?',
    ai_source: 'Source', ai_records: 'The day\'s records', ai_by_model: 'AI read the question', ai_reworded: 'Worded by AI; numbers checked',
    ai_not_understood: 'Not understood.', ai_settings_h: 'AI connection', ai_provider: 'Who answers',
    ai_mode_auto: 'My key, then Workers AI', ai_mode_own: 'Only my key', ai_mode_workers: 'Only Workers AI',
    ai_openrouter: 'Use OpenRouter (free models)', ai_get_key: 'Get a free key', ai_key_set: 'A key is saved',
    ai_key_clear: 'Remove the key', ai_test: 'Test the connection', ai_budget: 'Your Workers AI share today', ai_neurons: 'neurons',
    ai_state_ok: 'The connection works.', ai_state_needs_key: 'A key is needed.', ai_state_no_binding: 'Workers AI is unavailable.',
    ai_state_budget: 'Today\'s share is spent; it renews at midnight UTC.', ai_state_disabled: 'AI is off.', ai_state_failed: 'The connection failed.',
    ai_why_needs_key: 'no key', ai_why_no_binding: 'Workers AI missing', ai_why_budget: 'today\'s share spent', ai_why_disabled: 'off', ai_why_not_chosen: 'not chosen',
    ai_route_own: 'Your key', ai_route_workers: 'Workers AI', ai_answered_by: 'Answered by',
    ai_explain_h: 'What the numbers say', ai_explain_reword: 'Word it with AI',
  },
  uk: {
    ai_free_note: 'Безкоштовні моделі мають ліміти (близько 50 запитів на день), і деякі провайдери можуть зберігати запити. Dowiz надсилає цифри й замовлення без імен, телефонів і адрес клієнтів.',
    ai_title: 'ШІ', ai_hint: 'Питайте про цифри закладу. Кожне число береться з ваших записів і відкривається одним дотиком.',
    ai_ask_h: 'Запитайте цифри', ai_ask_ph: 'напр. Що продавалось найгірше в понеділок?', ai_send: 'Запитати',
    ai_q_revenue: 'Яка виручка за тиждень?', ai_q_best: 'Що продавалось найкраще за тиждень?', ai_q_worst_monday: 'Що продавалось найгірше в понеділок?',
    ai_q_hour: 'Яка найзавантаженіша година за місяць?', ai_q_food_cost: 'Який фудкост за тиждень?', ai_q_low: 'Що закінчується?',
    ai_source: 'Джерело', ai_records: 'Записи дня', ai_by_model: 'Питання прочитав ШІ', ai_reworded: 'Слова від ШІ; числа перевірено',
    ai_not_understood: 'Не зрозумів.', ai_settings_h: 'Підключення ШІ', ai_provider: 'Хто відповідає',
    ai_mode_auto: 'Мій ключ, потім Workers AI', ai_mode_own: 'Лише мій ключ', ai_mode_workers: 'Лише Workers AI',
    ai_openrouter: 'Використати OpenRouter (безкоштовні моделі)', ai_get_key: 'Отримати безкоштовний ключ', ai_key_set: 'Ключ збережено',
    ai_key_clear: 'Видалити ключ', ai_test: 'Перевірити підключення', ai_budget: 'Ваша частка Workers AI сьогодні', ai_neurons: 'нейронів',
    ai_state_ok: 'Підключення працює.', ai_state_needs_key: 'Потрібен ключ.', ai_state_no_binding: 'Workers AI недоступний.',
    ai_state_budget: 'Частку на сьогодні витрачено; оновиться опівночі UTC.', ai_state_disabled: 'ШІ вимкнено.', ai_state_failed: 'Підключення не вдалося.',
    ai_why_needs_key: 'немає ключа', ai_why_no_binding: 'немає Workers AI', ai_why_budget: 'частку витрачено', ai_why_disabled: 'вимкнено', ai_why_not_chosen: 'не обрано',
    ai_route_own: 'Ваш ключ', ai_route_workers: 'Workers AI', ai_answered_by: 'Відповів',
    ai_explain_h: 'Що кажуть цифри', ai_explain_reword: 'Сказати через ШІ',
  },
  ru: {
    ai_free_note: 'Бесплатные модели имеют лимиты (около 50 запросов в день), и некоторые провайдеры могут хранить запросы. Dowiz отправляет цифры и заказы без имён, телефонов и адресов клиентов.',
    ai_title: 'ИИ', ai_hint: 'Спрашивайте о цифрах заведения. Каждое число берётся из ваших записей и открывается одним касанием.',
    ai_ask_h: 'Спросите цифры', ai_ask_ph: 'напр. Что продавалось хуже всего в понедельник?', ai_send: 'Спросить',
    ai_q_revenue: 'Какая выручка за неделю?', ai_q_best: 'Что продавалось лучше всего за неделю?', ai_q_worst_monday: 'Что продавалось хуже всего в понедельник?',
    ai_q_hour: 'Какой самый загруженный час за месяц?', ai_q_food_cost: 'Какой фудкост за неделю?', ai_q_low: 'Что заканчивается?',
    ai_source: 'Источник', ai_records: 'Записи дня', ai_by_model: 'Вопрос прочитал ИИ', ai_reworded: 'Слова от ИИ; числа проверены',
    ai_not_understood: 'Не понял.', ai_settings_h: 'Подключение ИИ', ai_provider: 'Кто отвечает',
    ai_mode_auto: 'Мой ключ, затем Workers AI', ai_mode_own: 'Только мой ключ', ai_mode_workers: 'Только Workers AI',
    ai_openrouter: 'Использовать OpenRouter (бесплатные модели)', ai_get_key: 'Получить бесплатный ключ', ai_key_set: 'Ключ сохранён',
    ai_key_clear: 'Удалить ключ', ai_test: 'Проверить подключение', ai_budget: 'Ваша доля Workers AI сегодня', ai_neurons: 'нейронов',
    ai_state_ok: 'Подключение работает.', ai_state_needs_key: 'Нужен ключ.', ai_state_no_binding: 'Workers AI недоступен.',
    ai_state_budget: 'Доля на сегодня израсходована; обновится в полночь UTC.', ai_state_disabled: 'ИИ выключен.', ai_state_failed: 'Подключение не удалось.',
    ai_why_needs_key: 'нет ключа', ai_why_no_binding: 'нет Workers AI', ai_why_budget: 'доля израсходована', ai_why_disabled: 'выключен', ai_why_not_chosen: 'не выбран',
    ai_route_own: 'Ваш ключ', ai_route_workers: 'Workers AI', ai_answered_by: 'Ответил',
    ai_explain_h: 'Что говорят цифры', ai_explain_reword: 'Сказать через ИИ',
  },
};

merge(T, WORDS);
