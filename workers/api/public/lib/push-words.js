// What the "notify me" control says, in the four languages (W-PUSH).
// One table for the three surfaces (storefront, courier, console), so the
// control reads the same everywhere and a missing language is one place to fix.

export const PUSH_WORDS = {
  sq: {
    pushTitle: 'Njoftime në telefon',
    pushWhyCustomer: 'Ju njoftojmë kur porosia juaj ndryshon gjendje.',
    pushWhyCourier: 'Ju njoftojmë kur ju caktohet një porosi.',
    pushWhyStaff: 'Ju njoftojmë kur vjen një porosi e re.',
    pushOn: 'Aktivizo njoftimet',
    pushOff: 'Çaktivizo njoftimet',
    pushIsOn: 'Njoftimet janë aktive në këtë pajisje.',
    pushBlocked: 'Njoftimet janë bllokuar në shfletues. Lejojini te cilësimet e faqes.',
    pushUnsupported: 'Ky shfletues nuk mbështet njoftimet.',
    pushIos: 'Në iPhone/iPad (iOS 16.4+): shtojeni faqen në ekranin kryesor (Shpërndaj → Shto në ekranin bazë), hapeni prej andej dhe aktivizoni njoftimet.',
    pushFail: 'Njoftimet nuk u aktivizuan',
  },
  en: {
    pushTitle: 'Phone notifications',
    pushWhyCustomer: 'We tell you when your order changes.',
    pushWhyCourier: 'We tell you when an order is assigned to you.',
    pushWhyStaff: 'We tell you when a new order arrives.',
    pushOn: 'Turn on notifications',
    pushOff: 'Turn off notifications',
    pushIsOn: 'Notifications are on for this device.',
    pushBlocked: 'Notifications are blocked in this browser. Allow them in the site settings.',
    pushUnsupported: 'This browser cannot show notifications.',
    pushIos: 'On iPhone/iPad (iOS 16.4+): add this page to the Home Screen (Share → Add to Home Screen), open it from there and turn notifications on.',
    pushFail: 'Notifications were not turned on',
  },
  uk: {
    pushTitle: 'Сповіщення на телефон',
    pushWhyCustomer: 'Повідомимо, коли статус замовлення зміниться.',
    pushWhyCourier: 'Повідомимо, коли вам призначать замовлення.',
    pushWhyStaff: 'Повідомимо, коли надійде нове замовлення.',
    pushOn: 'Увімкнути сповіщення',
    pushOff: 'Вимкнути сповіщення',
    pushIsOn: 'Сповіщення увімкнено на цьому пристрої.',
    pushBlocked: 'Сповіщення заблоковано в браузері. Дозвольте їх у налаштуваннях сайту.',
    pushUnsupported: 'Цей браузер не показує сповіщення.',
    pushIos: 'На iPhone/iPad (iOS 16.4+): додайте сторінку на початковий екран (Поділитися → На початковий екран), відкрийте її звідти й увімкніть сповіщення.',
    pushFail: 'Сповіщення не ввімкнено',
  },
  ru: {
    pushTitle: 'Уведомления на телефон',
    pushWhyCustomer: 'Сообщим, когда статус заказа изменится.',
    pushWhyCourier: 'Сообщим, когда вам назначат заказ.',
    pushWhyStaff: 'Сообщим, когда придёт новый заказ.',
    pushOn: 'Включить уведомления',
    pushOff: 'Выключить уведомления',
    pushIsOn: 'Уведомления включены на этом устройстве.',
    pushBlocked: 'Уведомления заблокированы в браузере. Разрешите их в настройках сайта.',
    pushUnsupported: 'Этот браузер не показывает уведомления.',
    pushIos: 'На iPhone/iPad (iOS 16.4+): добавьте страницу на экран «Домой» (Поделиться → На экран «Домой»), откройте её оттуда и включите уведомления.',
    pushFail: 'Уведомления не включены',
  },
};

/// The words in `lang`, English for a language this table does not have.
export const pushWords = lang => PUSH_WORDS[lang] || PUSH_WORDS.en;
