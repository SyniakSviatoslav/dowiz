//! EVERY WORD THE BOARD DRAWS, in sq / en / uk / ru (lib/langs.js order).
//!
//! The langs gate on the HTML surfaces checks a JS table for missing keys after the fact. Here a
//! word is a `match` arm that must name all four languages positionally, so a missing language is
//! a COMPILE error and a new word without an arm is a non-exhaustive `match` -- the compiler is the
//! gate (research §5.3 "langs"). The words are the room's and the kitchen board's own
//! (room/i18n.js, admin/kitchen-i18n.js); order-status words are the console's and a test holds
//! them to admin/i18n.js + admin/i18n-ru.js (`board/tests.rs`), so this is a CHECKED copy.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Sq,
    En,
    Uk,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::Sq, Lang::En, Lang::Uk, Lang::Ru];

    pub fn from_code(c: &[u8]) -> Lang {
        match c {
            b"en" => Lang::En,
            b"uk" => Lang::Uk,
            b"ru" => Lang::Ru,
            _ => Lang::Sq,
        }
    }
    /// lib/langs.js `pickLang([stored], navigator.languages, 'sq')` over `lines`: the first line
    /// is the stored choice and counts only as an exact code; each later line counts by its first
    /// two letters, case-folded ("en-GB" is en).
    pub fn pick(lines: &[u8]) -> Lang {
        let known = |c: &[u8]| Lang::ALL.into_iter().find(|l| l.code().as_bytes() == c);
        for (i, l) in lines.split(|&c| c == b'\n').enumerate() {
            let c = match (i, l) {
                (0, _) => known(l),
                (_, [a, b, ..]) => known(&[a.to_ascii_lowercase(), b.to_ascii_lowercase()]),
                _ => None,
            };
            if let Some(c) = c {
                return c;
            }
        }
        Lang::Sq
    }
    pub const fn code(self) -> &'static str {
        match self {
            Lang::Sq => "sq",
            Lang::En => "en",
            Lang::Uk => "uk",
            Lang::Ru => "ru",
        }
    }
    /// The button's face (the room's `langBtn` shows the code in capitals).
    pub const fn badge(self) -> &'static str {
        match self {
            Lang::Sq => "SQ",
            Lang::En => "EN",
            Lang::Uk => "UK",
            Lang::Ru => "RU",
        }
    }
    /// The room's `nextLang`: sq -> en -> uk -> ru -> sq.
    pub const fn next(self) -> Lang {
        match self {
            Lang::Sq => Lang::En,
            Lang::En => Lang::Uk,
            Lang::Uk => Lang::Ru,
            Lang::Ru => Lang::Sq,
        }
    }
    pub fn index(self) -> u32 {
        self as u32
    }
    pub fn s(self, k: Str) -> &'static str {
        part(words(k), self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Str {
    Live, Offline, Polling, LoginLine, Email, Password, ClaimCode, SignIn, Claim, HaveCode, HaveAccount,
    ColNew, ColPreparing, ColReady, Tables, All, StSushi, StKitchen, StBar, KTable, KPickup, KDelivery,
    KMin, BumpConfirm, BumpPreparing, BumpReady, BumpCollected, KSeen, KUnseen, NoTickets, NoTables,
    NoAccess, Rounds, Due, Refresh, SignOut, Saved, Error, Loading, More, Waiter, CounterManager,
    Kitchen, Owner, GuestWaiting, Room, ForTime, StopReject, StopCancel, KReason, KReasonHint, KReasonNeeded, Close,
    // The table sheet (CV1b) and the theme button: room/i18n.js words, checked copy.
    ThemeSystem, ThemeDark, ThemeLight, Back, Send, NoLines, Subtotal, Discount, Total, Owed, AddItem, AddN, Search, NoMatch, SoldOut, Remove, Comp, Comped, MoveTable, Move, WhyRemove, WhyComp, RMistake, RGuestChanged, RUnavailable, RDropped, ROther, OtherText, NeedReason, ChangedReload, Take, TakeN, Amount, Method, Currency, Rate, MCash, MCard, MCheque, MTransfer, MGiftCard, MWallet, RateNeeded, OffTheBill, FillOwed, Taken, PaidInFull, BadAmount, BadRate, BadTip, Tip, WalletCode, NeedWallet, WalletNoTip, MoveLines, MoveLinesTo, PickLines, PickRound, NoTargets, NotAllLines, MoveSitting, MoveSittingHint, Moved, KitchenHasIt, RoundPaid, AlreadyThere, NotHere, GuestRound, GuestConfirm, GuestConfirmed, GuestRejected, QueuedSaved, QueueFull, QueueNoStore, MenuFailed, NoSlug,
}

impl Str {
    /// The word at `i` in `ALL` -- the number the host names a word by (room/canvas/table.js `W`
    /// lists the same order; feed.test.mjs holds the two equal).
    pub fn at(i: usize) -> Option<Str> {
        Str::ALL.get(i).copied()
    }
    /// Every word, for the four-language test.
    pub const ALL: [Str; 129] = [
        Str::Live, Str::Offline, Str::Polling, Str::LoginLine, Str::Email, Str::Password, Str::ClaimCode,
        Str::SignIn, Str::Claim, Str::HaveCode, Str::HaveAccount, Str::ColNew, Str::ColPreparing,
        Str::ColReady, Str::Tables, Str::All, Str::StSushi, Str::StKitchen, Str::StBar, Str::KTable,
        Str::KPickup, Str::KDelivery, Str::KMin, Str::BumpConfirm, Str::BumpPreparing, Str::BumpReady,
        Str::BumpCollected, Str::KSeen, Str::KUnseen, Str::NoTickets, Str::NoTables, Str::NoAccess,
        Str::Rounds, Str::Due, Str::Refresh, Str::SignOut, Str::Saved, Str::Error, Str::Loading, Str::More,
        Str::Waiter, Str::CounterManager, Str::Kitchen, Str::Owner, Str::GuestWaiting, Str::Room, Str::ForTime,
        Str::StopReject, Str::StopCancel, Str::KReason, Str::KReasonHint, Str::KReasonNeeded, Str::Close,
        Str::ThemeSystem, Str::ThemeDark, Str::ThemeLight, Str::Back, Str::Send, Str::NoLines, Str::Subtotal, Str::Discount, Str::Total, Str::Owed, Str::AddItem, Str::AddN, Str::Search, Str::NoMatch, Str::SoldOut, Str::Remove, Str::Comp, Str::Comped, Str::MoveTable, Str::Move, Str::WhyRemove, Str::WhyComp, Str::RMistake, Str::RGuestChanged, Str::RUnavailable, Str::RDropped, Str::ROther, Str::OtherText, Str::NeedReason, Str::ChangedReload, Str::Take, Str::TakeN, Str::Amount, Str::Method, Str::Currency, Str::Rate, Str::MCash, Str::MCard, Str::MCheque, Str::MTransfer, Str::MGiftCard, Str::MWallet, Str::RateNeeded, Str::OffTheBill, Str::FillOwed, Str::Taken, Str::PaidInFull, Str::BadAmount, Str::BadRate, Str::BadTip, Str::Tip, Str::WalletCode, Str::NeedWallet, Str::WalletNoTip, Str::MoveLines, Str::MoveLinesTo, Str::PickLines, Str::PickRound, Str::NoTargets, Str::NotAllLines, Str::MoveSitting, Str::MoveSittingHint, Str::Moved, Str::KitchenHasIt, Str::RoundPaid, Str::AlreadyThere, Str::NotHere, Str::GuestRound, Str::GuestConfirm, Str::GuestConfirmed, Str::GuestRejected, Str::QueuedSaved, Str::QueueFull, Str::QueueNoStore, Str::MenuFailed, Str::NoSlug,
    ];
}

/// One word in the four languages, joined by `|` in lib/langs.js order: sq|en|uk|ru. CHECKED AT
/// COMPILE TIME: `w!` evaluates `four` in a const block, and a word with three languages (or
/// five) fails that evaluation (E0080) -- a missing language is a compile error, not a blank
/// label; a word with no arm is a non-exhaustive `match`. One string per word, not four `&str`s:
/// the four-pointer rows were 4 KB of the module's data (W-CV1B wire budget).
const fn four(s: &'static str) -> &'static str {
    let b = s.as_bytes();
    let (mut i, mut n) = (0, 0);
    while i < b.len() {
        if b[i] == b'|' {
            n += 1;
        }
        i += 1;
    }
    assert!(n == 3, "a word needs exactly four languages: sq|en|uk|ru");
    s
}

macro_rules! w {
    ($s:literal) => {
        const { four($s) }
    };
}

/// Language `l`'s part of a `four` string.
fn part(s: &'static str, l: Lang) -> &'static str {
    let (mut start, mut seen) = (0, 0);
    for (i, &c) in s.as_bytes().iter().enumerate() {
        if c == b'|' {
            if seen == l as usize {
                return s.get(start..i).unwrap_or("");
            }
            seen += 1;
            start = i + 1;
        }
    }
    s.get(start..).unwrap_or("")
}

fn words(k: Str) -> &'static str {
    match k {
        Str::Live => w!("Në lidhje|Live|На зв’язку|На связи"),
        Str::Offline => w!("Pa lidhje|Offline|Без зв’язку|Нет связи"),
        Str::Polling => w!("rifreskim|refreshing|оновлення|обновление"),
        Str::LoginLine => w!("Salla, në dorën tuaj.|The room, in your hand.|Зал у вашій руці.|Зал в вашей руке."),
        Str::Email => w!("Email|Email|Email|Email"),
        Str::Password => w!("Fjalëkalimi|Password|Пароль|Пароль"),
        Str::ClaimCode => w!("Kodi i ftesës|Invite code|Код запрошення|Код приглашения"),
        Str::SignIn => w!("Hyni|Sign in|Увійти|Войти"),
        Str::Claim => w!("Aktivizo|Activate|Активувати|Активировать"),
        Str::HaveCode => w!("Kam një kod ftese|I have an invite code|У мене код запрошення|У меня код приглашения"),
        Str::HaveAccount => w!("Kam llogari|I have an account|У мене є акаунт|У меня есть аккаунт"),
        Str::ColNew => w!("Të reja|New|Нові|Новые"),
        Str::ColPreparing => w!("Në përgatitje|Preparing|Готуються|Готовятся"),
        Str::ColReady => w!("Gati|Ready|Готові|Готовы"),
        Str::Tables => w!("Tavolinat|Tables|Столи|Столы"),
        Str::All => w!("Të gjitha|All|Усі|Все"),
        Str::StSushi => w!("Sushi|Sushi|Суші|Суши"),
        Str::StKitchen => w!("Kuzhina|Kitchen|Кухня|Кухня"),
        Str::StBar => w!("Bar|Bar|Бар|Бар"),
        Str::KTable => w!("Tavolina|Table|Стіл|Стол"),
        Str::KPickup => w!("Merret|Pickup|Самовивіз|Самовывоз"),
        Str::KDelivery => w!("Dërgesë|Delivery|Доставка|Доставка"),
        Str::KMin => w!("min|min|хв|мин"),
        Str::BumpConfirm => w!("Prano|Accept|Прийняти|Принять"),
        Str::BumpPreparing => w!("Fillo|Start|Почати|Начать"),
        Str::BumpReady => w!("Gati|Ready|Готово|Готово"),
        Str::BumpCollected => w!("U dorëzua|Handed over|Видано|Выдано"),
        Str::KSeen => w!("parë|seen|побачено|увидели"),
        Str::KUnseen => w!("prekni kur ta shihni|tap when seen|торкніться, коли побачите|коснитесь, когда увидите"),
        Str::NoTickets => w!("Asnjë biletë e hapur|No open tickets|Відкритих чеків немає|Открытых чеков нет"),
        Str::NoTables => w!("Asnjë tavolinë e hapur|No open tables|Відкритих столів немає|Открытых столов нет"),
        Str::NoAccess => w!("Ky ekran është për kuzhinën dhe sallën; roli juaj nuk ka asnjërën.|This screen is for the pass and the floor; your role has neither.|Цей екран для кухні й залу; ваша роль не має жодного.|Этот экран для кухни и зала; у вашей роли нет ни того, ни другого."),
        Str::Rounds => w!("raunde|rounds|раунди|раунды"),
        Str::Due => w!("Për t’u paguar|Due|До сплати|К оплате"),
        Str::Refresh => w!("Rifresko|Refresh|Оновити|Обновить"),
        Str::SignOut => w!("Dilni|Sign out|Вийти|Выйти"),
        Str::Saved => w!("U ruajt|Saved|Збережено|Сохранено"),
        Str::Error => w!("Gabim|Error|Помилка|Ошибка"),
        Str::Loading => w!("Po ngarkohet…|Loading…|Завантажуємо…|Загружаем…"),
        Str::More => w!("të tjera|more|ще|ещё"),
        Str::Waiter => w!("Kamarier|Waiter|Офіціант|Официант"),
        Str::CounterManager => w!("Arkëtar-menaxher|Counter manager|Касир-менеджер|Кассир-менеджер"),
        Str::Kitchen => w!("Kuzhina|Kitchen|Кухня|Кухня"),
        Str::Owner => w!("Pronar|Owner|Власник|Владелец"),
        Str::GuestWaiting => w!("Porosi nga tavolina|Guest order|Замовлення гостя|Заказ гостя"),
        Str::Room => w!("Salla|Room|Зал|Зал"),
        Str::ForTime => w!("Për orën|For|На|На"),
        Str::StopReject => w!("Refuzo|Reject|Відхилити|Отклонить"),
        Str::StopCancel => w!("Anulo|Cancel|Скасувати|Отменить"),
        Str::KReason => w!("Arsyeja|Reason|Причина|Причина"),
        Str::KReasonHint => w!("Klienti e sheh arsyen.|The customer sees the reason.|Клієнт бачить причину.|Клиент видит причину."),
        Str::KReasonNeeded => w!("Shkruani arsyen.|Write the reason.|Напишіть причину.|Напишите причину."),
        Str::Close => w!("Mbyll|Close|Закрити|Закрыть"),
        Str::ThemeSystem => w!("Tema: si në telefon|Theme: as the phone|Тема: як на телефоні|Тема: как в телефоне"),
        Str::ThemeDark => w!("Tema: e errët|Theme: dark|Тема: темна|Тема: тёмная"),
        Str::ThemeLight => w!("Tema: e ndritshme|Theme: light|Тема: світла|Тема: светлая"),
        Str::Back => w!("Mbrapa|Back|Назад|Назад"),
        Str::Send => w!("Dërgo|Send|Надіслати|Отправить"),
        Str::NoLines => w!("Asnjë artikull|No items|Позицій немає|Позиций нет"),
        Str::Subtotal => w!("Nëntotali|Subtotal|Підсумок|Подытог"),
        Str::Discount => w!("Zbritja|Discount|Знижка|Скидка"),
        Str::Total => w!("Totali|Total|Разом|Итого"),
        Str::Owed => w!("Mbetet|Still owed|Залишилось|Осталось"),
        Str::AddItem => w!("Shto artikuj|Add items|Додати позиції|Добавить позиции"),
        Str::AddN => w!("Shto|Add|Додати|Добавить"),
        Str::Search => w!("Kërko|Search|Пошук|Поиск"),
        Str::NoMatch => w!("Asgjë nuk përputhet|Nothing matches|Нічого не знайдено|Ничего не найдено"),
        Str::SoldOut => w!("mbaroi|sold out|закінчилось|закончилось"),
        Str::Remove => w!("Hiq|Remove|Прибрати|Убрать"),
        Str::Comp => w!("Falas|Comp|За рахунок закладу|За счёт заведения"),
        Str::Comped => w!("Falas|Comped|За рахунок закладу|За счёт заведения"),
        Str::MoveTable => w!("Kalo në tavolinën|Move to table|Пересадити за стіл|Пересадить за стол"),
        Str::Move => w!("Kalo|Move|Пересадити|Пересадить"),
        Str::WhyRemove => w!("Pse hiqet?|Why remove it?|Чому прибрати?|Почему убрать?"),
        Str::WhyComp => w!("Pse falas?|Why comp it?|Чому безкоштовно?|Почему бесплатно?"),
        Str::RMistake => w!("Gabim|Mistake|Помилка|Ошибка"),
        Str::RGuestChanged => w!("Klienti ndryshoi|Guest changed|Гість передумав|Гость передумал"),
        Str::RUnavailable => w!("Mbaroi|Unavailable|Немає в наявності|Нет в наличии"),
        Str::RDropped => w!("Ra|Dropped|Впало|Упало"),
        Str::ROther => w!("Tjetër|Other|Інше|Другое"),
        Str::OtherText => w!("Shkruani arsyen|Say why|Вкажіть причину|Укажите причину"),
        Str::NeedReason => w!("Shkruani një arsye.|Give a reason.|Вкажіть причину.|Укажите причину."),
        Str::ChangedReload => w!("Porosia ndryshoi ndërkohë. U ringarkua — shikojeni dhe provoni sërish.|This order changed while you were editing. Reloaded — check it and try again.|Замовлення змінилось, поки ви редагували. Оновлено — перевірте й спробуйте ще.|Заказ изменился, пока вы редактировали. Обновлено — проверьте и попробуйте ещё."),
        Str::Take => w!("Merr pagesën|Take payment|Прийняти оплату|Принять оплату"),
        Str::TakeN => w!("Merr|Take|Прийняти|Принять"),
        Str::Amount => w!("Shuma|Amount|Сума|Сумма"),
        Str::Method => w!("Mënyra|Method|Спосіб|Способ"),
        Str::Currency => w!("Valuta|Currency|Валюта|Валюта"),
        Str::Rate => w!("Kursi|Rate|Курс|Курс"),
        Str::MCash => w!("Kesh|Cash|Готівка|Наличные"),
        Str::MCard => w!("Kartë|Card|Картка|Карта"),
        Str::MCheque => w!("Çek|Cheque|Чек|Чек"),
        Str::MTransfer => w!("Transfertë|Transfer|Переказ|Перевод"),
        Str::MGiftCard => w!("Kartë dhuratë|Gift card|Подарункова картка|Подарочная карта"),
        Str::MWallet => w!("Portofol|Wallet|Гаманець|Кошелёк"),
        Str::RateNeeded => w!("Shkruani kursin e tabelës.|Type the board rate.|Введіть курс з табло.|Введите курс с табло."),
        Str::OffTheBill => w!("nga fatura|off the bill|з рахунку|со счёта"),
        Str::FillOwed => w!("Sa mbetet|What is owed|Скільки залишилось|Сколько осталось"),
        Str::Taken => w!("U mor|Taken|Прийнято|Принято"),
        Str::PaidInFull => w!("E paguar plotësisht.|Paid in full.|Сплачено повністю.|Оплачено полностью."),
        Str::BadAmount => w!("Shuma nuk lexohet.|That amount does not read.|Суму не прочитати.|Сумму не прочитать."),
        Str::BadRate => w!("Kursi nuk lexohet.|That rate does not read.|Курс не прочитати.|Курс не прочитать."),
        Str::BadTip => w!("Bakshishi nuk lexohet.|That tip does not read.|Чайові не прочитати.|Чаевые не прочитать."),
        Str::Tip => w!("Bakshish|Tip|Чайові|Чаевые"),
        Str::WalletCode => w!("Kodi i portofolit të klientit (nga telefoni i tij)|The guest's wallet code (from their phone)|Код гаманця гостя (з його телефону)|Код кошелька гостя (с его телефона)"),
        Str::NeedWallet => w!("Shkruani portofolin që paguan.|Name the wallet that pays.|Вкажіть гаманець, що платить.|Укажите кошелёк, который платит."),
        Str::WalletNoTip => w!("Portofoli paguan vetëm faturën; bakshishi merret me para ose kartë.|A wallet pays the bill only; take the tip in cash or on the card.|Гаманець сплачує лише рахунок; чайові готівкою або карткою.|Кошелёк оплачивает только счёт; чаевые наличными или картой."),
        Str::MoveLines => w!("Kalo artikujt|Move lines|Перенести позиції|Перенести позиции"),
        Str::MoveLinesTo => w!("Te raundi|To round|До раунду|В раунд"),
        Str::PickLines => w!("Zgjidhni artikujt që kalojnë.|Pick the lines that move.|Виберіть позиції, що переходять.|Выберите позиции, которые переходят."),
        Str::PickRound => w!("Zgjidhni raundin ku shkojnë.|Pick the round they go to.|Виберіть раунд, куди вони йдуть.|Выберите раунд, куда они идут."),
        Str::NoTargets => w!("Asnjë raund tjetër nuk i pranon para kuzhinës.|No other round can take them before the kitchen.|Жоден інший раунд не прийме їх до кухні.|Ни один другой раунд не примет их до кухни."),
        Str::NotAllLines => w!("Të paktën një artikull duhet të mbetet; për të gjithë, anuloni raundin.|At least one line must stay; to move them all, cancel the round.|Хоч одна позиція має лишитись; щоб перенести всі, скасуйте раунд.|Хоть одна позиция должна остаться; чтобы перенести все, отмените раунд."),
        Str::MoveSitting => w!("Kalo tavolinën|Move the table|Пересадити стіл|Пересадить стол"),
        Str::MoveSittingHint => w!("Të gjitha raundet që janë ende në sallë kalojnë në tavolinën e re.|Every round still in the room moves to the new table.|Усі раунди, що ще в залі, переходять за новий стіл.|Все раунды, которые ещё в зале, переходят за новый стол."),
        Str::Moved => w!("U kalua|Moved|Перенесено|Перенесено"),
        Str::KitchenHasIt => w!("Kuzhina e ka tashmë këtë raund; artikujt nuk lëvizin më.|The kitchen already has this round; its lines no longer move.|Кухня вже має цей раунд; позиції більше не переносяться.|Кухня уже получила этот раунд; позиции больше не переносятся."),
        Str::RoundPaid => w!("Ky raund është paguar; nuk ndryshohet.|This round is paid; it cannot be changed.|Цей раунд сплачено; його не змінити.|Этот раунд оплачен; его не изменить."),
        Str::AlreadyThere => w!("Tavolina është tashmë aty.|The table is already there.|Стіл уже там.|Стол уже там."),
        Str::NotHere => w!("Kjo porosi nuk gjendet më; salla u ringarkua.|This order is no longer here; the room was reloaded.|Цього замовлення вже немає; зал оновлено.|Этого заказа уже нет; зал обновлён."),
        Str::GuestRound => w!("Klienti porositi nga kodi QR i tavolinës.|The guest ordered from the table's QR code.|Гість замовив через QR-код столика.|Гость заказал через QR-код столика."),
        Str::GuestConfirm => w!("Konfirmo|Confirm|Підтвердити|Подтвердить"),
        Str::GuestConfirmed => w!("U konfirmua|Confirmed|Підтверджено|Подтверждено"),
        Str::GuestRejected => w!("U refuzua|Rejected|Відхилено|Отклонено"),
        Str::QueuedSaved => w!("Pa lidhje — u ruajt, dërgohet vetë.|Offline — saved, it will send itself.|Без зв’язку — збережено, надішлеться саме.|Нет связи — сохранено, отправится само."),
        Str::QueueFull => w!("Radha e plotë — nuk u ruajt.|Queue full — not saved.|Черга повна — не збережено.|Очередь полна — не сохранено."),
        Str::QueueNoStore => w!("Ky shfletues nuk ruan — nuk u ruajt.|This browser will not store — not saved.|Браузер не зберігає — не збережено.|Браузер не сохраняет — не сохранено."),
        Str::MenuFailed => w!("Menyja nuk u ngarkua.|The menu did not load.|Меню не завантажилось.|Меню не загрузилось."),
        Str::NoSlug => w!("Hapeni nga adresa e lokalit tuaj.|Open this from your venue address.|Відкрийте з адреси вашого закладу.|Откройте с адреса вашего заведения."),
    }
}
