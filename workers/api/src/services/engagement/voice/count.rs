//! PURE. A stock count said aloud (P14, research 2026-10-03): `count <supply>
//! <quantity>` in sq/en/uk/ru, read into ONE integer in the supply's base unit.
//!
//! "count salmon two kilo three hundred", "count salmon 2,3 kg", "рахую лосось
//! два кіло триста", "numëro losos dy kile e treqind" are all 2300 g. Number
//! words compose (tens, hundreds, thousands, a half), digits take a decimal
//! comma OR a decimal point, and a bare number after kilos or litres is the
//! grams or millilitres that follow ("two kilo three hundred").
//!
//! A QUESTION, NEVER A GUESS. No unit ("count salmon 2") is asked, because 2 kg
//! and 2 g are both somebody's shelf; "2,300" is asked, because it is 2.3 in
//! Albanian and Ukrainian and 2300 in English; two numbers that do not compose
//! ("two three") are asked. Only zero needs no unit: none of anything is none.
//!
//! The words are read from the RAW transcript, not from `words::norm`, which
//! turns "2,3" into two numbers.

/// What the count verb heard. `None` from [`said`] means no count verb at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    Count { item: String, qty: i64, unit: Option<&'static str> },
    /// A `say` key: the question to ask back.
    Unclear(&'static str),
}

/// The most a count may say, in base units: a whole cold room, not a typo.
pub const QTY_MAX: i64 = 1_000_000;

const VERBS: &[&str] = &[
    "count", "counted", "counting", "stocktake", "рахую", "порахував", "порахувала", "порахуй", "підрахунок", "перерахунок",
    "інвентаризація", "считаю", "посчитал", "посчитала", "посчитай", "подсчет", "подсчёт", "пересчет", "пересчёт",
    "инвентаризация", "numero", "numeroj", "numerova", "numerim", "inventar",
];
/// Words that join or pad a quantity and name nothing.
const SKIP: &[&str] = &[
    "and", "a", "of", "the", "is", "there", "we", "have", "please", "e", "dhe", "ka", "kemi", "te", "ne", "me", "ju", "lutem",
    "і", "й", "та", "є", "маємо", "будь", "ласка", "на", "складі", "и", "есть", "имеем", "пожалуйста", "складе",
];
/// `(word, base unit, factor)`. Albanian words are written folded (ë -> e).
const UNITS: &[(&str, &str, i64)] = &[
    ("kg", "g", 1000), ("kilo", "g", 1000), ("kilos", "g", 1000), ("kilogram", "g", 1000), ("kilograms", "g", 1000),
    ("кг", "g", 1000), ("кіло", "g", 1000), ("кілограм", "g", 1000), ("кілограми", "g", 1000), ("кілограмів", "g", 1000),
    ("кілограма", "g", 1000), ("кило", "g", 1000), ("килограмм", "g", 1000), ("килограмма", "g", 1000), ("килограммов", "g", 1000),
    ("kile", "g", 1000), ("kilograme", "g", 1000),
    ("g", "g", 1), ("gr", "g", 1), ("gram", "g", 1), ("grams", "g", 1), ("г", "g", 1), ("гр", "g", 1), ("грам", "g", 1),
    ("грами", "g", 1), ("грамів", "g", 1), ("грама", "g", 1), ("грамм", "g", 1), ("грамма", "g", 1), ("граммов", "g", 1), ("grame", "g", 1),
    ("l", "ml", 1000), ("litre", "ml", 1000), ("litres", "ml", 1000), ("liter", "ml", 1000), ("liters", "ml", 1000), ("л", "ml", 1000),
    ("літр", "ml", 1000), ("літри", "ml", 1000), ("літрів", "ml", 1000), ("літра", "ml", 1000), ("литр", "ml", 1000),
    ("литра", "ml", 1000), ("литров", "ml", 1000), ("litra", "ml", 1000),
    ("ml", "ml", 1), ("millilitre", "ml", 1), ("milliliter", "ml", 1), ("millilitres", "ml", 1), ("мл", "ml", 1),
    ("мілілітр", "ml", 1), ("мілілітрів", "ml", 1), ("миллилитр", "ml", 1), ("миллилитров", "ml", 1), ("mililiter", "ml", 1), ("mililitra", "ml", 1),
    ("pcs", "unit", 1), ("pc", "unit", 1), ("piece", "unit", 1), ("pieces", "unit", 1), ("шт", "unit", 1), ("штук", "unit", 1),
    ("штуки", "unit", 1), ("штука", "unit", 1), ("cope", "unit", 1), ("copa", "unit", 1),
];

#[derive(Clone, Copy)]
enum W {
    /// A value and its place: 0 ones, 1 teens, 2 tens, 3 hundreds.
    N(i64, u8),
    Hundred,
    Thousand,
    Half,
    /// One and a half, said as one word ("півтора").
    OneHalf,
}

const NUMS: &[(&str, W)] = &{
    use W::*;
    [
        ("zero", N(0, 0)), ("нуль", N(0, 0)), ("ноль", N(0, 0)),
        ("one", N(1, 0)), ("two", N(2, 0)), ("three", N(3, 0)), ("four", N(4, 0)), ("five", N(5, 0)), ("six", N(6, 0)),
        ("seven", N(7, 0)), ("eight", N(8, 0)), ("nine", N(9, 0)), ("ten", N(10, 1)), ("eleven", N(11, 1)), ("twelve", N(12, 1)),
        ("thirteen", N(13, 1)), ("fourteen", N(14, 1)), ("fifteen", N(15, 1)), ("sixteen", N(16, 1)), ("seventeen", N(17, 1)),
        ("eighteen", N(18, 1)), ("nineteen", N(19, 1)), ("twenty", N(20, 2)), ("thirty", N(30, 2)), ("forty", N(40, 2)),
        ("fifty", N(50, 2)), ("sixty", N(60, 2)), ("seventy", N(70, 2)), ("eighty", N(80, 2)), ("ninety", N(90, 2)),
        ("hundred", Hundred), ("thousand", Thousand), ("half", Half),
        // Ukrainian (an apostrophe is dropped: "п'ять" -> "пять").
        ("один", N(1, 0)), ("одна", N(1, 0)), ("одне", N(1, 0)), ("одну", N(1, 0)), ("два", N(2, 0)), ("дві", N(2, 0)),
        ("три", N(3, 0)), ("чотири", N(4, 0)), ("пять", N(5, 0)), ("шість", N(6, 0)), ("сім", N(7, 0)), ("вісім", N(8, 0)),
        ("девять", N(9, 0)), ("десять", N(10, 1)), ("одинадцять", N(11, 1)), ("дванадцять", N(12, 1)), ("тринадцять", N(13, 1)),
        ("чотирнадцять", N(14, 1)), ("пятнадцять", N(15, 1)), ("шістнадцять", N(16, 1)), ("сімнадцять", N(17, 1)),
        ("вісімнадцять", N(18, 1)), ("девятнадцять", N(19, 1)), ("двадцять", N(20, 2)), ("тридцять", N(30, 2)),
        ("сорок", N(40, 2)), ("пятдесят", N(50, 2)), ("шістдесят", N(60, 2)), ("сімдесят", N(70, 2)), ("вісімдесят", N(80, 2)),
        ("девяносто", N(90, 2)), ("сто", N(100, 3)), ("двісті", N(200, 3)), ("триста", N(300, 3)), ("чотириста", N(400, 3)),
        ("пятсот", N(500, 3)), ("шістсот", N(600, 3)), ("сімсот", N(700, 3)), ("вісімсот", N(800, 3)), ("девятсот", N(900, 3)),
        ("тисяча", Thousand), ("тисячі", Thousand), ("тисяч", Thousand), ("пів", Half), ("половина", Half), ("півтора", OneHalf),
        // Russian, where it differs.
        ("одно", N(1, 0)), ("две", N(2, 0)), ("четыре", N(4, 0)), ("шесть", N(6, 0)), ("семь", N(7, 0)), ("восемь", N(8, 0)),
        ("одиннадцать", N(11, 1)), ("двенадцать", N(12, 1)), ("тринадцать", N(13, 1)), ("четырнадцать", N(14, 1)),
        ("пятнадцать", N(15, 1)), ("шестнадцать", N(16, 1)), ("семнадцать", N(17, 1)), ("восемнадцать", N(18, 1)),
        ("девятнадцать", N(19, 1)), ("двадцать", N(20, 2)), ("тридцать", N(30, 2)), ("пятьдесят", N(50, 2)),
        ("шестьдесят", N(60, 2)), ("семьдесят", N(70, 2)), ("восемьдесят", N(80, 2)), ("двести", N(200, 3)),
        ("четыреста", N(400, 3)), ("пятьсот", N(500, 3)), ("шестьсот", N(600, 3)), ("семьсот", N(700, 3)),
        ("восемьсот", N(800, 3)), ("девятьсот", N(900, 3)), ("тысяча", Thousand), ("тысячи", Thousand), ("тысяч", Thousand),
        ("пол", Half), ("полтора", OneHalf), ("полторы", OneHalf),
        // Albanian, folded (ë -> e, ç -> c).
        ("nje", N(1, 0)), ("dy", N(2, 0)), ("tre", N(3, 0)), ("tri", N(3, 0)), ("kater", N(4, 0)), ("pese", N(5, 0)),
        ("gjashte", N(6, 0)), ("shtate", N(7, 0)), ("tete", N(8, 0)), ("nente", N(9, 0)), ("dhjete", N(10, 1)),
        ("njembedhjete", N(11, 1)), ("dymbedhjete", N(12, 1)), ("trembedhjete", N(13, 1)), ("katermbedhjete", N(14, 1)),
        ("pesembedhjete", N(15, 1)), ("gjashtembedhjete", N(16, 1)), ("shtatembedhjete", N(17, 1)), ("tetembedhjete", N(18, 1)),
        ("nentembedhjete", N(19, 1)), ("njezet", N(20, 2)), ("tridhjete", N(30, 2)), ("dyzet", N(40, 2)),
        ("pesedhjete", N(50, 2)), ("gjashtedhjete", N(60, 2)), ("shtatedhjete", N(70, 2)), ("tetedhjete", N(80, 2)),
        ("nentedhjete", N(90, 2)), ("njeqind", N(100, 3)), ("dyqind", N(200, 3)), ("treqind", N(300, 3)),
        ("katerqind", N(400, 3)), ("peseqind", N(500, 3)), ("gjashteqind", N(600, 3)), ("shtateqind", N(700, 3)),
        ("teteqind", N(800, 3)), ("nenteqind", N(900, 3)), ("qind", Hundred), ("mije", Thousand), ("gjysme", Half),
    ]
};

/// Lowercase, apostrophes gone; a `.` or `,` survives
/// only between two digits (a decimal), everything else non-alphanumeric splits.
fn tokens(s: &str) -> Vec<String> {
    let cs: Vec<char> = s
        .to_lowercase()
        .chars()
        .filter(|c| !matches!(c, '\'' | '\u{2019}' | '\u{02bc}' | '`'))
        .collect();
    let mut out = vec![String::new()];
    for (i, &c) in cs.iter().enumerate() {
        let decimal = matches!(c, '.' | ',')
            && i > 0
            && cs[i - 1].is_ascii_digit()
            && cs.get(i + 1).is_some_and(|n| n.is_ascii_digit());
        if c.is_alphanumeric() || decimal {
            out.last_mut().unwrap().push(c);
        } else if !out.last().unwrap().is_empty() {
            out.push(String::new());
        }
    }
    out.retain(|w| !w.is_empty());
    out
}

/// A number being said: `num / den`, the thousands already closed off.
#[derive(Default)]
struct Acc {
    done: i64,
    cur: i64,
    den: i64,
    num: i64,
    /// The smallest place said so far in `cur` (4 = nothing yet).
    place: u8,
    any: bool,
    /// A digit token or a half: nothing else may join it.
    closed: bool,
}

impl Acc {
    fn new() -> Self {
        Acc { place: 4, den: 1, ..Default::default() }
    }
    /// The value as `(numerator, denominator)`.
    fn value(&self) -> (i64, i64) {
        if self.den == 1 { (self.done + self.cur, 1) } else { (self.num, self.den) }
    }
    fn word(&mut self, w: W) -> Result<(), &'static str> {
        if self.closed {
            return Err("two_numbers");
        }
        match w {
            W::N(n, p) => {
                // "twenty two" composes; "two three" and "twelve five" do not.
                let ok = self.place == 4 || (self.place == 3 && p < 3) || (self.place == 2 && p == 0);
                if !ok {
                    return Err("two_numbers");
                }
                self.cur += n;
                self.place = p;
            }
            W::Hundred if self.cur < 10 && self.place != 3 => {
                self.cur = self.cur.max(1) * 100;
                self.place = 3;
            }
            W::Hundred => return Err("two_numbers"),
            W::Thousand => {
                self.done += self.cur.max(1) * 1000;
                self.cur = 0;
                self.place = 4;
            }
            W::Half | W::OneHalf => {
                let whole = self.done + self.cur + i64::from(matches!(w, W::OneHalf));
                (self.num, self.den, self.closed) = (whole * 2 + 1, 2, true);
            }
        }
        self.any = true;
        Ok(())
    }
    fn digits(&mut self, t: &str) -> Result<(), &'static str> {
        if self.any {
            return Err("two_numbers");
        }
        let (int, frac) = t.split_once(['.', ',']).unwrap_or((t, ""));
        if frac.len() >= 3 {
            return Err("decimal_unclear");
        }
        let scale = 10i64.pow(frac.len() as u32);
        let m: i64 = format!("{int}{frac}").parse().map_err(|_| "how_much")?;
        if m > QTY_MAX * 10 {
            return Err("how_much");
        }
        if scale == 1 {
            self.cur = m;
        } else {
            (self.num, self.den) = (m, scale);
        }
        (self.any, self.closed, self.place) = (true, true, 0);
        Ok(())
    }
}

/// `n / d * factor` as a whole number of base units, or `None`.
fn whole(n: i64, d: i64, factor: i64) -> Option<i64> {
    let t = n.checked_mul(factor)?;
    (t % d == 0).then_some(t / d)
}

/// A count, a question about one, or `None` when no count verb was said.
pub fn said(transcript: &str) -> Option<Heard> {
    let toks = tokens(transcript);
    if !toks.iter().any(|t| VERBS.contains(&fold(t).as_str())) {
        return None;
    }
    Some(read(&toks).unwrap_or_else(Heard::Unclear))
}

/// A word as the tables hold it: Albanian `ë`/`ç` folded, so a recogniser
/// that drops the diacritics still counts. The SUPPLY keeps its letters --
/// the matcher compares it with the name as the venue wrote it.
fn fold(w: &str) -> String {
    w.chars().map(|c| match c { 'ë' => 'e', 'ç' => 'c', c => c }).collect()
}

fn read(toks: &[String]) -> Result<Heard, &'static str> {
    let (mut item, mut acc) = (Vec::new(), Acc::new());
    // Each said quantity: (numerator, denominator, unit).
    let mut parts: Vec<(i64, i64, Option<(&'static str, i64)>)> = Vec::new();
    for raw in toks {
        let f = fold(raw);
        let t = f.as_str();
        if VERBS.contains(&t) || SKIP.contains(&t) {
            continue;
        }
        if let Some((_, base, f)) = UNITS.iter().find(|(u, ..)| *u == t) {
            // "кіло триста": a bare kilo or litre is one; a bare gram is nothing said.
            let (n, d) = match (acc.any, *f > 1) {
                (true, _) => acc.value(),
                (false, true) => (1, 1),
                (false, false) => return Err("how_much"),
            };
            parts.push((n, d, Some((*base, *f))));
            acc = Acc::new();
        } else if t.starts_with(|c: char| c.is_ascii_digit()) && t.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',') {
            acc.digits(t)?;
        } else if let Some((_, w)) = NUMS.iter().find(|(s, _)| *s == t) {
            acc.word(*w)?;
        } else {
            if acc.any {
                // A number and then the supply ("count 3 salmon"): the unit is missing.
                let (n, d) = acc.value();
                parts.push((n, d, None));
                acc = Acc::new();
            }
            item.push(raw.as_str());
        }
    }
    if acc.any {
        let (n, d) = acc.value();
        parts.push((n, d, None));
    }
    let qty = match parts.as_slice() {
        [] => return Err("how_much"),
        [(0, _, None)] => (0, None),
        [(_, _, None)] => return Err("count_unit"),
        [(n, d, Some((b, f)))] => (whole(*n, *d, *f).ok_or("how_much")?, Some(*b)),
        // "two kilo three hundred": the grams after the kilos.
        [(n, d, Some((b, f))), (m, 1, None)] if *f > 1 && *m < *f => (whole(*n, *d, *f).ok_or("how_much")? + m, Some(*b)),
        // "1 kg 200 g": a smaller unit of the same kind.
        [(n, d, Some((b, f))), (m, e, Some((b2, f2)))] if b == b2 && f2 < f => {
            (whole(*n, *d, *f).ok_or("how_much")? + whole(*m, *e, *f2).ok_or("how_much")?, Some(*b))
        }
        _ => return Err("two_numbers"),
    };
    if item.is_empty() {
        return Err("which_supply");
    }
    if qty.0 > QTY_MAX {
        return Err("how_much");
    }
    Ok(Heard::Count { item: item.join(" "), qty: qty.0, unit: qty.1 })
}

pub mod propose;

#[cfg(test)]
mod tests;
