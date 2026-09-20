// dowiz.org — the front door's motion and its three languages.
//
// GSAP, ScrollTrigger, SplitText and Lenis are vendored under /lib/vendor and
// loaded as classic scripts before this module (script-src 'self'). This file
// only reads their globals. Every animation has a still frame: with reduced
// motion, or without the libraries, the page is complete and readable.

const gsap = window.gsap;
const ScrollTrigger = window.ScrollTrigger;
const SplitText = window.SplitText;
const Lenis = window.Lenis;
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;
const finePointer = matchMedia('(pointer: fine)').matches;
const DESK = '(min-width: 56rem)';
const motion = !!gsap && !reduced;

// ── the three languages ────────────────────────────────────────────────────
const T = {
  uk: {
    title: 'dowiz — власний додаток закладу, 0% комісії',
    nav: 'Мова та вхід', langs: 'Мова', signin: 'Вхід', manifesto: 'Маніфест',
    h1: 'Ваш заклад.<br>Ваш додаток.<br><em>Нуль</em> комісії.',
    lede: 'Агрегатори забирають 25–35% кожного замовлення. dowiz дає закладу власний додаток на власному домені — меню, оплата, кур\'єр наживо — за одну фіксовану підписку.',
    ctaLive: 'Живий заклад', ctaHow: 'Як це працює',
    filmH: 'Тридцять секунд, і все зрозуміло',
    filmP: 'Один кадр — одна думка: третина чека, що згорає; нуль; ваш додаток; кур\'єр на мапі; замок; ваша каса. Звук — за кнопкою.',
    filmSound: 'Зі звуком',
    filmMute: 'Без звуку',
    advH: 'Усе, що входить. Без зірочок.',
    advP: 'Двадцять речей, за які агрегатор бере відсоток, а dowiz — ні. Кожна з них уже працює в живому закладі.',
    a1t: '0% комісії', a1p: 'Одна фіксована підписка. Жодного відсотка з чека і жодного тарифу за доставку.',
    a2t: 'Власний домен і бренд', a2p: 'Адреса закладу, його логотип, кольори й печатка — не платформи.',
    a3t: 'Встановлюється як додаток', a3p: 'На iPhone і Android без магазинів; меню відкривається навіть без зв\'язку.',
    a4t: 'Три мови й валюта клієнта', a4p: 'Shqip, English, українська — і ціни в тій валюті, якою платить клієнт.',
    a5t: 'Меню, як у шефа', a5p: 'Фото, калорії, алергени, склад і рецептури — не сірі плашки.',
    a6t: 'Кошик і оплата', a6p: 'Карткою або готівкою, промокоди, підтвердження за хвилину.',
    a7t: 'Кур\'єр наживо', a7p: 'Клієнт бачить кур\'єра на мапі; час прибуття рахується з маршруту.',
    a8t: 'Один журнал замовлень', a8p: 'Вітрина, телефон, WhatsApp, Instagram і API партнерів — одна черга для кухні.',
    a9t: 'Консоль власника в телефоні', a9p: 'Замовлення, меню, кур\'єри, склад і статистика — одним великим пальцем.',
    a10t: 'Застосунок кур\'єра', a10p: 'Маршрут, статуси, готівка й підтвердження доставки — читається однією рукою.',
    a11t: 'Сповіщення в Telegram', a11p: 'Кожне замовлення — у ваш бот, за секунди, з усіма рядками й сумою.',
    a12t: 'Автопостинг з вашого схвалення', a12p: 'dowiz пише пости про страви; ви натискаєте «Опублікувати» — Telegram та Instagram.',
    a13t: 'Локальний ШІ-помічник', a13p: 'Відповідає про меню й замовлення на вашому сервері. Нічого не надсилається назовні.',
    a14t: 'Бронювання і чати', a14p: 'Столики з перепусткою, розмова з клієнтом і гаманець — усе в тому ж додатку.',
    a15t: 'Склад і рецептури', a15p: 'Залишки списуються з кожного замовлення; закінчується — страва ховається сама.',
    a16t: 'Постквантове шифрування', a16p: 'X25519 + ML-KEM-768 у каналі, ML-DSA-65 на кожному пакеті, AES-256-GCM локально.',
    a17t: 'Гроші без округлень', a17p: 'Цілі числа, подвійний запис, повернення сходяться в нуль — без плаваючої коми.',
    a18t: 'Нічні копії й API', a18p: 'Резервні копії в S3 щоночі; хаб відкритий як MCP-сервер для ваших інструментів.',
    a19t: 'Ваші дані — у закладі', a19p: 'Кожен вузол тримає власну базу; хмара необов\'язкова, посередника немає.',
    a20t: 'Відкритий код, AGPL-3.0', a20p: 'Протокол відкритий, без рейтингів кур\'єрів і прихованих алгоритмів.',
    ctaJoin: 'Стати в чергу',
    joinEmail: 'Пошта закладу', joinVenue: 'Назва закладу (необов\'язково)', joinGo: 'Стати в чергу', joinSending: 'Надсилаємо…',
    joinOk: 'Ви в списку. Напишемо на {email}.', joinInvalid: 'Перевірте адресу пошти.', joinBad: 'Не вдалося надіслати. Спробуйте ще раз за хвилину.',
    mq1: 'вітрина', mq2: 'телефон', mq3: 'один журнал замовлень',
    productH: 'Ваші кольори, ваше меню, ваші клієнти — і замовлення в кілька дотиків.',
    chip: 'хв · кур\'єр у дорозі', phoneCap: 'Dubin & Sushi, Дуррес — живий заклад на dowiz:',
    s1k: 'власний домен', s1h: 'Додаток закладу, не платформи',
    s1p: 'Адреса на власному домені, ваш логотип і кольори, три мови одразу. Встановлюється на телефон як звичайний додаток — без магазину — і відкриває меню навіть без зв\'язку.',
    s2k: 'меню й кошик', s2h: 'Фото шефа, калорії, алергени',
    s2p: 'Меню, кошик, оплата карткою чи готівкою і підтвердження за хвилину. Кожна страва — з фото, калоріями та алергенами, а не сірою плашкою.',
    s3k: 'наживо', s3h: 'Вони бачать, як воно їде',
    s3p: 'Прийнято, готується, у дорозі, час прибуття — клієнт стежить за кур\'єром на мапі. Менше дзвінків «де моє замовлення», більше повторних замовлень.',
    engineH: 'Рушій, якому не потрібен посередник',
    engineP: 'Під додатком — відкритий протокол доставки: один журнал замовлень, постквантове шифрування, локальний ШІ і жодного сервера, без якого все зупиниться.',
    p1h: 'Один хаб для кожного замовлення',
    p1p: 'Вітрина, телефон, WhatsApp, Instagram, API партнерів — усе приходить в один журнал. Кухня бачить одну чергу, кур\'єр — один маршрут, ви — одну касу. Кожна зміна стану — подія в незмінному журналі; заборонений перехід — помилка, а не тихе «нічого».',
    p2h: 'Час прибуття — з дороги, не з голови',
    p2p: 'Кур\'єр на мапі в реальному часі, ETA рахується з маршруту. Мапа — відкриті тайли без трекерів, тому в чужих логах не лишається ні адреси, ні номера замовлення.',
    p3h: 'Постквантовий захист',
    p3p: 'Канал узгоджується гібридом X25519 + ML-KEM-768, кожен пакет підписано ML-DSA-65. Дані закладу й клієнтів зашифровано локально і в дорозі; класичного «запасного» шляху немає.',
    p4h: 'Локальний ШІ. Нічого не надсилається назовні.',
    p4p: 'Помічник відповідає про меню й замовлення на вашому сервері, пише чернетки постів — ви схвалюєте, він публікує в Telegram та Instagram. Гроші, стани і підписи рахує детермінований код, а не модель.',
    p5h: 'Протокол, а не платформа',
    p5p: 'Відкритий код під AGPL-3.0. Кожен вузол — заклад, кур\'єр, клієнт — тримає власну базу; хмара необов\'язкова. Немає рейтингів кур\'єрів і прихованих алгоритмів.',
    statement: 'Немає нікого між тим, хто замовляє, і тим, хто везе. Немає відсотка з кожного чека. Немає сервера, без якого все зупиниться.',
    termsH: 'Одна фіксована підписка. Сто відсотків кожного замовлення — ваші.',
    t1h: 'агрегатор',
    t1p: 'З кожного замовлення. Щодня. Плюс ваш клієнт, ваша статистика і ваш бренд — у чужому додатку поруч із конкурентом.',
    t2p: 'Жодного відсотка з чека, жодного тарифу за доставку. Ціна не залежить від того, скільки ви продали, — тому на цій сторінці немає суми, а є розмова.',
    t2f1: 'Власний домен і додаток без магазину', t2f2: 'Три мови, оплата, кур\'єр наживо', t2f3: 'Дані лишаються у закладі',
    closeH: 'Запускаймо ваш додаток.',
    closeP: 'Лишіть пошту закладу — напишемо, коли відкриємо наступний хаб. Без розсилок: один лист, від людини.',
    ctaLive2: 'Живий заклад', ctaSignin: 'Вхід для адміністраторів',
    fLive: 'Живий заклад', fSignin: 'Вхід', fSecurity: 'Безпека',
    fNote: '© 2026 dowiz · відкритий код, AGPL-3.0 · децентралізований, локальний, постквантовий протокол доставки',
  },
  en: {
    title: 'dowiz — your venue\'s own app, 0% commission',
    nav: 'Language and sign in', langs: 'Language', signin: 'Sign in', manifesto: 'Manifesto',
    h1: 'Your venue.<br>Your app.<br><em>Zero</em> commission.',
    lede: 'Aggregators take 25–35% of every order. dowiz gives a venue its own app on its own domain — menu, payment, courier live on the map — for one flat subscription.',
    ctaLive: 'See a live venue', ctaHow: 'How it works',
    filmH: 'Thirty seconds, and it all makes sense',
    filmP: 'One shot, one thought: a third of the bill burning; zero; your app; the courier on the map; a lock; your till. Sound is behind the button.',
    filmSound: 'With sound',
    filmMute: 'Mute',
    advH: 'Everything included. No asterisks.',
    advP: 'Twenty things an aggregator charges a percentage for and dowiz does not. Every one of them already runs in a live venue.',
    a1t: '0% commission', a1p: 'One flat subscription. No percentage of the bill and no delivery tariff.',
    a2t: 'Own domain and brand', a2p: 'The venue\'s address, logo, colours and seal — not the platform\'s.',
    a3t: 'Installs as an app', a3p: 'On iPhone and Android without app stores; the menu opens even offline.',
    a4t: 'Three languages, the customer\'s currency', a4p: 'Shqip, English, Ukrainian — and prices in the currency the customer pays in.',
    a5t: 'A menu like the chef\'s', a5p: 'Photos, calories, allergens, ingredients and recipes — never grey placeholders.',
    a6t: 'Cart and payment', a6p: 'Card or cash, promo codes, a confirmation within a minute.',
    a7t: 'Courier live', a7p: 'The customer watches the courier on the map; arrival time is computed from the route.',
    a8t: 'One order log', a8p: 'Storefront, phone, WhatsApp, Instagram and partner APIs — one queue for the kitchen.',
    a9t: 'Owner console on a phone', a9p: 'Orders, menu, couriers, stock and numbers — under one thumb.',
    a10t: 'Courier app', a10p: 'Route, statuses, cash and proof of delivery — readable with one hand.',
    a11t: 'Telegram alerts', a11p: 'Every order lands in your bot within seconds, with every line and the total.',
    a12t: 'Autoposting you approve', a12p: 'dowiz drafts posts about dishes; you press Publish — Telegram and Instagram.',
    a13t: 'Local AI assistant', a13p: 'Answers about menu and orders on your own server. Nothing is sent away.',
    a14t: 'Bookings and chats', a14p: 'Tables with a pass, a conversation with the customer and a wallet — in the same app.',
    a15t: 'Stock and recipes', a15p: 'Stock is deducted with every order; when it runs out, the dish hides itself.',
    a16t: 'Post-quantum encryption', a16p: 'X25519 + ML-KEM-768 on the channel, ML-DSA-65 on every packet, AES-256-GCM at rest.',
    a17t: 'Money without rounding', a17p: 'Integers, double entry, refunds that net to zero — no floating point.',
    a18t: 'Nightly copies and an API', a18p: 'Backups to S3 every night; the hub is open as an MCP server for your tools.',
    a19t: 'Your data stays in the venue', a19p: 'Every node keeps its own database; the cloud is optional, there is no middleman.',
    a20t: 'Open source, AGPL-3.0', a20p: 'An open protocol with no courier ratings and no hidden algorithms.',
    ctaJoin: 'Join the list',
    joinEmail: 'Venue email', joinVenue: 'Venue name (optional)', joinGo: 'Join the list', joinSending: 'Sending…',
    joinOk: 'You are on the list. We will write to {email}.', joinInvalid: 'Check the email address.', joinBad: 'Could not send. Try again in a minute.',
    mq1: 'storefront', mq2: 'phone', mq3: 'one order log',
    productH: 'Your colours, your menu, your customers — and an order in a few taps.',
    chip: 'min · courier on the way', phoneCap: 'Dubin & Sushi, Durrës — a live venue on dowiz:',
    s1k: 'own domain', s1h: 'The venue\'s app, not the platform\'s',
    s1p: 'An address on your own domain, your logo and colours, three languages at once. It installs on the phone like any app — no store — and opens the menu even offline.',
    s2k: 'menu and cart', s2h: 'The chef\'s photos, calories, allergens',
    s2p: 'Menu, cart, card or cash, and a confirmation within a minute. Every dish comes with a photo, calories and allergens — never a grey placeholder.',
    s3k: 'live', s3h: 'They watch it come',
    s3p: 'Accepted, being made, on the way, arrival time — the customer follows the courier on the map. Fewer "where is my order" calls, more repeat orders.',
    engineH: 'An engine that needs no middleman',
    engineP: 'Under the app is an open delivery protocol: one order log, post-quantum encryption, local AI, and no server that everything stops without.',
    p1h: 'One hub for every order',
    p1p: 'Storefront, phone, WhatsApp, Instagram, partner APIs — everything lands in one log. The kitchen sees one queue, the courier one route, you one till. Every state change is an event in an immutable log; a forbidden transition is an error, not a silent nothing.',
    p2h: 'Arrival time from the road, not a guess',
    p2p: 'The courier on the map in real time, ETA computed from the route. The map uses open tiles with no trackers, so no address or order number ends up in someone else\'s logs.',
    p3h: 'Post-quantum security',
    p3p: 'Channels negotiate a hybrid X25519 + ML-KEM-768, every packet is signed with ML-DSA-65. Venue and customer data are encrypted at rest and in transit; there is no classical-only fallback.',
    p4h: 'Local AI. Nothing is sent away.',
    p4p: 'The assistant answers about menu and orders on your own server and drafts posts — you approve, it publishes to Telegram and Instagram. Money, states and signatures are computed by deterministic code, not a model.',
    p5h: 'A protocol, not a platform',
    p5p: 'Open source under AGPL-3.0. Every node — venue, courier, customer — keeps its own database; the cloud is optional. No courier ratings, no hidden algorithms.',
    statement: 'Nobody between the one who orders and the one who delivers. No percentage of any bill. No server that everything stops without.',
    termsH: 'One flat subscription. One hundred percent of every order is yours.',
    t1h: 'aggregator',
    t1p: 'Of every order. Every day. Plus your customer, your numbers and your brand — inside someone else\'s app, next to a competitor.',
    t2p: 'No percentage of the bill, no delivery tariff. The price does not depend on how much you sell — which is why this page has no amount, and a conversation instead.',
    t2f1: 'Own domain and an app without a store', t2f2: 'Three languages, payments, courier live', t2f3: 'Data stays in the venue',
    closeH: 'Let\'s launch your app.',
    closeP: 'Leave the venue\'s email and we will write when the next hub opens. No newsletters: one letter, from a person.',
    ctaLive2: 'See a live venue', ctaSignin: 'Administrator sign in',
    fLive: 'Live venue', fSignin: 'Sign in', fSecurity: 'Security',
    fNote: '© 2026 dowiz · open source, AGPL-3.0 · a decentralised, local-first, post-quantum delivery protocol',
  },
  sq: {
    title: 'dowiz — aplikacioni i lokalit tuaj, 0% komision',
    nav: 'Gjuha dhe hyrja', langs: 'Gjuha', signin: 'Hyr', manifesto: 'Manifesti',
    h1: 'Lokali juaj.<br>Aplikacioni juaj.<br><em>Zero</em> komision.',
    lede: 'Agregatorët marrin 25–35% të çdo porosie. dowiz i jep lokalit aplikacionin e vet në domenin e vet — menyja, pagesa, korrieri në hartë — për një abonim fiks.',
    ctaLive: 'Shih një lokal të gjallë', ctaHow: 'Si funksionon',
    filmH: 'Tridhjetë sekonda, dhe gjithçka kuptohet',
    filmP: 'Një kuadër, një mendim: një e treta e faturës që digjet; zero; aplikacioni juaj; korrieri në hartë; një dry; arka juaj. Zëri është pas butonit.',
    filmSound: 'Me zë',
    filmMute: 'Pa zë',
    advH: 'Gjithçka e përfshirë. Pa yjeza.',
    advP: 'Njëzet gjëra për të cilat agregatori merr përqindje dhe dowiz jo. Secila prej tyre punon tashmë në një lokal të gjallë.',
    a1t: '0% komision', a1p: 'Një abonim fiks. Asnjë përqindje nga fatura dhe asnjë tarifë dërgese.',
    a2t: 'Domeni dhe marka juaj', a2p: 'Adresa e lokalit, logoja, ngjyrat dhe vula — jo të platformës.',
    a3t: 'Instalohet si aplikacion', a3p: 'Në iPhone dhe Android pa dyqane; menyja hapet edhe pa lidhje.',
    a4t: 'Tri gjuhë, monedha e klientit', a4p: 'Shqip, English, ukrainisht — dhe çmimet në monedhën me të cilën paguan klienti.',
    a5t: 'Menyja si e shefit', a5p: 'Foto, kalori, alergjenë, përbërës dhe receta — kurrë kuti gri.',
    a6t: 'Shporta dhe pagesa', a6p: 'Me kartë ose cash, kode promocionale, konfirmim brenda një minute.',
    a7t: 'Korrieri drejtpërdrejt', a7p: 'Klienti sheh korrierin në hartë; koha e mbërritjes llogaritet nga itinerari.',
    a8t: 'Një regjistër porosish', a8p: 'Vitrina, telefoni, WhatsApp, Instagram dhe API e partnerëve — një radhë për kuzhinën.',
    a9t: 'Konsola e pronarit në telefon', a9p: 'Porositë, menyja, korrierët, stoku dhe shifrat — nën një gisht.',
    a10t: 'Aplikacioni i korrierit', a10p: 'Rruga, statuset, cash-i dhe prova e dorëzimit — lexohet me një dorë.',
    a11t: 'Njoftime në Telegram', a11p: 'Çdo porosi mbërrin në botin tuaj brenda sekondash, me çdo rresht dhe totalin.',
    a12t: 'Autopostim që e miratoni ju', a12p: 'dowiz shkruan postime për pjatat; ju shtypni Publiko — Telegram dhe Instagram.',
    a13t: 'Asistent AI lokal', a13p: 'Përgjigjet për menynë dhe porositë në serverin tuaj. Asgjë nuk dërgohet jashtë.',
    a14t: 'Rezervime dhe biseda', a14p: 'Tavolina me leje, bisedë me klientin dhe portofol — në të njëjtin aplikacion.',
    a15t: 'Stoku dhe recetat', a15p: 'Stoku zbritet me çdo porosi; kur mbaron, pjata fshihet vetë.',
    a16t: 'Kriptim post-kuantik', a16p: 'X25519 + ML-KEM-768 në kanal, ML-DSA-65 në çdo paketë, AES-256-GCM lokalisht.',
    a17t: 'Para pa rrumbullakim', a17p: 'Numra të plotë, regjistrim i dyfishtë, rimbursime që dalin zero — pa presje dhjetore.',
    a18t: 'Kopje çdo natë dhe API', a18p: 'Kopje rezervë në S3 çdo natë; qendra është e hapur si server MCP për mjetet tuaja.',
    a19t: 'Të dhënat mbeten në lokal', a19p: 'Çdo nyje mban bazën e vet; reja është opsionale, ndërmjetës nuk ka.',
    a20t: 'Kod i hapur, AGPL-3.0', a20p: 'Protokoll i hapur, pa vlerësime korrierësh dhe pa algoritme të fshehura.',
    ctaJoin: 'Hyr në listë',
    joinEmail: 'Emaili i lokalit', joinVenue: 'Emri i lokalit (opsional)', joinGo: 'Hyr në listë', joinSending: 'Po dërgohet…',
    joinOk: 'Jeni në listë. Do t\'ju shkruajmë në {email}.', joinInvalid: 'Kontrolloni adresën e emailit.', joinBad: 'Nuk u dërgua. Provoni sërish pas një minute.',
    mq1: 'vitrina', mq2: 'telefoni', mq3: 'një regjistër porosish',
    productH: 'Ngjyrat tuaja, menyja juaj, klientët tuaj — dhe porosi me pak prekje.',
    chip: 'min · korrieri po vjen', phoneCap: 'Dubin & Sushi, Durrës — lokal i gjallë në dowiz:',
    s1k: 'domeni juaj', s1h: 'Aplikacioni i lokalit, jo i platformës',
    s1p: 'Adresë në domenin tuaj, logoja dhe ngjyrat tuaja, tri gjuhë njëherësh. Instalohet në telefon si çdo aplikacion — pa dyqan — dhe hap menynë edhe pa lidhje.',
    s2k: 'menyja dhe shporta', s2h: 'Fotot e shefit, kaloritë, alergjenët',
    s2p: 'Menyja, shporta, pagesa me kartë ose cash dhe konfirmimi brenda një minute. Çdo pjatë me foto, kalori dhe alergjenë — kurrë një kuti gri.',
    s3k: 'drejtpërdrejt', s3h: 'E shohin duke ardhur',
    s3p: 'Pranuar, në përgatitje, rrugës, koha e mbërritjes — klienti ndjek korrierin në hartë. Më pak telefonata «ku është porosia», më shumë porosi të përsëritura.',
    engineH: 'Një motor që s\'ka nevojë për ndërmjetës',
    engineP: 'Nën aplikacion është një protokoll i hapur dërgese: një regjistër porosish, kriptim post-kuantik, AI lokal dhe asnjë server pa të cilin gjithçka ndalon.',
    p1h: 'Një qendër për çdo porosi',
    p1p: 'Vitrina, telefoni, WhatsApp, Instagram, API e partnerëve — gjithçka mbërrin në një regjistër. Kuzhina sheh një radhë, korrieri një rrugë, ju një arkë. Çdo ndryshim gjendjeje është një ngjarje në një regjistër të pandryshueshëm; kalimi i ndaluar është gabim, jo heshtje.',
    p2h: 'Koha e mbërritjes nga rruga, jo me hamendje',
    p2p: 'Korrieri në hartë në kohë reale, ETA llogaritet nga itinerari. Harta përdor pllaka të hapura pa gjurmues, kështu asnjë adresë a numër porosie nuk përfundon në regjistrat e të tjerëve.',
    p3h: 'Siguri post-kuantike',
    p3p: 'Kanali negociohet me hibridin X25519 + ML-KEM-768, çdo paketë nënshkruhet me ML-DSA-65. Të dhënat e lokalit dhe të klientëve kriptohen lokalisht dhe gjatë rrugës; nuk ka rrugë rezervë klasike.',
    p4h: 'AI lokal. Asgjë nuk dërgohet jashtë.',
    p4p: 'Asistenti përgjigjet për menynë dhe porositë në serverin tuaj dhe shkruan drafte postimesh — ju miratoni, ai boton në Telegram dhe Instagram. Paratë, gjendjet dhe nënshkrimet i llogarit kod determinist, jo një model.',
    p5h: 'Protokoll, jo platformë',
    p5p: 'Kod i hapur nën AGPL-3.0. Çdo nyje — lokali, korrieri, klienti — mban bazën e vet; reja është opsionale. Pa vlerësime korrierësh, pa algoritme të fshehura.',
    statement: 'Askush mes atij që porosit dhe atij që sjell. Asnjë përqindje nga asnjë faturë. Asnjë server pa të cilin gjithçka ndalon.',
    termsH: 'Një abonim fiks. Njëqind përqind e çdo porosie është e juaja.',
    t1h: 'agregatori',
    t1p: 'Nga çdo porosi. Çdo ditë. Plus klienti juaj, shifrat tuaja dhe marka juaj — në aplikacionin e dikujt tjetër, pranë konkurrentit.',
    t2p: 'Asnjë përqindje nga fatura, asnjë tarifë dërgese. Çmimi nuk varet nga sa shisni — prandaj kjo faqe nuk ka shumë, por një bisedë.',
    t2f1: 'Domeni juaj dhe aplikacion pa dyqan', t2f2: 'Tri gjuhë, pagesa, korrieri drejtpërdrejt', t2f3: 'Të dhënat mbeten në lokal',
    closeH: 'Ta nisim aplikacionin tuaj.',
    closeP: 'Lini emailin e lokalit dhe do t\'ju shkruajmë kur të hapet qendra tjetër. Pa buletine: një letër, nga një njeri.',
    ctaLive2: 'Shih një lokal të gjallë', ctaSignin: 'Hyrja e administratorit',
    fLive: 'Lokal i gjallë', fSignin: 'Hyr', fSecurity: 'Siguria',
    fNote: '© 2026 dowiz · kod i hapur, AGPL-3.0 · protokoll dërgese i decentralizuar, lokal, post-kuantik',
  },
};

// SplitText instances and the statement's word spans are rebuilt on a language
// change, so the registry is the only place that knows about them.
const splits = [];
let statementTween = null;

function applyLang(lang) {
  const t = T[lang] || T.uk;
  document.documentElement.lang = lang;
  document.title = t.title;
  $$('[data-i18n]').forEach(el => { const k = el.dataset.i18n; if (t[k] != null) el.textContent = t[k]; });
  $$('[data-i18n-html]').forEach(el => { const k = el.dataset.i18nHtml; if (t[k] != null) el.innerHTML = t[k]; });
  $$('[data-i18n-attr]').forEach(el => {
    const [attr, k] = el.dataset.i18nAttr.split(':');
    if (t[k] != null) el.setAttribute(attr, t[k]);
  });
  $$('.lang').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.lang === lang)));
  try { localStorage.setItem('dowiz-lang', lang); } catch { /* private window */ }
  document.dispatchEvent(new CustomEvent('dowiz:lang', { detail: lang }));
}

function pickLang() {
  try { const s = localStorage.getItem('dowiz-lang'); if (T[s]) return s; } catch { /* private window */ }
  const nav = (navigator.language || 'uk').slice(0, 2).toLowerCase();
  return T[nav] ? nav : 'uk';
}

// ── the statement: one span per word so scroll can read it ────────────────
function buildStatement() {
  const p = $('#statementP');
  if (!p) return [];
  const text = p.textContent.trim();
  p.textContent = '';
  const words = text.split(/\s+/);
  const spans = words.map((w, i) => {
    const s = document.createElement('span');
    s.className = 'word';
    s.textContent = w;
    p.appendChild(s);
    if (i < words.length - 1) p.appendChild(document.createTextNode(' '));
    return s;
  });
  return spans;
}

// ── still frame for reduced motion or a missing library ───────────────────
function stillFrame() {
  const loader = $('#loader');
  if (loader) loader.classList.add('is-hidden');
  $$('.step').forEach(s => s.classList.add('active'));
  $$('.panel').forEach(p => p.classList.add('is-on'));
  $$('.statement .word').forEach(w => { w.style.opacity = '1'; });
  const b = $('#screenB'); if (b) b.style.opacity = '0';
  setupFilm();
}

// ── motion ─────────────────────────────────────────────────────────────────
function revealLines(el, opts = {}) {
  const inst = SplitText.create(el, {
    type: 'lines', mask: 'lines', linesClass: 'line', autoSplit: true,
    onSplit(self) {
      return gsap.from(self.lines, {
        yPercent: 110, duration: 1.1, stagger: 0.085, ease: 'expo.out',
        ...(opts.scroll ? { scrollTrigger: { trigger: el, start: 'top 85%', once: true } } : {}),
        delay: opts.delay || 0,
      });
    },
  });
  splits.push(inst);
  return inst;
}

function setupStatement() {
  if (statementTween) { statementTween.scrollTrigger && statementTween.scrollTrigger.kill(); statementTween.kill(); }
  const words = buildStatement();
  statementTween = gsap.to(words, {
    opacity: 1, stagger: 0.06, ease: 'none',
    scrollTrigger: { trigger: '#statement', start: 'top 70%', end: 'bottom 55%', scrub: true },
  });
}

function setupScrollScenes() {
  const mm = gsap.matchMedia();

  // hero: the zero drifts as you leave it
  gsap.to('#zero', {
    yPercent: 45, rotate: -6, ease: 'none',
    scrollTrigger: { trigger: '#hero', start: 'top top', end: 'bottom top', scrub: true },
  });

  // the bar hides on the way down and returns on the way up
  const top = $('#top');
  ScrollTrigger.create({
    start: 0, end: 'max',
    onUpdate: self => top.classList.toggle('is-hidden', self.direction === 1 && self.scroll() > 140),
  });

  // headings rise into view, once
  $$('[data-reveal]:not([data-reveal="hero"])').forEach(el => revealLines(el, { scroll: true }));

  // product: the phone answers the step in view
  const screenA = $('#screenA'), screenB = $('#screenB'), chip = $('#chip');
  const show = which => {
    gsap.to(screenB, { opacity: which === 'a' ? 0 : 1, duration: 0.7, ease: 'power2.inOut' });
    gsap.to(chip, { y: which === 'c' ? 0 : '120%', opacity: which === 'c' ? 1 : 0, duration: 0.6, ease: 'expo.out' });
  };
  $$('.step').forEach(step => {
    ScrollTrigger.create({
      trigger: step, start: 'top 62%', end: 'bottom 38%',
      onToggle: self => {
        if (!self.isActive) return;
        $$('.step').forEach(s => s.classList.toggle('active', s === step));
        show(step.dataset.screen);
      },
    });
  });

  // engine: one horizontal run on a wide screen, a plain stack on a narrow one
  mm.add(DESK, () => {
    const track = $('#track');
    const dist = () => Math.max(0, track.scrollWidth - window.innerWidth);
    const run = gsap.to(track, {
      x: () => -dist(), ease: 'none',
      scrollTrigger: {
        trigger: '#engine', start: 'top top', end: () => '+=' + dist(), pin: true, scrub: 0.8,
        anticipatePin: 1, invalidateOnRefresh: true,
      },
    });
    $$('.panel').forEach(panel => ScrollTrigger.create({
      trigger: panel, containerAnimation: run, start: 'left 85%', once: true,
      onEnter: () => panel.classList.add('is-on'),
    }));
    return () => { $$('.panel').forEach(p => p.classList.remove('is-on')); };
  });
  mm.add('(max-width: 55.99rem)', () => {
    $$('.panel').forEach(panel => ScrollTrigger.create({
      trigger: panel, start: 'top 80%', once: true, onEnter: () => panel.classList.add('is-on'),
    }));
  });

  // the twenty rows arrive as a list: one stagger, capped, once
  const rows = $$('.adv-item');
  if (rows.length) {
    gsap.from(rows, {
      y: 28, opacity: 0, duration: 0.9, ease: 'expo.out', stagger: { each: 0.05, from: 'start' },
      scrollTrigger: { trigger: '#advList', start: 'top 82%', once: true },
    });
  }

  // the film: plays muted while on screen, sound behind its button
  setupFilm();

  // the statement reads itself
  setupStatement();

  // the numbers count when they arrive
  $$('[data-count]').forEach(el => {
    const to = Number(el.dataset.count), pre = el.dataset.prefix || '', suf = el.dataset.suffix || '';
    const from = to === 0 ? 35 : 0;
    const o = { v: from };
    ScrollTrigger.create({
      trigger: el, start: 'top 85%', once: true,
      onEnter: () => gsap.to(o, {
        v: to, duration: 1.6, ease: 'power3.out',
        onUpdate: () => { const n = Math.round(o.v); el.textContent = (n === 0 ? '' : pre || '−') + n + suf; },
        onComplete: () => { el.textContent = (to === 0 ? '' : pre) + to + suf; },
      }),
    });
  });

  // the wordmark rises out of the ground
  gsap.fromTo('#wordmark', { yPercent: 24 }, {
    yPercent: 0, ease: 'none',
    scrollTrigger: { trigger: '#wordmark', start: 'top bottom', end: 'bottom bottom', scrub: true },
  });

  // the marquee stops when nobody can see it
  const marq = $('#marq');
  if (marq && 'IntersectionObserver' in window) {
    new IntersectionObserver(([e]) => marq.classList.toggle('still', !e.isIntersecting)).observe(marq);
  }
}

function setupPointer() {
  if (!finePointer) return;
  const cursor = $('#cursor');
  const xTo = gsap.quickTo(cursor, 'x', { duration: 0.22, ease: 'power3' });
  const yTo = gsap.quickTo(cursor, 'y', { duration: 0.22, ease: 'power3' });
  window.addEventListener('pointermove', e => { cursor.classList.add('is-on'); xTo(e.clientX); yTo(e.clientY); }, { passive: true });
  document.addEventListener('pointerover', e => { if (e.target.closest('a,button')) cursor.classList.add('hover'); });
  document.addEventListener('pointerout', e => { if (e.target.closest('a,button')) cursor.classList.remove('hover'); });

  $$('[data-magnet]').forEach(btn => {
    const label = btn.querySelector('span');
    btn.addEventListener('pointermove', e => {
      const r = btn.getBoundingClientRect();
      const dx = (e.clientX - (r.left + r.width / 2)) / r.width;
      const dy = (e.clientY - (r.top + r.height / 2)) / r.height;
      gsap.to(btn, { x: dx * 18, y: dy * 14, duration: 0.4, ease: 'power3.out' });
      if (label) gsap.to(label, { x: dx * 6, y: dy * 4, duration: 0.4, ease: 'power3.out' });
    });
    btn.addEventListener('pointerleave', () => {
      gsap.to(btn, { x: 0, y: 0, duration: 0.7, ease: 'elastic.out(1, 0.4)' });
      if (label) gsap.to(label, { x: 0, y: 0, duration: 0.7, ease: 'elastic.out(1, 0.4)' });
    });
  });
}

function setupSmoothScroll() {
  if (!Lenis) return null;
  const lenis = new Lenis({ lerp: 0.09, smoothWheel: true });
  lenis.on('scroll', ScrollTrigger.update);
  gsap.ticker.add(t => lenis.raf(t * 1000));
  gsap.ticker.lagSmoothing(0);
  $$('a[href^="#"]').forEach(a => a.addEventListener('click', e => {
    const target = $(a.getAttribute('href'));
    if (!target) return;
    e.preventDefault();
    lenis.scrollTo(target, { offset: -24, duration: 1.4 });
  }));
  return lenis;
}

// The preloader counts the fee down to nothing, then the curtain lifts and the
// hero rises. On a second visit in the same session it is skipped: the point
// has been made.
function intro(lenis) {
  const loader = $('#loader');
  const n = $('#loaderN');
  let seen = false;
  try { seen = sessionStorage.getItem('dowiz-intro') === '1'; sessionStorage.setItem('dowiz-intro', '1'); } catch { /* private window */ }

  const hero = () => {
    revealLines('#h1', { delay: 0.05 });
    gsap.from('#zero', { scale: 0.6, rotate: -10, opacity: 0, duration: 1.4, ease: 'expo.out', delay: 0.25 });
    gsap.from('.hero .lede, .hero .cta', { y: 24, opacity: 0, duration: 1, stagger: 0.12, ease: 'expo.out', delay: 0.45 });
  };

  if (seen) { loader.classList.add('is-hidden'); hero(); return; }

  if (lenis) lenis.stop();
  const o = { v: 35 };
  const tl = gsap.timeline({
    onComplete: () => { loader.classList.add('is-hidden'); if (lenis) lenis.start(); ScrollTrigger.refresh(); },
  });
  tl.to(o, { v: 0, duration: 1.3, ease: 'power2.inOut', onUpdate: () => { const v = Math.round(o.v); n.textContent = v === 0 ? '0%' : '−' + v + '%'; } })
    .call(() => n.classList.add('hot'))
    .to(n, { scale: 1.06, duration: 0.25, ease: 'power2.out' })
    .to(loader, { yPercent: -100, duration: 0.9, ease: 'power4.inOut' }, '+=0.2')
    .call(hero, null, '<0.35');
}

function setupLangButtons() {
  $$('.lang').forEach(b => b.addEventListener('click', () => {
    const lang = b.dataset.lang;
    if (motion) {
      splits.forEach(s => s.revert());
      splits.length = 0;
    }
    applyLang(lang);
    if (motion) {
      revealLines('#h1');
      $$('[data-reveal]:not([data-reveal="hero"])').forEach(el => revealLines(el, { scroll: true }));
      setupStatement();
      ScrollTrigger.refresh();
    }
  }));
}

// ── the film ───────────────────────────────────────────────────────────────
// The source follows the language (one file per language), the poster shows
// before anything loads, autoplay is muted and only while the phone is on
// screen; the button turns the sound on and restarts from the top.
// The poster follows it too: the frame carries a burned-in subtitle, so a
// Ukrainian page must not open on an English still.
function filmLang() { const lang = document.documentElement.lang; return T[lang] ? lang : 'uk'; }
function filmSrc(v) { return v.dataset.src.replace('{lang}', filmLang()); }
function filmPoster(v) { return v.dataset.poster.replace('{lang}', filmLang()); }
function setupFilm() {
  const v = $('#promo'), btn = $('#filmSound');
  if (!v) return;
  const load = () => { const src = filmSrc(v); if (v.getAttribute('src') !== src) { v.setAttribute('src', src); v.load(); } };
  const repost = () => { const poster = filmPoster(v); if (v.getAttribute('poster') !== poster) v.setAttribute('poster', poster); };
  repost();
  let onScreen = false;
  if ('IntersectionObserver' in window) {
    new IntersectionObserver(([e]) => {
      onScreen = e.isIntersecting;
      if (onScreen) { load(); v.play().catch(() => {}); } else { v.pause(); }
    }, { threshold: 0.35 }).observe(v);
  } else { load(); }
  if (btn) btn.addEventListener('click', () => {
    const on = v.muted;
    v.muted = !on;
    btn.setAttribute('aria-pressed', String(on));
    const t = T[document.documentElement.lang] || T.uk; btn.querySelector('span').textContent = on ? t.filmMute : t.filmSound;
    if (on) { v.currentTime = 0; v.play().catch(() => {}); }
  });
  document.addEventListener('dowiz:lang', () => { repost(); if (onScreen) { load(); v.play().catch(() => {}); } else { v.removeAttribute('src'); } });
}

// ── the waiting list ───────────────────────────────────────────────────────
// One POST, three answers. The row is written server-side before any mail
// goes out, so a mail failure is never the visitor's problem.
function setupJoin() {
  const form = $('#join');
  if (!form) return;
  const email = $('#joinEmail'), venue = $('#joinVenue'), go = $('#joinGo'), said = $('#joinSaid');
  const t = () => T[document.documentElement.lang] || T.uk;
  const tell = (msg, kind) => { said.textContent = msg; said.className = 'join-said ' + (kind || ''); };
  form.addEventListener('submit', async e => {
    e.preventDefault();
    const addr = email.value.trim().toLowerCase();
    if (!/^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(addr)) { tell(t().joinInvalid, 'bad'); email.focus(); return; }
    go.disabled = true;
    const label = go.querySelector('span'); const was = label.textContent; label.textContent = t().joinSending;
    try {
      const r = await fetch('/api/waitlist', {
        method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email: addr, venue: venue.value.trim(), lang: document.documentElement.lang }),
      });
      if (!r.ok) throw new Error('HTTP ' + r.status);
      tell(t().joinOk.replace('{email}', addr), 'ok');
      form.classList.add('is-sent');
    } catch {
      tell(t().joinBad, 'bad');
      go.disabled = false;
    } finally {
      label.textContent = was;
    }
  });
}

// ── boot ───────────────────────────────────────────────────────────────────
applyLang(pickLang());
setupLangButtons();
setupJoin();

if (!motion) {
  buildStatement();
  stillFrame();
} else {
  gsap.registerPlugin(ScrollTrigger, SplitText);
  document.fonts.ready.then(() => {
    const lenis = setupSmoothScroll();
    setupScrollScenes();
    setupPointer();
    intro(lenis);
  });
}
