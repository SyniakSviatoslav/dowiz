//! PURE. THE SENTENCES, IN THE FOUR LANGUAGES, AND THE GUARD ON A MODEL'S REWORDING.
//!
//! Every answer and every explanation is first a TEMPLATE filled with numbers
//! the folds computed (`answer.rs`, `explain.rs`). A model may then reword it,
//! and [`keeps_numbers`] decides whether the rewording is shown: it must carry
//! every number of the template and no number that is not in it. A model that
//! "rounds" 1,840 lek to "about 2,000" is not shown; the template is.
//!
//! A template is `[sq, en, uk, ru]`, in the order of `dowiz_hub::lang::LANGS`;
//! `{0}`, `{1}`... are its arguments.

use dowiz_hub::lang::LANGS;

pub struct W(pub [&'static str; 4]);

impl W {
    pub fn in_lang(&self, lang: &str) -> &'static str {
        self.0[LANGS.iter().position(|l| *l == lang).unwrap_or(1)]
    }
}

/// `tpl` in `lang`, the arguments in place, the first letter capital.
pub fn fill(tpl: &W, lang: &str, args: &[String]) -> String {
    let mut s = tpl.in_lang(lang).to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => s,
    }
}

// ── the window ─────────────────────────────────────────────────────────────
pub const TODAY: W = W(["sot", "today", "сьогодні", "сегодня"]);
pub const LAST_N: W = W(["në {0} ditët e fundit", "in the last {0} days", "за останні {0} днів", "за последние {0} дней"]);
pub const ON_WEEKDAY: W = W(["të {0}, në {1} ditët e fundit", "on {0}s in the last {1} days", "у {0} за останні {1} днів", "по {0} за последние {1} дней"]);
pub const WEEKDAY_NAMES: [W; 7] = [
    W(["hënave", "Monday", "понеділки", "понедельникам"]),
    W(["martave", "Tuesday", "вівторки", "вторникам"]),
    W(["mërkurave", "Wednesday", "середи", "средам"]),
    W(["enjteve", "Thursday", "четверги", "четвергам"]),
    W(["premteve", "Friday", "п'ятниці", "пятницам"]),
    W(["shtunave", "Saturday", "суботи", "субботам"]),
    W(["dielave", "Sunday", "неділі", "воскресеньям"]),
];

/// "in the last 7 days", "today", "on Mondays in the last 30 days".
pub fn window(lang: &str, days: i64, weekday: Option<usize>) -> String {
    match (days, weekday) {
        (_, Some(w)) => fill(&ON_WEEKDAY, lang, &[WEEKDAY_NAMES[w].in_lang(lang).to_string(), days.to_string()]),
        (1, None) => TODAY.in_lang(lang).to_string(),
        (n, None) => LAST_N.in_lang(lang).replace("{0}", &n.to_string()),
    }
}

// ── answers (row 2) ────────────────────────────────────────────────────────
pub const REVENUE: W = W(["{0}: të ardhurat {1}, nga {2} porosi.", "{0}: revenue {1} from {2} orders.", "{0}: виручка {1}, замовлень {2}.", "{0}: выручка {1}, заказов {2}."]);
pub const ORDERS: W = W(["{0}: {1} porosi.", "{0}: {1} orders.", "{0}: {1} замовлень.", "{0}: {1} заказов."]);
pub const AVERAGE: W = W(["{0}: porosia mesatare ishte {1}.", "{0}: the average order was {1}.", "{0}: середнє замовлення {1}.", "{0}: средний заказ {1}."]);
pub const REJECTED: W = W(["{0}: u refuzuan {1} nga {2} porosi.", "{0}: {1} of {2} orders were refused.", "{0}: відхилено {1} з {2} замовлень.", "{0}: отклонено {1} из {2} заказов."]);
pub const BEST_DISH: W = W(["{0}: më e shitura ishte {1}, {2} porcione{3}.", "{0}: the best seller was {1}, {2} portions{3}.", "{0}: найкраще продавалась страва {1}: {2} порцій{3}.", "{0}: лучше всего продавалось блюдо {1}: {2} порций{3}."]);
pub const WORST_DISH: W = W(["{0}: nga pjatat e shitura, më pak u shit {1}, {2} porcione{3}.", "{0}: of the dishes that sold, the slowest was {1}, {2} portions{3}.", "{0}: з проданих страв найгірше продавалась {1}: {2} порцій{3}.", "{0}: из проданных блюд хуже всего продавалось {1}: {2} порций{3}."]);
pub const DISH_SOLD: W = W(["{0}: {1} u shit {2} porcione{3}.", "{0}: {1} sold {2} portions{3}.", "{0}: {1} продано {2} порцій{3}.", "{0}: {1} продано {2} порций{3}."]);
pub const BUSIEST_HOUR: W = W(["{0}: ora më e ngarkuar ishte {1}:00, me {2} porosi.", "{0}: the busiest hour was {1}:00, with {2} orders.", "{0}: найзавантаженіша година {1}:00, {2} замовлень.", "{0}: самый загруженный час {1}:00, {2} заказов."]);
pub const QUIET_HOUR: W = W(["{0}: ora më e qetë me porosi ishte {1}:00, me {2} porosi.", "{0}: the quietest hour with orders was {1}:00, with {2} orders.", "{0}: найтихіша година із замовленнями {1}:00, {2} замовлень.", "{0}: самый тихий час с заказами {1}:00, {2} заказов."]);
pub const BEST_DAY: W = W(["{0}: dita më e mirë ishte {1}, me {2}.", "{0}: the best day was {1}, with {2}.", "{0}: найкращий день {1}, виручка {2}.", "{0}: лучший день {1}, выручка {2}."]);
pub const WORST_DAY: W = W(["{0}: dita më e dobët ishte {1}, me {2}.", "{0}: the weakest day was {1}, with {2}.", "{0}: найслабший день {1}, виручка {2}.", "{0}: самый слабый день {1}, выручка {2}."]);
pub const FOOD_COST: W = W(["{0}: kostoja e ushqimit {1}%: {2} mallra kundrejt {3} shitjeve.", "{0}: food cost {1}%: {2} of goods against {3} of sales.", "{0}: фудкост {1}%: собівартість {2} при продажах {3}.", "{0}: фудкост {1}%: себестоимость {2} при продажах {3}."]);
pub const FOOD_COST_NONE: W = W(["{0}: kostoja e ushqimit nuk dihet: asnjë pjatë e shitur nuk ka recetë me çmim.", "{0}: food cost is unknown: no dish that sold has a costed recipe.", "{0}: фудкост невідомий: жодна продана страва не має рецепта з цінами.", "{0}: фудкост неизвестен: ни у одного проданного блюда нет рецепта с ценами."]);
pub const WASTE: W = W(["{0}: humbje {1}; arsyeja më e madhe: {2}.", "{0}: waste {1}; the largest reason: {2}.", "{0}: списано на {1}; найбільша причина: {2}.", "{0}: списано на {1}; главная причина: {2}."]);
pub const WASTE_NONE: W = W(["{0}: nuk u regjistrua asnjë humbje.", "{0}: no waste was recorded.", "{0}: списань не було.", "{0}: списаний не было."]);
pub const LOW: W = W(["{1} përbërës duhen porositur: {2}.", "{1} ingredients need ordering: {2}.", "Треба замовити {1} інгредієнтів: {2}.", "Нужно заказать {1} ингредиентов: {2}."]);
pub const LOW_NONE: W = W(["Asgjë nuk po mbaron me ritmin e tanishëm.", "Nothing is running low at the current pace.", "За поточного темпу нічого не закінчується.", "При текущем темпе ничего не заканчивается."]);
pub const MARGIN: W = W(["{0}: marzhi më i ulët për porcion është i {1}: {2}.", "{0}: the lowest margin per portion is {1}'s: {2}.", "{0}: найменша маржа на порцію у страви {1}: {2}.", "{0}: самая низкая маржа на порцию у блюда {1}: {2}."]);
pub const MARGIN_NONE: W = W(["{0}: asnjë pjatë e shitur nuk ka ende recetë me çmim, ndaj s'ka marzh për të krahasuar.", "{0}: no dish that sold has a costed recipe yet, so there is no margin to compare.", "{0}: жодна продана страва ще не має рецепта з цінами, тож маржу не порівняти.", "{0}: ни у одного проданного блюда ещё нет рецепта с ценами, поэтому маржу не сравнить."]);
pub const CHANNELS: W = W(["{0}: shumica e porosive erdhën nga {1} ({2} nga {3}).", "{0}: most orders came from {1} ({2} of {3}).", "{0}: найбільше замовлень з каналу {1} ({2} з {3}).", "{0}: больше всего заказов из канала {1} ({2} из {3})."]);
pub const NO_SALES: W = W(["{0}: nuk u shit asgjë.", "{0}: nothing was sold.", "{0}: нічого не продано.", "{0}: ничего не продано."]);
pub const UNKNOWN: W = W([
    "Mund të përgjigjem për: të ardhurat, porositë, porosinë mesatare, porositë e refuzuara, pjatën më të shitur dhe më pak të shitur (edhe sipas ditës së javës), shitjet e një pjate, orën më të ngarkuar dhe më të qetë, ditën më të mirë dhe më të dobët, koston e ushqimit, humbjet, çfarë po mbaron, marzhin më të ulët dhe kanalet.",
    "I can answer about: revenue, orders, the average order, refused orders, the best and worst selling dish (also by weekday), one dish's sales, the busiest and quietest hour, the best and weakest day, food cost, waste, what is running low, the lowest margin, and channels.",
    "Я відповідаю про: виручку, замовлення, середнє замовлення, відхилені замовлення, найкращу й найгіршу страву (також за днем тижня), продажі однієї страви, найзавантаженішу й найтихішу годину, найкращий і найслабший день, фудкост, списання, що закінчується, найменшу маржу та канали.",
    "Я отвечаю про: выручку, заказы, средний заказ, отклонённые заказы, лучшее и худшее блюдо (также по дню недели), продажи одного блюда, самый загруженный и самый тихий час, лучший и самый слабый день, фудкост, списания, что заканчивается, самую низкую маржу и каналы.",
]);

// ── explanations (row 3) ───────────────────────────────────────────────────
pub const TREND_PREV: W = W(["Të ardhurat {0} kundrejt {1} në periudhën e mëparshme ({2}).", "Revenue {0} against {1} in the period before ({2}).", "Виручка {0} проти {1} за попередній період ({2}).", "Выручка {0} против {1} за предыдущий период ({2})."]);
pub const TREND_HALF: W = W(["Gjysma e dytë e periudhës solli {0}, e para {1} ({2}).", "The second half of the period took {0}, the first {1} ({2}).", "Друга половина періоду дала {0}, перша {1} ({2}).", "Вторая половина периода дала {0}, первая {1} ({2})."]);
pub const HOURS: W = W(["Ora më e ngarkuar {0}:00 ({1} porosi); më e qeta me porosi {2}:00 ({3}).", "Busiest hour {0}:00 ({1} orders); quietest hour with orders {2}:00 ({3}).", "Найзавантаженіша година {0}:00 ({1} замовлень); найтихіша із замовленнями {2}:00 ({3}).", "Самый загруженный час {0}:00 ({1} заказов); самый тихий с заказами {2}:00 ({3})."]);
pub const MENU: W = W(["Menuja: {0} yje, {1} kuaj pune, {2} enigma, {3} qen.", "Menu: {0} stars, {1} plowhorses, {2} puzzles, {3} dogs.", "Меню: {0} зірок, {1} робочих конячок, {2} загадок, {3} собак.", "Меню: {0} звёзд, {1} рабочих лошадок, {2} загадок, {3} собак."]);
pub const STAR: W = W(["Yll: {0}, i dashur dhe fitimprurës; mbajeni siç është.", "Star: {0}, popular and profitable; keep it as it is.", "Зірка: {0}, популярна і прибуткова; лишіть як є.", "Звезда: {0}, популярное и прибыльное; оставьте как есть."]);
pub const PLOWHORSE: W = W(["Kalë pune: {0} shitet mirë, por fiton {1} më pak për porcion se mesatarja; ngrini çmimin ose uleni koston.", "Plowhorse: {0} sells well but earns {1} less per portion than the average; raise the price or lower its cost.", "Робоча конячка: {0} добре продається, але заробляє на {1} менше за порцію, ніж у середньому; підніміть ціну або здешевіть.", "Рабочая лошадка: {0} хорошо продаётся, но зарабатывает на {1} меньше за порцию, чем в среднем; поднимите цену или удешевите."]);
pub const PUZZLE: W = W(["Enigmë: {0} fiton mirë, por shitet pak; vendoseni më mirë në meny.", "Puzzle: {0} earns well but sells little; give it a better place on the menu.", "Загадка: {0} добре заробляє, але мало продається; дайте їй краще місце в меню.", "Загадка: {0} хорошо зарабатывает, но мало продаётся; дайте ему лучшее место в меню."]);
pub const DOG: W = W(["Qen: {0} shitet pak dhe fiton pak; zëvendësojeni ose ripunojeni.", "Dog: {0} sells little and earns little; replace it or rework it.", "Собака: {0} мало продається і мало заробляє; замініть або переробіть.", "Собака: {0} мало продаётся и мало зарабатывает; замените или переработайте."]);

/// "+12.3%" / "-4.0%" from ‰.
pub fn signed_pct(pm: i64) -> String {
    let sign = if pm < 0 { "-" } else { "+" };
    format!("{sign}{}%", pct(pm.abs()))
}

/// "31.2" from 312 ‰ (no sign, no % mark).
pub fn pct(pm: i64) -> String {
    format!("{}.{}", pm / 10, (pm % 10).abs())
}

/// The digit runs of a text, leading zeros dropped ("09" and "9" are one number).
pub fn numbers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            let t = cur.trim_start_matches('0');
            out.push(if t.is_empty() { "0".to_string() } else { t.to_string() });
            cur.clear();
        }
    }
    out
}

/// A rewording is shown only when it carries every number of the template and
/// no number that is not in it.
pub fn keeps_numbers(template: &str, reworded: &str) -> bool {
    let (want, got) = (numbers(template), numbers(reworded));
    !reworded.trim().is_empty() && want.iter().all(|n| got.contains(n)) && got.iter().all(|n| want.contains(n))
}
