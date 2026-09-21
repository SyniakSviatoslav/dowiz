// The storefront's words, in the three languages its diners read.
//
// THE LANGUAGE SWITCH CHANGES TEXT, NOT SCREENS. Every piece of static copy is
// written into the DOM with a `data-t="key"` beside it, so switching language
// walks those nodes and rewrites their text -- the cards, the scroll position,
// an open sheet and a half-typed search all stay exactly where they were.
// The old switch re-fetched and re-rendered the entire storefront, which threw
// the customer back to the top with an empty basket bar for a second.
//
// Dish names and descriptions are the VENUE'S words, not this table's: they
// arrive from the hub in the requested locale and are patched in place by
// menu.js (`patchTexts`), by product id.
//
// sq is the default: the diners are in Durrës.

import { safeGet, safeSet } from '/store/storage.js';

export const T = {
  sq: { cart:'Shporta', add:'Shto', total:'Totali', chargedIn:'Do të paguhet', checkout:'Vazhdo', empty:'Shporta është bosh',
        emptyHint:'Zgjidhni një pjatë nga menuja', name:'Emri', phone:'Telefoni', address:'Adresa',
        note:'Shënim për korrierin', pay:'Mënyra e pagesës', cash:'Para në dorë', card:'Kartë',
        cashNote:'Paguani korrierit në dorëzim', cardNote:'Kartë ose Apple/Google Pay', place:'Porosit',
        subtotal:'Nëntotali', delivery:'Dërgesa', free:'Falas', closed:'Mbyllur tani',
        closedHint:'Telefononi për të porositur', soldOut:'S’ka', min:'Porosia minimale',
        sent:'Porosia u dërgua', track:'Ndiqni porosinë', offline:'Jeni offline — telefononi',
        required:'E detyrueshme', optional:'(opsionale)', badPhone:'Numër i pavlefshëm', ordering:'Duke dërguar…',
        checkArea:'Kontrolloni adresën', checking:'Po kontrollojmë…',
        retry:'Provo përsëri', loadFail:'Menuja nuk u ngarkua', trackFail:'Porosia nuk u përditësua', loading:'Po ngarkohet…',
        hoursTitle:'Orari', openUntil:'Hapur deri në', opensAt2:'Hap në', closedNow:'Mbyllur tani', onMap:'Shfaq në hartë', directions:'Udhëzime', reviewsTitle:'Vlerësime', fromGoogle:'nga Google Maps', allReviews:'Të gjitha vlerësimet', callUs:'Telefono',
        savedAddresses:'Adresat e ruajtura', forget:'Hiqe', myOrders:'Porositë e mia', noOrders:'Ende asnjë porosi', when:'Kur', asap:'Sa më shpejt',
        onTable:'Shikoni në tryezë', arScan:'Drejtojeni nga tryeza…', arTap:'Prekni për ta vendosur', arFail:'Nuk u hap',
        opensAt:'Hapet', pausedNow:'Dërgesat janë ndalur',
        search:'Kërkoni në meny', sortBy:'Renditni', sortPop:'Si në meny', sortLow:'Çmimi: nga i ulëti',
        sortHigh:'Çmimi: nga i larti', sortAz:'Sipas emrit', noHits:'Asgjë nuk u gjet', clear:'Pastroni',
        onlyAvail:'Vetëm në dispozicion',
        later:'Në një orë tjetër', schedFail:'Koha nuk vlen',
        inArea:'Ne dërgojmë këtu', outArea:'Jashtë zonës sonë të dërgesës', noGeo:'Nuk morëm dot vendndodhjen',
        notDeclared:'Alergjenët nuk janë deklaruar', noneOf14:'Asnjë nga 14 alergjenët',
        tip:'Bakshish për korrierin', tipNo:'Pa bakshish', tipOther:'Tjetër',
        sayHow:'Si ishte?', sayHint:'Vetëm restoranti e lexon. Pa yje, pa vlerësime.',
        sayGo:'Dërgo', saidIt:'Faleminderit',
        how:'Si e merrni', toDoor:'Dërgesë', toPickup:'E marr vetë', pickupAt:'Merreni te',
        avoid:'Alergjenët', avoidHint:'Fshihni pjatat që i përmbajnë', avoidOn:'Fshehur',
        avoidUnknown:'pjata pa deklaratë', clearAvoid:'Shfaqni të gjitha',
        promo:'Kodi i zbritjes', promoApply:'Apliko', promoOff:'Hiq', discount:'Zbritja',
        notify:'Merrni njoftime në Telegram', notifyHint:'Ju njoftojmë sa herë ndryshon porosia',
        // ── the redesign's words ──
        menu:'Menyja', tabSearch:'Kërko', orders:'Porositë', info:'Info', prefs:'Gjuha dhe monedha', language:'Gjuha', currency:'Monedha',
        eta:'Dërgesa', etaMin:'min', prep:'Gatimi', weight:'Pesha', kcal:'kcal', protein:'Proteina', fat:'Yndyrë', carbs:'Karbohidrate',
        ingredients:'Përbërësit', about:'Rreth', filters:'Filtrat', all:'Të gjitha', chefPicks:'Zgjedhjet e shefit',
        viewOrder:'Shiko porosinë', items:'artikuj', pickOnMap:'Zgjidhni në hartë', useMyLocation:'Vendndodhja ime',
        confirmPin:'Konfirmo vendin', dragPin:'Tërhiqeni shenjën te dera juaj', findingAddress:'Po kërkojmë adresën…',
        contact:'Kontakti', extras:'Të tjera', summary:'Përmbledhja', back:'Prapa', done:'Në rregull',
        etaRange:'Koha e pritjes', yourOrder:'Porosia juaj', notDeclaredShort:'Pa deklaruar',
        // ── the cinematic redesign's words ──
        readIn:'Lexoni çmimet në', chooseLang:'Zgjidhni gjuhën',
        applePay:'Apple Pay', googlePay:'Google Pay', crypto:'Kriptovalutë', cardNote2:'Visa, Mastercard',
        walletNote:'Paguani pas porosisë, në adresën që shfaqet', payWith:'Paguani me', network:'Rrjeti',
        sendExactly:'Dërgoni saktësisht', toAddress:'në këtë adresë', copy:'Kopjo', copied:'U kopjua',
        cryptoWait:'Lokali e konfirmon pagesën sapo të mbërrijë', cryptoRate:'Shuma në kriptovalutë sipas kursit të ditës',
        nutrition:'Vlerat ushqyese', approx:'afërsisht, për porcion', pickup:'Marrje vetë', kitchenNote:'Shënim për kuzhinën',
        langs:{sq:'Shqip', en:'English', uk:'Українська'}, curs:{ALL:'Lekë', EUR:'Euro', USD:'Dollarë amerikanë'},
        addToOrder:'Shto në porosi', payLater:'Paguani në dorëzim', ready:'Gati për', min2:'min',
        street:'Rruga', house:'Numri', apartment:'Apartamenti', entrance:'Hyrja', floor:'Kati', privateHouse:'Shtëpi private',
        distance:'Larg', fromVenue:'nga lokali', headingOn:'Ndiq drejtimin tim', headingOff:'Veri lart',
        voice:'Porosit me zë', listening:'Po dëgjoj…', heard:'Dëgjova', notThis:'Jo kjo', sayLike:'Thoni: «dy Philadelphia»',
        noMatch:'Nuk e gjeta në meny', addedByVoice:'U shtua', micDenied:'Mikrofoni nuk u lejua', noVoice:'Shfletuesi nuk dëgjon dot',
        qtyWords:{'një':1,'nje':1,'dy':2,'tre':3,'tri':3,'katër':4,'kater':4,'pesë':5,'pese':5,'gjashtë':6,'gjashte':6,'shtatë':7,'shtate':7,'tetë':8,'tete':8,'nëntë':9,'nente':9},
        stateLbl:'Gjendja', ratingLbl:'Vlerësimi', timeLbl:'Koha e dërgesës', feeLbl:'Dërgesa',
        mapLegend:'Rruga e porosisë', mapVenue:'lokali', mapYou:'ju', mapCourier:'korrieri', taste:'Shija', taste_spicy:'djegës', taste_sweet:'i ëmbël', taste_salty:'i kripur', taste_sour:'i thartë', taste_richness:'i pasur', episode:'Porosia', stepOf:'hapi', ofSteps:'nga', whatTheySay:'Çfarë thonë të tjerët', nextUp:'Më pas', preparingTitle:'Po gatuhet për ju',
        installBody:'Menyja hapet me një prekje, edhe pa lidhje; porosia juaj gjurmohet nga ekrani kryesor.', installNow:'Instalo', installLater:'Më vonë', installNever:'Mos e shfaq më', installApp:'Instaloni si aplikacion', installHint:'Në Safari: Ndaj → Shto në ekranin kryesor', installed:'U instalua',
        tags:{ salmon:'Salmon', tuna:'Ton', shrimp:'Karkalec', vegetarian:'Vegjetariane', hot:'E nxehtë', popular:'Popullore',
               sets:'Sete', bowls:'Bowls', soups:'Supa', drinks:'Pije', freskuese:'Freskuese', kafeteria:'Kafe', alkool:'Alkool',
               birra:'Birra', 'lengje-frutash':'Lëngje' },
        st:{PENDING:'Duke pritur konfirmimin',CONFIRMED:'U konfirmua',PREPARING:'Po gatuhet',
            READY:'Gati',IN_DELIVERY:'Në rrugë',DELIVERED:'U dorëzua',
            REJECTED:'U refuzua',CANCELLED:'U anulua'} },
  en: { cart:'Cart', add:'Add', total:'Total', chargedIn:'You will be charged', checkout:'Checkout', empty:'Your cart is empty',
        emptyHint:'Pick a dish from the menu', name:'Name', phone:'Phone', address:'Address',
        note:'Note for the courier', pay:'Payment', cash:'Cash', card:'Card',
        cashNote:'Pay the courier on delivery', cardNote:'Card or Apple/Google Pay', place:'Place order',
        subtotal:'Subtotal', delivery:'Delivery', free:'Free', closed:'Closed right now',
        closedHint:'Call to order', soldOut:'Sold out', min:'Minimum order',
        sent:'Order placed', track:'Track your order', offline:'You are offline — call instead',
        required:'Required', optional:'(optional)', badPhone:'Invalid number', ordering:'Sending…',
        checkArea:'Check this address', checking:'Checking…',
        retry:'Try again', loadFail:'The menu did not load', trackFail:'The order did not update', loading:'Loading…',
        hoursTitle:'Opening hours', openUntil:'Open until', opensAt2:'Opens at', closedNow:'Closed now', onMap:'Show on map', directions:'Directions', reviewsTitle:'Reviews', fromGoogle:'from Google Maps', allReviews:'All reviews', callUs:'Call',
        savedAddresses:'Saved addresses', forget:'Remove', myOrders:'My orders', noOrders:'No orders yet', when:'When', asap:'As soon as possible',
        onTable:'See it on your table', arScan:'Point at your table…', arTap:'Tap to place it', arFail:'Could not open',
        opensAt:'Opens', pausedNow:'Delivery is paused',
        search:'Search the menu', sortBy:'Sort', sortPop:'As on the menu', sortLow:'Price: low first',
        sortHigh:'Price: high first', sortAz:'By name', noHits:'Nothing matched', clear:'Clear',
        onlyAvail:'Available only',
        later:'At a later time', schedFail:'That time will not work',
        inArea:'We deliver here', outArea:'Outside our delivery area', noGeo:'Could not get your location',
        notDeclared:'Allergens not declared', noneOf14:'None of the 14 allergens',
        tip:'Tip for the courier', tipNo:'No tip', tipOther:'Other',
        sayHow:'How was it?', sayHint:'Only the venue reads this. No stars, no ratings.',
        sayGo:'Send', saidIt:'Thank you',
        how:'How you get it', toDoor:'Delivery', toPickup:'I will collect', pickupAt:'Collect at',
        avoid:'Allergens', avoidHint:'Hide dishes that contain them', avoidOn:'hidden',
        avoidUnknown:'undeclared dishes', clearAvoid:'Show everything',
        promo:'Promo code', promoApply:'Apply', promoOff:'Remove', discount:'Discount',
        notify:'Get updates on Telegram', notifyHint:'We’ll message you each time this order moves',
        menu:'Menu', tabSearch:'Search', orders:'Orders', info:'Info', prefs:'Language & currency', language:'Language', currency:'Currency',
        eta:'Delivery', etaMin:'min', prep:'Prep', weight:'Weight', kcal:'kcal', protein:'Protein', fat:'Fat', carbs:'Carbs',
        ingredients:'Ingredients', about:'About', filters:'Filters', all:'All', chefPicks:'Chef’s picks',
        viewOrder:'View order', items:'items', pickOnMap:'Pick on the map', useMyLocation:'Use my location',
        confirmPin:'Confirm this spot', dragPin:'Drag the pin to your door', findingAddress:'Finding the address…',
        contact:'Contact', extras:'Extras', summary:'Summary', back:'Back', done:'Done',
        etaRange:'Estimated time', yourOrder:'Your order', notDeclaredShort:'Not declared',
        readIn:'Read prices in', chooseLang:'Choose a language',
        applePay:'Apple Pay', googlePay:'Google Pay', crypto:'Crypto', cardNote2:'Visa, Mastercard',
        walletNote:'Pay after ordering, to the address shown', payWith:'Pay with', network:'Network',
        sendExactly:'Send exactly', toAddress:'to this address', copy:'Copy', copied:'Copied',
        cryptoWait:'The venue confirms the payment as soon as it lands', cryptoRate:'Crypto amount at the day’s rate',
        nutrition:'Nutrition', approx:'approx., per portion', pickup:'Pickup', kitchenNote:'Note for the kitchen',
        langs:{sq:'Shqip', en:'English', uk:'Українська'}, curs:{ALL:'Lekë', EUR:'Euro', USD:'US dollars'},
        addToOrder:'Add to order', payLater:'Pay on delivery', ready:'Ready in', min2:'min',
        street:'Street', house:'House no.', apartment:'Apartment', entrance:'Entrance', floor:'Floor', privateHouse:'Private house',
        distance:'Away', fromVenue:'from the venue', headingOn:'Follow my heading', headingOff:'North up',
        voice:'Order by voice', listening:'Listening…', heard:'I heard', notThis:'Not this', sayLike:'Say: “two Philadelphia”',
        noMatch:'Not on the menu', addedByVoice:'Added', micDenied:'Microphone not allowed', noVoice:'This browser cannot listen',
        qtyWords:{one:1,a:1,two:2,three:3,four:4,five:5,six:6,seven:7,eight:8,nine:9},
        stateLbl:'Status', ratingLbl:'Rating', timeLbl:'Delivery time', feeLbl:'Delivery',
        mapLegend:'The way to you', mapVenue:'venue', mapYou:'you', mapCourier:'courier', taste:'Taste', taste_spicy:'spicy', taste_sweet:'sweet', taste_salty:'salty', taste_sour:'sour', taste_richness:'rich', episode:'Order', stepOf:'step', ofSteps:'of', whatTheySay:'What people say', nextUp:'Up next', preparingTitle:'Being made for you',
        installBody:'The menu opens with one tap, even offline; your order is tracked from the home screen.', installNow:'Install', installLater:'Later', installNever:'Don\'t show again', installApp:'Install as an app', installHint:'In Safari: Share → Add to Home Screen', installed:'Installed',
        tags:{ salmon:'Salmon', tuna:'Tuna', shrimp:'Shrimp', vegetarian:'Vegetarian', hot:'Hot', popular:'Popular',
               sets:'Sets', bowls:'Bowls', soups:'Soups', drinks:'Drinks', freskuese:'Soft drinks', kafeteria:'Coffee', alkool:'Spirits',
               birra:'Beer', 'lengje-frutash':'Juices' },
        st:{PENDING:'Awaiting confirmation',CONFIRMED:'Confirmed',PREPARING:'Being prepared',
            READY:'Ready',IN_DELIVERY:'On the way',DELIVERED:'Delivered',
            REJECTED:'Rejected',CANCELLED:'Cancelled'} },
  uk: { cart:'Кошик', add:'Додати', total:'Разом', chargedIn:'Буде списано', checkout:'Оформити', empty:'Кошик порожній',
        emptyHint:'Оберіть страву з меню', name:'Ім’я', phone:'Телефон', address:'Адреса',
        note:'Коментар кур’єру', pay:'Оплата', cash:'Готівка', card:'Картка',
        cashNote:'Оплата кур’єру при отриманні', cardNote:'Картка або Apple/Google Pay', place:'Замовити',
        subtotal:'Сума', delivery:'Доставка', free:'Безкоштовно', closed:'Зараз зачинено',
        closedHint:'Зателефонуйте, щоб замовити', soldOut:'Немає', min:'Мінімальне замовлення',
        sent:'Замовлення прийнято', track:'Стежити за замовленням', offline:'Немає зв’язку — телефонуйте',
        required:'Обов’язкове поле', optional:'(необов’язково)', badPhone:'Некоректний номер', ordering:'Надсилаємо…',
        checkArea:'Перевірити адресу', checking:'Перевіряємо…',
        retry:'Спробувати ще раз', loadFail:'Меню не завантажилось', trackFail:'Замовлення не оновилось', loading:'Завантажуємо…',
        hoursTitle:'Години роботи', openUntil:'Відчинено до', opensAt2:'Відчиняється о', closedNow:'Зараз зачинено', onMap:'Показати на карті', directions:'Маршрут', reviewsTitle:'Відгуки', fromGoogle:'з Google Maps', allReviews:'Усі відгуки', callUs:'Зателефонувати',
        savedAddresses:'Збережені адреси', forget:'Прибрати', myOrders:'Мої замовлення', noOrders:'Замовлень ще немає', when:'Коли', asap:'Якнайшвидше',
        onTable:'Подивитись на столі', arScan:'Наведіть на стіл…', arTap:'Торкніться, щоб поставити', arFail:'Не вдалося відкрити',
        opensAt:'Відчиняється', pausedNow:'Доставку призупинено',
        search:'Пошук у меню', sortBy:'Сортування', sortLow:'Ціна: від дешевших',
        sortPop:'Як у меню', sortHigh:'Ціна: від дорожчих', sortAz:'За назвою',
        noHits:'Нічого не знайдено', clear:'Очистити', onlyAvail:'Лише в наявності',
        later:'На інший час', schedFail:'Такий час не підходить',
        inArea:'Сюди доставляємо', outArea:'Поза зоною доставки', noGeo:'Не вдалося визначити місце',
        notDeclared:'Алергени не заявлено', noneOf14:'Жодного з 14 алергенів',
        tip:'Чайові кур\'єру', tipNo:'Без чайових', tipOther:'Інша сума',
        sayHow:'Як вам?', sayHint:'Читає лише заклад. Без зірок і оцінок.',
        sayGo:'Надіслати', saidIt:'Дякуємо',
        how:'Як заберете', toDoor:'Доставка', toPickup:'Заберу сам', pickupAt:'Забрати за адресою',
        avoid:'Алергени', avoidHint:'Сховати страви, що їх містять', avoidOn:'сховано',
        avoidUnknown:'страв без заяви', clearAvoid:'Показати все',
        promo:'Промокод', promoApply:'Застосувати', promoOff:'Прибрати', discount:'Знижка',
        notify:'Сповіщення в Telegram', notifyHint:'Напишемо щоразу, коли статус зміниться',
        menu:'Меню', tabSearch:'Пошук', orders:'Замовлення', info:'Про нас', prefs:'Мова та валюта', language:'Мова', currency:'Валюта',
        eta:'Доставка', etaMin:'хв', prep:'Готування', weight:'Вага', kcal:'ккал', protein:'Білки', fat:'Жири', carbs:'Вуглеводи',
        ingredients:'Інгредієнти', about:'Про заклад', filters:'Фільтри', all:'Усе', chefPicks:'Вибір шефа',
        viewOrder:'Переглянути замовлення', items:'позицій', pickOnMap:'Обрати на карті', useMyLocation:'Моє місце',
        confirmPin:'Підтвердити точку', dragPin:'Перетягніть мітку до своїх дверей', findingAddress:'Шукаємо адресу…',
        contact:'Контакт', extras:'Додатково', summary:'Підсумок', back:'Назад', done:'Готово',
        etaRange:'Час очікування', yourOrder:'Ваше замовлення', notDeclaredShort:'Не вказано',
        readIn:'Показувати ціни у', chooseLang:'Оберіть мову',
        applePay:'Apple Pay', googlePay:'Google Pay', crypto:'Криптовалюта', cardNote2:'Visa, Mastercard',
        walletNote:'Оплата після замовлення на вказану адресу', payWith:'Оплатити через', network:'Мережа',
        sendExactly:'Надішліть рівно', toAddress:'на цю адресу', copy:'Копіювати', copied:'Скопійовано',
        cryptoWait:'Заклад підтвердить оплату, щойно вона надійде', cryptoRate:'Сума в криптовалюті за курсом дня',
        nutrition:'Харчова цінність', approx:'приблизно, на порцію', pickup:'Самовивіз', kitchenNote:'Коментар для кухні',
        langs:{sq:'Shqip', en:'English', uk:'Українська'}, curs:{ALL:'Леки', EUR:'Євро', USD:'Долари США'},
        addToOrder:'Додати до замовлення', payLater:'Оплата при отриманні', ready:'Готово за', min2:'хв',
        street:'Вулиця', house:'Будинок', apartment:'Квартира', entrance:'Під’їзд', floor:'Поверх', privateHouse:'Приватний будинок',
        distance:'Відстань', fromVenue:'від закладу', headingOn:'За моїм напрямком', headingOff:'Північ угорі',
        voice:'Замовити голосом', listening:'Слухаю…', heard:'Я почув', notThis:'Не це', sayLike:'Скажіть: «два Philadelphia»',
        noMatch:'Не знайшов у меню', addedByVoice:'Додано', micDenied:'Мікрофон не дозволено', noVoice:'Цей браузер не вміє слухати',
        qtyWords:{'один':1,'одну':1,'одна':1,'два':2,'дві':2,'три':3,'чотири':4,'п’ять':5,"п'ять":5,'пять':5,'шість':6,'сім':7,'вісім':8,'дев’ять':9,"дев'ять":9},
        stateLbl:'Статус', ratingLbl:'Рейтинг', timeLbl:'Час доставки', feeLbl:'Доставка',
        mapLegend:'Шлях замовлення', mapVenue:'заклад', mapYou:'ви', mapCourier:'кур’єр', taste:'Смак', taste_spicy:'гострий', taste_sweet:'солодкий', taste_salty:'солоний', taste_sour:'кислий', taste_richness:'насичений', episode:'Замовлення', stepOf:'крок', ofSteps:'з', whatTheySay:'Що кажуть гості', nextUp:'Далі', preparingTitle:'Готується для вас',
        installBody:'Меню відкривається одним дотиком, навіть без мережі; замовлення відстежується з головного екрана.', installNow:'Встановити', installLater:'Пізніше', installNever:'Більше не показувати', installApp:'Встановити як додаток', installHint:'У Safari: Поділитися → На Початковий екран', installed:'Встановлено',
        tags:{ salmon:'Лосось', tuna:'Тунець', shrimp:'Креветка', vegetarian:'Вегетаріанське', hot:'Гаряче', popular:'Популярне',
               sets:'Сети', bowls:'Боули', soups:'Супи', drinks:'Напої', freskuese:'Безалкогольне', kafeteria:'Кава', alkool:'Алкоголь',
               birra:'Пиво', 'lengje-frutash':'Соки' },
        st:{PENDING:'Очікує підтвердження',CONFIRMED:'Підтверджено',PREPARING:'Готується',
            READY:'Готове',IN_DELIVERY:'У дорозі',DELIVERED:'Доставлено',
            REJECTED:'Відхилено',CANCELLED:'Скасовано'} },
};

export let lang = safeGet('dw_lang') || 'sq';

/// A key, in the current language, falling back to English and then to the key
/// itself -- a missing translation must read as a word, never as `undefined`.
export const t = k => (T[lang] && T[lang][k]) ?? T.en[k] ?? k;

/// A dish tag, in words. The venue files a dish as `salmon` or `hot`; the
/// customer reads "Лосось" or "Гаряче". An unknown tag is shown as itself
/// rather than dropped: the venue wrote it down for a reason.
export const tagName = tag => (T[lang]?.tags?.[tag]) ?? T.en.tags[tag] ?? tag;

export const LANGS = ['sq', 'en', 'uk'];

/// The Intl locale for money and dates.
export const intlLocale = () => lang === 'uk' ? 'uk' : lang === 'en' ? 'en' : 'sq';

/// Switch the language and rewrite every translated node IN PLACE. The caller
/// (app.js) is responsible for the venue's own words -- it re-fetches the menu
/// in the new locale and patches names by id -- and for re-formatting money.
export function setLang(code){
  if (!LANGS.includes(code) || code === lang) return false;
  lang = code;
  safeSet('dw_lang', lang);
  document.documentElement.lang = lang;
  retranslate(document);
  return true;
}

/// Rewrite text under `root`. Two attributes:
///   data-t="key"                  → textContent = t(key)
///   data-t-attr="placeholder:key aria-label:key"  → each attribute set
/// Nothing else moves: no node is created or removed, so scroll, focus and
/// the caret in a search field all survive the switch.
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
  for (const el of root.querySelectorAll('[data-t-tag]')) {
    el.textContent = tagName(el.dataset.tTag);
  }
  for (const el of root.querySelectorAll('[data-t-st]')) {
    el.textContent = t('st')[el.dataset.tSt] || el.dataset.tSt;
  }
}
