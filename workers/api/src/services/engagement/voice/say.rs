//! PURE. What the hub says back, in the speaker's language.
//!
//! A refusal the speaker cannot read is not a refusal, it is a noise: every
//! key the grammar, the matcher and the room can refuse with has a line here
//! in every language of `dowiz_hub::lang::LANGS`, and a test holds that no key
//! falls through to English silently -- a missing key reads as the key itself.

/// One line per language, in `LANGS` order: `[sq, en, uk, ru]`. The length is
/// the language count, so a language added there does not compile until it
/// is said here.
type Said = [&'static str; dowiz_hub::lang::LANGS.len()];

/// The index of the speaker's language in `LANGS`; anything else is English.
fn pick(lang: &str) -> usize {
    dowiz_hub::lang::LANGS.iter().position(|l| lang.starts_with(l)).unwrap_or(1)
}

/// `[sq, en, uk, ru]` per refusal key.
const LINES: &[(&str, Said)] = &[
    ("nothing", ["Nuk u dëgjua asgjë", "Nothing was said", "Нічого не почуто", "Ничего не услышано"]),
    ("more_than_one", ["U dëgjuan dy komanda — thoni vetëm një", "Heard two commands — say one", "Почуто дві команди — скажіть одну", "Услышаны две команды — скажите одну"]),
    ("which_table", ["Cila tavolinë?", "Which table?", "Який стіл?", "Какой стол?"]),
    ("cash_or_card", ["Kesh apo kartë?", "Cash or card?", "Готівка чи картка?", "Наличные или карта?"]),
    ("which_dish", ["Cila pjatë?", "Which dish?", "Яка страва?", "Какое блюдо?"]),
    ("two_numbers", ["U dëgjuan dy numra — sa copë?", "Heard two numbers — how many?", "Почуто два числа — скільки?", "Услышаны два числа — сколько?"]),
    ("not_room", ["Kjo nuk është komandë e sallës", "That is not a room command", "Це не команда для зали", "Это не команда для зала"]),
    ("no_dish", ["S'ka pjatë me këtë emër", "No dish by that name", "Немає страви з такою назвою", "Нет блюда с таким названием"]),
    ("many_dishes", ["Disa pjata përshtaten:", "Several dishes fit:", "Підходить кілька страв:", "Подходит несколько блюд:"]),
    ("dish_off", ["Kjo pjatë nuk është në shitje", "That dish is off sale", "Ця страва не в продажу", "Это блюдо не в продаже"]),
    ("no_table", ["Kjo tavolinë nuk është e hapur", "That table is not open", "Цей стіл не відкритий", "Этот стол не открыт"]),
    ("two_sittings", ["Dy tavolina me këtë numër — zgjidhni në ekran", "Two tables have that number — pick on the screen", "Два столи з цим номером — оберіть на екрані", "Два стола с этим номером — выберите на экране"]),
    ("two_rounds", ["Dy raunde përshtaten — zgjidhni në ekran", "Two rounds fit — pick on the screen", "Підходить два раунди — оберіть на екрані", "Подходят два раунда — выберите на экране"]),
    ("nothing_owed", ["Kjo tavolinë nuk ka borxh", "Nothing is owed at that table", "За цим столом нічого не винні", "За этим столом ничего не должны"]),
    ("nothing_to_send", ["Raundi është bosh", "The round is empty", "Раунд порожній", "Раунд пуст"]),
    ("other_table", ["Raundi në telefon është për një tavolinë tjetër", "The round on this phone is for another table", "Раунд на телефоні — для іншого столу", "Раунд на телефоне — для другого стола"]),
    ("cap_orders", ["Roli juaj nuk merr porosi", "Your role does not take orders", "Ваша роль не приймає замовлення", "Ваша роль не принимает заказы"]),
    ("cap_payment", ["Roli juaj nuk merr pagesa", "Your role does not take payments", "Ваша роль не приймає оплату", "Ваша роль не принимает оплату"]),
    ("shift_owner", ["Turnet janë të korrierëve", "Shifts are the couriers'", "Зміни — це для кур'єрів", "Смены — это для курьеров"]),
    ("bad_draft", ["Raundi në telefon nuk lexohet", "The round on this phone cannot be read", "Раунд на телефоні не читається", "Раунд на телефоне не читается"]),
    // The kitchen's words (voice/kitchen.rs), 2026-09-26.
    ("cap_kitchen", ["Roli juaj nuk e bën këtë", "Your role does not do that", "Ваша роль цього не робить", "Ваша роль этого не делает"]),
    ("how_much", ["Sa?", "How much?", "Скільки?", "Сколько?"]),
    ("which_supply", ["Cili përbërës?", "Which ingredient?", "Який інгредієнт?", "Какой ингредиент?"]),
    ("no_supply", ["S'ka përbërës me këtë emër", "No ingredient by that name", "Немає інгредієнта з такою назвою", "Нет ингредиента с таким названием"]),
    ("stock_unit", ["Ky përbërës numërohet me njësi tjetër", "That ingredient is counted in another unit", "Цей інгредієнт рахують в іншій одиниці", "Этот ингредиент считают в другой единице"]),
    // Which ticket, and the confirmation's own refusals (W-QA 2026-09-26: these
    // were Ukrainian literals whatever the reader's language).
    ("no_orders", ["Tani s'ka porosi", "There are no orders now", "Зараз немає замовлень", "Сейчас нет заказов"]),
    ("no_such_number", ["S'ka porosi të hapur me këtë numër", "No open order has that number", "Такого номера серед відкритих немає", "Такого номера среди открытых нет"]),
    ("many_numbers", ["Disa porosi përshtaten — thoni më shumë shifra", "Several orders fit — say more of the number", "Під цей номер підходить кілька — скажіть більше цифр", "Под этот номер подходит несколько — скажите больше цифр"]),
    ("which_order", ["Cila porosi? Thoni numrin e biletës", "Which order? Say the ticket's number", "Яке саме? Назвіть номер", "Какой именно? Назовите номер"]),
    ("not_yours", ["Ky konfirmim nuk është i juaji", "That confirmation is not yours", "Це підтвердження не ваше", "Это подтверждение не ваше"]),
    ("bad_answer", ["Përgjigje e gabuar", "That is not the right answer", "Не та відповідь", "Не тот ответ"]),
    ("answer_expired", ["Kjo përgjigje nuk vlen më — thoni përsëri", "That answer has expired — say it again", "Та відповідь уже не дійсна — скажіть ще раз", "Этот ответ уже недействителен — скажите ещё раз"]),
    ("waste_reason", ["Pse hidhet? I prishur, i rënë, i pashitur, i kthyer apo për stafin", "Why? Spoiled, dropped, unsold, returned or staff meal", "Чому? Зіпсувалось, впало, непродане, повернули чи для персоналу", "Почему? Испортилось, упало, не продано, вернули или для персонала"]),
];

/// The line for a refusal key. An unknown key reads as itself.
pub fn line<'a>(key: &'a str, lang: &str) -> &'a str {
    LINES.iter().find(|(k, _)| *k == key).map(|(_, l)| l[pick(lang)]).unwrap_or(key)
}

/// Every key, for the test that holds them all translated.
#[cfg(test)]
pub fn keys() -> impl Iterator<Item = &'static str> {
    LINES.iter().map(|(k, _)| *k)
}

/// "2 × Margherita"
pub fn lines_of(items: &[(String, u32)]) -> String {
    items.iter().map(|(n, q)| format!("{q} × {n}")).collect::<Vec<_>>().join(", ")
}

/// Read-backs for the room's writes and the owner's extras.
pub fn add(lang: &str, qty: u32, dish: &str, table: &str) -> String {
    match pick(lang) {
        0 => format!("shto {qty} × {dish} në tavolinën {table}"),
        2 => format!("додати {qty} × {dish} на стіл {table}"),
        3 => format!("добавить {qty} × {dish} на стол {table}"),
        _ => format!("add {qty} × {dish} to table {table}"),
    }
}
pub fn send(lang: &str, table: &str, items: &[(String, u32)]) -> String {
    let what = lines_of(items);
    match pick(lang) {
        0 => format!("dërgo në tavolinën {table}: {what}"),
        2 => format!("відправити на стіл {table}: {what}"),
        3 => format!("отправить на стол {table}: {what}"),
        _ => format!("send to table {table}: {what}"),
    }
}
pub fn paid(lang: &str, table: &str, cash: bool) -> String {
    match (pick(lang), cash) {
        (0, true) => format!("tavolina {table}: paguar me kesh"),
        (0, false) => format!("tavolina {table}: paguar me kartë"),
        (2, true) => format!("стіл {table}: оплата готівкою"),
        (2, false) => format!("стіл {table}: оплата карткою"),
        (3, true) => format!("стол {table}: оплата наличными"),
        (3, false) => format!("стол {table}: оплата картой"),
        (_, true) => format!("table {table}: paid in cash"),
        (_, false) => format!("table {table}: paid by card"),
    }
}
pub fn dish_sale(lang: &str, dish: &str, on: bool) -> String {
    match (pick(lang), on) {
        (0, true) => format!("rikthe në shitje: {dish}"),
        (0, false) => format!("hiq nga shitja: {dish}"),
        (2, true) => format!("повернути в продаж: {dish}"),
        (2, false) => format!("зняти з продажу: {dish}"),
        (3, true) => format!("вернуть в продажу: {dish}"),
        (3, false) => format!("снять с продажи: {dish}"),
        (_, true) => format!("put back on sale: {dish}"),
        (_, false) => format!("take off sale: {dish}"),
    }
}
pub fn venue(lang: &str, state: &str) -> String {
    let [sq, en, uk, ru] = match state {
        "open" => ["i hapur", "open", "відкрито", "открыто"],
        "busy" => ["i zënë", "busy", "зайнято", "занято"],
        _ => ["i mbyllur", "closed", "закрито", "закрыто"],
    };
    match pick(lang) {
        0 => format!("lokali: {sq}"),
        2 => format!("заклад: {uk}"),
        3 => format!("заведение: {ru}"),
        _ => format!("the venue: {en}"),
    }
}

/// "2000 g Salmon" -- a movement's quantity in the supply's own unit.
fn amount(qty: i64, unit: &str, what: &str) -> String {
    format!("{qty} {unit} {what}")
}
pub fn receive(lang: &str, qty: i64, unit: &str, what: &str) -> String {
    let a = amount(qty, unit, what);
    match pick(lang) {
        0 => format!("erdhi në magazinë: {a}"),
        2 => format!("прихід на склад: {a}"),
        3 => format!("приход на склад: {a}"),
        _ => format!("received onto the shelf: {a}"),
    }
}
pub fn waste(lang: &str, qty: i64, unit: &str, what: &str, reason: &str) -> String {
    let a = amount(qty, unit, what);
    let [sq, en, uk, ru] = match reason {
        "spoiled" => ["i prishur", "spoiled", "зіпсувалось", "испортилось"],
        "dropped" => ["i rënë", "dropped", "впало", "упало"],
        "unsold" => ["i pashitur", "unsold", "непродане", "не продано"],
        "returned" => ["i kthyer", "returned", "повернули", "вернули"],
        _ => ["për stafin", "staff meal", "для персоналу", "для персонала"],
    };
    match pick(lang) {
        0 => format!("hiq nga magazina: {a} ({sq})"),
        2 => format!("списати: {a} ({uk})"),
        3 => format!("списать: {a} ({ru})"),
        _ => format!("write off: {a} ({en})"),
    }
}

#[cfg(test)]
mod tests;
