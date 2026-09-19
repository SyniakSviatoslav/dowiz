// The console's words, in the three languages its owners read.
//
// THE LANGUAGE SWITCH CHANGES TEXT, NOT SCREENS, the same way the storefront
// does it: every piece of static copy carries a `data-t` key and is rewritten
// in place. The console used to be written in Ukrainian only; a venue in
// Durrës is run by Albanians and its owner may well be neither.
//
// sq is the default: the venue is in Albania.

import { safeGet, safeSet } from '/store/storage.js';

export const T = {
  sq: {
    street:'Rruga', house:'Nr.', apartment:'Ap.', entrance:'Hyrja', floor:'Kati', hoursAgo:'orë më parë', daysAgo:'ditë më parë',
    // added 2026-09-19: promos, live eta, address parts, stock, notifications
    cash:'Para në dorë', discount:'Zbritje', maxUses:'Përdorime maks.', promo:'Kodi', until:'Deri më', etaMin:'min', etaRange:'Koha e mbërritjes', onMap:'Në hartë', privateHouse:'Shtëpi private', reserved:'rezervuar',
    tgToken:'Tokeni i botit', tgTokenHint:'Krijoni një bot te @BotFather dhe ngjisni tokenin. Boti ju shkruan për çdo porosi të re.', tgChatHint:'ID e bisedës: shkruajini botit një herë, pastaj shtypni provën.', tokenSet:'tokeni është ruajtur', testOk:'Mesazhi mbërriti',
    // shell
    console:'Paneli i pronarit', signIn:'Hyni', signingIn:'Duke hyrë…', email:'Email', password:'Fjalëkalimi', signOut:'Dilni',
    language:'Gjuha', retry:'Provo përsëri', loading:'Po ngarkohet…', save:'Ruaj', saved:'U ruajt', cancel:'Anulo', done:'Në rregull',
    back:'Prapa', add:'Shto', remove:'Hiq', edit:'Ndrysho', search:'Kërko', close:'Mbyll', copy:'Kopjo', copied:'U kopjua',
    on:'Ndezur', off:'Fikur', today:'Sot', week:'7 ditë', month:'30 ditë', more:'Më shumë', none:'Asnjë', all:'Të gjitha',
    // tabs
    tabOrders:'Porositë', tabMenu:'Menyja', tabStock:'Magazina', tabCouriers:'Korrierët', tabMore:'Tjetër',
    // venue state
    open:'Hapur', closed:'Mbyllur', busy:'I zënë', paused:'Dërgesat ndalur', setState:'Gjendja e lokalit',
    // dashboard
    todayOrders:'Porosi sot', pending:'Presin', active:'Në punë', revenue:'Xhiro', scheduled:'Me orar',
    // orders
    live:'Në punë', history:'Historia', noOrders:'Ende asnjë porosi', noLive:'Asgjë në punë tani', findOrder:'Numër, emër, telefon, pjatë…',
    newOrder:'Porosi e re', order:'Porosia', items:'Artikuj', customer:'Klienti', address:'Adresa', pickup:'Marrje vetë', delivery:'Dërgesë',
    note:'Shënim', payment:'Pagesa', total:'Totali', tip:'Bakshish', when:'Kur', asap:'Sa më shpejt', courier:'Korrieri', assign:'Cakto',
    accept:'Prano', startCooking:'Fillo gatimin', markReady:'Gati', handToCourier:'Jepja korrierit', delivered:'U dorëzua', reject:'Refuzo',
    cancelOrder:'Anulo porosinë', reason:'Arsyeja', print:'Kopjo tekstin', call:'Telefono', minutesAgo:'min më parë', justNow:'tani',
    st:{PENDING:'E re',CONFIRMED:'Pranuar',PREPARING:'Po gatuhet',READY:'Gati',IN_DELIVERY:'Në rrugë',DELIVERED:'Dorëzuar',
        REJECTED:'Refuzuar',CANCELLED:'Anuluar',SCHEDULED:'Planifikuar',PICKED_UP:'Marrë'},
    pay:{cash:'Para në dorë',card:'Kartë',apple_pay:'Apple Pay',google_pay:'Google Pay',crypto:'Kripto'},
    // menu
    categories:'Kategoritë', dishes:'Pjata', onSale:'Në shitje', stopList:'Stop-lista', price:'Çmimi', photo:'Fotografia', noPhoto:'Pa fotografi',
    uploadPhoto:'Ngarko foto', removePhoto:'Hiq foton', ingredients:'Përbërësit', nutrition:'Vlerat ushqyese', kcal:'kcal', protein:'Proteina', fat:'Yndyrë', carbs:'Karbohidrate',
    weight:'Pesha, g', cookingMin:'Gatimi, min', tags:'Etiketat', translations:'Përkthimet', name:'Emri', description:'Përshkrimi',
    unavailableNote:'Pse s’ka', putOnSale:'Vëre në shitje', takeOff:'Hiqe nga shitja', importMenu:'Importo menynë', sold:'shitur', undeclared:'pa deklaruar',
    // stock
    supplies:'Furnizimet', ingredient:'Përbërësi', level:'Sasia', unit:'Njësia', low:'Pak', out:'Mbaroi', received:'U mor', wasted:'U hodh', counted:'U numërua',
    move:'Lëvizje', addSupply:'Shto furnizim', minLevel:'Minimumi', noStock:'Magazina s’është modeluar ende', stockHint:'Shtoni përbërësit që ndiqni; porosia refuzohet vetë kur mbaron diçka.',
    // couriers
    invite:'Fto korrier', inviteHint:'Kodi vlen 7 ditë; korrieri e shkruan në aplikacionin e vet.', phone:'Telefoni', inviteCode:'Kodi', onShift:'Në turn', offShift:'Jashtë turnit',
    activeC:'Aktiv', deactivate:'Çaktivizo', activate:'Aktivizo', noCouriers:'Ende asnjë korrier', deliveries:'Dorëzime', lastSeen:'Parë së fundi', uninvite:'Anulo ftesën',
    // more
    marketing:'Marketing', promos:'Kodet e zbritjes', posts:'Postimet', analytics:'Analitika', customers:'Klientët', settings:'Cilësimet', health:'Gjendja e sistemit',
    notifications:'Njoftimet', channels:'Kanalet e porosive', integrations:'Integrimet', branding:'Marka', hours:'Orari', deliveryTerms:'Dërgesa', payments:'Pagesat',
    features:'Funksionet', apiKeys:'Çelësat API', backup:'Rezervë', activation:'Aktivizimi', venue:'Lokali',
    // notifications
    telegram:'Telegram', whatsapp:'WhatsApp', tgHow:'Lidhni botin: shkruani /start botit nga telefoni i lokalit — porositë e reja vijnë aty.', tgChannel:'Kanali i postimeve',
    tgNotSet:'Boti i Telegram-it nuk është konfiguruar në këtë hub', waNotYet:'WhatsApp Business nuk është lidhur ende — kërkon një llogari WhatsApp Business API.',
    ownerChat:'Chat-i i pronarit', linked:'I lidhur', notLinked:'Jo i lidhur', testMessage:'Dërgo mesazh prove',
    // channels
    chStore:'Dyqani online', chPhone:'Me telefon', chTelegramBot:'Bot Telegram', chApi:'API / partnerë', chAggregators:'Agregatorë (Wolt, Glovo…)', comingSoon:'Së shpejti',
    apiHint:'Një çelës API lejon një sistem tjetër (kasë, agregator) të lexojë menynë dhe të dërgojë porosi.', newKey:'Çelës i ri', revoke:'Revoko', keyOnce:'Kopjojeni tani — nuk shfaqet më.',
    // social
    social:'Rrjetet sociale', autopost:'Autopostim', autopostHint:'Hubi propozon postime nga fakte të vërteta (pjatë e re, më e porositura, u hap). Asgjë s’publikohet pa ju.',
    draft:'Draft', approve:'Publiko', rejectPost:'Hiqe', drafts:'Draftet', published:'Publikuar', failed:'Dështoi', noPosts:'Ende asnjë draft', makeDraft:'Propozo postim',
    instagram:'Instagram', facebook:'Facebook', tiktok:'TikTok', socialNotYet:'nuk është lidhur ende',
    // settings
    venueName:'Emri', venuePhone:'Telefoni', venueAddress:'Adresa', deliveryFee:'Tarifa e dërgesës', freeOver:'Falas nga', minOrder:'Porosia minimale',
    pickupOn:'Marrje vetë', stripe:'Stripe (kartë, Apple/Google Pay)', stripeNotSet:'Çelësat e Stripe s’janë vendosur në hub', cryptoWallets:'Portofolat kripto', network:'Rrjeti', symbol:'Monedha', walletAddress:'Adresa',
    seal:'Vula', motif:'Motivi', warmTone:'Ngjyra e ngrohtë', sageTone:'Ngjyra e gjelbër', leaf:'Gjethe', wave:'Valë', noneMotif:'Pa motiv',
    primary:'Ngjyra kryesore', paper:'Letra', typePair:'Shkronjat', radius:'Rrumbullakimi', preview:'Parapamja',
    day:{0:'E hënë',1:'E martë',2:'E mërkurë',3:'E enjte',4:'E premte',5:'E shtunë',6:'E diel'}, closedDay:'Mbyllur',
    // analytics
    orders7:'Porosi', revenue7:'Xhiro', avgCheck:'Çeku mesatar', topDishes:'Më të porositurat', byHour:'Sipas orës', byDay:'Sipas ditës',
    // health
    hubOk:'Hubi në rregull', latency:'Vonesa', images:'Imazhet', lastBackup:'Rezerva e fundit', download:'Shkarko',
    // errors
    loadFail:'Nuk u ngarkua', sessionOver:'Sesioni mbaroi', required:'E detyrueshme',
  },
  en: {
    street:'Street', house:'No.', apartment:'Apt', entrance:'Entrance', floor:'Floor', hoursAgo:'h ago', daysAgo:'d ago',
    // added 2026-09-19: promos, live eta, address parts, stock, notifications
    cash:'Cash', discount:'Discount', maxUses:'Max uses', promo:'Code', until:'Until', etaMin:'min', etaRange:'Arrives in', onMap:'On the map', privateHouse:'Private house', reserved:'held',
    tgToken:'Bot token', tgTokenHint:'Make a bot with @BotFather and paste its token. It messages you about every new order.', tgChatHint:'Chat id: write to the bot once, then press test.', tokenSet:'token is saved', testOk:'The message arrived',
    console:'Owner console', signIn:'Sign in', signingIn:'Signing in…', email:'Email', password:'Password', signOut:'Sign out',
    language:'Language', retry:'Try again', loading:'Loading…', save:'Save', saved:'Saved', cancel:'Cancel', done:'Done',
    back:'Back', add:'Add', remove:'Remove', edit:'Edit', search:'Search', close:'Close', copy:'Copy', copied:'Copied',
    on:'On', off:'Off', today:'Today', week:'7 days', month:'30 days', more:'More', none:'None', all:'All',
    tabOrders:'Orders', tabMenu:'Menu', tabStock:'Stock', tabCouriers:'Couriers', tabMore:'More',
    open:'Open', closed:'Closed', busy:'Busy', paused:'Delivery paused', setState:'Venue state',
    todayOrders:'Orders today', pending:'Waiting', active:'In progress', revenue:'Revenue', scheduled:'Scheduled',
    live:'Live', history:'History', noOrders:'No orders yet', noLive:'Nothing in progress', findOrder:'Number, name, phone, dish…',
    newOrder:'New order', order:'Order', items:'Items', customer:'Customer', address:'Address', pickup:'Pickup', delivery:'Delivery',
    note:'Note', payment:'Payment', total:'Total', tip:'Tip', when:'When', asap:'As soon as possible', courier:'Courier', assign:'Assign',
    accept:'Accept', startCooking:'Start cooking', markReady:'Ready', handToCourier:'Hand to courier', delivered:'Delivered', reject:'Reject',
    cancelOrder:'Cancel order', reason:'Reason', print:'Copy as text', call:'Call', minutesAgo:'min ago', justNow:'just now',
    st:{PENDING:'New',CONFIRMED:'Accepted',PREPARING:'Cooking',READY:'Ready',IN_DELIVERY:'On the way',DELIVERED:'Delivered',
        REJECTED:'Rejected',CANCELLED:'Cancelled',SCHEDULED:'Scheduled',PICKED_UP:'Picked up'},
    pay:{cash:'Cash',card:'Card',apple_pay:'Apple Pay',google_pay:'Google Pay',crypto:'Crypto'},
    categories:'Categories', dishes:'Dishes', onSale:'On sale', stopList:'Stop list', price:'Price', photo:'Photo', noPhoto:'No photo',
    uploadPhoto:'Upload photo', removePhoto:'Remove photo', ingredients:'Ingredients', nutrition:'Nutrition', kcal:'kcal', protein:'Protein', fat:'Fat', carbs:'Carbs',
    weight:'Weight, g', cookingMin:'Cooking, min', tags:'Tags', translations:'Translations', name:'Name', description:'Description',
    unavailableNote:'Why unavailable', putOnSale:'Put on sale', takeOff:'Take off sale', importMenu:'Import menu', sold:'sold', undeclared:'undeclared',
    supplies:'Supplies', ingredient:'Ingredient', level:'Level', unit:'Unit', low:'Low', out:'Out', received:'Received', wasted:'Wasted', counted:'Counted',
    move:'Movement', addSupply:'Add supply', minLevel:'Minimum', noStock:'Stock is not modelled yet', stockHint:'Add the ingredients you track; an order is refused by itself when something runs out.',
    invite:'Invite courier', inviteHint:'The code is valid for 7 days; the courier types it in their app.', phone:'Phone', inviteCode:'Code', onShift:'On shift', offShift:'Off shift',
    activeC:'Active', deactivate:'Deactivate', activate:'Activate', noCouriers:'No couriers yet', deliveries:'Deliveries', lastSeen:'Last seen', uninvite:'Cancel invite',
    marketing:'Marketing', promos:'Promo codes', posts:'Posts', analytics:'Analytics', customers:'Customers', settings:'Settings', health:'System health',
    notifications:'Notifications', channels:'Order channels', integrations:'Integrations', branding:'Brand', hours:'Hours', deliveryTerms:'Delivery', payments:'Payments',
    features:'Features', apiKeys:'API keys', backup:'Backup', activation:'Activation', venue:'Venue',
    telegram:'Telegram', whatsapp:'WhatsApp', tgHow:'Link the bot: send /start to the bot from the venue phone — new orders arrive there.', tgChannel:'Posting channel',
    tgNotSet:'The Telegram bot is not configured on this hub', waNotYet:'WhatsApp Business is not linked yet — it needs a WhatsApp Business API account.',
    ownerChat:'Owner chat', linked:'Linked', notLinked:'Not linked', testMessage:'Send a test message',
    chStore:'Online storefront', chPhone:'By phone', chTelegramBot:'Telegram bot', chApi:'API / partners', chAggregators:'Aggregators (Wolt, Glovo…)', comingSoon:'Coming soon',
    apiHint:'An API key lets another system (a till, an aggregator) read the menu and send orders.', newKey:'New key', revoke:'Revoke', keyOnce:'Copy it now — it is not shown again.',
    social:'Social media', autopost:'Autoposting', autopostHint:'The hub drafts posts from true facts (a new dish, the most ordered, reopened). Nothing publishes without you.',
    draft:'Draft', approve:'Publish', rejectPost:'Discard', drafts:'Drafts', published:'Published', failed:'Failed', noPosts:'No drafts yet', makeDraft:'Draft a post',
    instagram:'Instagram', facebook:'Facebook', tiktok:'TikTok', socialNotYet:'not linked yet',
    venueName:'Name', venuePhone:'Phone', venueAddress:'Address', deliveryFee:'Delivery fee', freeOver:'Free from', minOrder:'Minimum order',
    pickupOn:'Pickup', stripe:'Stripe (card, Apple/Google Pay)', stripeNotSet:'Stripe keys are not set on the hub', cryptoWallets:'Crypto wallets', network:'Network', symbol:'Coin', walletAddress:'Address',
    seal:'Seal', motif:'Motif', warmTone:'Warm tone', sageTone:'Leaf tone', leaf:'Leaf', wave:'Wave', noneMotif:'No motif',
    primary:'Primary colour', paper:'Paper', typePair:'Type', radius:'Corner radius', preview:'Preview',
    day:{0:'Monday',1:'Tuesday',2:'Wednesday',3:'Thursday',4:'Friday',5:'Saturday',6:'Sunday'}, closedDay:'Closed',
    orders7:'Orders', revenue7:'Revenue', avgCheck:'Average check', topDishes:'Most ordered', byHour:'By hour', byDay:'By day',
    hubOk:'Hub is fine', latency:'Latency', images:'Images', lastBackup:'Last backup', download:'Download',
    loadFail:'Could not load', sessionOver:'Session expired', required:'Required',
  },
  uk: {
    street:'Вулиця', house:'Буд.', apartment:'Кв.', entrance:'Під’їзд', floor:'Поверх', hoursAgo:'год тому', daysAgo:'дн тому',
    // added 2026-09-19: promos, live eta, address parts, stock, notifications
    cash:'Готівка', discount:'Знижка', maxUses:'Макс. використань', promo:'Код', until:'До', etaMin:'хв', etaRange:'Прибуде за', onMap:'На мапі', privateHouse:'Приватний будинок', reserved:'зарезервовано',
    tgToken:'Токен бота', tgTokenHint:'Створіть бота у @BotFather і вставте токен. Бот писатиме вам про кожне нове замовлення.', tgChatHint:'ID чату: напишіть боту один раз, потім натисніть перевірку.', tokenSet:'токен збережено', testOk:'Повідомлення дійшло',
    console:'Панель власника', signIn:'Увійти', signingIn:'Входимо…', email:'Email', password:'Пароль', signOut:'Вийти',
    language:'Мова', retry:'Спробувати ще', loading:'Завантажуємо…', save:'Зберегти', saved:'Збережено', cancel:'Скасувати', done:'Готово',
    back:'Назад', add:'Додати', remove:'Прибрати', edit:'Змінити', search:'Пошук', close:'Закрити', copy:'Копіювати', copied:'Скопійовано',
    on:'Увімкнено', off:'Вимкнено', today:'Сьогодні', week:'7 днів', month:'30 днів', more:'Ще', none:'Немає', all:'Усі',
    tabOrders:'Замовлення', tabMenu:'Меню', tabStock:'Склад', tabCouriers:'Кур’єри', tabMore:'Ще',
    open:'Відчинено', closed:'Зачинено', busy:'Зайнято', paused:'Доставку призупинено', setState:'Стан закладу',
    todayOrders:'Замовлень сьогодні', pending:'Чекають', active:'У роботі', revenue:'Виручка', scheduled:'На час',
    live:'У роботі', history:'Історія', noOrders:'Замовлень ще немає', noLive:'Зараз нічого в роботі', findOrder:'Номер, ім’я, телефон, страва…',
    newOrder:'Нове замовлення', order:'Замовлення', items:'Позиції', customer:'Клієнт', address:'Адреса', pickup:'Самовивіз', delivery:'Доставка',
    note:'Коментар', payment:'Оплата', total:'Разом', tip:'Чайові', when:'Коли', asap:'Якнайшвидше', courier:'Кур’єр', assign:'Призначити',
    accept:'Прийняти', startCooking:'Готувати', markReady:'Готово', handToCourier:'Віддати кур’єру', delivered:'Доставлено', reject:'Відхилити',
    cancelOrder:'Скасувати замовлення', reason:'Причина', print:'Скопіювати текстом', call:'Зателефонувати', minutesAgo:'хв тому', justNow:'щойно',
    st:{PENDING:'Нове',CONFIRMED:'Прийнято',PREPARING:'Готується',READY:'Готове',IN_DELIVERY:'У дорозі',DELIVERED:'Доставлено',
        REJECTED:'Відхилено',CANCELLED:'Скасовано',SCHEDULED:'Заплановано',PICKED_UP:'Забрано'},
    pay:{cash:'Готівка',card:'Картка',apple_pay:'Apple Pay',google_pay:'Google Pay',crypto:'Крипто'},
    categories:'Категорії', dishes:'Страви', onSale:'У продажу', stopList:'Стоп-лист', price:'Ціна', photo:'Фото', noPhoto:'Без фото',
    uploadPhoto:'Завантажити фото', removePhoto:'Прибрати фото', ingredients:'Інгредієнти', nutrition:'Харчова цінність', kcal:'ккал', protein:'Білки', fat:'Жири', carbs:'Вуглеводи',
    weight:'Вага, г', cookingMin:'Готування, хв', tags:'Теги', translations:'Переклади', name:'Назва', description:'Опис',
    unavailableNote:'Чому немає', putOnSale:'У продаж', takeOff:'Зняти з продажу', importMenu:'Імпортувати меню', sold:'продано', undeclared:'не заявлено',
    supplies:'Постачання', ingredient:'Інгредієнт', level:'Залишок', unit:'Одиниця', low:'Мало', out:'Закінчилось', received:'Прийнято', wasted:'Списано', counted:'Перераховано',
    move:'Рух', addSupply:'Додати позицію', minLevel:'Мінімум', noStock:'Склад ще не змодельовано', stockHint:'Додайте інгредієнти, які відстежуєте; замовлення саме відхилиться, коли щось закінчиться.',
    invite:'Запросити кур’єра', inviteHint:'Код дійсний 7 днів; кур’єр вводить його у своєму застосунку.', phone:'Телефон', inviteCode:'Код', onShift:'На зміні', offShift:'Не на зміні',
    activeC:'Активний', deactivate:'Деактивувати', activate:'Активувати', noCouriers:'Кур’єрів ще немає', deliveries:'Доставок', lastSeen:'Востаннє', uninvite:'Скасувати запрошення',
    marketing:'Маркетинг', promos:'Промокоди', posts:'Публікації', analytics:'Аналітика', customers:'Клієнти', settings:'Налаштування', health:'Стан системи',
    notifications:'Сповіщення', channels:'Канали замовлень', integrations:'Інтеграції', branding:'Бренд', hours:'Години роботи', deliveryTerms:'Доставка', payments:'Оплата',
    features:'Функції', apiKeys:'API-ключі', backup:'Резервна копія', activation:'Активація', venue:'Заклад',
    telegram:'Telegram', whatsapp:'WhatsApp', tgHow:'Підключіть бота: напишіть йому /start з телефону закладу — нові замовлення приходитимуть туди.', tgChannel:'Канал для публікацій',
    tgNotSet:'Telegram-бот на цьому хабі не налаштований', waNotYet:'WhatsApp Business ще не підключено — потрібен акаунт WhatsApp Business API.',
    ownerChat:'Чат власника', linked:'Підключено', notLinked:'Не підключено', testMessage:'Надіслати тестове повідомлення',
    chStore:'Онлайн-вітрина', chPhone:'По телефону', chTelegramBot:'Telegram-бот', chApi:'API / партнери', chAggregators:'Агрегатори (Wolt, Glovo…)', comingSoon:'Незабаром',
    apiHint:'API-ключ дає іншій системі (касі, агрегатору) читати меню й надсилати замовлення.', newKey:'Новий ключ', revoke:'Відкликати', keyOnce:'Скопіюйте зараз — більше не покажемо.',
    social:'Соцмережі', autopost:'Автопостинг', autopostHint:'Хаб пропонує пости з правдивих фактів (нова страва, найпопулярніша, відкриття). Нічого не публікується без вас.',
    draft:'Чернетка', approve:'Опублікувати', rejectPost:'Відхилити', drafts:'Чернетки', published:'Опубліковано', failed:'Не вдалося', noPosts:'Чернеток ще немає', makeDraft:'Запропонувати пост',
    instagram:'Instagram', facebook:'Facebook', tiktok:'TikTok', socialNotYet:'ще не підключено',
    venueName:'Назва', venuePhone:'Телефон', venueAddress:'Адреса', deliveryFee:'Вартість доставки', freeOver:'Безкоштовно від', minOrder:'Мінімальне замовлення',
    pickupOn:'Самовивіз', stripe:'Stripe (картка, Apple/Google Pay)', stripeNotSet:'Ключі Stripe на хабі не задано', cryptoWallets:'Криптогаманці', network:'Мережа', symbol:'Монета', walletAddress:'Адреса',
    seal:'Печатка', motif:'Мотив', warmTone:'Теплий тон', sageTone:'Тон листя', leaf:'Листя', wave:'Хвиля', noneMotif:'Без мотиву',
    primary:'Основний колір', paper:'Папір', typePair:'Шрифт', radius:'Заокруглення', preview:'Попередній перегляд',
    day:{0:'Понеділок',1:'Вівторок',2:'Середа',3:'Четвер',4:'Пʼятниця',5:'Субота',6:'Неділя'}, closedDay:'Зачинено',
    orders7:'Замовлень', revenue7:'Виручка', avgCheck:'Середній чек', topDishes:'Найпопулярніші', byHour:'За годинами', byDay:'За днями',
    hubOk:'Хаб у нормі', latency:'Затримка', images:'Образи', lastBackup:'Остання копія', download:'Завантажити',
    loadFail:'Не завантажилось', sessionOver:'Сесія завершилась', required:'Обов’язково',
  },
};

export const LANGS = ['sq', 'en', 'uk'];
export let lang = LANGS.includes(safeGet('dw_admin_lang')) ? safeGet('dw_admin_lang') : 'sq';
export const t = k => (T[lang] && T[lang][k]) ?? T.en[k] ?? k;
export const st = s => (t('st')[s]) || s;
export const payName = p => (t('pay')[p]) || p || '';
export const intlLocale = () => lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq';

/// Ukrainian needs three forms; Albanian and English two.
export function plural(n, key){
  const forms = t(key);
  if (!Array.isArray(forms)) return String(forms);
  if (lang !== 'uk') return n === 1 ? forms[0] : forms[forms.length - 1];
  const m10 = n % 10, m100 = n % 100;
  if (m10 === 1 && m100 !== 11) return forms[0];
  if (m10 >= 2 && m10 <= 4 && (m100 < 12 || m100 > 14)) return forms[1];
  return forms[2];
}

export function setLang(code){
  if (!LANGS.includes(code) || code === lang) return false;
  lang = code;
  safeSet('dw_admin_lang', lang);
  document.documentElement.lang = lang;
  retranslate(document);
  return true;
}

/// Rewrite text under `root`: `data-t="key"`, `data-t-attr="placeholder:key"`,
/// `data-t-st="STATUS"`. Nothing else moves.
export function retranslate(root = document){
  for (const el of root.querySelectorAll('[data-t]')) {
    const v = t(el.dataset.t);
    if (typeof v === 'string' && el.textContent !== v) el.textContent = v;
  }
  for (const el of root.querySelectorAll('[data-t-attr]')) {
    for (const pair of el.dataset.tAttr.split(/\s+/)) {
      const [attr, key] = pair.split(':');
      if (attr && key) el.setAttribute(attr, t(key));
    }
  }
  for (const el of root.querySelectorAll('[data-t-st]')) el.textContent = st(el.dataset.tSt);
}
