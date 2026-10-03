//! PURE. A QUESTION IN ALBANIAN, ENGLISH, UKRAINIAN OR RUSSIAN, READ WITHOUT A MODEL.
//!
//! Stems, not words: "продавалось", "продавали" and "продажі" share "прода",
//! "porosi" and "porositë" share "porosi". Each list mixes the four languages
//! on purpose -- the reader is not asked which language a question is in
//! before it is understood, because owners mix them ("скільки futomaki?").
//!
//! THE ORDER OF THE CHECKS IS THE MEANING. "Which dish has the worst margin"
//! is about margin, not about the worst dish; "busiest hour" is about hours,
//! not about orders. The narrow topics are tried first, then the superlatives
//! by what they qualify, then the plain counts. A question none of them reads
//! is `None`, and goes to the model's picker or to the list of examples.

use super::query::{Dish, Kind, Query};

const MARGIN: &[&str] = &["margin", "маржа", "маржин", "марж", "marzh"];
const FOOD_COST: &[&str] = &["food cost", "foodcost", "собівартіст", "фудкост", "себестоим", "kosto e ushq", "kostoja e ushq"];
const WASTE: &[&str] = &["waste", "spoil", "списан", "відход", "отход", "humb", "mbetj", "skadu"];
const LOW: &[&str] = &["run out", "running low", "low stock", "закінчу", "закінчи", "законч", "заканч", "мало на склад", "мало ", "mbaro", "pak në stok", "po mbaron"];
const CHANNELS: &[&str] = &["channel", "канал", "kanal"];
const REJECTED: &[&str] = &["reject", "refus", "cancel", "відхил", "скасов", "отклон", "отмен", "refuz", "anul"];
const AVERAGE: &[&str] = &["average", "середн", "средн", "mesatar"];
const HOUR: &[&str] = &["hour", "time of day", "годин", "о котор", "в котор час", "час дня", "часы", " час ", "время", "orë", "ora ", "në çfarë ore"];
const DAY: &[&str] = &["day", "день", "дня", "дні", "дни", "ditë", "dita"];
const DISH: &[&str] = &["dish", "sold", "sell", "item", "product", "страв", "прода", "товар", "блюд", "позиц", "pjat", "shit", "produkt"];
const WORST: &[&str] = &["worst", "least", "slowest", "fewest", "найгірш", "найменш", "гірше", "хуже", "худш", "меньше всего", "наименьш", "më pak", "më keq", "më e dobët", "më i dobët"];
const BEST: &[&str] = &["best", "most", "top", "busiest", "завантаж", "загруж", "ngarkuar", "найкращ", "найбільш", "найпопуляр", "найбільше", "лучш", "больше всего", "популярн", "më shumë", "më mirë", "më i shitur", "më e shitur"];
const QUIET: &[&str] = &["quiet", "calm", "найтихіш", "спокійн", "тих", "спокойн", "më e qetë", "qetë"];
const REVENUE: &[&str] = &["revenue", "takings", "turnover", "earn", "made", "money", "виручк", "дохід", "заробил", "гроші", "выручк", "доход", "заработ", "деньг", "të ardhura", "xhiro", "fitu", "para"];
const ORDERS: &[&str] = &["order", "замовлен", "заказ", "porosi"];
const HOW_MANY: &[&str] = &["how many", "how much", "скільки", "сколько", "sa "];

/// A question about a PERSON is never read (no scoring of any participant,
/// `tools/gates/no-scoring.sh`): "who is the best courier" has no kind.
const PERSON: &[&str] = &["courier", "customer", "client", "guest", "waiter", "staff", "employee", "driver", "rider",
    "кур'єр", "курʼєр", "кур’єр", "курьер", "клієнт", "клиент", "гост", "офіціант", "официант", "персонал", "працівник", "сотрудник", "водій", "водител",
    "korrier", "klient", "kamarier", "punonjës", "shofer"];

const TODAY: &[&str] = &["today", "сьогодні", "сегодня", "sot"];
const MONTH: &[&str] = &["month", "30 days", "місяц", "месяц", "muaj"];
const WEEK: &[&str] = &["week", "7 days", "тижд", "недел", "javë", "javës"];

/// Weekday stems, Monday first. "середн"/"средн" (average) must not read as
/// Wednesday, so Wednesday's stems carry the next letter.
const WEEKDAY: [&[&str]; 7] = [
    &["monday", "понеділ", "понедельн", "hënë", "hene"],
    &["tuesday", "вівтор", "вторник", "martë", "marte"],
    &["wednesday", "середу", "середа", "середи", "среду", "среда", "среды", "mërkur", "merkur"],
    &["thursday", "четвер", "четверг", "enjte"],
    &["friday", "п'ятниц", "пʼятниц", "пятниц", "п’ятниц", "premte"],
    &["saturday", "субот", "суббот", "shtunë", "shtune"],
    &["sunday", "неділ", "воскрес", "diel"],
];

fn has(q: &str, stems: &[&str]) -> bool {
    stems.iter().any(|s| q.contains(s))
}

/// Lower case, one space between words, a space at both ends (so "sa " and
/// "ora " match at the end of a question too).
pub fn norm(question: &str) -> String {
    let words: Vec<String> = question
        .split(|c: char| c.is_whitespace() || ",.?!;:«»\"()".contains(c))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    format!(" {} ", words.join(" "))
}

/// The dish the question names, longest name first ("Futomaki salmon" before
/// "Futomaki"). A name under three letters is never matched inside words.
pub fn dish_named(q: &str, dishes: &[Dish]) -> Option<String> {
    let mut by_len: Vec<&Dish> = dishes.iter().filter(|(_, n)| n.chars().count() >= 3).collect();
    by_len.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    by_len.into_iter().find(|(_, n)| q.contains(&n.to_lowercase())).map(|(id, _)| id.clone())
}

pub fn weekday_in(q: &str) -> Option<usize> {
    WEEKDAY.iter().position(|stems| has(q, stems))
}

pub fn window_in(q: &str, weekday: bool) -> i64 {
    if has(q, TODAY) {
        1
    } else if has(q, MONTH) {
        30
    } else if has(q, WEEK) {
        7
    } else if weekday {
        // "on Monday" with no window: the Mondays of the last thirty days.
        30
    } else {
        7
    }
}

/// The question as the model's picker may see it: a word that looks like a
/// phone number (seven digits or more) or an e-mail address is withheld, as
/// `assist::redact` withholds a contact. The picker needs neither.
pub fn withheld(question: &str) -> String {
    // Runs of number-ish words ("+355 69 123 4567") count as one: a phone is
    // written with spaces as often as without.
    let numberish = |w: &str| w.chars().all(|c| c.is_ascii_digit() || "+-().".contains(c));
    let digits = |ws: &[&str]| ws.iter().map(|w| w.chars().filter(char::is_ascii_digit).count()).sum::<usize>();
    let words: Vec<&str> = question.split_whitespace().collect();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if numberish(words[i]) {
            let mut j = i;
            while j < words.len() && numberish(words[j]) {
                j += 1;
            }
            if digits(&words[i..j]) >= 7 {
                out.push("[withheld]");
            } else {
                out.extend_from_slice(&words[i..j]);
            }
            i = j;
        } else {
            out.push(if words[i].contains('@') || digits(&words[i..=i]) >= 7 { "[withheld]" } else { words[i] });
            i += 1;
        }
    }
    out.join(" ")
}

/// The question names a person: it is refused before any model sees it.
pub fn about_person(question: &str) -> bool {
    has(&norm(question), PERSON)
}

/// The question as a query, or `None`.
pub fn read(question: &str, dishes: &[Dish]) -> Option<Query> {
    let q = norm(question);
    if has(&q, PERSON) {
        return None;
    }
    let weekday = weekday_in(&q);
    let days = window_in(&q, weekday.is_some());
    let dish = dish_named(&q, dishes);
    let (worst, best) = (has(&q, WORST), has(&q, BEST));
    let kind = if has(&q, MARGIN) {
        Kind::WorstMargin
    } else if has(&q, FOOD_COST) {
        Kind::FoodCost
    } else if has(&q, WASTE) {
        Kind::Waste
    } else if has(&q, LOW) {
        Kind::LowStock
    } else if has(&q, CHANNELS) {
        Kind::Channels
    } else if has(&q, REJECTED) {
        Kind::Rejected
    } else if has(&q, AVERAGE) {
        Kind::AverageOrder
    } else if has(&q, HOUR) && (best || worst || has(&q, QUIET)) {
        if worst || has(&q, QUIET) { Kind::QuietestHour } else { Kind::BusiestHour }
    } else if dish.is_some() && !best && !worst {
        Kind::DishSold
    } else if (best || worst) && has(&q, DAY) && weekday.is_none() && !has(&q, DISH) {
        if worst { Kind::WorstDay } else { Kind::BestDay }
    } else if worst {
        Kind::WorstDish
    } else if best {
        Kind::BestDish
    } else if has(&q, ORDERS) {
        Kind::Orders
    } else if has(&q, REVENUE) || (has(&q, HOW_MANY) && has(&q, DISH)) {
        Kind::Revenue
    } else {
        return None;
    };
    let weekday = weekday.filter(|_| kind.takes_weekday());
    let days = if weekday.is_some() { days.max(7) } else { days };
    let dish = dish.filter(|_| kind == Kind::DishSold);
    Some(Query { kind, days, weekday, dish })
}

/// The language a question is written in, when the console did not say:
/// Ukrainian and Russian apart by the letters only one of them has, then by
/// their commonest question words; Albanian by its letters and words.
pub fn lang_of(question: &str) -> &'static str {
    let q = norm(question);
    let cyr = q.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c));
    if cyr {
        if q.chars().any(|c| "іїєґ".contains(c)) || has(&q, &[" що ", " як ", " який ", " яка "]) {
            return "uk";
        }
        return "ru";
    }
    if q.chars().any(|c| "ëç".contains(c)) || has(&q, &[" sa ", " cila ", " cili ", " sot ", " çfarë ", " porosi"]) {
        return "sq";
    }
    "en"
}
