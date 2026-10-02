// The courier's words, in every language of lib/langs.js.
//
// Same mechanism as the console: static copy carries `data-t`, templates call
// `t()`, and a language change rewrites text in place. Albanian first: the
// courier is in Albania. Voice recognition follows the same choice, because a
// recogniser told the wrong language hears nothing.

import { safeGet, safeSet } from '/store/storage.js';

import { LANGS, pickLang, tagFor } from '../lib/langs.js';

export { LANGS };
const STORAGE_KEY = 'dw_c_lang';

export const T = {
  sq: {
    ex_login: 'p.sh. +355 69 123 4567 ose emaili', ex_phone: 'p.sh. +355 69 123 4567',
    appTitle: 'dowiz · korrieri', offline: 'offline', onShift: 'në turn', gps: 'GPS', gpsDenied: 'GPS i ndaluar', gpsUnavailable: 'GPS s’është i disponueshëm', gpsUnit: 'm',
    backgroundGps: 'Aplikacioni ishte në sfond — GPS mund të jetë ndërprerë', sessionOver: 'Sesioni mbaroi',
    theme: 'Tema', themeSystem: 'Tema: si në telefon', themeDark: 'Tema: e errët', themeLight: 'Tema: e ndritshme', language: 'Gjuha', sayCommand: 'Thuaj një komandë', stopListening: 'Ndal regjistrimin', voice: 'Zëri', yes: 'Po', no: 'Jo',
    loginTitle: 'Hyrja e korrierit', loginLine: 'Një ekran, një punë. Rruga ju çon deri te «U dorëzua».', emailOrPhone: 'Email ose telefon', password: 'Fjalëkalimi', signIn: 'Hyni', signingIn: 'Duke hyrë…', haveCode: 'Kam një kod ftese',
    claimTitle: 'Kodi i ftesës', claimHint: 'Kodin jua dha lokali. Vlen një javë dhe punon një herë.', yourPhone: 'Telefoni juaj', code: 'Kodi', choosePassword: 'Zgjidhni një fjalëkalim', passwordHint: 'Të paktën 8 shenja. Lokali nuk e sheh.', start: 'Fillo', checking: 'Duke kontrolluar…', havePassword: 'Kam tashmë fjalëkalim',
    loading: 'Po ngarkohet', noLink: 'S’ka lidhje me lokalin', retry: 'Provo përsëri', youAreOffline: 'Jeni offline', staleAsOf: 'pa lidhje · të dhënat e {t}', offlineHint: 'Porositë nuk vijnë derisa të hapni turnin', openShift: 'Fillo turnin', endShift: 'Mbyll turnin',
    noneFree: 'S’ka porosi të lira', noneFreeHint: 'Sapo diçka të jetë gati, shfaqet këtu', askPlaceholder: 'Pyet për dërgesat e mia…', send: 'Dërgo', myShifts: 'Turnet e mia', history: 'Historia', thinking: 'Po mendon…',
    readyForPickup: 'Gati për marrje', pcs: 'copë', pickOne: 'zgjidhni dhe merrni', take: 'Merr', delivering: 'Po dorëzoni', pickUpOrder: 'Merrni porosinë', onTheWay: 'Në rrugë', ready: 'Gati', items: 'artikuj', paidOnline: 'Paguar online',
    delivered: 'U dorëzua', deliveredAria: 'U dorëzua — rrëshqitni ose shtypni', swipe: 'rrëshqitni →', pickedUp: 'E mora', inMaps: 'Në hartë', call: 'Telefono', saving: 'Po ruhet…', confirmDelivered: 'Ta shënoj si të dorëzuar?', refusedAtDoor: 'Refuzuar te dera', confirmRefused: 'Klienti e refuzoi porosinë? Nuk u mor asnjë para.', refusedDone: 'U shënua: refuzuar te dera', refusedNoteLabel: 'Shënim (jo i detyrueshëm)', refusedNoteHint: 'p.sh. askush nuk hapi derën',
    howMuchCash: 'Sa para në dorë morët?', confirm: 'Konfirmo', back: 'Prapa', badAmount: 'Shumë e pasaktë', shortfall: 'Mungesë', recorded: 'u regjistrua',
    queued: 'Do ta dërgojmë', queuedN: '{n} pa dërguar', queuedHint: 'S’ka lidhje — e ruajtëm dhe e dërgojmë vetë sapo të kthehet rrjeti', queuedSaved: 'E ruajtëm — do të dërgohet vetvetiu',
    queueFull: 'Shumë veprime pa dërguar — prisni të kthehet rrjeti', queueNoStore: 'Telefoni s’e ruan dot — provoni sërish kur të ketë rrjet', queuedChanged: 'Porosia ndryshoi sa ishit pa lidhje', queuedRefused: 'Veprimi nuk u pranua',
    offered: 'Ju propozohet', offerLapsed: 'Koha kaloi — porosia është sërish e lirë, por ende mund ta merrni', timeLeft: 'Mbetën',
    cashInHand: 'Para në dorë', stillOnRoad: 'Ende në rrugë', today: 'Sot', days7: '7 ditë', days30: '30 ditë', earningsHint: 'Dorëzime, para të mbledhura dhe bakshishe. Paratë ia jepni lokalit, bakshishet janë tuajat. dowiz nuk llogarit pagën.', tips: 'bakshishe',
    emptyHistory: 'Ende bosh', emptyHistoryHint: 'Dorëzimet e mbyllura shfaqen këtu',
    voiceUnsupported: 'Shfletuesi nuk njeh zërin', micDenied: 'S’ka leje për mikrofonin', voiceOffline: 'Njohja e zërit s’punon offline', asking: 'Po pyes…', openWaiting: 'Të hapura {open}, në pritje {waiting}',
    etaToDoor: 'deri te dera', km: 'km', min: 'min', forTime: 'Për orën',
    chatOpen: 'Bisedë', chatTitle: 'Bisedë me klientin', chatPlaceholder: 'Shkruani klientit…', chatSend: 'Dërgo', chatClosed: 'Biseda u mbyll', chatEmpty: 'Ende asnjë mesazh', chatYou: 'Ju', chatCustomer: 'Klienti', chatFail: 'Biseda nuk u hap',
    help: 'Ndihma', gStep: 'Hapi {i} nga {n}', gSkip: 'Kapërce', gLater: 'Mbaro më vonë', gBack: 'Prapa', gNext: 'Tjetra', gDone: 'Gati', gPaused: 'Turi u ruajt — «Ndihma» vazhdon nga këtu', gSkipped: 'Turi u kapërcye — hapet gjithnjë nga «Ndihma»', gFinished: 'Gati. «?» të vegjël pranë elementeve shpjegojnë secilin', gUnfinished: 'Turi s’u mbyll — «Ndihma» vazhdon nga i njëjti vend', gWhat: 'Çfarë është', gClose: 'Mbyll',
    hWelcomeT: 'Ky është aplikacioni juaj i korrierit', hWelcome: 'Një ekran — një punë. Tre hapa të shkurtër tregojnë çfarë është ku. Mund ta kapërceni ose ta mbaroni më vonë.',
    hShiftT: 'Turni', hShift: 'Derisa turni s’është hapur, porositë nuk vijnë. Në turn këtu shihni dorëzimet dhe paratë e mbledhura.',
    hSheetT: 'Një punë në ekran', hSheet: 'Porositë e gatshme shfaqen këtu. Zgjidhni një, shtypni «Merr» — dhe ekrani ju çon hap pas hapi deri te «U dorëzua».',
    hMicT: 'Me zë', hMic: 'Thoni «e mora», «u dorëzua» ose «ku është tjetra». Aplikacioni përsërit çfarë dëgjoi dhe kërkon konfirmim.',
    hAskT: 'Pyetje për dërgesat', hAsk: 'Përgjigjet vetëm për turnet dhe porositë tuaja: sa fituat, ku të shkoni, çfarë ndodhi dje.',
    hHelpT: 'Ndihma', hHelp: 'Ky buton përsërit turin. Ekziston vetëm kur qëndroni, jo në rrugë.',
    learn: 'Mësimet', learnNew: 'I ri', learnDone: 'Mbaruar', learnPaused: 'Në pauzë', learnWatch: 'Shiko videon', learnWrites: 'ndryshon të dhëna reale', learnSteps: '{n} hapa', learnEmpty: 'Ky mësim nuk u gjet', learnOffline: 'Mësimet duan lidhje interneti', agent: 'Agjenti AI (MCP)',
  },
  en: {
    ex_login: 'e.g. +355 69 123 4567 or your e-mail', ex_phone: 'e.g. +355 69 123 4567',
    appTitle: 'dowiz · courier', offline: 'offline', onShift: 'on shift', gps: 'GPS', gpsDenied: 'GPS denied', gpsUnavailable: 'GPS unavailable', gpsUnit: 'm',
    backgroundGps: 'The app was in the background — GPS may have paused', sessionOver: 'Session ended',
    theme: 'Theme', themeSystem: 'Theme: as the phone', themeDark: 'Theme: dark', themeLight: 'Theme: light', language: 'Language', sayCommand: 'Say a command', stopListening: 'Stop listening', voice: 'Voice', yes: 'Yes', no: 'No',
    loginTitle: 'Courier sign-in', loginLine: 'One screen, one job. The road leads to “Delivered”.', emailOrPhone: 'Email or phone', password: 'Password', signIn: 'Sign in', signingIn: 'Signing in…', haveCode: 'I have an invite code',
    claimTitle: 'Invite code', claimHint: 'The venue gave you the code. It lasts a week and works once.', yourPhone: 'Your phone', code: 'Code', choosePassword: 'Choose a password', passwordHint: 'At least 8 characters. The venue never sees it.', start: 'Start', checking: 'Checking…', havePassword: 'I already have a password',
    loading: 'Loading', noLink: 'No connection to the venue', retry: 'Try again', youAreOffline: 'You are offline', staleAsOf: 'no connection · as of {t}', offlineHint: 'Orders will not come in until you open a shift', openShift: 'Start shift', endShift: 'End shift',
    noneFree: 'No free orders', noneFreeHint: 'As soon as something is ready, it appears here', askPlaceholder: 'Ask about my deliveries…', send: 'Send', myShifts: 'My shifts', history: 'History', thinking: 'Thinking…',
    readyForPickup: 'Ready for pickup', pcs: 'pcs', pickOne: 'pick one and take it', take: 'Take', delivering: 'Delivering', pickUpOrder: 'Pick up the order', onTheWay: 'On the way', ready: 'Ready', items: 'items', paidOnline: 'Paid online',
    delivered: 'Delivered', deliveredAria: 'Delivered — swipe or press', swipe: 'swipe →', pickedUp: 'Picked up', inMaps: 'In maps', call: 'Call', saving: 'Saving…', confirmDelivered: 'Mark as delivered?', refusedAtDoor: 'Refused at the door', confirmRefused: 'The customer refused the order? No money was taken.', refusedDone: 'Recorded: refused at the door', refusedNoteLabel: 'Note (optional)', refusedNoteHint: 'e.g. nobody opened the door',
    howMuchCash: 'How much cash did you receive?', confirm: 'Confirm', back: 'Back', badAmount: 'Invalid amount', shortfall: 'Short by', recorded: 'recorded',
    queued: 'Will be sent', queuedN: '{n} unsent', queuedHint: 'No connection — saved on this phone and sent by itself as soon as there is one', queuedSaved: 'Saved — it will send itself',
    queueFull: 'Too many unsent actions — wait for a connection', queueNoStore: 'This phone will not store it — try again when there is a connection', queuedChanged: 'This order changed while you were away', queuedRefused: 'That action was not accepted',
    offered: 'Offered to you', offerLapsed: 'Time is up — the order is free again, but you can still take it', timeLeft: 'Left',
    cashInHand: 'Cash in hand', stillOnRoad: 'Still on the road', today: 'Today', days7: '7 days', days30: '30 days', earningsHint: 'Deliveries, cash collected and tips. Cash goes to the venue, tips are yours. dowiz does not compute pay.', tips: 'tips',
    emptyHistory: 'Nothing yet', emptyHistoryHint: 'Finished deliveries appear here',
    voiceUnsupported: 'This browser cannot recognise speech', micDenied: 'No microphone permission', voiceOffline: 'Speech recognition needs a connection', asking: 'Asking…', openWaiting: 'Open {open}, waiting {waiting}',
    etaToDoor: 'to the door', km: 'km', min: 'min', forTime: 'For',
    chatOpen: 'Chat', chatTitle: 'Chat with the customer', chatPlaceholder: 'Write to the customer…', chatSend: 'Send', chatClosed: 'The chat is closed', chatEmpty: 'No messages yet', chatYou: 'You', chatCustomer: 'Customer', chatFail: 'The chat did not open',
    help: 'Help', gStep: 'Step {i} of {n}', gSkip: 'Skip', gLater: 'Finish later', gBack: 'Back', gNext: 'Next', gDone: 'Done', gPaused: 'Tour saved — “Help” continues from here', gSkipped: 'Tour skipped — “Help” opens it any time', gFinished: 'Done. The small “?” beside controls explain each one', gUnfinished: 'Tour unfinished — “Help” continues from the same spot', gWhat: 'What is', gClose: 'Close',
    hWelcomeT: 'This is your courier app', hWelcome: 'One screen — one job. Three short steps show what is where. You can skip or finish later.',
    hShiftT: 'Shift', hShift: 'Until a shift is open, no orders come in. On shift, this shows deliveries and cash collected.',
    hSheetT: 'One job on screen', hSheet: 'Ready orders appear here. Pick one, press “Take” — and the screen leads step by step to “Delivered”.',
    hMicT: 'By voice', hMic: 'Say “picked up”, “delivered” or “where next”. The app repeats what it heard and asks you to confirm.',
    hAskT: 'Questions about deliveries', hAsk: 'Answers only about your shifts and orders: how much you made, where to go next, what happened yesterday.',
    hHelpT: 'Help', hHelp: 'This button repeats the tour. It is here only while you stand still, not on a run.',
    learn: 'Lessons', learnNew: 'New', learnDone: 'Done', learnPaused: 'Paused', learnWatch: 'Watch the video', learnWrites: 'changes real data', learnSteps: '{n} steps', learnEmpty: 'That lesson was not found', learnOffline: 'Lessons need a connection', agent: 'AI agent (MCP)',
  },
  uk: {
    ex_login: 'напр. +355 69 123 4567 або e-mail', ex_phone: 'напр. +355 69 123 4567',
    appTitle: 'dowiz · кур’єр', offline: 'офлайн', onShift: 'на зміні', gps: 'GPS', gpsDenied: 'GPS заборонено', gpsUnavailable: 'GPS недоступний', gpsUnit: 'м',
    backgroundGps: 'Застосунок був у фоні — GPS міг перерватися', sessionOver: 'Сесію завершено',
    theme: 'Тема', themeSystem: 'Тема: як на телефоні', themeDark: 'Тема: темна', themeLight: 'Тема: світла', language: 'Мова', sayCommand: 'Сказати команду', stopListening: 'Зупинити запис', voice: 'Голос', yes: 'Так', no: 'Ні',
    loginTitle: 'Вхід для кур’єра', loginLine: 'Один екран, одна справа. Дорога веде до «Доставлено».', emailOrPhone: 'Email або телефон', password: 'Пароль', signIn: 'Увійти', signingIn: 'Входимо…', haveCode: 'У мене код запрошення',
    claimTitle: 'Код запрошення', claimHint: 'Код дав вам заклад. Він діє тиждень і спрацьовує один раз.', yourPhone: 'Ваш телефон', code: 'Код', choosePassword: 'Придумайте пароль', passwordHint: 'Щонайменше 8 символів. Заклад його не побачить.', start: 'Почати', checking: 'Перевіряємо…', havePassword: 'У мене вже є пароль',
    loading: 'Завантажуємо', noLink: 'Немає зв’язку із закладом', retry: 'Спробувати ще раз', youAreOffline: 'Ви офлайн', staleAsOf: 'немає зв’язку · дані на {t}', offlineHint: 'Замовлення не надходитимуть, поки зміну не відкрито', openShift: 'Почати зміну', endShift: 'Завершити зміну',
    noneFree: 'Вільних замовлень немає', noneFreeHint: 'Щойно щось буде готове — з’явиться тут', askPlaceholder: 'Спитати про мої доставки…', send: 'Надіслати', myShifts: 'Мої зміни', history: 'Історія', thinking: 'Думає…',
    readyForPickup: 'Готові до забору', pcs: 'шт.', pickOne: 'оберіть і візьміть', take: 'Взяти', delivering: 'Доставляєте', pickUpOrder: 'Заберіть замовлення', onTheWay: 'В дорозі', ready: 'Готове', items: 'поз.', paidOnline: 'Оплачено онлайн',
    delivered: 'Доставлено', deliveredAria: 'Доставлено — проведіть або натисніть', swipe: 'проведіть →', pickedUp: 'Забрав', inMaps: 'У картах', call: 'Подзвонити', saving: 'Записуємо…', confirmDelivered: 'Позначити як доставлене?', refusedAtDoor: 'Відмова біля дверей', confirmRefused: 'Клієнт відмовився від замовлення? Гроші не отримано.', refusedDone: 'Записано: відмова біля дверей', refusedNoteLabel: 'Примітка (необов\'язково)', refusedNoteHint: 'напр. ніхто не відчинив',
    howMuchCash: 'Скільки готівки отримано?', confirm: 'Підтвердити', back: 'Назад', badAmount: 'Некоректна сума', shortfall: 'Недостача', recorded: 'записано',
    queued: 'Надішлемо', queuedN: '{n} не надіслано', queuedHint: 'Немає зв’язку — зберегли на телефоні й надішлемо самі, щойно він з’явиться', queuedSaved: 'Зберегли — надішлеться саме',
    queueFull: 'Забагато ненадісланих дій — дочекайтеся зв’язку', queueNoStore: 'Телефон не зберігає — спробуйте, коли буде зв’язок', queuedChanged: 'Замовлення змінилося, поки ви були без зв’язку', queuedRefused: 'Дію не прийнято',
    offered: 'Вам пропонують', offerLapsed: 'Час вийшов — замовлення знову вільне, але ви ще можете його взяти', timeLeft: 'Залишилось',
    cashInHand: 'Готівка на руках', stillOnRoad: 'Ще в дорозі', today: 'Сьогодні', days7: '7 днів', days30: '30 днів', earningsHint: 'Доставки, зібрана готівка й чайові. Готівку віддаєте закладу, чайові — ваші. Розрахунок оплати dowiz не веде.', tips: 'чайові',
    emptyHistory: 'Поки порожньо', emptyHistoryHint: 'Завершені доставки з’являться тут',
    voiceUnsupported: 'Браузер не розпізнає голос', micDenied: 'Немає дозволу на мікрофон', voiceOffline: 'Розпізнавання недоступне офлайн', asking: 'Питаю…', openWaiting: 'Відкритих {open}, чекає {waiting}',
    etaToDoor: 'до дверей', km: 'км', min: 'хв', forTime: 'На',
    chatOpen: 'Чат', chatTitle: 'Чат із клієнтом', chatPlaceholder: 'Написати клієнту…', chatSend: 'Надіслати', chatClosed: 'Чат закрито', chatEmpty: 'Повідомлень ще немає', chatYou: 'Ви', chatCustomer: 'Клієнт', chatFail: 'Чат не відкрився',
    help: 'Довідка', gStep: 'Крок {i} з {n}', gSkip: 'Пропустити', gLater: 'Завершити пізніше', gBack: 'Назад', gNext: 'Далі', gDone: 'Готово', gPaused: 'Тур збережено — «Довідка» продовжить з цього місця', gSkipped: 'Тур пропущено — його завжди можна відкрити через «Довідка»', gFinished: 'Готово. Маленькі «?» біля елементів пояснюють кожен окремо', gUnfinished: 'Тур не завершено — «Довідка» продовжить з того ж місця', gWhat: 'Що це', gClose: 'Закрити',
    hWelcomeT: 'Це ваш застосунок кур’єра', hWelcome: 'Один екран — одна справа. Три короткі кроки покажуть, що тут до чого. Можна пропустити або завершити пізніше.',
    hShiftT: 'Зміна', hShift: 'Поки зміну не відкрито, замовлення не надходять. На зміні тут видно кількість доставок і зібрану готівку.',
    hSheetT: 'Одна справа на екрані', hSheet: 'Готові замовлення з’являються тут. Оберіть одне, натисніть «Взяти» — і далі екран веде крок за кроком до «Доставлено».',
    hMicT: 'Голосом', hMic: 'Скажіть «взяв», «доставив» або «де наступне». Застосунок повторить, що почув, і попросить підтвердити.',
    hAskT: 'Питання про доставки', hAsk: 'Відповідає лише про ваші зміни й замовлення: скільки заробили, куди їхати далі, що було вчора.',
    hHelpT: 'Довідка', hHelp: 'Ця кнопка повторить тур. Вона є лише тоді, коли ви стоїте, не на маршруті.',
    learn: 'Уроки', learnNew: 'Новий', learnDone: 'Пройдено', learnPaused: 'На паузі', learnWatch: 'Дивитися відео', learnWrites: 'змінює реальні дані', learnSteps: 'кроків: {n}', learnEmpty: 'Такого уроку не знайдено', learnOffline: 'Для уроків потрібен інтернет', agent: 'AI-агент (MCP)',
  },
  ru: {
    ex_login: 'напр. +355 69 123 4567 или e-mail', ex_phone: 'напр. +355 69 123 4567',
    appTitle: 'dowiz · курьер', offline: 'офлайн', onShift: 'на смене', gps: 'GPS', gpsDenied: 'GPS запрещён', gpsUnavailable: 'GPS недоступен', gpsUnit: 'м',
    backgroundGps: 'Приложение было в фоне — GPS мог прерваться', sessionOver: 'Сессия завершена',
    theme: 'Тема', themeSystem: 'Тема: как в телефоне', themeDark: 'Тема: тёмная', themeLight: 'Тема: светлая', language: 'Язык', sayCommand: 'Сказать команду', stopListening: 'Остановить запись', voice: 'Голос', yes: 'Да', no: 'Нет',
    loginTitle: 'Вход для курьера', loginLine: 'Один экран, одно дело. Дорога ведёт к «Доставлено».', emailOrPhone: 'Email или телефон', password: 'Пароль', signIn: 'Войти', signingIn: 'Входим…', haveCode: 'У меня код приглашения',
    claimTitle: 'Код приглашения', claimHint: 'Код дало вам заведение. Он действует неделю и срабатывает один раз.', yourPhone: 'Ваш телефон', code: 'Код', choosePassword: 'Придумайте пароль', passwordHint: 'Не меньше 8 символов. Заведение его не увидит.', start: 'Начать', checking: 'Проверяем…', havePassword: 'У меня уже есть пароль',
    loading: 'Загружаем', noLink: 'Нет связи с заведением', retry: 'Попробовать ещё раз', youAreOffline: 'Вы офлайн', staleAsOf: 'нет связи · данные на {t}', offlineHint: 'Заказы не будут поступать, пока смена не открыта', openShift: 'Начать смену', endShift: 'Завершить смену',
    noneFree: 'Свободных заказов нет', noneFreeHint: 'Как только что-то будет готово — появится здесь', askPlaceholder: 'Спросить о моих доставках…', send: 'Отправить', myShifts: 'Мои смены', history: 'История', thinking: 'Думает…',
    readyForPickup: 'Готовы к выдаче', pcs: 'шт.', pickOne: 'выберите и возьмите', take: 'Взять', delivering: 'Доставляете', pickUpOrder: 'Заберите заказ', onTheWay: 'В пути', ready: 'Готов', items: 'поз.', paidOnline: 'Оплачено онлайн',
    delivered: 'Доставлено', deliveredAria: 'Доставлено — проведите или нажмите', swipe: 'проведите →', pickedUp: 'Забрал', inMaps: 'В картах', call: 'Позвонить', saving: 'Записываем…', confirmDelivered: 'Отметить как доставленный?', refusedAtDoor: 'Отказ у двери', confirmRefused: 'Клиент отказался от заказа? Деньги не получены.', refusedDone: 'Записано: отказ у двери', refusedNoteLabel: 'Примечание (необязательно)', refusedNoteHint: 'напр. никто не открыл',
    howMuchCash: 'Сколько наличных получено?', confirm: 'Подтвердить', back: 'Назад', badAmount: 'Некорректная сумма', shortfall: 'Недостача', recorded: 'записано',
    queued: 'Отправим', queuedN: '{n} не отправлено', queuedHint: 'Нет связи — сохранили на телефоне и отправим сами, как только она появится', queuedSaved: 'Сохранили — отправится само',
    queueFull: 'Слишком много неотправленных действий — дождитесь связи', queueNoStore: 'Телефон не сохраняет — попробуйте, когда будет связь', queuedChanged: 'Заказ изменился, пока вы были без связи', queuedRefused: 'Действие не принято',
    offered: 'Вам предлагают', offerLapsed: 'Время вышло — заказ снова свободен, но вы ещё можете его взять', timeLeft: 'Осталось',
    cashInHand: 'Наличные на руках', stillOnRoad: 'Ещё в пути', today: 'Сегодня', days7: '7 дней', days30: '30 дней', earningsHint: 'Доставки, собранные наличные и чаевые. Наличные отдаёте заведению, чаевые — ваши. Расчёт оплаты dowiz не ведёт.', tips: 'чаевые',
    emptyHistory: 'Пока пусто', emptyHistoryHint: 'Завершённые доставки появятся здесь',
    voiceUnsupported: 'Браузер не распознаёт голос', micDenied: 'Нет разрешения на микрофон', voiceOffline: 'Распознавание недоступно офлайн', asking: 'Спрашиваю…', openWaiting: 'Открытых {open}, ждёт {waiting}',
    etaToDoor: 'до двери', km: 'км', min: 'мин', forTime: 'На',
    chatOpen: 'Чат', chatTitle: 'Чат с клиентом', chatPlaceholder: 'Написать клиенту…', chatSend: 'Отправить', chatClosed: 'Чат закрыт', chatEmpty: 'Сообщений пока нет', chatYou: 'Вы', chatCustomer: 'Клиент', chatFail: 'Чат не открылся',
    help: 'Справка', gStep: 'Шаг {i} из {n}', gSkip: 'Пропустить', gLater: 'Завершить позже', gBack: 'Назад', gNext: 'Далее', gDone: 'Готово', gPaused: 'Тур сохранён — «Справка» продолжит с этого места', gSkipped: 'Тур пропущен — его всегда можно открыть через «Справка»', gFinished: 'Готово. Маленькие «?» рядом с элементами объясняют каждый отдельно', gUnfinished: 'Тур не завершён — «Справка» продолжит с того же места', gWhat: 'Что это', gClose: 'Закрыть',
    hWelcomeT: 'Это ваше приложение курьера', hWelcome: 'Один экран — одно дело. Три коротких шага покажут, что здесь к чему. Можно пропустить или завершить позже.',
    hShiftT: 'Смена', hShift: 'Пока смена не открыта, заказы не поступают. На смене здесь видно количество доставок и собранные наличные.',
    hSheetT: 'Одно дело на экране', hSheet: 'Готовые заказы появляются здесь. Выберите один, нажмите «Взять» — и дальше экран ведёт шаг за шагом до «Доставлено».',
    hMicT: 'Голосом', hMic: 'Скажите «взял», «доставил» или «где следующий». Приложение повторит, что услышало, и попросит подтвердить.',
    hAskT: 'Вопросы о доставках', hAsk: 'Отвечает только о ваших сменах и заказах: сколько заработали, куда ехать дальше, что было вчера.',
    hHelpT: 'Справка', hHelp: 'Эта кнопка повторит тур. Она есть только тогда, когда вы стоите, не на маршруте.',
    learn: 'Уроки', learnNew: 'Новый', learnDone: 'Пройден', learnPaused: 'На паузе', learnWatch: 'Смотреть видео', learnWrites: 'меняет реальные данные', learnSteps: 'шагов: {n}', learnEmpty: 'Такой урок не найден', learnOffline: 'Для уроков нужен интернет', agent: 'AI-агент (MCP)',
  },
};

/// A choice made here wins; else the phone's own languages on first open; else Albanian.
export let lang = pickLang([safeGet(STORAGE_KEY)], globalThis.navigator?.languages || [], 'sq');
export const t = (k, vars) => {
  let v = (T[lang] && T[lang][k]) ?? T.en[k] ?? k;
  if (vars) for (const [name, val] of Object.entries(vars)) v = v.replace(`{${name}}`, String(val));
  return v;
};
export const intlLocale = () => lang;
/// The BCP-47 tag the browser's recogniser wants (lib/langs.js INTL).
export const voiceLocale = () => tagFor(lang);
/// The next language in the ring, for a one-button switch.
export const nextLang = () => LANGS[(LANGS.indexOf(lang) + 1) % LANGS.length];

export function setLang(code){
  if (!LANGS.includes(code) || code === lang) return false;
  lang = code;
  safeSet(STORAGE_KEY, lang);
  document.documentElement.lang = lang;
  document.title = t('appTitle');
  retranslate(document);
  return true;
}

export function retranslate(root = document){
  for (const el of root.querySelectorAll('[data-t]')) {
    const v = t(el.dataset.t);
    if (el.textContent !== v) el.textContent = v;
  }
  for (const el of root.querySelectorAll('[data-t-attr]')) {
    for (const pair of el.dataset.tAttr.split(/\s+/)) {
      const [attr, key] = pair.split(':');
      if (attr && key) el.setAttribute(attr, t(key));
    }
  }
}
