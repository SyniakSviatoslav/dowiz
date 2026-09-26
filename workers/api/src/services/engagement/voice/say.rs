//! PURE. What the hub says back, in the speaker's language.
//!
//! A refusal the speaker cannot read is not a refusal, it is a noise: every
//! key the grammar, the matcher and the room can refuse with has a line here
//! in sq, en and uk, and a test holds that no key falls through to English
//! silently -- a missing key reads as the key itself, which a test catches.

/// Which of the three; anything else is English.
fn pick(lang: &str) -> usize {
    if lang.starts_with("uk") {
        2
    } else if lang.starts_with("sq") {
        0
    } else {
        1
    }
}

/// `[sq, en, uk]` per refusal key.
const LINES: &[(&str, [&str; 3])] = &[
    ("nothing", ["Nuk u dëgjua asgjë", "Nothing was said", "Нічого не почуто"]),
    ("more_than_one", ["U dëgjuan dy komanda — thoni vetëm një", "Heard two commands — say one", "Почуто дві команди — скажіть одну"]),
    ("which_table", ["Cila tavolinë?", "Which table?", "Який стіл?"]),
    ("cash_or_card", ["Kesh apo kartë?", "Cash or card?", "Готівка чи картка?"]),
    ("which_dish", ["Cila pjatë?", "Which dish?", "Яка страва?"]),
    ("two_numbers", ["U dëgjuan dy numra — sa copë?", "Heard two numbers — how many?", "Почуто два числа — скільки?"]),
    ("not_room", ["Kjo nuk është komandë e sallës", "That is not a room command", "Це не команда для зали"]),
    ("no_dish", ["S'ka pjatë me këtë emër", "No dish by that name", "Немає страви з такою назвою"]),
    ("many_dishes", ["Disa pjata përshtaten:", "Several dishes fit:", "Підходить кілька страв:"]),
    ("dish_off", ["Kjo pjatë nuk është në shitje", "That dish is off sale", "Ця страва не в продажу"]),
    ("no_table", ["Kjo tavolinë nuk është e hapur", "That table is not open", "Цей стіл не відкритий"]),
    ("two_sittings", ["Dy tavolina me këtë numër — zgjidhni në ekran", "Two tables have that number — pick on the screen", "Два столи з цим номером — оберіть на екрані"]),
    ("two_rounds", ["Dy raunde përshtaten — zgjidhni në ekran", "Two rounds fit — pick on the screen", "Підходить два раунди — оберіть на екрані"]),
    ("nothing_owed", ["Kjo tavolinë nuk ka borxh", "Nothing is owed at that table", "За цим столом нічого не винні"]),
    ("nothing_to_send", ["Raundi është bosh", "The round is empty", "Раунд порожній"]),
    ("other_table", ["Raundi në telefon është për një tavolinë tjetër", "The round on this phone is for another table", "Раунд на телефоні — для іншого столу"]),
    ("cap_orders", ["Roli juaj nuk merr porosi", "Your role does not take orders", "Ваша роль не приймає замовлення"]),
    ("cap_payment", ["Roli juaj nuk merr pagesa", "Your role does not take payments", "Ваша роль не приймає оплату"]),
    ("shift_owner", ["Turnet janë të korrierëve", "Shifts are the couriers'", "Зміни — це для кур'єрів"]),
    ("bad_draft", ["Raundi në telefon nuk lexohet", "The round on this phone cannot be read", "Раунд на телефоні не читається"]),
    // The kitchen's words (voice/kitchen.rs), 2026-09-26.
    ("cap_kitchen", ["Roli juaj nuk e bën këtë", "Your role does not do that", "Ваша роль цього не робить"]),
    ("how_much", ["Sa?", "How much?", "Скільки?"]),
    ("which_supply", ["Cili përbërës?", "Which ingredient?", "Який інгредієнт?"]),
    ("no_supply", ["S'ka përbërës me këtë emër", "No ingredient by that name", "Немає інгредієнта з такою назвою"]),
    ("stock_unit", ["Ky përbërës numërohet me njësi tjetër", "That ingredient is counted in another unit", "Цей інгредієнт рахують в іншій одиниці"]),
    ("waste_reason", ["Pse hidhet? I prishur, i rënë, i pashitur, i kthyer apo për stafin", "Why? Spoiled, dropped, unsold, returned or staff meal", "Чому? Зіпсувалось, впало, непродане, повернули чи для персоналу"]),
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
        _ => format!("add {qty} × {dish} to table {table}"),
    }
}
pub fn send(lang: &str, table: &str, items: &[(String, u32)]) -> String {
    let what = lines_of(items);
    match pick(lang) {
        0 => format!("dërgo në tavolinën {table}: {what}"),
        2 => format!("відправити на стіл {table}: {what}"),
        _ => format!("send to table {table}: {what}"),
    }
}
pub fn paid(lang: &str, table: &str, cash: bool) -> String {
    match (pick(lang), cash) {
        (0, true) => format!("tavolina {table}: paguar me kesh"),
        (0, false) => format!("tavolina {table}: paguar me kartë"),
        (2, true) => format!("стіл {table}: оплата готівкою"),
        (2, false) => format!("стіл {table}: оплата карткою"),
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
        (_, true) => format!("put back on sale: {dish}"),
        (_, false) => format!("take off sale: {dish}"),
    }
}
pub fn venue(lang: &str, state: &str) -> String {
    let [sq, en, uk] = match state {
        "open" => ["i hapur", "open", "відкрито"],
        "busy" => ["i zënë", "busy", "зайнято"],
        _ => ["i mbyllur", "closed", "закрито"],
    };
    match pick(lang) {
        0 => format!("lokali: {sq}"),
        2 => format!("заклад: {uk}"),
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
        _ => format!("received onto the shelf: {a}"),
    }
}
pub fn waste(lang: &str, qty: i64, unit: &str, what: &str, reason: &str) -> String {
    let a = amount(qty, unit, what);
    let [sq, en, uk] = match reason {
        "spoiled" => ["i prishur", "spoiled", "зіпсувалось"],
        "dropped" => ["i rënë", "dropped", "впало"],
        "unsold" => ["i pashitur", "unsold", "непродане"],
        "returned" => ["i kthyer", "returned", "повернули"],
        _ => ["për stafin", "staff meal", "для персоналу"],
    };
    match pick(lang) {
        0 => format!("hiq nga magazina: {a} ({sq})"),
        2 => format!("списати: {a} ({uk})"),
        _ => format!("write off: {a} ({en})"),
    }
}

#[cfg(test)]
mod tests;
