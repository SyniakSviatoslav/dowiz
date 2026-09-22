// The courier's words, in the three languages a Durrës courier might read.
//
// Same mechanism as the console: static copy carries `data-t`, templates call
// `t()`, and a language change rewrites text in place. Albanian first: the
// courier is in Albania. Voice recognition follows the same choice, because a
// recogniser told the wrong language hears nothing.

import { safeGet, safeSet } from '/store/storage.js';

export const LANGS = ['sq', 'en', 'uk'];
const STORAGE_KEY = 'dw_c_lang';
/// BCP-47 tags the browser's recogniser wants, per language.
const VOICE = { sq: 'sq-AL', en: 'en-US', uk: 'uk-UA' };

export const T = {
  sq: {
    appTitle: 'dowiz · korrieri', offline: 'offline', onShift: 'në turn', gps: 'GPS', gpsDenied: 'GPS i ndaluar', gpsUnavailable: 'GPS s’është i disponueshëm', gpsUnit: 'm',
    backgroundGps: 'Aplikacioni ishte në sfond — GPS mund të jetë ndërprerë', sessionOver: 'Sesioni mbaroi',
    theme: 'Tema', themeSystem: 'Tema: si në telefon', themeDark: 'Tema: e errët', themeLight: 'Tema: e ndritshme', language: 'Gjuha', sayCommand: 'Thuaj një komandë', stopListening: 'Ndal regjistrimin', voice: 'Zëri', yes: 'Po', no: 'Jo',
    loginTitle: 'Hyrja e korrierit', loginLine: 'Një ekran, një punë. Rruga ju çon deri te «U dorëzua».', emailOrPhone: 'Email ose telefon', password: 'Fjalëkalimi', signIn: 'Hyni', signingIn: 'Duke hyrë…', haveCode: 'Kam një kod ftese',
    claimTitle: 'Kodi i ftesës', claimHint: 'Kodin jua dha lokali. Vlen një javë dhe punon një herë.', yourPhone: 'Telefoni juaj', code: 'Kodi', choosePassword: 'Zgjidhni një fjalëkalim', passwordHint: 'Të paktën 8 shenja. Lokali nuk e sheh.', start: 'Fillo', checking: 'Duke kontrolluar…', havePassword: 'Kam tashmë fjalëkalim',
    loading: 'Po ngarkohet', noLink: 'S’ka lidhje me lokalin', retry: 'Provo përsëri', youAreOffline: 'Jeni offline', offlineHint: 'Porositë nuk vijnë derisa të hapni turnin', openShift: 'Fillo turnin', endShift: 'Mbyll turnin',
    noneFree: 'S’ka porosi të lira', noneFreeHint: 'Sapo diçka të jetë gati, shfaqet këtu', askPlaceholder: 'Pyet për dërgesat e mia…', myShifts: 'Turnet e mia', history: 'Historia', thinking: 'Po mendon…',
    readyForPickup: 'Gati për marrje', pcs: 'copë', pickOne: 'zgjidhni dhe merrni', take: 'Merr', delivering: 'Po dorëzoni', pickUpOrder: 'Merrni porosinë', onTheWay: 'Në rrugë', ready: 'Gati', items: 'artikuj', paidOnline: 'Paguar online',
    delivered: 'U dorëzua', deliveredAria: 'U dorëzua — rrëshqitni ose shtypni', swipe: 'rrëshqitni →', pickedUp: 'E mora', inMaps: 'Në hartë', call: 'Telefono', saving: 'Po ruhet…', confirmDelivered: 'Ta shënoj si të dorëzuar?',
    howMuchCash: 'Sa para në dorë morët?', confirm: 'Konfirmo', back: 'Prapa', badAmount: 'Shumë e pasaktë', shortfall: 'Mungesë', recorded: 'u regjistrua',
    queued: 'Do ta dërgojmë', queuedN: '{n} pa dërguar', queuedHint: 'S’ka lidhje — e ruajtëm dhe e dërgojmë vetë sapo të kthehet rrjeti', queuedSaved: 'E ruajtëm — do të dërgohet vetvetiu',
    queueFull: 'Shumë veprime pa dërguar — prisni të kthehet rrjeti', queueNoStore: 'Telefoni s’e ruan dot — provoni sërish kur të ketë rrjet', queuedChanged: 'Porosia ndryshoi sa ishit pa lidhje', queuedRefused: 'Veprimi nuk u pranua',
    offered: 'Ju propozohet', offerLapsed: 'Koha kaloi — porosia është sërish e lirë, por ende mund ta merrni', timeLeft: 'Mbetën',
    cashInHand: 'Para në dorë', stillOnRoad: 'Ende në rrugë', today: 'Sot', days7: '7 ditë', days30: '30 ditë', earningsHint: 'Dorëzime, para të mbledhura dhe bakshishe. Paratë ia jepni lokalit, bakshishet janë tuajat. dowiz nuk llogarit pagën.', tips: 'bakshishe',
    emptyHistory: 'Ende bosh', emptyHistoryHint: 'Dorëzimet e mbyllura shfaqen këtu',
    voiceUnsupported: 'Shfletuesi nuk njeh zërin', micDenied: 'S’ka leje për mikrofonin', voiceOffline: 'Njohja e zërit s’punon offline', asking: 'Po pyes…', openWaiting: 'Të hapura {open}, në pritje {waiting}',
    etaToDoor: 'deri te dera', km: 'km', min: 'min',
    help: 'Ndihma', gStep: 'Hapi {i} nga {n}', gSkip: 'Kapërce', gLater: 'Mbaro më vonë', gBack: 'Prapa', gNext: 'Tjetra', gDone: 'Gati', gPaused: 'Turi u ruajt — «Ndihma» vazhdon nga këtu', gSkipped: 'Turi u kapërcye — hapet gjithnjë nga «Ndihma»', gFinished: 'Gati. «?» të vegjël pranë elementeve shpjegojnë secilin', gUnfinished: 'Turi s’u mbyll — «Ndihma» vazhdon nga i njëjti vend', gWhat: 'Çfarë është', gClose: 'Mbyll',
    hWelcomeT: 'Ky është aplikacioni juaj i korrierit', hWelcome: 'Një ekran — një punë. Tre hapa të shkurtër tregojnë çfarë është ku. Mund ta kapërceni ose ta mbaroni më vonë.',
    hShiftT: 'Turni', hShift: 'Derisa turni s’është hapur, porositë nuk vijnë. Në turn këtu shihni dorëzimet dhe paratë e mbledhura.',
    hSheetT: 'Një punë në ekran', hSheet: 'Porositë e gatshme shfaqen këtu. Zgjidhni një, shtypni «Merr» — dhe ekrani ju çon hap pas hapi deri te «U dorëzua».',
    hMicT: 'Me zë', hMic: 'Thoni «e mora», «u dorëzua» ose «ku është tjetra». Aplikacioni përsërit çfarë dëgjoi dhe kërkon konfirmim.',
    hAskT: 'Pyetje për dërgesat', hAsk: 'Përgjigjet vetëm për turnet dhe porositë tuaja: sa fituat, ku të shkoni, çfarë ndodhi dje.',
    hHelpT: 'Ndihma', hHelp: 'Ky buton përsërit turin. Ekziston vetëm kur qëndroni, jo në rrugë.',
  },
  en: {
    appTitle: 'dowiz · courier', offline: 'offline', onShift: 'on shift', gps: 'GPS', gpsDenied: 'GPS denied', gpsUnavailable: 'GPS unavailable', gpsUnit: 'm',
    backgroundGps: 'The app was in the background — GPS may have paused', sessionOver: 'Session ended',
    theme: 'Theme', themeSystem: 'Theme: as the phone', themeDark: 'Theme: dark', themeLight: 'Theme: light', language: 'Language', sayCommand: 'Say a command', stopListening: 'Stop listening', voice: 'Voice', yes: 'Yes', no: 'No',
    loginTitle: 'Courier sign-in', loginLine: 'One screen, one job. The road leads to “Delivered”.', emailOrPhone: 'Email or phone', password: 'Password', signIn: 'Sign in', signingIn: 'Signing in…', haveCode: 'I have an invite code',
    claimTitle: 'Invite code', claimHint: 'The venue gave you the code. It lasts a week and works once.', yourPhone: 'Your phone', code: 'Code', choosePassword: 'Choose a password', passwordHint: 'At least 8 characters. The venue never sees it.', start: 'Start', checking: 'Checking…', havePassword: 'I already have a password',
    loading: 'Loading', noLink: 'No connection to the venue', retry: 'Try again', youAreOffline: 'You are offline', offlineHint: 'Orders will not come in until you open a shift', openShift: 'Start shift', endShift: 'End shift',
    noneFree: 'No free orders', noneFreeHint: 'As soon as something is ready, it appears here', askPlaceholder: 'Ask about my deliveries…', myShifts: 'My shifts', history: 'History', thinking: 'Thinking…',
    readyForPickup: 'Ready for pickup', pcs: 'pcs', pickOne: 'pick one and take it', take: 'Take', delivering: 'Delivering', pickUpOrder: 'Pick up the order', onTheWay: 'On the way', ready: 'Ready', items: 'items', paidOnline: 'Paid online',
    delivered: 'Delivered', deliveredAria: 'Delivered — swipe or press', swipe: 'swipe →', pickedUp: 'Picked up', inMaps: 'In maps', call: 'Call', saving: 'Saving…', confirmDelivered: 'Mark as delivered?',
    howMuchCash: 'How much cash did you receive?', confirm: 'Confirm', back: 'Back', badAmount: 'Invalid amount', shortfall: 'Short by', recorded: 'recorded',
    queued: 'Will be sent', queuedN: '{n} unsent', queuedHint: 'No connection — saved on this phone and sent by itself as soon as there is one', queuedSaved: 'Saved — it will send itself',
    queueFull: 'Too many unsent actions — wait for a connection', queueNoStore: 'This phone will not store it — try again when there is a connection', queuedChanged: 'This order changed while you were away', queuedRefused: 'That action was not accepted',
    offered: 'Offered to you', offerLapsed: 'Time is up — the order is free again, but you can still take it', timeLeft: 'Left',
    cashInHand: 'Cash in hand', stillOnRoad: 'Still on the road', today: 'Today', days7: '7 days', days30: '30 days', earningsHint: 'Deliveries, cash collected and tips. Cash goes to the venue, tips are yours. dowiz does not compute pay.', tips: 'tips',
    emptyHistory: 'Nothing yet', emptyHistoryHint: 'Finished deliveries appear here',
    voiceUnsupported: 'This browser cannot recognise speech', micDenied: 'No microphone permission', voiceOffline: 'Speech recognition needs a connection', asking: 'Asking…', openWaiting: 'Open {open}, waiting {waiting}',
    etaToDoor: 'to the door', km: 'km', min: 'min',
    help: 'Help', gStep: 'Step {i} of {n}', gSkip: 'Skip', gLater: 'Finish later', gBack: 'Back', gNext: 'Next', gDone: 'Done', gPaused: 'Tour saved — “Help” continues from here', gSkipped: 'Tour skipped — “Help” opens it any time', gFinished: 'Done. The small “?” beside controls explain each one', gUnfinished: 'Tour unfinished — “Help” continues from the same spot', gWhat: 'What is', gClose: 'Close',
    hWelcomeT: 'This is your courier app', hWelcome: 'One screen — one job. Three short steps show what is where. You can skip or finish later.',
    hShiftT: 'Shift', hShift: 'Until a shift is open, no orders come in. On shift, this shows deliveries and cash collected.',
    hSheetT: 'One job on screen', hSheet: 'Ready orders appear here. Pick one, press “Take” — and the screen leads step by step to “Delivered”.',
    hMicT: 'By voice', hMic: 'Say “picked up”, “delivered” or “where next”. The app repeats what it heard and asks you to confirm.',
    hAskT: 'Questions about deliveries', hAsk: 'Answers only about your shifts and orders: how much you made, where to go next, what happened yesterday.',
    hHelpT: 'Help', hHelp: 'This button repeats the tour. It is here only while you stand still, not on a run.',
  },
  uk: {
    appTitle: 'dowiz · кур’єр', offline: 'офлайн', onShift: 'на зміні', gps: 'GPS', gpsDenied: 'GPS заборонено', gpsUnavailable: 'GPS недоступний', gpsUnit: 'м',
    backgroundGps: 'Застосунок був у фоні — GPS міг перерватися', sessionOver: 'Сесію завершено',
    theme: 'Тема', themeSystem: 'Тема: як на телефоні', themeDark: 'Тема: темна', themeLight: 'Тема: світла', language: 'Мова', sayCommand: 'Сказати команду', stopListening: 'Зупинити запис', voice: 'Голос', yes: 'Так', no: 'Ні',
    loginTitle: 'Вхід для кур’єра', loginLine: 'Один екран, одна справа. Дорога веде до «Доставлено».', emailOrPhone: 'Email або телефон', password: 'Пароль', signIn: 'Увійти', signingIn: 'Входимо…', haveCode: 'У мене код запрошення',
    claimTitle: 'Код запрошення', claimHint: 'Код дав вам заклад. Він діє тиждень і спрацьовує один раз.', yourPhone: 'Ваш телефон', code: 'Код', choosePassword: 'Придумайте пароль', passwordHint: 'Щонайменше 8 символів. Заклад його не побачить.', start: 'Почати', checking: 'Перевіряємо…', havePassword: 'У мене вже є пароль',
    loading: 'Завантажуємо', noLink: 'Немає зв’язку із закладом', retry: 'Спробувати ще раз', youAreOffline: 'Ви офлайн', offlineHint: 'Замовлення не надходитимуть, поки зміну не відкрито', openShift: 'Почати зміну', endShift: 'Завершити зміну',
    noneFree: 'Вільних замовлень немає', noneFreeHint: 'Щойно щось буде готове — з’явиться тут', askPlaceholder: 'Спитати про мої доставки…', myShifts: 'Мої зміни', history: 'Історія', thinking: 'Думає…',
    readyForPickup: 'Готові до забору', pcs: 'шт.', pickOne: 'оберіть і візьміть', take: 'Взяти', delivering: 'Доставляєте', pickUpOrder: 'Заберіть замовлення', onTheWay: 'В дорозі', ready: 'Готове', items: 'поз.', paidOnline: 'Оплачено онлайн',
    delivered: 'Доставлено', deliveredAria: 'Доставлено — проведіть або натисніть', swipe: 'проведіть →', pickedUp: 'Забрав', inMaps: 'У картах', call: 'Подзвонити', saving: 'Записуємо…', confirmDelivered: 'Позначити як доставлене?',
    howMuchCash: 'Скільки готівки отримано?', confirm: 'Підтвердити', back: 'Назад', badAmount: 'Некоректна сума', shortfall: 'Недостача', recorded: 'записано',
    queued: 'Надішлемо', queuedN: '{n} не надіслано', queuedHint: 'Немає зв’язку — зберегли на телефоні й надішлемо самі, щойно він з’явиться', queuedSaved: 'Зберегли — надішлеться саме',
    queueFull: 'Забагато ненадісланих дій — дочекайтеся зв’язку', queueNoStore: 'Телефон не зберігає — спробуйте, коли буде зв’язок', queuedChanged: 'Замовлення змінилося, поки ви були без зв’язку', queuedRefused: 'Дію не прийнято',
    offered: 'Вам пропонують', offerLapsed: 'Час вийшов — замовлення знову вільне, але ви ще можете його взяти', timeLeft: 'Залишилось',
    cashInHand: 'Готівка на руках', stillOnRoad: 'Ще в дорозі', today: 'Сьогодні', days7: '7 днів', days30: '30 днів', earningsHint: 'Доставки, зібрана готівка й чайові. Готівку віддаєте закладу, чайові — ваші. Розрахунок оплати dowiz не веде.', tips: 'чайові',
    emptyHistory: 'Поки порожньо', emptyHistoryHint: 'Завершені доставки з’являться тут',
    voiceUnsupported: 'Браузер не розпізнає голос', micDenied: 'Немає дозволу на мікрофон', voiceOffline: 'Розпізнавання недоступне офлайн', asking: 'Питаю…', openWaiting: 'Відкритих {open}, чекає {waiting}',
    etaToDoor: 'до дверей', km: 'км', min: 'хв',
    help: 'Довідка', gStep: 'Крок {i} з {n}', gSkip: 'Пропустити', gLater: 'Завершити пізніше', gBack: 'Назад', gNext: 'Далі', gDone: 'Готово', gPaused: 'Тур збережено — «Довідка» продовжить з цього місця', gSkipped: 'Тур пропущено — його завжди можна відкрити через «Довідка»', gFinished: 'Готово. Маленькі «?» біля елементів пояснюють кожен окремо', gUnfinished: 'Тур не завершено — «Довідка» продовжить з того ж місця', gWhat: 'Що це', gClose: 'Закрити',
    hWelcomeT: 'Це ваш застосунок кур’єра', hWelcome: 'Один екран — одна справа. Три короткі кроки покажуть, що тут до чого. Можна пропустити або завершити пізніше.',
    hShiftT: 'Зміна', hShift: 'Поки зміну не відкрито, замовлення не надходять. На зміні тут видно кількість доставок і зібрану готівку.',
    hSheetT: 'Одна справа на екрані', hSheet: 'Готові замовлення з’являються тут. Оберіть одне, натисніть «Взяти» — і далі екран веде крок за кроком до «Доставлено».',
    hMicT: 'Голосом', hMic: 'Скажіть «взяв», «доставив» або «де наступне». Застосунок повторить, що почув, і попросить підтвердити.',
    hAskT: 'Питання про доставки', hAsk: 'Відповідає лише про ваші зміни й замовлення: скільки заробили, куди їхати далі, що було вчора.',
    hHelpT: 'Довідка', hHelp: 'Ця кнопка повторить тур. Вона є лише тоді, коли ви стоїте, не на маршруті.',
  },
};

export let lang = LANGS.includes(safeGet(STORAGE_KEY)) ? safeGet(STORAGE_KEY) : 'sq';
export const t = (k, vars) => {
  let v = (T[lang] && T[lang][k]) ?? T.en[k] ?? k;
  if (vars) for (const [name, val] of Object.entries(vars)) v = v.replace(`{${name}}`, String(val));
  return v;
};
export const intlLocale = () => lang;
export const voiceLocale = () => VOICE[lang] || VOICE.en;
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
