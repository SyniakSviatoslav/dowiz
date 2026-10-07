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
        words(self, k)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Str {
    Live, Offline, Polling, LoginLine, Email, Password, ClaimCode, SignIn, Claim, HaveCode, HaveAccount,
    ColNew, ColPreparing, ColReady, Tables, All, StSushi, StKitchen, StBar, KTable, KPickup, KDelivery,
    KMin, BumpConfirm, BumpPreparing, BumpReady, BumpCollected, KSeen, KUnseen, NoTickets, NoTables,
    NoAccess, Rounds, Due, Refresh, SignOut, Saved, Error, Loading, More, Waiter, CounterManager,
    Kitchen, Owner, GuestWaiting, Room, ForTime, StopReject, StopCancel, KReason, KReasonHint, KReasonNeeded, Close,
}

impl Str {
    /// Every word, for the four-language test.
    pub const ALL: [Str; 53] = [
        Str::Live, Str::Offline, Str::Polling, Str::LoginLine, Str::Email, Str::Password, Str::ClaimCode,
        Str::SignIn, Str::Claim, Str::HaveCode, Str::HaveAccount, Str::ColNew, Str::ColPreparing,
        Str::ColReady, Str::Tables, Str::All, Str::StSushi, Str::StKitchen, Str::StBar, Str::KTable,
        Str::KPickup, Str::KDelivery, Str::KMin, Str::BumpConfirm, Str::BumpPreparing, Str::BumpReady,
        Str::BumpCollected, Str::KSeen, Str::KUnseen, Str::NoTickets, Str::NoTables, Str::NoAccess,
        Str::Rounds, Str::Due, Str::Refresh, Str::SignOut, Str::Saved, Str::Error, Str::Loading, Str::More,
        Str::Waiter, Str::CounterManager, Str::Kitchen, Str::Owner, Str::GuestWaiting, Str::Room, Str::ForTime,
        Str::StopReject, Str::StopCancel, Str::KReason, Str::KReasonHint, Str::KReasonNeeded, Str::Close,
    ];
}

/// One word in the four languages, positionally: sq, en, uk, ru.
const fn w(l: Lang, sq: &'static str, en: &'static str, uk: &'static str, ru: &'static str) -> &'static str {
    match l {
        Lang::Sq => sq,
        Lang::En => en,
        Lang::Uk => uk,
        Lang::Ru => ru,
    }
}

fn words(l: Lang, k: Str) -> &'static str {
    match k {
        Str::Live => w(l, "Në lidhje", "Live", "На зв’язку", "На связи"),
        Str::Offline => w(l, "Pa lidhje", "Offline", "Без зв’язку", "Нет связи"),
        Str::Polling => w(l, "rifreskim", "refreshing", "оновлення", "обновление"),
        Str::LoginLine => w(l, "Salla, në dorën tuaj.", "The room, in your hand.", "Зал у вашій руці.", "Зал в вашей руке."),
        Str::Email => w(l, "Email", "Email", "Email", "Email"),
        Str::Password => w(l, "Fjalëkalimi", "Password", "Пароль", "Пароль"),
        Str::ClaimCode => w(l, "Kodi i ftesës", "Invite code", "Код запрошення", "Код приглашения"),
        Str::SignIn => w(l, "Hyni", "Sign in", "Увійти", "Войти"),
        Str::Claim => w(l, "Aktivizo", "Activate", "Активувати", "Активировать"),
        Str::HaveCode => w(l, "Kam një kod ftese", "I have an invite code", "У мене код запрошення", "У меня код приглашения"),
        Str::HaveAccount => w(l, "Kam llogari", "I have an account", "У мене є акаунт", "У меня есть аккаунт"),
        Str::ColNew => w(l, "Të reja", "New", "Нові", "Новые"),
        Str::ColPreparing => w(l, "Në përgatitje", "Preparing", "Готуються", "Готовятся"),
        Str::ColReady => w(l, "Gati", "Ready", "Готові", "Готовы"),
        Str::Tables => w(l, "Tavolinat", "Tables", "Столи", "Столы"),
        Str::All => w(l, "Të gjitha", "All", "Усі", "Все"),
        Str::StSushi => w(l, "Sushi", "Sushi", "Суші", "Суши"),
        Str::StKitchen => w(l, "Kuzhina", "Kitchen", "Кухня", "Кухня"),
        Str::StBar => w(l, "Bar", "Bar", "Бар", "Бар"),
        Str::KTable => w(l, "Tavolina", "Table", "Стіл", "Стол"),
        Str::KPickup => w(l, "Merret", "Pickup", "Самовивіз", "Самовывоз"),
        Str::KDelivery => w(l, "Dërgesë", "Delivery", "Доставка", "Доставка"),
        Str::KMin => w(l, "min", "min", "хв", "мин"),
        Str::BumpConfirm => w(l, "Prano", "Accept", "Прийняти", "Принять"),
        Str::BumpPreparing => w(l, "Fillo", "Start", "Почати", "Начать"),
        Str::BumpReady => w(l, "Gati", "Ready", "Готово", "Готово"),
        Str::BumpCollected => w(l, "U dorëzua", "Handed over", "Видано", "Выдано"),
        Str::KSeen => w(l, "parë", "seen", "побачено", "увидели"),
        Str::KUnseen => w(l, "prekni kur ta shihni", "tap when seen", "торкніться, коли побачите", "коснитесь, когда увидите"),
        Str::NoTickets => w(l, "Asnjë biletë e hapur", "No open tickets", "Відкритих чеків немає", "Открытых чеков нет"),
        Str::NoTables => w(l, "Asnjë tavolinë e hapur", "No open tables", "Відкритих столів немає", "Открытых столов нет"),
        Str::NoAccess => w(
            l,
            "Ky ekran është për kuzhinën dhe sallën; roli juaj nuk ka asnjërën.",
            "This screen is for the pass and the floor; your role has neither.",
            "Цей екран для кухні й залу; ваша роль не має жодного.",
            "Этот экран для кухни и зала; у вашей роли нет ни того, ни другого.",
        ),
        Str::Rounds => w(l, "raunde", "rounds", "раунди", "раунды"),
        Str::Due => w(l, "Për t’u paguar", "Due", "До сплати", "К оплате"),
        Str::Refresh => w(l, "Rifresko", "Refresh", "Оновити", "Обновить"),
        Str::SignOut => w(l, "Dilni", "Sign out", "Вийти", "Выйти"),
        Str::Saved => w(l, "U ruajt", "Saved", "Збережено", "Сохранено"),
        Str::Error => w(l, "Gabim", "Error", "Помилка", "Ошибка"),
        Str::Loading => w(l, "Po ngarkohet…", "Loading…", "Завантажуємо…", "Загружаем…"),
        Str::More => w(l, "të tjera", "more", "ще", "ещё"),
        Str::Waiter => w(l, "Kamarier", "Waiter", "Офіціант", "Официант"),
        Str::CounterManager => w(l, "Arkëtar-menaxher", "Counter manager", "Касир-менеджер", "Кассир-менеджер"),
        Str::Kitchen => w(l, "Kuzhina", "Kitchen", "Кухня", "Кухня"),
        Str::Owner => w(l, "Pronar", "Owner", "Власник", "Владелец"),
        Str::GuestWaiting => w(l, "Porosi nga tavolina", "Guest order", "Замовлення гостя", "Заказ гостя"),
        Str::Room => w(l, "Salla", "Room", "Зал", "Зал"),
        Str::ForTime => w(l, "Për orën", "For", "На", "На"),
        Str::StopReject => w(l, "Refuzo", "Reject", "Відхилити", "Отклонить"),
        Str::StopCancel => w(l, "Anulo", "Cancel", "Скасувати", "Отменить"),
        Str::KReason => w(l, "Arsyeja", "Reason", "Причина", "Причина"),
        Str::KReasonHint => w(l, "Klienti e sheh arsyen.", "The customer sees the reason.", "Клієнт бачить причину.", "Клиент видит причину."),
        Str::KReasonNeeded => w(l, "Shkruani arsyen.", "Write the reason.", "Напишіть причину.", "Напишите причину."),
        Str::Close => w(l, "Mbyll", "Close", "Закрити", "Закрыть"),
    }
}
