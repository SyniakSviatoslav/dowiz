// The SMS screen's own words (W-SMS). Merged into the console's dictionary by
// sms.js for every language the console has; a language with no row here
// reads English (never a blank control).
//
// ASCII QUOTES ONLY, and no apostrophes inside the words: a typographic quote
// once took down the whole console.

export const WORDS = {
  sq: {
    sms_hint: 'Klienti që zgjedh kutinë SMS në pagesë merr një SMS kur porosia konfirmohet, është gati ose niset. Vetëm numri i porosisë dhe emri i lokalit.',
    sms_setupTitle: 'Falas me telefonin tuaj Android',
    sms_setup: '1. Instaloni SMS Gateway for Android (SMSGate) nga github.com/capcom6/android-sms-gateway. 2. Lejoni SMS, ndizni Cloud server dhe shtypni Online. 3. Fikni optimizimin e baterisë. 4. Kopjoni këtu emrin e përdoruesit dhe fjalëkalimin që tregon aplikacioni. SMS-të dërgohen nga SIM-i i lokalit, me çmimin e paketës suaj.',
    sms_on: 'Dërgo SMS klientëve', sms_onHint: 'Vetëm atyre që zgjodhën kutinë SMS.',
    sms_provider: 'Mënyra e dërgimit', sms_p_smsgate: 'Telefoni im Android (SMSGate, falas)', sms_p_textbee: 'Telefoni im Android (textbee)', sms_p_twilio: 'Twilio (me pagesë, llogaria juaj)',
    sms_url: 'Adresa e serverit (bosh = cloud i SMSGate)', sms_user: 'Përdoruesi (nga aplikacioni) / Account SID', sms_secret: 'Fjalëkalimi / çelësi API', sms_secretSet: 'i ruajtur; shkruani për ta ndryshuar',
    sms_from: 'Numri dërgues (vetëm Twilio)', sms_daily: 'Kufiri ditor i SMS', sms_dailyHint: 'Mbron SIM-in nga bllokimi i operatorit. Parazgjedhja 60.',
    sms_test: 'Dërgo provë', sms_testPhone: 'Numri juaj për provë', sms_testOk: 'U dërgua. Shikoni telefonin.',
    sms_status: 'Gjendja', sms_live: 'aktiv', sms_failing: 'gabim', sms_sentToday: 'Sot', sms_lastOk: 'I fundit i dërguar', sms_lastErr: 'Gabimi i fundit', sms_waiting: 'në pritje', sms_stopped: 'ndalur nga klienti', sms_overBudget: 'mbi kufi', sms_never: 'ende asgjë',
    sms_stopTitle: 'Klienti tha STOP', sms_stopHint: 'Shkruani numrin e tij. Asnjë SMS tjetër nuk i shkon atij numri.', sms_stopBtn: 'Ndalo SMS', sms_stoppedOk: 'U ndal.',
    sms_missing_secret: 'mungon fjalëkalimi / çelësi', sms_missing_user: 'mungon përdoruesi', sms_missing_from: 'mungon numri dërgues',
    sms_why_auth: 'përdoruesi ose fjalëkalimi është i gabuar', sms_why_offline: 'telefoni nuk është lidhur (Online në aplikacion?)', sms_why_network: 'nuk u arrit gateway', sms_why_refused: 'numri u refuzua', sms_why_busy: 'shumë kërkesa, provohet sërish', sms_why_provider: 'gabim i ofruesit',
  },
  en: {
    sms_hint: 'A customer who ticks the SMS box at checkout gets a text when the order is confirmed, ready or on its way. Only the order number and the venue name.',
    sms_setupTitle: 'Free, with your own Android phone',
    sms_setup: '1. Install SMS Gateway for Android (SMSGate) from github.com/capcom6/android-sms-gateway. 2. Allow SMS, switch Cloud server on and tap Online. 3. Turn battery optimisation off. 4. Copy the username and password the app shows here. Texts go out from the venue SIM at your plan price.',
    sms_on: 'Text customers', sms_onHint: 'Only those who ticked the SMS box.',
    sms_provider: 'How texts are sent', sms_p_smsgate: 'My Android phone (SMSGate, free)', sms_p_textbee: 'My Android phone (textbee)', sms_p_twilio: 'Twilio (paid, your own account)',
    sms_url: 'Server address (empty = SMSGate cloud)', sms_user: 'Username (from the app) / Account SID', sms_secret: 'Password / API key', sms_secretSet: 'saved; type to change it',
    sms_from: 'Sender number (Twilio only)', sms_daily: 'Daily SMS limit', sms_dailyHint: 'Keeps the SIM from being blocked by the carrier. Default 60.',
    sms_test: 'Send a test', sms_testPhone: 'Your number for the test', sms_testOk: 'Sent. Check the phone.',
    sms_status: 'Status', sms_live: 'working', sms_failing: 'failing', sms_sentToday: 'Today', sms_lastOk: 'Last sent', sms_lastErr: 'Last error', sms_waiting: 'waiting', sms_stopped: 'stopped by the customer', sms_overBudget: 'over the limit', sms_never: 'nothing yet',
    sms_stopTitle: 'A customer said STOP', sms_stopHint: 'Type their number. No more texts go to it.', sms_stopBtn: 'Stop texts', sms_stoppedOk: 'Stopped.',
    sms_missing_secret: 'the password / key is missing', sms_missing_user: 'the username is missing', sms_missing_from: 'the sender number is missing',
    sms_why_auth: 'wrong username or password', sms_why_offline: 'the phone is not connected (Online in the app?)', sms_why_network: 'the gateway did not answer', sms_why_refused: 'the number was refused', sms_why_busy: 'too many requests, retrying', sms_why_provider: 'provider error',
  },
  uk: {
    sms_hint: 'Клієнт, який позначив SMS при оформленні, отримує SMS, коли замовлення підтверджено, готове або в дорозі. Лише номер замовлення і назва закладу.',
    sms_setupTitle: 'Безкоштовно, з вашим Android-телефоном',
    sms_setup: '1. Встановіть SMS Gateway for Android (SMSGate) з github.com/capcom6/android-sms-gateway. 2. Дозвольте SMS, увімкніть Cloud server і натисніть Online. 3. Вимкніть оптимізацію батареї. 4. Скопіюйте сюди логін і пароль, які показує застосунок. SMS ідуть із SIM закладу за ціною вашого тарифу.',
    sms_on: 'Надсилати SMS клієнтам', sms_onHint: 'Лише тим, хто позначив SMS.',
    sms_provider: 'Як надсилати', sms_p_smsgate: 'Мій Android-телефон (SMSGate, безкоштовно)', sms_p_textbee: 'Мій Android-телефон (textbee)', sms_p_twilio: 'Twilio (платно, ваш акаунт)',
    sms_url: 'Адреса сервера (порожньо = хмара SMSGate)', sms_user: 'Логін (із застосунку) / Account SID', sms_secret: 'Пароль / API-ключ', sms_secretSet: 'збережено; введіть, щоб змінити',
    sms_from: 'Номер відправника (лише Twilio)', sms_daily: 'Денний ліміт SMS', sms_dailyHint: 'Захищає SIM від блокування оператором. Типово 60.',
    sms_test: 'Надіслати тест', sms_testPhone: 'Ваш номер для тесту', sms_testOk: 'Надіслано. Перевірте телефон.',
    sms_status: 'Стан', sms_live: 'працює', sms_failing: 'помилка', sms_sentToday: 'Сьогодні', sms_lastOk: 'Останнє надіслане', sms_lastErr: 'Остання помилка', sms_waiting: 'в черзі', sms_stopped: 'зупинено клієнтом', sms_overBudget: 'понад ліміт', sms_never: 'ще нічого',
    sms_stopTitle: 'Клієнт сказав STOP', sms_stopHint: 'Введіть його номер. Більше SMS на нього не підуть.', sms_stopBtn: 'Зупинити SMS', sms_stoppedOk: 'Зупинено.',
    sms_missing_secret: 'немає пароля / ключа', sms_missing_user: 'немає логіна', sms_missing_from: 'немає номера відправника',
    sms_why_auth: 'неправильний логін або пароль', sms_why_offline: 'телефон не підключено (Online у застосунку?)', sms_why_network: 'шлюз не відповів', sms_why_refused: 'номер відхилено', sms_why_busy: 'забагато запитів, повтор', sms_why_provider: 'помилка провайдера',
  },
  ru: {
    sms_hint: 'Клиент, отметивший SMS при оформлении, получает SMS, когда заказ подтверждён, готов или в пути. Только номер заказа и название заведения.',
    sms_setupTitle: 'Бесплатно, с вашим Android-телефоном',
    sms_setup: '1. Установите SMS Gateway for Android (SMSGate) с github.com/capcom6/android-sms-gateway. 2. Разрешите SMS, включите Cloud server и нажмите Online. 3. Отключите оптимизацию батареи. 4. Скопируйте сюда логин и пароль, которые показывает приложение. SMS уходят с SIM заведения по цене вашего тарифа.',
    sms_on: 'Отправлять SMS клиентам', sms_onHint: 'Только тем, кто отметил SMS.',
    sms_provider: 'Как отправлять', sms_p_smsgate: 'Мой Android-телефон (SMSGate, бесплатно)', sms_p_textbee: 'Мой Android-телефон (textbee)', sms_p_twilio: 'Twilio (платно, ваш аккаунт)',
    sms_url: 'Адрес сервера (пусто = облако SMSGate)', sms_user: 'Логин (из приложения) / Account SID', sms_secret: 'Пароль / API-ключ', sms_secretSet: 'сохранён; введите, чтобы изменить',
    sms_from: 'Номер отправителя (только Twilio)', sms_daily: 'Дневной лимит SMS', sms_dailyHint: 'Защищает SIM от блокировки оператором. По умолчанию 60.',
    sms_test: 'Отправить тест', sms_testPhone: 'Ваш номер для теста', sms_testOk: 'Отправлено. Проверьте телефон.',
    sms_status: 'Состояние', sms_live: 'работает', sms_failing: 'ошибка', sms_sentToday: 'Сегодня', sms_lastOk: 'Последнее отправленное', sms_lastErr: 'Последняя ошибка', sms_waiting: 'в очереди', sms_stopped: 'остановлено клиентом', sms_overBudget: 'сверх лимита', sms_never: 'ещё ничего',
    sms_stopTitle: 'Клиент сказал STOP', sms_stopHint: 'Введите его номер. Больше SMS на него не уйдут.', sms_stopBtn: 'Остановить SMS', sms_stoppedOk: 'Остановлено.',
    sms_missing_secret: 'нет пароля / ключа', sms_missing_user: 'нет логина', sms_missing_from: 'нет номера отправителя',
    sms_why_auth: 'неверный логин или пароль', sms_why_offline: 'телефон не подключён (Online в приложении?)', sms_why_network: 'шлюз не ответил', sms_why_refused: 'номер отклонён', sms_why_busy: 'слишком много запросов, повтор', sms_why_provider: 'ошибка провайдера',
  },
};
